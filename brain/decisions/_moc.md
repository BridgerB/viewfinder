---
type: moc
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
---

# Decisions

Accuracy:

- [[pixel-scaled-error]]: error counted in pixels of a 70° frame, not degrees.
- [[near-penalty-off]]: the near-ground penalty is weighted 0.

Speed:

- [[two-pass-screen]]: screen every place, fully score the best few percent.
- [[screen-lens-spacing]]: screen lenses 10° apart, not 15°.
- [[yaw-step-scales-with-lens]]: screen yaw step fov/25, not a fixed 3°.
- [[f32-screen-kernel]]: the screen's cross term in f32.
- [[no-hzn3]]: keep the HZN2 file format.
- [[rust-port]]: the locator rewritten in Rust.
