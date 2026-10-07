# Slice: part 17 (the AI in Rust, generation 0), chunk 1 of 3 (#405, parent #306)
BUILDS-RUN: 0

## FILES
All new (every one was an empty placeholder; no night-bot port existed for this part), each the
whole TS file at `91cc43c` (= current `packages/ai/src`): every function in TS order, its doc
comment and every comment that states a rule or cites a ruling.
`crates/ai/src/{baselines, candidates, config, decide, deck, determinize, dev_run, evaluate, gate,
lethal, match_, mulligan, observe, reply, search, shadow_ban}.rs`. Nothing left.
`config.rs` leaves out `EMOTE_TRIGGERS`, `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS`,
`EmoteTrigger`, `EmoteReplyKey`, `PersonaName`, `PersonaSpec` (moved to the web, part 21).

## SURFACE
§9 matched: `decide(&GameState, PlayerId, &mut AiOptions) -> Option<Decision>`,
`build_ai_deck(&mut Rng, i32, &AiDeckOptions) -> Vec<String>`, `redact`, `ai_to_act`,
`random_action`, `greedy_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>`,
`play_match(&MatchConfig, &mut MatchHooks) -> MatchRecord`, `SHADOW_BAN`/`SHADOW_WATCH:
&[(&str, &str)]` (11 entries, sorted). `action_key` is `jackioh_engine::canonical` of the action's JSON.
Engine names are called through the crate root (`jackioh_engine::<name>`, which part 1's `lib.rs`
globs from every module) and `jackioh_engine::subsystems::<name>`, so a function that landed in
another module than TS's still resolves.

## DEPENDS-ON / GAPS

### Chunk 2 of this part (`types.rs`, `simulate.rs`), names and shapes I call
- `types`: `SearchBudget { nodes, lethal_nodes, determinizations, beam_width, root_branching,
  branching, max_depth, finalists }` all `i32`, `Copy + PartialEq + Debug + Serialize + Deserialize`
  (config.rs builds `AI_BUDGET` with that literal; `SeatController`, `MatchConfig`, `DevRunOptions`
  derive serde/`PartialEq`/`Copy` over it). `AiOptions<'a> { rng: Rng, budget: SearchBudget,
  should_stop: Option<&'a dyn Fn() -> bool> }` (SURFACE). `Decision { action, reason, line, stats }`
  (`Clone + Debug`). `DecisionReason::{Forced, Mulligan, DrawOffer, Lethal, Prompt, Search, Fallback}`
  (`PartialEq`). `SearchStats { nodes: i32, determinizations: i32, lines: i32, sim_errors: i32,
  stopped_by: StoppedBy, score: f64 }`. **`StoppedBy::{Exhausted, Budget, Clock}`** (my name for TS's
  `SearchStats["stoppedBy"]`; `Copy + PartialEq`).
- **`NodeCounter` as a trait** with methods `used() -> i32`, `limit() -> i32`, `take(&mut self) -> bool`,
  `stopped_by() -> StoppedBy`, and **`sim_errors() -> i32` on the trait itself** (decide reads it
  through `&dyn NodeCounter`). Every function here takes `counter: &mut dyn NodeCounter`.
- `simulate`: `LineStatus::{Open, Passed, Yielded, Over}` (`Clone + Debug + PartialEq`),
  `create_node_counter(i32, Option<&dyn Fn() -> bool>) -> <a NodeCounter>`,
  `create_sub_counter(&mut dyn NodeCounter, i32) -> <a NodeCounter borrowing its parent>`,
  **`simulate(&GameState, PlayerId, &ActionBody, &mut dyn NodeCounter) -> Option<Result<GameState,
  String>>`** (TS `SimStep | null`: `Ok` = `{ ok: true, state }`, `Err` = `{ ok: false, error }`),
  `line_status(&GameState, PlayerId, i32) -> LineStatus`, `close_line(&GameState, PlayerId, i32,
  &mut dyn NodeCounter) -> GameState` (owned; I test "no step taken" with `==`, see Decisions),
  `static_score(&GameState, PlayerId, i32) -> f64`, `search_signature(&GameState, PlayerId) -> String`.
