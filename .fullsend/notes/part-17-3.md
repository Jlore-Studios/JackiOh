# Slice: part 17 (the AI in Rust, generation 0), chunk 3 of 3 (#405, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All fifteen were empty placeholders on `staging` (no night-bot port of this part existed, and no
`part-17-bot.md`); each is now the whole TS test file: every `it` as a `#[test]` (one `mod` per
`describe`, the TS title kept verbatim as the test's doc comment, ruling ids leading the fn name by
SURFACE §7.3), every helper in TS order, every comment that states a rule or cites a ruling.
`crates/ai/tests/ai/{observe_instance_data, observe, prompts_v020, puzzles, redact_announce,
redact_backrow_piles, redact_board_history, redact_fusion, redact_last_boards,
redact_live_face_down, reply, search, shadow_ban, surface, tutorial_tier}.rs`.
Test counts equal TS's `it` counts (search: TS's `for` over two budgets makes two tests). Nothing left.

## SURFACE
- Imports: `use jackioh_ai::*; use jackioh_engine::testkit::*; use super::support::{…};` in every file.
  `scenario` is the testkit's (not re-imported from `support`), so the files do not depend on whether
  support.rs re-exports it.
- §9 as fixed: `decide(&GameState, PlayerId, &mut AiOptions)`, `AiOptions { rng, budget, should_stop:
  Option<&dyn Fn() -> bool> }` (a counting clock is a `std::cell::Cell` behind an `Fn`), `redact`,
  `ai_to_act`, `build_ai_deck(&mut Rng, i32, &AiDeckOptions)`, `play_match(&MatchConfig, &mut
  MatchHooks)`, `SHADOW_BAN`/`SHADOW_WATCH: &[(&str, &str)]`.
