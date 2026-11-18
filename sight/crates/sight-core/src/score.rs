//! The full search at one place: for each lens, every requested yaw; for each yaw the tilt that
//! puts the skyline on the horizon, then a trimmed error in degrees. Sums are taken in the same
//! order as the reference, so results match it to the last bit where the math library agrees.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::camera::{Camera, Lens};
use crate::config::ScoreConfig;
use crate::dem::Dem;
use crate::field::Field;
use crate::horizon::{AZIMUTHS, CENTI, LAT_M, MISSING};

/// The best camera at one place: what a search returns, best (lowest loss) first.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    /// Index of the place in the field it was scored from.
    pub index: usize,
    pub lat: f64,
    pub lon: f64,
    /// Ground elevation, m.
    pub elev: f64,
    pub yaw: f64,
    pub fov: f64,
    pub roll: f64,
    pub pitch: f64,
    pub rmse: f64,
    pub gsigma: f64,
    pub hsigma: f64,
    pub corr: f64,
    pub loss: f64,
    pub used: usize,
}

/// One yaw's fit, before the lens and foreground terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measure {
    pub rmse: f64,
    /// Residual tilt left after the camera's own pitch.
    pub pitch: f64,
    pub used: f64,
    pub gsigma: f64,
    pub hsigma: f64,
    pub corr: f64,
    pub loss: f64,
}

/// Reusable per-thread buffers, sized to the skyline.
#[derive(Clone, Debug, Default)]
pub struct Scratch {
    gs: Vec<f64>,
    hs: Vec<f64>,
    ds: Vec<f64>,
    diffs: Vec<f64>,
    work: Vec<f64>,
    falls: Vec<f64>,
}

impl Scratch {
    fn fit(&mut self, n: usize) {
        for v in [&mut self.gs, &mut self.hs, &mut self.ds, &mut self.diffs, &mut self.work] {
            v.clear();
            v.resize(n, 0.0);
        }
    }
}

/// Azimuth into [0, 360). One step is enough: yaw is a whole degree and offsets are within a
/// field of view.
#[inline]
fn wrap(azimuth: f64) -> f64 {
    if azimuth >= 360.0 {
        azimuth - 360.0
    } else if azimuth < 0.0 {
        azimuth + 360.0
    } else {
        azimuth
    }
}

/// Centidegrees of the horizon at `azimuth`, interpolated between the two whole degrees around
/// it; `None` when either is missing.
#[inline]
fn angle_at(angles: &[i16], azimuth: f64) -> Option<f64> {
    let i0 = azimuth as usize;
    let i1 = if i0 == AZIMUTHS - 1 { 0 } else { i0 + 1 };
    let t = azimuth - i0 as f64;
    let (h0, h1) = (angles[i0], angles[i1]);
    if h0 == MISSING || h1 == MISSING {
        return None;
    }
    let (h0, h1) = (f64::from(h0), f64::from(h1));
    Some(h0 + t * (h1 - h0))
}

/// Fewest columns that must land on the horizon for a fit to count.
fn min_columns(n: usize) -> usize {
    24.max((0.45 * n as f64) as usize)
}

/// The k-th smallest of `values` (reordering them): exactly the value a sort would put at k.
fn select(values: &mut [f64], k: usize) -> f64 {
    let (_, v, _) = values.select_nth_unstable_by(k, f64::total_cmp);
    *v
}

/// Largest `rank` that `nth_largest` serves; beyond it `measure` falls back to `select`.
const TOP_SLOTS: usize = 8;

/// The `rank`-th largest (1-based, 1 ≤ rank ≤ `TOP_SLOTS`) of `values`, which must be finite and
/// at least `rank` long: the `rank` largest are kept in a descending buffer.
fn nth_largest(values: impl Iterator<Item = f64>, rank: usize) -> f64 {
    let mut top = [f64::NEG_INFINITY; TOP_SLOTS];
    let top = &mut top[..rank];
    for v in values {
        if v > top[rank - 1] {
            let mut i = rank - 1;
            while i > 0 && top[i - 1] < v {
                top[i] = top[i - 1];
                i -= 1;
            }
            top[i] = v;
        }
    }
    top[rank - 1]
}

