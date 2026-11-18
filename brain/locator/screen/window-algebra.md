---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/screen.rs#pub fn fill(&mut self, angles: &[i16]) {
  - sight/crates/sight-core/src/screen.rs#let sk = (s.pj[end] - s.pj[b]) - b as f64 * s1;
  - sight/crates/sight-core/src/screen.rs#let tilt = if tpl.sxx > 0.0 { (-tilt_cap).max(tilt_cap.min(sxr / tpl.sxx)) } else { 0.0 };
  - sight/crates/sight-core/src/screen.rs#let rss = (srr - (sr * sr) / mf) + -2.0 * tilt * sxr + tilt * tilt * tpl.sxx;
  - sight/crates/sight-core/src/screen.rs#let vt = (tpl.vg + tilt * tilt * (tpl.sxx / mf) + 2.0 * tilt * cgk).max(0.0);
  - sight/crates/sight-core/src/screen.rs#if fit + shape + drop >= best {
  - sight/crates/sight-core/src/config.rs#tilt: 0.055,
  - sight/PERF.md#Roll in the screen.
---

# Window algebra

Every term of the screen loss for one window comes from a handful of sums, so a window costs one multiply-add pass (the cross term) plus ~40 flops.

## Setup

The place's horizon, in degrees, laid out twice round: e_j for j = 0..719 (so a window never wraps). Prefix sums P0 (e), P2 (e²), Pj (j·e), Pn (missing count). A missing degree is 0 in the value sums and 1 in Pn. Per template (computed once): g1 = Σg, g2 = Σg², gk = Σk·g, k̄ = (m−1)/2, Sxx = Σ(k − k̄)², mean and variance of g.

## One window

For yaw y, the window starts at b = (y + start) mod 360, length m.

- If Pn[b+m] − Pn[b] ≠ 0 the window touches a missing degree: rejected.
- s1 = Σe, s2 = Σe² by two reads each; sk = Σk·e_{b+k} = (Pj diff) − b·s1.
- cross = Σ g_k·e_{b+k}: the only loop ([[dot-kernels]]).
- Residual r = h − g: Σr = s1 − g1, Σr² = s2 − 2·cross + g2, Σk·r = sk − gk, Sxr = Σk·r − k̄·Σr.

## Roll as a line

A small roll tilts the skyline: in a frame rolled by ρ the residual gains roughly a straight-line trend across the window. Least squares for r ≈ c + t(k − k̄) gives t = Sxr / Sxx, clamped to ±0.055 (degrees per degree, about 3° of roll). Then

- RSS = (Σr² − (Σr)²/m) − 2t·Sxr + t²·Sxx; fit = √(RSS/m) · 70/fov. The mean (pitch) is absorbed by the first bracket.
- The template is treated as tilted, g' = g + t(k − k̄): its variance vt = vg + t²·Sxx/m + 2t·cov(g, k), and cov(g', h) = cov(g, h) + t·cov(k, h). Correlation and the relief gap use g'.

Without the line fit the library photo's true spot scored 0.83 in the screen against 0.22 in the full search and was culled; with it 0.29, and the photo resolves to 0.35 km (baseline 1.62 km).

## Loss and the ln skip

loss = fit + max(0, |ln(√vh / √vt)| − 0.10) + 2(1 − corr) + drop, the same terms as the full [[locator/scoring/loss]] without trimming or the floor term. The relief term is never negative, so if fit + shape + drop already reaches the best loss so far for this place, the `ln` is skipped and the window rejected: same answer, less time (the ranking workload went from 1.52–1.74 s to 1.38–1.62 s; `log` was ~5% of the Clojure profile).
