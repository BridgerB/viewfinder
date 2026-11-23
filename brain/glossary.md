---
type: governing
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
---

# Glossary

- **Azimuth**: compass direction in whole degrees, 0 north, clockwise. A horizon has one sample per azimuth.
- **Centidegree**: 1/100°. Horizon angles are stored as `i16` centidegrees; `i16::MIN` means missing.
- **Corr**: Pearson correlation between the photo skyline's elevations and the horizon's, over kept columns. Shape agreement independent of offset and scale.
- **DEM**: digital elevation model. Here nine USGS 1/3″ tiles cropped to three counties plus 50 km (`wasatch`, 29,598 × 24,549 cells, ~9.26e-5° ≈ 10 m spacing) as ENVI int16; see [[locator/data/three-counties]].
- **Field**: horizons for a lattice of places, stored as an HZN2 file. See [[locator/data/horizon-field]].
- **FOV**: horizontal field of view of the lens, degrees.
- **gsigma / hsigma**: standard deviation (relief) of the photo skyline's elevations and of the horizon's, over kept columns, degrees.
- **Kappa**: mean cosine of a lens's column offsets; how much one degree of camera pitch lifts the skyline on average. See [[locator/camera/lens-tilts]].
- **Lens**: one FOV × roll, projected at every whole pitch from −5° to 30°.
- **Loss**: degrees; lower is better. See [[locator/scoring/loss]].
- **Place**: a lat/lon with ground, scored as a possible camera spot.
- **Refinement**: fresh horizons on finer grids around the best places. See [[locator/search/refinement]].
- **Screen**: the cheap first pass over every place. See [[locator/screen/screen-templates]].
- **Screen rank / truth rank**: in the bench, the 1-based rank of the first row within 1 km of the true spot, in the screen and in the final rows.
- **Template**: the photo skyline through one lens and tilt, resampled to one value per whole degree, for the screen.
- **Trimmed RMSE**: RMS of residuals about their mean after dropping the worst 2% by distance from the median. See [[locator/scoring/measure]].
- **Yaw**: the azimuth the optical axis points at.
