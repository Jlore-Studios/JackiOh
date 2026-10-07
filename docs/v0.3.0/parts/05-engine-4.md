# v0.3.0 (part 5 of 40): engine 4: turn, setup, reduce, view, validator, wire helpers and the testkit

Part 5 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | `.fullsend/notes/part-05.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the action layer and the view (reduce, legal actions, turn, setup, game over, game summary, replay and the hash, viewFor and its helpers), the deck validator, the wire helpers the server needs (stats, codes, emotes, aim), and the testkit from TypeScript to Rust, file for file (the table below), by SURFACE.md §4's rules, so that every function keeps its name (snake_cased), its module, its behaviour and its RNG draws. These modules are called by every other engine part, the cards and the AI by the names §4.2 gives them.

**How you work (fullsend builder rules, in priority order; full text in `.claude/skills/fullsend/agents/builder.md`).**
1. Never run `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or any test runner. Nothing on `staging` compiles until Wave 3; every error you would see is someone else's unwritten file. Count any you ran as `BUILDS-RUN`.
2. Write only the files in your Files to touch table and your two notes files. Read anything in `packages/`, `apps/`, `docs/`, `SPEC.md` (the TypeScript is your spec); do not read other parts' Rust under `crates/`: it is half-written.
3. Match SURFACE.md exactly at every boundary: paths by §4.1, names by §4.2, types by §4.3, the shapes of §5–§14.
4. No stubs: no `todo!()`, `unimplemented!()`, placeholder bodies or `// TODO`. Port the real body; if you truly cannot, leave the function out and list it under GAPS.
5. Duplicate on purpose: a small helper another part probably writes, write your own private copy.
6. Call what you wish existed: another module's function by its TS name snake_cased at its TS module's Rust path.
7. Don't ask questions: decide, and record the decision in your notes.
8. Stop when Done when holds.

