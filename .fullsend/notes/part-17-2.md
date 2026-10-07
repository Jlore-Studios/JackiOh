# Slice: part 17 (the AI in Rust, generation 0), chunk 2 of 3 (#405, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
Every file was an empty placeholder on `staging` (no night-bot port, no `part-17-bot.md`); all 15 are
full ports of `packages/ai` at `91cc43c` (identical to `staging`'s copy). No `todo!`, `unimplemented!`
or `// TODO`.
- `crates/ai/src/types.rs` ← `types.ts`
- `crates/ai/src/simulate.rs` ← `simulate.ts`
- `crates/ai/src/sweep.rs` ← `sweep.ts`
- `crates/ai/tests/ai/support.rs` ← `test/_support.ts` (+ `test/setup.ts`'s `registerAll()` as `register_cards`)
- `crates/ai/tests/ai/{activate, answer_key, decide, deck, determinize_shown_cost, dev_run, evaluate_v020,
  evaluate, lethal, match_refusal, match_}.rs` ← the matching `test/*.test.ts`: one `mod` per `describe`,
  one `#[test]` per `it` (an `it` inside a TS `for` is one `#[test]` per value), R-ids leading the names.

## SURFACE (shapes this chunk fixed; other chunks call them)
- `types.rs`: `SearchBudget` (all `usize`, `Copy`, camelCase serde); `AiOptions<'a> { rng: Rng, budget:
  SearchBudget, should_stop: Option<&'a dyn Fn() -> bool> }` (SURFACE §9) with `AiOptions::new(rng)` (AI_BUDGET)
  and `AiOptions::with_budget(rng, budget)`; `DecisionReason` (`"draw-offer"` renamed; `as_str`, `Display`);
  `StoppedBy::{Exhausted, Budget, Clock}` (TS `SearchStats["stoppedBy"]`); `SearchStats { nodes, determinizations,
  lines, sim_errors: usize, stopped_by, score: f64 }`; `Decision { action, reason, line, stats }` (serde, for the
  WASM `ai_decide` answer); **`trait NodeCounter { used, limit, take(&self) -> bool, stopped_by,
  sim_error_tally(&self) -> Option<usize> (default None), set_sim_error_tally(&self, usize) (default no-op) }`**.
- `simulate.rs`: `LineStatus::{Open, Passed, Yielded, Over}`; `CountingNodeCounter<'a>` (one struct for a root
  counter and a sub-counter; inherent `sim_errors() -> usize`, `set_sim_errors(usize)`);
  `create_node_counter(limit: usize, should_stop: Option<&'a dyn Fn() -> bool>) -> CountingNodeCounter<'a>`;
  `create_sub_counter(parent: &'a dyn NodeCounter, limit: usize) -> CountingNodeCounter<'a>`;
  `line_status(&GameState, PlayerId, root_turn: i32)`; `simulate(&GameState, PlayerId, &ActionBody, &dyn NodeCounter)
  -> Option<Result<GameState, String>>` (`None` = TS `null`, `Some(Err(msg))` = `{ ok: false }`);
  `close_line(..) -> GameState`; `static_score(..) -> f64`; `terminal_score(.., &dyn NodeCounter) -> f64`;
  `search_signature(&GameState, PlayerId) -> String`.
- `sweep.rs`: `SweepFlag` (camelCase; `as_str`); `AiSweep`/`AI_SWEEP` (`tiers: &'static [Difficulty]`,
  `self_harm_delta: f64`, `at_risk_boost: i32`, the rest `i32`); `SweepStats` (counts `i32`, `eval_delta_sum: f64`);
  `SweepResult { #[serde(flatten)] stats, tier, flags, unswept }` with `Deref<Target = SweepStats>`;
  `SweepSuspect`, `SweepPass2`, `SweepVerdict` (`reason`/`watch: Option<String>`, serialised null);
  `SweepOptions<'a> { seeds: Option<i32>, now: Option<&'a dyn Fn() -> f64>, tier: Option<Difficulty> }: Default`
  (TS's anonymous options); `sweep_flags`, `half_flags`, `ban_flags(&str)`, `at_risk_ids(&[SweepResult],
  ban: &[(&str, &str)], watch: &[(&str, &str)])`, `pass2_keep_out(&[SweepResult], ban)`, `sweep_card(&str,
  &SweepOptions) -> SweepResult`, `sweep_at_risk(&str, at_risk: &[String], keep_out: &[String], &SweepOptions)`,
  `pass2_stats(&[SweepPass2], &str, Difficulty)`, `sweep_verdict(&[SweepResult], &[SweepPass2])`. TS's default
  arguments are passed explicitly (`SHADOW_BAN`, `SHADOW_WATCH`, `&[]`).
- `tests/ai/support.rs` (other chunks' tests use it): `register_cards()`; `AI`, `HUMAN`; `ai_pool`, `trap_pool`,
  `every_card(&GameState) -> Vec<&CardInstance>`, `card_by_id -> Option<&CardInstance>`, `on_field`, `in_graveyard`;
  `is_legal(&GameState, PlayerId, impl Borrow<ActionBody>)`; `act(&GameState, PlayerId, impl Borrow<ActionBody>)
  -> GameState` and `act_with_nonce(.., nonce: &str)` (TS's optional nonce); `clone`; `PuzzleRun { start, turn,
  end }`, `run_puzzle(name, setup: Value)` (AI_BUDGET) and `run_puzzle_with(name, setup, budget)`; `trace(&AiTurnResult)`;
  `all_out_attack(&GameState, PlayerId)`; `random_decks(seed)` and `random_decks_sized(seed, (i32, i32))`;
  `dealt_game(seed)`; `random_policy_states(seed, every: usize, max_actions: usize)`; `scenario`, `Scenario`,
  `ScenarioOptions` re-exported from the testkit (not wrapped, so a test globbing both `support::*` and
  `testkit::*` names one item). A test that builds a `scenario` itself calls `register_cards()` first.

## GAPS
### Names called in other modules (TS name snake_cased at its TS module's path; the shape assumed)
AI crate, chunks 17.1/17.3:
- **Every function that spends nodes takes `counter: &dyn NodeCounter`** and passes it on as is (`find_lethal`,
  `beam_search`, `score_line`, `simulate_reply`, `reply_score`, reply's `reduce_for`, `greedy_action`'s counter);
  a sub-counter is `create_sub_counter(&counter, n)`; `decide` reads `counter.sim_errors()` (a method) and
  `beam_counter.stopped_by()` with `NodeCounter` in scope.
- `config`: `AI_BUDGET`, `AI_GATE_BUDGET: SearchBudget`; `AI_EVAL`, `GREEDY_EVAL: EvalWeights` (`f64` fields
  `unspent_mana`, `drawn`, `enemy_health`, `threat_per_damage`, `answerable_threat`, `defense_attack_share`,
  `attack`, `position_grants`, `armor_point`, `animated_share`, `spell_damage`, `hand_cost_delta`, and
  `keyword.{taunt, immune_to_spells}`; tests copy the const and change a field: `let mut w = AI_EVAL;`);
  `AI_SEARCH.{max_auto_answers, lethal_quick_nodes}` (cast with `as usize`, so `i32` or `usize` both compile);
  `AI_MULLIGAN.keep_max_cost: i32`.
- `evaluate`: `evaluate(&GameState, PlayerId, NextSwing, &EvalWeights) -> f64` (TS's defaults written out:
  `NextSwing::Enemy`, `&AI_EVAL`); `NextSwing::{Enemy, Seat}`; `unit_worth(&GameState, &CardInstance, &EvalWeights)
  -> f64`; `face_threat(&GameState, PlayerId) -> i32`; `damage_past_taunts(&GameState, PlayerId, &[i32]) -> i32`.
- `deck`: `build_ai_deck(&mut Rng, i32, &AiDeckOptions) -> Vec<String>` (SURFACE §9; panics where TS threw);
  `AiDeckOptions: Default + Deserialize` (camelCase `banned`, `include`, `theme`, `manaCap`, `boost: { ids, by }`;
  built with `json_as` in `sweep.rs` and the tests, so the boost struct's name does not matter) with
  `theme: Option<Option<String>>` (`Some(None)` is TS's `theme: null`); `CostBucket` serialised as `"0-1"`, `"2"`,
  `"3"`, `"4+"` (the tests read it through serde); `cost_bucket(&CardDef) -> CostBucket`; `curve_targets(i32, i32)`
  and `AI_DECK.curve` serialisable as an object keyed by those literals; `AI_DECK.{curve_shift_per_mana,
  curve_tolerance, min_unit_share, theme_min_share}: f64`, `AI_DECK.{cost_slack, min_theme_size}`.
- `match_`: `MatchConfig { seed: String, decks: (Vec<String>, Vec<String>), handicaps: Option<PerPlayerOpt<Handicap>>,
  controllers: PerPlayer<SeatController>, max_actions: Option<_> }` (built as a literal; `max_actions` is set
  with `as _`); `SeatController::{Ai { budget: Option<SearchBudget> }, Greedy, Random}` (`PartialEq`);
  `MatchHooks<'a>: Default { after_action: Option<Box<dyn FnMut(&GameState, &GameState, PlayerId, &ActionBody) + 'a>>,
  time_decision: Option<Box<dyn FnMut(PlayerId, &mut dyn FnMut()) + 'a>> }` (TS's generic `<T>(seat, run: () => T)`
  as a `run` that stores the controller's answer itself); `play_match(&MatchConfig, &mut MatchHooks) -> MatchRecord`
  (SURFACE §9) with `result: Option<GameResult>`, `log: Vec<Action>`, `hash`, `rejected: Vec<{ seat, action, error }>`,
  `thrown: Vec<{ seat, message }>`, `fallbacks`, `decisions`, `nodes`; it must catch a panic in a controller call
  (TS's try/catch, `time_decision` included) as `thrown`; `play_ai_turn(&GameState, PlayerId, &mut AiOptions) ->
  AiTurnResult { state, actions: Vec<Action>, decisions: Vec<Decision> }`.
- **New, a test seam (match_.rs):** `MatchHooks.override_choice: Option<Box<dyn FnMut(&GameState, PlayerId,
  Option<ActionBody>) -> Option<ActionBody> + 'a>>`, called with each controller call's answer inside the
  controller's try (after `chooseFor`, before a null is replaced by `random_action`); its answer is the
  controller's. It stands in for `match-refusal.test.ts`'s `vi.mock` of `baselines.ts`; a panic in it is a
  controller throw.
- **New (lethal.rs):** `find_lethal_with_quick_nodes(dets, seat, counter, limit, quick_nodes: usize)`: `find_lethal`
  with the depth-first walk's share given, where TS's test set `AI_SEARCH.lethalQuickNodes` at run time
  (`find_lethal` is it with `AI_SEARCH.lethal_quick_nodes`). Used by `lethal.rs`'s "the depth-first walk alone" test.
- `lethal`: `find_lethal(&[GameState], PlayerId, &dyn NodeCounter, usize) -> Option<Vec<ActionBody>>`;
  `ready_gap(&GameState, PlayerId) -> f64` (TS's ±Infinity at a result).
- `shadow_ban`: `SHADOW_BAN: &[(&str, &str)]` (SURFACE §9), `SHADOW_BAN_IDS` (iterable of `&str` or `String`).
- `candidates`: `action_key(&ActionBody) -> String`, `candidate_actions(&GameState, PlayerId) -> Vec<ActionBody>`.
- `observe`: `redact`, `ai_to_act` (SURFACE §9), `unanswered_draw_offer(&GameState, PlayerId) -> bool`.
- `determinize`: `determinize(&GameState, PlayerId, &mut Rng, &DeterminizeOptions) -> GameState`,
  `DeterminizeOptions { match_shown_cost: Option<bool> }: Default`.
- `mulligan`: `mulligan_keep(&GameState, PlayerId, keep_max_cost: i32) -> Vec<String>`.
- `gate`: `game_config(Matchup, i32, SearchBudget, series: &str) -> MatchConfig`, `Matchup::{AiVsRandom,
  AiVsGreedy, HardVsEasy}`, `AI_GATE.seed_series: &str`.
- `baselines`: `random_action`, `greedy_action` (SURFACE §9).
- `dev_run`: `AI_DEV_RUN.series: &str`; `DevRunOptions { series: String, patch: String, budget: Option<SearchBudget> }:
  Clone` (TS's `Pick<…>` callers pass one with an empty patch); `dev_game_config(i32, &DevRunOptions) -> MatchConfig`;
  `dev_game_record(i32, &DevRunOptions) -> Option<GameRecord>`; `dev_record_id(&str, &str) -> String`.

Engine and cards (their owners' notes' shapes):
- `reduce`, `legal_actions`, `begin_game`, `seat_to_act -> Option<PlayerId>` (part 5.2); `create_game` (part 1).
- `zones::active_units_of` (iterated only, so `Vec<&CardInstance>` or owned); `query::zone_cards(&GameState,
  PlayerId, OffFieldZone::Hand)` (iterated only).
- `mana::{effective_cost(&GameState, &CardInstance, CostOptions), CostOptions: Default}` (part 4.1).
- `catalog::{query(&CatalogQueryArgs) (Default, Deserialize), def_of(Option<&GameState>, &str) -> &CardDef,
  query_cost(&CardDef) -> i32, registered_catalog()}` (part 2.2).
- `setup::mulligan_prompt_for -> Option<&PendingChoice>` (part 5.3).
- `replay::{fold(&FoldArgs) -> { state, errors }, hash_state}`, `FoldArgs: Deserialize` (built with `json_as`)
  (part 5.1); `game_summary::summarize_game(&FoldArgs) -> Option<GameSummary>`.
- `subsystems::{choose_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>, abilities_of(&GameState,
  &CardInstance) -> Vec<ActivationDecl>, POWER_KEY: &str}` (part 8).
- `prompts::{ANSWER_KEY: &str, answer_key_of(&IndexMap<String, Value>) -> Option<String>}`;
  `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext,
  HookOptions { controller: Option<PlayerId>, .. }: Default}` (part 3.2).
- `effects::{damage, choose_answer}` (argument structs via `json_as`; `correct` an index), `effects::answered_correctly(
  &EffectContext) -> bool` (parts 6–7).
- `layers::unit_view(..) -> UnitView { attack: i32, keywords: Vec<Keyword>, .. }`; `restrictions::cannot_attack(
  &GameState, &CardInstance) -> bool`; `brittle_count::give_brittle_count(&GameState, &mut CardInstance, i32)` and
  `tuning::tuning_of(&mut CardInstance) -> &mut Tuning` (part 2.1); `animated::{animate_card(&mut EngineSink,
  &CardInstance, AnimateOptions) -> bool, AnimateOptions { position: Option<Position> }}` (part 2.2).
- Testkit (part 5.1): `scenario(Value) -> Scenario` (`state()`, `play(ref, Value)`, `attack`, `backrow(seat, lane)`,
  `hand(seat)`, `card(ref)`), `register_catalog_as(CardDefs, &str)`, `register_scripts(IndexMap<String, CardScripts>)`.
- Cards (part 1): `jackioh_cards::{register_all, CATALOG, scripts_of, catalog_version}`.

### Not ported
- `dev-run.test.ts`'s third `it`, "reads the run's options, and refuses one it does not know": it tests
  `scripts/stats.ts`'s `parseDevRunArgs`, which is `cargo jackioh stats`'s argument parser now
  (`crates/tools/src/stats.rs`, part 22). The AI test binary cannot link `jackioh-tools` (it depends on
  `jackioh-ai`), so the case belongs in part 22's tests: defaults `{ games: AI_DEV_RUN.games, from: 1, patch: null,
  series: AI_DEV_RUN.series, out: null }`; `-- --games=50 --from=51 --patch=v0.2.5 --series=a --out=run.jsonl`;
  refusals "--games must be a positive integer", "--from must be a positive integer", "Unrecognised option
  --budget", "Unrecognised argument".
- TS's per-test `{ timeout }`s (no `cargo test` twin).

## Decisions
- `NodeCounter` takes `&self` and keeps its tallies in `Cell`s (not banned by `clippy.toml`; `RefCell` is): TS's
  counter is one object the lethal solver, the beam, the replies and `decide` share while a sub-counter holds its
  parent, and `&self` lets `decide` spend the parent between a sub-counter's uses with no borrow conflict.
  `&dyn NodeCounter` parameters accept a `&counter` and a `&mut counter` alike.
- TS's loose `typeof counter.simErrors === "number"` is the trait's `sim_error_tally() -> Option<usize>`.
- Budgets and node counts are `usize` (part 1's rule for loop bounds); game numbers `i32`; scores `f64`.
- `simulate`'s try/catch and `play_sweep_game`'s are `catch_unwind(AssertUnwindSafe(…))`: an engine panic where TS
  threw is a failed simulation (`sim:` error noted) or a sweep error, never a crashed decision. A panic's payload
  (`&str` or `String`) is the message, as TS's `error.message`.
- `close_line` hands back a clone where TS handed back the same object; `terminal_score` keeps TS's identity test
  through the private `close_line_step` (`None` = unchanged).
- Sweep: `SweepFlag::as_str` writes the reason strings; `mean(..).toFixed(1)` is `to_fixed_1` (JS's exact rounding:
  Rust's `{:.1}` except an exact tie, which JS rounds away from zero, and no `-0.0`). `AI_SWEEP.at_risk_boost` is the
  whole number 4 (`i32`), so its JSON reads into `AiDeckOptions.boost.by` whatever that field's number type.
  `AiDeckOptions` is built with `json_as` from TS's literal. The hooks' closures borrow the game's counters; the
  hooks are dropped before the counters are read.
- Tests: TS's import-time registration of a test-only card (`activate`'s pricey token, `evaluate-v020`'s idle wall,
  `answer-key`'s quiz script) is per test here (`install_*`), since the testkit's override is per thread; the
  catalog and scripts are `jackioh_cards::CATALOG` and `scripts_of()` plus the test card. TS helpers that wrote
  through a live card write the state's copy, found with `find_instance_mut`. Jest's `toBeCloseTo(x, 10)` is
  `|a − b| < 10⁻¹⁰ / 2`; `toThrow()` on a non-step is `catch_unwind(..).is_err()`; `toMatchObject` on a record
  compares the named fields of its JSON; an object-identity check (`before === plan.firedOn`) is value equality.
- `support.rs`'s nonce counter is a `thread_local!` `Cell<u32>` (TS's module `let`), so nonces never repeat in a test.
