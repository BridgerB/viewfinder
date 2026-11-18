//! The screen: a cheap score for every place so only the likely ones get the full search. Per
//! lens and tilt the photograph's skyline becomes a template sampled once per whole degree; it
//! is swept around the place's horizon, and each window's error, correlation and relief come
//! from running sums plus one cross term. A capped straight-line fit to the residual absorbs a
//! small roll.
//!
//! A sweep scores every whole yaw, in three loops shaped for the vector units: the cross terms
//! of all 360 windows with the template index outermost (so the inner loop runs along the
//! horizon and needs no per-window reduction), the window algebra for all 360 windows without
//! branches, and last the logarithm of the relief term, only for windows that can still win.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::camera::{Camera, Photo};
use crate::config::{ScoreConfig, ScreenConfig};
use crate::dem::Dem;
use crate::field::Field;
use crate::horizon::{AZIMUTHS, CENTI, MISSING};
use crate::score::{drop_penalty, falls_into};

/// The skyline through one lens and tilt, one elevation per whole degree of azimuth offset:
/// `elevs[k]` is at offset `start + k`.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub fov: f64,
    pub start: i64,
    pub elevs: Vec<f64>,
    /// `elevs` minus their mean, as f32: the cross-term weights. Centred weights keep the f32
    /// sums near the size of the covariance they feed, not of the raw elevations.
    centered: Vec<f32>,
    /// `start` wrapped into [0, 360): the window of yaw y starts at (y + rot) mod 360.
    rot: usize,
    // Constants of the window algebra, computed once.
    g1: f64,
    g2: f64,
    gk: f64,
    sxx: f64,
    mg: f64,
    vg: f64,
    gsigma: f64,
    lens_scale: f64,
    /// 1/m.
    inv_m: f64,
    /// Mean column index, (m − 1)/2.
    kbar: f64,
    /// cov(g, k): how the template leans across the window.
    cgk: f64,
    /// Sxx/m.
    sxx_m: f64,
    /// 1/Sxx, or 0 for a window too short to fit a line (no tilt).
    inv_sxx: f64,
}

impl Template {
    #[must_use]
    pub fn new(photo: &Photo, fov: f64, pitch: f64, score: &ScoreConfig) -> Self {
        let cam = Camera::new(photo, fov, 0.0, pitch);
        let (offsets, geoms) = (&cam.offsets, &cam.elevs);
        let n = offsets.len();
        // Fewer than two columns cannot be interpolated: an empty template, which sweep rejects.
        let (start, end) = if n < 2 {
            (0, -1)
        } else {
            (offsets[0].ceil() as i64, offsets[n - 1].floor() as i64)
        };
        let m = (end - start + 1).max(0) as usize;
        let last_pair = n.saturating_sub(2);
        let elevs: Vec<f64> = (0..m)
            .map(|k| {
                let target = (start + k as i64) as f64;
                let mut j = 0;
                while j < last_pair && offsets[j + 1] < target {
                    j += 1;
                }
                let (o0, o1) = (offsets[j], offsets[j + 1]);
                let t = if o1 > o0 {
                    (target - o0) / (o1 - o0)
                } else {
                    0.0
                };
                geoms[j] + t.clamp(0.0, 1.0) * (geoms[j + 1] - geoms[j])
            })
            .collect();
        let g1 = elevs.iter().fold(0.0, |s, &v| s + v);
        let g2 = elevs.iter().fold(0.0, |s, &v| s + v * v);
        let gk = elevs
            .iter()
            .enumerate()
            .fold(0.0, |s, (k, &v)| s + k as f64 * v);
        let mf = m as f64;
        let k1 = (mf * (mf - 1.0)) / 2.0;
        let k2 = ((mf - 1.0) * mf * (2.0 * mf - 1.0)) / 6.0;
        let sxx = k2 - (k1 * k1) / mf;
        let mg = g1 / mf;
        let vg = (g2 / mf - mg * mg).max(0.0);
        let kbar = k1 / mf;
        Self {
            fov,
            start,
            centered: elevs.iter().map(|&v| (v - mg) as f32).collect(),
            rot: start.rem_euclid(AZIMUTHS as i64) as usize,
            elevs,
            g1,
            g2,
            gk,
            sxx,
            mg,
            vg,
            gsigma: vg.sqrt(),
            lens_scale: score.ref_fov / fov,
            inv_m: 1.0 / mf,
            kbar,
            cgk: gk / mf - mg * kbar,
            sxx_m: sxx / mf,
            inv_sxx: if sxx > 0.0 { 1.0 / sxx } else { 0.0 },
        }
    }
}

