---
type: decision
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/config.rs#Degrees added when the rest of the window is near ground. Off: it sent a canyon-mouth
  - sight/crates/sight-core/src/config.rs#near_weight: 0.0,
---

# Decision: near-ground penalty off

**Chosen:** `near_weight = 0`. The term is still computed (the count of kept columns whose ground is under 2 km, beyond a 20% allowance) so it can be turned back on for an experiment.

**Why it existed:** before the 3D projection, a valley photo could fit a place on a hillside that "matched" by looking at its own slope: near ground filling the frame. At 1.5° per unit it pushed those away.

**Why it is off:** it sent the Little Cottonwood canyon-mouth photo 51 km away, because from the true spot the canyon walls are near ground and that is the photo. After pitch moved inside the projection and the error became pixel-scaled, no valley photo in the bench needs it; the floor term (ground under 500 m, the walk's first step) covers the real hillside case.

**What would change it:** a bench photo that lands on a hillside looking at its own slope with the current loss.
