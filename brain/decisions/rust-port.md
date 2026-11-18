---
type: decision
status: verified
verifiedAgainst: 244c568 + uncommitted Rust port, 2026-11-18
sourceRefs:
  - sight/PERF.md#The search was rewritten in Rust
  - sight/crates/sight-core/src/lib.rs#forbid(unsafe_code)
  - sight/crates/sight-core/clippy.toml#Instant::now
---

# Decision: port the locator to Rust

**State before:** Clojure, ~5 s warm and ~12 s cold per search. Cold time was mostly JVM start and JIT warm-up (~1.3 s of it `nix develop`, ~0.4 s loading); the hot loop had no SIMD.

**Chosen:** a Cargo workspace. `sight-core` is pure: no files, no clock (clippy forbids `Instant::now`, `SystemTime::now`, `env::var` there), `forbid(unsafe_code)`, errors as `thiserror` values, no `unwrap`/`expect`/`panic` outside tests. `sight-io` holds the one `unsafe` (memory map) and all formats. `sight-cli` is the binary. Parallelism is rayon `map_init` with per-thread scratch; results are sorted stably so they do not depend on scheduling.

**Measured:** straight port 2.3–4.8 s per bench photo; after the kernel work 1.2–2.4 s; load 0.02 s against ~0.4 s. Parity: [[locator/tools/parity]].

**Where the language did not matter:** the horizon walk is bound by random reads across the 234 MB grid: rugged build 10.7 s in Rust vs 14 s in Clojure.

**Not chosen:** staying on the JVM with a long-lived server and the incubating Vector API (both listed in PERF.md as candidates). They would remove start-up and add SIMD, but keep a JVM in the dev shell and need an incubator module flag; the port was the owner's call, and it also gave a pure, testable core. Neither alternative was measured.

**Open:** the Clojure sources stay as the reference until cut-over; then `sight/src`, `sight/test`, `sight/deps.edn` and the JDK/Clojure packages in the flake go, and notes citing `.clj` files must move their sourceRefs to Rust or PERF.md.