/// Every template the screen tries: every other lens of the full list (all of them when the
/// lens is known), at each screen pitch.
#[must_use]
pub fn templates(photo: &Photo, score: &ScoreConfig, screen: &ScreenConfig) -> Vec<Template> {
    let fovs: Vec<f64> = if score.fovs.len() <= 3 {
        score.fovs.clone()
    } else {
        score.fovs.iter().step_by(2).copied().collect()
    };
    fovs.iter()
        .flat_map(|&fov| screen.pitches.iter().map(move |&p| (fov, p)))
        .map(|(fov, pitch)| Template::new(photo, fov, pitch, score))
        .collect()
}

/// One place's horizon twice round (so a window never wraps), in degrees, with prefix sums of
/// the value, its square, its index times the value, and missing degrees.
#[derive(Clone, Debug)]
pub struct Sums {
    e: Vec<f64>,
    /// `e` minus the place's mean horizon, as f32, for the cross terms. Missing degrees are 0
    /// here; a window over one is rejected by `pn` before its cross term is used.
    ec: Vec<f32>,
    p0: Vec<f64>,
    p2: Vec<f64>,
    pj: Vec<f64>,
    pn: Vec<f64>,
}

impl Default for Sums {
    fn default() -> Self {
        Self {
            e: vec![0.0; 2 * AZIMUTHS],
            ec: vec![0.0; 2 * AZIMUTHS],
            p0: vec![0.0; 2 * AZIMUTHS + 1],
            p2: vec![0.0; 2 * AZIMUTHS + 1],
            pj: vec![0.0; 2 * AZIMUTHS + 1],
            pn: vec![0.0; 2 * AZIMUTHS + 1],
        }
    }
}

impl Sums {
    /// Fills from `angles`; a missing degree is 0 in the value sums and 1 in the missing count.
    pub fn fill(&mut self, angles: &[i16]) {
        let (mut total, mut present) = (0.0, 0.0);
        for &c in &angles[..AZIMUTHS] {
            if c != MISSING {
                total += f64::from(c) / CENTI;
                present += 1.0;
            }
        }
        let mean = if present > 0.0 { total / present } else { 0.0 };
        for j in 0..2 * AZIMUTHS {
            let c = angles[j % AZIMUTHS];
            let gone = c == MISSING;
            let v = if gone { 0.0 } else { f64::from(c) / CENTI };
            self.e[j] = v;
            self.ec[j] = if gone { 0.0 } else { (v - mean) as f32 };
            self.p0[j + 1] = self.p0[j] + v;
            self.p2[j + 1] = self.p2[j] + v * v;
            self.pj[j + 1] = self.pj[j] + j as f64 * v;
            self.pn[j + 1] = self.pn[j] + if gone { 1.0 } else { 0.0 };
        }
    }
}

/// Σ g[k]·e[b+k] for one window, in f64 with eight independent running sums: the reference the
/// sweep's cross terms are checked against.
#[inline]
#[must_use]
pub fn dot(g: &[f64], e: &[f64], b: usize) -> f64 {
    const LANES: usize = 8;
    let e = &e[b..b + g.len()];
    let mut acc = [0.0_f64; LANES];
    let (gc, gt) = g.as_chunks::<LANES>();
    let (ec, et) = e.as_chunks::<LANES>();
    for (gs, es) in gc.iter().zip(ec) {
        for i in 0..LANES {
            acc[i] = gs[i].mul_add(es[i], acc[i]);
        }
    }
    let mut sum = ((acc[0] + acc[4]) + (acc[1] + acc[5])) + ((acc[2] + acc[6]) + (acc[3] + acc[7]));
    for (x, y) in gt.iter().zip(et) {
        sum = x.mul_add(*y, sum);
    }
    sum
}

/// Window starts per register tile of `cross_all`: ten 4-lane registers, and 360 = 9 × 40.
const TILE: usize = 40;

/// Σ g[k]·e[b+k] for every window start b in 0..360, into `out`. Each tile of 40 window starts
/// keeps its sums in registers for the whole template, so a template value costs ten vector
/// loads and ten fused multiply-adds, with no loads or stores of the sums in between.
#[inline]
pub fn cross_all(g: &[f32], e: &[f32], out: &mut [f32; AZIMUTHS]) {
    for (t, tile) in out.as_chunks_mut::<TILE>().0.iter_mut().enumerate() {
        let b0 = t * TILE;
        let mut acc = [0.0_f32; TILE];
        for (k, &w) in g.iter().enumerate() {
            let row: &[f32; TILE] = &e[b0 + k..b0 + k + TILE].as_chunks::<TILE>().0[0];
            for i in 0..TILE {
                acc[i] = w.mul_add(row[i], acc[i]);
            }
        }
        *tile = acc;
    }
}

