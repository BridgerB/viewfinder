# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Viewfinder works out where a photograph of a mountain skyline was taken by comparing the skyline in the photo against terrain horizons computed from USGS elevation data. It is two programs that only meet through JSON files:

- `sight/` — a Rust workspace (the `sight` CLI) that does all the heavy computation: horizon ray-walks over the DEM, skyline tracing, scoring every candidate place.
- `src/` — a SvelteKit 3 site (Svelte 5, `@sveltejs/adapter-cloudflare`, deployed as a Cloudflare Worker) that only _displays_ pregenerated results bundled from `src/lib/assets/*.json`. The viewfinder pages have no server routes and no runtime computation.

`brain/` is the project's knowledge base: why every number and decision is what it is, with each claim cited to source text. Read `brain/_index.md` before changing the locator; run `node brain/check.ts` after (see `brain/operating-manual.md`).

## Site commands

```sh
pnpm dev            # vite dev (default port 5173)
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

Imports use the package `imports` alias `#lib/...` (not `$lib`), with the `.ts` extension for TypeScript modules: `import * as photoMatch from "#lib/photo-match.ts"`.

The base also carries the `sv` add-on demos: Better Auth on D1 via Drizzle (`src/lib/server`, `drizzle.config.ts`, `.env.example`) under `/demo`.

The Worker bundle includes the imported JSON: `timpanogos.json` (4.2 MB) is the bulk of an ~885 KiB gzipped upload (`pnpm exec wrangler deploy --dry-run` prints the size; the free-plan limit is 3 MiB).

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
```

`--skyline` replaces the tracer with hand-traced `[x, y]` pixel pairs; `--fov` pins the lens from EXIF (35 mm-equivalent f → 2·atan(18/f)). The tracer takes the first non-sky row per column, so trees, poles and roofs in front of the ridge break it; use `--skyline` for such photos.

`search` takes several fields comma-separated. Use both: `data/elevation/field/n41w112.hzn,data/elevation/field/n41w112-rough500.hzn`. Near mountains the good fit is only ~125 m wide, so the 2 km field alone misses it (Little Cottonwood Canyon landed 51 km off). Refinement is 250 m around the best 60, then 100 m around the best 15.

Each pass ranks places in two stages (`search::rank`): a screen that sweeps every yaw of each lens × tilt template over every place with prefix sums and register-tiled cross terms, then the full 3D-projected trimmed-error search on the best `max(200, 3%)` places within ±8° of the screen's yaw. A search takes ~1.5–2 s.

Performance work is logged in `sight/PERF.md`. Before changing scoring, run `bench` (all known-location photos; `--quick` for three) and the tests; `golden` must still match unless a change is meant to alter results:

```sh
sight/target/release/sight bench GRID FIELDS sight/bench/cases.toml [--quick]
sight/target/release/sight profile GRID FIELDS IMAGE [--places 32000] [--reps 3]   # also splits screen vs full search
cargo test --release && cargo bench -p sight-core     # from sight/
```

`sight-core` is pure (no files, clocks or `unsafe`; clippy enforces it), `sight-io` does all file formats, `sight-cli` is the binary. Every tunable number is in `sight-core/src/config.rs` with a comment saying what wrong answer it prevents. The old Clojure implementation in `sight/src`, `sight/test` and `sight/deps.edn` is kept only as the parity reference until it is removed.

Toolchain: Node 22+, pnpm 11, Rust (cargo, clippy, rustfmt) and Xcode's macOS SDK. If linking fails with `unable to find sdk: 'macosx'`, a Nix `cc`/`xcrun` is ahead of Apple's on `PATH`; run cargo with `PATH=/usr/bin:$PATH`.

## Data that is not in git

`/data/` is gitignored and must exist locally for `sight` to run:

- `data/elevation/tif/USGS_13_n41w112.tif` — USGS 1/3 arc-second source tile.
- `data/elevation/envi/n41w112` (+ `.hdr`) — the same tile converted with GDAL to ENVI int16, band-sequential. The reader rejects anything else. The conversion command is not recorded in the repo.
- `data/elevation/field/n41w112.hzn`, `n41w112-rough500.hzn` — the two horizon fields.
- `data/bench/` — bench photos, guided skylines and run history.

`scripts/generate-timpanogos.ts` (the older TypeScript generator, using `@bridgerb/horizon` + `gdal-async`, which are not installed) points at `data/n41w112_30m.tif`, which no longer matches the layout above.

## Locator design

The search compares the photo's skyline against stored horizons; nothing in it names a town or a summit, and fixes belong in the general loss. A pixel is a 3D direction: roll about the optical axis, then pitch, then yaw. Pitch is part of the projection, because in a wide frame a tilted camera's rows are not lines of equal elevation; treating elevation as `pitch + atan(y/f)` is wrong by several degrees at the edges of an ultrawide frame. The trimmed RMSE is scaled by `ref_fov / fov` (pixels, not degrees): a trace is off by pixels, and counting in degrees rewarded the narrowest lens, which sent Sandy and Timpanogos kilometres away whenever narrow lenses were allowed. The loss is in degrees: trimmed RMSE + log relief gap + 2×(1 − correlation), plus penalties for first-step berms and ground that falls away ahead of the camera. The brain has a note per term.

## Site

Routes:

- `/timpanogos` — animated 360° orbit of horizons seen from an 8.9 km ring around the peak (`timpanogos.json`, 360 viewpoints, from `scripts/generate-timpanogos.ts`).
- `/timpanogos/match` — photo with traced skyline and the model horizon overlaid, per-ring-angle shape match, elevation-angle profile, and the search heatmap.
- `/sandy/match` — a second photo: traced skyline + search heatmap only.
- `/results` — every bench photo on a map, one at a time: true spot, guess, and the view wedge out to the ridge the skyline sits on.

`src/lib/photo-match.ts` holds all the pure logic for the match pages (camera projection/unprojection that mirrors the locator's `ray`, SVG point strings, heat weighting). `src/lib/MatchMap.svelte` and `src/lib/ResultsMap.svelte` are Leaflet maps; Leaflet is imported dynamically inside an action because it needs `window`, and `leaflet.heat` needs `L` on `globalThis` first (`src/lib/leaflet-heat.d.ts` declares it, since it ships no types).

Where each asset comes from:

| Asset | Source |
| --- | --- |
| `timpanogos.json` | `scripts/generate-timpanogos.ts` |
| `timpanogos-search.json`, `sandy-search.json` | `sight search` surface output (`[{lat, lon, loss}]`) |
| `sandy-match.json` | `sight search` match output |
| `results.json` | assembled from bench runs and `sight ridges` |
| `timpanogos-match.json`, `sandy-skyline.json` | No generator is checked in. Their shapes are defined by `RawMatch` in `photo-match.ts` and the sandy page. |

Photos live in `static/` and are referenced by absolute URL; the JSON is imported through Vite so it is content-hashed. `static/results/LICENSE.md` records the licence of every Wikimedia photo; the repository is public, so only add photos that may be redistributed and that show no one who has not agreed to it.

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
