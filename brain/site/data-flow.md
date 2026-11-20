---
type: area
status: verified
verifiedAgainst: bc9323c, 2026-11-19
sourceRefs:
  - sight/crates/sight-io/src/lib.rs#/// Ridges per camera id as `{"id":[[lat,lon],…]}`.
  - src/routes/examples/+page.svelte#import resultsData from "#lib/assets/results.json";
  - src/routes/how/+page.svelte#import example from "#lib/assets/sandy-example.json";
---

# Site data flow

A person's own search comes live from the locator through `/api/sight/*`, and is saved for sharing in D1 (the result) and R2 (the photo). Everything else is JSON in `src/lib/assets/`, imported through Vite so it is content-hashed. Photos live in `static/` and are referenced by absolute URL.

| Asset | Used by | Comes from |
|---|---|---|
| `results.json` | `/examples` | assembled by hand from bench runs and `sight ridges` (truth, guess, error, yaw, fov, pitch, ridge, credit) |
| `sandy-example.json` | `/how` | one `POST /trace` and `POST /locate` of `static/sandy.jpg` on `sight serve` with the lens set to 100°, coordinates rounded |

The output formats of `sight-io` and of `sight serve` are the contract. Changing one means updating the page that reads it.

Licences: the bench photos shown on `/examples` are Wikimedia Commons images, copied to `static/results/` with credits in `static/results/LICENSE.md` and `results.json`; one photo with identifiable children was removed for the public repo.
