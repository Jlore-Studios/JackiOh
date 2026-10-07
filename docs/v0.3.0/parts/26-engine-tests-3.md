# v0.3.0 (part 26 of 40): engine tests 3: view, turn, setup, combat

Part 26 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-26.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the engine's view, layer, turn, setup, reduce, replay, state, combat, damage and targeting tests from vitest to Rust tests, file for file (the table below), against the names SURFACE.md fixes. These tests, written without seeing the Rust engine, are the arbiter in Wave 3: when they and the implementation disagree, the implementation changes.

**How you work (fullsend spec-tester rules, adapted to a port; full text in `.claude/skills/fullsend/agents/spec-tester.md`).**
1. Your spec is the TypeScript test file plus SURFACE.md. Port every `it(...)` against the Rust names SURFACE.md fixes. Do not read the Rust under `crates/*/src/`: it is being written now, and your tests must not ratify what it happens to do.
2. Keep every assertion; never weaken or drop a test except where your table says so. A test you cannot express against SURFACE.md goes to `.fullsend/notes/spec-gaps-part-26.md` with its TS line, not into a guess.
3. Names by SURFACE §7.3: an `R<n>` in the title is a leading `r<n>_` token (`spec check` reads it).
4. Never run the compiler or the tests: they are expected not to compile until Wave 3.
5. Write only your table's files; no stubs; call what you wish existed; don't ask; stop when Done when holds.

