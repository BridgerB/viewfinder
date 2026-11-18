---
type: how-to
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/crates/sight-cli/src/main.rs#/// Every place within REACH m of LAT LON, STEP m apart, scored against the photograph.
  - sight/crates/sight-cli/src/main.rs#/// The traced skyline, as fractions of the frame.
---

# Debug a miss

A photo with a known spot lands far off. Work down this list; each step rules out one layer.

1. **Is the skyline right?** Run `search` with `--overlay` and look at the traced line on the photo. If it follows clouds, trees or a pole, fix the input (guided `--skyline`) before touching the loss ([[locator/sky/skyline-trace]]).
2. **Is the truth on the grid with open views?** Off the tile, or looking off its edge, the floor on error is real ([[gotchas/tile-edge]]).
3. **Does the true spot fit at all?** `sight around GRID IMAGE LAT LON` ranks a 125 m grid within 1 km. If the best loss there is clearly worse than the search winner's, the loss prefers the wrong place: compare the terms (rmse, relief gap, corr, drop) at both to see which one decides, and see [[how-to/change-the-loss]].
4. **If it fits but search misses**, the screen culled it: check `screen_rank` in the bench and [[gotchas/screen-culls-the-truth]].
5. **If the right area wins but the spot is off by a few hundred meters**, that is the along-sight limit ([[concepts/along-sight-ambiguity]]); a known FOV helps most.
