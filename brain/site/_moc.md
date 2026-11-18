---
type: moc
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
---

# Site

SvelteKit (Svelte 5, adapter-node). It only displays results computed offline by the locator; there are no server routes and no runtime computation. `pnpm dev` serves on port 4372.

- [[data-flow]]: which JSON each page imports and where it comes from.
- [[match-pages]]: `/timpanogos`, `/timpanogos/match`, `/sandy/match`, and `photo-match.ts`.
- [[results-map]]: `/results`, every bench photo on a map, one at a time.
