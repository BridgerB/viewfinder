---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/PERF.md#Parity with the Clojure version:
  - sight/crates/sight-cli/src/main.rs#fn golden(grid: &Path, field_path: &Path, image: &Path, golden: &Path, skyline: Option<&Path>) -> Result<()> {
  - sight/crates/sight-cli/src/main.rs#fn compare_fields(a: &Path, b: &Path) -> Result<()> {
  - sight/crates/sight-core/src/java.rs#pub struct JavaRandom {
  - sight/tests/fixtures/golden.json#"loss": 1.0611357427237869
---

# Parity with the Clojure reference

The Rust port was proven equal, not assumed equal:

1. **Fields**: `sight compare-fields` on Rust-built vs Clojure-built files. 2,352 places / 846,720 values and 29,669 places / 10,680,840 values: 0 differing angles or distances. Needed Java rounding ([[gotchas/java-round-parity]]) and lattice coordinates by repeated addition.
2. **Full search (golden)**: 60 places of the 2 km field picked by `JavaRandom(7)` (the exact `java.util.Random` LCG, checked against jshell output), scored by both. With the same skyline input all 60 match to 1.9e-13 in loss. Needed sums in the reference's order and the reference's lens tie rule ([[locator/scoring/best-yaw-and-ties]]).
3. **Trace**: `sight compare-trace` showed the one input difference: JPEG decoding ([[gotchas/jpeg-decoder-differences]]).
4. **Bench**: same error, truth rank and screen rank as the last Clojure run on all 8 photos.

The screen is not bit-equal by design (f32 kernel): judged by the bench only.

Once the Clojure code is deleted, `tests/fixtures/golden.json` is the frozen answer; re-record it only for a change meant to alter results ([[how-to/change-the-loss]]).
