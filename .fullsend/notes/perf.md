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
