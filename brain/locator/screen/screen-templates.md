---
type: area
status: verified
verifiedAgainst: working tree on 8a1e79e (screen sweep), 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/screen.rs#pub fn templates(photo: &Photo, score: &ScoreConfig, screen: &ScreenConfig) -> Vec<Template> {
  - sight/crates/sight-core/src/screen.rs#fn sweep(
  - sight/crates/sight-core/src/config.rs#pitches: vec![-3.0, 3.0, 9.0, 15.0, 21.0],
  - sight/PERF.md#Tilted screen templates.
  - sight/PERF.md#Lenses 10° apart in the screen, not 15°.
  - sight/PERF.md#The sweep scores all 360 yaws
---

# Screen templates

A template is the photo's skyline through one lens at one tilt with no roll, resampled to one elevation per whole degree of azimuth offset (linear interpolation between the projected columns, from ceil of the first offset to floor of the last). It has m ≈ fov values. A template shorter than 8 degrees or flatter than 0.2° is skipped.

## What is tried, and why

- **Lenses: every other one of the 16** (40°, 50°, … 110°), 10° apart. At 15° apart a 52° photo fell between the 40° and 55° templates and lost its place (library photo 68 km off): [[decisions/screen-lens-spacing]]. With a known lens all three are used.
- **Tilts −3°, 3°, 9°, 15°, 21°.** The first screen used only a level camera, treating tilt as a vertical shift. For a 100° lens tilted 14° up the edges bend by degrees ([[locator/camera/camera-ray]]), and Sandy dropped out (70 km off). With 5 tilts 6° apart the nearest is within 3°, and Sandy is back to 0.22 km. The full search then refines tilt to the degree.
- **No roll in the templates**; roll is a line fit on the residual: [[window-algebra]].
- **Every whole yaw.** Each template is swept over all 360 yaws. This used to be a coarse step of ⌊fov/25⌋ degrees plus the degrees around the best, chosen for speed ([[decisions/yaw-step-scales-with-lens]]). Since the cross terms of all 360 windows are computed together ([[dot-kernels]]), every yaw now costs less than the stepped scan did, and the screen's best is exact over whole yaws.

So 8 lenses × 5 tilts = 40 templates × 360 windows = 14,400 closed-form windows per place, against the full search's 17,280 trimmed three-dimensional fits. The single-thread benchmark `screen_64_places` runs about 120 µs per place.

## Per place

The drop penalty is computed once per place for all 360 yaws, then shared by every template. The place's best (loss, yaw, fov) over all templates becomes a `ScreenRow`. Rows are sorted stably by loss. The full search later tries ±8° around this yaw: [[locator/search/two-pass-rank]].
