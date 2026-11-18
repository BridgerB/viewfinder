---
type: gotcha
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - data/elevation/envi/n41w112.hdr#map info = {Geographic Lat/Lon, 1, 1, -112.000555556193
  - sight/bench/cases.toml#truth = [40.55505, -112.00346]
---

# Tile edge

**Symptom:** a photo near the edge of the tile cannot get closer than a few hundred meters, or its rays to one side are short.

**Cause:** the DEM is one 1° tile. Places off the grid have no ground and are never scored; rays that leave the grid stop collecting samples (off-grid is nodata, not 0). The Daybreak photo's true spot (−112.00346) is about 245 m west of the tile's west edge (−112.00056), so no place exists at the truth; the search can at best find the nearest on-grid place with a similar view. Daybreak's 1.54 km error and truth rank 115 include this.

Rays that leave the tile also see less: a camera near the north edge (41°N) misses ridges beyond it. The winning yaws in the latest bench run all lie between 22° and 169° (north-east through south), toward the Wasatch, which is inside the tile.

**Fix:** mosaic neighbouring tiles into one grid (GDAL `gdalbuildvrt` then translate to ENVI int16 BSQ) and rebuild both fields. Not done; the conversion command for the current tile is not recorded either ([[locator/data/dem-grid]]).
