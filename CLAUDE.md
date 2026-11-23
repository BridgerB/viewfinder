# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Viewfinder works out where a photograph of a mountain skyline was taken by comparing the skyline in the photo against terrain horizons computed from USGS elevation data. It is two programs that only meet through JSON files:

- `sight/` — a Rust workspace (the `sight` CLI) that does all the heavy computation: horizon ray-walks over the DEM, skyline tracing, scoring every candidate place.
- `src/` — a SvelteKit 3 site (Svelte 5, `@sveltejs/adapter-cloudflare`, deployed as a Cloudflare Worker) that lets anyone upload a photo, see where it was taken, and share the result. It forwards searches to `sight serve` and only draws the answer; it does no terrain or camera maths itself.

`brain/` is the project's knowledge base: why every number and decision is what it is, with each claim cited to source text. Read `brain/_index.md` before changing the locator; run `node brain/check.ts` after (see `brain/operating-manual.md`).

## Site commands

```sh
pnpm dev            # vite dev on port 4372
pnpm build          # wrangler types --check, then vite build -> .svelte-kit/cloudflare
pnpm preview        # wrangler dev on the built worker, port 4173
pnpm check          # wrangler types --check + svelte-kit sync + svelte-check
pnpm lint           # biome check (format + lint + import order), no writes
pnpm format         # biome format --write
pnpm gen            # wrangler types: regenerate worker-configuration.d.ts after editing wrangler.jsonc
pnpm test           # vitest (unit + component) and playwright e2e
```

Lefthook runs on every commit (`lefthook.yml`): Biome on staged files, `cargo fmt --check` and clippy when Rust changed, the brain checker, `pnpm check`, and `pnpm build`. pnpm 11 reads `allowBuilds` from `pnpm-workspace.yaml`; lefthook, esbuild, workerd and sharp must stay allowed there.

Biome (`biome.json`) is the default config plus two things: it skips `worker-configuration.d.ts`, Markdown (its Markdown formatter does not understand YAML frontmatter and mangles the brain notes) and the generated `src/lib/assets/*.json`; and in `.svelte` files it turns off the unused-import/variable rules, because Biome only reads the `<script>` block and cannot see template usage.

Imports use the package `imports` alias `#lib/...` (not `$lib`), with the `.ts` extension for TypeScript modules: `import { verdict } from "#lib/locate.ts"`.

The base also carries the `sv` add-on demos: Better Auth on D1 via Drizzle (`src/lib/server`, `drizzle.config.ts`, `.env.example`) under `/demo`.

The Worker bundle includes the imported JSON, `results.json` and `sandy-example.json` (`pnpm exec wrangler deploy --dry-run` prints the size; the free-plan limit is 3 MiB).

## Deployment

Two Workers on Cloudflare (Workers Paid):

- `viewfinder` (root `wrangler.jsonc`): the site, at https://viewfinder.bridgerb.com (custom domain, in `routes`; https://viewfinder.bridgerb.workers.dev also stays on). Meant to deploy from GitHub via Workers Builds (build `pnpm build`, deploy `pnpm exec wrangler deploy`); by hand it is `pnpm deploy`. Bindings: D1 `viewfinder` (migrations in `drizzle/`, applied with `wrangler d1 migrations apply DB --remote`), R2 `viewfinder-photos` (`PHOTOS`, shared photos), and the service binding `SIGHT`. Secrets `ORIGIN` and `BETTER_AUTH_SECRET` are set with `wrangler secret put`.
- `viewfinder-sight` (`cloud/sight/`): the locator. A Durable Object starts one container running `sight serve` (standard-4: 4 vCPU, 12 GiB), waits for `/health`, forwards requests, and lets it stop after 15 idle minutes; a cold start takes about 5 s. It has no public URL (`workers_dev: false`); the site's `/api/sight/*` route forwards to it, and in `pnpm dev` the same route forwards to `pnpm sight` on port 4373. Deploy it by hand with `pnpm deploy:sight` (Docker running): the image bakes in the grid (1.45 GB), the fields and the sky model from the gitignored `data/`, so it cannot build from Git. A running container keeps its old image until it next stops.

