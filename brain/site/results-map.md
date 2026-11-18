---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - src/lib/ResultsMap.svelte#.bindTooltip("The ridge its skyline sits on")
  - src/lib/ResultsMap.svelte#map.fitBounds(
  - src/routes/results/+page.svelte#aria-label="Previous photo"
---

# Results map

`/results` shows one bench photo at a time (previous/next buttons and a strip of thumbnails), because all eight at once overlapped into noise.

For the selected photo the map draws:

- the true spot (blue dot) and the search's guess (dot coloured by error), joined by a dashed line;
- the view wedge: a filled polygon from the guess out to the [[locator/camera/outline|ridge]], the ground each skyline degree was matched to, so the wedge ends at the horizon the photo actually shows, not at an arbitrary radius;
- the ridge itself as a solid line.

The map is centred on the guess: bounds are symmetric around it, wide enough to hold the truth and the whole ridge, with a maximum zoom of 14. Earlier fitting to the bounding box put Timpanogos off-centre.
