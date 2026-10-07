# v0.3.0 (part 27 of 40): engine tests 4: play pipeline and cross-card rules

Part 27 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-27.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the engine's play-pipeline, mana, cost, draw and Echo tests, and the cards package's cross-card rules suites from vitest to Rust tests, file for file (the table below), against the names SURFACE.md fixes. These tests, written without seeing the Rust engine, are the arbiter in Wave 3: when they and the implementation disagree, the implementation changes.

**How you work (fullsend spec-tester rules, adapted to a port; full text in `.claude/skills/fullsend/agents/spec-tester.md`).**
1. Your spec is the TypeScript test file plus SURFACE.md. Port every `it(...)` against the Rust names SURFACE.md fixes. Do not read the Rust under `crates/*/src/`: it is being written now, and your tests must not ratify what it happens to do.
2. Keep every assertion; never weaken or drop a test except where your table says so. A test you cannot express against SURFACE.md goes to `.fullsend/notes/spec-gaps-part-27.md` with its TS line, not into a guess.
3. Names by SURFACE §7.3: an `R<n>` in the title is a leading `r<n>_` token (`spec check` reads it).
4. Never run the compiler or the tests: they are expected not to compile until Wave 3.
5. Write only your table's files; no stubs; call what you wish existed; don't ask; stop when Done when holds.

