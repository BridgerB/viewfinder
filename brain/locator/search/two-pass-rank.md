---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/search.rs#pub fn rank(
  - sight/crates/sight-core/src/config.rs#keep_share: 0.03,
  - sight/crates/sight-core/src/config.rs#reach: 8,
  - data/bench/runs/rust-1794987848.json#"screen_rank": 565,
---

# Two-pass rank

`rank(field)`:

1. [[locator/screen/screen-templates|Screen]] every place; rows best first.
2. Keep the best max(200, 3% of the field). For the stored fields: 200 of 2,352 and 890 of 29,669, so ~1,090 places (the bench's coarse count).
3. Build a sub-field of those places and run the full [[locator/scoring/best-yaw-and-ties|surface]] with yaws limited to ±8° of each place's screen yaw (17 yaws instead of 360).

## Why it is safe enough

The screen is an approximation (no trim, no floor term, coarse lenses, tilts and yaws), so it is judged by the bench, not by equality. In the latest run every photo's true area survives the screen. The tightest case is the library photo: its true area first appears at rank 565 of the merged screen (both fields), inside the 890 kept from the rugged field, and it is rank 1 after the full search. That margin is thin: a smaller `keep_share` should be checked against this photo first.

## Cost

Clojure: ~9 min → ~5 s per search. Rust: coarse stage 1.2–1.9 s of a 2–3 s search on the loaded machine. Decision record: [[decisions/two-pass-screen]].
