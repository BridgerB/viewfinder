---
type: governing
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
---

# Conventions

## Layout

- `locator/<area>/` notes about the Rust search, one folder per area, each with `_moc.md`.
- `site/` notes about the SvelteKit pages.
- `concepts/`, `decisions/`, `gotchas/`, `how-to/`, `results/` cross-cutting notes, each folder with `_moc.md`.
- File names are lower-case kebab-case and unique across the brain, so a bare `[[name]]` link resolves to one note.

## Frontmatter

```yaml
---
type: area | concept | decision | gotcha | how-to | result | moc | index | governing
status: verified | draft
verifiedAgainst: <commit or working-tree description>, <date>
sourceRefs:
  - sight/crates/sight-core/src/score.rs#pub fn measure(
---
```

## sourceRefs

- `path#anchor`, path from the repository root.
- The anchor is text the file actually says: a declaration (`pub fn dot32(`), a constant line (`near_weight: 0.0,`), or a phrase from a doc comment or log. Never a line number: line numbers move with every edit, text moves only when its meaning does.
- Matching ignores markdown and code punctuation `* _ ` #` and runs of whitespace, so an anchor survives reformatting.
- The anchor must match exactly one place in the file. If it matches two, lengthen it.

## Links

- `[[name]]` or `[[folder/name|label]]`. Links inside code spans or fences are not links.
- Numbers in prose carry their unit (°, m, km, s, px).
