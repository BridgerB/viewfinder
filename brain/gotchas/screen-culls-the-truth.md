---
type: gotcha
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/PERF.md#Tilted screen templates.
  - sight/PERF.md#Roll in the screen.
  - sight/crates/sight-cli/src/main.rs#fn around(
---

# The screen culls the true place

**Symptom:** `search` misses badly, but `sight around GRID IMAGE LAT LON` at the true spot shows the ground there fits well (a low loss, close to or below the winner's). In the bench: `screen_rank` is empty or past the kept count while the full search would have ranked the truth first.

**Cause:** the screen's approximations disagree with the full search at that place. Each one has bitten once:

| Approximation | Photo | Effect |
|---|---|---|
| level camera only, tilt as a shift | Sandy (100°, 14° up) | 70.7 km off |
| lenses 15° apart | library (52°) | 68 km off |
| no roll | library | true spot 0.83 in screen vs 0.22 full: culled |
| fixed 3° yaw step | Daybreak (27°) | 39 km off |

**Fix pattern:** find which approximation differs at the true place (compare the screen loss there with the full loss from `around`), then make the screen closer to the full search for that case without a location-specific rule. Re-run the whole bench; each row above is guarded by a bench photo.
