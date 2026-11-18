---
type: area
status: verified
verifiedAgainst: working tree on 8a1e79e (screen sweep), 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/screen.rs#pub fn cross_all(g: &[f32], e: &[f32], out: &mut [f32; AZIMUTHS]) {
  - sight/crates/sight-core/src/screen.rs#const TILE: usize = 40;
  - sight/crates/sight-core/src/screen.rs#let srr = (s2 - 2.0 * (centred + tpl.mg * s1)) + tpl.g2;
  - sight/crates/sight-core/benches/kernels.rs#fn cross_term_all(bencher: divan::Bencher, m: usize) {
  - sight/crates/sight-core/benches/kernels.rs#fn screen_64_places(bencher: divan::Bencher) {
  - sight/PERF.md#Cross terms in register tiles of 40 window starts
---

# Cross-term kernel

Each window needs one sum that prefix sums cannot give: the cross term Σ g_k·e_{b+k} between the template and the horizon. The screen computes it for all 360 window starts of a template at once (`cross_all`).

## Loop order

The template index k is the outer loop and the window start b the inner one: `out[b] += g[k]·e[b+k]`. The inner loop walks contiguous horizon values with one broadcast weight, so it vectorises with no per-window horizontal reduction. Computed one window at a time (`dot`, the old way), every window ended in a 16-lane fold and conversions to f64. That fold cost as much as the multiplies for a 40–70° template.

## Register tiles

Written naively, the inner loop loads and stores all 360 sums once per template value and is bound by stores (~3.5 multiply-adds per cycle). `cross_all` instead takes 40 window starts at a time (360 = 9 × 40, ten 4-lane registers) and keeps their sums in registers through the whole template. Each template value then costs ten vector loads and ten fused multiply-adds. Measured (`cross_term_all`, 360 windows): 2.27 → 0.81 µs at 70°, 3.65 → 1.37 µs at 115°; the old per-window f64 `dot` takes 5.7 µs and 9.0 µs.

## Precision: centred f32

The weights are the template minus its mean, and the horizon values are the place's horizon minus its mean, both f32. Σ(g − ḡ)(e − ē) = Σ g·e − ḡ·Σe because the centred weights sum to zero, so the full cross term is rebuilt in f64 as centred + ḡ·s1. Centring keeps the f32 sums near the size of the covariance they feed (σg·σh·m) rather than the raw elevations (ḡ·ē·m), which matters because the residual sum Σr² = Σe² − 2Σg·e + Σg² cancels heavily for a good fit.

The test `sweep_matches_every_window_from_the_definitions` checks every template's best window against a direct f64 evaluation of all 360 windows, to 1e-5.

`dot` (f64, 8 lanes) remains as the reference kernel in the tests and the benchmark. Decision and alternatives: [[decisions/f32-screen-kernel]].
