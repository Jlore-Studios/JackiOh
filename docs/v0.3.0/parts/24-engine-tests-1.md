# v0.3.0 (part 24 of 40): engine tests 1: effect verbs and fixtures

Part 24 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-24.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the engine's effect-verb tests (`effects-*.test.ts`) and the engine's test fixtures (test-only catalogs and scripts) from vitest to Rust tests, file for file (the table below), against the names SURFACE.md fixes. These tests, written without seeing the Rust engine, are the arbiter in Wave 3: when they and the implementation disagree, the implementation changes.

**How you work (fullsend spec-tester rules, adapted to a port; full text in `.claude/skills/fullsend/agents/spec-tester.md`).**
1. Your spec is the TypeScript test file plus SURFACE.md. Port every `it(...)` against the Rust names SURFACE.md fixes. Do not read the Rust under `crates/*/src/`: it is being written now, and your tests must not ratify what it happens to do.
2. Keep every assertion; never weaken or drop a test except where your table says so. A test you cannot express against SURFACE.md goes to `.fullsend/notes/spec-gaps-part-24.md` with its TS line, not into a guess.
3. Names by SURFACE §7.3: an `R<n>` in the title is a leading `r<n>_` token (`spec check` reads it).
4. Never run the compiler or the tests: they are expected not to compile until Wave 3.
5. Write only your table's files; no stubs; call what you wish existed; don't ask; stop when Done when holds.

