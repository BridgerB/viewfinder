---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/config.rs#pub struct Config {
  - sight/crates/sight-core/src/config.rs#pub fn with_known_fov(mut self, fov: f64) -> Self {
  - sight/crates/sight-core/src/config.rs#fovs: (0..16).map(|i| 40.0 + 5.0 * f64::from(i)).collect(),
---

# Config

`Config` gathers five structs, and `Default` is the tuned configuration. Each field's doc comment says what wrong answer the number prevents; keep those comments true when tuning.

| Struct | What it controls | Key values |
|---|---|---|
| `HorizonConfig` | the ray walk ([[locator/data/horizon-walk]]) | eye 2 m, start 400 m, stop 50 km, step ≈0.4°, R = 7/6 × 6371 km, ceiling 4123 m |
| `ScoreConfig` | the full search ([[locator/scoring/loss]]) | 16 lenses 40–115° by 5°, rolls −2/0/2°, pitch −5..30°, slack 1°, keep 98%, ref FOV 70° |
| `ScreenConfig` | the screen ([[locator/screen/screen-templates]]) | tilts −3/3/9/15/21°, roll slope cap 0.055, yaw step fov/25, keep max(200, 3%), reach ±8° |
| `RefineConfig` | refinement ([[locator/search/refinement]]) | (60 places, 250 m, 500 m), then (15, 100 m, 300 m) |
| `SkyConfig` | the tracer ([[locator/sky/skyline-trace]]) | rock run 160 px, 160 columns |

`with_known_fov(fov)` replaces the 16 lenses with `fov − 1.5, fov, fov + 1.5` when EXIF gives the focal length. With three or fewer lenses the screen uses all of them instead of every other one.
