---
type: area
status: verified
verifiedAgainst: working tree on 8a1e79e (screen sweep), 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/score.rs#pub fn measure(
  - sight/crates/sight-core/src/score.rs#fn nth_largest(values: impl Iterator<Item = f64>, rank: usize) -> f64 {
  - sight/crates/sight-core/src/score.rs#fn angle_at(angles: &[i16], azimuth: f64) -> Option<f64> {
  - sight/crates/sight-core/src/score.rs#fn min_columns(n: usize) -> usize {
  - sight/crates/sight-core/src/score.rs#fn select(values: &mut [f64], k: usize) -> f64 {
  - sight/crates/sight-core/src/score.rs#let corr = if denom < 1.0e-6 { 0.0 } else { ((sgh / k - mg * mh) / denom).clamp(-1.0, 1.0) };
  - sight/crates/sight-core/src/config.rs#keep_fraction: 0.98,
---

# Measure: one camera at one yaw

Inputs: the place's 360 angles and distances, one projected camera (offsets o_i, elevations g_i for ~160 skyline columns), a yaw.

1. **Sample the horizon.** For each column, azimuth = yaw + o_i wrapped to [0, 360); horizon h_i = linear interpolation between the two whole degrees around it (`angle_at`). A column whose neighbours are missing is skipped. Residual r_i = h_i − g_i.
2. **Enough columns.** At least max(24, 45% of columns) must land, or the yaw does not count.
3. **Trim.** Median of the residuals, then the cutoff = the k-th smallest |r_i − median| with k = 98% of the used columns. Columns beyond the cutoff (a pole, a tree, a mis-traced cloud) are dropped. The median comes from `select_nth_unstable_by(total_cmp)`, which returns exactly the value a full sort would put at that index. The cutoff is the (keep − 1)-th smallest deviation, i.e. about the 4th largest of ~160, so `nth_largest` keeps the few largest in a small sorted buffer in one pass. It falls back to a second select if more than 8 columns are trimmed. Both give the sorted value exactly, so the golden check still matches to 1.9e-13. The Clojure version did two full sorts per measure, ~49% of all time ([[decisions/two-pass-screen]] has the profile).
4. **Moments over kept columns** (one pass, plain sums in a fixed order so results match the reference to the last bit): count k, Σr, Σr², Σg, Σh, Σg², Σh², Σgh, and counts of kept columns whose ground is under 2000 m and under 500 m away.
5. **Pitch** = mean residual: the leftover tilt the whole skyline needs to sit on the horizon. **RMSE** = the residual standard deviation about that mean (so a constant offset costs nothing; pitch absorbs it).
6. **Relief**: gsigma = std of g, hsigma = std of h. If gsigma < 0.2° the photo's skyline is flat and cannot place a camera: `None`.
7. **Corr** = Pearson of g and h, 0 when either is flat (denominator < 1e-6).
8. The partial loss is assembled here: [[loss]].
