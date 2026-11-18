---
type: moc
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
---

# Gotchas

Symptom first.

- [[jpeg-decoder-differences]]: "Rust and Clojure give slightly different losses on the same photo."
- [[java-round-parity]]: "a few horizon angles differ by 1 centidegree from the reference."
- [[narrow-basin-near-mountains]]: "a canyon or foothill photo lands tens of km away."
- [[tile-edge]]: "the error can't go below a few hundred meters for one photo."
- [[screen-culls-the-truth]]: "the full search would find it (`around` says so) but `search` doesn't."
