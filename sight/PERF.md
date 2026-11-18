# Search performance log

How the locator spends its time, how that was measured, and what each change did. Commands run from the repository root with the viewfinder flake (`nix develop -c …`).

## Tools

- `bench GRID FIELDS sight/bench/cases.edn [--quick]` runs every known-location photo (quick: Timpanogos, Sandy, Little Cottonwood). For each it records error km, the rank of the first place within 1 km of the truth, lens, wall seconds, per-stage seconds and counters. It writes `data/bench/runs/<time>.edn` and prints a table against the previous run.
- `profile GRID FIELDS IMAGE SKYLINE|- N REPS none|cpu|alloc` scores a fixed seeded sample of N places, REPS times. That makes a ~10 s workload. With `-M:profile` (run from `sight/`, paths `../…`) and `cpu`/`alloc` it writes a clj-async-profiler flame graph and collapsed stacks under `/tmp/clj-async-profiler/results/`.
- Counters: run with `-J-Dviewfinder.count=true`. They are compiled out otherwise (`viewfinder.trace/counting?` is `^:const`).
- Tests: `clojure -M:test` from `sight/`, or with `-Sdeps` from the root. `golden_test` replays 60 fixed places of the 2 km field against `sight/test/viewfinder/golden.edn`, so an exact speedup must reproduce it. Delete that file to re-record on purpose.

## Baseline (commit efca147)

- One search: ~9 min wall, ~4,800 CPU-s, ~880% CPU on 10 cores (8 performance + 2 efficiency).
- Building horizons is cheap: 29,669 places in 37 s.
- Workload of 501 places against Sandy: 12.0 s per rep.
- Counters per place: exactly 17,280 `fit` (16 lenses × 3 rolls × 360 yaws), 1.0 `measure` per fit, second tilt on 0.09%. A search scores ~32k places, so ~550 M measures. That is the 9 minutes, at ~12.5 µs CPU each.

CPU flame graph of that workload (self time):

| Frame | Share |
|---|---|
| `DualPivotQuicksort` (two full sorts per measure, for a median and a trim cutoff) | ~49% |
| `RT.longCast`, `Short.longValue`, `RT.aset`, `RT.doubleCast`, `Var.getRawRoot`, `Numbers.equiv` (boxing) | ~27% |
| `angle-at` (inclusive) | ~25% |
| `tilt-guess` (inclusive) | ~18% |

The boxing came from `measure$fn__406`: a `loop` bound inside a `let` is compiled as a boxed inner function. `horizon/missing` was also read as a var on every interpolation.

## Experiments

1. **Exact fixes** (golden test passes unchanged):
   - `horizon/missing` is now `^:const`.
   - The column loop in `measure` runs as a statement and returns its count through the slot.
   - Both sorts are replaced by quickselect (`select`, `split-at-pivot`), which returns exactly the value a sort would put at that index. Test `select-agrees-with-sort` checks this on 500 random arrays with ties.
2. **Two-pass ranking** (`score/rank`, off with `-Dviewfinder.rank=full`). The screen covers every place with a quarter of the columns, a third of the lenses, no roll, and every 2nd yaw. The best `max(200, 3%)` places then get the full search, within ±8° of the screen's yaw. This is approximate, so it is judged by the bench's error and true-area rank.

3. **Tilted screen templates.** The first screen used a level camera and pretended tilt only shifts the skyline. That holds for a narrow lens, but a 100° lens tilted 14° up bends its edges, so the Sandy photo lost its true place (70 km off). The screen now tries 5 tilts (−3° to 21°, 6° apart), and Sandy is back to 0.22 km.
4. **Lenses 10° apart in the screen, not 15°.** A 52° photo screened with 40°/55° templates and lost its place (library 68 km off).
5. **Roll in the screen.** A 2° roll tilts the skyline linearly. The screen fits a capped straight line (`screen-tilt`, about 3° of roll) to the residual and applies it to the error, the correlation and the relief comparison. Without it, the library photo's true spot scored 0.83 in the screen against 0.22 in the full search. With it, 0.29, and the library photo now resolves to 0.35 km (baseline 1.62).
6. **Running sums in the screen.** The horizon is laid out twice round with prefix sums of the value, its square, its index times the value and missing degrees. Each window's sums are then two reads, and only the cross term (`dot`) needs a pass.
7. **Yaw step that scales with the lens.** The screen scores every `fov/25` degrees (at least 1), then the degrees between around the best. A fixed 3° step lost the 27° telephoto Daybreak photo (39 km off); a 1° step at 27° keeps it.
8. **No boxing in the screen.** Per-yaw constants go through one `double[]` (`slide-yaw`), and `dot` keeps 4 running sums so the adds overlap.
9. **Horizon walk compares slopes.** `walk-angle` compared angles, with two `atan2` per step and a boxed `[angle dist]` vector per ray. It now compares rise over run (the same order, since atan increases) and writes into a `double[]`. Rebuilding the 29,669-place rugged field went from 37 s to 14 s (CPU 263 s to 72 s), and all 10.7 M angles and distances are identical.

## Results

Full bench, 8 photos, warm JVM, seconds per photo (km off in brackets):

| Photo | Baseline (efca147) | Two-pass, first screen | Current |
|---|---|---|---|
| Timpanogos | ~540 (0.15) | 8.1 (0.22) | 6.7 (0.15) |
| Sandy | ~540 (0.22) | 5.7 (**70.7**) | 5.2 (0.22) |
| Little Cottonwood | ~540 (0.12) | 5.1 (**14.8**) | 4.8 (0.12) |
| Library roof | ~540 (1.62) | — | 4.5 (**0.35**) |
| Capitol, hazy | ~540 (0.43) | — | 4.4 (0.43) |
| Capitol, cloudy | ~540 (2.56) | — | 4.8 (2.56) |
| Red Butte | ~540 (0.70) | — | 4.6 (0.70) |
| Daybreak (lens given) | ~540 (1.53) | — | 2.7 (1.54) |

Every case is at or better than baseline error. A search is ~110× faster.

A cold `search` from the command line is 11.9 s. About 1.3 s of that is `nix develop` (not needed under direnv) and ~0.4 s is loading the DEM and fields; the rest is JIT warm-up. Stages warm: read and trace ~0.4 s, coarse ~4 s, 250 m pass ~0.9 s, 100 m pass ~0.5 s.

The latest CPU profile of the coarse stage: `slide-yaw` (with `dot` inlined) ~61%, `log` in the relief term 5%, the full search on the survivors ~7%, the DEM reads for the foreground drop ~2%. Threads wait ~1%, so the waves keep all 10 cores busy.

## Next, measured candidates

- Skip the relief `log` when the rest of the loss already loses to the best (~5%).
- SIMD for `dot` with the JDK Vector API (`jdk.incubator.vector`, float lanes). It needs `--add-modules` at JVM start.
- A long-lived process (REPL or small server) holding the DEM, fields and a warm JIT: a query is then the ~5 s warm path, not ~12 s cold.
- Prune tilt templates per place: try the middle tilt first, then neighbours of the best.
- Baseline bench of the old code (`git stash` or the efca147 tree) for an exact before table; ~9 min per photo, so run it unattended.
