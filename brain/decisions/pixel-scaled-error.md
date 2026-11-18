---
type: decision
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/config.rs#The trimmed error is counted as if the frame spanned this many degrees: a trace is off
  - sight/crates/sight-core/src/score.rs#let lens_scale = cfg.ref_fov / lens.fov - 1.0;
  - CLAUDE.md#counting in degrees rewarded the narrowest lens
---

# Decision: count the fit error in pixels

**Chosen:** the trimmed RMSE (degrees) is multiplied by `70 / fov` before it enters the loss.

**Why:** a traced skyline is wrong by some number of pixels, whatever the lens. Through a 40° lens a pixel is 0.57× the angle it is through a 70° lens, so the same tracing noise costs fewer degrees. In degrees the narrowest lens always looked best, and through the [[concepts/lens-distance-tradeoff]] a narrow lens means a camera too far back. Sandy and Timpanogos landed kilometres away whenever narrow lenses were allowed.

**Rejected:**

- *Error in degrees* (the original): the bias above.
- *Restricting the lens list to wide lenses only*: hides the bias for phone photos and breaks real telephoto shots (Daybreak is 26.7°).

**What would change it:** a skyline source whose error is in degrees rather than pixels (for example a model that outputs angles directly), or photos at very different resolutions where a pixel is not comparable.
