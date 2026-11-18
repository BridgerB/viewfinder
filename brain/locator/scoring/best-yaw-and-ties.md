---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/score.rs#fn best_yaw(
  - sight/crates/sight-core/src/score.rs#fn min_hit(hits: &[Hit]) -> Option<Hit> {
  - sight/crates/sight-core/src/score.rs#pub fn surface(field: &Field, dem: &Dem<'_>, lenses: &[Lens], yaws: Yaws<'_>, cfg: &ScoreConfig) -> Vec<Place> {
  - sight/crates/sight-core/src/score.rs#pub fn yaws_around(center: f64, reach: i64) -> Vec<u16> {
---

# Best yaw, best lens, ties

`surface` scores every place of a field in parallel (rayon `map_init`, one `Scratch` of buffers per worker thread, no allocation per place):

1. Compute the foreground falls for the place (360 bilinear reads).
2. For each of the 48 lenses, `best_yaw` walks the yaw list (all 360, or the 17 degrees within ±8° of the screen's yaw) and keeps the lowest loss, strict `<`, so the first yaw wins a tie.
3. `min_hit` picks the best lens with the reference's odd tie rule: of the first two, the later wins a tie; after that the earlier stays. It is kept only so the [[locator/tools/parity|golden check]] stays exact.
4. Rows are sorted by loss with a stable sort (`total_cmp`), so ties keep field order. Results never depend on thread scheduling.

Cost without the screen: 48 lenses × 360 yaws = 17,280 fits per place, ~1 measure each. ~32k places made ~550 M measures, the original 9 minutes. The screen cuts this to ~1,100 places × 48 × 17: [[locator/search/two-pass-rank]].
