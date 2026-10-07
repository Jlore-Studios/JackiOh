# v0.3.0 (part 25 of 40): engine tests 2: rulings, subsystems, prompts and triggers

Part 25 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-25.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the engine's rulings suites (`rulings-a/b/c`), subsystem tests, and prompt, work, trigger and state-check tests from vitest to Rust tests, file for file (the table below), against the names SURFACE.md fixes. These tests, written without seeing the Rust engine, are the arbiter in Wave 3: when they and the implementation disagree, the implementation changes.

**How you work (fullsend spec-tester rules, adapted to a port; full text in `.claude/skills/fullsend/agents/spec-tester.md`).**
1. Your spec is the TypeScript test file plus SURFACE.md. Port every `it(...)` against the Rust names SURFACE.md fixes. Do not read the Rust under `crates/*/src/`: it is being written now, and your tests must not ratify what it happens to do.
2. Keep every assertion; never weaken or drop a test except where your table says so. A test you cannot express against SURFACE.md goes to `.fullsend/notes/spec-gaps-part-25.md` with its TS line, not into a guess.
3. Names by SURFACE §7.3: an `R<n>` in the title is a leading `r<n>_` token (`spec check` reads it).
4. Never run the compiler or the tests: they are expected not to compile until Wave 3.
5. Write only your table's files; no stubs; call what you wish existed; don't ask; stop when Done when holds.

