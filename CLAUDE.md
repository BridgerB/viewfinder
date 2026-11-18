# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Viewfinder works out where a photograph of a mountain skyline was taken by comparing the skyline in the photo against terrain horizons computed from USGS elevation data. It is two programs that only meet through JSON files:

- `sight/` — a Clojure CLI that does all the heavy computation (horizon ray-walks over the DEM, skyline tracing, scoring every candidate place).
- `src/` — a SvelteKit (Svelte 5, adapter-node) site that only _displays_ pregenerated results bundled from `src/lib/assets/*.json`. There are no server routes and no runtime computation.

## Commands

The dev shell comes from `flake.nix` via direnv (`use flake`): node 22, gdal, jdk, clojure.

```sh
pnpm dev            # vite dev; port is derived from the project name by @bridgerb/port-from-name (4372)
pnpm build
pnpm check          # svelte-kit sync + svelte-check
pnpm lint           # prettier --check + eslint
pnpm format
```

The lockfile checked in is still `package-lock.json`. There are no tests and no CI workflows in this repo.

The Clojure tool runs from `sight/` (paths are relative to that directory):

```sh
cd sight
clojure -M:run info    ../data/elevation/envi/n41w112
clojure -M:run time    GRID LAT LON                       # time one horizon
clojure -M:run trace   IMAGE                              # print the traced skyline as frame fractions
clojure -M:run build   GRID OUT STEP_M [SOUTH NORTH WEST EAST]   # write a horizon field (.hzn)
clojure -M:run query   GRID FIELD IMAGE                   # top 16 places for a photo
clojure -M:run surface GRID FIELD IMAGE OUT.json          # every place's best loss -> heatmap JSON
clojure -M:run search  GRID FIELD IMAGE SURFACE.json MATCH.json [OVERLAY.jpg [SKYLINE.json|- [FOV]]]
                                                          # field, then fresh 500 m and 125 m grids around the best;
                                                          # SKYLINE replaces the tracer, FOV (from EXIF) pins the lens
clojure -M:run around  GRID IMAGE LAT LON SKYLINE.json|- FOV|-   # score a 125 m grid within 1 km of a point (diagnostic)
clojure -M:run build-rough GRID OUT 500 3000 300          # 500 m field where relief within 3 km ≥ 300 m (~30k places, ~40 s)
clojure -M:run ridges  GRID CAMERAS.txt OUT.json          # per-degree ridge points for "id lat lon yaw fov" lines
```

`search` takes several fields comma-separated. Use both: `data/elevation/field/n41w112.hzn,data/elevation/field/n41w112-rough500.hzn`. Near mountains the good fit is only ~125 m wide, so the 2 km field alone misses it (Little Cottonwood Canyon landed 51 km off). Refinement is 250 m around the best 60, then 100 m around the best 15. A full search is ~9 minutes.

`GRID` is the ENVI path _without_ `.hdr` (the header is read from `GRID.hdr`).

## Data that is not in git

`/data/` is gitignored and must exist locally for `sight` to run:

- `data/elevation/tif/USGS_13_n41w112.tif` — USGS 1/3 arc-second source tile.
- `data/elevation/envi/n41w112` (+ `.hdr`) — the same tile converted with GDAL to ENVI int16, band-sequential. `dem/read-envi` rejects anything else. The conversion command is not recorded in the repo.
- `data/elevation/field/n41w112.hzn` — a built horizon field.

`scripts/generate-timpanogos.ts` (the older TypeScript generator, using `@bridgerb/horizon` + `gdal-async`) points at `data/n41w112_30m.tif`, which no longer matches the layout above.

## Pipeline