/// Per-thread buffers for one sweep, indexed by window start.
struct Sweep {
    cross: [f32; AZIMUTHS],
    /// fit + shape + drop, or infinity for a window over a missing degree.
    partial: [f64; AZIMUTHS],
    /// Variance of the horizon and of the tilted template, for the relief term.
    vh: [f64; AZIMUTHS],
    vt: [f64; AZIMUTHS],
    /// The drop penalty of the yaw whose window starts here.
    drop: [f64; AZIMUTHS],
}

impl Default for Sweep {
    fn default() -> Self {
        Self {
            cross: [0.0; AZIMUTHS],
            partial: [0.0; AZIMUTHS],
            vh: [0.0; AZIMUTHS],
            vt: [0.0; AZIMUTHS],
            drop: [0.0; AZIMUTHS],
        }
    }
}

/// The first and last 360 values of a prefix array for windows of length `m`.
fn span(p: &[f64], m: usize) -> (&[f64], &[f64]) {
    (&p[..AZIMUTHS], &p[m..m + AZIMUTHS])
}

/// The best (loss, yaw) of one template at one place over every whole yaw, ties to the lowest
/// yaw. `drops` is the drop penalty per yaw. `None` when the template is too short or flat, or
/// no window fits.
fn sweep(
    s: &Sums,
    tpl: &Template,
    drops: &[f64; AZIMUTHS],
    tilt_cap: f64,
    min_relief: f64,
    w: &mut Sweep,
) -> Option<(f64, usize)> {
    let m = tpl.elevs.len();
    if m < 8 || tpl.gsigma < min_relief {
        return None;
    }
    cross_all(&tpl.centered, &s.ec, &mut w.cross);
    // The window starting at b belongs to yaw (b − rot) mod 360.
    let rot = tpl.rot;
    w.drop[rot..].copy_from_slice(&drops[..AZIMUTHS - rot]);
    w.drop[..rot].copy_from_slice(&drops[AZIMUTHS - rot..]);
    // Window b spans [b, b + m): each prefix array is sliced once at both ends to exactly 360
    // values, so the loop below has no bounds checks and vectorises.
    let span = |p| span(p, m);
    let ((p0a, p0z), (p2a, p2z), (pja, pjz), (pna, pnz)) =
        (span(&s.p0), span(&s.p2), span(&s.pj), span(&s.pn));
    for b in 0..AZIMUTHS {
        let s1 = p0z[b] - p0a[b];
        let s2 = p2z[b] - p2a[b];
        let sk = (pjz[b] - pja[b]) - b as f64 * s1;
        let sr = s1 - tpl.g1;
        let sxr = (sk - tpl.gk) - tpl.kbar * sr;
        // The tilt a small roll puts on the skyline, fitted to the residual and capped, then
        // applied to the photograph's skyline for every term.
        let tilt = (-tilt_cap).max(tilt_cap.min(sxr * tpl.inv_sxx));
        let mh = s1 * tpl.inv_m;
        let vh = (s2 * tpl.inv_m - mh * mh).max(0.0);
        let vt = (tpl.vg + tilt * tilt * tpl.sxx_m + 2.0 * tilt * tpl.cgk).max(0.0);
        // Centred: Σ (g − ḡ)(e − ē) = Σ g·e − ḡ·Σe, since the centred weights sum to zero.
        let centred = f64::from(w.cross[b]);
        let srr = (s2 - 2.0 * (centred + tpl.mg * s1)) + tpl.g2;
        let rss = (srr - sr * sr * tpl.inv_m) + -2.0 * tilt * sxr + tilt * tilt * tpl.sxx;
        let mse = (rss * tpl.inv_m).max(0.0);
        let denom = (vh * vt).sqrt();
        let cov = centred * tpl.inv_m + tilt * (sk * tpl.inv_m - mh * tpl.kbar);
        let corr = if denom < 1.0e-9 { 0.0 } else { cov / denom };
        let partial = mse.sqrt() * tpl.lens_scale + 2.0 * (1.0 - corr) + w.drop[b];
        let gone = pnz[b] - pna[b] != 0.0;
        w.partial[b] = if gone { f64::INFINITY } else { partial };
        w.vh[b] = vh;
        w.vt[b] = vt;
    }
    // The relief term is never negative, so a window whose other terms already reach the best
    // so far cannot win and its logarithm is skipped.
    let mut best = (f64::INFINITY, 0);
    for yaw in 0..AZIMUTHS {
        let b = if yaw + rot >= AZIMUTHS {
            yaw + rot - AZIMUTHS
        } else {
            yaw + rot
        };
        let partial = w.partial[b];
        if partial < best.0 {
            // |ln(σh/σt)| with each σ floored at 1e-3, from the variances: ½·|ln(vh/vt)|.
            let ratio = w.vh[b].max(1.0e-6) / w.vt[b].max(1.0e-6);
            let scale = (0.5 * ratio.ln().abs() - 0.10).max(0.0);
            let loss = partial + scale;
            if loss < best.0 {
                best = (loss, yaw);
            }
        }
    }
    best.0.is_finite().then_some(best)
}

