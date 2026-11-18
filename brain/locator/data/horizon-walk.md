---
type: area
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/horizon.rs#fn walk(&self, dem: &Dem<'_>, cfg: &HorizonConfig, azimuth: usize) -> (f64, f64) {
  - sight/crates/sight-core/src/horizon.rs#let slope = ((f64::from(z) - self.eye) - drop) / d;
  - sight/crates/sight-core/src/horizon.rs#if d > cfg.max_distance_m || (hit && (headroom - (d * d) / two_r) / d < best) {
  - sight/crates/sight-core/src/horizon.rs#growth: 1.0 + cfg.step_deg.to_radians().tan(),
  - sight/crates/sight-core/src/config.rs#radius_m: (6_371_000.0 * 7.0) / 6.0,
  - sight/PERF.md#Horizon walk compares slopes.
---

# Horizon walk

For each whole azimuth 0..359, walk a ray outward from the place and keep the sample that rises highest above the eye. That angle is the skyline the camera would see in that direction.

## Geometry

- Eye height = bilinear ground + 2 m.
- A sample at distance d with elevation z is seen at slope `(z − eye − d²/2R) / d`. The `d²/2R` term is earth curvature: the ground falls away below the tangent plane. R is 7/6 of the earth's radius (6371 km), the standard allowance for atmospheric refraction bending light down (~13% of curvature). At 50 km curvature alone is 196 m; with refraction 168 m.
- The local frame is flat: 111,320 m per degree of latitude, times cos(lat) for longitude. Good to well under a cell inside 50 km.

## Stepping

- Starts at 400 m: closer ground is the slope underfoot, which would otherwise be the "horizon" in every direction on a hillside.
- Each step multiplies d by `1 + tan(0.4°)`, so each step subtends about 0.4° of elevation change for level ground: dense near (where a cell is a big angle) and sparse far. About 700 steps reach 50 km (ln(50000/400) / ln(1.00698)).
- Stops at 50 km (beyond that a ridge is haze) or as soon as even the highest ground in Utah (4123 m, Kings Peak) at the current distance could not beat the best slope: `(4123 − eye − d²/2R)/d < best`. Since that bound falls with d, the walk ends early behind any big ridge.

## Slopes, not angles

The walk compares rise-over-run and takes one `atan` at the end. atan is monotonic, so the winner is identical; it removed two `atan2` per step. In the Clojure version this alone took the rugged field build from 37 s to 14 s with all 10.7 M values identical.

## Output

Angle in centidegrees, rounded with Java's rule, `i16::MIN` if the ray never touched ground; distance in meters to the winning sample, as f32. The distance is what makes [[locator/camera/outline|ridge drawing]] and the floor penalty possible.
