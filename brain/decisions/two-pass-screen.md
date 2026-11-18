---
type: decision
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/PERF.md#Two-pass ranking
  - sight/PERF.md#One search: ~9 min wall, ~4,800 CPU-s
  - sight/PERF.md#`DualPivotQuicksort` (two full sorts per measure, for a median and a trim cutoff)
---

# Decision: two-pass ranking

**Problem:** at commit efca147 one search took ~9 min on 10 cores: ~32k places × 17,280 fits ≈ 550 M trimmed measures at ~12.5 µs CPU each. The flame graph: two full sorts per measure ~49%, boxing ~27%.

**Chosen, in order:**

1. Exact fixes first (golden test unchanged): quickselect instead of sorts, no boxing in the loop.
2. A screen over every place with sliding sums ([[locator/screen/window-algebra]]), then the full search on the best max(200, 3%) within ±8° of the screen's yaw.

Result: ~5 s warm in Clojure (~110×), with every bench photo at or better than its baseline error. In Rust ~1.2–2.4 s per photo (2–3 s on the loaded machine).

**Rejected:**

- *A coarser full search only* (fewer lenses or yaws everywhere): every reduction applies to the true place too, and the bench showed misses at each step tried for the screen itself ([[screen-lens-spacing]], [[yaw-step-scales-with-lens]]); the two-pass keeps the full resolution where it matters.
- *Triangulation from individual peaks*: needs peak identification in the photo, which the tracer cannot do reliably; not tried.

**Failure modes found and fixed while building it:** level-only templates (Sandy 70.7 km off), 15° lens spacing (library 68 km), no roll (library culled), fixed 3° yaw step (Daybreak 39 km). Each is now a test case in the bench.

**What would change it:** a photo whose true area is culled by the screen. The bench's `screen_rank` shows how close each photo is to that ([[locator/search/two-pass-rank]]).
