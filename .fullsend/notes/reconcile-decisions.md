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

# tools and wasm

Measured by a shadow build (engine, cards and ai bodies stubbed; `.fullsend/damage/tools-wasm.md`): 3 errors before, 0 after, in `crates/tools` and `crates/wasm`, every target.

## Cluster: the fuzz deal (pool, decks per seed, handicap rotation, R265 actor order)
Winner: `crates/tools/src/fuzz.rs` (`FUZZ_POOL`, `decks_for_seed`, `handicap_for_seed` → `SeedHandicap`, `handicap_decks_for_seed`, `actor_of`) — score 36 (4 SURFACE §13.1 behaviours, 4 branches) — the port of fuzz.test.ts/fuzz-handicap.test.ts, which §13.1 says golden deals "exactly as"
Losers: `crates/tools/src/golden.rs`'s copies (`POOL_EXCLUSIONS`, `deck_legal_ids`, `fuzz_pool`, `handicap_pool`, `decks_for_seed(seed, pool)`, `handicap_for_seed -> (seat, tier, handicap)`, `handicap_decks_for_seed(.., pool)`, the inline actor choice) — score 36, tie → more callers → deleted (record.ts copied them only because TS cannot import a vitest file; a Rust module can)
Callers updated: 3 (golden.rs `spec_for_seed` ×2, `record_seed`); `actor_of` and `handicap_decks_for_seed` made `pub(crate)`
Semantic conflicts: none (same seeds, streams and pools)
Unresolved: none
Lines: before 76, after 11

