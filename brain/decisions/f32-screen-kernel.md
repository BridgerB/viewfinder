---
type: decision
status: verified
verifiedAgainst: working tree on 8a1e79e (screen sweep), 2026-11-18
sourceRefs:
  - sight/PERF.md#`dot32`: f32 lanes, summed to f64
  - sight/PERF.md#Cross terms in register tiles of 40 window starts
  - sight/crates/sight-core/src/screen.rs#let centred = f64::from(w.cross[b]);
---

# Decision: f32 cross terms in the screen

**Chosen:** the screen's cross terms are f32 (centred weights and horizon, tiled over 40 window starts, [[locator/screen/dot-kernels]]); the rest of the window algebra and the whole full search stay f64.

**Measured, in two steps:**

1. Per-window `dot32` (16 f32 lanes, total in f64) against the f64 `dot`: 5.54 → 3.56 µs (70° template) and 8.83 → 4.04 µs (115°) for 360 windows; ranking workload 1.38–1.62 → 1.27–1.52 s; Daybreak true-area rank 117 → 115, everything else unchanged.
2. All 360 windows at once in register tiles: 0.81 µs (70°) and 1.37 µs (115°). The bench's error and truth rank were unchanged on every photo.

Horizon angles are centidegrees, so f32's 7 digits are far beyond the data's precision. The risk is cancellation in Σr² = Σe² − 2Σg·e + Σg², which is why the f32 sums are centred and the cancelling sum is done in f64.

**Rejected:**

- *f32 in the full search*: breaks the bit-exact golden check for little gain.
- *An explicit SIMD crate (`wide`)*: plain loops over fixed-size arrays auto-vectorise once the loop order is right.

**Correction to an earlier claim:** this note used to say the per-window loop "already fills the NEON pipes". It did not. Register blocking over window starts (which that note listed as untested) made the cross terms ~3× faster.

**What would change it:** a window that cancels more heavily, or horizons with far larger values. The sweep test (`sweep_matches_every_window_from_the_definitions`, f64 reference to 1e-5) is the check.
