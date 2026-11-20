---
type: moc
status: verified
verifiedAgainst: bc9323c, 2026-11-19
---

# Site

SvelteKit 3 (Svelte 5) on `@sveltejs/adapter-cloudflare`, deployed as a Cloudflare Worker. It is one app: upload a photo on `/`, get the place, and share the result at `/p/{id}`. The search itself runs in the locator container ([[locate-app]]); the site only forwards to it and draws its answer. Imports use the `#lib/...` alias. `pnpm dev` serves on port 4372.

- [[locate-app]]: the pages, the result layout, and how the photo overlays and box work.
- [[data-flow]]: which JSON each page imports and where it comes from.
- [[results-map]]: `/examples`, every bench photo on a map, one at a time.
