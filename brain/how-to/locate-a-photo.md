---
type: how-to
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-cli/src/main.rs#/// Where a photograph was taken: writes the heatmap and the winning camera's horizon.
  - sight/crates/sight-io/src/lib.rs#pub fn write_surface(path: &Path, rows: &[Place]) -> Result<(), IoError> {
---

# Locate a photo

From the repository root, with `data/` present ([[locator/data/dem-grid]]):

```sh
cd sight && cargo build --release && cd ..
sight/target/release/sight search \
  data/elevation/envi/n41w112 \
  data/elevation/field/n41w112.hzn,data/elevation/field/n41w112-rough500.hzn \
  static/sandy.jpg  /tmp/surface.json  /tmp/match.json  --overlay /tmp/overlay.jpg
```

- Add `--fov DEG` when the EXIF focal length is known (35 mm-equivalent f → fov = 2·atan(18/f)). It removes the [[concepts/lens-distance-tradeoff]].
- Add `--skyline FILE` (JSON `[x, y]` pairs in image pixels) when the tracer fails: clouds on the ridge, white sky, trees. Check first with `sight trace IMAGE` or the overlay.
- Outputs: the surface JSON (`[{lat, lon, loss}]`, every place given the full search) feeds the heatmap; the match JSON holds the winning camera and its outline; the overlay JPEG draws the traced skyline and the model horizon on the photo.

To show it on the site, copy the JSON into `src/lib/assets/` and wire it as the existing pages do ([[site/data-flow]]).
