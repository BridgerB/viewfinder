//! The screen: a cheap score for every place so only the likely ones get the full search. Per
//! lens and tilt the photograph's skyline becomes a template sampled once per whole degree; it
//! slides around the place's horizon, and each window's error, correlation and relief come from
//! running sums plus one multiply-add pass. A capped straight-line fit to the residual absorbs a
//! small roll.

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
    /// The same elevations as f32, for the cross-term kernel.
    elevs32: Vec<f32>,
    // Constants of the window algebra, computed once.
    g1: f64,
    g2: f64,
    gk: f64,
    k1: f64,
    sxx: f64,
    mg: f64,
    vg: f64,
    gsigma: f64,
    lens_scale: f64,
    step: usize,
}

impl Template {
    #[must_use]
    pub fn new(photo: &Photo, fov: f64, pitch: f64, score: &ScoreConfig, screen: &ScreenConfig) -> Self {
        let cam = Camera::new(photo, fov, 0.0, pitch);
        let (offsets, geoms) = (&cam.offsets, &cam.elevs);
        let n = offsets.len();
        // Fewer than two columns cannot be interpolated: an empty template, which slide rejects.
        let (start, end) = if n < 2 { (0, -1) } else { (offsets[0].ceil() as i64, offsets[n - 1].floor() as i64) };
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
                let t = if o1 > o0 { (target - o0) / (o1 - o0) } else { 0.0 };
                geoms[j] + t.clamp(0.0, 1.0) * (geoms[j + 1] - geoms[j])
            })
            .collect();
        let g1 = elevs.iter().fold(0.0, |s, &v| s + v);
        let g2 = elevs.iter().fold(0.0, |s, &v| s + v * v);
        let gk = elevs.iter().enumerate().fold(0.0, |s, (k, &v)| s + k as f64 * v);
        let mf = m as f64;
        let k1 = (mf * (mf - 1.0)) / 2.0;
        let k2 = ((mf - 1.0) * mf * (2.0 * mf - 1.0)) / 6.0;
        let sxx = k2 - (k1 * k1) / mf;
        let mg = g1 / mf;
        let vg = (g2 / mf - mg * mg).max(0.0);
        Self {
            fov,
            start,
            elevs32: elevs.iter().map(|&v| v as f32).collect(),
            elevs,
            g1,
            g2,
            gk,
            k1,
            sxx,
            mg,
            vg,
            gsigma: vg.sqrt(),
            lens_scale: score.ref_fov / fov,
            step: 1.max((fov as i64 / screen.yaw_span) as usize),
        }
    }
}

/// Every template the screen tries: every other lens of the full list (all of them when the
/// lens is known), at each screen pitch.
#[must_use]
pub fn templates(photo: &Photo, score: &ScoreConfig, screen: &ScreenConfig) -> Vec<Template> {
    let fovs: Vec<f64> =
        if score.fovs.len() <= 3 { score.fovs.clone() } else { score.fovs.iter().step_by(2).copied().collect() };
    fovs.iter()
        .flat_map(|&fov| screen.pitches.iter().map(move |&p| (fov, p)))
        .map(|(fov, pitch)| Template::new(photo, fov, pitch, score, screen))
        .collect()
}

/// One place's horizon twice round (so a window never wraps), in degrees, with prefix sums of
/// the value, its square, its index times the value, and missing degrees.
#[derive(Clone, Debug)]
pub struct Sums {
    e: Vec<f64>,
    e32: Vec<f32>,
    p0: Vec<f64>,
    p2: Vec<f64>,
    pj: Vec<f64>,
    pn: Vec<f64>,
}

