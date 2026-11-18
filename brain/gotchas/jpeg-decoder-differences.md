---
type: gotcha
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/PERF.md#JPEG decoding is the one input difference
  - sight/crates/sight-cli/src/main.rs#fn compare_trace(image: &Path, skyline: &Path) -> Result<()> {
---

# JPEG decoder differences

**Symptom:** the Rust and Clojure searches on the same JPEG give losses that differ around the third decimal, or a slightly different winner, while the golden check passes.

**Cause:** the `image` crate and Java's ImageIO decode JPEGs differently (IDCT and chroma upsampling). On the Sandy photo 196 of 3,438 traced columns differ, by up to 32 px, because the sky/rock colour thresholds sit right on the changed pixels in places ([[locator/sky/skyline-trace]]).

**Fix / how to compare:** compare with a guided skyline (`--skyline`), which bypasses decoding; then results match to 1.9e-13. `sight compare-trace IMAGE SKYLINE` shows per-column differences. Bench photos with guided skylines are unaffected; Timpanogos and Sandy (traced) give the same bench error either way.