1. **DEM** (`dem.clj`) — loads the whole ENVI grid into a short array. Off-grid and nodata are `nil`, never zero, so a ray can tell it left the loaded land.
2. **Horizon** (`horizon.clj`) — from one place, for each whole degree of azimuth (0 = north, clockwise), the highest elevation angle the ground reaches, corrected for earth curvature + refraction. The walk starts 400 m out (so ground underfoot can't hide the ridge), steps geometrically, stops at 50 km, and exits early once no possible ground could beat the current best.
3. **Field** (`field.clj`) — horizons for a lattice of places, packed into a binary `.hzn` file (magic `HZN2`, little-endian; per place: lat, lon, ground, 360 centidegree angles as shorts, 360 distances as floats). Missing rays are the `Short/MIN_VALUE` sentinel. Built once, read on every query. Changing the record layout means bumping the magic and rebuilding.
4. **Sky** (`sky.clj`) — traces the skyline of a JPEG/PNG: per column, the first row that is not sky/haze and does not return to sky within `rock-run` pixels; then despike (poles) and median-fill; decimated to ~160 columns.
5. **Score** (`score.clj`) — for every place × field of view (40–115° by 5, a 50 mm lens through a phone's ultrawide) × roll (−2/0/2°) × yaw (every degree), compares the projected photo skyline to the stored horizon. A pixel is a 3D direction: roll about the optical axis, then pitch, then yaw (`ray`). Pitch is part of the projection, because in a wide frame a tilted camera's rows are not lines of equal elevation; each lens is pre-projected at every whole degree of pitch, the level camera guesses the tilt per yaw, and only the last ≤1° is left to the mean residual. Treating elevation as `pitch + atan(y/f)` is wrong by several degrees at the edges of an ultrawide frame and was why the Sandy photo used to land in Millcreek. The trimmed RMSE is scaled by `ref-fov / fov` (pixels, not degrees): a trace is off by pixels, and counting in degrees rewarded the narrowest lens, which sent Sandy and Timpanogos kilometres away whenever narrow lenses were allowed. The loss is in degrees: trimmed RMSE + log relief gap + 2×(1 − correlation), plus penalties for near-ground windows, first-step berms, and ground that falls away ahead of the camera. The namespace docstring and the comments on each constant record _why_ each term exists (what wrong answer it fixed) — read them before tuning, and keep them current when changing the loss.
6. **Wave** (`wave.clj`) — the parallelism primitive for both build and scoring: batches of 2000 places on a fixed processor-count pool.

`main.clj` states the design constraint: "The search is the field. Nothing in here names a town or a summit." Keep `sight` free of location-specific special cases; fixes belong in the general loss.

The hot loops in `horizon.clj` and `score.clj` run with `*warn-on-reflection*` and `*unchecked-math* :warn-on-boxed`; type hints are placed in `let` bindings because primitive-typed fns can't take that many array args. Maps use namespaced keys (`:dem/…`, `:horizon/…`, `:field/…`, `:sky/…`, `:place/…`, `:camera/…`).

## Site

Routes:

- `/timpanogos` — animated 360° orbit of horizons seen from an 8.9 km ring around the peak (`timpanogos.json`, 360 viewpoints, from `scripts/generate-timpanogos.ts`).
- `/timpanogos/match` — photo with traced skyline and the model horizon overlaid, per-ring-angle shape match, elevation-angle profile, and the search heatmap.
- `/sandy/match` — a second photo: traced skyline + search heatmap only.

`src/lib/photo-match.ts` holds all the pure logic for the match pages (camera projection/unprojection that mirrors `score.clj`'s `ray`, SVG point strings, heat weighting). `src/lib/MatchMap.svelte` is the Leaflet + `leaflet.heat` map; Leaflet is imported dynamically inside an action because it needs `window`, and `leaflet.heat` needs `L` on `globalThis` first.

Where each asset comes from:

| Asset                                          | Source                                                                                                     |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `timpanogos.json`                              | `scripts/generate-timpanogos.ts`                                                                           |
| `timpanogos-search.json`, `sandy-search.json`  | `sight` `surface` command (`[{lat, lon, loss}]`)                                                           |
| `timpanogos-match.json`, `sandy-skyline.json`  | No generator is checked in. Their shapes are defined by `RawMatch` in `photo-match.ts` and the sandy page. |

Photos live in `static/` and are referenced by absolute URL; the JSON is imported through Vite so it is content-hashed.

## Svelte MCP

You are able to use the Svelte MCP server, where you have access to comprehensive Svelte 5 and SvelteKit documentation. Here's how to use the available tools effectively:

### 1. list-sections

Use this FIRST to discover all available documentation sections. Returns a structured list with titles, use_cases, and paths. When asked about Svelte or SvelteKit topics, ALWAYS use this tool at the start of the chat to find relevant sections.

### 2. get-documentation

Retrieves full documentation content for specific sections. Accepts single or multiple sections. After calling the list-sections tool, you MUST analyze the returned documentation sections (especially the use_cases field) and then use the get-documentation tool to fetch ALL documentation sections that are relevant for the user's task.

### 3. svelte-autofixer

Analyzes Svelte code and returns issues and suggestions. You MUST use this tool whenever writing Svelte code before sending it to the user. Keep calling it until no issues or suggestions are returned.

### 4. playground-link

Generates a Svelte Playground link with the provided code. After completing the code, ask the user if they want a playground link. Only call this tool after user confirmation and NEVER if code was written to files in their project.