impl Default for Sums {
    fn default() -> Self {
        Self {
            e: vec![0.0; 2 * AZIMUTHS],
            e32: vec![0.0; 2 * AZIMUTHS],
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
        for j in 0..2 * AZIMUTHS {
            let c = angles[j % AZIMUTHS];
            let gone = c == MISSING;
            let v = if gone { 0.0 } else { f64::from(c) / CENTI };
            self.e[j] = v;
            self.e32[j] = v as f32;
            self.p0[j + 1] = self.p0[j] + v;
            self.p2[j + 1] = self.p2[j] + v * v;
            self.pj[j + 1] = self.pj[j] + j as f64 * v;
            self.pn[j + 1] = self.pn[j] + if gone { 1.0 } else { 0.0 };
        }
    }
}

/// Σ g[k]·e[b+k]: most of the screen's time. Eight independent running sums over fixed-size
/// chunks, so LLVM keeps them in vector registers and issues fused multiply-adds side by side
/// instead of waiting on one sum. The order differs from a plain sum only in rounding.
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

/// Σ g[k]·e[b+k] in f32: sixteen lanes, four per NEON register, summed to f64 at the end.
#[inline]
#[must_use]
pub fn dot32(g: &[f32], e: &[f32], b: usize) -> f64 {
    const LANES: usize = 16;
    let e = &e[b..b + g.len()];
    let mut acc = [0.0_f32; LANES];
    let (gc, gt) = g.as_chunks::<LANES>();
    let (ec, et) = e.as_chunks::<LANES>();
    for (gs, es) in gc.iter().zip(ec) {
        for i in 0..LANES {
            acc[i] = gs[i].mul_add(es[i], acc[i]);
        }
    }
    let mut sum: f64 = acc.iter().map(|&v| f64::from(v)).sum();
    for (x, y) in gt.iter().zip(et) {
        sum += f64::from(*x) * f64::from(*y);
    }
    sum
}

/// The screen loss of `tpl` at `yaw` when it beats `best`; `None` when it cannot, or its window
/// touches a missing degree. The relief term is never negative, so once the rest of the loss
/// reaches `best` its logarithm is skipped: the answer is the same, without the cost.
#[inline]
fn slide_yaw(
    s: &Sums,
    tpl: &Template,
    falls: &[f64],
    yaw: usize,
    tilt_cap: f64,
    score: &ScoreConfig,
    best: f64,
) -> Option<f64> {
    let m = tpl.elevs.len();
    let mf = m as f64;
    let b = (yaw as i64 + tpl.start + 720).rem_euclid(AZIMUTHS as i64) as usize;
    let end = b + m;
    if s.pn[end] - s.pn[b] != 0.0 {
        return None;
    }
    let s1 = s.p0[end] - s.p0[b];
    let s2 = s.p2[end] - s.p2[b];
    let sk = (s.pj[end] - s.pj[b]) - b as f64 * s1;
    let cross = dot32(&tpl.elevs32, &s.e32, b);
    let sr = s1 - tpl.g1;
    let srr = (s2 - 2.0 * cross) + tpl.g2;
    let skr = sk - tpl.gk;
    let sxr = skr - (tpl.k1 * sr) / mf;
    // The tilt a small roll puts on the skyline, fitted to the residual and capped, then applied
    // to the photograph's skyline for every term.
    let tilt = if tpl.sxx > 0.0 { (-tilt_cap).max(tilt_cap.min(sxr / tpl.sxx)) } else { 0.0 };
    let rss = (srr - (sr * sr) / mf) + -2.0 * tilt * sxr + tilt * tilt * tpl.sxx;
    let mse = (rss / mf).max(0.0);
    let mh = s1 / mf;
    let vh = (s2 / mf - mh * mh).max(0.0);
    let kbar = tpl.k1 / mf;
    let cgk = tpl.gk / mf - tpl.mg * kbar;
    let chk = sk / mf - mh * kbar;
    let vt = (tpl.vg + tilt * tilt * (tpl.sxx / mf) + 2.0 * tilt * cgk).max(0.0);
    let denom = (vt * vh).sqrt();
    let corr = if denom < 1.0e-9 { 0.0 } else { ((cross / mf - mh * tpl.mg) + tilt * chk) / denom };
    let fit = mse.sqrt() * tpl.lens_scale;
    let shape = 2.0 * (1.0 - corr);
    let drop = drop_penalty(falls[yaw], score);
    if fit + shape + drop >= best {
        return None;
    }
    let scale = ((vh.sqrt().max(1.0e-3) / vt.sqrt().max(1.0e-3)).ln().abs() - 0.10).max(0.0);
    let loss = fit + scale + shape + drop;
    (loss < best).then_some(loss)
}

/// The best (loss, yaw) of one template at one place: every `step` degrees, then the degrees
/// between around the best. `None` when the template is too short or flat, or no window fits.
fn slide(s: &Sums, tpl: &Template, falls: &[f64], score: &ScoreConfig, screen: &ScreenConfig) -> Option<(f64, usize)> {
    if tpl.elevs.len() < 8 || tpl.gsigma < score.min_relief {
        return None;
    }
    let consider = |best: &mut (f64, usize), yaw: usize| {
        if let Some(loss) = slide_yaw(s, tpl, falls, yaw, screen.tilt, score, best.0) {
            *best = (loss, yaw);
        }
    };
    let mut best = (f64::INFINITY, 0);
    let step = tpl.step;
    for j in 0..AZIMUTHS / step {
        consider(&mut best, j * step);
    }
    if best.0.is_finite() {
        let center = best.1;
        for d in 1..step {
            consider(&mut best, (center + d) % AZIMUTHS);
            consider(&mut best, (center + (AZIMUTHS - d)) % AZIMUTHS);
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

#[derive(Default)]
struct ScreenScratch {
    sums: Sums,
    falls: Vec<f64>,
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
            falls_into(dem, field.lats[i], field.lons[i], f64::from(field.grounds[i]), score, &mut sc.falls);
            let mut best: Option<ScreenRow> = None;
            for tpl in templates {
                if let Some((loss, yaw)) = slide(&sc.sums, tpl, &sc.falls, score, screen)
                    && loss < best.map_or(f64::INFINITY, |b| b.loss)
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

    #[test]
    fn dot_matches_a_plain_sum() {
        let mut rng = crate::java::JavaRandom::new(5);
        for _ in 0..200 {
            let m = 1 + rng.next_int(130) as usize;
            let b = rng.next_int(300) as usize;
            let g: Vec<f64> = (0..m).map(|_| f64::from(rng.next_int(2000)) / 100.0 - 10.0).collect();
            let e: Vec<f64> = (0..720).map(|_| f64::from(rng.next_int(2000)) / 100.0 - 10.0).collect();
            let plain: f64 = (0..m).map(|k| g[k] * e[b + k]).sum();
            assert!((plain - dot(&g, &e, b)).abs() < 1e-9);
        }
    }

    #[test]
    fn dot32_is_close_to_the_f64_sum() {
        let mut rng = crate::java::JavaRandom::new(9);
        for _ in 0..200 {
            let m = 8 + rng.next_int(110) as usize;
            let b = rng.next_int(300) as usize;
            let g: Vec<f64> = (0..m).map(|_| f64::from(rng.next_int(1500)) / 100.0).collect();
            let e: Vec<f64> = (0..720).map(|_| f64::from(rng.next_int(1500)) / 100.0).collect();
            let g32: Vec<f32> = g.iter().map(|&v| v as f32).collect();
            let e32: Vec<f32> = e.iter().map(|&v| v as f32).collect();
            let exact = dot(&g, &e, b);
            assert!((dot32(&g32, &e32, b) - exact).abs() < 1e-4 * exact.abs().max(1.0));
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
            let missing = (b..b + m).filter(|j| angles[j % AZIMUTHS] == MISSING).count() as f64;
            assert_eq!(s.pn[b + m] - s.pn[b], missing);
        }
    }
}
