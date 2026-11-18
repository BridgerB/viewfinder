//! The horizon from one place: for each whole degree of azimuth (0 north, clockwise), the
//! highest elevation angle the ground reaches, with the earth's curve taken off, and the meters
//! to the sample that won.

use crate::config::HorizonConfig;
use crate::dem::{Dem, NODATA};
use crate::java;

/// One sample per degree.
pub const AZIMUTHS: usize = 360;

/// A degree whose ray never touched ground; not a real angle.
pub const MISSING: i16 = i16::MIN;

/// Centidegrees per degree in the stored angles.
pub const CENTI: f64 = 100.0;

/// Meters per degree of latitude, the flat local frame used throughout.
pub const LAT_M: f64 = 111_320.0;

/// One place's horizon: 360 centidegree angles and the meters to each winning sample.
#[derive(Clone, Debug, PartialEq)]
pub struct Horizon {
    pub angles: [i16; AZIMUTHS],
    pub distances: [f32; AZIMUTHS],
}

impl Default for Horizon {
    fn default() -> Self {
        Self { angles: [MISSING; AZIMUTHS], distances: [0.0; AZIMUTHS] }
    }
}

/// The walk along one azimuth. It compares slopes (rise over run) rather than angles: atan only
/// increases, so the winner is the same, and one atan is taken at the end instead of two per
/// step. It starts at `min_m` so a berm beside the camera cannot hide the ridge, grows each
/// step to subtend about `step_deg`, and stops early once even the highest ground could not
/// beat the best so far.
struct Ray {
    lat: f64,
    lon: f64,
    eye: f64,
    lat_m: f64,
    lon_m: f64,
    growth: f64,
}

impl Ray {
    /// The winning angle in degrees (NaN when the ray never touched ground) and its distance.
    #[inline]
    fn walk(&self, dem: &Dem<'_>, cfg: &HorizonConfig, azimuth: usize) -> (f64, f64) {
        let g = &dem.geo;
        let az = (azimuth as f64).to_radians();
        let north_u = az.cos();
        let east_u = az.sin();
        let two_r = 2.0 * cfg.radius_m;
        let headroom = cfg.max_ground_m - self.eye;
        let mut d = cfg.min_distance_m;
        let mut best = f64::NEG_INFINITY;
        let mut best_d = 0.0;
        let mut hit = false;
        loop {
            if d > cfg.max_distance_m || (hit && (headroom - (d * d) / two_r) / d < best) {
                break;
            }
            let drop = (d * d) / two_r;
            let lat2 = self.lat + (d * north_u) / self.lat_m;
            let lon2 = self.lon + (d * east_u) / self.lon_m;
            let col = java::round((lon2 - g.west) / g.dx);
            let row = java::round((lat2 - g.north) / g.dy);
            let z = dem.nearest(row, col);
            if z != NODATA {
                let slope = ((f64::from(z) - self.eye) - drop) / d;
                if slope > best {
                    best = slope;
                    best_d = d;
                }
                hit = true;
            }
            d *= self.growth;
        }
        if hit { (best.atan().to_degrees(), best_d) } else { (f64::NAN, 0.0) }
    }
}

fn centi(degrees: f64) -> i16 {
    if degrees.is_nan() { MISSING } else { java::round(CENTI * degrees) as i16 }
}

/// The horizon from `lat`, `lon` into `out`, or `false` (and `out` untouched) when that cell
/// has no ground.
pub fn view_into(dem: &Dem<'_>, cfg: &HorizonConfig, lat: f64, lon: f64, out: &mut Horizon) -> bool {
    let Some(ground) = dem.elev_at(lat, lon) else {
        return false;
    };
    let ray = Ray {
        lat,
        lon,
        eye: cfg.eye_m + ground,
        lat_m: LAT_M,
        lon_m: LAT_M * lat.to_radians().cos(),
        growth: 1.0 + cfg.step_deg.to_radians().tan(),
    };
    for az in 0..AZIMUTHS {
        let (angle, dist) = ray.walk(dem, cfg, az);
        out.angles[az] = centi(angle);
        out.distances[az] = dist as f32;
    }
    true
}

/// The horizon from `lat`, `lon`, or `None` when that cell has no ground.
#[must_use]
pub fn view(dem: &Dem<'_>, cfg: &HorizonConfig, lat: f64, lon: f64) -> Option<Horizon> {
    let mut out = Horizon::default();
    view_into(dem, cfg, lat, lon, &mut out).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dem::Georef;

    #[test]
    fn a_wall_to_the_east_is_seen_only_to_the_east() {
        // 200 x 200 cells of 30 m (about 0.00027 deg), flat at 0 except a 300 m wall 2 km east.
        let n = 200;
        let cell = 30.0 / LAT_M;
        let mut samples = vec![0_i16; n * n];
        for row in 0..n {
            for col in 170..175 {
                samples[row * n + col] = 300;
            }
        }
        let geo = Georef { north: 0.03, west: 0.0, dx: cell, dy: -cell, width: n, height: n };
        let dem = Dem::new(geo, &samples).unwrap();
        let cfg = HorizonConfig { max_distance_m: 5000.0, ..HorizonConfig::default() };
        let lat = 0.03 - 100.0 * cell;
        let lon = 100.0 * cell;
        let h = view(&dem, &cfg, lat, lon).unwrap();
        let east = f64::from(h.angles[90]) / CENTI;
        let west = f64::from(h.angles[270]) / CENTI;
        assert!(east > 5.0 && east < 10.0, "wall about 300 m up at about 2.1 km: {east}");
        assert!(west < 0.0, "flat ground to the west sits below the eye: {west}");
        assert!((f64::from(h.distances[90]) - 2100.0).abs() < 200.0);
    }
}
