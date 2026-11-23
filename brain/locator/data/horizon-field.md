---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/field.rs#pub struct Field {
  - sight/crates/sight-core/src/field.rs#pub const MAGIC: &[u8; 4] = b"HZN2";
  - sight/crates/sight-core/src/field.rs#fn steps(start: f64, end: f64, step: f64) -> Vec<f64> {
  - sight/crates/sight-core/src/field.rs#pub fn rugged(dem: &Dem<'_>, lat: f64, lon: f64, radius_m: f64, relief_m: f64) -> bool {
  - sight/crates/sight-core/src/field.rs#pub fn gather(dem: &Dem<'_>, cfg: &HorizonConfig, places: &[(f64, f64)]) -> Field {
  - sight/PERF.md#Horizon fields built by Rust are identical to the Clojure files
---

# Horizon field

A field is the horizons of many places, walked once and read on every search.

## On disk: HZN2

Little-endian. Header 12 bytes: magic `HZN2`, i32 count, i32 azimuths (360). Then per place a 2180-byte record: f64 lat, f64 lon, f32 ground, 360 × i16 centidegree angles, 360 × f32 distances. A changed record layout means a new magic and a rebuild.

## In memory: structure of arrays

`Field { lats, lons, grounds, angles, distances }`, with angles and distances place-major (360 per place). The scoring loops read one place's 360 angles as one contiguous `&[i16]` (720 bytes, a few cache lines). Parsing HZN2 into this layout takes ~0.02 s for both fields, so a zero-copy layout was not worth a new format: [[decisions/no-hzn3]].

## The two stored fields

The fields in use are `wasatch.hzn` and `wasatch-rough500.hzn`, built the same way over three counties ([[three-counties]]). The originals, over the single tile:


- `n41w112.hzn`: a 2 km lattice over the tile, 2352 places.
- `n41w112-rough500.hzn`: a 500 m lattice kept only where `rugged` holds: the ground within 3 km spans ≥ 300 m of relief, sampled on 4 rings × 16 bearings. 29,669 places, ~10.7 s to build in Rust.

Both are searched, comma-separated. Near mountains the skyline changes by degrees within a few hundred meters, so the 2 km field alone has no place close enough to fit: Little Cottonwood Canyon landed 51 km off with it ([[gotchas/narrow-basin-near-mountains]]).

## Lattice

Rows south to north, cells of `step` m in latitude and `step / cos(mid-lat)` in longitude, starting half a cell in. Coordinates are built by repeated addition (`steps`), not `start + i·step`, because the Clojure range accumulated the same way and the files must match bit for bit. They do: both fields, 0 differing values out of 11.5 M ([[locator/tools/parity]]).

## Fresh fields

Refinement walks new horizons on the fly with `gather` (parallel, place order kept, places without ground dropped): [[locator/search/refinement]].
