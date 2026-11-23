---
type: moc
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
---

# Locator

`sight/` is a Cargo workspace: `sight-core` (pure computation, `#![forbid(unsafe_code)]`, never reads a clock or a file), `sight-io` (memory-mapped DEM, HZN2 files, images, JSON, bench cases), `sight-cli` (the `sight` binary). The pipeline, in the order data flows:

1. [[locator/data/_moc|Data]]: DEM grid → horizon walk → horizon field.
2. [[locator/sky/skyline-trace|Sky]]: photo → skyline pixels; with the network, [[locator/sky/sky-model]].
3. [[locator/camera/_moc|Camera]]: skyline pixels → directions (offset, elevation) per lens, roll and pitch.
4. [[locator/scoring/_moc|Scoring]]: the full trimmed-error search at one place.
5. [[locator/screen/_moc|Screen]]: the cheap pass that decides which places get the full search.
6. [[locator/search/_moc|Search]]: rank each field, then refine.
7. [[locator/tools/_moc|Tools]]: CLI, bench, parity checks.

All tunable numbers live in one file, `sight/crates/sight-core/src/config.rs`, each with a comment saying what it guards against: [[locator/config]].