The image (`cloud/sight/Dockerfile`, context = repo root, filtered by `/.dockerignore`) cross-compiles `sight` to x86-64 with `target-cpu=x86-64-v3`: without FMA, every `mul_add` in the screen becomes a software call and a search takes ~36 s. HEIC uploads are converted by libheif's `heif-convert` in the container (`sips` locally).

## Locator commands

Run from the repository root; build once with `cargo build --release` in `sight/`. `GRID` is the ENVI path _without_ `.hdr`.

```sh
sight/target/release/sight info    GRID
sight/target/release/sight build   GRID OUT STEP_M [SOUTH NORTH WEST EAST]   # write a horizon field (.hzn)
sight/target/release/sight build-rough GRID OUT 500 3000 300               # 500 m field where relief within 3 km ≥ 300 m
sight/target/release/sight search  GRID FIELDS IMAGE SURFACE.json MATCH.json [--overlay JPG] [--skyline JSON] [--fov DEG]
sight/target/release/sight around  GRID IMAGE LAT LON [--skyline] [--fov] [--step 125] [--reach 1000]   # diagnostic
sight/target/release/sight ridges  GRID CAMERAS.txt OUT.json   # ridge points for "id lat lon yaw fov" lines
sight/target/release/sight trace   IMAGE
sight/target/release/sight serve   GRID FIELDS [--host 0.0.0.0] [--port 4373] [--sky MODEL]   # HTTP API for the upload page (pnpm sight)
sight/target/release/sight sky-trace IMAGE MODEL [--overlay OUT.jpg] [--guide SKYLINE.json]   # the sky-model trace, drawn on the photo
sight/target/release/sight rewalk  GRID FIELD OUT   # the same places walked again on another grid
sight/target/release/sight trace-box IMAGE X0 Y0 X1 Y1 [--overlay OUT.jpg]   # the box tracer, drawn on the photo
```

`--sky data/models/skyseg.onnx` (on `serve`, `bench`, `sky-trace`) traces with a sky-segmentation network (U²-Net, run by tract) and a dynamic-programming path instead of the colour rules: it reads snow, cloud and sunset skies the colour rules get wrong (`brain/locator/sky/sky-model.md`). `pnpm sight` and the container use it. `bench --no-guides` ignores the hand-traced skylines to test the tracer. `--skyline` replaces the tracer with hand-traced `[x, y]` pixel pairs; `--fov` pins the lens from EXIF (35 mm-equivalent f → 2·atan(18/f)). The tracer takes the first non-sky row per column, so trees, poles and roofs in front of the ridge break it; use `--skyline` for such photos.

`GRID` is `data/elevation/envi/wasatch`. `search` takes several fields comma-separated. Use both: `data/elevation/field/wasatch.hzn,data/elevation/field/wasatch-rough500.hzn`. Near mountains the good fit is only ~125 m wide, so the 2 km field alone misses it (Little Cottonwood Canyon landed 51 km off). Refinement is 250 m around the best 60, then 100 m around the best 15.

Each pass ranks places in two stages (`search::rank`): a screen that sweeps every yaw of each lens × tilt template over every place with prefix sums and register-tiled cross terms, then the full 3D-projected trimmed-error search on the best `max(200, 3%)` places within ±8° of the screen's yaw. A search takes ~1.5–2 s.

Performance work is logged in `sight/PERF.md`. Before changing scoring, run `bench` (all known-location photos; `--quick` for three) and the tests; `golden` must still match unless a change is meant to alter results:

```sh
sight/target/release/sight bench GRID FIELDS sight/bench/cases.toml [--quick] [--sky MODEL] [--no-guides]
sight/target/release/sight profile GRID FIELDS IMAGE [--places 32000] [--reps 3]   # also splits screen vs full search
cargo test --release && cargo bench -p sight-core     # from sight/
```