- Chunk 2's `sweep.rs` will call mine as written here: `play_match`, `MatchHooks { after_action:
  Option<Box<dyn FnMut(&GameState, &GameState, PlayerId, &ActionBody) + 'a>>, time_decision:
  Option<Box<dyn FnMut(PlayerId, &mut dyn FnMut()) + 'a>> }` (`Default`; the timing hook must call the
  run closure once, the run keeps its result), `build_ai_deck` with `AiDeckOptions { include, boost:
  Some(AiDeckBoost { ids, by }), .. }`, `greedy_action`, `evaluate(.., NextSwing, &EvalWeights)`,
  `SHADOW_BAN`/`SHADOW_WATCH` slices.

### Other parts (signature as I call it)
- Part 2: `find_def(Option<&GameState>, &str) -> Option<&CardDef>`; `query(&CatalogQueryArgs) ->
  Vec<&CardDef>` (deck.rs annotates `Vec<&CardDef>`, as part 2.2's notes say it returns
  `Vec<&'static CardDef>`), `CatalogQueryArgs: Default + Deserialize` (determinize builds it with
  `prelude::json_as`), `query_cost(&CardDef) -> i32`; `scripts_for(&GameState, &str) -> CardScripts`;
  `active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>`; `unit_view(&GameState, &CardInstance)
  -> UnitView { attack, max_health, health, keywords, armor, position: Position }`; `unclamped_attack
  -> i32`; `active_brittle_count(&CardInstance) -> Option<i32>`; `animated_kind_of -> Option<_>`;
  `cannot_attack -> bool`; `own_cost -> Option<i32>`; `announced_face_down_to(&GameState,
  &CardInstance, PlayerId) -> bool`.
- Part 3/4: `effective_cost(&GameState, &CardInstance, CostOptions) -> i32` (`CostOptions::default()`);
  `hero_armor_of(&GameState, PlayerId) -> i32`; `combat::{can_attack(&GameState, &CardInstance,
  &AttackTarget) -> bool, has_exertion(&GameState, &CardInstance, ExertionKind) -> bool}`,
  `AttackTarget::{Hero { player }, Unit { instance: CardInstance }}`, `ExertionKind::Attack`;
  `prompts::ANSWER_KEY: &str`.
