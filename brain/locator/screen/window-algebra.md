---
type: area
status: verified
verifiedAgainst: working tree on 8a1e79e (screen sweep), 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/screen.rs#pub fn fill(&mut self, angles: &[i16]) {
  - sight/crates/sight-core/src/screen.rs#let sk = (pjz[b] - pja[b]) - b as f64 * s1;
  - sight/crates/sight-core/src/screen.rs#let tilt = (-tilt_cap).max(tilt_cap.min(sxr * tpl.inv_sxx));
  - sight/crates/sight-core/src/screen.rs#let rss = (srr - sr * sr * tpl.inv_m) + -2.0 * tilt * sxr + tilt * tilt * tpl.sxx;
  - sight/crates/sight-core/src/screen.rs#let vt = (tpl.vg + tilt * tilt * tpl.sxx_m + 2.0 * tilt * tpl.cgk).max(0.0);
  - sight/crates/sight-core/src/screen.rs#let scale = (0.5 * ratio.ln().abs() - 0.10).max(0.0);
  - sight/crates/sight-core/src/screen.rs#fn sweep_matches_every_window_from_the_definitions() {
  - sight/crates/sight-core/src/config.rs#tilt: 0.055,
  - sight/PERF.md#Roll in the screen.
---

# Window algebra

Every term of the screen loss for one window comes from a handful of sums, so a window costs one cross term ([[dot-kernels]]) plus ~40 flops, with no division: everything that divides by m or Sxx uses a reciprocal stored on the template.

## Setup

The place's horizon, in degrees, laid out twice round: e_j for j = 0..719 (so a window never wraps). Prefix sums P0 (e), P2 (e²), Pj (j·e), Pn (missing count). A missing degree is 0 in the value sums and 1 in Pn. Per template (computed once): g1 = Σg, g2 = Σg², gk = Σk·g, k̄ = (m−1)/2, Sxx = Σ(k − k̄)², 1/m, 1/Sxx, Sxx/m, mean and variance of g, cov(g, k).

## One window

The window of yaw y starts at b = (y + start) mod 360 and has length m. The sweep computes all 360 starts in one branch-free loop: each prefix array is sliced once to its first 360 and its 360 from m on, so the loop has no bounds checks.

- Pn[b+m] − Pn[b] ≠ 0: the window touches a missing degree, and its loss is set to infinity.
- s1 = Σe, s2 = Σe² by two reads each; sk = Σk·e_{b+k} = (Pj diff) − b·s1.
- cross = Σ g_k·e_{b+k}, precomputed for all starts.
- Residual r = h − g: Σr = s1 − g1, Σr² = s2 − 2·cross + g2, Σk·r = sk − gk, Sxr = Σk·r − k̄·Σr.

## Roll as a line

A small roll tilts the skyline: in a frame rolled by ρ the residual gains roughly a straight-line trend across the window. Least squares for r ≈ c + t(k − k̄) gives t = Sxr / Sxx, clamped to ±0.055 (degrees per degree, about 3° of roll). Then

- RSS = (Σr² − (Σr)²/m) − 2t·Sxr + t²·Sxx; fit = √(RSS/m) · 70/fov. The mean (pitch) is absorbed by the first bracket.
- The template is treated as tilted, g' = g + t(k − k̄): its variance vt = vg + t²·Sxx/m + 2t·cov(g, k), and cov(g', h) = cov(g, h) + t·cov(k, h). Correlation and the relief gap use g'.

Without the line fit the library photo's true spot scored 0.83 in the screen against 0.22 in the full search and was culled; with it 0.29, and the photo resolves to 0.35 km (baseline 1.62 km).

## Loss and the log last

loss = fit + max(0, ½·|ln(vh/vt)| − 0.10) + 2(1 − corr) + drop, the same terms as the full [[locator/scoring/loss]] without trimming or the floor term (½·ln of the variance ratio is ln of the σ ratio, with one square root fewer). The branch-free loop stores fit + shape + drop and the two variances per window. A second pass walks the yaws in order and takes the logarithm only when that partial loss is below the template's best so far: the relief term is never negative, so any other window cannot win. Ties go to the lowest yaw.

The test `sweep_matches_every_window_from_the_definitions` rebuilds each window's loss straight from the definitions (residuals, least-squares line, variances) in f64 and checks the sweep's best against all 360.
