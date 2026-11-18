---
type: area
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/camera.rs#pub fn outline(angles: &[i16], pose: Pose, width: f64, height: f64) -> Vec<(f64, f64)> {
  - sight/crates/sight-core/src/camera.rs#fn outline_inverts_ray() {
  - sight/crates/sight-core/src/geo.rs#pub fn ridge(distances: &[f32], lat: f64, lon: f64, yaw: f64, fov: f64) -> Vec<(f64, f64)> {
---

# Outline and ridge

Two ways to show a result.

## Outline: the horizon drawn into the photo

`outline` runs [[camera-ray]] backwards: for each whole degree within fov/2 + 10° of the yaw, turn (azimuth step, elevation) into a unit vector, undo pitch, drop anything behind the camera (`depth ≤ 0`), divide by depth for the pinhole, then undo roll and shift to pixel coordinates. Points outside the frame width are dropped. The test `outline_inverts_ray` checks that every outline pixel projects back through `ray` to a whole azimuth and the stored elevation to 1e-6°, across 3 FOVs × 3 rolls × 3 pitches. The overlay JPEG and match JSON use it.

## Ridge: the ground the skyline sits on

`ridge` uses the stored distances: for each whole degree inside the FOV, the lat/lon of the sample that won that degree in the [[locator/data/horizon-walk]]. On the results map this is the line the view wedge reaches to, so a viewer sees which mountain the photo's skyline was matched to ([[site/results-map]]).
