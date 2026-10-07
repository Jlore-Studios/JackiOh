# Slice: part 8 (engine 7: subsystems), chunk 1 of 3 — issue #396, parent #306
BUILDS-RUN: 0

## FILES
All seven are full ports of their TS files, every function in TS order with its doc comment and every
comment that states a rule or cites a ruling:
- `crates/engine/src/subsystems/ai_policy.rs` ← `aiPolicy.ts`
- `crates/engine/src/subsystems/audit.rs` ← `audit.ts`
- `crates/engine/src/subsystems/call_to_chaos_plus.rs` ← `callToChaosPlus.ts`
- `crates/engine/src/subsystems/fuse.rs` ← `fuse.ts` (no `syncFusedScripts`, `ensureFused` or registry/digest-table writes; `compose_fused_scripts` added, SURFACE §6.6)
- `crates/engine/src/subsystems/glitch.rs` ← `glitch.ts`
- `crates/engine/src/subsystems/perfect_hand.rs` ← `perfectHand.ts`
- `crates/engine/src/subsystems/rotation.rs` ← `rotation.ts`

## SURFACE
Matched as written, with part 1's frozen code where the two differ (hooks take `&mut EffectContext`).
Public names: `AI_SKIPPED_ACTIONS`, `PolicyOptions`, `policy_actions`, `choose_action(state, player,
rng)`, `choose_action_with(.., options)`, `PlayoutStop`, `PlayoutResult`, `play_out_turn`,
`AI_TURN_WORK`, `run_owed_ai_turn`; `lines_of_code`, `AuditArgs`, `audit_targets`;
`replace_deck_with_call_to_chaos`, `CHAOS_PLUS_EFFECTS`; `FuseArgs`, `HandPrice`, `fused_digest`,
`fused_id_specs_in`, `fused_ingredients`, `fused_ingredient_specs`, `compose_fused_scripts`,
`rebuild_fused_def`, `fuse`; `count_system_play`, `seats_swapped`, `seat_played_by`, `glitch`,
`reset_match`; `RankPerfectHandOptions`, `ReplaceHandWithPerfectArgs`, `rank_perfect_hand`,
`replace_hand_with_perfect`; `ROTATION_ROWS`, `RotationDirection` (re-export of the wire's),
`RotationArgs`, `RotationResult`, `rotate_rings`. Constants TS declared here (`AI_PLAYOUT_STEP_CAP`,
`FUSE_MIN_INGREDIENTS`, `CRAFTED_CARD_COST`) are used from `crate::config`, never redefined.

## DEPENDS-ON
Every name under GAPS below, at the path and with the signature given there.

## GAPS
Names I call that other parts provide (signature as I call it; part 31 reconciles):
- part 5 `reduce.rs`: `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`, `reduce(&GameState,
  &Action) -> ReduceResult { state, events, error: Option<String> }`. `reduce.rs`'s
  `answerForLockedOut` and part 7's `effects/combat.rs` call `play_out_turn(sink, player,
  PolicyOptions::default())` and read `.actions`.
- **TS `reduce(state, action, sink.rng)` has no Rust form** (SURFACE §6.1 drops the rng). `ai_policy`
  emulates it: the sink's cursor goes into `state.rng_cursor`, the sink's rng is rebuilt from the
  result's cursor. Exact while the sink's rng is the match stream (every real action). Under a side
  stream (a scorer dry run, `Rng::new("zephyrs-dry-run", 0)`) the inner `reduce` draws from the match
  stream instead and the side rng is left as it was; and the inner `reduce`'s own sink starts with
  `dry_running = false`, where TS's module flag stayed set. Only a dry run that reaches My Pawn's AI
  turn can tell. Fix if a golden trace shows it: a `reduce_with(state, action, sink)` in `reduce.rs`.
- part 3 `triggers.rs`: `mark_dispatched(&[GameEvent])` — TS marks by object identity (a WeakSet);
  Rust has none and `EngineSink` has no field for it. I pass the slice of `sink.events` the playout
  step just appended (`&sink.events[from..]`); triggers.rs must decide how it remembers them.
  `settle(&mut EngineSink, SettleOptions)` (called with `Default::default()`).
- part 3 `work.rs`: `owe(&mut EngineSink, Resume) -> _` (TS's variadic; every caller passes one), the
  dispatcher arm `"@aiTurn"` → `crate::subsystems::ai_policy::run_owed_ai_turn(sink, &WorkItem)`;
  constants `PART_KEY`, `PART_DEPTH_KEY`, `REMEMBERED_KEY`.
- part 2 `scripts.rs`: `script_of(&GameState, &str) -> CardScripts` (calls `compose_fused_scripts` for a
  fused id in `state.transient_defs`), `INGREDIENTS_KEY`.
- part 2 `catalog.rs`: `def_of(Option<&GameState>, &str)`, `find_def(Option<&GameState>, &str) ->
  Option<&CardDef>`, `query(CatalogQueryArgs) -> Vec<CardDef | &CardDef>` with `CatalogQueryArgs:
  Deserialize` (built with `json_as`), `pick_generated(&mut Rng, &[CardDef], Option<&GameState>) ->
  Option<&CardDef>` (TS passed the state as `GlitchOdds`), `def_by_index(SetName, &str)`,
  `FUSED_DIGEST_MARK`, `RADIANT_INGREDIENT_MARK`. `catalog::fused_id_specs`/`self_def_ids` are not
  called: their digest lookup needs the state, so `fuse::fused_id_specs_in(state, id)` does it.
- part 2 `zones.rs`: `ZoneSlot { player, row, lane }`, `slots_of(PlayerId, Row)`, `active_units_of`,
  `card_at`, `zone_contents` (lists or options of `&CardInstance` or owned; I take either through
  `Borrow`), `ring_order(Row, PlayerId)`, `ring_neighbor(&ZoneSlot, RotationDirection, PlayerId)`,
  `is_locked`/`is_reserved(&GameState, &ZoneSlot)`, `unlock_zone(&mut GameState, &ZoneSlot)`,
  `first_free_zone(&GameState, PlayerId, Row) -> Option<ZoneSlot>`, `place_on_field(&mut GameState,
  &CardInstance, &ZoneSlot, PlaceOnFieldOptions { stack: Option<bool> }) -> bool`,
  `remove_from_field(&mut GameState, &CardInstance, RemoveFromFieldOptions { with_pile: Option<bool> })`,
  `cease_to_exist(&mut GameState, &CardInstance)`, `move_to_zone(&mut GameState, &CardInstance,
  OffFieldZone, MoveToZoneOptions) -> MoveResult`, `report_graveyard_landing(&mut EngineSink,
  &CardInstance, MoveResult)`. `place_on_field` and `bounce_card` must accept a card the state no longer
  holds (rotation lifts every ring card first, as TS did).
- part 2: `layers::{worn_stats_override(&CardDef, &CardInstance) -> Option<AttackHealth>,
  unit_has(&GameState, &CardInstance, KeywordKind) -> bool}`, `tuning::sum_tunings(&[Option<Tuning>])
  -> Option<Tuning>`, `enchantments::united_enchantments(&[CardInstance]) -> Option<Vec<Enchantment>>`.
- part 4: `mana::{printed_cost(&GameState, &CardInstance) -> i32, effective_cost(&GameState,
  &CardInstance, CostOptions) -> i32}`, `draw::add_to_hand(&mut EngineSink, &CardInstance)` (its
  `"hand" | "burned"` answer is not read: the card's zone after it is), `play_choices::{
  DECLARATION_SLICES_KEY, selections_per_declaration(&GameState, PlayerId, &CardInstance,
  &[TargetDecl], &[Selection]) -> Vec<Vec<Selection>>}`, `prompts::run_start_of_game(&mut EngineSink,
  &CardInstance, PlayerId) -> bool`.
- part 3/5: `combat::enter_new_side(&mut EngineSink, &CardInstance, PlayerId)` (writes through to the
  state's instance by id), `game_over::end_game(&mut EngineSink, Winner, GameOverReason)`,
  `setup::begin_setup(&mut EngineSink)`.
- part 7 effects (each built from its TS object literal with `json_as`, so each args type must derive
  `Deserialize`): `add_to_hand::{add_to_hand, add_random_from_catalog}`, `destroy::destroy_all`,
  `tune::{upgrade, degrade}`, `fuse::fuse_random_into`, `summon::summon`, `transform::transform`,
  `move_::bounce_card(&mut EngineSink, &CardInstance)`. Part 7's `effects/fuse.rs` calls `fuse(ctx,
  FuseArgs { .. })` and `HandPrice`; `effects/last_board.rs` calls `rebuild_fused_def(state, id, owner)`.
- part 8 chunks 2/3: `scorer::{dry_run_base(&GameState, PlayerId) -> Option<GameState>,
  score_def(&GameState, PlayerId, &CardDef, ScorerOptions, Option<&GameState>) -> Scored, Scored {
  def, score: f64, .. }, ScorerOptions { radiant: Option<bool> }}`; `call_to_chaos::{CHAOS_TAG: Tag,
  ChaosEffectDef { name: &'static str, label: &'static str, build: fn() -> Effect },
  cast_random_call_to_chaos() -> Effect}` — `CHAOS_PLUS_EFFECTS` is a `pub const &[ChaosEffectDef]`,
  so `build` must be a plain fn pointer (if chunk 2 made it an `Arc<dyn Fn>`, the table becomes a
  function and each `build:` an `Arc::new(build_…)`).
- part 17 (AI): TS `subsystems.fusedIngredients(defId)` is `fused_ingredients(state, def_id)` (the
  digest needs the state); `subsystems.syncFusedScripts(state)` (lethal.ts) is gone — drop the call.
  Part 5's `reduce`/`legal_actions`/`view_for` drop their `syncFusedScripts` calls likewise.

## Decisions
- **Calls into other parts**, one convention throughout: an instance or other struct argument by `&`;
  a TS option object with a default (`options = {}`) by value as `<FnName>Options` (`Default::default()`
  where TS passed none, `{ field: Some(..), ..Default::default() }` where it passed one); a string union
  as its enum; a TS argument object an effect takes as `json_as(json!({ … }))`.
- **TS's live objects**: wherever TS read an instance after a call that moved or changed it, Rust reads
  it back from the state by id (`find_instance`): the controller a rotation gave a card, the zone a
  bounce or a crafted card landed in, the kept fused card after `keep_instance` and the start-of-game
  roll, a graveyard landing's zone. `fuse` refreshes its ingredients and target from the state as it
  starts (`as_it_stands`); a card the state no longer holds (consumed by an earlier fusion, a
  Discovered definition in no pile) is the caller's copy with its zone set to `gone`, as TS's live
  object of a ceased card was, so ceasing it again records no second departure.
- **Fused scripts** are composed on lookup: `compose_fused_scripts(state, def)` reads the ingredient list
  off the definition (`CardDef.ingredients`, digests included), or the id; an ingredient that is itself
  fused is composed recursively here (`scripts_for`), with TS's `seen` guard; any other id comes from
  `scripts::script_of`. TS's untyped `combineValues`/`combineObjects` became per-field combination of
  the `Script` struct (`combine_objects` and the helpers its doc lists), each field by the rule TS
  applied to its kind; the order of checks in `combineValues` is kept where it shows (one ingredient's
  static flags or target-check table or quest book returned as is; the summed flags summed even when
  only one is set, so `quickstriker: true` becomes `Count(1)` once two ingredients have flags).
  `targeting_discards` and `records_play_as` are `None` on a fused script: TS turned them into a
  combined effect hook that nothing ever calls (their readers walk a fused card's ingredients).
- `fused_aura`: an aura's entries may borrow the card asked about; for an ingredient read at its own
  price (a copy, `as_ingredient`) each entry owns that copy and asks the aura again per unit (a pure
  read, the same answer). Ingredients with no price record pass the arguments straight through.
- Effect kinds are `&'static str`: `fused:part0`…`fused:part7` (then `fused:part`), and one static
  `callToChaosPlus:<name>` per entry; nothing reads a kind but the debug output. `ingredient_part`'s
  memo is `Some(Value::Null)` (TS `null`), an entry's is `None` (TS absent).
