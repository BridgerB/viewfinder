---
type: moc
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
---

# Site

SvelteKit 3 (Svelte 5) on `@sveltejs/adapter-cloudflare`, deployed as a Cloudflare Worker. The viewfinder pages only display results computed offline by the locator; they have no server routes and no runtime computation. The JSON they import is bundled into the Worker (`timpanogos.json` dominates an ~885 KiB gzipped upload). Imports use the `#lib/...` alias. `pnpm dev` serves on Vite's default port, 5173.

- [[data-flow]]: which JSON each page imports and where it comes from.
- [[match-pages]]: `/timpanogos`, `/timpanogos/match`, `/sandy/match`, and `photo-match.ts`.
- [[results-map]]: `/results`, every bench photo on a map, one at a time.
