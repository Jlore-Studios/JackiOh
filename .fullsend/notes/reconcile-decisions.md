# engine tests

## Cluster: fixture CardDef shape (static vs fn)
Winner: each `crates/engine/tests/rules/fixtures/<x>.rs` as written — score n/a (rule 2: the fixture file owns its names) — LazyLock statics in combat, field, instance_data, generation, damage_combat, datacenter, activate, call_to_chaos_plus, core_patches, fruit, kill_credit, board_history; `fn x() -> CardDef` in play_pipeline_b, prompts, quests, scripts, turn, twice_forward, ky_test, papaya
Losers: the callers' guesses (`plain().id` for a static, `titan.id` for a fn) — rewritten, no fixture changed
Callers updated: 1,354 sites (1,344 static calls → `x.field` / `x.field.clone()` where moved / `&*x` / `x.clone()`; 10 fn field reads → `x().id`) in 89 test files
Semantic conflicts: test.fixture_defs → the fixture file's own definition (rule 2)
Unresolved: none
Lines: before 99,006, after 98,968 (whole `crates/engine/tests`, all clusters)

## Cluster: fixture script tables (TS static vs brief `scripts()` vs hedge alias)
Winner: the TS-named statics (`TURN_SCRIPTS`, `TWICE_FORWARD_SCRIPTS`, `PB_SCRIPTS`, `PROMPT_SCRIPTS`, `FIXTURE_SCRIPTS`, `*_DEFS`; SURFACE §4.2: constants keep their TS names) — score 0 behaviours lost, 0 options — plus each fixture's brief-mandated `scripts()`/`catalog()` (part 24 brief step 2), kept
Losers: `turn_scripts()`, `turn_defs()`, `twice_forward_scripts()`, `pb_scripts()`, `pb_defs()`, `prompt_scripts()`, `prompt_defs()`, `fixture_scripts()`, `fixture_defs()` (hedge aliases that only cloned the static) — deleted
Callers updated: 5 files (turn_wiring, delayed_kinds, draw_limit, twice_forward, effects_fruit); plus names no fixture had, pointed at the owners': `activate_scripts()`→`ACTIVATE_SCRIPTS`, `combat_scripts()`→`COMBAT_SCRIPTS`, `fruit_scripts()`→`FRUIT_SCRIPTS`, `gen_scripts()`→`GEN_SCRIPTS`, `core_patch_scripts()`→`CORE_PATCH_SCRIPTS`, `lab_pool()`→`LAB_POOL`, `test_mark()`→`TEST_MARK`, `ct()`→`CT`, `rng_child::run`→`rng_child::rng_child` (8 files)
Semantic conflicts: turn::scripts() vs twice_forward_scripts()/activate_scripts() naming → TS constant names
Unresolved: `scripts()` and the static are still two names for one table in every fixture; kept because part 24's brief mandates `scripts()` and the TS name is the static (removing either touches ~25 files)
Lines: 49 deleted in fixtures

## Cluster: harness helper signatures (put, new_game, in_hand, set_library, flush, vanilla_deck, token_def, cast_now, recorder, replayable, action helpers)
Winner: the owners' definitions — `harness::put(state, id, slot, options: Value)` (4 args), `harness::new_game(&str, Option<(Vec, Vec)>)`, `instance_data::instance_game(&str, Option<..>)`, `harness::in_hand(.., count: i32)`, `harness::set_library(.., &[impl AsRef<str>])`, `field::flush(.., mana: i32)`, `catalog::vanilla_deck(i32, i32)`/`vanilla_catalog(i32, i32)`, `catalog::token_def(name, impl Into<Vec<Tag>>)`, `prompt_harness::cast_now(.., PlayerId, bool) -> SinkResult`, `damage_combat::recorder(&GameState)`, `prompt_harness::replayable(&str, &[String], &[String])`, and `act`/`act_result`/`pb_act`/`pb_reduce`/`play`/`refusal` taking `impl Serialize`
Losers: callers' 3-arg `put` (794), 1-arg `new_game`/`instance_game` (11), `None`/`Some(n)` for TS defaults (in_hand 41, flush 9, vanilla_deck/catalog 35 calls, token_def 5, cast_now 3, instance_game 2), `harness::PutOptions` (3), `&[]` without a type (14), owned `recorder` args (32), `json_as(json!)` into `impl Serialize` (81), `&[x.as_str()]` into replayable (6), `OffFieldZone` into `damage_combat::in_pile` (18 → `ZoneName`)
Callers updated: ~1,050 sites
Semantic conflicts: test.default_params → fixture owners take TS defaults as plain values (TS `= 1`, `= 10`, `= 20, 1`, `= ["Token"]`, `= "p1", false`); test.literals → `json!` into fixture helpers that take `impl Serialize`
Unresolved: none for fixtures; engine functions with the same Option-vs-plain question are the engine owner's (damage file, Seams)

