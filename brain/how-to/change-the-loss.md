---
type: how-to
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - CLAUDE.md#Before changing scoring, run `bench`
  - sight/crates/sight-core/src/config.rs#Every constant the search uses, named and explained in one place.
---

# Change the loss (or any scoring constant)

1. Read the comment on the constant and the note for its term ([[locator/scoring/loss]]). It says what wrong answer the current value prevents; the change must not bring it back.
2. Keep it general. No rule may name or bound a location ([[concepts/why-a-skyline-locates]]).
3. Before: run `sight bench … sight/bench/cases.toml` and keep the run file.
4. Make the change in `config.rs` (or the term's code), and update its comment with the new reason.
5. After: run the bench again. Accept only if no case gets worse in error or truth rank; note any case that improved and why.
6. Golden: if the change is meant to alter results, re-record `sight/tests/fixtures/golden.json` and say so in the commit; if not, `sight golden` must still match.
7. Log the measurement in `sight/PERF.md` (speed) or a decision note here (accuracy), and run `node brain/check.ts affected` to find notes that cite what you changed.
