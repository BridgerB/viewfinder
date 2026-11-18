---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-cli/src/main.rs#enum Cmd {
  - sight/crates/sight-cli/src/main.rs#fn photo(image: &Path, skyline: Option<&Path>, cfg: &Config) -> Result<(Photo, Vec<sight_core::camera::Pixel>)> {
  - sight/Cargo.toml#lto = "fat"
---

# The `sight` CLI

Built with `cargo build --release` in `sight/` (release profile: fat LTO, one codegen unit, `panic = "abort"`); binary at `sight/target/release/sight`. Paths below are from the repository root. `GRID` is the ENVI path without `.hdr`.

| Command | What it does |
|---|---|
| `info GRID` | bounds and corner ground |
| `build GRID OUT STEP [S N W E]` | a horizon field on a lattice |
| `build-rough GRID OUT 500 3000 300` | the rugged 500 m field |
| `search GRID FIELDS IMAGE SURFACE.json MATCH.json [--overlay JPG] [--skyline JSON] [--fov DEG]` | the search; writes the heatmap rows and the winning camera's outline |
| `around GRID IMAGE LAT LON [--skyline] [--fov] [--step 125] [--reach 1000]` | rank a fresh grid near a point (diagnostic) |
| `ridges GRID CAMERAS.txt OUT.json` | ridge points for `id lat lon yaw fov` lines, for the results map |
| `trace IMAGE` | the traced skyline as frame fractions |
| `bench GRID FIELDS sight/bench/cases.toml [--quick]` | [[bench]] |
| `profile GRID FIELDS IMAGE [--skyline] [--places 500] [--reps 3]` | a fixed `JavaRandom(42)` sample scored repeatedly, for profilers |
| `compare-fields A B`, `golden …`, `compare-trace …` | [[parity]] checks |

`FIELDS` is comma-separated; always pass both: `data/elevation/field/n41w112.hzn,data/elevation/field/n41w112-rough500.hzn`.

Every command that reads a photo goes through `photo`: decode, then the guided skyline if given or the [[locator/sky/skyline-trace|tracer]], then decimate to 160 columns. `--fov` switches to the known-lens config ([[locator/config]]).

Walkthrough: [[how-to/locate-a-photo]].