**Delivering (night bot).** Build on your own branch and open your pull request to `main` as you always do. Never push to `staging` and never create it: #306's orchestrator merges `main` into `staging`. Commit only the files in your table and your two notes files. Your pull request adds new files and changes none, so `main`'s TypeScript CI stays green. Do not run `cargo` (builder rule 1): `main` has no Cargo workspace, and the Rust is expected not to compile until Wave 3 on `staging`. The plan's documents are not on `main`. Read them from the plan branch without committing them: `git fetch origin claude/relaxed-archimedes-i5fdah && git show origin/claude/relaxed-archimedes-i5fdah:docs/v0.3.0/SURFACE.md` (likewise `README.md`, `PORT-MAP.md`, `port-map.tsv`, `research/*.md` and `parts/<your brief>.md`).

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/engine/tests/rules/effects_after_check.rs` | new | port of `packages/engine/test/effects-after-check.test.ts` (229 lines). |
| `crates/engine/tests/rules/effects_animate.rs` | new | port of `packages/engine/test/effects-animate.test.ts` (76 lines). |
| `crates/engine/tests/rules/effects_boardwide.rs` | new | port of `packages/engine/test/effects-boardwide.test.ts` (698 lines). |
| `crates/engine/tests/rules/effects_brittle.rs` | new | port of `packages/engine/test/effects-brittle.test.ts` (87 lines). |
| `crates/engine/tests/rules/effects_buff.rs` | new | port of `packages/engine/test/effects-buff.test.ts` (298 lines). |
| `crates/engine/tests/rules/effects_card_scope.rs` | new | port of `packages/engine/test/effects-cardScope.test.ts` (100 lines). |
| `crates/engine/tests/rules/effects_cast_chaos.rs` | new | port of `packages/engine/test/effects-cast-chaos.test.ts` (80 lines). |
| `crates/engine/tests/rules/effects_cast.rs` | new | port of `packages/engine/test/effects-cast.test.ts` (496 lines). |
| `crates/engine/tests/rules/effects_choose_where.rs` | new | port of `packages/engine/test/effects-choose-where.test.ts` (72 lines). |
| `crates/engine/tests/rules/effects_choose.rs` | new | port of `packages/engine/test/effects-choose.test.ts` (455 lines). |
| `crates/engine/tests/rules/effects_combat.rs` | new | port of `packages/engine/test/effects-combat.test.ts` (639 lines). |
| `crates/engine/tests/rules/effects_core.rs` | new | port of `packages/engine/test/effects-core.test.ts` (784 lines). |
| `crates/engine/tests/rules/effects_cost.rs` | new | port of `packages/engine/test/effects-cost.test.ts` (203 lines). |
| `crates/engine/tests/rules/effects_counters.rs` | new | port of `packages/engine/test/effects-counters.test.ts` (200 lines). |
| `crates/engine/tests/rules/effects_cry.rs` | new | port of `packages/engine/test/effects-cry.test.ts` (248 lines). |
| `crates/engine/tests/rules/effects_damage.rs` | new | port of `packages/engine/test/effects-damage.test.ts` (209 lines). |
| `crates/engine/tests/rules/effects_datacenter.rs` | new | port of `packages/engine/test/effects-datacenter.test.ts` (262 lines). |
| `crates/engine/tests/rules/effects_delay.rs` | new | port of `packages/engine/test/effects-delay.test.ts` (555 lines). |
| `crates/engine/tests/rules/effects_destroy.rs` | new | port of `packages/engine/test/effects-destroy.test.ts` (338 lines). |
| `crates/engine/tests/rules/effects_draw_while.rs` | new | port of `packages/engine/test/effects-drawWhile.test.ts` (128 lines). |
| `crates/engine/tests/rules/effects_each.rs` | new | port of `packages/engine/test/effects-each.test.ts` (110 lines). |
| `crates/engine/tests/rules/effects_enchant.rs` | new | port of `packages/engine/test/effects-enchant.test.ts` (115 lines). |
| `crates/engine/tests/rules/effects_flicker.rs` | new | port of `packages/engine/test/effects-flicker.test.ts` (224 lines). |
| `crates/engine/tests/rules/effects_fruit.rs` | new | port of `packages/engine/test/effects-fruit.test.ts` (356 lines). |
| `crates/engine/tests/rules/effects_give.rs` | new | port of `packages/engine/test/effects-give.test.ts` (353 lines). |
| `crates/engine/tests/rules/effects_hand_exile.rs` | new | port of `packages/engine/test/effects-hand-exile.test.ts` (88 lines). |
| `crates/engine/tests/rules/effects_heal.rs` | new | port of `packages/engine/test/effects-heal.test.ts` (182 lines). |
| `crates/engine/tests/rules/effects_health.rs` | new | port of `packages/engine/test/effects-health.test.ts` (105 lines). |
| `crates/engine/tests/rules/effects_library.rs` | new | port of `packages/engine/test/effects-library.test.ts` (600 lines). |
| `crates/engine/tests/rules/effects_locks.rs` | new | port of `packages/engine/test/effects-locks.test.ts` (199 lines). |
| `crates/engine/tests/rules/effects_move.rs` | new | port of `packages/engine/test/effects-move.test.ts` (492 lines). |
| `crates/engine/tests/rules/effects_perks.rs` | new | port of `packages/engine/test/effects-perks.test.ts` (99 lines). |
| `crates/engine/tests/rules/effects_plague_random_cast.rs` | new | port of `packages/engine/test/effects-plague-random-cast.test.ts` (86 lines). |
| `crates/engine/tests/rules/effects_plague.rs` | new | port of `packages/engine/test/effects-plague.test.ts` (464 lines). |
| `crates/engine/tests/rules/effects_plus_c.rs` | new | port of `packages/engine/test/effects-plus-c.test.ts` (290 lines). |
| `crates/engine/tests/rules/effects_radiant.rs` | new | port of `packages/engine/test/effects-radiant.test.ts` (265 lines). |
| `crates/engine/tests/rules/effects_random.rs` | new | port of `packages/engine/test/effects-random.test.ts` (720 lines). |
| `crates/engine/tests/rules/effects_reveal.rs` | new | port of `packages/engine/test/effects-reveal.test.ts` (47 lines). |
| `crates/engine/tests/rules/effects_shuffle_card.rs` | new | port of `packages/engine/test/effects-shuffle-card.test.ts` (106 lines). |
| `crates/engine/tests/rules/effects_split.rs` | new | port of `packages/engine/test/effects-split.test.ts` (87 lines). |
| `crates/engine/tests/rules/effects_statuses.rs` | new | port of `packages/engine/test/effects-statuses.test.ts` (101 lines). |
| `crates/engine/tests/rules/effects_steal.rs` | new | port of `packages/engine/test/effects-steal.test.ts` (192 lines). |
| `crates/engine/tests/rules/effects_summon_copies.rs` | new | port of `packages/engine/test/effects-summon-copies.test.ts` (382 lines). |
| `crates/engine/tests/rules/effects_summon.rs` | new | port of `packages/engine/test/effects-summon.test.ts` (401 lines). |
| `crates/engine/tests/rules/effects_summon_this.rs` | new | port of `packages/engine/test/effects-summonThis.test.ts` (231 lines). |
| `crates/engine/tests/rules/effects_swap.rs` | new | port of `packages/engine/test/effects-swap.test.ts` (306 lines). |
| `crates/engine/tests/rules/effects_targets.rs` | new | port of `packages/engine/test/effects-targets.test.ts` (320 lines). |
| `crates/engine/tests/rules/effects_transform.rs` | new | port of `packages/engine/test/effects-transform.test.ts` (372 lines). |
| `crates/engine/tests/rules/effects_tune.rs` | new | port of `packages/engine/test/effects-tune.test.ts` (649 lines). |
| `crates/engine/tests/rules/effects_turn_end.rs` | new | port of `packages/engine/test/effects-turnEnd.test.ts` (294 lines). |
| `crates/engine/tests/rules/fixtures/activate.rs` | new | port of `packages/engine/test/fixtures/activate.ts` (393 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/board_history.rs` | new | port of `packages/engine/test/fixtures/boardHistory.ts` (57 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/call_to_chaos_plus.rs` | new | port of `packages/engine/test/fixtures/callToChaosPlus.ts` (81 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/catalog.rs` | new | port of `packages/engine/test/fixtures/catalog.ts` (78 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/combat.rs` | new | port of `packages/engine/test/fixtures/combat.ts` (408 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/copied_text.rs` | new | port of `packages/engine/test/fixtures/copiedText.ts` (136 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/core_patches.rs` | new | port of `packages/engine/test/fixtures/corePatches.ts` (86 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/damage_combat.rs` | new | port of `packages/engine/test/fixtures/damage-combat.ts` (510 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/datacenter.rs` | new | port of `packages/engine/test/fixtures/datacenter.ts` (63 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/field.rs` | new | port of `packages/engine/test/fixtures/field.ts` (301 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/fruit.rs` | new | port of `packages/engine/test/fixtures/fruit.ts` (71 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/generation.rs` | new | port of `packages/engine/test/fixtures/generation.ts` (428 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/harness.rs` | new | port of `packages/engine/test/fixtures/harness.ts` (107 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/instance_data.rs` | new | port of `packages/engine/test/fixtures/instanceData.ts` (361 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/kill_credit.rs` | new | port of `packages/engine/test/fixtures/killCredit.ts` (96 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/ky_test.rs` | new | port of `packages/engine/test/fixtures/kyTest.ts` (63 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/last_boards.rs` | new | port of `packages/engine/test/fixtures/lastBoards.ts` (78 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/papaya.rs` | new | port of `packages/engine/test/fixtures/papaya.ts` (70 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/play_pipeline_a.rs` | new | port of `packages/engine/test/fixtures/playPipelineA.ts` (352 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/play_pipeline_b.rs` | new | port of `packages/engine/test/fixtures/playPipelineB.ts` (384 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/prompt_harness.rs` | new | port of `packages/engine/test/fixtures/promptHarness.ts` (125 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/prompts.rs` | new | port of `packages/engine/test/fixtures/prompts.ts` (649 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/quests.rs` | new | port of `packages/engine/test/fixtures/quests.ts` (266 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/rng_child.rs` | new | port of `packages/engine/test/fixtures/rng-child.ts` (9 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/scripts.rs` | new | port of `packages/engine/test/fixtures/scripts.ts` (253 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/turn.rs` | new | port of `packages/engine/test/fixtures/turn.ts` (285 lines). test-only scripts/catalog, registered through testkit |
| `crates/engine/tests/rules/fixtures/twice_forward.rs` | new | port of `packages/engine/test/fixtures/twiceForward.ts` (67 lines). test-only scripts/catalog, registered through testkit |

Do **not** touch: anything under `crates/*/src/` (the implementation), other parts' test files.

### 3. Steps

1. Read SURFACE.md §4, §6, §8 and §15 (test names) fully.
2. First port the fixtures (`packages/engine/test/fixtures/*.ts` → `crates/engine/tests/rules/fixtures/*.rs`): each exports its test catalog as `pub fn catalog() -> CardDefs` (from the TS object literals, through `json_as`) and its scripts as `pub fn scripts() -> IndexMap<String, CardScripts>`; a test installs them with `testkit::register_catalog`/`register_scripts` (the thread-local override). `fixtures/lint/` is not ported.
3. For each row: write the destination file as a module of `crates/engine/tests/rules.rs` or `crates/cards/tests/cards.rs` (part 1 declared `pub mod <name>;` for it). Start with `use jackioh_engine::testkit::*;` (the whole engine plus the scenario harness).
4. One `#[test]` per TS `it`, one `mod` per TS `describe`, names by §7.3 (`it("R113 resumes …")` → `fn r113_resumes_…`). Translate `expect` calls with §8's table. Keep the TS file's header comment.
5. TS helpers local to a test file stay local Rust functions in that file. Seeds stay byte-identical strings.
6. A test that asserts on an error message keeps the exact text (the engine keeps TS's messages, §4.4.9).
7. Commit after every 5 files on your branch.

### 4. Tests

- Every `it` in every TS file of the table has a Rust `#[test]` with the same assertions, or a line in `.fullsend/notes/spec-gaps-part-24.md` saying why not.

### 5. Done when

- [ ] Every destination in the table exists with its tests; `TESTS-IN:` line and `## SPEC GAPS` heading in `.fullsend/notes/spec-gaps-part-24.md` (empty list is fine).
- [ ] No test is skipped, `#[ignore]`d or weakened.
- [ ] `.fullsend/notes/part-24.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A TS test that pokes engine internals with a name the Rust may not keep: write it against §4.2's name anyway; part 31 reconciles.
- A TS test that reads its own or another file's source text (`readFileSync` of a `.ts`) is dropped and listed in spec-gaps (#133's rule).

<!-- /jackioh-bot:plan -->
