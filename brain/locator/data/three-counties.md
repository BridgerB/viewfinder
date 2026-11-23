---
type: area
status: verified
verifiedAgainst: 01608dc, 2026-11-20
sourceRefs:
  - sight/crates/sight-cli/src/main.rs#fn rewalk(grid: &Path, field: &Path, out: &Path) -> Result<()> {
  - scripts/clip-field.ts#const keep = (lat: number, lon: number) =>
  - scripts/merge-fields.ts#writeFileSync(outPath, Buffer.concat([header, ...bodies]));
  - package.json#data/elevation/envi/wasatch
---

# Salt Lake, Utah and Davis counties

The locator covers three counties: a camera can stand anywhere in Salt Lake, Utah or Davis County and face any direction. Before, it was one USGS tile (`n41w112`), so a camera west of 112° W had no place to stand and a view west or north stopped at the tile edge ([[gotchas/tile-edge]]).

## The grid

Camera places need the counties (39.78–41.15° N, 110.86–112.49° W, from OpenStreetMap outlines). Each horizon reaches 50 km ([[horizon-walk]]), so the terrain must reach 50 km past them: 39.33–41.60° N, 110.26–113.09° W. Nine 1/3″ tiles cover that (`n40`–`n42` × `w111`–`w113`, 3.7 GB from `prd-tnm.s3.amazonaws.com/StagedProducts/Elevation/13/TIFF/current/`, no sign-in). The 7 km strip west of 113° W is flat desert at the far end of the reach and is left out.

```sh
gdalbuildvrt tif/wasatch.vrt tif/USGS_13_n4{0,1,2}w11{1,2,3}.tif
gdal_translate -of ENVI -ot Int16 -a_nodata -32768 -projwin -113.0005556 41.60 -110.26 39.327 tif/wasatch.vrt envi/wasatch
```

The result is `data/elevation/envi/wasatch`, 29,598 × 24,549 cells, 1.45 GB, on the same cell lattice as the old tile: 2,000 random points inside `n41w112` read the same height from both ([[dem-grid]]).

## The fields

Both fields are the old tile's places plus new ones outside it, clipped to the county outlines with 1 km to spare (`scripts/clip-field.ts`, outlines in `data/elevation/counties.geojson`):

- Inside the old tile the old lattice is kept and re-walked on the new grid (`sight rewalk`), so its horizons now see past the tile edge. A fresh 500 m lattice over the whole area put Little Cottonwood's nearest place 322 m from the truth instead of 175 m, and the photo landed 14.9 km away; near that canyon the good fit is only ~125 m wide ([[gotchas/narrow-basin-near-mountains]]).
- Outside it, new places from `sight build` (2 km) and `sight build-rough` (500 m, rugged ground), both given the counties' bounding box, then clipped with `--outside` the old tile.

`scripts/merge-fields.ts` joins the two halves: `wasatch.hzn` 2,439 places, `wasatch-rough500.hzn` 24,417; 26,856 in all against 32,021 for the old tile, which had more rugged ground east of the counties. With the hand-traced skylines the bench is unchanged on all 8 photos ([[results/bench-results]]).
