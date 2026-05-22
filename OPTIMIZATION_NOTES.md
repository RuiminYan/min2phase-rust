# Performance Optimization Handover

This document gets a fresh agent (Claude/Cursor/whoever) ready to make the
next big jump in solver perf. Read it end-to-end before touching any code.

## What this repo is

Rust port of cs0x7f's min2phase (3x3 Kociemba two-phase). Workspace:

- `crates/m2p-core/` — algorithm (Tables, Solver, CubieCube, CoordCube, tools)
- `crates/m2p-cli/`  — `m2p` binary + `bench` subcommand for measurement
- `crates/m2p-wasm/` — wasm-bindgen wrapper, ships as `pkg/`
- `fixtures/` — `java_{100,500,2000}.tsv` ground-truth from upstream Java impl

`pkg/` is the wasm-pack output, copied into a separate SPA's
`src/wasm/m2p/` for production use. Both files are committed — the wasm
itself is a release artifact.

## Current performance (commit `175b483` on `main`)

5000-cube benchmark, seed 42, median of 5 runs:

| Metric         | Value         |
| -------------- | ------------- |
| total          | **2503 ms**   |
| avg            | 0.500 ms      |
| p50            | 0.292 ms      |
| p95            | 1.649 ms      |
| p99            | 3.003 ms      |
| max            | ~14 ms        |
| avg_len        | 20.5600       |
| len_hist       | 16:1 17:10 18:76 19:321 20:1285 21:3307 |
| init (Tables)  | ~74 ms        |

