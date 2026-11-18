//! Ground positions derived from a horizon: where a camera's skyline actually sits.

use crate::horizon::{AZIMUTHS, LAT_M};

/// The ground the skyline sits on: for every whole degree inside the camera's field of view,
/// (lat, lon) of the sample that won that degree. A degree that never touched ground is left out.
#[must_use]
pub fn ridge(distances: &[f32], lat: f64, lon: f64, yaw: f64, fov: f64) -> Vec<(f64, f64)> {
    let lon_m = LAT_M * lat.to_radians().cos();
    let half = (fov / 2.0).ceil() as i64;
    (-half..=half)
        .filter_map(|step| {
            let az = (yaw as i64 + step).rem_euclid(AZIMUTHS as i64);
            let d = f64::from(distances[az as usize]);
            let t = (az as f64).to_radians();
            (d > 0.0).then(|| (lat + (d * t.cos()) / LAT_M, lon + (d * t.sin()) / lon_m))
        })
        .collect()
}