**Delivering (night bot).** Build on your own branch and open your pull request to `main` as you always do. Never push to `staging` and never create it: #306's orchestrator merges `main` into `staging`. Commit only the files in your table and your two notes files. Your pull request adds new files and changes none, so `main`'s TypeScript CI stays green. Do not run `cargo` (builder rule 1): `main` has no Cargo workspace, and the Rust is expected not to compile until Wave 3 on `staging`. The plan's documents are not on `main`. Read them from the plan branch without committing them: `git fetch origin claude/relaxed-archimedes-i5fdah && git show origin/claude/relaxed-archimedes-i5fdah:docs/v0.3.0/SURFACE.md` (likewise `README.md`, `PORT-MAP.md`, `port-map.tsv`, `research/*.md` and `parts/<your brief>.md`).

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/engine/tests/rules/activate.rs` | new | port of `packages/engine/test/activate.test.ts` (827 lines). |
| `crates/engine/tests/rules/ai_policy.rs` | new | port of `packages/engine/test/aiPolicy.test.ts` (256 lines). |
| `crates/engine/tests/rules/audit.rs` | new | port of `packages/engine/test/audit.test.ts` (99 lines). |
| `crates/engine/tests/rules/board_history.rs` | new | port of `packages/engine/test/boardHistory.test.ts` (317 lines). |
| `crates/engine/tests/rules/call_to_chaos.rs` | new | port of `packages/engine/test/callToChaos.test.ts` (682 lines). |
| `crates/engine/tests/rules/call_to_chaos_plus.rs` | new | port of `packages/engine/test/callToChaosPlus.test.ts` (339 lines). |
| `crates/engine/tests/rules/combo_index.rs` | new | port of `packages/engine/test/comboIndex.test.ts` (522 lines). |
| `crates/engine/tests/rules/copied_text.rs` | new | port of `packages/engine/test/copied-text.test.ts` (353 lines). |
| `crates/engine/tests/rules/core_patches.rs` | new | port of `packages/engine/test/corePatches.test.ts` (390 lines). |
| `crates/engine/tests/rules/death_pause.rs` | new | port of `packages/engine/test/death-pause.test.ts` (504 lines). |
| `crates/engine/tests/rules/delayed_kinds.rs` | new | port of `packages/engine/test/delayed-kinds.test.ts` (270 lines). |
| `crates/engine/tests/rules/fuse_registry.rs` | new | port of `packages/engine/test/fuse-registry.test.ts` (196 lines). |
| `crates/engine/tests/rules/fuse_variants.rs` | new | port of `packages/engine/test/fuse-variants.test.ts` (540 lines). |
| `crates/engine/tests/rules/fuse.rs` | new | port of `packages/engine/test/fuse.test.ts` (615 lines). |
| `crates/engine/tests/rules/glitch.rs` | new | port of `packages/engine/test/glitch.test.ts` (218 lines). |
| `crates/engine/tests/rules/hero_power.rs` | new | port of `packages/engine/test/heroPower.test.ts` (528 lines). |
| `crates/engine/tests/rules/ky_test.rs` | new | port of `packages/engine/test/kyTest.test.ts` (282 lines). |
| `crates/engine/tests/rules/last_boards.rs` | new | port of `packages/engine/test/lastBoards.test.ts` (283 lines). |
| `crates/engine/tests/rules/modifiers.rs` | new | port of `packages/engine/test/modifiers.test.ts` (573 lines). |
| `crates/engine/tests/rules/papaya.rs` | new | port of `packages/engine/test/papaya.test.ts` (420 lines). |
| `crates/engine/tests/rules/pauses.rs` | new | port of `packages/engine/test/pauses.test.ts` (589 lines). |
| `crates/engine/tests/rules/perfect_hand.rs` | new | port of `packages/engine/test/perfectHand.test.ts` (257 lines). |
| `crates/engine/tests/rules/prompt_kinds.rs` | new | port of `packages/engine/test/prompt-kinds.test.ts` (601 lines). |
| `crates/engine/tests/rules/prompts.rs` | new | port of `packages/engine/test/prompts.test.ts` (909 lines). |
| `crates/engine/tests/rules/quests.rs` | new | port of `packages/engine/test/quests.test.ts` (658 lines). |
| `crates/engine/tests/rules/rulings_a.rs` | new | port of `packages/engine/test/rulings-a.test.ts` (1385 lines). |
| `crates/engine/tests/rules/rulings_b.rs` | new | port of `packages/engine/test/rulings-b.test.ts` (1682 lines). |
| `crates/engine/tests/rules/rulings_c.rs` | new | port of `packages/engine/test/rulings-c.test.ts` (2539 lines). |
| `crates/engine/tests/rules/scorer.rs` | new | port of `packages/engine/test/scorer.test.ts` (264 lines). |
| `crates/engine/tests/rules/start_of_opponent_turn.rs` | new | port of `packages/engine/test/start-of-opponent-turn.test.ts` (77 lines). |
| `crates/engine/tests/rules/statecheck.rs` | new | port of `packages/engine/test/statecheck.test.ts` (641 lines). |
| `crates/engine/tests/rules/stays.rs` | new | port of `packages/engine/test/stays.test.ts` (98 lines). |
| `crates/engine/tests/rules/trap_cardresolved.rs` | new | port of `packages/engine/test/trap-cardresolved.test.ts` (261 lines). |
| `crates/engine/tests/rules/trap_window_pause.rs` | new | port of `packages/engine/test/trap-window-pause.test.ts` (349 lines). |
| `crates/engine/tests/rules/trigger_zones.rs` | new | port of `packages/engine/test/trigger-zones.test.ts` (366 lines). |
| `crates/engine/tests/rules/triggers.rs` | new | port of `packages/engine/test/triggers.test.ts` (432 lines). |
| `crates/engine/tests/rules/twice_forward.rs` | new | port of `packages/engine/test/twiceForward.test.ts` (255 lines). |

Do **not** touch: anything under `crates/*/src/` (the implementation), other parts' test files.

### 3. Steps

1. Read SURFACE.md §4, §6, §8 and §15 (test names) fully.
2. For each row: write the destination file as a module of `crates/engine/tests/rules.rs` or `crates/cards/tests/cards.rs` (part 1 declared `pub mod <name>;` for it). Start with `use jackioh_engine::testkit::*;` (the whole engine plus the scenario harness).
3. One `#[test]` per TS `it`, one `mod` per TS `describe`, names by §7.3 (`it("R113 resumes …")` → `fn r113_resumes_…`). Translate `expect` calls with §8's table. Keep the TS file's header comment.
4. TS helpers local to a test file stay local Rust functions in that file. Seeds stay byte-identical strings.
5. A test that asserts on an error message keeps the exact text (the engine keeps TS's messages, §4.4.9).
6. Commit after every 5 files on your branch.

### 4. Tests

- Every `it` in every TS file of the table has a Rust `#[test]` with the same assertions, or a line in `.fullsend/notes/spec-gaps-part-25.md` saying why not.

### 5. Done when

- [ ] Every destination in the table exists with its tests; `TESTS-IN:` line and `## SPEC GAPS` heading in `.fullsend/notes/spec-gaps-part-25.md` (empty list is fine).
- [ ] No test is skipped, `#[ignore]`d or weakened.
- [ ] `.fullsend/notes/part-25.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A TS test that pokes engine internals with a name the Rust may not keep: write it against §4.2's name anyway; part 31 reconciles.
- A TS test that reads its own or another file's source text (`readFileSync` of a `.ts`) is dropped and listed in spec-gaps (#133's rule).

<!-- /jackioh-bot:plan -->
