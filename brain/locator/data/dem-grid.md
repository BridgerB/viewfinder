---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/dem.rs#pub const NODATA: i16 = i16::MIN;
  - sight/crates/sight-core/src/dem.rs#pub fn elev_at(&self, lat: f64, lon: f64) -> Option<f64> {
  - sight/crates/sight-core/src/dem.rs#pub fn nearest(&self, row: i64, col: i64) -> i16 {
  - sight/crates/sight-core/src/dem.rs#if get("data type").as_deref() != Some("2") {
  - sight/crates/sight-io/src/lib.rs#pub fn dem(&self) -> Dem<'_> {
  - data/elevation/envi/n41w112.hdr#samples = 10812
---

# DEM grid

The grid is one USGS 1/3 arc-second tile, `n41w112`, converted by GDAL to ENVI: int16, band-sequential, little-endian, 10812 × 10812 cells, ~9.26e-5° per cell (≈10 m north-south, ≈7.8 m east-west at 40.5°N), covering about 40°N–41°N, 112°W–111°W. The header's tie point is the north-west corner of the first cell, half a cell outside the whole degree (west −112.000556, north 41.000556). `EnviHeader::parse` refuses anything not int16 BSQ.

`sight-io` memory-maps the file and borrows it as `&[i16]` with no copy, so loading the 234 MB grid costs nothing until a cell is touched. `Dem<'a>` is just the georeference plus that slice.

Two readers, deliberately different:

- `nearest(row, col)`: one cell, for the horizon walk. The caller rounds with Java's rule ([[gotchas/java-round-parity]]). Off the grid returns `NODATA`.
- `elev_at(lat, lon)`: bilinear over the four surrounding cells, for a place's own ground and the foreground drop ([[locator/scoring/loss]]). `None` if any of the four is nodata or the point is off the grid.

Off-grid and nodata are never 0: a 0 m elevation would look like a canyon 1300 m deep below the Salt Lake valley and wreck the walk and the drop penalty. This is also why a camera just outside the tile has no place under it: [[gotchas/tile-edge]].
