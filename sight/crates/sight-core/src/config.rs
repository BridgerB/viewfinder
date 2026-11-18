//! Every constant the search uses, named and explained in one place. `Default` is the tuned
//! configuration; each comment says what the number guards against.

use serde::{Deserialize, Serialize};

/// The horizon walk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HorizonConfig {
    /// Meters the camera sits above the bare earth.
    pub eye_m: f64,
    /// The highest ground in Utah (Kings Peak), so a ray stops once nothing farther can win.
    pub max_ground_m: f64,
    /// Closer than this is the slope underfoot, not the ridge in the frame.
    pub min_distance_m: f64,
    /// Past this a ridge is haze, not a skyline.
    pub max_distance_m: f64,
    /// Each step along a ray subtends about this many degrees.
    pub step_deg: f64,
    /// Earth radius with the usual refraction allowance (about 13% of curvature).
    pub radius_m: f64,
}

impl Default for HorizonConfig {
    fn default() -> Self {
        Self {
            eye_m: 2.0,
            max_ground_m: 4123.0,
            min_distance_m: 400.0,
            max_distance_m: 50_000.0,
            step_deg: 0.4,
            radius_m: (6_371_000.0 * 7.0) / 6.0,
        }
    }
}

/// The full trimmed-error search at one place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreConfig {
    /// Lens widths tried when the lens is unknown: a 50 mm lens through a phone's ultrawide.
    pub fovs: Vec<f64>,
    /// Rolls tried, degrees.
    pub rolls: Vec<f64>,
    /// The trimmed error is counted as if the frame spanned this many degrees: a trace is off
    /// by pixels, and a narrow lens makes a pixel a smaller angle.
    pub ref_fov: f64,
    /// Lowest pitch tried; a skyline photograph is not aimed at the ground.
    pub pitch_low: i32,
    /// Highest pitch tried; past this the frame is sky.
    pub pitch_high: i32,
    /// Degrees the mean residual may absorb. More than this bends the rows of a wide frame.
    pub pitch_slack: f64,
    /// Share of columns kept: a pole or a tree is dropped, not a tenth of the ridge.
    pub keep_fraction: f64,
    /// Degrees. Below this the photograph is a flat line and cannot place a camera.
    pub min_relief: f64,
    /// Closer than this, once it fills the frame, is the ground the camera stands on.
    pub near_m: f64,
    /// A foothill may cover this share of the window and still be the photograph.
    pub near_keep: f64,
    /// Degrees added when the rest of the window is near ground. Off: it sent a canyon-mouth
    /// photo 51 km away, and with the 3D projection no valley photo needs it.
    pub near_weight: f64,
    /// A hit this close is the walk's first step, the berm beside the camera.
    pub floor_m: f64,
    /// Degrees if that berm is the whole window.
    pub floor_weight: f64,
    /// A valley photograph is still on the flat at this distance ahead.
    pub foreground_m: f64,
    /// Meters of fall allowed before the drop penalty starts: a field is not level.
    pub drop_allow_m: f64,
    /// A fall of this many meters past the allowance costs one degree.
    pub drop_scale_m: f64,
}

impl Default for ScoreConfig {
    fn default() -> Self {
        Self {
            fovs: (0..16).map(|i| 40.0 + 5.0 * f64::from(i)).collect(),
            rolls: vec![-2.0, 0.0, 2.0],
            ref_fov: 70.0,
            pitch_low: -5,
            pitch_high: 30,
            pitch_slack: 1.0,
            keep_fraction: 0.98,
            min_relief: 0.2,
            near_m: 2000.0,
            near_keep: 0.20,
            near_weight: 0.0,
            floor_m: 500.0,
            floor_weight: 2.0,
            foreground_m: 1200.0,
            drop_allow_m: 40.0,
            drop_scale_m: 400.0,
        }
    }
}

/// The screen that every place passes through before the full search.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScreenConfig {
    /// Tilts tried, degrees. Six apart: the full search refines from the nearest.
    pub pitches: Vec<f64>,
    /// Largest skyline slope (degrees of elevation per degree of azimuth) credited to roll:
    /// about three degrees of roll.
    pub tilt: f64,
    /// Yaw step is fov / this (at least 1), then the degrees between around the best.
    pub yaw_span: i64,
    /// Survivors per field: the larger of this and `keep_share` of the places.
    pub keep_min: usize,
    /// Share of a field's places kept for the full search.
    pub keep_share: f64,
    /// Degrees either side of the screen's yaw the full search tries.
    pub reach: i64,
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            pitches: vec![-3.0, 3.0, 9.0, 15.0, 21.0],
            tilt: 0.055,
            yaw_span: 25,
            keep_min: 200,
            keep_share: 0.03,
            reach: 8,
        }
    }
}

/// The refinement passes around the best places.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefineConfig {
    /// (places refined around, grid step m, reach m) for each pass, in order.
    pub passes: Vec<(usize, f64, f64)>,
}

impl Default for RefineConfig {
    fn default() -> Self {
        Self { passes: vec![(60, 250.0, 500.0), (15, 100.0, 300.0)] }
    }
}

/// Skyline tracing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkyConfig {
    /// A hit that returns to sky within this many pixels is a cloud, not the ridge.
    pub rock_run: usize,
    /// Columns the score keeps; the rest are the same ridge.
    pub columns: usize,
}

impl Default for SkyConfig {
    fn default() -> Self {
        Self { rock_run: 160, columns: 160 }
    }
}

/// Everything, as one value a run can record.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub horizon: HorizonConfig,
    pub score: ScoreConfig,
    pub screen: ScreenConfig,
    pub refine: RefineConfig,
    pub sky: SkyConfig,
}

impl Config {
    /// The lens list when the lens is known from the camera's focal length: that width and
    /// 1.5° either side.
    #[must_use]
    pub fn with_known_fov(mut self, fov: f64) -> Self {
        self.score.fovs = vec![fov - 1.5, fov, fov + 1.5];
        self
    }
}