vs cs0x7f's upstream `min2phase-rust 0.2.4`: **1.71× faster on solve**,
also shorter solutions (upstream avg_len 20.6452 — we're Java-bit-perfect,
upstream isn't).

The optimizations that got us here (in the v1 commit `175b483`):

1. Flattened `Vec<Vec<u16>>` lookup tables → flat `Vec<u16>` with
   compile-time-constant stride. ~5 table types touched, the rest stayed
   small fixed arrays.
2. Dropped `Option<Vec<...>>` on `twist_flip_prun` and `flip_s2rf`
   (always `Some` when `USE_TWIST_FLIP_PRUN: bool = true`, but inner
   loops were paying the deref/unwrap).
3. `CubieCube`: derived `Copy` (20-byte POD), `#[inline]` on
   `corn_mult` / `edge_mult`.
4. `#[inline]` on `CoordCube::{calc_pruning, set_with_prun, do_move_prun,
   do_move_prun_conj}`.

Reading these commits is a precondition for further work — same patterns,
don't undo them.

## How to measure

```pwsh
cargo run -p m2p-cli --release -- bench 5000 42
```

- **Always** run at least 3, prefer 5, take median. Single-run noise is
  ±60 ms / ±2.5%.
- **`len_hist` must stay `16:1 17:10 18:76 19:321 20:1285 21:3307`** for
  seed 42. Anything else means you broke length-optimality and the
  solution-correctness invariant. Stop and fix.
- `cargo test -p m2p-core --lib --release` must keep passing all 35
  tests, especially `solves_java_fixture_100` which round-trips against
  Java ground-truth output.

For micro-bench of specific paths:
```pwsh
cargo bench -p m2p-core
```
(see `benches/solve.rs` — has `solve_random/max_depth_21`, `solve_super_flip`,
`tables_build` groups.)

## Things already tried — don't redo

See full discussion in conversation history if available, but in short:

| Attempt | Result | Why it didn't help |
|---|---|---|
| `target-cpu=native` (`.cargo/config.toml`) | -0.3% in 5-run median | LLVM already emits decent generic x86-64 for this code; BMI2/AVX2 unlock didn't help hot path. (Was kept anyway for risk-free upside; you can re-add via cfg-guarded config.) |
| PGO with single 5000-cube profile run | Within noise | min2phase hot path is statically obvious; PGO had no new signal beyond `#[inline]`. Worth retrying with diverse profile run (super_flip, near-solved, deep cases). |
| `get_pruning` unsafe `get_unchecked` only | Single-run -2%, lost in median noise | Kept hidden bounds checks were already speculatively hoisted by LLVM in some sites. Worth keeping as cleanup but not perf. |
| All u16/u8 lookups → unsafe `get_unchecked` helpers | **-5 to -10%** (slower) | Adding `#[inline(always)] fn u16at(...)` helpers blew inline budget on `do_move_prun`, made LLVM stop inlining call sites that previously got inlined. Lesson: measure each `unsafe` step individually. |

## Where the big wins likely are

Static micro-optimization is exhausted. The remaining gains require
**profiling first** to find the actual bottleneck, then algorithmic or
memory-layout work.

### Step 1: actually profile

Install `samply` or `cargo-flamegraph`:
```pwsh
cargo install samply
samply record target/release/m2p.exe bench 5000 42
```

This will open Firefox Profiler showing where time is spent. Until you
have this view, don't optimize — past this point, intuition is wrong
about 50% of the time on tight code.

### Likely hot spots based on algorithm structure

(Hypotheses, not measurements.)

1. **L1 cache misses on pruning tables.** `ud_slice_twist_prun` is
   ~80 KB (495×324×0.5 bytes packed, 4 bits/entry). L1 D-cache is
   typically 32-48 KB. The IDA* recursion walks this table with
   semi-random access — every miss is ~10-20 cycles.

   *Possible win:* reorder the table so that during one `solve()` call,
   consecutive lookups hit the same cache line more often. Today the
   index is `twist * N_SLICE + slice_conj_lookup` — `twist` varies
   slowly across phase1 nodes, `slice_conj_lookup` varies fast. Try
   flipping: store as `slice * N_TWIST_SYM + twist`. Or 2D blocking
   (cache-tile the table).

2. **Phase1 IDA* recursion overhead.** `search.rs` `phase1()` is a
   recursive fn that allocates a fresh `CoordCube` on the stack each
   level. Up to depth 13. Could rewrite as an explicit work-stack array
   (already partially there via `node_ud`) and a loop. Less obviously a
   win — LLVM already inlines and SROA-s the recursion well.

3. **`do_move_prun` data dependencies.** Reads `ud_slice_move` →
   `flip_move` → `twist_move` → three pruning lookups, mostly serially.
   The pruning lookups could in principle be issued in parallel using
   prefetch (`_mm_prefetch`) — kick off the next pruning lookup before
   the current one's result is needed. Tricky to get right but
   potentially big (10-20%).

4. **CubieCube `corn_mult` / `edge_mult` are gather-loads.**
   `prod.ea[ed] = a.ea[b.ea[ed] >> 1] ^ (b.ea[ed] & 1)` — indirect
   indexing on 12 bytes. Hard to SIMD, but AVX-512 `vpgatherdb` exists.
   Probably not worth it; these are called only during URF setup +
   pre-moves, not in the hot IDA* loop.

5. **More diverse PGO profile.** Try
   ```
   m2p.exe bench 50000 42
   m2p.exe solve <super_flip_facelets>
   m2p.exe solve <near_solved_states ...>
   ```
   into the profile-generate run.

### Step 2: algorithmic, harder but more promising

- **Stronger phase1→phase2 cutover heuristic.** Java's MAX_DEPTH2 = 13
  is hardcoded; our code uses 12 (`MAX_DEPTH2_DEFAULT` in
  `search.rs:20`). Java's choice was tuned for JVM cost model; on Rust
  with our table layout the optimum may differ. Sweep MAX_DEPTH2 ∈
  {10..14}, watch `total` time AND `avg_len` (must stay 20.560 for seed
  42).

- **Better admissible lower bound at IDA* entry.** Today's
  `set_with_prun` computes max of `ud_slice_twist`, `ud_slice_flip`,
  `twist_flip` lookups. Could add a corner-edge separation prune or use
  a perfect distance table for small subspaces. Both well-studied in
  Kociemba literature; check Tomas Rokicki's papers + cs0x7f's Java
  source for ideas we may not have ported.

- **Phase2 ordering.** Phase2 has 10 moves. Walking them in a
  pruning-table-aware order (cheapest-prune-first) instead of
  position-in-array order can cut subtree explorations 10-30% based on
  general IDA* literature.

## Constraints — don't break these

1. **Length histogram for seed-42 5000 cubes must be exactly
   `16:1 17:10 18:76 19:321 20:1285 21:3307`.** This is Java-bit-perfect
   and reproduces upstream's correctness baseline.

2. **All 35 unit tests must pass**, especially
   `solves_java_fixture_100` (round-trips solutions against Java
   ground-truth) and `prun_depth_distribution_matches_java` (verifies
   pruning table content vs `pruningValue.txt`).

3. **Wasm build must keep working.** Verify with:
   ```pwsh
   wasm-pack build crates/m2p-wasm --release --target web --out-dir ../../pkg
   rm pkg/.gitignore   # wasm-pack always regenerates this; delete it.
   ```
   If you put anything in `.cargo/config.toml`, cfg-guard it with
   `[target.'cfg(not(target_arch = "wasm32"))']` so wasm stays portable.

4. **No external runtime deps.** No new C libraries, no CDN-loaded
   anything, no remote API calls. Pure Rust + std + the existing
   wasm-bindgen / rand / lazy_static / criterion crates.

5. **`Tables` API surface stays compatible** so the existing SPA
   integration (separate repo, `src/wasm/m2p/`) doesn't break. The
   public types (`Min2Phase` class in wasm crate, `Tables` /
   `Solver` / `tools::*` in core) keep their signatures, or version-bump
   visibly.

## Workflow expectations

1. **Branch off main.** Don't push to main directly; let the user
   review + merge.

2. **One optimization per commit.** Easier to bisect when something
   regresses. Each commit should have a 1-line bench delta in the
   message ("median 2503 → 2412 ms, -3.6%").

3. **Always re-measure baseline at start.** Machine state shifts; don't
   trust the 2503 ms number above without re-confirming on the current
   machine in the current session.

4. **Stop and report if perf goes negative.** Don't pile on more changes
   hoping one will compensate. Roll back, understand why, then retry.

5. **commit message in English** (project convention).
