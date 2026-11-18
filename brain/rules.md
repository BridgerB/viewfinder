---
type: governing
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
---

# Rules

The checker (`node brain/check.ts`) holds these at zero. A brain with any of them non-zero is broken, not "mostly fine".

1. **Broken links**: a `[[link]]` whose target is no note.
2. **Ambiguous links**: a bare `[[name]]` that matches two notes in different folders.
3. **Orphans**: a note nothing links to. Exempt: `_index`, every `_moc`, and the governing documents ([[operating-manual]], [[conventions]], [[glossary]], [[questions]], this one).
4. **Unresolved sourceRefs**: a missing file, a bare path without an anchor, or an anchor that matches zero or several places.
5. **Metadata**: a `verified` note without `verifiedAgainst`, or (outside maps and the index) without sourceRefs.

Project rules the notes rely on, which the brain does not enforce but must not contradict:

- The locator is location-agnostic: nothing in `sight` names a town or summit. A fix belongs in the general loss.
- Scoring changes run the bench and the golden check first ([[how-to/change-the-loss]]).
- Output formats (surface, match, ridges JSON) are the contract with the site; changing one means changing [[site/data-flow]] too.