- Part 5: `reduce(&GameState, &Action) -> ReduceResult`, `begin_game`, `legal_actions(&GameState,
  PlayerId) -> Vec<ActionBody>`, `seat_to_act -> Option<PlayerId>` (`None` read as `state.active`,
  TS's last fallback), `setup::{SETUP_WORK: &str, mulligan_prompt_for(&GameState, PlayerId) ->
  Option<&PendingChoice>}`, `replay::{canonical(&Value) -> String, hash_state, fold(&FoldArgs) ->
  FoldResult { state, errors: Vec<_> }}`, **`FoldArgs: Default`** with fields `seed, decks, log,
  handicaps` (I write `..FoldArgs::default()`), `game_summary::summarize_game(&FoldArgs) ->
  Option<GameSummary>`.
- Part 8: `subsystems::{choose_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>`
  (the default skip set is `AI_SKIPPED_ACTIONS`, which TS passed explicitly), `AI_SKIPPED_ACTIONS:
  &[ActionType]`, `fused_ingredients(&GameState, &str) -> Option<Vec<String>>`,
  `power_ability_of(&GameState, &CardInstance) -> Option<&ActivationDecl>`, `abilities_of(&GameState,
  &CardInstance) -> Vec<ActivationDecl>`, **`projected_hero_damage(&GameState, PlayerId, i32, bool)
  -> i32`** (TS's `pierce = false` passed as `false`)}`. `syncFusedScripts` (lethal.ts `readyGap`) is
  dropped, as part 8.1's notes ask.
- Part 19.2 wrote `AiDeckOptions { banned: Vec<String>, .. }`: here `banned: Option<Vec<String>>`
  (SURFACE §4.3; TS's absent is "the shadow ban", which an empty `Vec` default could not say). Callers
  that ban nothing pass `banned: Some(vec![])`.

### Not ported
Nothing. `index.ts` is part 1's `lib.rs`.

## Decisions
- **Defaulted TS parameters are explicit arguments** (`evaluate(state, seat, NextSwing::Enemy,
  &AI_EVAL)`, `unit_worth(.., &AI_EVAL)`, `mulligan_keep(.., AI_MULLIGAN.keep_max_cost)`,
  `game_config(matchup, n, AI_GATE_BUDGET, AI_GATE.seed_series)`, `run_gate(.., budget)`,
  `simulate_reply`/`reply_score(.., hidden: &IndexSet<String>)` with TS's default `hidden_card_ids(..)`
  computed by the caller, `score_line(.., reply: bool, hidden: Option<&IndexSet<String>>)` since
  TS's `hidden?` had no default value, `determinize(.., DeterminizeOptions)` by value with `Default`).
  `dev_game_config(n, series: &str, budget: Option<SearchBudget>)` takes TS's `Pick<…>` fields as
  arguments.
- **`catch_unwind` stands in for TS's `try`/`catch`** around `ai_to_act` and the body of `decide`,
  around `reduce` in reply's `reduce_for`, and around the controller call and `reduce` in
  `play_match` (an engine invariant is a panic in Rust, SURFACE §4.4.9); a refusal still comes back as
  `ReduceResult.error`. The panic's message (`String`/`&str` payload, else "panic") fills TS's
  `messageOf`.
- `decide` builds its node counter first (TS built it after the determinizations) so the counter
  outlives the guarded body; stats are unchanged (nothing takes a node before the lethal solver). The
  beam's sub-counter stop reason is read as the beam ends (the slice cannot outlive its `&mut`
  parent); nothing takes a node through it later, so it is TS's end-of-decide value.
- `search::beam_search`: TS compared `closeLine`'s answer to its input by identity; here `end ==
  line.state` (a simulated step always changes the state).
- lethal's frames share states through `Rc<GameState>` (TS shared object references); search's open
  lines own theirs.
- `observe`: cards are walked by place (`Place`) so step 3 reads `effective_cost` on the clone before
  it writes the placeholder, in TS's order; queue entries are edited in place on the clone; a dispatch
  event is scrubbed through its JSON and parsed back; setup's owed mulligan is scrubbed as JSON
  (`to_placeholder_json` mirrors `to_placeholder` key for key). `instance_number` answers `f64`
  (`+∞` for a non-minted id).
- `determinize`: TS's `rng.int(2 ** 31)` overflows `Rng::int(i32)`; `int_2_31` makes the same single
  draw by hand (`floor(next() * 2³¹) % 2³¹`). Placeholders are addressed by `TrapSlot` (top, dormant,
  resolving) while candidate traps are tried in place.
- `deck`: `CostBucket` enum (`"0-1"`, `"2"`, `"3"`, `"4+"`) and `ByBucket<T>` (`Index<CostBucket>`)
  for TS's `Record<CostBucket, T>`; `AiDeckOptions.theme: Option<Option<String>>` keeps TS's three
  states (absent = roll, `null` = none, a tag) with a `deserialize_with` that maps JSON `null` to
  `Some(None)`; `boost: Option<AiDeckBoost { ids, by }>`; TS's throws are `panic!` with the same text;
  `weighted_index` answers `Option<usize>` (TS's -1) and always takes its draw.
- `gate`: `Matchup` (serde literals, `ALL`, `as_str`, `Display`) and `ByMatchup<T>` with
  `Index<Matchup>` for `AI_GATE.full_seeds[matchup]` (part 22.1's notes). `AiGate`'s ms fields are
  `f64`, counts `i32`. `GateGame.replay_errors: i32`.
- `match_`: `AI_MATCH`/`AI_TURN_MAX_ACTIONS` stay module-private; `MatchConfig.max_actions:
  Option<usize>`; TS's anonymous record rows are `RejectedAction { seat, action, error }` and
  `ThrownError { seat, message }`; `SeatController` is tagged on `kind` with `kind()`. The AI
  controller's `Rng` goes into `AiOptions` and is written back after `decide`. `played_def_id` reads
  `state.players[seat].hand` (TS's `zoneCards(state, seat, "hand")` was a copy of it).
- `config`: one `const` struct per TS object (`AiSearch`, `AiReply`, `AiMulligan` (also
  `GreedyMulligan`), `AiDeterminize`); `AI_EVAL`'s struct is TS's `EvalWeights` (alias `AiEval`), its
  `keyword` table `KeywordWeights` with `of(KeywordKind) -> Option<f64>` (TS's `weights[kind]` was
  `undefined` for an unnamed kind). Every weight is `f64`; every count `i32`.
- `shadow_ban`: `SHADOW_BAN_IDS` is computed from `SHADOW_BAN` in a `const` block (the table is kept
  sorted, so its ids in order are `Object.keys(...).sort()`).
- Hand-written enums (not `wire::string_union!`, which is `pub(crate)` to the engine and exports TS
  types): `NextSwing`, `CostBucket`, `Matchup`, `SeatController`.
