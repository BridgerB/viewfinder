---
type: area
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - src/lib/photo-match.ts#const unproject = (lens: Lens, ray: Ray): Pixel => {
  - src/lib/photo-match.ts#const searchHeat = (hits: readonly SearchHit[]) => {
  - src/routes/timpanogos/match/+page.svelte#import MatchMap from '$lib/MatchMap.svelte';
---

# Match pages

- `/timpanogos`: an animated 360° orbit of horizons seen from an 8.9 km ring around the peak (360 viewpoints).
- `/timpanogos/match`: the photo with its traced skyline and the model horizon overlaid, the per-ring-angle shape match, the elevation-angle profile, and the search heatmap.
- `/sandy/match`: the Sandy photo with traced skyline and heatmap.

`src/lib/photo-match.ts` holds all the pure logic: `project`/`unproject` mirror the locator's [[locator/camera/camera-ray]] (roll, pitch, yaw in 3D) so the horizon overlay lands where the search measured it; SVG point strings; `searchHeat` turns losses into heat weights.

`src/lib/MatchMap.svelte` is the Leaflet + `leaflet.heat` map. Leaflet is imported dynamically inside an action because it needs `window`, and `leaflet.heat` needs `L` on `globalThis` before it loads.
