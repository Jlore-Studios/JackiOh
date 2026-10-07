# v0.3.0 (part 17 of 40): the AI in Rust (generation 0)

Part 17 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-17.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the practice AI as it is at `91cc43c` (the last and only AI that passed its gates: 94/100 vs random, 35/50 vs greedy, 47/50 Hard vs Easy) to `crates/ai`, unchanged in algorithm and numbers: the lethal solver, the beam search on K=3 determinizations, the rule-based reply, the hand-weighted evaluation, `redact`/`determinize` (R185), the deck builder, the 11-card shadow ban, the gate, sweep, match and dev-run logic. This port is generation 0. The emote personas move to the web (part 21).

**How you work (fullsend builder rules, in priority order; full text in `.claude/skills/fullsend/agents/builder.md`).**
1. Never run `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or any test runner. Nothing on `staging` compiles until Wave 3; every error you would see is someone else's unwritten file. Count any you ran as `BUILDS-RUN`.
2. Write only the files in your Files to touch table and your two notes files. Read anything in `packages/`, `apps/`, `docs/`, `SPEC.md` (the TypeScript is your spec); do not read other parts' Rust under `crates/`: it is half-written.
3. Match SURFACE.md exactly at every boundary: paths by §4.1, names by §4.2, types by §4.3, the shapes of §5–§14.
4. No stubs: no `todo!()`, `unimplemented!()`, placeholder bodies or `// TODO`. Port the real body; if you truly cannot, leave the function out and list it under GAPS.
5. Duplicate on purpose: a small helper another part probably writes, write your own private copy.
6. Call what you wish existed: another module's function by its TS name snake_cased at its TS module's Rust path.
7. Don't ask questions: decide, and record the decision in your notes.
8. Stop when Done when holds.

