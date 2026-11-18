---
type: area
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - sight/crates/sight-io/src/lib.rs#/// Every scored place as `[{"lat","lon","loss"}]`, the heatmap the match page draws.
  - sight/crates/sight-io/src/lib.rs#/// Ridges per camera id as `{"id":[[lat,lon],…]}`.
  - src/routes/sandy/match/+page.svelte#import skylineData from '$lib/assets/sandy-skyline.json';
  - src/routes/results/+page.svelte#import resultsData from '$lib/assets/results.json';
  - CLAUDE.md#No generator is checked in.
---

# Site data flow

All data is JSON in `src/lib/assets/`, imported through Vite so it is content-hashed. Photos live in `static/` and are referenced by absolute URL.

| Asset | Used by | Comes from |
|---|---|---|
| `timpanogos.json` | `/timpanogos`, `/timpanogos/match` | `scripts/generate-timpanogos.ts` (old TS generator; its DEM path no longer matches `data/`) |
| `timpanogos-search.json`, `sandy-search.json` | match pages' heatmaps | `sight search` surface output: `[{lat, lon, loss}]` |
| `sandy-match.json` | `/sandy/match` | `sight search` match output: winning camera + outline pixels |
| `timpanogos-match.json`, `sandy-skyline.json` | match pages | no generator checked in; shape is `RawMatch` in `photo-match.ts` and the Sandy page |
| `results.json` | `/results` | assembled by hand from bench runs and `sight ridges` (truth, guess, error, yaw, fov, pitch, ridge, credit) |

The output formats of `sight-io` (`write_surface`, `write_match`, `write_ridges`) are the contract. Changing one means updating the page that reads it.

Licences: the bench photos shown on `/results` are Wikimedia Commons images, copied to `static/results/` with credits in `static/results/LICENSE.md` and `results.json`; one photo with identifiable children was removed for the public repo.
