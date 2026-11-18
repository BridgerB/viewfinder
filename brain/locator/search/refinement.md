---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/search.rs#pub fn search(
  - sight/crates/sight-core/src/search.rs#fn neighbourhood(rows: &[Place], top: usize, step_m: f64, reach_m: f64) -> Vec<(f64, f64)> {
  - sight/crates/sight-core/src/config.rs#passes: vec![(60, 250.0, 500.0), (15, 100.0, 300.0)],
  - sight/crates/sight-core/src/search.rs#pub fn around(
---

# Refinement

After both stored fields are ranked and merged by loss:

1. **250 m pass.** Around each of the best 60 rows, a 5 × 5 grid at 250 m (reach 500 m). Duplicates are dropped by exact lat/lon bits, first kept. Horizons are walked fresh ([[locator/data/horizon-field|gather]]), then the same [[two-pass-rank]] runs on them.
2. **100 m pass.** Around the best 15 rows of the previous pass, a 7 × 7 grid at 100 m (reach 300 m), walked and ranked the same way.
3. All rows from every stage are merged and sorted by loss; the first is the answer.

Each pass's fresh field is at most 1,500 and 735 places, so `keep_min` 200 decides the survivors (bench counts 200, 200). Timing in the latest run: 0.5–0.7 s and 0.25–0.5 s (Red Butte 3.1 s and 1.8 s under load).

Why refine at all: the right camera spot is often only ~125 m wide near mountains, smaller than the 500 m rugged lattice. The walks are cheap: the rugged field's 29,669 horizons take 10.7 s on 10 cores, so a 1,500-place pass walks in about half a second.

`around` is the diagnostic version: rank a fresh grid around a known point, to see whether the ground near a true spot fits and by how much the winner beats it.
