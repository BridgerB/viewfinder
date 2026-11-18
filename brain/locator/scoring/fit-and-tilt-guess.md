---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/score.rs#fn tilt_guess(angles: &[i16], level: &Camera, yaw: usize, kappa: f64) -> Option<f64> {
  - sight/crates/sight-core/src/score.rs#let second = lens.tilt_index(cam.pitch + m.pitch / lens.kappa)?;
  - sight/crates/sight-core/src/config.rs#pitch_slack: 1.0,
  - sight/PERF.md#second tilt on 0.09%
---

# Fit: choosing the tilt

Trying all 36 pitches at every yaw would be 36× the work. Instead:

1. **Guess.** With the level camera, average the gap h − g over the columns that land, divide by [[locator/camera/lens-tilts|kappa]]: that is the tilt that would lift the skyline onto the horizon on average.
2. **Measure** the camera at the nearest whole pitch. Its mean residual `pitch` is what is left over.
3. **Accept** if |leftover| ≤ 1° (`pitch_slack`). The reported pitch is the camera's whole pitch + the leftover.
4. **Retry once** otherwise: in a wide frame the level camera's guess is bent (rows are not equal-elevation lines, [[locator/camera/camera-ray]]), so tilt again by leftover / kappa and measure. Accept only if this leftover is ≤ 1°; else the yaw does not count.

Why the slack is small: a leftover is applied as a flat shift, which is exactly the approximation the 3D projection replaced. Bounding it to 1° (the pitch grid's own spacing) keeps that error small; letting it absorb several degrees would bring the Sandy failure back. Measured in the Clojure baseline: the second tilt runs on 0.09% of fits, so the retry costs nothing.
