---
type: concept
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/src/viewfinder/main.clj#The search is the field. Nothing in here names a town or a summit.
  - sight/crates/sight-core/src/search.rs#pub fn search(
---

# Why a skyline locates a camera

From any place, the terrain defines a horizon: the elevation angle of the highest ground in every direction. Moving the camera changes that curve. Moving sideways changes which peaks occlude which (parallax between near and far ridges) and shifts their azimuths; moving toward or away from a range scales its angular height roughly as 1/distance. A photograph samples a stretch of that curve, up to unknowns the camera adds: where it points (yaw), how it is held (pitch, roll) and how wide the lens is (FOV).

So the problem is a search: over places × camera poses, find the horizon segment the photo's skyline fits best. Everything in the locator is a way to make that search exact where it matters and cheap where it does not:

- Precompute horizons once per place ([[locator/data/horizon-field]]); a photo only has to be compared, not ray-traced.
- Project the photo correctly in 3D ([[locator/camera/camera-ray]]), so tilt and lens width do not masquerade as shape.
- Score with terms that each break one specific confusion ([[locator/scoring/loss]]).
- Screen cheaply, then score fully, then refine ([[locator/search/two-pass-rank]]).

The design rule from the original code still holds: the search is the field, with no location named anywhere. A fix that only helps one photo is not a fix.

What the skyline does not pin down well is distance along the view direction: [[along-sight-ambiguity]].
