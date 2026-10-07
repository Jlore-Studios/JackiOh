# Slice: part 7, chunk 2 of 2 (engine 6: effect verbs, second half) — #395, parent #306
BUILDS-RUN: 0

## FILES
All new, each the whole port of its TS file (every function in TS order, the header as `//!`, every
comment that states a rule or cites a ruling; the R-ids of each Rust file equal its TS file's, checked
by grep): `crates/engine/src/effects/{library, library_copies, mana, move_, perks, plague, radiant,
reveal, rounds, shuffle_card, shuffle_random, split, steal, summon, summon_this}.rs`. No `todo!`,
`unimplemented!` or `// TODO`. Nothing else touched but these notes.

## SURFACE
- Effect constructors are `pub fn <verb>(args: <Args>) -> Effect` over `Effect::new` /
  `resolve::lazy_part` (SURFACE §6.6). A TS anonymous argument object is a struct named
  `<VerbPascal>Args` (`ExileArgs`, `DiscardRandomArgs`, `PlacePlagueTokensArgs`, …), camelCase serde,
  `Default` when every field is optional; a TS `= {}` default is the caller's `Args::default()`.
  Named TS types keep their names: `SummonArgs`, `SummonPlacement`, `SummonCopyArgs`, `StatsOverride`
  (= `wire::AttackHealth`), `RecruitFilter`, `RecruitSource`, `StealTarget`, `RadiantTarget`,
  `RadiantZone`, `ExileZone`, `CostFilter`, `CounterDestination`, `Storm`, `CastStorm`.
- TS intersections `{ x } & BoardScope` / `& CostFilter` flatten the other type (`#[serde(flatten)]
  scope` / `filter`), so the JSON is TS's. `SummonArgs` carries `SummonPlacement`'s fields flat (a card
  writes `SummonArgs { def_id, lane, ..Default::default() }`); `SummonArgs::placement()` gives the half.
