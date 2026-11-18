---
type: decision
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/PERF.md#Loading the DEM (memory-mapped, zero-copy) and both fields takes 0.02 s
  - sight/crates/sight-core/src/field.rs#pub fn parse(bytes: &[u8]) -> Result<Self, FieldError> {
---

# Decision: keep HZN2, no zero-copy HZN3

**Considered:** a new field format already laid out as structure-of-arrays and page-aligned, so the field could be memory-mapped and used without parsing, like the DEM.

**Measured:** parsing HZN2 (interleaved records) into SoA vectors takes about 0.02 s together with mapping the DEM, out of a 1–3 s search. A new format would save at most that, and cost a format bump, a rebuild of both fields, and a second reader.

**Chosen:** keep HZN2; parse once into SoA in memory.

**What would change it:** a much larger field (all of Utah at 500 m would be ~880k places, ~1.9 GB at 2180 bytes each), where parsing cost and memory both matter and mapping would win.
