---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/score.rs#let loss = rmse
  - sight/crates/sight-core/src/score.rs#let scale = ((hsigma.max(1.0e-3) / gsigma.max(1.0e-3)).ln().abs() - 0.10).max(0.0);
  - sight/crates/sight-core/src/score.rs#let loss = m.loss + m.rmse * lens_scale + drop_penalty(falls[yaw], cfg);
  - sight/crates/sight-core/src/score.rs#pub fn drop_penalty(fall: f64, cfg: &ScoreConfig) -> f64 {
  - sight/crates/sight-core/src/score.rs#pub fn falls_into(
  - sight/crates/sight-core/src/config.rs#near_weight: 0.0,
---

# Loss

Degrees, lower is better. For one camera at one yaw:

```
loss = rmse · (70 / fov)                      fit, in pixels of a 70° frame
     + max(0, |ln(hsigma / gsigma)| − 0.10)   relief gap
     + 2 · (1 − corr)                         shape
     + 2 · (kept columns under 500 m) / k     floor
     + 0 · near                               (off)
     + max(0, (fall − 40 m) / 400 m)          drop
```

Each term exists because of a specific wrong answer:

- **Fit scaled to pixels.** `rmse` is in degrees, then multiplied by `ref_fov / fov` (in code: `rmse + rmse·(70/fov − 1)`). A traced skyline is off by pixels, and a pixel is a smaller angle through a narrow lens, so raw degrees always favoured the narrowest lens. When narrow lenses were allowed that sent Sandy and Timpanogos kilometres away: [[decisions/pixel-scaled-error]].
- **Relief gap.** At the right place and lens the photo's skyline has the same spread in degrees as the horizon. A log ratio is symmetric (twice too big = half too big). The 0.10 dead zone is about the relief change of one 5° lens step, so the coarse lens grid is not punished: a gap inside it is the grid, not a wrong distance. This term is what fights [[concepts/along-sight-ambiguity]].
- **Shape.** 2(1 − corr) ranges 0–4°; it rejects places where the horizon has the right spread but the wrong bumps.
- **Floor.** A kept column whose winning ground is under 500 m away is the walk's first step: the berm beside the camera, not a ridge. Up to 2° if all of them are.
- **Near (off).** Penalised windows dominated by ground under 2 km. It sent a canyon-mouth photo (Little Cottonwood) 51 km away, and once pitch was inside the projection no valley photo needed it: [[decisions/near-penalty-off]].
- **Drop.** Ground 1200 m ahead (bilinear, per azimuth, off-grid counts as level) more than 40 m below the camera costs 1° per 400 m. The config's reasoning: a valley photograph is still on the flat 1.2 km ahead, so a place on a bench or ridge edge whose ground falls away in the view direction is less likely, even when its far skyline matches. Which wrong answer prompted it is not recorded in the repo.

The screen computes the same terms from window sums, plus a roll line fit: [[locator/screen/window-algebra]].