## Cluster: the gate run's report
Winner: `crates/ai/src/gate.rs` `GateReport` — owner (gate.ts's `GateReport`), same five fields
Losers: `crates/tools/src/gate.rs` `GateRun` — deleted
Callers updated: 5 (`play_gate`, `write_shard`, `losing_seeds`, `check_report`, `check_clean`); the tests read the same field names, unchanged
Semantic conflicts: none
Unresolved: none
Lines: before 12, after 1

## Cluster: the matchup list
Winner: `crates/ai/src/gate.rs` `Matchup::ALL` / `Matchup::as_str` — owner
Losers: `tools/src/gate.rs` `matchups()`'s parse of `MATCHUP_NAMES` and `matchup_name`'s serde round trip — bodies replaced by the owner's
Callers updated: 0 (same signatures)
Semantic conflicts: tools.matchup ("read only through serde", 22-1) → the owner's `as_str`/`ALL`, which serde mirrors
Unresolved: `MATCHUP_NAMES` stays: `merge_holds_the_shards_together…` indexes it (tests are not edited); it is otherwise only `parse_matchup`'s error text
Lines: before 9, after 5

## Cluster: dates (unix days → YYYY-MM-DD, and today's UTC date)
Winner: `crates/tools/src/patches.rs` `utc_date_of` — rule 2 (patches.ts's `utcDateOf`; R646's test pins it); scores tie at 8–9 (one behaviour, 1–3 branches each). For "today": `crates/tools/src/arena.rs` `utc_date` — 2 callers (arena, promote) against 1
Losers: `arena.rs` `civil_date` and `SECONDS_PER_DAY`; `sweep.rs` `utc_date_today`, `SECONDS_PER_DAY`, `DAYS_FROM_0000_03_01_TO_EPOCH`, `DAYS_PER_ERA`, `YEARS_PER_ERA` — deleted
Callers updated: 4 (`arena::utc_date`'s body, `sweep::report`, and two tests' subjects: arena's `civil_dates_count_from_the_epoch` now holds `utc_date_of(days × 86 400)` to the same four dates, sweep's `the_date_is_iso_shaped` calls `utc_date()`; no assertion changed)
Semantic conflicts: year padding (`{year:04}` vs `{full_year}`) differs only before year 1000
Unresolved: none
Lines: before 41, after 6

## Cluster: git and the repository root
Winner: `crates/tools/src/patches.rs` `git(repo, args, env)` (now `pub(crate)`) and `repo_root()` — rule 2 (patches.ts's `execFileSync("git")` and `REPO_ROOT`)
Losers: `promote.rs` `git` (trimmed, `-C`) and `repo_root` (git toplevel of the cwd); `spec.rs` `default_root` (cwd ancestor search) — deleted
Callers updated: 5 (promote `ai_tree`, `ai_source_dirty`, `parent_commit` trim themselves; promote `run`; spec `run`, whose `--root` flag still overrides, doc updated)
Semantic conflicts: paths.root (22-2) vs spec.root (28) vs promote's cwd → source-relative, as TS's `REPO_ROOT` and rulings-coverage.ts's `here` were (SURFACE silent). Every documented use (`cargo jackioh …` in the checkout, CI, training/loop.sh) runs the binary built from the checkout it reads, so nothing moves.
Unresolved: none
Lines: before 32, after 9

## Cluster: small helpers written twice
Winner: `spec.rs` `is_ident_char` (3 callers to 1, same body; now `pub(crate)`); `agent.rs` `panic_message` (6 callers to 2; identical but for the no-message fallback text, which no test reads); `gate.rs` `MS_PER_SECOND` (now `pub(crate)`)
Losers: `catalog.rs` `is_ident_char`; `fuzz.rs` `panic_message`; `sweep.rs` `MS_PER_SECOND` and the bare `1000.0`s in arena.rs (1) and fuzz.rs (3) — deleted / named
Callers updated: 7
Semantic conflicts: none. (TS had no function for fuzz's message, a template literal; match.ts's `messageOf` is agent's, so rule 3's "TS had both" does not apply.)
Unresolved: none
Lines: before 20, after 5

## Cluster: copies of the ai crate's and the engine's own exports
Winner: `jackioh_ai::SHADOW_BAN_IDS`, `jackioh_ai::RejectedAction`, `jackioh_engine::subsystems::AI_SKIPPED_ACTIONS` — owners
Losers: `agent.rs` `own_shadow_ban` (re-sorted `SHADOW_BAN`), `wasm` `constants()`'s sort, `arena.rs` `Rejected` and `SKIPPED_ACTIONS` — deleted
Callers updated: 7 (agent `own_info`; arena's random entrant now takes `own_info().shadow_ban`, the same list; wasm `constants`; arena `ArenaOutcome.rejected`, `accept`, `play_game`, `replacements_for`); agent's tests import `SHADOW_BAN` themselves
Semantic conflicts: none
Unresolved: arena.rs's referee (`play_game`, `replacements_for`, `ARENA_MAX_ACTIONS`) is a second `playMatch` beside `jackioh_ai::play_match`. Kept: the ai crate's `SeatController` has no external-agent seat and `MatchHooks` no controller override, so arena cannot call it without a change in `crates/ai` (the ai reconciler's, or part 37's). The private ai pieces it mirrors (`replacements_for`, `AI_MATCH.max_actions`, `message_of`) are not exported.
Lines: before 31, after 6

## Decision: seams fixed to the owners' shapes
`sweep.rs` passes `&SweepOptions` to `sweep_card`/`sweep_at_risk` (2 calls, E0308); `gate.rs`'s `GREEDY_PROBE_ACTIONS` is `usize` (`MatchConfig.max_actions`, E0308); `wasm` `validator("validateTrio")` calls the validator's `validate_trio` (was `validate_loadout`); `checkDeckDraft` reads `nameMaxLength` as `DeckDraftInput.name_max_length`'s `usize` (the `Value` hedge goes); `sweep.rs`'s unused `Difficulty` import dropped. Checked against the owners and unchanged (all match): `FoldArgs` camelCase serde (the engine has it) and `replay` printing `{hash, errors}` only, no `browserHash`; `AiOptions`, `Decision`, `SearchBudget`, `AiDeckOptions` (`Default`, `banned: Option<Vec<String>>`, `mana_cap: Option<i32>`); `SweepResult.stats` (flatten), `SweepOptions<'a>`, `MatchHooks<'a>`, `evaluate`'s four arguments, `AI_GATE` indexed by `Matchup`; `main.rs`'s dispatch against every module's `Args`/`run`. `DeckDraftInput` holds closures, so `checkDeckDraft` crosses as `{name, cards, deckable, portrait?, portraitKnown?, nameMaxLength}` and the binding builds them (spec-gaps).
Lines: 6 changed

## Not a collision
`catalog::newest_version` (catalog-version.mjs) and `stats::newest_patch` (stats.ts): TS had both; each has its own test and message (rule 3). `tools/src/golden.rs` and `engine/tests/golden.rs`: SURFACE §13 mandates both, and neither can call the other (a bin crate, an integration test). `cards/tests/cross/registry.rs`'s private `naming` copy: out of scope, and a cards test cannot depend on the tools binary (part 37). `name_of` in trace.rs and sweep.rs, `describe` in catalog.rs and trace.rs, `play_game` in fuzz.rs and arena.rs: different TS functions, different jobs.

# engine src

## Cluster: the sink's "events of this action dispatched" (`EngineSink`)
Winner: `crates/engine/src/triggers.rs` `Frontier`/`FrontierSlot` as `EngineSink.frontier` (part 3.3) — score 10 (§10.3's frontier, which settle actually uses), 0 options, 1 lifetime parameter — `new` sets its own, `reborrow` shares the parent's
Losers: part 3.1's `EngineSink.dispatched: Option<usize>` (combat.rs) — never added; part 6.1's `SettleSink { sink, dispatched }` struct (effects/combat.rs) — deleted (`SettleSink` is triggers' alias of `EngineSink`)
Callers updated: 4 (combat.rs `withhold_from_frontier` → `triggers::{dispatched, set_dispatched}`, combat.rs's interposer mark and ai_policy's playout → `mark_dispatched(sink, &[GameEvent])`, effects/combat.rs `settle_before_playout` on `&mut ctx.sink`)
Semantic conflicts: `owed_behind` (part 3.1, TS `DrainSink.owedBehind`, R117) is not part of the collision (it is work's drain bookkeeping, not dispatch) → added as its own field, cloned by `reborrow` as 3.1 decided; `converting`/`dry_running` copied down by `reborrow` (paired increments make the copy exact, part 3.2's report)
Unresolved: TS's `makeContext` built a fresh object, so a context carried no `owedBehind`; Rust's contexts (built by `reborrow`) carry a clone. Only a drain nested in a context that a resumed item opened can tell.
Lines: with the next two decisions, commit 85aeec0: +73 −77

## Decision: `script::TriggerWhen` takes `&mut EffectContext`
Winner: part 8.3's report (Classic+ #74's `when` writes its own card while it declines) — `TriggerWhen` and `TriggerDef::with_when` take `&mut EffectContext<'_>`
Losers: part 1's `&EffectContext` alias — changed (part 1's frozen file, recorded)
Callers updated: 1 (traps.rs `fire_trap` builds `let mut ctx` and passes `&mut ctx`); a `with_when` closure that only reads compiles either way
Semantic conflicts: none
Unresolved: none
Lines: +2

## Decision: refusals are `Result<(), EngineError>` (SURFACE §4.4.9)
Winner: `Result<(), EngineError>` with TS's text at the owner
Losers: `restrictions::attack_restriction -> Option<&'static str>`, `zones::why_cannot_carry -> Option<&'static str>`, `setup::why_mulligan_refused -> Option<String>` — changed at the owner
Callers updated: 2 (`zones::carrier_zones_for` `.is_ok()`); combat.rs, play_choices.rs and reduce.rs already read them as `Result`
Semantic conflicts: none
Unresolved: none
Lines: ±0

## Cluster: option-argument structs and enums (`zones`, `prompts`, `draw`, `card_scope`, `work`, `catalog`, `heal`)
Winner: each owner's name and fields — zones (2.1) `MoveToZoneOptions { position: Option<LibraryPosition>, keep_state }`, `LibraryPosition::{Top, Bottom, At(i32)}`, `PlaceOnFieldOptions { stack }`, `RemoveFromFieldOptions`, `OffFieldZone`, `GraveyardRedirect::{Exile, LibraryBottom}`; prompts (3.2) `HookResumableOptions`, `OpenPromptArgs`, `ResumeAtArgs`, `HookInstance`; draw (4.2) `AddToHandOutcome`; card_scope (6.1) `cards_in_card_scope(ctx, scope, Option<&CardScopeOptions>)`; work (3.1) `WorkPlan` (fields written out, `WorkPlan::new(resume, owner)`); catalog (2.2) `GlitchOdds = GameState`; heal (6.1) `HealArgs::{Amount, ToFull, UpTo}`; damage (3.1) `DamageArgs`/`DamageFlags`; resolve (3.2) `HookOptions`, `CastOptions`, `CastAfterward`
Losers: callers' `MoveOptions` (13), `MovePosition` (2), `PlaceOptions` (4), `MoveToZonePosition::Index` (1), `RunHookOptions` (3), `AddToHandResult` (2), `CardsInCardScopeOptions` (1) + `Default::default()` for the `Option` (3), `WorkPlan { resume, owner }` (4), `GlitchOdds { system_plays }` (3), `HealArgs { amount, to_full, up_to }` (1), `GraveyardRedirect` from JSON (1)
Callers updated: 38
Semantic conflicts: none
Unresolved: none
Lines: commit f92b3d2 (with the call shapes below): +107 −118

## Decision: effect argument structs are data (`Deserialize`, camelCase)
Winner: SURFACE §6.6 — derived at the owner: `ResumeAtArgs`, `CastNewArgs`/`CastNewDef` (untagged, `Read` `#[serde(skip)]`), `CastRandomArgs`/`CastRandomQuery`/`CastRandomCount` (likewise), `CastAfterward`, `CastOptions`, `OpenPromptArgs`, `HookResumableOptions`, `PlaceOnFieldOptions`. Every `f(json_as(…))` call in `crates/` now names a `Deserialize` argument (checked by script).
Losers: none
Callers updated: 0
Semantic conflicts: none
Unresolved: `CastEachArgs`, `DrawWhileArgs`, `ForEachCardArgs`, `WithKillCreditArgs`, `CallToChaosArgs` hold required closures or fn tables and stay `Clone` only: built by struct literal.
Lines: commits 2783d0e and 37482eb: +27 −10 (the rest inside f92b3d2)

## Decision: closure fields of effect arguments (orchestrator's item 1)
Winner: the owners' shapes (effects/*.rs, parts 6–7), unchanged: `ForEachCardArgs.cards: Arc<dyn Fn(&mut EffectContext) -> Vec<String>>` (card ids) and `.each: Arc<dyn Fn(&str) -> Effect>`; `CastEachArgs.cards: Fn(&mut EffectContext) -> Vec<String>`; `DrawWhileArgs.more: Fn(&mut EffectContext) -> bool` (+ `player: Option<PlayerSpec>`); `DiscoverFromCatalogArgs.query_fn: Fn(&mut EffectContext) -> CatalogQueryArgs`; `CastNewDef::Read: Fn(&mut EffectContext) -> Option<CastDef>`; `CastRandomQuery::Read`/`CastRandomCount::Read: Fn(&mut EffectContext) -> …`; `KillCreditPairs: Fn(&mut EffectContext, &CardInstance)`. The two filters stay `&`: `ChooseTargetWhereArgs.where_: Fn(&EffectContext, Option<&CardInstance>) -> bool` and `ChooseFromHandArgs.where_: Fn(&EffectContext, &CardInstance) -> bool` (no TS caller mutates or draws in a `where`; #32 Felinor Feelings, the one user, reads)
Losers: card files writing `|ctx: &EffectContext| …` for `cards`/`more`/`query_fn`, or `CardInstance`s for `cards`, and `&mut` for a `where_` — the cards reconciler updates them (Mid Runner, Classic #22, shuffles in `cards`: `&mut` is what it needs)
Callers updated: 1 in scope (hero_power's Brainstorm, now `effects::each::for_each_card`)
Semantic conflicts: none
Unresolved: none
Lines: 0

## Decision: the prelude's `register_catalog` (orchestrator's item 2)
Winner: the testkit's `register_catalog` owns the name in a card test (SURFACE §8)
Losers: `prelude.rs`'s `pub use crate::catalog::*` — replaced by an explicit list of catalog's names without `register_catalog` (cards' `register_all` names it by path)
Callers updated: 0
Semantic conflicts: none
Unresolved: the six prelude/testkit name pairs part 1 recorded (`draw`, `add_to_hand`, `end_turn`, `gain_mana`, `refresh_mana`, `lose_health`: effect verb vs engine function, both TS's) stay; so do the module names `damage`, `draw`, `combat`, `mana`, `plague`, `kill_credit`, `fuse` (under `effects::` in the prelude, at the root in the testkit). Each is ambiguous only where a card test names it bare.
Lines: +6

## Decision: `subsystems::audit`'s argument struct (orchestrator's item 3)
Winner: `AuditArgs` (part 8.1, the owner)
Losers: part 25.1's `AuditTargetsArgs` — for the engine-tests reconciler to rename at the call
Callers updated: 0 in scope
Semantic conflicts: none
Unresolved: none
Lines: 0

## Cluster: TS `scripts.scriptOf(instance)` / `flagsOf` / `textsOf`
Winner: `crates/engine/src/scripts.rs` `script_of(state, &card) -> Script` (`ScriptKey`), `flags_of`, `texts_of`, `TextFlags` (part 2.1) — score 10 (§6.3 Vanilla guard, R102's texts), 0 options
Losers: `running_script` ×9, `script_of_card` ×9, `face_script` ×6, `with_face` ×3, private `flags_of` ×3, `flags_of_card` ×4, damage.rs's `texts_of`/`CardText` — deleted
Callers updated: 56
Semantic conflicts: none (every copy was the same Vanilla-guarded face pick)
Unresolved: none
Lines: this and the next four clusters are commit dd94430: 35 files, +114 −949 (net −835)

## Cluster: R102's ingredient records
Winner: `scripts.rs` `IngredientRecord`, `ingredients_of`, `ingredient_paid(&CardInstance, &[usize])`, `as_ingredient`, `ingredient_record` (part 2.1)
Losers: subsystems/fuse.rs's private copies (paths as `i64`), damage.rs's `IngredientEntry`/`records_from` — deleted
Callers updated: 7
Semantic conflicts: path index `i64` (fuse) vs `usize` (scripts, work) → `usize`
Unresolved: none
Lines: in dd94430

## Cluster: fused-id parse (R179, R468, R469)
Winner: `catalog.rs` `fused_id_specs(Option<&GameState>, id)`, `fused_id_parts`, `self_def_ids`, `is_digest_id`, and `fused_head_len`/`copy_spec` (now `pub`/`pub(crate)`) (part 2.2, TS `catalog.fusedIdSpecs`)
Losers: subsystems/fuse.rs `fused_id_specs_in`, `fused_head_len`, `is_digest_id`, `copy_spec`; params.rs `fused_id_parts`, `fused_head_len`, `FUSED_DIGEST_MARK`, `RADIANT_INGREDIENT_MARK`; scripts.rs `fused_head_len`; perfect_hand.rs and glitch.rs `self_def_ids` — deleted
Callers updated: 14
Semantic conflicts: none (identical parses; catalog's reads a digest's list through `find_def(Some(state))`, transient defs first)
Unresolved: none
Lines: in dd94430

## Cluster: part paths and remembered keys (R102, R77)
Winner: `work.rs` `part_path_of -> Option<Vec<usize>>`, `reroot_remembered(memory, usize)`, `memory_of_part`, `PART_KEY` (part 3.1)
Losers: subsystems/fuse.rs's copies (`i64`), params.rs's `part_path_of -> Vec<Value>` and `PART_KEY` — deleted
Callers updated: 6
Semantic conflicts: TS `partPathOf` kept any number and `params.param` threw on an unusable one; `work`'s drops non-`u64` entries, so a malformed path is skipped rather than thrown on
Unresolved: TS's `findIndex` = -1 for a kept card that is no ingredient rerooted to `key@-1`; Rust skips the reroot
Lines: in dd94430

## Cluster: composed-list runners and declaration readers
Winner: `resolve.rs` `lazy_part`, `apply_effects` (part 3.2); `play_choices.rs` `active_target_decls`, `stored_declaration_slices` (part 4.2)
Losers: subsystems/fuse.rs and subsystems/call_to_chaos_plus.rs `lazy_part`/`apply_effects`; fuse.rs's two play_choices copies — deleted
Callers updated: 5
Semantic conflicts: `stored_declaration_slices`: fuse's copy read `as_u64`, play_choices' `as_f64().max(0)` → play_choices'
Unresolved: none
Lines: in dd94430

## Cluster: board readers, stay marks, paused/owe/settle
Winner: `zones.rs` (part 2.1) `acts_on_field`, `slot_of`, `row_size`, `card_at`, `carried_at`, `is_carrier`, `is_reserved`, `reserve_zone`, `release_zone`, `OffFieldZone`; `stays.rs` (part 2.1) `exit_mark`, `event_mark`, `left_field_after`; `work.rs` (3.1) `paused`, `owe`; `triggers.rs` (3.3) `settle`
Losers: private copies in replacements, traps, targeting, quests, state_check, modifiers, carriers, resolve, prompts, triggers, turn, testkit/scenario (`card_at`, `PileZone`) — deleted
Callers updated: 60
Semantic conflicts: replacements' `acts_on_field` checked "top of its zone", zones' "on the field, not buried": the same set
Unresolved: none
Lines: commit 9db75e7: +54 −251 (the testkit's `card_at`/`PileZone` in ebc028a)

## Cluster: smaller single-owner copies
Winner: `tuning::{tuned_count, numbered_sum}`, `enchantments::{add_enchantment, united_enchantments}`, `times_played::count_play`, `kill_credit::credited_killer_id(&source, &victim)`, `setup::mulligan_prompt_for`, `effects::each::for_each_card`, `effects::targets::player_of`, `catalog::query` (the testkit's filler deck), `catalog::find_def` (the testkit's `def_of`)
Losers: numbers.rs, shuffle_random.rs, summon.rs, play_steps.rs, damage.rs, testkit/invariants.rs, subsystems/hero_power.rs copies; `player_or_self` ×7; `owned` ×9 (a hedge over return kinds now known); the testkit's `is_token`/`index_rank`/`set_rank`/`catalog_order`
Callers updated: 50
Semantic conflicts: shuffle_random/summon compared enchantments with `==`, enchantments.rs with its own `same_enchantment` → enchantments.rs's (TS's module)
Unresolved: none
Lines: commit 8cc91d0: +58 −268; the testkit's in ebc028a (+15 −111, with `card_mut`)

## Cluster: `CardScope.side`
Winner: `effects::targets::ScopeSide` and `sides_of` (part 7.1; TS `BoardScope["side"]`, which `cardScope.ts` passed to `targets.sidesOf`)
Losers: effects/card_scope.rs `CardScopeSide` and its private `sides_of` — deleted
Callers updated: 1
Semantic conflicts: none (same three literals, same walk order)
Unresolved: card files that name `CardScopeSide` (none found) would rename
Lines: commit 2d1370f: +2 −24

## Decision: testkit methods the card files call
Winner: `Scenario::card_mut(impl Into<CardRef>) -> &mut CardInstance` added (resolved as `card()`); every other method the cards call (`state_mut`, `expect_refused_with`, `last_events`, `view`, `backrow`, `pile`, `unit`, …) already existed
Losers: 17 private `fn card_mut(s, id)` in `crates/cards/**` tests — for the cards reconciler to delete
Callers updated: 0 in scope
Semantic conflicts: none
Unresolved: none
Lines: in ebc028a

## Decision: `catalog::def_of`'s shape
Winner: part 2.2's `def_of(Option<&GameState>, &str) -> &CardDef` (panics with TS's text) and `find_def(Option<&GameState>, &str) -> Option<&CardDef>`
Losers: callers passing a bare `&GameState` — 16 in engine src updated; in `crates/cards` 137 calls already pass `Some(..)`, 15 pass `&…`, 1 `state`, 1 `g` — for the cards reconciler
Callers updated: 16
Semantic conflicts: none
Unresolved: none
Lines: 0

## Decision: other call shapes settled at the owner
Winner: `triggers::settle(sink, SettleOptions)`, `combat::switch_position(sink, &unit, SwitchPositionOptions)`, `ai_policy::play_out_turn(sink, player, PolicyOptions)`, `mana::effective_cost(state, card, CostOptions)`, `damage::pierces(state, Option<&CardInstance>, Option<&DamageFlags>)`, `catalog::excluding_def_id/self_def_ids/fused_id_parts(Option<&GameState>, …)`, `work::owe(sink, impl Into<OweItem>)` (one item), `prompts::resume_self(ctx, step, IndexMap)`, `resolve::make_context(&mut EngineSink, …)` (it reborrows), `resolve::cast_card(sink, &CardInstance, CastOptions)`, `prompts::{apply_resumable, run_resumable_list}(ctx, &plan, Vec<Effect>, Option<PausedStep>)`, `work::park_work(sink, &plan, &PausedStep)`, `targets::{cards_in_scope, adjacent_to}(ctx, …, &BoardScope)`, `play_steps::run_play_steps(sink, player, &PlayAction)`, `activate::activate_ability(sink, player, &ActivateAction)`, `catalog::query(&CatalogQueryArgs) -> Vec<&'static CardDef>`
Losers: the callers' guesses (≈60 sites, all updated)
Callers updated: ≈60
Semantic conflicts: none
Unresolved: the movers' `&mut CardInstance` (zones, draw, brittle_count) and `pick_generated(&[&CardDef])` are the owners' too; their ~30 call sites are left to Wave 3 as local E0308s
Lines: ±0

## Decision: test seams the engine-tests reconciler asked about (`mock_*`, `register_work_handler`)
Winner: none added. TS's `vi.mock` of brittle/animated/cleanup (part 26.6, turn_wiring.rs) and `registerWorkHandler` (part 25.3, pauses.rs) were test-framework or registration hooks SURFACE §6.6 removed; adding thread-local hooks to engine paths is a feature neither the Rust nor SURFACE has (reconciler: no new features)
Losers: the two test files' wished names — for the orchestrator: rewrite those tests against the real board, or drop them
Callers updated: 0
Semantic conflicts: none
Unresolved: yes, listed in spec-gaps.md (engine src)
Lines: 0

## Totals (engine src)
Lines: before 69,294, after 67,929 (`git diff --shortstat 85aeec0^ HEAD -- crates/engine/src`: 80 files, 513 insertions, 1,878 deletions). Errors (type-check phase): 187 → 45, all E0308 and local; see `.fullsend/damage/engine-src.md`.

# server

## Cluster: Socket
Winner: `crates/server/src/actor/ws_server.rs`'s `Socket` + `SocketFrame` — SURFACE §11.2 names `actor::ws_server::Socket` (rule 1)
Losers: `actor/contracts.rs`'s `Socket`, `SocketFrame`, `SocketSendError`, `NEXT_SOCKET_ID` — deleted (contracts.rs keeps `SocketHandlers`)
Callers updated: 2 (registry.rs's import; match_actor.rs: `send` answers nothing, `previous != socket` for `same`, and the close handler tells the seat's own socket by `is_open` instead of an id, so no handle cycle)
Semantic conflicts: none
Unresolved: none
Lines: before 152 (contracts' transport half), after 0

## Cluster: handlers' receiver (SURFACE §11.2 hole)
Winner: `&Arc<App>` for every route handler, `app::Handler`, `h!`, `api::http::dispatch`/`run_route` — `Registry::start` needs the `Arc` its actor keeps; three handlers already took it
Losers: `&App` handlers; `Registry::bind`/`app()` and its `OnceLock<Weak<App>>` (19.2), the `series.rs` "registry not bound" branch and `run_sweeper`'s bind — deleted
Callers updated: 34 handler signatures in 13 files; series.rs's `ensure_series_game`, `start_series_game`, `resume_series`, `write_transition`, `player_transition`, `sweep_one`, `sweep_series` take `&Arc<App>`
Semantic conflicts: none
Unresolved: recorded in spec-gaps.md (SURFACE §11.2 writes `&App`)
Lines: −22 net (bind/app and the branch)

## Cluster: the server clock
Winner: `crates/server/src/app.rs`'s `now_ms` — score 10 − 0 branches (epoch anchored once, advanced by `tokio::time::Instant`, so a paused test clock moves it); more callers
Losers: `actor/clock.rs`'s `now_ms` (signed-offset branch), the private copies in `api/queue.rs`, `api/results.rs`, `api/rematch.rs`, `cli/mint_code.rs` — deleted
Callers updated: registry.rs, match_actor.rs, series.rs, clock.rs, the four files above, `tests/actor/clock.rs`, `tests/actor/match_actor.rs`
Semantic conflicts: clock.server/time.server → `app::now_ms`
Unresolved: the fake store's default clock stays the wall clock (`fake::system_now`, TS `createMemoryStore()`'s `Date.now()`); TS's test deps handed it `timers.now`. Tests that pause tokio and read store stamps may need `FakeData.now = Arc::new(app::now_ms)` (Wave 3)
Lines: −62

## Cluster: StoreError
Winner: `crates/server/src/db/store.rs` — `Duplicate(String)`, `Db(sqlx::Error)`, `Other(String)` (the owner, 20.4)
Losers: `Duplicate { match_id }` (fake.rs), `DuplicateResult` (results.rs) — rewritten to the owner's variant
Callers updated: 2 (fake.rs's `duplicate_result`, results.rs's `insert_error`); the tests' `StoreError::Duplicate { .. }` patterns already match a tuple variant
Semantic conflicts: none
Unresolved: none
Lines: 0

## Cluster: store method shapes
Winner: `crates/server/src/db/store.rs`'s 88 `Tx` methods (names and argument types; every caller name already matched)
Losers: pg.rs's spread `profiles_create(user_id, email, rating, at, display_name)` and `player_stats_list_public(search, limit, offset)`; pg/fake `i32`/`usize` caps, counts and positions (`decks_upsert`, `trios_upsert`, `tutorial_merge`, `last_boards_sample_others`, `ranked_note_peak_jlorious`, `codes_count_*`, `tickets_count_open`), `IndexMap<QueueMode, _>` for `PerMode<i64>`; fake's `ListPublicOptions`/`CardCount` names; `fake::begin`/`commit` answering `Result`
Callers updated: pg.rs (9 fns, binds kept `int4` behind TS's `::int` casts), fake.rs (8 fns); api callers (registry, results, rematch, tutorial, collection, series_rules) converted to the store's `i64`
Semantic conflicts: store.ints → `i64` (store.rs's header); series rows `i64` (19.1's `usize` converts at the row); `null.optional` `x?: T | null` → `Option<Option<T>>` (absent_or_null), pg/fake/queue/series_rules read and write it so
Unresolved: none
Lines: ±0 (conversions)

## Cluster: TS tuples in the store
Winner: tuples `(Option<String>, Option<String>, Option<String>)` for `TrioSlots` and `(FrozenDeck, FrozenDeck, FrozenDeck)` for `FrozenTrio.decks` — SURFACE §4.3 (`[A, B]` → `(A, B)`, rule 1) and every caller
Losers: store.rs's `[T; 3]`
Callers updated: 0 (decks.rs, series_rules.rs, pg.rs, fake.rs already used `.0`/`.1`/`.2`)
Semantic conflicts: none (serde writes both as the same 3-array)
Unresolved: none
Lines: 2 changed

## Cluster: the empty in-memory store
Winner: `impl Default for FakeData` in `db/fake.rs` (TS `createMemoryStore()`: wall clock, default redemption, no launch grant), behind store.rs's `Db::fake()` — 3 lines, 0 options, most callers (6)
Losers: `fake::create_memory_store` + `MemoryStoreOptions` (no callers; two config options) — deleted
Callers updated: `app.rs`'s E2E store is now `create_e2e_store(E2eStoreOptions { catalog, now: app::now_ms })` (TS `createE2EStore({ catalog, now: timers.now })`: R111's launch grant needs the catalog); `tests/support/deps.rs` uses `Db::fake()` or `TestAppOptions.db`
Semantic conflicts: none
Unresolved: none
Lines: −19

## Cluster: mint_invite_code
Winner: `api/codes.rs`'s `mint_invite_code(MintDeps { db, code_pepper }, MintInput { max_uses: Option<i32>, expires_at })` (the owner, 18.5's shape)
Losers: the guessed `(&Db, &Hashes, MintInviteCodeInput)` at `cli/mint_code.rs`, `tests/api/codes.rs`, `tests/store/mint_code.rs`
Callers updated: 4 (the CLI passes `env.code_pepper` and narrows its `i64` count; the tests pass their pepper)
Semantic conflicts: none
Unresolved: none
Lines: −3

## Cluster: owners' signatures at call sites (src)
Winner: each owner — `api::ranked::rate_ranked_game(&RankedGameInput)`, `api::series::{advance_series_in_tx(&SeriesGameResult), resume_series(Option<&SeriesRow>), start_series(NewSeriesInput by value)}`, `RatedReason::Series(SeriesEnd)` (no `From`), `api::http::rate_limit_address(raw)`, `jackioh_ai::AiDeckOptions.banned: Option<Vec<String>>` (the ai reconciler's decision), `jackioh_engine::validator::TRIO_DECKS`, `jackioh_engine::wire::emotes::is_portrait_id(&Value)`, `db::Ticket.portrait` three-state
Losers: the callers' guesses
Callers updated: results.rs (7), series.rs (1), queue.rs (3), rooms.rs (1), ws_server.rs (1), engine.rs (1), seed_accounts.rs (1), decks.rs (1), http.rs (2: `candidate` → `auth`, a local slip)
Semantic conflicts: none
Unresolved: none
Lines: ±0

## Cluster: gaps added to owners
Winner (added, shaped as the callers call them): `pub use store::*;` in `db/mod.rs` (SURFACE §11.2's `db::Db`, `db::Profile`); `Catalog.banned` (R164, 18.4's GAPS; `defs_json` public so `Catalog { banned, ..catalog }` builds); serde on `OpenedSeason`, `RankedGameInput`/`RankedSideInput`, `NewSeriesInput`/`NewSeriesSide`, `ModeChoiceInput`, `FrozenChoice` (TS's `mode`-tagged shapes), `fake::CollectionRow`/`LastBoardRow`; `jsonwebtoken` `rust_crypto`
Losers: none
Callers updated: 0
Semantic conflicts: none
Unresolved: none
Lines: +40

## Cluster: test support (`tests/support/{deps,engine,socket}.rs`)
Winner: the support files' own names and shapes, reshaped where every caller disagreed: `FakeSocket` methods take `&self` (28 sockets bound immutably, 0 `&mut`), `receive_json(Value)` (40 by value), `TestAppOptions.db` (a restart over a store; no reseed); kept: `add_user(app, user, email, verified)` (15 of 24 callers; the other 9 pass TS's default `true`), `call(.., body: Value)` (~20 of 37), `install_test_cards()` for TS `createFakeEngine` (the real engine always opens on the mulligans, so its `mulligan` option has nothing to switch), `TEST_CATALOG_VERSION`
Losers: `create_fake_engine`/`FakeEngineOptions`, `TestAppOptions.e2e`, `TEST_PATCH_VERSION`, results.rs's local `install_test_cards` wrapper
Callers updated: ~200 test call sites (the actor's sync methods un-awaited ×103, `&ClockView` ×26, `call` bodies ×17, `add_user` ×8, fake-engine ×7, `start_series` in a transaction ×4, `unwon_slots`/`first_unwon` given `&series.games` ×7, `SupabaseAuth::new` ×2, `E2E*` names, `PlayerStatsListOptions`, `Arc` `on_call` ×3, `tables.game_records`, `Some(reason)` for `grant_entire_catalog` ×6, `with_cors` per request)
Semantic conflicts: none
Unresolved: `TestAppOptions.e2e` — every test app runs `E2E=1` (`test_env`), so rooms.rs's non-E2E arm (R143's seed ignored outside E2E) is the same app; `TEST_PATCH_VERSION` — the Rust test app rates in the compiled-in version's season, so season_start.rs's v0.1/v0.2 expectations need a seam (both listed in spec-gaps.md for part 35)
Lines: tests +330 −301

## Cluster: copied helpers (fullsend rule 5)
Winner: one home each — `ApiError::{new, with_details, internal}`, `http::{bad_request, rate_limited, ok, lock}` and `From<StoreError> for ApiError` (http.rs owns `ApiError`); `collection::{caller_profile, owned_in}` (TS `callerProfile` is collection.ts's); `queue::{new_uuid, new_seed, json_list}`; `QueueMode::as_str`; `PlayerId::opponent`; `env::{js_number, quoted}`; `mint_code::is_integer`; `card_stats::literal`; `auth::is_uuid` (fewer lines); `Value::{as_object, is_object}`; `fake::{count_number, to_public_player_summary}` (pg.rs called its copies private); `StoreError::from` for `fail`; `pg::assert_postgres_url` (store.ts's); `codes::pad_to` (http.rs's had no caller); `catalog::load_patch_versions`
Losers: `api_error` ×8, `bad_request` ×4, `internal` ×4, `store_failure` ×3, `hide_internal` ×2, `caller_profile` ×8, `rate_limited`, `ok`, `new_uuid` ×2, `new_seed` ×2, `lock` ×4, `json_list`, `mode_name` ×2, `other` ×2, `js_number` ×3, `is_integer`, `json_text` ×3, `quoted`, `literal`, `is_uuid`, `is_record` ×2, `count_number`, `to_public_player_summary`, `fail` ×2, `assert_postgres_url`, `pad_to` + `sleep`, `load_patch_versions`, `owned_in` — deleted
Callers updated: ~250 call sites in 27 files
Semantic conflicts: error.style → `dispatch` is the one place a non-API error is logged `handler.threw` and answered 500 "something went wrong" (`store_failure`, `hide_internal` and tutorial's `internal(path, error)` did it a second time, so TS's one log line was two)
Unresolved: none
Lines: src +315 −891 for this cluster

# cards

## Cluster: TS `stepParam(s.card(x), …)` and the live card in a test (`card_mut`)
Winner: testkit `Scenario::card_mut(impl Into<CardRef>) -> &mut CardInstance` (added by the engine reconciler) with engine `params::step_param`, called inline as TS wrote it — score 10 (one behaviour: write through the live card), 0 options, 0 type parameters, 0 branches at the call
Losers: 102 `step` + 2 `step_id` + 4 `step_param_of` + `step_param_on` + 3 `step_card_param` + `step_live_param` + `step_box`/`step_flag`/`step_fungus` (Classic, Classic+, cross `preview.rs`); 17 `card_mut(s, id)` (13 card tests, cross `combat_windows`, `deaths_and_reborn`, `fused_hooks`, `hand_returns`); 9 `()`-returning `make_radiant`/`flag_radiant`/`set_radiant` (Core #33–#35, #51.1, #52, #57, #59, #97, #99) — deleted (each scored the same behaviour plus a lookup the testkit already does)
Callers updated: 321 step calls, 32 card_mut calls, 25 radiant writes
Semantic conflicts: none (every copy resolved the reference as `s.card()` does, then wrote the live instance)
Unresolved: TS's own `makeRadiant` (Core #16, #17, returning the scenario) stays, as TS had it per file. 98 inline `find_instance_mut(s.state_mut(), &id)` writes are no helper and stay (equal to `s.card_mut(&id)`).
Lines: commits 5579d4a (+332 −1171, with the glow cluster) and part of 9710602

## Cluster: `_glow.ts` in card tests
Winner: `testkit::glow::{glows, hand_glows, backrow_glows}` (part 5, the port of `_glow.ts`)
Losers: Core #38's `glows`/`hand_glows`, Core #41's `glows`/`backrow_glows` (private copies) — deleted
Callers updated: 6 (TS's default viewer `p1` passed explicitly)
Semantic conflicts: none
Unresolved: none. `tests/cross/condition_active.rs` keeps its own `glows`/`hand_glows`: TS's condition-active.test.ts defined its own too (rule 3's exception).
Lines: in 5579d4a

## Cluster: "register the shipped cards, then `scenario()`" (TS's vitest globalSetup)
Winner: one per test binary — `crate::scenario` (`src/lib.rs`, `#[cfg(test)]`) for the card files' tests, `cross::scenario` (`tests/cross/mod.rs`) for the cross tests: `register_all(); testkit::scenario(opts)`
Losers: 72 identical local `fn scenario(…)` wrappers in card tests, 9 in cross files, 7 cross `fn game(setup)` wrappers (the same body under another name) — deleted; each module imports the winner
Callers updated: 88 modules (71 `game(` calls renamed to `scenario(`)
Semantic conflicts: none (identical bodies)
Unresolved: none. Tests that call `crate::register_all()` before `scenario` still do (idempotent, left alone).
Lines: 70ee480 (+95 −421), a31f95d (+78 −106)

## Cluster: small JSON test helpers TS never had as functions (`js`, `matches_object`, `merged`, `unit_or_blank`)
Winner: `crate::js` (the `?Sized` signature, covering every caller), `crate::matches_object`, `crate::merged`, `crate::unit_or_blank` (`src/lib.rs`, `#[cfg(test)]`)
Losers: 141 `js`, 36 `matches_object`, 30 `merged`, 10 `unit_or_blank` in card-file tests — deleted
Callers updated: 217 modules import them
Semantic conflicts: `matches_object` on arrays: 5 copies compared with `==`, 31 item by item → item by item (Jest's `toMatchObject`, which the TS tests ran under)
Unresolved: `spread` (22 copies: TS's `{ ...a, ...b }` again, with `(Value, &Value)` and `(Value, Value)` shapes) is the same concept as `merged`; left as is (two call shapes, more files changed than it saves). `must` is NOT a collision: TS's test files had 32 `must`s of their own (rule 3's exception) — the 25 a first pass folded were put back.
Lines: 383f5f5 (+198 −932), 67781fa (+61 −328)

## Cluster: "owned whatever the reader returns" hedges
Winner: the owners' return types (`query::zone_cards`, `subsystems::audit_targets` answer `Vec<CardInstance>`)
Losers: `owned<C: Borrow<CardInstance>>` in Core #51 and C+ #44 — deleted (as the engine reconciler deleted its 9)
Callers updated: 2
Semantic conflicts: none
Unresolved: none
Lines: in 9710602

## Decision: call shapes the engine reconciler settled, applied in `crates/cards/**`
Winner: the owners' shapes — `def_of(Option<&GameState>, …)`, `fused_id_parts(Option<&GameState>, …)`, `scripts_for(&GameState, …)`; `TriggerWhen`, `ForEachCardArgs.cards`, `DrawWhileArgs.more`, `CastNewDef::Read`, `KillCreditPairs` on `&mut EffectContext`; testkit `unit()`/`backrow()` answering owned copies and `expect_refused*` closures returning `&mut Scenario`; `CastRandomArgs { query, count, radiant, target_enemies, afterward }`; `AuditArgs`, `SweepReader`, `CostOptions` by value; `roll_chaos_effects(rng, radiant, Option<&[ChaosEffectDef]>)`; `score_def(…, &ScorerOptions, Option<&mut GameState>)` with TS's defaults; `answer_prompt(sink, &AnswerInput)`; `EngineSink::new`; the movers' `&mut CardInstance`
Losers: the callers' guesses
Callers updated: ≈160 (see `.fullsend/damage/cards.md`, Seams)
Semantic conflicts: none
Unresolved: none
Lines: 9710602 (+235 −411, with the shim and hedge clusters)

## Decision: `query.rs` (part 9) is the owner of the card-facing pool surface
Winner: `query(&CardQuery) -> Vec<&'static CardDef>`, `pool(own_id, &CardQuery) -> Vec<&'static CardDef>`, `TRAP_TYPES`, `query_cost`, and `catalog` as a value with methods `query`/`pool`/`cost` and field `trap_types` (TS's `catalog` object). `CardQuery = CatalogQueryArgs` (engine) already derives `Serialize`/`Deserialize`.
Losers: Transmogulate's by-value `pool(ID, json_as(…))` and `Vec<CardDef>` pools; cross `query.rs`'s `catalog::query(`/`catalog::TRAP_TYPES` module paths
Callers updated: 23
Semantic conflicts: none
Unresolved: card files that call `jackioh_engine::catalog::query` directly (8) are not a collision — TS had `catalog.query` in the engine and the wrapper in cards; same function underneath.
Lines: in 9710602

## Decision: the registry and `ky_test_bank.rs`
Winner: build.rs (part 1) — 318 card files ↔ 318 catalog ids, no duplicate `ID`; `ky_test_bank.rs` (part 15) keeps its shape, `KY_TEST_BANK: LazyLock<Vec<KyTestProblem>>` over the engine's `subsystems::ky_test::KyTestProblem` (no second problem type), handed to `subsystems::ky_test_script(&[KyTestProblem])` as `&KY_TEST_BANK` (deref to a slice)
Losers: none (Core #37/#38, deleted by part 14.2's rebase, were restored by the orchestrator in 9684288)
Callers updated: 0
Semantic conflicts: none
Unresolved: none
Lines: 0

## Unresolved (cards)
- naming.ts is written twice: `crates/tools/src/patches.rs` (the owner, port-map) and `crates/cards/tests/cross/registry.rs`'s private `mod naming` (part 22.3), which the cards test binary needs and cannot import from the tools binary. Fewer files changed: left as is. The clean fix is moving the `naming` tests into `patches.rs`'s `#[cfg(test)]` and deleting the cards copy — outside this scope (tools crate).
- Classic #28's `recalled_on(card, key)` is a private copy of `query::recalled` for a pure-read replacement `when`, because the engine's `recalled` takes `&EffectContext` while TS's took `Pick<EffectContext, "self" | "data">`. Two lines; left. If the engine owner widens `recalled` to TS's shape, the copy goes.

## Totals (cards)
Lines: before 150,207, after 147,837 (`git diff --shortstat 9684288 HEAD -- crates/cards`: 274 files, 999 insertions, 3,369 deletions). Errors: shadow build 183 → real build 0 (1 warning); clippy 273 warnings for Wave 3. See `.fullsend/damage/cards.md`.

# engine green

Part 32 step 1 (the engine library compiles). `cargo check -p jackioh-engine --lib --features testkit`:
45 errors (all E0308, part 31's leftovers) → 0, and 0 after the borrow check (no E0499/E0502/E0505/E0507
appeared); `cargo clippy --lib -D warnings` (with and without `testkit`, and with `ts`): 286 → 0; the
lib's unit tests 146/146 (142 before the seams' 4).

## Decision: the movers' callers hand over a mutable copy
Winner: the owners' `&mut CardInstance` (zones' `move_to_zone`, `place_on_field`, `remove_from_any_zone`,
`cease_to_exist`, `replace_in_zone`; draw's `add_to_hand`, `shuffle_into_library`) — the mover brings the
copy up to date with the live card (`live_or`) and leaves it as it landed, which is TS's live object
Losers: none (part 31 left the ~30 call sites as E0308s)
Callers updated: the 45 sites part 31 listed (`let mut card = …`, `&mut card.clone()`, `&mut card`)
Semantic conflicts: brittle_count's `give_brittle_count`/`gain_brittle_count`/`start_brittle_on_field`
take a `&mut CardInstance` beside `&GameState`, so a card that is in the state cannot be handed over
live: `effects::brittle` and `subsystems::twice_forward::reveal_self` run the verb on a copy of the live
card and write its `brittle` back by id (TS wrote through the live object; the same count results)
Unresolved: none
Lines: commit f0e8727, 25 files +62 −48

## Decision: catalog pools stay `&'static CardDef`
Winner: `catalog::query -> Vec<&'static CardDef>` and `pick_generated(&[&CardDef])` (part 31); the
callers stop cloning into owned pools (`effects::transform` ×2, `call_to_chaos_plus`, `perfect_hand`),
and `scorer::candidate_defs` now returns `Vec<&'static CardDef>` too
Losers: `owned_defs` in perfect_hand.rs and call_to_chaos_plus.rs — deleted
Callers updated: 5 in src; `tests/rules/{scorer,rulings_a}.rs` and Core #97's test read `candidate_defs()`
through `.iter()` field reads, which compile unchanged
Semantic conflicts: none
Unresolved: none
Lines: in f0e8727

## Decision: clippy's remaining lints
Winner: fixes, not allows, except one: `damage::DamageTarget` keeps `#[allow(clippy::large_enum_variant)]`
(its `Unit { instance: CardInstance }` is matched by value at ~90 sites in cards and tests; a `Box` buys
nothing there). `turn::Due::Delayed` boxes its effect (private); `CastNewDef::Read(CastDefReader)` and
`InterceptArgs.accepts: Option<&InterceptorFilter>` name their function types; the Cry's ask order is a
pair of `Asker` fn pointers (clippy's `if_same_then_else` read `a && b` / `b && a` as one block, but the
order is the point); `catalog::fused_id_specs` ranges over `head`, the value `start` began at (the loop
moves `start`, which never moved the range). `validator::LoadoutResult.errors` exports under `ts` as
`ts(as = "Option<Vec<LoadoutError>>", optional)` (ts-rs 12 refuses `optional` on a `Vec`; the TS type
`errors?: LoadoutError[]` is unchanged)
Losers: none
Callers updated: 0 outside src
Semantic conflicts: none
Unresolved: none
Lines: commit 63e6160, 70 files

## Decision: the two test seams (orchestrator's decision; spec-gaps.md "engine tests")
Winner: `crates/engine/src/testkit/seams.rs`, under `testkit` only and globbed into `testkit::*`:
(a) `register_work_handler(hook: &str, handler: WorkHandler) -> Option<WorkHandler>` (returns the one it
replaced, as TS's did), `unregister_work_handler(hook)`, `work_handler(hook)`, with
`WorkHandler = fn(&mut EngineSink<'_>, &WorkItem)`; a thread-local table that `work::run_work_item`'s
fallthrough reads before a card's own continuation (TS: the handler map before the default handler)
and `work::can_resume` reads after the built-in hooks. (b) `mock_brittle_tick`, `mock_animate_at_turn_start`,
`mock_return_at_cleanup`, each `impl Fn(&mut EngineSink<'_>, PlayerId) + 'static` (closures may capture
an `mpsc::Sender`, as turn_wiring.rs's do), held per thread as `Rc<StageDouble>`; `turn.rs`'s
`start_of_turn_brittle`, `start_of_turn_animate` and `cleanup` run a set double in place of the real
stage. `clear_seams()` puts every seam back. Thread-locals are `Cell`s (clippy.toml bans `RefCell`), as
the registry override's are.
Losers: part 31's "none added" (superseded by the orchestrator)
Callers updated: 0 (the engine-tests owner ports `pauses.rs`'s two tests back and builds turn_wiring.rs)
Semantic conflicts: a built-in hook (`@setup`, `play`, …) cannot be overridden by a test handler — the
seam is the `_` arm only, as the orchestrator specified; TS let a test replace any handler, and no TS
test did
Unresolved: none
Lines: seams.rs +239 (with its 4 tests), turn.rs +30 −3, work.rs +12, testkit/mod.rs +5