**Delivering (night bot).** Build on your own branch and open your pull request to `main` as you always do. Never push to `staging` and never create it: #306's orchestrator merges `main` into `staging`. Commit only the files in your table and your two notes files. Your pull request adds new files and changes none, so `main`'s TypeScript CI stays green. Do not run `cargo` (builder rule 1): `main` has no Cargo workspace, and the Rust is expected not to compile until Wave 3 on `staging`. The plan's documents are not on `main`. Read them from the plan branch without committing them: `git fetch origin claude/relaxed-archimedes-i5fdah && git show origin/claude/relaxed-archimedes-i5fdah:docs/v0.3.0/SURFACE.md` (likewise `README.md`, `PORT-MAP.md`, `port-map.tsv`, `research/*.md` and `parts/<your brief>.md`).

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/ai/src/baselines.rs` | new | port of `packages/ai/src/baselines.ts` (66 lines). |
| `crates/ai/src/candidates.rs` | new | port of `packages/ai/src/candidates.ts` (195 lines). |
| `crates/ai/src/config.rs` | new | port of `packages/ai/src/config.ts` (381 lines). minus EMOTE_TRIGGERS, EMOTE_REPLY_KEYS, AI_EMOTE, AI_PERSONAS, which move to apps/web/src/practice/personas.ts (part 21) |
| `crates/ai/src/decide.rs` | new | port of `packages/ai/src/decide.ts` (225 lines). |
| `crates/ai/src/deck.rs` | new | port of `packages/ai/src/deck.ts` (242 lines). |
| `crates/ai/src/determinize.rs` | new | port of `packages/ai/src/determinize.ts` (166 lines). |
| `crates/ai/src/dev_run.rs` | new | port of `packages/ai/src/devRun.ts` (80 lines). |
| `crates/ai/src/evaluate.rs` | new | port of `packages/ai/src/evaluate.ts` (240 lines). |
| `crates/ai/src/gate.rs` | new | port of `packages/ai/src/gate.ts` (249 lines). |
| `crates/ai/src/lethal.rs` | new | port of `packages/ai/src/lethal.ts` (247 lines). |
| `crates/ai/src/match_.rs` | new | port of `packages/ai/src/match.ts` (234 lines). |
| `crates/ai/src/mulligan.rs` | new | port of `packages/ai/src/mulligan.ts` (21 lines). |
| `crates/ai/src/observe.rs` | new | port of `packages/ai/src/observe.ts` (373 lines). |
| `crates/ai/src/reply.rs` | new | port of `packages/ai/src/reply.ts` (234 lines). |
| `crates/ai/src/search.rs` | new | port of `packages/ai/src/search.ts` (143 lines). |
| `crates/ai/src/shadow_ban.rs` | new | port of `packages/ai/src/shadowBan.ts` (53 lines). |
| `crates/ai/src/simulate.rs` | new | port of `packages/ai/src/simulate.ts` (212 lines). |
| `crates/ai/src/sweep.rs` | new | port of `packages/ai/src/sweep.ts` (450 lines). |
| `crates/ai/src/types.rs` | new | port of `packages/ai/src/types.ts` (65 lines). |
| `crates/ai/tests/ai/support.rs` | new | port of `packages/ai/test/_support.ts` (212 lines). |
| `crates/ai/tests/ai/activate.rs` | new | port of `packages/ai/test/activate.test.ts` (200 lines). |
| `crates/ai/tests/ai/answer_key.rs` | new | port of `packages/ai/test/answer-key.test.ts` (90 lines). |
| `crates/ai/tests/ai/decide.rs` | new | port of `packages/ai/test/decide.test.ts` (403 lines). |
| `crates/ai/tests/ai/deck.rs` | new | port of `packages/ai/test/deck.test.ts` (275 lines). |
| `crates/ai/tests/ai/determinize_shown_cost.rs` | new | port of `packages/ai/test/determinize-shown-cost.test.ts` (105 lines). |
| `crates/ai/tests/ai/dev_run.rs` | new | port of `packages/ai/test/dev-run.test.ts` (97 lines). |
| `crates/ai/tests/ai/evaluate_v020.rs` | new | port of `packages/ai/test/evaluate-v020.test.ts` (258 lines). |
| `crates/ai/tests/ai/evaluate.rs` | new | port of `packages/ai/test/evaluate.test.ts` (223 lines). |
| `crates/ai/tests/ai/lethal.rs` | new | port of `packages/ai/test/lethal.test.ts` (188 lines). |
| `crates/ai/tests/ai/match_refusal.rs` | new | port of `packages/ai/test/match-refusal.test.ts` (120 lines). |
| `crates/ai/tests/ai/match_.rs` | new | port of `packages/ai/test/match.test.ts` (294 lines). |
| `crates/ai/tests/ai/observe_instance_data.rs` | new | port of `packages/ai/test/observe-instance-data.test.ts` (43 lines). |
| `crates/ai/tests/ai/observe.rs` | new | port of `packages/ai/test/observe.test.ts` (663 lines). |
| `crates/ai/tests/ai/prompts_v020.rs` | new | port of `packages/ai/test/prompts-v020.test.ts` (267 lines). |
| `crates/ai/tests/ai/puzzles.rs` | new | port of `packages/ai/test/puzzles.test.ts` (273 lines). |
| `crates/ai/tests/ai/redact_announce.rs` | new | port of `packages/ai/test/redact-announce.test.ts` (40 lines). |
| `crates/ai/tests/ai/redact_backrow_piles.rs` | new | port of `packages/ai/test/redact-backrow-piles.test.ts` (49 lines). |
| `crates/ai/tests/ai/redact_board_history.rs` | new | port of `packages/ai/test/redact-board-history.test.ts` (30 lines). |
| `crates/ai/tests/ai/redact_fusion.rs` | new | port of `packages/ai/test/redact-fusion.test.ts` (51 lines). |
| `crates/ai/tests/ai/redact_last_boards.rs` | new | port of `packages/ai/test/redact-last-boards.test.ts` (25 lines). |
| `crates/ai/tests/ai/redact_live_face_down.rs` | new | port of `packages/ai/test/redact-live-face-down.test.ts` (96 lines). |
| `crates/ai/tests/ai/reply.rs` | new | port of `packages/ai/test/reply.test.ts` (170 lines). |
| `crates/ai/tests/ai/search.rs` | new | port of `packages/ai/test/search.test.ts` (337 lines). |
| `crates/ai/tests/ai/shadow_ban.rs` | new | port of `packages/ai/test/shadowBan.test.ts` (432 lines). |
| `crates/ai/tests/ai/surface.rs` | new | port of `packages/ai/test/surface.test.ts` (319 lines). |
| `crates/ai/tests/ai/tutorial_tier.rs` | new | port of `packages/ai/test/tutorial-tier.test.ts` (240 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3, §4, §6 and §9, then the TS sources in your table.
2. Port each file by the table. `AiOptions.should_stop` is `Option<&dyn Fn() -> bool>`: the crate never reads a clock (§9). Node budgets count `reduce` calls exactly as `simulate.ts` does.
3. `config.rs`: everything in `packages/ai/src/config.ts` except `EMOTE_TRIGGERS`, `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS` and their types (they move to the web with the personas).
4. `shadow_ban.rs`: `SHADOW_BAN` as a sorted `&[(&str, &str)]` of the 11 TS entries with their reasons verbatim, `SHADOW_BAN_IDS`, `SHADOW_WATCH` empty; keep the TS header comment (the sweep of record).
5. `candidates.rs`'s `action_key` is the sorted-key JSON of the action: use `jackioh_engine::replay::canonical`.
6. Port the tests in the table to `crates/ai/tests/ai/*.rs` (they are TS behaviour, so porting them here does not ratify your code); `_support.ts` → `support.rs`. The gate tests are part 22's.
7. Commit after every 5 files on your branch.

### 4. Tests

- Every ported AI test (decide, search, lethal, evaluate, observe and the six `redact-*`, determinize, deck, shadowBan, match, puzzles, reply, prompts, activate, tutorial-tier, dev-run, answer-key, surface) as a Rust test.
- `shadow_ban.rs` holds exactly the 11 TS ids (V12).

### 5. Done when

- [ ] Every destination in the table is a full port; no persona code in the crate.
- [ ] `.fullsend/notes/part-17.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- The AI calls ~40 engine items hundreds of times per decision; name each by §4.2 at its TS module's path.
- The AI must stay seeded and deterministic: same `(state, seed, budget)` → same decision.

<!-- /jackioh-bot:plan -->
