---
type: governing
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
---

# Questions this brain answers

If a question here cannot be answered from the linked note, the note is incomplete.

| Question | Note |
|---|---|
| How does a skyline determine a place at all? | [[concepts/why-a-skyline-locates]] |
| What exactly is a horizon, and how are curvature and refraction handled? | [[locator/data/horizon-walk]] |
| What is in an HZN2 file, and why two fields? | [[locator/data/horizon-field]] |
| Why is `pitch + atan(y/f)` wrong? | [[locator/camera/camera-ray]] |
| What is kappa? | [[locator/camera/lens-tilts]] |
| How is one camera pose scored, and why trimmed? | [[locator/scoring/measure]] |
| Why does each loss term exist? | [[locator/scoring/loss]] |
| How is tilt chosen without trying all 36? | [[locator/scoring/fit-and-tilt-guess]] |
| How can the screen score a window without a loop over its columns? | [[locator/screen/window-algebra]] |
| How is roll handled in the screen? | [[locator/screen/window-algebra]] |
| Why is the screen's kernel f32 and the full search f64? | [[decisions/f32-screen-kernel]] |
| How many places get the full search, and is that safe? | [[locator/search/two-pass-rank]] |
| Why can't the search get the distance exactly? | [[concepts/along-sight-ambiguity]] |
| Why not just pick the lens with the smallest error? | [[decisions/pixel-scaled-error]] |
| How do I know the Rust port matches the Clojure one? | [[locator/tools/parity]] |
| Which photos are solved, and which are not? | [[results/bench-results]] |
| A photo landed far off: where do I start? | [[how-to/debug-a-miss]] |
| What must I do before changing a scoring constant? | [[how-to/change-the-loss]] |
| Where does each JSON on the site come from? | [[site/data-flow]] |
| Why is the near-ground penalty zero? | [[decisions/near-penalty-off]] |