/// Fits one camera at one yaw. Pitch is the mean residual of the inliers; the worst
/// `1 - keep_fraction` of columns (a pole, a tree) are dropped first. `None` when too few
/// columns reach the horizon or the photograph's relief is under `min_relief`.
pub fn measure(
    angles: &[i16],
    distances: &[f32],
    cam: &Camera,
    yaw: usize,
    cfg: &ScoreConfig,
    s: &mut Scratch,
) -> Option<Measure> {
    let n = cam.offsets.len();
    s.fit(n);
    let mut used = 0;
    for i in 0..n {
        let az = wrap(yaw as f64 + cam.offsets[i]);
        let Some(h) = angle_at(angles, az) else { continue };
        let g = cam.elevs[i];
        let hd = h / CENTI;
        s.gs[used] = g;
        s.hs[used] = hd;
        s.ds[used] = f64::from(distances[az as usize]);
        s.diffs[used] = hd - g;
        used += 1;
    }
    if used < min_columns(n) {
        return None;
    }
    let keep = 1.max((cfg.keep_fraction * used as f64) as usize);
    s.work[..used].copy_from_slice(&s.diffs[..used]);
    let median = select(&mut s.work[..used], used / 2);
    // The cutoff is the (keep − 1)-th smallest deviation, i.e. the (used − keep + 1)-th largest:
    // with 2% trimmed that is about the 4th largest, which a few-slot running buffer finds in one
    // pass, with exactly the value a sort would give.
    let rank = used - keep + 1;
    let cutoff = if rank <= TOP_SLOTS {
        nth_largest(s.diffs[..used].iter().map(|r| (r - median).abs()), rank)
    } else {
        for i in 0..used {
            s.work[i] = (s.diffs[i] - median).abs();
        }
        select(&mut s.work[..used], keep - 1)
    };
    let (mut k, mut sr, mut srr, mut sg, mut sh, mut sgg, mut shh, mut sgh, mut near, mut floor) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..used {
        let r = s.diffs[i];
        if (r - median).abs() <= cutoff {
            let (g, h, d) = (s.gs[i], s.hs[i], s.ds[i]);
            k += 1.0;
            sr += r;
            srr += r * r;
            sg += g;
            sh += h;
            sgg += g * g;
            shh += h * h;
            sgh += g * h;
            if d > 0.0 && d < cfg.near_m {
                near += 1.0;
            }
            if d > 0.0 && d < cfg.floor_m {
                floor += 1.0;
            }
        }
    }
    let pitch = sr / k;
    let rmse = (srr / k - pitch * pitch).max(0.0).sqrt();
    let mg = sg / k;
    let mh = sh / k;
    let vg = (sgg / k - mg * mg).max(0.0);
    let vh = (shh / k - mh * mh).max(0.0);
    let gsigma = vg.sqrt();
    let hsigma = vh.sqrt();
    let denom = (vg * vh).sqrt();
    let corr = if denom < 1.0e-6 { 0.0 } else { ((sgh / k - mg * mh) / denom).clamp(-1.0, 1.0) };
    // One step of the coarse field of view moves the relief by about this much; a gap inside
    // it is the lens grid, not a wrong distance.
    let scale = ((hsigma.max(1.0e-3) / gsigma.max(1.0e-3)).ln().abs() - 0.10).max(0.0);
    let slope = (near / k - cfg.near_keep).max(0.0);
    let loss = rmse + scale + 2.0 * (1.0 - corr) + cfg.near_weight * slope + cfg.floor_weight * (floor / k);
    if gsigma < cfg.min_relief {
        return None;
    }
    Some(Measure { rmse, pitch, used: k, gsigma, hsigma, corr, loss })
}

/// How far the level camera must tilt for its skyline to sit on the horizon at `yaw`, in
/// degrees: the mean gap, seen through kappa. `None` when too few columns reach the horizon.
fn tilt_guess(angles: &[i16], level: &Camera, yaw: usize, kappa: f64) -> Option<f64> {
    let n = level.offsets.len();
    let (mut k, mut sum) = (0_usize, 0.0);
    for i in 0..n {
        if let Some(h) = angle_at(angles, wrap(yaw as f64 + level.offsets[i])) {
            k += 1;
            sum += h / CENTI - level.elevs[i];
        }
    }
    (k >= min_columns(n)).then(|| sum / (k as f64 * kappa))
}

