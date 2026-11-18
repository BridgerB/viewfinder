---
type: decision
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/PERF.md#`dot32`: f32 lanes, summed to f64
  - sight/crates/sight-core/src/screen.rs#let cross = dot32(&tpl.elevs32, &s.e32, b);
---

# Decision: f32 cross term in the screen

**Chosen:** the screen's cross term uses `dot32` (16 f32 lanes, total in f64); everything else in the screen and the whole full search stay f64.

**Measured:** microbenchmark 5.54 → 3.56 µs (70° template) and 8.83 → 4.04 µs (115°) for 360 windows; ranking workload 1.38–1.62 → 1.27–1.52 s. Bench accuracy unchanged (Daybreak true-area rank 117 → 115). Horizon angles are centidegrees, so f32's 7 digits are far beyond the data's precision.

**Rejected:**

- *f32 in the full search*: breaks the bit-exact golden check for little gain (the full search is ~7% of time).
- *Explicit SIMD crate (`wide`) or 4-yaw register blocking*: the auto-vectorised fixed-chunk loop already fills the NEON pipes; not needed after this step. Untested, so it remains a candidate if the screen dominates again.

**What would change it:** a screen change that cancels large sums (for example subtracting means inside the loop), where f32 rounding could matter; re-check with `dot32_is_close_to_the_f64_sum`.