- `lazy_part`, `apply_effects`, `active_target_decls`, `stored_declaration_slices`, `part_path_of`,
  `reroot_remembered`, `memory_of_part` and the `INGREDIENTS_KEY` readers are private copies (rule 5),
  reading the owning modules' key constants so the two cannot disagree on a key.
- `fused_digest` over UTF-16 code units with `wrapping_mul` and `u32` shifts; the id cap compares the
  body's UTF-16 length.
- `choose_action(state, player, rng)` (the brief's three-argument form) plus `choose_action_with(..,
  options)`; `policy_actions` and `play_out_turn` take `PolicyOptions` by value. `run_owed_ai_turn` is
  `pub` for `work.rs`'s dispatcher and takes `&WorkItem`; TS's `finally` is the work put back after the
  body returns.
- Not wire types, so plain derives without the `string_union!` macro (no TS binding generated):
  `PlayoutStop`, `HandPrice` (serde camelCase, for `FuseArgs`). Named TS's anonymous argument types:
  `AuditArgs`, `RankPerfectHandOptions`, `ReplaceHandWithPerfectArgs`; `FuseArgs`, `RotationArgs` and
  these derive serde camelCase so a card can `json_as` them.
- `CHAOS_PLUS_EFFECTS` is a constant table; TS's `addFree(name, pool, count)` thunks became the named
  builders `build_fruits`, `build_books`, `build_classic` (and one per other entry).
- `perfect_hand`: TS's `excludingDefId` is the query's `excludeDefId` key, filled with the running card's
  ids (`self_def_ids`, via `fuse::fused_id_specs_in`) only when there are any, as TS left the query
  unchanged otherwise. The sort keeps TS's `b.score - a.score || id` with a NaN difference as a tie.
- `glitch::reset_match` copies `applied`, `last_boards`, `glitch_boards` and `seat_swaps` across as TS
  did, and passes `handicaps: Some(..)` (possibly empty) as TS passed `{}`.