**Delivering (night bot).** Build on your own branch and open your pull request to `main` as you always do. Never push to `staging` and never create it: #306's orchestrator merges `main` into `staging`. Commit only the files in your table and your two notes files. Your pull request adds new files and changes none, so `main`'s TypeScript CI stays green. Do not run `cargo` (builder rule 1): `main` has no Cargo workspace, and the Rust is expected not to compile until Wave 3 on `staging`. The plan's documents are not on `main`. Read them from the plan branch without committing them: `git fetch origin claude/relaxed-archimedes-i5fdah && git show origin/claude/relaxed-archimedes-i5fdah:docs/v0.3.0/SURFACE.md` (likewise `README.md`, `PORT-MAP.md`, `port-map.tsv`, `research/*.md` and `parts/<your brief>.md`).

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/engine/tests/rules/after_attack.rs` | new | port of `packages/engine/test/after-attack.test.ts` (135 lines). |
| `crates/engine/tests/rules/animated.rs` | new | port of `packages/engine/test/animated.test.ts` (473 lines). |
| `crates/engine/tests/rules/auto_end_turn.rs` | new | port of `packages/engine/test/auto-end-turn.test.ts` (161 lines). |
| `crates/engine/tests/rules/backrow_death.rs` | new | port of `packages/engine/test/backrow-death.test.ts` (73 lines). |
| `crates/engine/tests/rules/backrow_piles.rs` | new | port of `packages/engine/test/backrow-piles.test.ts` (438 lines). |
| `crates/engine/tests/rules/brittle.rs` | new | port of `packages/engine/test/brittle.test.ts` (395 lines). |
| `crates/engine/tests/rules/carried_damage.rs` | new | port of `packages/engine/test/carried-damage.test.ts` (66 lines). |
| `crates/engine/tests/rules/combat_positions.rs` | new | port of `packages/engine/test/combat-positions.test.ts` (347 lines). |
| `crates/engine/tests/rules/combat_resolution.rs` | new | port of `packages/engine/test/combat-resolution.test.ts` (303 lines). |
| `crates/engine/tests/rules/combat_validation.rs` | new | port of `packages/engine/test/combat-validation.test.ts` (428 lines). |
| `crates/engine/tests/rules/combat_property.rs` | new | port of `packages/engine/test/combat.property.test.ts` (465 lines). |
| `crates/engine/tests/rules/condition_active.rs` | new | port of `packages/engine/test/conditionActive.test.ts` (825 lines). drop l.770–825 (the R195 prose block, #133); keep the behaviour tests |
| `crates/engine/tests/rules/config.rs` | new | port of `packages/engine/test/config.test.ts` (54 lines). |
| `crates/engine/tests/rules/control_change_property.rs` | new | port of `packages/engine/test/control-change.property.test.ts` (309 lines). |
| `crates/engine/tests/rules/control_change.rs` | new | port of `packages/engine/test/control-change.test.ts` (477 lines). |
| `crates/engine/tests/rules/damage_pipeline.rs` | new | port of `packages/engine/test/damage-pipeline.test.ts` (180 lines). |
| `crates/engine/tests/rules/damage.rs` | new | port of `packages/engine/test/damage.test.ts` (476 lines). |
| `crates/engine/tests/rules/destroyed_face.rs` | new | port of `packages/engine/test/destroyed-face.test.ts` (32 lines). |
| `crates/engine/tests/rules/endgame.rs` | new | port of `packages/engine/test/endgame.test.ts` (177 lines). |
| `crates/engine/tests/rules/faces.rs` | new | port of `packages/engine/test/faces.test.ts` (145 lines). |
| `crates/engine/tests/rules/game_summary.rs` | new | port of `packages/engine/test/game-summary.test.ts` (277 lines). |
| `crates/engine/tests/rules/generation_replay.rs` | new | port of `packages/engine/test/generation-replay.test.ts` (122 lines). |
| `crates/engine/tests/rules/glow_facts.rs` | new | port of `packages/engine/test/glow-facts.test.ts` (218 lines). |
| `crates/engine/tests/rules/handicap.rs` | new | port of `packages/engine/test/handicap.test.ts` (1303 lines). |
| `crates/engine/tests/rules/hotseat_smoke.rs` | new | port of `packages/engine/test/hotseat.smoke.test.ts` (32 lines). |
| `crates/engine/tests/rules/instance_data.rs` | new | port of `packages/engine/test/instance-data.test.ts` (262 lines). |
| `crates/engine/tests/rules/kill_credit.rs` | new | port of `packages/engine/test/kill-credit.test.ts` (111 lines). |
| `crates/engine/tests/rules/layers.rs` | new | port of `packages/engine/test/layers.test.ts` (682 lines). |
| `crates/engine/tests/rules/lethal.rs` | new | port of `packages/engine/test/lethal.test.ts` (206 lines). |
| `crates/engine/tests/rules/library_copies.rs` | new | port of `packages/engine/test/library-copies.test.ts` (119 lines). |
| `crates/engine/tests/rules/mulligan_concurrent.rs` | new | port of `packages/engine/test/mulligan-concurrent.test.ts` (339 lines). |
| `crates/engine/tests/rules/own_library.rs` | new | port of `packages/engine/test/ownLibrary.test.ts` (253 lines). |
| `crates/engine/tests/rules/params.rs` | new | port of `packages/engine/test/params.test.ts` (190 lines). |
| `crates/engine/tests/rules/pools.rs` | new | port of `packages/engine/test/pools.test.ts` (172 lines). |
| `crates/engine/tests/rules/preview_ids.rs` | new | port of `packages/engine/test/preview-ids.test.ts` (97 lines). |
| `crates/engine/tests/rules/preview.rs` | new | port of `packages/engine/test/preview.test.ts` (616 lines). |
| `crates/engine/tests/rules/query.rs` | new | port of `packages/engine/test/query.test.ts` (372 lines). |
| `crates/engine/tests/rules/recruit_variants.rs` | new | port of `packages/engine/test/recruit-variants.test.ts` (232 lines). |
| `crates/engine/tests/rules/reduce.rs` | new | port of `packages/engine/test/reduce.test.ts` (185 lines). |
| `crates/engine/tests/rules/replacements.rs` | new | port of `packages/engine/test/replacements.test.ts` (685 lines). |
| `crates/engine/tests/rules/replay_scripted.rs` | new | port of `packages/engine/test/replay-scripted.test.ts` (414 lines). |
| `crates/engine/tests/rules/replay.rs` | new | port of `packages/engine/test/replay.test.ts` (33 lines). |
| `crates/engine/tests/rules/restrictions.rs` | new | port of `packages/engine/test/restrictions.test.ts` (262 lines). |
| `crates/engine/tests/rules/rng.rs` | new | port of `packages/engine/test/rng.test.ts` (98 lines). |
| `crates/engine/tests/rules/rotation.rs` | new | port of `packages/engine/test/rotation.test.ts` (366 lines). |
| `crates/engine/tests/rules/rounds.rs` | new | port of `packages/engine/test/rounds.test.ts` (143 lines). |
| `crates/engine/tests/rules/self_tribute.rs` | new | port of `packages/engine/test/self-tribute.test.ts` (132 lines). |
| `crates/engine/tests/rules/setup_aside.rs` | new | port of `packages/engine/test/setup-aside.test.ts` (524 lines). |
| `crates/engine/tests/rules/setup.rs` | new | port of `packages/engine/test/setup.test.ts` (237 lines). |
| `crates/engine/tests/rules/shuffle_random.rs` | new | port of `packages/engine/test/shuffle-random.test.ts` (98 lines). |
| `crates/engine/tests/rules/state.rs` | new | port of `packages/engine/test/state.test.ts` (77 lines). |
| `crates/engine/tests/rules/targeting.rs` | new | port of `packages/engine/test/targeting.test.ts` (465 lines). |
| `crates/engine/tests/rules/temporary.rs` | new | port of `packages/engine/test/temporary.test.ts` (107 lines). |
| `crates/engine/tests/rules/transform_variants.rs` | new | port of `packages/engine/test/transform-variants.test.ts` (194 lines). |
| `crates/engine/tests/rules/tribute_zones.rs` | new | port of `packages/engine/test/tribute-zones.test.ts` (199 lines). |
| `crates/engine/tests/rules/tribute.rs` | new | port of `packages/engine/test/tribute.test.ts` (569 lines). |
| `crates/engine/tests/rules/turn_cap.rs` | new | port of `packages/engine/test/turn-cap.test.ts` (65 lines). |
| `crates/engine/tests/rules/turn_wiring.rs` | new | port of `packages/engine/test/turn-wiring.test.ts` (359 lines). |
| `crates/engine/tests/rules/turn.rs` | new | port of `packages/engine/test/turn.test.ts` (514 lines). |
| `crates/engine/tests/rules/view_marks.rs` | new | port of `packages/engine/test/view-marks.test.ts` (173 lines). |
| `crates/engine/tests/rules/view_for.rs` | new | port of `packages/engine/test/viewFor.test.ts` (818 lines). |
| `crates/engine/tests/rules/windfury.rs` | new | port of `packages/engine/test/windfury.test.ts` (151 lines). |
| `crates/engine/tests/rules/zones.rs` | new | port of `packages/engine/test/zones.test.ts` (232 lines). |

Do **not** touch: anything under `crates/*/src/` (the implementation), other parts' test files.

### 3. Steps

1. Read SURFACE.md §4, §6, §8 and §15 (test names) fully.
2. `conditionActive.test.ts`: port everything except l.770–825 (the R195 block that reads SPEC.md's wording and the rulings index; #133). `lint-ban.test.ts` is not yours (deleted).
3. For each row: write the destination file as a module of `crates/engine/tests/rules.rs` or `crates/cards/tests/cards.rs` (part 1 declared `pub mod <name>;` for it). Start with `use jackioh_engine::testkit::*;` (the whole engine plus the scenario harness).
4. One `#[test]` per TS `it`, one `mod` per TS `describe`, names by §7.3 (`it("R113 resumes …")` → `fn r113_resumes_…`). Translate `expect` calls with §8's table. Keep the TS file's header comment.
5. TS helpers local to a test file stay local Rust functions in that file. Seeds stay byte-identical strings.
6. A test that asserts on an error message keeps the exact text (the engine keeps TS's messages, §4.4.9).
7. Commit after every 5 files on your branch.

### 4. Tests

- Every `it` in every TS file of the table has a Rust `#[test]` with the same assertions, or a line in `.fullsend/notes/spec-gaps-part-26.md` saying why not.

### 5. Done when

- [ ] Every destination in the table exists with its tests; `TESTS-IN:` line and `## SPEC GAPS` heading in `.fullsend/notes/spec-gaps-part-26.md` (empty list is fine).
- [ ] No test is skipped, `#[ignore]`d or weakened.
- [ ] `.fullsend/notes/part-26.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A TS test that pokes engine internals with a name the Rust may not keep: write it against §4.2's name anyway; part 31 reconciles.
- A TS test that reads its own or another file's source text (`readFileSync` of a `.ts`) is dropped and listed in spec-gaps (#133's rule).

<!-- /jackioh-bot:plan -->
