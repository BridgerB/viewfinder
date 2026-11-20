---
type: area
status: verified
verifiedAgainst: bc9323c, 2026-11-19
sourceRefs:
  - src/routes/+page.svelte#// No confirmation when the match is good: the result asks only if it is weak.
  - src/lib/ResultView.svelte#const place = $derived(level === "poor" ? "Unclear" : placeName(at));
  - src/lib/PhotoFrame.svelte#<svg viewBox="0 0 {width} {height}" preserveAspectRatio="none" aria-hidden="true">
  - src/lib/places.ts#const placeName = (at: LatLon) => {
  - src/lib/locate.ts#const searchHeat = (hits: Located["surface"]) => {
---

# The locate app

The site is one product. The design is the "editorial" direction picked from ten design studies: white paper, Newsreader for headlines, DM Sans for text, one terracotta accent. The tokens are in `src/app.css`.

Pages:

- `/`: the upload. Choosing a photo shows it at once, then a working screen (steps, a clock, a progress bar), then the result. A good match needs no confirmation; only a weak one asks the person to adjust the box. Errors get a whole screen when there is nothing else to show (unreadable file, server down, expired upload). A search that fails on a traced photo shows the photo with the box so it can be fixed.
- `/p/{id}`: a saved result, read-only, with link-preview metadata. Every search on `/` saves one and switches the address bar to it.
- `/examples`: the bench photos ([[results-map]]).
- `/how`: four steps (trace, compare, pin, adjust) drawn from a real search on the Sandy sample, saved in `sandy-example.json`.
- `/locate` and `/results` redirect to `/` and `/examples`. The Timpanogos orbit and the two match pages were removed.

`ResultView` is the result layout shared by `/` and `/p`: verdict and place name, the photo, coordinates, the map, camera facts, the top five spots, and the share link. The place name comes from a built-in list of towns (`places.ts`), so no geocoding service sees anyone's coordinates.

`PhotoFrame` draws the overlays in the photo's own pixel coordinates (an SVG with the photo's size as its viewBox): the traced skyline in yellow, the terrain horizon at the answer in cyan, and the box in pink. The horizon is the locator's `outline`, already projected to pixels by [[locator/camera/outline]], so the site does no camera maths of its own. The drag surface sits 14 px outside the photo so a box edge on the photo's border can still be grabbed.

The map heat weights each searched place by `exp(−(loss − best) / 0.06)` and drops places under 0.15, so it shows only the plausible area. Map tiles are OpenStreetMap, desaturated in CSS so the pin, wedge and heat carry the colour.
