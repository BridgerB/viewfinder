---
type: concept
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/src/viewfinder/score.clj#A narrower lens shrinks a farther copy of the ridge until the relief matches.
  - sight/crates/sight-core/src/camera.rs#pub fn focal(width: f64, fov: f64) -> f64 {
---

# Lens–distance trade-off

A skyline spanning P pixels of height in a frame W pixels wide through a lens of width F covers roughly P·F/W degrees. Choose a narrower lens and the same pixels become fewer degrees: the photo now looks like a lower ridge, which is what a farther camera would see. So a (narrower lens, farther place) pair can mimic the (right lens, right place) pair, as the reference's own docstring puts it: a narrower lens shrinks a farther copy of the ridge until the relief matches.

Consequences in the design:

- The lens list spans 40°–115° (a 50 mm-equivalent through a phone ultrawide), and the search must not prefer an end of it for free: [[decisions/pixel-scaled-error]].
- The winning FOV is a by-product, not a measurement. In the latest bench, Timpanogos won at 70° and Sandy at 100°.
- When EXIF gives the focal length, `--fov` should always be passed: it removes the trade-off ([[locator/config]]).

This is the mechanism behind [[along-sight-ambiguity]].
