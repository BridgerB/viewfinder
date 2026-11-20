---
type: area
status: verified
verifiedAgainst: bc9323c, 2026-11-19
sourceRefs:
  - src/lib/ResultsMap.svelte#.bindTooltip("The ridge in the photo")
  - src/lib/ResultsMap.svelte#map.fitBounds(
  - src/routes/examples/+page.svelte#const level = (errorKm: number) =>
---

# Examples map

`/examples` (formerly `/results`) shows one bench photo at a time, picked from a row of chips or with previous/next and the arrow keys, because all eight at once overlapped into noise. Each chip shows the miss in green within 1 km, amber within 10 km, red beyond.

For the selected photo the map draws:

- the true spot (ink dot) and Viewfinder's answer (terracotta dot), joined by a dashed line;
- the view wedge: a filled polygon from the answer out to the [[locator/camera/outline|ridge]], the ground each skyline degree was matched to, so the wedge ends at the horizon the photo actually shows, not at an arbitrary radius;
- the ridge itself as a solid line.

The map is centred on the answer: bounds are symmetric around it, wide enough to hold the truth and the whole ridge, with a maximum zoom of 14. Earlier fitting to the bounding box put Timpanogos off-centre.
