---
type: area
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/crates/sight-cli/src/main.rs#fn bench(grid: &Path, fields: &str, cases: &Path, quick: bool, runs: &Path) -> Result<()> {
  - sight/crates/sight-cli/src/main.rs#fn first_within(truth: [f64; 2], points: impl Iterator<Item = [f64; 2]>) -> Option<usize> {
  - sight/bench/cases.toml#baseline_km is the search's error when this bench was set up.
---

# Bench

`sight bench GRID FIELDS sight/bench/cases.toml [--quick]` runs every known-location photo through the full search and writes `data/bench/runs/rust-<unix time>.json` (gitignored), printing a table against the previous run.

Per case it records:

- `error_km`: great-circle distance from the winner to the truth.
- `truth_rank`: 1-based rank of the first final row within 1 km of the truth. 1 means solved.
- `screen_rank`: the same in the merged screen. If this is past the kept count the full search never saw the true area.
- `fov`, `yaw`, `loss` of the winner; wall `seconds`; per-stage seconds (photo, prepare, coarse, refine0, refine1); places scored per stage.

`cases.toml` holds 8 photos with truth lat/lon, a baseline error, an optional guided skyline and an optional known FOV; `quick = true` marks the three used by `--quick` (Timpanogos, Sandy, Little Cottonwood). Photos under `data/` are gitignored; the Wikimedia ones are credited in `static/results/LICENSE.md`.

How to use it: the bench is the judge for anything approximate (the screen, the loss). A change is good if no case gets worse in `error_km` or `truth_rank`. Seconds are noisy on a loaded machine; use `profile` or the divan microbenchmarks for speed. Latest numbers: [[results/bench-results]].