**Pushing.** README §3.2: `git fetch origin staging && git checkout -B work origin/staging`, write, commit only your files, `git pull --rebase origin staging && git push origin HEAD:staging` (retry with back-off). Commit often. A session that is cut off leaves its work on `staging`; the next session for this part starts at the first file in the table that is missing or empty.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/engine/src/testkit/glow.rs` | new | port of `packages/cards/test/_glow.ts` (35 lines). |
| `crates/engine/src/testkit/scenario.rs` | new | port of `packages/cards/test/_harness.test.ts` (664 lines). as `#[cfg(test)] mod tests` of the testkit; port of `packages/cards/test/_harness.ts` (1149 lines). SURFACE §8 |
| `crates/engine/src/testkit/invariants.rs` | new | port of `packages/cards/test/_invariants.ts` (258 lines). |
| `crates/cards/tests/cross/game_summary.rs` | new | port of `packages/cards/test/game-summary.test.ts` (221 lines). |
| `crates/cards/tests/cross/invariants.rs` | new | port of `packages/cards/test/invariants.test.ts` (55 lines). |
| `crates/engine/src/condition.rs` | new | port of `packages/engine/src/condition.ts` (70 lines). |
| `crates/engine/src/counter_warning.rs` | new | port of `packages/engine/src/counterWarning.ts` (50 lines). |
| `crates/engine/src/game_over.rs` | new | port of `packages/engine/src/gameOver.ts` (23 lines). |
| `crates/engine/src/game_summary.rs` | new | port of `packages/engine/src/gameSummary.ts` (186 lines). |
| `crates/engine/src/instance_view.rs` | new | port of `packages/engine/src/instanceView.ts` (62 lines). |
| `crates/engine/src/reduce.rs` | new | port of `packages/engine/src/reduce.ts` (567 lines). drops the `rng` argument and syncFusedScripts |
| `crates/engine/src/replay.rs` | new | port of `packages/engine/src/replay.ts` (83 lines). |
| `crates/engine/src/setup.rs` | new | port of `packages/engine/src/setup.ts` (605 lines). |
| `crates/engine/src/turn.rs` | new | port of `packages/engine/src/turn.ts` (842 lines). |
| `crates/engine/src/view_for.rs` | new | port of `packages/engine/src/viewFor.ts` (1177 lines). |
| `crates/engine/src/wire/aim.rs` | new | port of `packages/shared/src/aim.ts` (90 lines). TS copy kept for the web as apps/web/src/wire/aim.ts (part 21) |
| `crates/engine/src/wire/codes.rs` | new | port of `packages/shared/src/codes.ts` (358 lines). TS copy kept for the web as apps/web/src/wire/codes.ts (part 21); port of `packages/shared/test/codes.test.ts` (486 lines). as `#[cfg(test)] mod tests` |
| `crates/engine/src/wire/emotes.rs` | new | port of `packages/shared/src/emotes.ts` (139 lines). TS copy kept for the web as apps/web/src/wire/emotes.ts (part 21); port of `packages/shared/test/emotes.test.ts` (196 lines). as `#[cfg(test)] mod tests` |
| `crates/engine/src/wire/stats.rs` | new | port of `packages/shared/src/stats.ts` (442 lines). TS copy kept for the web as apps/web/src/wire/stats.ts (part 21); port of `packages/shared/test/stats.test.ts` (284 lines). as `#[cfg(test)] mod tests` |
| `crates/engine/tests/fixtures/code-input-cases.json` | new | converted from `packages/shared/test/fixtures/code-input-cases.ts` (461 lines). one fixture for the Rust and the web tests |
| `crates/engine/src/validator.rs` | new | port of `packages/validator/src/index.ts` (526 lines). the one validator; the web calls it through WASM |
| `crates/engine/tests/rules/validator_drafts.rs` | new | port of `packages/validator/test/drafts.test.ts` (284 lines). |
| `crates/engine/tests/rules/fixtures/validator_loadouts.rs` | new | port of `packages/validator/test/fixtures/loadouts.ts` (247 lines). |
| `crates/engine/tests/rules/validator.rs` | new | port of `packages/validator/test/validator.test.ts` (270 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3–§6 fully, then each TS file in your table (and, read-only, any TS file it imports, to know what it calls).
2. `reduce.rs`: `reduce(state, action)` (no rng argument; the match rng is rebuilt from `(state.seed, state.rng_cursor)` and the cursor written back), `legal_actions` in exactly `eachLegalAction`'s order (reduce.ts:492; research A §3.7), `begin_game`, `seat_to_act`, the timeout and auto-end-turn paths, the `activatePower` alias kept (SURFACE §6.1). A refused action returns the input state, no events and TS's message.
3. `replay.rs`: `canonical`, `fnv1a32_utf16`, `hash_state` exactly as SURFACE §5.2, and `fold(args) -> FoldResult { state, errors: Vec<FoldError { nonce, error }> }`.
4. `view_for.rs`: the hidden-information filter, field for field; it is the client's contract (golden `v` hashes).
5. `validator.rs`: `packages/validator/src/index.ts` whole (`validate_loadout`, `validate_trio`, `validate_deck`, `trio_conflicts`, `normalize_name`, `check_deck_draft`, `check_trio_draft`, `check_import_room`, `LOADOUT_DECKS`, `TRIO_DECKS`, the types). `DECK_SIZE`/`MAX_COPIES` come from `crate::config`.
6. `wire/{stats,codes,emotes,aim}.rs`: the runtime helpers of `packages/shared/src/` with their tests as `#[cfg(test)] mod tests`; convert `packages/shared/test/fixtures/code-input-cases.ts` to `crates/engine/tests/fixtures/code-input-cases.json` (an array of `{input, format, expected}` objects, the TS values verbatim) and read it with `include_str!` in `codes.rs`'s tests.
7. `testkit/scenario.rs`: `_harness.ts` whole, by SURFACE §8 (`scenario`, the `Scenario` methods, `expect_*`, card references by id, index or name, `DEFAULT_SEED`, `DEFAULT_TURN`), plus `register_scripts`/`register_catalog` setting the thread-local override that `catalog.rs`/`scripts.rs` consult (SURFACE §8; the one allowed `thread_local!` + `RefCell`). Drop the harness's dead fallbacks for "combat arrives with M2"/"prompts arrive with M3" (`_harness.ts:193-200, :901, :933`). `testkit/invariants.rs` is `_invariants.ts` (I1–I4); `testkit/glow.rs` is `_glow.ts`.
8. For each row: write the Rust file. Port every function, exported or not, in TS order; keep the TS header comment as `//!` lines and every inline comment that states a rule or cites a ruling (`R113`, `§10.3`) — `spec check` reads the R-ids (§15).
9. Constants that TS declared at the top of your files now live in `crate::config` (part 1 moved them, SURFACE §6.4); use them from there.
10. A function another module provides is called by its TS name snake_cased at its TS module's path (`crate::zones::place_on_field`), taking `&GameState`, `&mut GameState` or `&mut EngineSink` by §6.5. Don't look at whether it exists yet.
11. Push after every 3–5 files (README §3.2).

### 4. Tests

- The tests in your table are yours: the validator's (drafts, validator, the loadouts fixture), the wire helpers' (codes against the shared JSON fixture, emotes, stats), the harness's own tests (`_harness.test.ts` as `scenario.rs`'s `mod tests`), `invariants` and `game-summary`. Port each `it` with its assertions (SURFACE §8's table).
- The rest of the engine's tests are parts 24–27's. Part 32 compiles and runs everything in Wave 3.

### 5. Done when

- [ ] Every destination in the table holds the full port of its TS file (no function missing that TS exported).
- [ ] No `todo!`, `unimplemented!`, placeholder bodies or `// TODO` in your files.
- [ ] `.fullsend/notes/part-05.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- `legal_actions`' order matters for the AI and the fuzz policy (they pick by index): copy `eachLegalAction`'s loop structure exactly.
- SURFACE §4.4's semantics (stable sorts, insertion-ordered maps, presence of optional fields, RNG order) are where a port goes quietly wrong; the golden traces catch it in Wave 3, at the cost of a debugging session.

<!-- /jackioh-bot:plan -->
