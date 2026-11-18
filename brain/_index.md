---
type: index
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
---

# Viewfinder brain

What this repository does, why each number in it is what it is, and what was tried and thrown away. Start with [[operating-manual]], then the map for the part you are touching.

## Pillars

- [[locator/_moc|Locator]]: the Rust `sight` workspace that finds where a photo was taken.
- [[site/_moc|Site]]: the SvelteKit pages that display pregenerated results.

## Cross-cutting

- [[concepts/_moc|Concepts]]: the geometry and statistics everything rests on.
- [[decisions/_moc|Decisions]]: choices with the options rejected and the measurements that rejected them.
- [[gotchas/_moc|Gotchas]]: symptom first, then cause, then fix.
- [[how-to/_moc|How-to]]: procedures.
- [[results/bench-results|Bench results]]: where the search stands on the known photos.

## Governing documents

- [[operating-manual]]: how to use and maintain this brain.
- [[conventions]]: note format, frontmatter, sourceRefs.
- [[rules]]: the invariants and what the checker enforces.
- [[glossary]]: the words with a precise meaning here.
- [[questions]]: questions this brain must answer, with the note that answers each.

## Limits of this brain

- Numbers about speed were measured on a 10-core Apple laptop under heavy load from other work (load average 48–84). Treat seconds as ±30%; the microbenchmarks in [[decisions/f32-screen-kernel]] are the clean comparison.
- The truth set is 8 photos, all in Salt Lake and Utah counties, all on one USGS tile (`n41w112`). Nothing here has been tested on another tile, another range, or a photo whose skyline is not a mountain ridge.
- The Clojure implementation still exists in `sight/src` as the reference until cut-over; notes describe the Rust code, which matches it bit for bit except where [[gotchas/jpeg-decoder-differences]] says otherwise.
- `timpanogos-match.json` and `sandy-skyline.json` have no generator in the repo; [[site/data-flow]] says what is known.
