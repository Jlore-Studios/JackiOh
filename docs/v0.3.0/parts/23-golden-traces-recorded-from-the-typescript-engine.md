# v0.3.0 (part 23 of 40): golden traces recorded from the TypeScript engine

Part 23 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-23.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Record the golden traces from the TypeScript engine while it still runs (SURFACE §13), and write the Rust side that replays them: `crates/engine/tests/golden.rs` and `cargo jackioh golden check|bless`. These traces are the run's spec-tester output: written from the TypeScript, blind to the Rust, and the final word in Wave 3.

**How you work.** This part runs the TypeScript engine (it records the oracle) but never builds the Rust. Write only your table's files; `packages/` stays read-only.

**Delivering (night bot).** Build on your own branch and open your pull request to `main` as you always do. Never push to `staging` and never create it: #306's orchestrator merges `main` into `staging`. Commit only the files in your table and your two notes files. Your pull request adds new files and changes none, so `main`'s TypeScript CI stays green. Do not run `cargo` (builder rule 1): `main` has no Cargo workspace, and the Rust is expected not to compile until Wave 3 on `staging`. The plan's documents are not on `main`. Read them from the plan branch without committing them: `git fetch origin claude/relaxed-archimedes-i5fdah && git show origin/claude/relaxed-archimedes-i5fdah:docs/v0.3.0/SURFACE.md` (likewise `README.md`, `PORT-MAP.md`, `port-map.tsv`, `research/*.md` and `parts/<your brief>.md`).

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `scripts/golden/record.ts` | new | SURFACE §13.1–§13.3; `--seed k --dump-step n` writes TS's canonical state, views and events for one step |
| `crates/engine/tests/golden/games.jsonl` | new | 240 games (200 + 40 handicapped) |
| `crates/tools/src/golden.rs` | new | `check` (same logic as golden.rs) and `bless` (rewrites games.jsonl from Rust after an intended rules change) |
| `crates/engine/tests/golden.rs` | new | replays games.jsonl and the hotseat fixture ("a798906b"); port of `packages/cards/test/hotseat-replay.test.ts` (213 lines). fold of 01-hotseat-full-game.json must hash "a798906b" |
| `crates/engine/tests/golden/01-hotseat-full-game.json` | copy | byte for byte from `packages/cards/test/fixtures/01-hotseat-full-game.json`. |


### 3. Steps

1. Read SURFACE.md §5.2 and §13 fully, then `packages/cards/test/fuzz.test.ts`, `fuzz-handicap.test.ts`, `hotseat-replay.test.ts` and `packages/engine/src/replay.ts`.
2. This part runs TypeScript (it is the oracle): `pnpm install --frozen-lockfile`, then write `scripts/golden/record.ts` importing from `packages/engine/src` and `packages/cards/src` (`CATALOG`, `registerAll`), with a verbatim copy of `packages/cards/test/fuzz.test.ts`'s `decksForSeed` and the constants it reads (`POOL_EXCLUSIONS`, `EXCLUDED_IDS`, …). Do not import the test file: its top-level `describe` needs the vitest runner.
3. Run `pnpm exec tsx scripts/golden/record.ts > crates/engine/tests/golden/games.jsonl`. If the file is over 15 MB, record seeds 1–150 and 201–230 instead and say so in your notes.
4. Run the recorder twice and `cmp` the two outputs (determinism).
5. Write `crates/engine/tests/golden.rs` and `crates/tools/src/golden.rs` against SURFACE §6.1 and §13.3 (no Rust exists yet: do not build).
6. Commit on your branch and open your pull request.

### 4. Tests

- `games.jsonl` is byte-identical across two runs of the recorder.
- golden.rs replays every line and the hotseat fixture (runs in Wave 3, V4).

### 5. Done when

- [ ] `games.jsonl` committed with 240 (or the noted fallback) games; `record.ts` runnable with `--dump-step`.
- [ ] `.fullsend/notes/part-23.md` and `.assumptions` committed (`BUILDS-RUN` counts only Rust builds: 0).

### 6. Risks

- A non-integer number in a view would make the canonical form depend on float printing; the recorder aborts on one with its JSON path, and part 32 decides.

<!-- /jackioh-bot:plan -->
