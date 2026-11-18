---
type: governing
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
---

# Operating manual

## Reading

1. Open [[_index]], pick the pillar, open its map, open only the notes you need. Read, do not dump: a note is short so it can be read whole.
2. Trust a `verified` note as of its `verifiedAgainst` pin. If the code has moved since, re-check the sourceRefs before relying on a number.
3. A `draft` note is a lead, not a fact.

## Writing

1. One idea per note. If a note needs two headings that could each stand alone, split it.
2. Every claim about code carries a sourceRef to the text that proves it (see [[conventions]]). A claim about a measurement cites the log or run file that holds the number.
3. A decision note records the options rejected, what each did when measured, and what would change the answer.
4. A gotcha starts with the symptom a person would see, then the cause, then the fix.
5. Link the new note from its map, and link to it from at least one note that would lead a reader there.

## Keeping it true

Run the checker from the repository root after any edit to the brain or to cited code:

```sh
node brain/check.ts            # 0 broken, 0 ambiguous, 0 orphans, 0 unresolved sourceRefs, 0 metadata
node brain/check.ts affected   # which notes cite a file changed in the working tree
```

When `affected` lists a note, re-read it against the diff. If the claim still holds, update `verifiedAgainst`. If it no longer holds, fix the note or set it to `draft` with a line saying what changed. Never leave a `verified` note that is known to be wrong.

After a commit, re-pin `verifiedAgainst` on the notes you re-checked to the new short hash.
