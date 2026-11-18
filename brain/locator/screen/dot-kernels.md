---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/screen.rs#pub fn dot(g: &[f64], e: &[f64], b: usize) -> f64 {
  - sight/crates/sight-core/src/screen.rs#pub fn dot32(g: &[f32], e: &[f32], b: usize) -> f64 {
  - sight/crates/sight-core/src/screen.rs#fn dot32_is_close_to_the_f64_sum() {
  - sight/crates/sight-core/benches/kernels.rs#fn cross_term_f32
  - sight/PERF.md#Microbenchmark (`cargo bench -p sight-core`)
---

# Dot kernels

The cross term Σ g_k·e_{b+k} is the screen's only per-window loop and most of the search's time (`slide-yaw` with `dot` inlined was ~61% of the Clojure coarse stage).

## dot (f64)

8 independent accumulators over fixed-size chunks (`as_chunks::<8>`), each updated with `mul_add`, combined pairwise at the end, tail added serially. Independent sums let the CPU issue FMAs back to back instead of waiting ~4 cycles on one running sum, and fixed-size chunks let LLVM keep the accumulators in vector registers. Moving from a plain loop to this took the 32k-place ranking workload from 2.46–3.04 s to 1.52–1.74 s.

## dot32 (f32, used)

Template elevations and the doubled horizon are also kept as f32. 16 f32 lanes (four 128-bit NEON registers), summed to f64 at the end. Twice the elements per register, half the bytes read. Relative error against the f64 sum is under 1e-4 (test `dot32_is_close_to_the_f64_sum`); the screen only ranks, and the bench is unchanged (Daybreak true-area rank 117 → 115).

Microbenchmark, 360 windows single-threaded: f64 5.54 µs at a 70° template, 8.83 µs at 115°; f32 3.56 µs and 4.04 µs. Decision and alternatives: [[decisions/f32-screen-kernel]].

The full search stays f64 and keeps the reference's summation order, so the golden check stays exact.