- §8 testkit: `scenario(Value)`, `.state()`, `.play(card, json!({}))`, `.attack(card, "hero")`,
  `.end_turn()`, `.unit(seat, lane) -> Option<CardInstance>`, `.hand(seat)`, `.pile(seat,
  "graveyard")` (part 5.1's notes); `register_scripts(IndexMap)`, `register_catalog_as(CardDefs,
  &str)` (the thread-local override).
- Aligned with part 17.1's notes (seen on `staging` before the last push): `NodeCounter` a trait
  (`used()`, `limit()`, `take()`, `stopped_by()`); `simulate -> Option<Result<GameState, String>>`;
  `determinize(.., DeterminizeOptions)` by value; `simulate_reply`/`reply_score(.., hidden:
  &IndexSet<String>)` required (TS's default `hiddenCardIds(state, seat)` passed explicitly);
  `score_line(.., reply: bool, hidden: Option<&IndexSet<String>>)`; `evaluate(.., NextSwing,
  &EvalWeights)`; `MatchConfig { seed, decks: (Vec, Vec), handicaps: Option<PerPlayerOpt<Handicap>>,
  controllers: PerPlayer<SeatController>, max_actions: None }`; `SeatController::{Ai { budget },
  Greedy}`; `MatchHooks { after_action: Some(Box::new(|before: &GameState, after: &GameState, seat:
  PlayerId, action: &ActionBody| …)), ..Default::default() }`.
- Aligned with part 22.1's notes: `SweepOptions { seeds: Option<i32>, now: Option<&dyn Fn() -> f64>,
  tier: Option<Difficulty> }` by value; `sweep_card(&str, SweepOptions)`, `sweep_at_risk(&str,
  &[String], &[String], SweepOptions)`, `at_risk_ids(&[SweepResult], &[(&str, &str)], &[(&str,
  &str)])`, `pass2_keep_out(&[SweepResult], &[(&str, &str)])`, `sweep_verdict(&[SweepResult],
  &[SweepPass2])` (TS's default tables passed as `SHADOW_BAN`, `SHADOW_WATCH`).

## DEPENDS-ON / GAPS

### `crates/ai/tests/ai/support.rs` (another chunk of this part), as I call it
`AI`, `HUMAN: PlayerId`; `ai_pool()`, `trap_pool() -> Vec<String>`; `every_card(&GameState)`
(`Vec<&CardInstance>` or `Vec<CardInstance>`: both compile here); `card_by_id(&GameState, &str)`
(`Option<&CardInstance>` or `Option<CardInstance>`: both compile); `on_field`, `in_graveyard(&GameState,
PlayerId, &str) -> bool`; `is_legal(&GameState, PlayerId, &ActionBody) -> bool`; **`act(&GameState,
PlayerId, &ActionBody, Option<&str>) -> GameState`** (TS's optional nonce); `clone(&GameState) ->
GameState`; **`PuzzleRun { start, turn: AiTurnResult, end }`** (pub fields; puzzles.rs builds one);
**`run_puzzle(&str, Value, Option<SearchBudget>) -> PuzzleRun`** (`None` = TS's default AI_BUDGET);
`trace(&AiTurnResult) -> String`; `all_out_attack(&GameState, PlayerId) -> GameState`;
**`random_decks(&str, Option<(usize, usize)>) -> (Vec<String>, Vec<String>)`**; `dealt_game(&str) ->
GameState`; **`random_policy_states(&str, every, Option<max_actions>) -> Vec<GameState>`** (integer
literals, so `i32` or `usize` both work).

### `jackioh_ai` (chunks 1 and 2), beyond part 17.1's notes
- `Decision: PartialEq + Debug` (B11 and B15 compare decisions deep-equal, as TS's `toEqual` does);
  `DecisionReason: Copy` (read out of `&Decision`; SURFACE §5.1 gives unit unions `Copy`).
- `SearchStats.stopped_by` and `NodeCounter::stopped_by()`: compared by their serde literal
  (`"clock"`, `"budget"`), so the enum's name does not matter but it must `Serialize`.
- `SearchBudget: Copy` (a budget is passed to `AiOptions` and read again).
- `Line { actions, score: f64, .. }`; `AiTurnResult { state, actions: Vec<Action>, decisions:
  Vec<Decision> }` (no `Drop`: fields are moved out separately); `MatchRecord { result, log, hash,
  thrown, rejected, fallbacks: i32, .. }`; `hidden_instance_ids`, `hidden_card_ids -> IndexSet<String>`;
  `HIDDEN_DEF_ID: &str`; `AI_DETERMINIZE.exclude_def_ids: &[&str]`; `AI_DECK.min_pool`,
  `AI_SWEEP.{tiers, seeds_per_card, seeds_per_card_at_risk, decision_ms, min_affordable_turns,
  self_harm_delta, min_harm_plays, at_risk_boost, ban_affordable_turns, ban_harm_plays}` (read with
  `as` casts, so integer or float widths do not matter); `SHADOW_BAN_IDS` iterable of `&str` with `len()`.
- Sweep types must round-trip serde (camelCase): `SweepStats`, `SweepResult` (flattened or flat),
  `SweepPass2`, `SweepVerdict`, `SweepFlag` (its literals). shadow_ban.rs builds every fixture from
  TS's literal with `json_as` and reads every answer as JSON. `eval_delta_sum` may be `f64`; the
  count fields are written as JSON integers.
- `TierGame`'s parts (`MatchConfig`, `MatchRecord`) must be `Send + Sync` (cached in a `OnceLock`).

### Engine (other parts), as I call it
`hash_state`, `view_for(&GameState, PlayerId)`, `legal_actions`, `create_game(&CreateGameArgs)`
(`..Default::default()`), `begin_game(&GameState).state`, `seat_to_act -> Option<PlayerId>`,
`fold(&json_as(json!({ seed, decks, log, handicaps })))` (**`FoldArgs: Deserialize`**, part 22.1
assumes it too) `-> { state, errors }`; `catalog::{query(&CatalogQueryArgs), registered_catalog() ->
&CardDefs, catalog_version(), def_of(Option<&GameState>, &str)}`; `scripts::registered_scripts()`
(owned or `&`: `.clone()` taken); `zones::{place_on_field(&mut GameState, &mut CardInstance,
&ZoneSlot, PlaceOnFieldOptions { stack }), card_at(&GameState, &ZoneRef)}` with `ZoneSlot { player,
row, lane }` (part 2.1's alias of `ZoneRef`); `announce::begin_announce(&mut GameState,
AnnounceRecord)`; `subsystems::board_history::record_board_snapshot(&mut GameState)`;
`layers::unit_view(&GameState, &CardInstance) -> UnitView { attack, health, max_health, .. }`;
`mana::effective_cost(&GameState, &CardInstance, Default::default())` (part 4.1's three arguments);
`setup::mulligan_prompt_for(&GameState, PlayerId) -> Option<&PendingChoice>`;
`resolve::{make_context(&mut EngineSink, None, HookOptions { controller: Some(_), .. }), HookOptions:
Default}` (part 3.2's shape); `effects::{damage, destroy, choose_number, choose_pick, choose_cell,
choose_reward, choose_answer, choose_mode}(json_as(TS literal))`, `effects::{chosen_number(&ctx) ->
Option<i32>, chosen_cells(&ctx) -> Vec<ZoneRef>, chosen_options(&ctx) -> Vec<String>}`.

### Not ported
Nothing dropped. TS's `{ timeout }` options and tutorial-tier's `msPerGame`/`msFixed` (the timeout's
parts) have no `cargo test` twin.

## Decisions
- TS object literals (scenario setups, action bodies with many optional fields, effect arguments,
  sweep fixtures, `FoldArgs`, `CatalogQueryArgs`, `CardDef`) port as `json!` + `json_as`, so a Rust
  struct's field layout is not pinned by these tests; answers whose type names SURFACE does not fix
  (sweep results, `stoppedBy`, last boards, prompt options) are compared as JSON.
- fast-check (observe's property): no such crate in the workspace (SURFACE §2), so 150 cases are drawn
  from `create_rng("observe-property:185", 0)` (TS's `seed: 185`): base index, seat index, and a signed
  mutation seed from two draws. No shrinking.
- TS module-level state: `REAL_STATES` (observe) and tutorial-tier's `played` cache are `OnceLock`s
  local to the test binary, filled once by the first test that asks (read-only after, like the
  catalog). prompts-v020's module-level `registerCatalog`/`registerScripts` and observe's fixture
  script are installed per test through the testkit's thread-local override (`install()`), since each
  `#[test]` runs on its own thread.
- Every test (or the fixture builder it calls) calls `jackioh_cards::register_all()` first: TS's
  `setup.ts` and `_support.ts` did it on import.
- `expect(() => …).not.toThrow()` is the call itself (a panic fails the test); `toMatchObject` is a
  small recursive subset check; regexes are hand checks (`REASON` as `matches_reason`, the anchored
  ones as `starts_with` or exact equality: no regex crate in a pure crate).
- tutorial-tier's two `process.stdout.write` lines go to `std::io::stdout()` directly, past the test
  harness's capture, as TS wrote past vitest's.
- search's "omitting the budget is AI_BUDGET": `AiOptions.budget` is not optional (SURFACE §9), so the
  "implicit" decision is built as every caller without a budget builds it (`budget: AI_BUDGET`).
- redact_live_face_down reads `pile[pile.length - 1]` (the pile's last card) as TS did.
