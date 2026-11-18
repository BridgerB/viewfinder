---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-core/src/camera.rs#pub fn ray(width: f64, height: f64, f: f64, roll: f64, pitch: f64, x: f64, y: f64) -> (f64, f64) {
  - sight/crates/sight-core/src/camera.rs#pub fn focal(width: f64, fov: f64) -> f64 {
  - src/lib/photo-match.ts#const project = (lens: Lens, pixel: Pixel): Ray => {
---

# Camera ray

A pinhole camera. Focal length in pixels `f = (W/2) / tan(fov/2)`. A pixel (x, y), y down, becomes a direction:

1. Centre it: `rx = x − W/2`, `ry = H/2 − y` (up positive).
2. **Roll** about the optical axis: `xr = rx·cos r − ry·sin r`, `yr = rx·sin r + ry·cos r`.
3. **Pitch** about the horizontal axis: `up = yr·cos p + f·sin p`, `forward = f·cos p − yr·sin p`.
4. Azimuth offset from the axis `atan2(xr, forward)`; elevation `atan2(up, hypot(xr, forward))`. Yaw is added later as a plain azimuth shift.

## Why pitch is inside the projection

The old model said elevation = `pitch + atan(y/f)` and offset = `atan(x/f)`. That holds only on the vertical centre line. In a wide frame tilted up, a row of pixels is not a line of equal elevation: the edges of the row point lower than its middle, because the tilted image plane is further from level out there. At 100° FOV and 14° of pitch the error at the frame edges is several degrees, which is bigger than the whole skyline relief of a distant range. That error sent the Sandy photo to Millcreek; with the 3D projection it lands 0.22 km from the truth.

The site's `photo-match.ts` mirrors this function (`project` / `unproject`) so the overlays draw where the search measured: [[site/match-pages]].