/// The tilted camera of `lens` whose skyline sits on the horizon at `yaw`, and its fit. The
/// level camera guesses the tilt; when more than `pitch_slack` is left over the wide frame bent
/// the guess, so the camera tilts once more by what was left.
fn fit<'l>(
    angles: &[i16],
    distances: &[f32],
    lens: &'l Lens,
    yaw: usize,
    cfg: &ScoreConfig,
    s: &mut Scratch,
) -> Option<(&'l Camera, Measure)> {
    let level = &lens.tilts[lens.level];
    let first = lens.tilt_index(tilt_guess(angles, level, yaw, lens.kappa)?)?;
    let cam = &lens.tilts[first];
    let m = measure(angles, distances, cam, yaw, cfg, s)?;
    if m.pitch.abs() <= cfg.pitch_slack {
        return Some((cam, m));
    }
    let second = lens.tilt_index(cam.pitch + m.pitch / lens.kappa)?;
    let cam = &lens.tilts[second];
    let m = measure(angles, distances, cam, yaw, cfg, s)?;
    (m.pitch.abs() <= cfg.pitch_slack).then_some((cam, m))
}

/// Degrees added when the ground ahead falls more than the allowance below the camera.
#[inline]
#[must_use]
pub fn drop_penalty(fall: f64, cfg: &ScoreConfig) -> f64 {
    ((fall - cfg.drop_allow_m) / cfg.drop_scale_m).max(0.0)
}

/// Meters the ground `foreground_m` ahead is below the camera, per degree of azimuth; positive
/// when the earth drops. Off the grid counts as level: missing ground is not a canyon.
pub fn falls_into(dem: &Dem<'_>, lat: f64, lon: f64, ground: f64, cfg: &ScoreConfig, out: &mut Vec<f64>) {
    out.clear();
    let lon_m = LAT_M * lat.to_radians().cos();
    let dist = cfg.foreground_m;
    out.extend((0..AZIMUTHS).map(|az| {
        let r = (az as f64).to_radians();
        dem.elev_at(lat + (dist * r.cos()) / LAT_M, lon + (dist * r.sin()) / lon_m).map_or(0.0, |z| ground - z)
    }));
}

/// A lens's best yaw at one place.
#[derive(Clone, Copy, Debug)]
struct Hit {
    loss: f64,
    yaw: usize,
    pitch: f64,
    m: Measure,
    fov: f64,
    roll: f64,
}

fn best_yaw(
    angles: &[i16],
    distances: &[f32],
    lens: &Lens,
    falls: &[f64],
    yaws: &[u16],
    cfg: &ScoreConfig,
    s: &mut Scratch,
) -> Option<Hit> {
    let lens_scale = cfg.ref_fov / lens.fov - 1.0;
    let mut best: Option<Hit> = None;
    for &yaw in yaws {
        let yaw = usize::from(yaw);
        let Some((cam, m)) = fit(angles, distances, lens, yaw, cfg, s) else { continue };
        let loss = m.loss + m.rmse * lens_scale + drop_penalty(falls[yaw], cfg);
        if loss < best.map_or(f64::INFINITY, |b| b.loss) {
            best = Some(Hit { loss, yaw, pitch: cam.pitch + m.pitch, m, fov: cam.fov, roll: cam.roll });
        }
    }
    best
}

/// The lowest-loss hit, with the reference's tie rule: of the first two the later wins a tie,
/// after that the earlier one stays.
fn min_hit(hits: &[Hit]) -> Option<Hit> {
    let (&first, rest) = hits.split_first()?;
    let Some((&second, rest)) = rest.split_first() else { return Some(first) };
    let start = if first.loss < second.loss { first } else { second };
    Some(rest.iter().fold(start, |cur, &h| if h.loss < cur.loss { h } else { cur }))
}

/// Which yaws to try at a place.
#[derive(Clone, Copy, Debug)]
pub enum Yaws<'a> {
    /// Every whole degree.
    All,
    /// Per place index, the given list.
    PerPlace(&'a [Vec<u16>]),
}

/// The best camera at every place of `field` that scored, best first (stable: ties keep field
/// order).
#[must_use]
pub fn surface(field: &Field, dem: &Dem<'_>, lenses: &[Lens], yaws: Yaws<'_>, cfg: &ScoreConfig) -> Vec<Place> {
    let every: Vec<u16> = (0..AZIMUTHS as u16).collect();
    let mut rows: Vec<Place> = (0..field.len())
        .into_par_iter()
        .map_init(Scratch::default, |s, i| {
            let ground = f64::from(field.grounds[i]);
            let mut falls = std::mem::take(&mut s.falls);
            falls_into(dem, field.lats[i], field.lons[i], ground, cfg, &mut falls);
            let list: &[u16] = match yaws {
                Yaws::All => &every,
                Yaws::PerPlace(per) => &per[i],
            };
            let hits: Vec<Hit> = lenses
                .iter()
                .filter_map(|lens| best_yaw(field.angles_of(i), field.distances_of(i), lens, &falls, list, cfg, s))
                .collect();
            s.falls = falls;
            min_hit(&hits).map(|h| Place {
                index: i,
                lat: field.lats[i],
                lon: field.lons[i],
                elev: ground,
                yaw: h.yaw as f64,
                fov: h.fov,
                roll: h.roll,
                pitch: h.pitch,
                rmse: h.m.rmse,
                gsigma: h.m.gsigma,
                hsigma: h.m.hsigma,
                corr: h.m.corr,
                loss: h.loss,
                used: h.m.used as usize,
            })
        })
        .flatten()
        .collect();
    rows.sort_by(|a, b| a.loss.total_cmp(&b.loss));
    rows
}

