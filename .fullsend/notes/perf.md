# Performance notes (v0.3.0)

Wall times of the whole-game checks, Rust against the TypeScript they replace, on the machine each was measured on. Append a section per measurement; say the machine and its load, since the Wave 3 sessions share it.

## fuzz (part 34)

Measured 2026-10-07 by part 34 (second half) on the Wave 3 cloud machine: 4 cores, shared with other sessions (load average in brackets, from `uptime` before and after). Rust: `cargo build --release -p jackioh-tools` at staging `c58d9de` (engine with ab40b14's borrowed script lookups), `target/release/jackioh`. TS: `pnpm fuzz` in the same checkout (`CYPRESS_INSTALL_BINARY=0 pnpm install --frozen-lockfile`, Node 22.22, vitest 5.0.1).

| Run | Seeds | Wall | CPU (user) | Seeds/s (wall) | Load | Result |
|---|---|---|---|---|---|---|
| Rust `fuzz --seeds 1000` | 1–1000 | 116 s | 338 s | 8.6 | 1.6 → 5.6 | 1000 passed; 0 panics, 0 stalls, 0 replay mismatches, 0 invariant violations |
| Rust `fuzz --seeds 1000` (first run) | 1–1000 | 253 s | — | 4.0 | 9.6 → 10 | same |
| Rust `fuzz --seeds 200 --handicap` | 1–200 | 84 s | 117 s | 2.4 | 5.3 → 10.1 | 0 failed; hero-death 198, turn-cap 1, both-heroes-dead 1 |
| Rust `fuzz --seeds 200 --handicap` (first run) | 1–200 | 91 s | — | 2.2 | 10.1 → 10.4 | same |
| TS `pnpm fuzz`, `fuzz.test.ts` | 1–1000 | 560 s | ≈ 560 s (one worker) | 1.8 | 6.3 → 2.0 | 1000 passed, same endings and action counts as Rust |
| TS `pnpm fuzz`, both files in parallel | 1–1000 each | 648 s | — | — | 6.3 → 2.0 | handicap: 0 failed; hero-death 994, turn-cap 4, both-heroes-dead 2 |

Both engines report the same plain wave to the line: hero-death 997, both-heroes-dead 3, 96 actions a game on average, 280 at most, the longest game ending on turn 53 of 60, 268 deck-legal cards, I6 every 5 states.

Per core, Rust's plain wave spends 0.34 s of CPU a seed against TS's 0.56 s (TS's worker is single-threaded, so its wall time is its CPU time); rayon spreads Rust's over the cores, so its wall time is about 4.8× shorter on this machine at low load. The handicapped wave's wall time is held up by the machine's load (117 s of CPU in 84 s of wall: about 1.4 of 4 cores were free), not by the wave.

## gate and promote (part 34)

Same machine, release build, load average 9–23 throughout (other sessions' builds), so the wall times are upper bounds.

| Run | Wall | Counts |
|---|---|---|
| `gate` (smoke, 20 games each) | 365 s | ai-vs-random 20/20 (17 needed), ai-vs-greedy 17/20 (10), hard-vs-easy 20/20 (16); perf: slowest 137 ms (scaled) of 1,500 |
| `gate --full` | 1,601 s (random 508 s, greedy 305 s, hard-vs-easy 695 s, perf 93 s) | ai-vs-random 94/100 (91 needed), ai-vs-greedy 35/50 (28), hard-vs-easy 47/50 (40), no turn-cap draws; perf: 155 decisions, slowest 478 ms scaled (1,138 ms raw), wide boards 612 / 364 ms scaled |
| `promote --lane improve --parent-bin target/release/jackioh --dry-run` | 2,166 s | vs random 97/100, vs parent (itself, gen 0) 53/100 with 1 draw and no game without a result |

The full gate's counts are generation 0's TS counts to the game (`crates/ai/generation.json`: `packages/ai` at 91cc43c, 94/100, 35/50, 47/50).

## Engine and WASM bindings (perf agent, after part 32)

Measured 2026-10-07 on the same Wave 3 machine (4 cores, shared; load average 2–15 while these ran, so the comparisons below are CPU time where it says so: `CLOCK_THREAD_CPUTIME_ID` natively, `process.cpuUsage()` in Node, `user` from `time` for whole runs; A and B runs alternate, back to back). **Staging** is `c58d9de` (part 32's borrowed registry entries already in); **branch** is `61832d4` (engine) + `ef3308b` (bindings, wrapper, module opt-level). Every number here comes from the scripts in `.fullsend/notes/perf-bench/` (how to run them is at the top of each).

### Workloads

- **Golden replay**: the first 60 lines of `crates/engine/tests/golden/games.jsonl` (5,579 steps); per step `legal_actions` for the actor, `reduce`, `hash_state`, `view_for` for both seats, which is what `golden.rs` asks minus its own hashing of the answers. TS: `packages/engine` under tsx (`bench-ts.mts`); native: a release crate on the engine with thin LTO (`perf-bench/rust`); WASM: `apps/web/src/wasm/index.ts` over the release module under Node (`bench-wasm.mts`, which also checks every step's `hashState` against the trace's `s`: 5,579/5,579).
- **Micro**: one state, line 2 (`jackioh-fuzz-3`) after 100 steps: 81.4 KB of JSON, next action `activate` by p2.
- **Fuzz**: seeds 1–200 of the plain wave, `fuzz.test.ts` against `cargo jackioh fuzz`.

### Fuzz, seeds 1–200 (back to back, load 4–8)

| Run | Wall | CPU (user) |
|---|---|---|
| TS `fuzz.test.ts` (one vitest worker) | 114.6 s | 123.9 s |
| Rust staging, all cores | 53.7 s | 73.6 s |
| Rust branch, all cores | **16.8 s** | 32.5 s |
| Rust staging, `RAYON_NUM_THREADS=1` | 70.2 s | 68.1 s |
| Rust branch, `RAYON_NUM_THREADS=1` | **36.2 s** | 35.3 s |

Same endings, same 94 actions a game. On one thread the branch spends 0.18 s of CPU a seed against staging's 0.34 s and TS's ≈ 0.55 s: 3× TS on one core, 6.8× in wall time on the shared machine.

### Golden replay, 60 games

| | legal | reduce | hash | view ×2 | total CPU / step |
|---|---|---|---|---|---|
| TS in-process (wall per call) | 0.59 | 3.02 | 2.11 | 1.93 | **6.6 ms** (37.1 s) |
| native staging (CPU per call) | 0.30–0.34 | 1.02–1.11 | 0.72–0.76 | 0.75–0.82 | 2.8–3.0 ms |
| native branch (CPU per call) | 0.12 | 0.46 | 0.30 | 0.34 | **1.2 ms** (6.8 s) |
| WASM staging, through the wrapper (wall per call) | 1.62 | 3.48 | 2.01 | 3.71 | 10.1 ms (56.4 s) |
| WASM branch, through the wrapper (wall per call) | 0.52 | 1.73 | 0.73 | 1.50 | **4.7 ms** (26.2 s) |

So the WASM engine, called the way the web calls it (JSON both ways), now replays the traces 1.4× faster than the TS engine in-process; it was 1.5× slower.

### One state (micro), ms per call

| Call | TS | native staging → branch (CPU) | WASM staging (CPU) | WASM branch (wall, back to back with TS) |
|---|---|---|---|---|
| `view_for` | 0.66–0.91 | 0.23 → 0.09 | 1.88–2.08 | **0.61–0.77** |
| `legal_actions` | 0.28 | 0.16 → 0.06 | 1.88 | 0.50 |
| `reduce` | 1.52–1.70 | 0.50 → 0.25 | 4.12 | **1.59** |
| `hash_state` | 1.65–1.68 | 0.95 → 0.28 | 2.93 | 0.74 |

Natively the branch is 5–7× TS per call. Through the wrapper `view_for` and `reduce` are at parity with TS's in-process calls and `hash_state` is 2× faster; `legal_actions` is the one still slower, by the 0.23 ms the wrapper spends writing the state (below).

### Where the time was (perf on the native release build, then `node --cpu-prof` on the module)

Natively, after part 32, a golden step still spent its time in: copying a `Script` out of the registry for every instance lookup (`instance_script`, 27% inclusive, half of a `view_for`); building a `serde_json::Value` tree to hash the state (`hash_state`, 24%: BTreeMap inserts, mallocs); composing a fused card's both faces on every lookup (18% once Fuses appear); writing every event of a view's window out as a `Map` and reading it back (`redact_event`, 12–19% of a view); building and cloning a `TriggerHolder` (card, triggers, script) for every card in every zone on every dispatch and for every counter warning (10–19%); SipHash on card ids in the two registries (8–13%). Each is gone (commit `61832d4` lists the changes); nothing is cached across calls in the engine, which keeps no state of its own (SURFACE §3).

Through the bindings, part 36's 4.2 ms `view_for` was, at staging: the parse of the 81 KB state (1.2 ms of a `seat_to_act`), `view_for` itself (0.35 ms), the wrapper's `JSON.stringify` (0.37 ms) and wasm-bindgen copying the string in. The parse is gone on every call but the first for a state (the bindings keep the last four states by their exact text; the text a `reduce` returns is the text `JSON.stringify` writes back, 525/525 golden steps checked), and so is the copy (0.18 ms of wasm-bindgen's per-character loop for an all-ASCII string; the wrapper now hands it `[" ",<state>]`, which goes through `TextEncoder.encodeInto` in 0.005 ms). `pass.mts` on the branch: `seat_to_act` 0.22 → 0.05 ms, `view_for` raw 0.65 → 0.32 ms, `viewFor` through the wrapper 0.61 ms, of which `JSON.stringify(state)` is 0.23 ms.

A parse still happens once per state the bindings have not seen (the first call after `createGame`, or a state the client changed): `serde_json::from_str::<GameState>` takes 0.41 ms natively for the 81 KB state, already straight into `GameState` with no `Value` in between; most of it is serde buffering the internally tagged enums (`GameEvent` in `applied`, `Zone`) into `Content` before it can pick the variant, which only hand-written `Deserialize` impls would avoid.

The engine-heavy web tests (practice core, core-glitch, core-fallback, hotseat, animations.window, resume, tutorial lessons; the three sweeps part 36 saw time out) take 21.6 s on the branch against 48.9 s at staging, 109/109 both.

### What is left, and a handle-based API

Every state-taking call still pays `JSON.stringify(state)` in JS (0.23 ms for 81 KB, about a third of a `viewFor`'s profile), and `reduce` also pays for the whole new state coming back as text: serde writes 82 KB (≈ 0.25 ms in WASM) and `JSON.parse` reads it (0.46–0.56 ms). The engine's own share of a `viewFor`, writing the view included, is about 0.27 ms in WASM (0.1 ms natively; dlmalloc and no SIMD account for most of the difference). The wrapper cannot skip the `stringify`: callers hand it plain objects they may change (`practice/core-glitch.test.ts` sets `state.result` on a returned state), so a text remembered per object could be stale.

A handle-based API (states kept in WASM memory, JS holding an id; `reduce` answering events and a new id; `view_for` and `legal_actions` taking the id) would remove about 0.3 ms from every call and another 0.7 ms from every `reduce`: roughly `viewFor` 0.3 ms, `legalActions` 0.2, `reduce` 0.9, against TS's 0.66, 0.28 and 1.6. The practice worker and the hotseat make 5–6 such calls a step, so that is ≈ 2 ms a step. It needs a SURFACE §10.1 change (the client keeps `GameState` JSON for saves, folds and its tests today, so a handle would need an explicit `toJSON` for those) and is not made here; it is worth a part of its own if practice-mode step time matters.

### Profiles

- **WASM module**: `[profile.release.package.jackioh-wasm] opt-level = "s"` removed, so the module builds at 3 like the server. The override reached only the bindings crate, where serde_json's writer and the state cache are instantiated: at "s" serde's string escaping ran a byte-at-a-time extend loop (4% of a `viewFor`'s samples). The same code at both settings: "s" 12,635,652 bytes (2,381,249 gzipped), 3 12,380,611 bytes (2,308,769 gzipped), and the golden replay through the wrapper 30.6 s of CPU at "s" against 27.5 s at 3. (Staging's module: 12,363,913 bytes, 2,303,969 gzipped.)
- **Tests**: part 32's opt-level 2 for the engine, the cards, serde_json and indexmap stays; no `[profile.dev.package."*"]` was added, because the other dependencies on the hot path (serde, hashbrown) are generic and compile into the calling crate at its level. `jackioh-ai` at opt-level 2 was measured and not adopted: its 276 tests run in 28.1 s instead of 38.8 s (load ≈ 2), but its crate and test binary take 49.5 s to compile instead of 12.1 s, a net loss for CI's cold `rust (test ai)` group. The 54 minutes the AI suite was briefed at predate part 32's opt-level 2 and these changes; it is now ≈ 51 s, compile and run.
- **Engine suite** (`cargo test -p jackioh-engine --features testkit`, test profile, on `ef3308b`, load ≈ 3): 147 unit, 2 export, 19 golden in 16.0 s, 2,201 rules in 16.9 s; 37 s to run against a 224 s build (784 s of CPU). Part 32 measured the golden traces at 31 s after its own fix, and this machine took 64 s for them at load 15 before the hasher and holder changes. The build is what costs now.
