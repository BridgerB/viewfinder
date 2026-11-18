---
type: decision
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/PERF.md#A fixed 3° step lost the 27° telephoto Daybreak photo (39 km off); a 1° step at 27° keeps it.
  - sight/crates/sight-core/src/config.rs#pub yaw_span: i64,
---

# Decision: screen yaw step = fov / 25

**Chosen:** the screen tries every ⌊fov/25⌋ degrees (at least 1), then the degrees between around the best.

**Rejected:** a fixed 3° step for all lenses. The fit's sharpness in yaw scales with the lens: a 3° miss is a tenth of a 27° frame, enough to wreck the correlation, but a thirtieth of a 100° frame. The 27° Daybreak telephoto lost its place (39 km off) at 3°.

**What would change it:** none expected; the step is already proportional. A finer `yaw_span` costs time linearly in the wide lenses only.