/// Whole degrees within `reach` of `center`, wrapped.
#[must_use]
pub fn yaws_around(center: f64, reach: i64) -> Vec<u16> {
    let c = crate::java::round(center);
    (-reach..=reach).map(|d| (c + d).rem_euclid(AZIMUTHS as i64) as u16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn horizon_from(f: impl Fn(f64) -> f64) -> ([i16; AZIMUTHS], [f32; AZIMUTHS]) {
        let mut a = [0_i16; AZIMUTHS];
        for (az, v) in a.iter_mut().enumerate() {
            *v = crate::java::round(CENTI * f(az as f64)) as i16;
        }
        (a, [5000.0; AZIMUTHS])
    }

    #[test]
    fn select_is_the_sorted_value() {
        let mut rng = crate::java::JavaRandom::new(3);
        for _ in 0..500 {
            let n = 1 + rng.next_int(200) as usize;
            let vals: Vec<f64> = (0..n).map(|_| f64::from(rng.next_int(41) - 20)).collect();
            let mut sorted = vals.clone();
            sorted.sort_by(f64::total_cmp);
            let k = rng.next_int(n as i32) as usize;
            assert_eq!(select(&mut vals.clone(), k), sorted[k]);
        }
    }

    #[test]
    fn nth_largest_is_the_sorted_value() {
        let mut rng = crate::java::JavaRandom::new(4);
        for _ in 0..2000 {
            let n = 8 + rng.next_int(200) as usize;
            let vals: Vec<f64> = (0..n).map(|_| f64::from(rng.next_int(21)) / 4.0).collect();
            let rank = 1 + rng.next_int(TOP_SLOTS as i32) as usize;
            let mut sorted = vals.clone();
            sorted.sort_by(f64::total_cmp);
            assert_eq!(nth_largest(vals.iter().copied(), rank), sorted[n - rank]);
        }
    }

    #[test]
    fn pitch_absorbs_a_constant_offset() {
        let shape = |az: f64| 4.0 + 2.0 * (6.0 * az).to_radians().sin();
        let (angles, dists) = horizon_from(shape);
        let offsets: Vec<f64> = (-40..40).map(f64::from).collect();
        let elevs: Vec<f64> = offsets
            .iter()
            .map(|&o| f64::from(angles[(o as i64).rem_euclid(360) as usize]) / CENTI - 1.5)
            .collect();
        let cam = Camera { fov: 80.0, roll: 0.0, pitch: 0.0, offsets, elevs };
        let m = measure(&angles, &dists, &cam, 0, &ScoreConfig::default(), &mut Scratch::default()).unwrap();
        assert!((m.pitch - 1.5).abs() < 0.02);
        assert!(m.rmse < 0.02);
        assert!((m.corr - 1.0).abs() < 1e-3);
    }

    #[test]
    fn a_flat_photograph_cannot_place_a_camera() {
        let (angles, dists) = horizon_from(|_| 2.0);
        let offsets: Vec<f64> = (-40..40).map(f64::from).collect();
        let cam = Camera { fov: 80.0, roll: 0.0, pitch: 0.0, elevs: vec![2.0; offsets.len()], offsets };
        assert!(measure(&angles, &dists, &cam, 0, &ScoreConfig::default(), &mut Scratch::default()).is_none());
    }

    #[test]
    fn min_hit_tie_rule() {
        let h = |loss, yaw| Hit {
            loss,
            yaw,
            pitch: 0.0,
            m: Measure { rmse: 0.0, pitch: 0.0, used: 0.0, gsigma: 0.0, hsigma: 0.0, corr: 0.0, loss },
            fov: 0.0,
            roll: 0.0,
        };
        assert_eq!(min_hit(&[h(1.0, 0), h(1.0, 1)]).unwrap().yaw, 1, "later of the first two");
        assert_eq!(min_hit(&[h(2.0, 0), h(1.0, 1), h(1.0, 2)]).unwrap().yaw, 1, "then the earlier stays");
    }
}
