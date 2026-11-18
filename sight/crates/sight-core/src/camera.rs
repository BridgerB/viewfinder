//! Camera geometry. A pixel is a direction in space: roll about the optical axis, then pitch,
//! then yaw. Away from the middle of a wide frame a tilted camera's rows are not lines of equal
//! elevation, so pitch is part of the projection, not a constant offset.

use crate::horizon::{AZIMUTHS, CENTI, MISSING};

/// A skyline pixel in image coordinates (y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel {
    pub x: f64,
    pub y: f64,
}

/// A photograph's size and its skyline.
#[derive(Clone, Debug, PartialEq)]
pub struct Photo {
    pub width: u32,
    pub height: u32,
    pub skyline: Vec<Pixel>,
}

/// Focal length in pixels for a horizontal field of view.
#[must_use]
pub fn focal(width: f64, fov: f64) -> f64 {
    width / 2.0 / (fov / 2.0).to_radians().tan()
}

/// Azimuth offset from the optical axis and elevation above level, in degrees, of pixel `x`, `y`.
#[inline]
#[must_use]
pub fn ray(width: f64, height: f64, f: f64, roll: f64, pitch: f64, x: f64, y: f64) -> (f64, f64) {
    let rx = x - width / 2.0;
    let ry = height / 2.0 - y;
    let c = roll.to_radians().cos();
    let s = roll.to_radians().sin();
    let xr = rx * c - ry * s;
    let yr = rx * s + ry * c;
    let cp = pitch.to_radians().cos();
    let sp = pitch.to_radians().sin();
    let up = yr * cp + f * sp;
    let forward = f * cp - yr * sp;
    (xr.atan2(forward).to_degrees(), up.atan2(xr.hypot(forward)).to_degrees())
}

/// The skyline seen through one field of view, roll and pitch: parallel arrays of azimuth
/// offset and elevation.
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub fov: f64,
    pub roll: f64,
    pub pitch: f64,
    pub offsets: Vec<f64>,
    pub elevs: Vec<f64>,
}

impl Camera {
    #[must_use]
    pub fn new(photo: &Photo, fov: f64, roll: f64, pitch: f64) -> Self {
        let (w, h) = (f64::from(photo.width), f64::from(photo.height));
        let f = focal(w, fov);
        let (offsets, elevs) = photo.skyline.iter().map(|p| ray(w, h, f, roll, pitch, p.x, p.y)).unzip();
        Self { fov, roll, pitch, offsets, elevs }
    }
}

/// One field of view and roll, projected at every whole degree of pitch from `pitch_low`.
#[derive(Clone, Debug, PartialEq)]
pub struct Lens {
    pub fov: f64,
    pub roll: f64,
    pub pitch_low: i32,
    /// Cameras at pitch_low, pitch_low + 1, …
    pub tilts: Vec<Camera>,
    /// Index of the level camera in `tilts`.
    pub level: usize,
    /// Mean cosine of the level camera's offsets: lifting the camera a degree lifts a column
    /// that far off the axis by its cosine of a degree.
    pub kappa: f64,
}

impl Lens {
    #[must_use]
    pub fn new(photo: &Photo, fov: f64, roll: f64, pitch_low: i32, pitch_high: i32) -> Self {
        let tilts: Vec<Camera> =
            (pitch_low..=pitch_high).map(|p| Camera::new(photo, fov, roll, f64::from(p))).collect();
        let level = (-pitch_low).max(0) as usize;
        let offsets = &tilts[level.min(tilts.len().saturating_sub(1))].offsets;
        let sum = offsets.iter().fold(0.0, |s, o| s + o.to_radians().cos());
        let kappa = sum / offsets.len().max(1) as f64;
        Self { fov, roll, pitch_low, tilts, level, kappa }
    }

    /// The camera pitched nearest to `degrees`, or `None` outside the pitches tried.
    #[must_use]
    pub fn tilt_index(&self, degrees: f64) -> Option<usize> {
        if degrees.is_nan() {
            return None;
        }
        let i = crate::java::round(degrees) - i64::from(self.pitch_low);
        (0..self.tilts.len() as i64).contains(&i).then_some(i as usize)
    }
}

/// Every field of view and roll, in that nesting order.
#[must_use]
pub fn lenses(photo: &Photo, fovs: &[f64], rolls: &[f64], pitch_low: i32, pitch_high: i32) -> Vec<Lens> {
    fovs.iter()
        .flat_map(|&fov| rolls.iter().map(move |&roll| (fov, roll)))
        .map(|(fov, roll)| Lens::new(photo, fov, roll, pitch_low, pitch_high))
        .collect()
}

/// A camera pose to draw a horizon through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub yaw: f64,
    pub fov: f64,
    pub roll: f64,
    pub pitch: f64,
}

/// The horizon drawn through a camera: (x, y) pixels, one per degree of azimuth that lands
/// inside the frame. This is `ray` run backwards: yaw, then pitch, then roll.
#[must_use]
pub fn outline(angles: &[i16], pose: Pose, width: f64, height: f64) -> Vec<(f64, f64)> {
    let yaw = pose.yaw as i64;
    let roll = pose.roll.to_radians();
    let pitch = pose.pitch.to_radians();
    let f = focal(width, pose.fov);
    let (cr, sr, cp, sp) = (roll.cos(), roll.sin(), pitch.cos(), pitch.sin());
    let reach = (pose.fov / 2.0 + 10.0).ceil() as i64;
    (-reach..=reach)
        .filter_map(|step| {
            let c = angles[(yaw + step).rem_euclid(AZIMUTHS as i64) as usize];
            if c == MISSING {
                return None;
            }
            let e = (f64::from(c) / CENTI).to_radians();
            let a = (step as f64).to_radians();
            let right = e.cos() * a.sin();
            let up = e.sin();
            let forward = e.cos() * a.cos();
            let lifted = up * cp - forward * sp;
            let depth = forward * cp + up * sp;
            if depth <= 0.0 {
                return None;
            }
            let xr = (f * right) / depth;
            let yr = (f * lifted) / depth;
            let x = width / 2.0 + (xr * cr + yr * sr);
            let y = height / 2.0 - (yr * cr - xr * sr);
            (0.0..=width).contains(&x).then_some((x, y))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_inverts_ray() {
        let (w, h) = (2000.0, 1500.0);
        let mut angles = [0_i16; AZIMUTHS];
        for (az, a) in angles.iter_mut().enumerate() {
            *a = crate::java::round(CENTI * (3.0 + 2.0 * (3.0 * az as f64).to_radians().sin())) as i16;
        }
        for fov in [40.0, 70.0, 100.0] {
            for roll in [-2.0, 0.0, 2.0] {
                for pitch in [-3.0, 0.0, 12.0] {
                    let pose = Pose { yaw: 120.0, fov, roll, pitch };
                    let px = outline(&angles, pose, w, h);
                    assert!(!px.is_empty());
                    for (x, y) in px {
                        let (off, elev) = ray(w, h, focal(w, fov), roll, pitch, x, y);
                        let az = 120.0 + off;
                        let whole = az.round();
                        assert!((az - whole).abs() < 1e-6, "on a whole degree");
                        let want = f64::from(angles[(whole as i64).rem_euclid(360) as usize]) / CENTI;
                        assert!((elev - want).abs() < 1e-6, "elevation round-trips");
                    }
                }
            }
        }
    }
}
