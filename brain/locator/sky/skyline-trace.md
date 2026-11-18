---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/sky.rs#fn is_sky(r: i64, g: i64, b: i64) -> bool {
  - sight/crates/sight-core/src/sky.rs#fn is_haze(r: i64, g: i64, b: i64) -> bool {
  - sight/crates/sight-core/src/sky.rs#fn ridge_row(img: &Image, x: usize, rock_run: usize) -> Option<usize> {
  - sight/crates/sight-core/src/sky.rs#let cleaned = fill_median(&despike(&raw, 70, 50), 8);
  - sight/crates/sight-core/src/sky.rs#pub fn decimate(skyline: &[Pixel], target: usize) -> Vec<Pixel> {
---

# Skyline trace

From decoded ARGB pixels to one skyline pixel per usable column. Simple colour rules, no learning:

1. **Sky**: blue clearly above red (b > r + 12), not much below green (b ≥ g − 8), and bright (b > 140).
2. **Haze**: bright (luma > 155) and nearly colourless (saturation < 0.18): pale sky and cloud edges. A sunlit cliff is redder and fails this.
3. **Ridge row** per column: the first row from the top that is neither sky nor haze and does not return to sky within 160 px (`rock_run`). A cloud bottom or a gap in a tree returns to sky; rock does not.
4. **Despike**: drop a column more than 50 px above the median of its ±70 neighbours: a pole or a tree top, not a summit.
5. **Median fill** ±8 columns (needs ≥ 8 present): a thin spike loses the vote, a gap takes the ridge beside it.
6. **Frame band**: drop columns whose ridge is in the top 15% or below 78% of the frame height.
7. **Decimate** to ~160 evenly spaced columns: the rest are the same ridge, and every column costs time in every fit.

Pixel centres are used (x + 0.5, y + 0.5).

The rules are tuned on blue-sky photos. Six of the eight bench photos use a guided skyline JSON (`--skyline`) instead of the tracer, so the tracer itself is exercised only by Timpanogos and Sandy ([[results/bench-results]]). The capitol-cloudy photo, the one case whose true area is not ranked first, is where clouds touch the ridge. The tracer's output also depends on the JPEG decoder: [[gotchas/jpeg-decoder-differences]].
