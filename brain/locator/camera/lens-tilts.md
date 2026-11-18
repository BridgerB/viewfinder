---
type: area
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/camera.rs#pub struct Lens {
  - sight/crates/sight-core/src/camera.rs#pub fn tilt_index(&self, degrees: f64) -> Option<usize> {
  - sight/crates/sight-core/src/camera.rs#let kappa = sum / offsets.len().max(1) as f64;
  - sight/crates/sight-core/src/camera.rs#pub fn lenses(photo: &Photo, fovs: &[f64], rolls: &[f64], pitch_low: i32, pitch_high: i32) -> Vec<Lens> {
---

# Lens and tilts

Projecting every skyline pixel costs trig, and the full search tries ~17,000 camera poses per place, so projection is done once per photo, not per pose.

A `Lens` is one FOV × roll, projected at every whole pitch from −5° to 30° (36 `Camera`s, each two parallel arrays: offsets and elevations). 16 FOVs × 3 rolls = 48 lenses, 1728 cameras, built in ~6 ms per photo ("prepare" stage in the bench). Yaw needs no projection: it just adds to every offset.

## Kappa

Tilting the camera up by δ lifts a column at azimuth offset o by about δ·cos(o), not δ: the rotation is about a horizontal axis, and a column off to the side moves less in elevation. `kappa` is the mean of cos(offset) over the level camera's columns. For a skyline spanning the frame, a 40° lens has kappa ≈ 0.98 and a 115° lens ≈ 0.79 (asinh(t)/t with t = tan(fov/2): columns are even in pixels, which crowds them toward the edges in angle). A skyline covering only part of the frame has its own kappa. Dividing the mean elevation gap by kappa turns "the skyline sits 3° low on average" into "tilt the camera ~3.5°": [[locator/scoring/fit-and-tilt-guess]].

## tilt_index

Nearest whole pitch by Java rounding, `None` outside −5..30. A photo tilted beyond that range does not fit; such a photo is mostly ground or mostly sky.