**Delivering (night bot).** Build on your own branch and open your pull request to `main` as you always do. Never push to `staging` and never create it: #306's orchestrator merges `main` into `staging`. Commit only the files in your table and your two notes files. Your pull request adds new files and changes none, so `main`'s TypeScript CI stays green. Do not run `cargo` (builder rule 1): `main` has no Cargo workspace, and the Rust is expected not to compile until Wave 3 on `staging`. The plan's documents are not on `main`. Read them from the plan branch without committing them: `git fetch origin claude/relaxed-archimedes-i5fdah && git show origin/claude/relaxed-archimedes-i5fdah:docs/v0.3.0/SURFACE.md` (likewise `README.md`, `PORT-MAP.md`, `port-map.tsv`, `research/*.md` and `parts/<your brief>.md`).

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/cards/tests/cross/after_resolution.rs` | new | port of `packages/cards/test/after-resolution.test.ts` (420 lines). |
| `crates/cards/tests/cross/combat_windows.rs` | new | port of `packages/cards/test/combat-windows.test.ts` (852 lines). |
| `crates/cards/tests/cross/condition_active.rs` | new | port of `packages/cards/test/condition-active.test.ts` (959 lines). |
| `crates/cards/tests/cross/control_change_carry.rs` | new | port of `packages/cards/test/control-change-carry.test.ts` (104 lines). |
| `crates/cards/tests/cross/control_change.rs` | new | port of `packages/cards/test/control-change.test.ts` (599 lines). |
| `crates/cards/tests/cross/costs_and_mana.rs` | new | port of `packages/cards/test/costs-and-mana.test.ts` (225 lines). |
| `crates/cards/tests/cross/deaths_and_reborn.rs` | new | port of `packages/cards/test/deaths-and-reborn.test.ts` (482 lines). |
| `crates/cards/tests/cross/echo_and_exile.rs` | new | port of `packages/cards/test/echo-and-exile.test.ts` (230 lines). |
| `crates/cards/tests/cross/forced_attacks.rs` | new | port of `packages/cards/test/forced-attacks.test.ts` (141 lines). |
| `crates/cards/tests/cross/fuse_registry.rs` | new | port of `packages/cards/test/fuse-registry.test.ts` (74 lines). |
| `crates/cards/tests/cross/fused_hooks.rs` | new | port of `packages/cards/test/fused-hooks.test.ts` (637 lines). |
| `crates/cards/tests/cross/fused_nested_resume.rs` | new | port of `packages/cards/test/fused-nested-resume.test.ts` (53 lines). |
| `crates/cards/tests/cross/fused_target_checks.rs` | new | port of `packages/cards/test/fused-target-checks.test.ts` (59 lines). |
| `crates/cards/tests/cross/game_over.rs` | new | port of `packages/cards/test/game-over.test.ts` (122 lines). |
| `crates/cards/tests/cross/hand_returns.rs` | new | port of `packages/cards/test/hand-returns.test.ts` (321 lines). |
| `crates/cards/tests/cross/hidden_information.rs` | new | port of `packages/cards/test/hidden-information.test.ts` (1474 lines). |
| `crates/cards/tests/cross/lasting_effects.rs` | new | port of `packages/cards/test/lasting-effects.test.ts` (222 lines). |
| `crates/cards/tests/cross/my_pawn.rs` | new | port of `packages/cards/test/my-pawn.test.ts` (387 lines). |
| `crates/cards/tests/cross/paused_sequences.rs` | new | port of `packages/cards/test/paused-sequences.test.ts` (2175 lines). |
| `crates/cards/tests/cross/play_choices.rs` | new | port of `packages/cards/test/play-choices.test.ts` (363 lines). |
| `crates/cards/tests/cross/plays_and_casts.rs` | new | port of `packages/cards/test/plays-and-casts.test.ts` (342 lines). |
| `crates/cards/tests/cross/preview.rs` | new | port of `packages/cards/test/preview.test.ts` (1383 lines). |
| `crates/cards/tests/cross/re_entry.rs` | new | port of `packages/cards/test/re-entry.test.ts` (964 lines). |
| `crates/cards/tests/cross/resolving_face.rs` | new | port of `packages/cards/test/resolving-face.test.ts` (286 lines). |
| `crates/cards/tests/cross/self_generation.rs` | new | port of `packages/cards/test/self-generation.test.ts` (162 lines). |
| `crates/cards/tests/cross/setup_and_mulligan.rs` | new | port of `packages/cards/test/setup-and-mulligan.test.ts` (540 lines). |
| `crates/cards/tests/cross/stacks_and_reborn.rs` | new | port of `packages/cards/test/stacks-and-reborn.test.ts` (369 lines). |
| `crates/cards/tests/cross/tributes.rs` | new | port of `packages/cards/test/tributes.test.ts` (371 lines). |
| `crates/cards/tests/cross/trigger_stays.rs` | new | port of `packages/cards/test/trigger-stays.test.ts` (606 lines). |
| `crates/cards/tests/cross/turn_clock_and_legality.rs` | new | port of `packages/cards/test/turn-clock-and-legality.test.ts` (481 lines). |
| `crates/cards/tests/cross/turn_stages.rs` | new | port of `packages/cards/test/turn-stages.test.ts` (689 lines). |
| `crates/cards/tests/cross/vanilla_and_positions.rs` | new | port of `packages/cards/test/vanilla-and-positions.test.ts` (266 lines). |
| `crates/engine/tests/rules/announce.rs` | new | port of `packages/engine/test/announce.test.ts` (488 lines). |
| `crates/engine/tests/rules/cost_rules.rs` | new | port of `packages/engine/test/cost-rules.test.ts` (322 lines). |
| `crates/engine/tests/rules/counter_warning.rs` | new | port of `packages/engine/test/counterWarning.test.ts` (140 lines). |
| `crates/engine/tests/rules/draw_complete.rs` | new | port of `packages/engine/test/draw-complete.test.ts` (180 lines). |
| `crates/engine/tests/rules/draw_limit.rs` | new | port of `packages/engine/test/draw-limit.test.ts` (322 lines). |
| `crates/engine/tests/rules/draw_pause.rs` | new | port of `packages/engine/test/draw-pause.test.ts` (421 lines). |
| `crates/engine/tests/rules/draw.rs` | new | port of `packages/engine/test/draw.test.ts` (195 lines). |
| `crates/engine/tests/rules/echo.rs` | new | port of `packages/engine/test/echo.test.ts` (411 lines). |
| `crates/engine/tests/rules/graveyard_play.rs` | new | port of `packages/engine/test/graveyard-play.test.ts` (338 lines). |
| `crates/engine/tests/rules/mana_before_play.rs` | new | port of `packages/engine/test/mana-before-play.test.ts` (95 lines). |
| `crates/engine/tests/rules/mana.rs` | new | port of `packages/engine/test/mana.test.ts` (183 lines). |
| `crates/engine/tests/rules/overflow_events.rs` | new | port of `packages/engine/test/overflow-events.test.ts` (297 lines). |
| `crates/engine/tests/rules/play_pipeline_b_replay.rs` | new | port of `packages/engine/test/play-pipeline-b-replay.test.ts` (141 lines). |
| `crates/engine/tests/rules/play_step3.rs` | new | port of `packages/engine/test/play-step3.test.ts` (240 lines). |
| `crates/engine/tests/rules/play_choices_filters.rs` | new | port of `packages/engine/test/playChoices-filters.test.ts` (703 lines). |
| `crates/engine/tests/rules/play_choices.rs` | new | port of `packages/engine/test/playChoices.test.ts` (464 lines). |
| `crates/engine/tests/rules/play_counts.rs` | new | port of `packages/engine/test/playCounts.test.ts` (177 lines). |

Do **not** touch: anything under `crates/*/src/` (the implementation), other parts' test files.

### 3. Steps

1. Read SURFACE.md §4, §6, §8 and §15 (test names) fully.
2. For each row: write the destination file as a module of `crates/engine/tests/rules.rs` or `crates/cards/tests/cards.rs` (part 1 declared `pub mod <name>;` for it). Start with `use jackioh_engine::testkit::*;` (the whole engine plus the scenario harness).
3. One `#[test]` per TS `it`, one `mod` per TS `describe`, names by §7.3 (`it("R113 resumes …")` → `fn r113_resumes_…`). Translate `expect` calls with §8's table. Keep the TS file's header comment.
4. TS helpers local to a test file stay local Rust functions in that file. Seeds stay byte-identical strings.
5. A test that asserts on an error message keeps the exact text (the engine keeps TS's messages, §4.4.9).
6. Commit after every 5 files on your branch.

### 4. Tests

- Every `it` in every TS file of the table has a Rust `#[test]` with the same assertions, or a line in `.fullsend/notes/spec-gaps-part-27.md` saying why not.

### 5. Done when

- [ ] Every destination in the table exists with its tests; `TESTS-IN:` line and `## SPEC GAPS` heading in `.fullsend/notes/spec-gaps-part-27.md` (empty list is fine).
- [ ] No test is skipped, `#[ignore]`d or weakened.
- [ ] `.fullsend/notes/part-27.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A TS test that pokes engine internals with a name the Rust may not keep: write it against §4.2's name anyway; part 31 reconciles.
- A TS test that reads its own or another file's source text (`readFileSync` of a `.ts`) is dropped and listed in spec-gaps (#133's rule).

<!-- /jackioh-bot:plan -->
