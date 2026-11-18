---
type: gotcha
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/java.rs#pub fn round(x: f64) -> i64 {
  - sight/crates/sight-core/src/field.rs#reason = "the reference computes (a + b) / 2; the lattice must land on the same bits"
---

# Java rounding parity

**Symptom:** a ported field differs from the reference in a few cells, or a few angles by ±1 centidegree.

**Cause:** Java's `Math.round` rounds halves toward +∞ (`floor(x + 0.5)` semantics: −2.5 → −2). Rust's `f64::round` rounds halves away from zero (−2.5 → −3). The walk rounds DEM row/column indices and centidegree angles; negative angles are common (ground below the eye), and any value landing on an exact negative half would round differently.

Two more bit-level traps of the same kind:

- `(a + b) / 2` vs `f64::midpoint`: clippy suggests the latter; it can differ in the last bit, which moves a lattice row. Allowed with a reason.
- A lattice computed as `start + i·step` differs from one built by repeated addition; the reference accumulates, so the port does too.

**Fix:** `java::round` everywhere the reference used `Math/round`, with tests on the halves. Result: 0 differing values in both fields ([[locator/tools/parity]]).