## Cluster: LOG_LANE type
Winner: `i32` (fixtures/activate.rs, fixtures/damage_combat.rs) — settled by part 1's frozen `ZoneRef.lane: i32`, which `harness::slot` takes
Losers: `fixtures/turn.rs`'s `pub const LOG_LANE: usize` — changed to `i32`, its two indexing uses cast
Callers updated: 0 (all four callers already passed it to `slot`)
Semantic conflicts: lane number type → i32 (part 1, SURFACE §4.3 game quantity)
Unresolved: none
Lines: 0

## Decision: local mechanical fixes taken with the clusters
One syntax error (`fixtures/copied_text.rs:253`, a missing `)`, which stopped the whole `rules` binary at the parser); four parameters renamed that shadowed fixture statics (E0530: `body` in fuse_variants, instance_data ×2, params); `use std::sync::Arc;` in four files (the testkit exports no `Arc`); `preview.rs`'s helper reads `wire::UnitView` (part 1: the root `UnitView` is `layers`'). No assertion changed.

# ai

Part 31's AI reconciler, scope `crates/ai/**` (part 17's three chunks). Measured by a shadow build
against the engine's signatures on `staging` (`.fullsend/damage/ai.md` says how): AI lib 21 → 0
errors, AI test binary 41 → 0. Commits: the four `v0.3.0 part 31 (ai): …` on `staging`.

## Cluster: NodeCounter (the shared node counter)
Winner: `crates/ai/src/types.rs` + `crates/ai/src/simulate.rs` — owner (decision rule 2; no rival implementation existed, only 17.1's and 17.3's callers' guesses) — `trait NodeCounter` with `&self` methods and `usize` counts, `Cell` tallies, `sim_error_tally()`; every node-spending function takes `&dyn NodeCounter`
Losers: 17.1's `&mut dyn NodeCounter` with a trait `sim_errors()` (callers in `decide.rs`, `lethal.rs`, `reply.rs`, `search.rs`, `baselines.rs`) — rewritten to the winner; nothing to delete
Callers updated: 31 in src (`&mut dyn` → `&dyn` ×11 signatures, `&mut *counter`/`&*counter` → `counter` ×20, sub-counters bound immutably) and 43 in tests (`&mut counter`/`&mut create_node_counter(..)` → `&`, `let mut counter` → `let`); `decide`'s stats closure takes `&CountingNodeCounter` and reads its inherent `sim_errors()` (TS's `CountingNodeCounter.simErrors`)
Semantic conflicts: `ai.counter` (17.1, 17.3: `&mut dyn`; 17.2: `&dyn`) → `&dyn` (orchestrator's decision; `types.rs` owns it)
Unresolved: —
Lines: this cluster and the next, together: 57 lines changed in src (decide 18, lethal 18, search 9, reply 8, baselines 2, match_ 2), none added; tests: 43 counter arguments changed in place

## Cluster: node budgets and counts (`SearchBudget` and what it feeds)
Winner: `crates/ai/src/types.rs` — owner — `SearchBudget`, `SearchStats` and `NodeCounter` count nodes in `usize`
Losers: 17.1's `i32` node counts — rewritten: `find_lethal(.., limit: usize)` (17.2's tests already call it so), `MatchRecord.nodes: usize`, `decide`'s `Scratch { determinizations, lines }: usize`, search's `expansion(.., width: usize)`; TS's `Math.max(0, a - b)` is `saturating_sub`
Callers updated: 12 (decide 6, lethal 2, search 2, baselines 1, match_ 1)
Semantic conflicts: `ai.weights` (17.1 "every AI count i32") against 17.2 "node counts usize" → node budgets and counts `usize`; the AI's own config counts (`AI_SEARCH.*`, `AI_REPLY.*`, `AI_MULLIGAN.keep_max_cost`) stay `i32` (17.1 owns `config.rs`) and are cast with `as usize` where they meet a node count, as 17.2's notes asked. Not settled by SURFACE (spec-gaps.md).
Unresolved: —
Lines: counted with the cluster above

## Decision: derives the tests need (17.3)
Winner: `crates/ai/src/types.rs`, `crates/ai/src/sweep.rs` — already there: `Decision: PartialEq`, `DecisionReason: Copy`, `SearchBudget: Copy`, `StoppedBy: Serialize`, `SweepStats`/`SweepResult`/`SweepPass2`/`SweepVerdict`/`SweepFlag: Serialize + Deserialize`
Losers: —
Callers updated: 0
Semantic conflicts: —
Unresolved: —
Lines: before 0, after 0