`sight-core` is pure (no files, clocks or `unsafe`; clippy enforces it), `sight-io` does all file formats, `sight-cli` is the binary. Every tunable number is in `sight-core/src/config.rs` with a comment saying what wrong answer it prevents. The old Clojure implementation in `sight/src`, `sight/test` and `sight/deps.edn` is kept only as the parity reference until it is removed.

Toolchain: `flake.nix` provides everything: `nix develop` (or `nix develop -c pnpm dev`) gives Node 24, pnpm (which switches to the `packageManager` version, 11.5.3), Rust with clippy and rustfmt, GDAL, and libheif with a `heif-convert` shim (libheif 1.18+ calls it `heif-dec`). The repository moved from macOS arm64 to NixOS x86-64: after a move, rerun `pnpm install` (native packages are per platform) and `cargo build --release`. On macOS the old Xcode note still applies: if linking fails with `unable to find sdk: 'macosx'`, run cargo with `PATH=/usr/bin:$PATH`.

## Data that is not in git

`/data/` is gitignored and must exist locally for `sight` to run:

- `data/elevation/tif/USGS_13_n{40,41,42}w{111,112,113}.tif` — nine USGS 1/3 arc-second tiles, from `https://prd-tnm.s3.amazonaws.com/StagedProducts/Elevation/13/TIFF/current/` (public, no sign-in).
- `data/elevation/envi/wasatch` (+ `.hdr`) — the tiles mosaicked and cropped with GDAL to Salt Lake, Utah and Davis counties plus 50 km, ENVI int16 band-sequential; the reader rejects anything else. Commands in `brain/locator/data/three-counties.md`.
- `data/elevation/field/wasatch.hzn`, `wasatch-rough500.hzn` — the two horizon fields over the three counties (built with `build`, `build-rough`, `rewalk`, `scripts/clip-field.ts` and `scripts/merge-fields.ts` against `data/elevation/counties.geojson`).
- `data/elevation/envi/n41w112`, `field/n41w112*.hzn` — the original single tile and its fields, still used by `golden`.
- `data/models/skyseg.onnx` — the sky-segmentation network (Hugging Face `JianyuanWang/skyseg`, MIT, 176 MB).
- `data/shares/` — shares downloaded from D1 and R2 for the dev-only `/dev` review page (`rows.json` + `{id}.jpg`). People's photos: never commit or publish them.
- `data/bench/` — bench photos, guided skylines and run history.

## Locator design

The search compares the photo's skyline against stored horizons; nothing in it names a town or a summit, and fixes belong in the general loss. A pixel is a 3D direction: roll about the optical axis, then pitch, then yaw. Pitch is part of the projection, because in a wide frame a tilted camera's rows are not lines of equal elevation; treating elevation as `pitch + atan(y/f)` is wrong by several degrees at the edges of an ultrawide frame. The trimmed RMSE is scaled by `ref_fov / fov` (pixels, not degrees): a trace is off by pixels, and counting in degrees rewarded the narrowest lens, which sent Sandy and Timpanogos kilometres away whenever narrow lenses were allowed. The loss is in degrees: trimmed RMSE + log relief gap + 2×(1 − correlation), plus penalties for first-step berms and ground that falls away ahead of the camera. The brain has a note per term.

## Site

The look is the "editorial" direction chosen from ten design studies: white paper, Newsreader headlines, DM Sans text, JetBrains Mono for links and codes, one terracotta accent (`--accent`). Every token is in `src/app.css`; the trace (yellow), horizon (cyan) and box (pink) colours appear only on photos and their legends. The header (`+layout.svelte`) links Examples, How it works and New photo.

Routes:

- `/` — the app. Upload a photo (JPEG, PNG, or HEIC: `sips` on macOS, `heif-convert` on Linux and in the container), watch a working screen (steps, clock, progress bar), get the result. The page calls the locator through `/api/sight/*`; `sight serve` keeps the grid and fields in memory and holds the last 8 uploads. "New photo" in the header starts over (`afterNavigate`).
  - Upload traces the whole photo and searches at once; nothing is confirmed unless the verdict is weak. The verdict (`verdict` in `#lib/locate.ts`) is good only if the loss is low, the top 5 candidates agree within 3 km, and the skyline has shape (`shapeOf` ≥ 0.4% of the photo height: a trace that slid along a smooth line fits many places with a deceptively low loss).
  - A pink box always sits around the ridge the search read (`fitBox`), with corner handles; dragging an edge or corner calls `POST /retrace` with the box and searches again. The box tracer (`sky::trace_region`) takes, per column, the topmost step down in brightness + blueness of at least `MIN_STEP`; unlike the whole-photo tracer it does not need clearly blue sky, so it handles hazy or evening photos. The drag surface is 14 px larger than the photo so handles on the photo's border can be grabbed.
  - Errors with nothing else to show (unreadable file, server down, expired upload) get a whole screen; a search that fails on a traced photo shows the photo and box to fix.
- `/p/{uuid}` — a shared result, read-only, unlisted (`noindex`) but open to anyone with the link, with Open Graph tags for link previews. Every search on `/` saves one through `POST /api/shares`: the upright photo goes to R2 (`PHOTOS`, `photos/{id}.jpg`) and the result to the D1 table `share`; the address bar switches to the link. A missing id renders `+error.svelte` as "This link is gone."
- `/examples` — every bench photo on a map, one at a time: true spot, Viewfinder's answer, and the view wedge out to the ridge the skyline sits on.
- `/how` — trace, compare, pin, adjust, drawn from a real search of `static/sandy.jpg`.
- `/locate` and `/results` redirect (308) to `/` and `/examples`.
- `/dev` — dev only: every share downloaded into `data/shares/`, grouped with its retries, with a "Rerun locally" button, notes to copy into a chat, and a map per photo (`src/routes/dev/TruthMap.svelte`) to mark where it was really taken and which way it faced, saved to `data/shares/truth.json`. `node scripts/rerun-shares.ts` sends every group through the local locator (`pnpm sight` behind `pnpm dev`) and saves the answers to `data/shares/reruns.json`, which the page shows. A Vite middleware (`devShares` in `vite.config.ts`) serves that folder at `/dev-data/` under `pnpm dev` only, so the photos never enter a build; in a build the page is an empty component (`devPageOnlyInDev`) and its `+page.ts` answers 404. Refresh the folder with `wrangler d1 execute DB --remote --json --command "select * from share"` into `rows.json` and `wrangler r2 object get viewfinder-photos/photos/{id}.jpg --remote --file data/shares/{id}.jpg`.

`ResultView.svelte` is the result layout shared by `/` and `/p` (verdict, place name, photo, coordinates, map, camera facts, top 5, share). `PhotoFrame.svelte` draws the photo with its overlays in the photo's own pixel coordinates; the cyan horizon is the locator's `outline`, already in pixels. `places.ts` names the place from a built-in list of towns, so no geocoding service sees anyone's coordinates. `LocateMap`, `ResultsMap` and `CoverageMap` are Leaflet maps on OpenStreetMap tiles (desaturated by the `.muted-tiles` class); `basemap.ts` loads Leaflet on demand, because it needs `window`, and `leaflet.heat` needs `L` on `globalThis` first (`src/lib/leaflet-heat.d.ts` declares it, since it ships no types).

Where each asset comes from:

| Asset | Source |
| --- | --- |
| `results.json` | assembled from bench runs and `sight ridges` |
| `sandy-example.json` | one `POST /trace` + `POST /locate` of `static/sandy.jpg` on `sight serve` with `fov` 100, coordinates rounded |

Photos live in `static/` and are referenced by absolute URL; the JSON is imported through Vite so it is content-hashed. `static/mount-timpanogos.jpg` and `static/sandy.jpg` are bench photos (`sight/bench/cases.toml`), and `sandy.jpg` is also the "Try a sample" photo. `static/results/LICENSE.md` records the licence of every Wikimedia photo; the repository is public, so only add photos that may be redistributed and that show no one who has not agreed to it.

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