- New names TS left inline: `CostParity` (`CostFilter.parity`), `SplitAmong` (`damageSplit.among`),
  `StormSide` (`Storm.side`, which is `BoardScope["side"]`'s literals; it reaches a `BoardScope` through
  JSON so this file names no type of chunk 1's), `ExileHandArgs = DiscardHandArgs`,
  `SetRadiantArgs = RadiantTarget`. `Readers`/`READERS` (radiant) stay private, as in TS (a public
  `Readers` would collide with `card_scope`'s in the effects glob).
- `PLAGUE_PLACEMENT_HOOK` and `EXILE_ZONE_ORDER` are `pub const`. The plague placement answerer is
  `pub fn answer_placement(sink: &mut EngineSink, answer: &AnswerInput) -> Result<(), EngineError>`
  for `prompts.rs` to call for that hook (TS `registerPromptAnswerer`), refusals verbatim.
- `bounce_card(&mut EngineSink, &CardInstance)` and `discard_from_hand(&mut EngineSink, &CardInstance)`
  stay public (TS exported them for `targeting_point` and others); `clone_of` too.

## DEPENDS-ON
Called by TS name at TS path, snake_cased (signatures are guesses by the decisions below):
- `effects::targets` (chunk 1): `PlayerSpec`, `TargetSpec` (`TargetSpec::Chosen { index: None }`
  named; every other variant built from JSON), `BoardScope` (built from JSON or `Default`),
  `player_of(&EffectContext, PlayerSpec)`, `instance_of`, `resolve_target`, `instance_on_its_stay`,
  `cards_in_scope(&EffectContext, &BoardScope)`, `adjacent_to(ctx, &TargetSpec, &BoardScope)`.
- `effects::{brittle::give_brittle, cost::set_cost_mod, buff::grant_random_keywords,
  damage::damage_all, cast::cast_new, after_check::after_state_check}` (part 6 / chunk 1).
- `zones`: `move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, options)`, `MoveResult::
  {Moved, Vanished, Replaced}`, `OffFieldZone::{Hand, Library, Graveyard, Exile}`, `is_unit_token`,
  `report_graveyard_landing(&mut EngineSink, &CardInstance, MoveResult)`, `ZoneSlot { player, row,
  lane }`, `slots_of`, `card_at`, `slot_of`, `is_open`, `is_empty`, `is_reserved`, `first_entry_zone`,
  `row_size`, `fill_board_zones`, `place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot,
  PlaceOptions)`, `PlaceOptions { stack: Option<bool> }`, `remove_from_field(state, &card, options)`,
  `remove_from_any_zone(&mut GameState, &mut CardInstance)`, `lands_face_down`,
  `fresh_face_down_id(&mut GameState, &mut CardInstance) -> String`, `is_buried`.
- `draw::{add_to_hand(&mut EngineSink, &mut CardInstance) -> AddToHandResult, AddToHandResult::Hand,
  shuffle_into_library(&mut EngineSink, &mut CardInstance, bool, Option<_>)}`.
- `echo::exile_on_landing(&mut CardInstance)` (on the live card); `ownership::take_into_hand(sink,
  &mut CardInstance, PlayerId)`; `announce::{innermost_live_announce, is_announce_live,
  mark_countered}`; `mana::{effective_cost(state, &card, options), is_x_cost, NEXT_REFRESH_MODIFIER_ID,
  gain_mana(&mut PlayerState, i32), refresh_some_mana, mana_event(PlayerId, &PlayerState) -> GameEvent}`;
  `query::zone_cards(state, PlayerId, OffFieldZone)`; `plague::{permanents_on_field(state, PlayerId),
  place_plague_on(sink, &CardInstance, i32), remove_plague}`; `random_cast::{cast_mode_for_prompt(state,
  PlayerId, Option<&str>) -> Option<{ random, target_enemies }>, prefer_enemies(state, PlayerId, &[T],
  Fn(&T) -> Selection, n)}`; `prompts::{AnswerInput { player_id, choice_id, selection }, OpenPromptArgs
  { player, kind, aim, prompt, options, min, max, budget, owner, resume }, open_prompt, close_prompt,
  in_offered_order, resume_at(<args from JSON>), resume_of(&Resume), why_answer_refused(..) ->
  Result<(), EngineError>, run_start_of_game(sink, &CardInstance, PlayerId)}`; `work::{begin_work_cascade,
  drain_work}`; `catalog::{def_of(&GameState, &str), CatalogQueryArgs (Default + serde + PartialEq),
  excluding_def_id(&CatalogQueryArgs, Option<&str>), query(&CatalogQueryArgs), pick_generated(&mut Rng,
  &[CardDef], Option<GlitchOdds>), GlitchOdds { system_plays: Option<i32> }}`; `faces::card_type_of(state,
  &CardInstance)`; `brittle_count::start_brittle_on_field(&GameState, &mut CardInstance, bool)`;
  `resolve::lazy_part(&'static str, Fn(&mut EffectContext, &Memo) -> EffectPart)`; `damage::{DamageTarget::
  {Unit { instance }, Hero { player }}, DamageArgs { source, target, amount, flags }, deal_damage(sink,
  &DamageArgs)}`; `layers::unit_view(state, &CardInstance).health`; `combat::{enter_new_side(sink,
  &CardInstance, PlayerId), is_active_on_field}`; `animated::animate_on_entry(sink, &CardInstance)`;
  `stays::exit_mark(&GameState) -> u32`.

## GAPS
No function was left out. Every name above is another part's, and its shape is a guess part 31
reconciles; the riskiest:
- `crate::draw::AddToHandResult` (TS `"hand" | "burned"`, no TS name) — named by me.
- `crate::zones::PlaceOptions` (TS `placeOnField`'s anonymous `{ stack? }`) — named by me; every other
  options object (`moveToZone`'s, `removeFromField`'s, `effectiveCost`'s `CostOptions`) is passed as
  `Default::default()`, so its name does not matter.
- `crate::catalog::GlitchOdds { system_plays }` passed by value inside `Option`.
- `crate::prompts::why_answer_refused` returning `Result<(), EngineError>` (SURFACE §4.4.9) — used
  with `?`; `resume_of` taking `&Resume` (TS took any `{ resume }` holder).
- `crate::prompts::resume_at`'s argument struct and the argument structs of `give_brittle`,
  `set_cost_mod`, `grant_random_keywords`, `damage_all`, `cast_new` must derive `Deserialize`: I build
  them with `prelude::json_as(json!({ … }))` from TS's literal, the type inferred from the parameter.
  `cast_new`'s `def` may be a function in TS; a string `def` must deserialize.
- `targets::PlayerSpec` must be `Copy` (SURFACE §5.1's unit unions), as must `RadiantZone` etc.
- Ownership of lookup answers (`instance_of`, `cards_in_scope`, `card_at`, `permanents_on_field`, …) is
  not assumed: each answer goes through a private `owned(impl Borrow<CardInstance>)`, so a lent
  reference and a handed-over copy both compile.

## Decisions
- Instances: TS handed the live object around and wrote through it. Here a card is an owned snapshot
  (`clone`); a callee that moves the card between zones or creates/places it (`move_to_zone`,
  `remove_from_any_zone`, `place_on_field`, `fresh_face_down_id`, `add_to_hand`,
  `shuffle_into_library`, `take_into_hand`) takes `&mut CardInstance` so the caller's copy follows the
  move; every other callee takes `&CardInstance` and finds the card in the state by id. My own writes
  to a card that stays in the state go through `state::find_instance_mut` (radiant flag, Divine Shield,
  `owner` on a bounce, `summonedTurn`/`statsOverride`/`faceUp` after a summon, `revealed`), and a card
  is read again by id after any call that may have changed it (summon, steal, make radiant).
- TS `ctx.self` was live: where TS reads its current zone or state (`summon_this`, a damage source) I use
  `ctx.live_self()`; where it reads only the id or def id, `ctx.self_`.
- `{ ...ctx, exitsFrom }` (summon's keyword roll) is `ctx.exits_from` set for the one effect and put
  back.
- `playerOf(ctx, spec ?? "self")` is a private `player_or_self` per file (None → `ctx.controller`), so
  no file names `PlayerSpec`'s "self" variant; `playerOf(ctx, "enemy")` is `opponent_of(ctx.controller)`.
- `{ of: "self" }` and `{ of: "instance", instanceId }` are built from JSON (`json_as`), so only
  `TargetSpec::Chosen { index: None }` (SURFACE §7.1's own example) is named.
- Two disjoint sink fields at once (`rng` and `state`) are reached as `ctx.sink.rng` / `ctx.sink.state`
  (Deref would borrow the whole context twice).
- Small helpers duplicated privately instead of called: `copyTuning` (a `.clone()`),
  `enchantments.addEnchantment` (shuffle_random) and `unitedEnchantments` (summon), compared with
  `==` as SURFACE §4.4.3 says; `cardTypeOf`'s partial `{ defId, radiant }` is a scratch instance
  numbered from a local `u32` counter (`new_instance` on the `NextId` for `u32`), so the state's ids
  never move.
- Counts that TS truncated (`Math.trunc`) are `i32` already; picks and quotas over card lists are
  `usize`. `typeof memo === "number"` and the placement bag's numbers read `as_i64`, then `as_f64`.
- Sorting: `addLibraryCopies`' sort by def id is `sort_by` (stable) with `str::cmp`.