## Decision: `AiDeckOptions` (SURFACE §4.3)
Winner: `crates/ai/src/deck.rs` — already `banned: Option<Vec<String>>`, `mana_cap: Option<i32>`, `Default`
Losers: part 19.2's `banned: Vec<String>` at `crates/server/src/actor/engine.rs:187` (`banned: vec![]`) — **for the server reconciler**: write `banned: Some(vec![])` (a human's or a live game's random deck bans nothing; `None` would be the AI's shadow ban). Not edited here (outside the scope).
Callers updated: 0 in scope (tools' `arena.rs` and `gate.rs` already match)
Semantic conflicts: —
Unresolved: the server line above
Lines: before 0, after 0

## Decision: `MatchHooks` (one shape, boxed closures, plus the test seam)
Winner: `crates/ai/src/match_.rs` — boxed `FnMut` hooks, `Default`; gains 17.2's GAPS item `override_choice: Option<OverrideChoiceHook<'a>>` (`Box<dyn FnMut(&GameState, PlayerId, Option<ActionBody>) -> Option<ActionBody> + 'a>`), called inside the controller's `catch_unwind` after `choose_for` and before a `None` is replaced by `random_action`; it stands in for `match-refusal.test.ts`'s `vi.mock` of the baselines
Losers: —
Callers updated: 0 (every literal outside the file spreads `..MatchHooks::default()`)
Semantic conflicts: —
Unresolved: —
Lines: before 0, after 14

## Decision: `find_lethal_with_quick_nodes` (17.2 GAPS)
Winner: `crates/ai/src/lethal.rs` — added: `find_lethal` with the depth-first share given; `find_lethal` calls it with `AI_SEARCH.lethal_quick_nodes` (TS's test set `AI_SEARCH.lethalQuickNodes` at run time, which a `const` cannot be)
Losers: —
Callers updated: 1 (`find_lethal`)
Semantic conflicts: —
Unresolved: —
Lines: before 0, after 12

## Cluster: test helpers (`tests/ai/support.rs`)
Winner: `crates/ai/tests/ai/support.rs` (17.2) — owner — `act(state, seat, body)` + `act_with_nonce(.., nonce)`, `run_puzzle(name, setup)` + `run_puzzle_with(.., budget)`, `random_decks(seed)` + `random_decks_sized`, `random_policy_states(seed, every, max_actions)`, `PuzzleRun` with pub fields
Losers: 17.3's guessed shapes (`act(.., Option<&str>)`, `run_puzzle(.., Option<SearchBudget>)`, `random_decks(seed, None)`, `random_policy_states(.., Some(max))`) — calls rewritten, no assertion touched
Callers updated: 19 (observe 8, reply 4, surface 3, puzzles 2, search 1, prompts_v020 1 with its import)
Semantic conflicts: —
Unresolved: `support.rs:29` re-exports `Scenario`/`ScenarioOptions`, which no test imports through it (an unused-import warning; a `-D warnings` item for Wave 3, left since it is 17.2's stated design)
Lines: before 20, after 19 (one multi-line call shortened)

## Cluster: owners' argument forms in test calls
Winner: the owners' signatures — `determinize(.., DeterminizeOptions)` by value (`determinize.rs`, 17.1), `beam_search(.., SearchBudget)` by value (`search.rs`, 17.1), `sweep_card`/`sweep_at_risk(.., &SweepOptions)` (`sweep.rs`, 17.2; tools' `sweep.rs` already passes `&`), `dev_game_config(n, series: &str, budget: Option<SearchBudget>)` (`dev_run.rs`, 17.1; TS's `Pick<DevRunOptions, "series" | "budget">` as two arguments)
Losers: the tests' `&DeterminizeOptions`, `&AI_GATE_BUDGET`, by-value `SweepOptions`, `dev_game_config(n, &DevRunOptions)` and its `series_only` helper (deleted)
Callers updated: 14 (shadow_ban 5, surface 3, dev_run 3, determinize_shown_cost 2, lethal 1)
Semantic conflicts: —
Unresolved: —
Lines: before 23, after 14 (series_only deleted)

## Decision: default arguments passed explicitly
Winner: the owners' arities — `evaluate(state, seat, NextSwing, &EvalWeights)`, `game_config(matchup, n, budget, series)`, `mulligan_keep(.., keep_max_cost)`, `simulate_reply`/`reply_score(.., hidden)`, `score_line(.., reply, hidden: Option<_>)`; every caller in the crate, tools and wasm already matches (shadow build)
Losers: —
Callers updated: 0
Semantic conflicts: `ai.defaults` (17.1, 17.3 agree)
Unresolved: —
Lines: before 0, after 0

## Not a collision: the panic-message helper
`match_::message_of` and `simulate::panic_message` both turn a panic payload into TS's `error instanceof Error ? error.message : String(error)`. TS wrote that twice as well (`match.ts`'s `messageOf`, `simulate.ts` inline), so both stay (decision rule 3). Per-file test helpers (`ai_options`, `det`, `js`, `spread`, `eval`, …) port per-file TS idioms and stay; tests are not edited beyond their calls.

## Checked, nothing to do
No persona code in `crates/ai` (R645: `config.rs` and `lib.rs` only say where it went). `crates/ai/Cargo.toml` is pure (SURFACE §3). Every engine name the AI calls resolves with the arity and reference kinds it uses, against `staging` as pulled before the last push.
