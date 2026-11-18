---
type: decision
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/PERF.md#A 52° photo screened with 40°/55° templates and lost its place (library 68 km off).
  - sight/crates/sight-core/src/screen.rs#if score.fovs.len() <= 3 { score.fovs.clone() } else { score.fovs.iter().step_by(2).copied().collect() };
---

# Decision: screen lenses 10° apart

**Chosen:** the screen uses every other lens of the 5°-spaced list: 40, 50, … 110°.

**Rejected:** every third lens (15° apart: 40, 55, 70 …). A 52° photo fell between the 40° and 55° templates; neither fit well enough to survive, and the library photo landed 68 km away.

**Cost:** 8 lenses instead of 6, a third more screen time.

**What would change it:** a better relief term in the screen that tolerates a larger lens mismatch, or per-place lens refinement in the screen. Check the library photo first.
