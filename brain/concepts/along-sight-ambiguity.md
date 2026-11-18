---
type: concept
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/score.rs#One step of the coarse field of view moves the relief by about this much; a gap inside
  - sight/crates/sight-core/src/config.rs#Lens widths tried when the lens is unknown: a 50 mm lens through a phone's ultrawide.
---

# Along-sight ambiguity

Moving the camera across the line of sight changes parallax between near and far ridges, and azimuths; it is well constrained. Moving it along the line of sight mostly scales the range's angular size: a ridge 10 km away seen from 9 km is ~11% taller in degrees, with almost the same shape.

The search has two ways to absorb a scale change without moving: pitch absorbs a vertical offset, and a different lens width changes how many degrees the frame spans, so the photo's skyline relief in degrees changes ([[lens-distance-tradeoff]]). With the lens unknown, a camera too far back can fit almost as well as the right one with a narrower lens.

What pins it down:

- **The relief gap** compares the spread in degrees of the photo skyline and the horizon, with a 0.10 dead zone the size of one lens step. A camera at the wrong distance through the right lens has the wrong spread.
- **The pixel-scaled error** stops narrow lenses from winning just because a pixel is a smaller angle ([[decisions/pixel-scaled-error]]).
- **Parallax**: when near foothills and far peaks are both in frame their relative size changes with distance faster than either alone. Photos with layered ridges locate better.
- **A known FOV** (EXIF) removes the lens freedom entirely (Daybreak, 26.7°).

Whether the remaining bench errors (0.12–0.70 km on solved photos) lie mostly along the view direction has not been measured; the bench records distance only. Adding the bearing of the error relative to the winning yaw would answer it.