/// A place's screen result.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScreenRow {
    pub index: usize,
    pub lat: f64,
    pub lon: f64,
    pub loss: f64,
    pub yaw: f64,
    pub fov: f64,
}

struct ScreenScratch {
    sums: Sums,
    falls: Vec<f64>,
    drops: [f64; AZIMUTHS],
    sweep: Box<Sweep>,
}

impl Default for ScreenScratch {
    fn default() -> Self {
        Self {
            sums: Sums::default(),
            falls: Vec::new(),
            drops: [0.0; AZIMUTHS],
            sweep: Box::default(),
        }
    }
}

/// The screen over every place of `field`, best first (stable: ties keep field order).
#[must_use]
pub fn screen(
    field: &Field,
    dem: &Dem<'_>,
    templates: &[Template],
    score: &ScoreConfig,
    screen: &ScreenConfig,
) -> Vec<ScreenRow> {
    let mut rows: Vec<ScreenRow> = (0..field.len())
        .into_par_iter()
        .map_init(ScreenScratch::default, |sc, i| {
            sc.sums.fill(field.angles_of(i));
            falls_into(
                dem,
                field.lats[i],
                field.lons[i],
                f64::from(field.grounds[i]),
                score,
                &mut sc.falls,
            );
            for (d, &f) in sc.drops.iter_mut().zip(&sc.falls) {
                *d = drop_penalty(f, score);
            }
            let mut best: Option<ScreenRow> = None;
            for tpl in templates {
                if let Some((loss, yaw)) = sweep(
                    &sc.sums,
                    tpl,
                    &sc.drops,
                    screen.tilt,
                    score.min_relief,
                    &mut sc.sweep,
                ) && loss < best.map_or(f64::INFINITY, |b| b.loss)
                {
                    best = Some(ScreenRow {
                        index: i,
                        lat: field.lats[i],
                        lon: field.lons[i],
                        loss,
                        yaw: yaw as f64,
                        fov: tpl.fov,
                    });
                }
            }
            best
        })
        .flatten()
        .collect();
    rows.sort_by(|a, b| a.loss.total_cmp(&b.loss));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Pixel;

    #[test]
    fn dot_matches_a_plain_sum() {
        let mut rng = crate::java::JavaRandom::new(5);
        for _ in 0..200 {
            let m = 1 + rng.next_int(130) as usize;
            let b = rng.next_int(300) as usize;
            let g: Vec<f64> = (0..m)
                .map(|_| f64::from(rng.next_int(2000)) / 100.0 - 10.0)
                .collect();
            let e: Vec<f64> = (0..720)
                .map(|_| f64::from(rng.next_int(2000)) / 100.0 - 10.0)
                .collect();
            let plain: f64 = (0..m).map(|k| g[k] * e[b + k]).sum();
            assert!((plain - dot(&g, &e, b)).abs() < 1e-9);
        }
    }

    #[test]
    fn window_sums_match_naive() {
        let mut angles = [0_i16; AZIMUTHS];
        for (i, a) in angles.iter_mut().enumerate() {
            *a = ((i * 37) % 900) as i16 - 200;
        }
        angles[100] = MISSING;
        let mut s = Sums::default();
        s.fill(&angles);
        for (b, m) in [(0, 50), (330, 60), (90, 20), (101, 40)] {
            let naive: f64 = (b..b + m).map(|j| s.e[j]).sum();
            assert!((s.p0[b + m] - s.p0[b] - naive).abs() < 1e-9);
            let missing = (b..b + m)
                .filter(|j| angles[j % AZIMUTHS] == MISSING)
                .count() as f64;
            assert_eq!(s.pn[b + m] - s.pn[b], missing);
        }
    }

    /// One window's screen loss straight from the definitions, in f64, with no running sums:
    /// residual r = h − g, a capped least-squares line in the column index for roll, then RMSE
    /// about that line, correlation and relief against the tilted template.
    fn window_loss(e: &[f64], tpl: &Template, b: usize, tilt_cap: f64, drop: f64) -> f64 {
        let g = &tpl.elevs;
        let m = g.len() as f64;
        let h = &e[b..b + g.len()];
        let kbar = (m - 1.0) / 2.0;
        let r: Vec<f64> = h.iter().zip(g).map(|(h, g)| h - g).collect();
        let rbar = r.iter().sum::<f64>() / m;
        let sxx: f64 = (0..g.len()).map(|k| (k as f64 - kbar).powi(2)).sum();
        let sxr: f64 = r
            .iter()
            .enumerate()
            .map(|(k, r)| (k as f64 - kbar) * r)
            .sum();
        let tilt = (sxr / sxx).clamp(-tilt_cap, tilt_cap);
        let gt: Vec<f64> = g
            .iter()
            .enumerate()
            .map(|(k, g)| g + tilt * (k as f64 - kbar))
            .collect();
        let rss: f64 = r
            .iter()
            .enumerate()
            .map(|(k, r)| (r - rbar - tilt * (k as f64 - kbar)).powi(2))
            .sum();
        let (mg, mh) = (gt.iter().sum::<f64>() / m, h.iter().sum::<f64>() / m);
        let vt = gt.iter().map(|v| (v - mg).powi(2)).sum::<f64>() / m;
        let vh = h.iter().map(|v| (v - mh).powi(2)).sum::<f64>() / m;
        let cov = gt
            .iter()
            .zip(h)
            .map(|(a, b)| (a - mg) * (b - mh))
            .sum::<f64>()
            / m;
        let corr = cov / (vt * vh).sqrt();
        let scale = ((vh.sqrt() / vt.sqrt()).ln().abs() - 0.10).max(0.0);
        (rss / m).sqrt() * tpl.lens_scale + scale + 2.0 * (1.0 - corr) + drop
    }

    #[test]
    fn sweep_matches_every_window_from_the_definitions() {
        let score = ScoreConfig::default();
        let screen_cfg = ScreenConfig::default();
        let skyline: Vec<Pixel> = (0..160)
            .map(|k| {
                let x = 12.5 + 25.0 * f64::from(k);
                Pixel {
                    x,
                    y: 1200.0 + 150.0 * (x / 300.0).sin() + 60.0 * (x / 97.0).cos(),
                }
            })
            .collect();
        let photo = Photo {
            width: 4000,
            height: 3000,
            skyline,
        };
        let mut rng = crate::java::JavaRandom::new(11);
        let mut w = Box::<Sweep>::default();
        for place in 0..6 {
            let p = f64::from(place);
            let mut angles = [0_i16; AZIMUTHS];
            for (az, a) in angles.iter_mut().enumerate() {
                let t = (az as f64).to_radians();
                *a = (300.0 + 250.0 * (3.0 * t + p).sin() + 120.0 * (8.0 * t + 2.0 * p).sin())
                    as i16;
            }
            let mut drops = [0.0; AZIMUTHS];
            for d in &mut drops {
                *d = f64::from(rng.next_int(100)) / 400.0;
            }
            let mut s = Sums::default();
            s.fill(&angles);
            for tpl in templates(&photo, &score, &screen_cfg) {
                let Some((loss, yaw)) =
                    sweep(&s, &tpl, &drops, screen_cfg.tilt, score.min_relief, &mut w)
                else {
                    continue;
                };
                let direct = (0..AZIMUTHS)
                    .map(|y| {
                        let b = (y + tpl.rot) % AZIMUTHS;
                        (window_loss(&s.e, &tpl, b, screen_cfg.tilt, drops[y]), y)
                    })
                    .fold((f64::INFINITY, 0), |a, c| if c.0 < a.0 { c } else { a });
                assert!(
                    (loss - direct.0).abs() < 1e-5,
                    "fov {} loss {loss} direct {}",
                    tpl.fov,
                    direct.0
                );
                let at_yaw = window_loss(
                    &s.e,
                    &tpl,
                    (yaw + tpl.rot) % AZIMUTHS,
                    screen_cfg.tilt,
                    drops[yaw],
                );
                assert!(
                    (at_yaw - direct.0).abs() < 1e-5,
                    "the chosen yaw is a best window"
                );
            }
        }
    }
}
