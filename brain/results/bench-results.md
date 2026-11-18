---
type: result
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - data/bench/runs/rust-1794987848.json#"at": "1794987848",
  - data/bench/runs/rust-1794989831.json#"at": "1794989831",
  - sight/bench/cases.toml#id = "capitol-cloudy"
  - sight/PERF.md#Every case is at or better than baseline error. A search is ~110× faster.
---

# Bench results

Latest full Rust run (`data/bench/runs/rust-1794987848.json`, 2026-11-18, machine heavily loaded):

| Photo | Skyline | Error km | Baseline | Truth rank | Screen rank | Lens | Seconds |
|---|---|---|---|---|---|---|---|
| Timpanogos | traced | 0.15 | 0.15 | 1 | 8 | 70° | 2.2 |
| Sandy | traced | 0.22 | 0.22 | 1 | 2 | 100° | 2.0 |
| Little Cottonwood | guided | 0.12 | 0.12 | 1 | 1 | 60° | 2.1 |
| Library roof | guided | 0.35 | 0.35 | 1 | 565 | 50° | 2.1 |
| Capitol, hazy | guided | 0.43 | 0.43 | 1 | 2 | 50° | 3.2 |
| Capitol, cloudy | guided | 2.56 | 2.56 | 585 | 2 | 70° | 3.1 |
| Red Butte | guided | 0.70 | 0.70 | 1 | 1 | 60° | 8.6 (load spike) |
| Daybreak | guided, fov 26.7° | 1.54 | 1.54 | 115 | — | 28.2° | 6.0 |

Reading it:

- Six of eight are solved (truth rank 1, under 1 km).
- **Capitol, cloudy**: the screen keeps the true area (rank 2) but the full search prefers a place 2.56 km away. The two Capitol photos are taken ~70 m apart; the hazy one is solved, so the cloudy photo's skyline is the difference. Not yet diagnosed.
- **Daybreak**: the true spot is off the tile ([[gotchas/tile-edge]]) and the lens is a telephoto; no screen row lands within 1 km.
- Timings in this run: the coarse stage is 1.2–1.9 s and dominates; refinement 0.75–1.2 s.

**After the screen sweep** (`data/bench/runs/rust-1794989831.json`, load average ~20): every error and truth rank is identical, and capitol-cloudy's screen rank improves 2 → 1. Coarse stage 0.75–1.03 s, whole search 1.5–2.2 s per photo (Daybreak, with a known lens, 1.1 s). Refinement (~0.75–1.2 s) is now as large as the coarse stage.

History: Clojure baseline ~540 s per photo at efca147; Clojure two-pass ~5 s; Rust 1.2–2.4 s; with the screen sweep 1.5–2.2 s under heavy load. Method and logs: [[locator/tools/bench]].
