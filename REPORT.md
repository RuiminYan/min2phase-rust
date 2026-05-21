# min2phase-rust — Phase 5 Report

A Rust port of [cs0x7f/min2phase](https://github.com/cs0x7f/min2phase) (Kociemba two-phase 3x3 solver).
Targets: native CLI / daemon, browser WASM, shared `m2p-core` library. Date: 2026-05-21.

## TL;DR

The port reproduces the Java algorithm **bit-perfectly** (2000/2000 cubes solve to byte-identical
solution-length distributions) and runs **~20% faster** on solve and **~2× faster** on init.
WASM build is 146 KB and runs at 0.9 ms/solve in Chrome — 35% slower than native, fast enough
to drop into `/scramble/*` in place of the cubing.js solver worker.

## Repo layout

```
D:\cube\min2phase-rust\
├── Cargo.toml                          workspace root (LTO, panic=abort release)
├── crates\
│   ├── m2p-core\src\                   pure-Rust algorithm (3747 LOC)
│   │   ├── lib.rs                      Tables struct + N_* constants + verbose flags
│   │   ├── util.rs   (428 LOC)         Cnk, Solution, helpers (getNPerm/getComb/etc)
│   │   ├── cubie.rs  (703 LOC)         CubieCube + sym/move tables init
│   │   ├── coord.rs  (828 LOC)         CoordCube + 5 pruning tables (bit-packed 4-bit)
│   │   ├── search.rs (1043 LOC)        Solver + IDA* + searchopt + premoves
│   │   └── tools.rs  (518 LOC)         random cube, scramble parser, verify
│   ├── m2p-cli\src\                    native CLI
│   │   ├── main.rs                     solve / scramble / random / bench / daemon
│   │   └── bin\m2p_shootout.rs         apples-to-apples vs Java
│   └── m2p-wasm\src\lib.rs             wasm-bindgen wrapper (Min2Phase class)
├── benches\solve.rs                    criterion micro-bench (4 groups)
├── fixtures\
│   ├── java_100.tsv                    100-cube Java reference
│   ├── java_500.tsv                    500-cube Java reference
│   └── java_2000.tsv                   2000-cube Java reference
└── pkg\                                wasm-pack output (m2p_wasm.js + .wasm + .d.ts)
```

Java reference source kept untouched at `D:\cube\min2phase\`. Two extra Java files
written into its `test/`:
- `FixtureGen.java` — emits N-cube TSV fixtures with seeded RNG.
- `Shootout.java` — apples-to-apples solver harness (stdin facelets, stdout TSV, stderr stats).

## Correctness

**2000/2000 cubes round-trip-verify in both languages.** Verification per cube:
`from_scramble(solve(facelets)) == facelets` (with INVERSE_SOLUTION flag). Same Java assertion
the upstream test uses.

**Length distributions are byte-identical between Java and Rust on the same 2000 inputs:**

```
15:1   16:1   17:3   18:20   19:119   20:514   21:1342     (Java and Rust both)
```

Average solution length 20.5825 in both, matching the published Java baseline of ~20.6.

Three subtle Java semantics had to be rediscovered during port and are called out in code comments:

1. **`new CubieCube()` is solved, not zero.** Java's implicit `byte[] ca = {0..7}; ea = {0,2,4,..22}`
   is a load-bearing initial state for `set_flip` / `set_twist` (which only patch sub-bits).
   Rust port has `CubieCube::solved()` vs `CubieCube::empty()` and the call sites are explicit.

2. **Pruning BFS update reads post-assigned `val`.** In `initRawSymPrun`:
   ```java
   val &= val >> 1;
   PrunTable[i] += val & (val >> 2) & 0x11111111
   ```
   The `val >> 2` reads the already-updated `val`, not the original. First Rust port translated
   the original — pruning entry 0 returned 2 instead of 0 once depth+1 ≥ 8.

3. **`CornConjugate(moveCube[j], SymMultInv[0][s], c)`** — `idx` parameter is the sym-mult-inv
   table lookup, not the loop index `s`. First port used `s` directly; pruning tables happened
   to be self-consistent under the wrong (but coherent) sym mapping, but `do_move_prun_conj`
   broke the axis-pruning shortcut.

## Performance — Java vs Rust, 2000-cube shootout

Same inputs (`fixtures/java_2000.tsv`), same `maxDepth=21`, `probeMax=100000`, 200-cube warmup.

|             | Java 21    | Rust (LTO)  |  Δ          |
|-------------|-----------:|------------:|------------:|
| init (full) | 149.6 ms   | 79.9 ms     | **1.87×**   |
| avg solve   | 773 µs     | 630 µs      | **18.5%**   |
| p50         | 453 µs     | 356 µs      | **21.4%**   |
| p95         | 2461 µs    | 1992 µs     | **19.1%**   |
| p99         | 4853 µs    | 4071 µs     | **16.1%**   |
| min         | 20 µs      | 15 µs       | 25%         |
| max         | 16.9 ms    | 19.5 ms     | -15%        |
| avg length  | 20.5825    | **20.5825** | identical   |

Single outlier worse on Rust max (one 19ms vs Java's 16.9ms) is BFS variance; long-tail behavior
is otherwise tighter on Rust.

## Performance — criterion micro-bench

```
tables_build/full_init     78.7 ms — 81.1 ms      (vs Java ~150-200 ms)
tables_build/partial_init   8.9 ms —  9.1 ms      (vs Java ~60 ms,  ~6.7x)
solve_random/max_depth_21  560 µs — 578 µs        (mean 569 µs)
solve_random/max_depth_22  483 µs — 499 µs        (more freedom, slightly faster)
solve_super_flip           7.44 ms — 7.58 ms      (hardest known case)
from_scramble/len=5        336 ns — 346 ns        (~67 ns/move overhead)
from_scramble/len=8        413 ns — 425 ns
from_scramble/len=17       619 ns — 632 ns        (~37 ns/move steady-state)
```

## WASM

`wasm-pack build --release --target web` produces:

- `m2p_wasm_bg.wasm` — **146 KB**
- `m2p_wasm.js` — 16.7 KB glue
- `m2p_wasm.d.ts` — 3.6 KB types

Smoke-tested in Chrome via Playwright:

```
wasm init       : 9.0 ms
Min2Phase ctor  : 106.5 ms  (tables built)
solve superFlip : 10.8 ms   → 21 moves ✓
T-perm scramble : <1 ms     → 9 moves ✓
50 random cubes : avg 0.904 ms / p50 0.500 ms / p95 3.800 ms
                  avg length 20.580
```

Native solve 569 µs → browser WASM 904 µs: **+35% overhead**, dominated by browser's
non-SIMD bit-twiddle codegen and lack of native CPU features. Acceptable for a UI-thread
solver. Pruning tables live in WASM memory — no separate `.bin` fetch needed.

## Public API surface (`m2p-core`)

```rust
pub struct Solver { /* ... */ }
pub struct Tables { /* ... */ }                  // share across solvers via Arc<Tables>

impl Solver {
    pub fn new() -> Self;                        // full init, ~80 ms
    pub fn new_partial() -> Self;                // partial init, ~9 ms (slower solves)
    pub fn with_tables(tables: Arc<Tables>) -> Self;
    pub fn solve(&mut self, facelets: &str, max_depth: i32,
                 probe_max: u64, probe_min: u64,
                 verbose: u32) -> Result<String, SolverError>;
    pub fn next(&mut self, probe_max: u64, probe_min: u64,
                verbose: u32) -> Result<String, SolverError>;
    pub fn length(&self) -> usize;
    pub fn number_of_probes(&self) -> u64;
}

pub mod verbose { /* USE_SEPARATOR / INVERSE_SOLUTION / APPEND_LENGTH / OPTIMAL_SOLUTION */ }

pub mod tools {
    pub fn from_scramble(s: &str, tables: &Tables) -> String;
    pub fn from_scramble_moves(scramble: &[u8], tables: &Tables) -> String;
    pub fn random_cube<R: Rng>(rng: &mut R) -> String;
    pub fn random_last_layer<R: Rng>(rng: &mut R) -> String;
    pub fn super_flip() -> String;
    pub fn verify_facelets(facelets: &str, tables: &Tables) -> Result<(), VerifyError>;
}

pub enum SolverError {
    FaceletParse, EdgeMissing, EdgeFlip, CornerMissing,
    CornerTwist, Parity, NoSolutionInDepth, ProbeLimitExceeded,
}
```

## CLI

```
m2p solve <FACELETS>           one-shot solve
m2p scramble "R U R' U' ..."   scramble → facelets
m2p random [seed]              uniformly random cube facelets
m2p bench [n=100] [seed=42]    solve N cubes, full stats
m2p daemon                     line-driven solver on stdin/stdout
```

Daemon protocol mirrors the existing cube555 Java daemon pattern in `cuberoot.me` core:

```
> random
OK 21 412 us  D' B2 D' L2 D  R2 D2 B2 L2 U' B' F' R  F2 D  U  R2 U2 F' L' R'
> scramble R U R' U' R' F R F'
OK 9 178 us  R2 U  R2 U' R' F  R2 F' R'
> quit
```

## Tests

```
$ cargo test -p m2p-core --lib --release
running 31 tests
... 31 passed; 0 failed
```

Coverage:
- 13 cubie tests (move composition, sym table invariants, perm/comb round-trips, Cnk)
- 8 coord tests (pruning pack, BFS depth distribution matches `pruningValue.txt` exactly)
- 8 search tests including `solves_java_fixture_100` (round-trip-validates all 100 fixture entries)
- 4 tools tests (verify, random round-trip, scramble round-trip)

## Integration suggestions for cuberoot.me

1. **Drop in for /scramble/* SAB pages.** Current cubing.js search worker is fine; this is an
   option, not a mandate. Wins: smaller bundle (146 KB vs cubing.js search worker ~MB),
   no `prewarm` queue contention with 4x4/5x5 ([[feedback_cubing_prewarm_serial]]).

2. **Server-side daemon.** Replace any future Java solver process with the native CLI
   `m2p daemon` — eliminates JVM RAM overhead ([[project_555_rs]] memory pressure). Single
   process, line protocol, easy supervise via existing pm2.

3. **Recon assist.** /recon could call `m2p` to compute optimality / shortest solutions for
   given states without round-tripping cubing.js.

## What was NOT ported

- `Tools.initFrom` / `saveTo` — Java's table cache file format. Skipped; runtime build is fast
  enough (80 ms) that the disk cache is no longer worth the bytes. If we want sub-millisecond
  cold start later, `bincode::serialize` to a `tables.bin` and `mmap` it would be 50-100 lines.
- `Tools.randomLastSlot` / `randomZBLastLayer` / `randomCorner*` / `randomEdge*` — partial-state
  generators. Easy to add when needed; the underlying `randomState` is already there in
  `tools.rs` as a private helper, just exposed less broadly.

## License

GPL-3.0-or-later OR MIT (matches upstream dual license).

## Files modified outside this repo

- `D:\cube\min2phase\test\FixtureGen.java` — new
- `D:\cube\min2phase\test\Shootout.java` — new

(Both go via the existing Makefile's classpath; can be deleted without breaking the upstream tree.)
