# Slice: part 7 chunk 1 of 2, engine 6: effect verbs, second half (#395, parent #306)
BUILDS-RUN: 0

## FILES
All new, each the whole port of its TS file (every function, exported or not, in TS order, header and
rule-stating comments kept):
`crates/engine/src/effects/{last_board,locks,lose_health,memory,player_mods,position,random_picks,rotate,
shuffle_into,statuses,swap,targets,transform,tune,turn_end}.rs`. Nothing left out; no `todo!`,
`unimplemented!` or `// TODO`.

## SURFACE
Matched §4.1 paths, §4.2 names (TS names snake_cased, TS type names kept), §4.3 types, §6.6 effect
shape (`Effect::new(kind, move |ctx| …)`), part 1's frozen types (`EffectContext`, `EngineSink`,
`CardInstance`, `GameEvent`, `PromptOption`, `Resume`, `ModifierExpiry`/`ModifierKind`,
`BrittleCounter`, `Exertion`, `wire::SwapWhat`, `wire::RotationDirection`, `wire::TuningChange`).

## DEPENDS-ON / GAPS
Names I call that other parts provide, with the shape I called them in. Each is my guess from the TS;
part 31 reconciles.

Engine modules:
- `catalog::{def_of(&GameState, &str) -> &CardDef, self_def_ids(&str) -> Vec<String>,
  query(&CatalogQueryArgs) -> Vec<&CardDef>, excluding_def_id(&CatalogQueryArgs, Option<&str>) ->
  CatalogQueryArgs, pick_generated(&mut Rng, &[CardDef], Option<&GlitchOdds>) -> Option<CardDef>,
  CatalogQueryArgs (Deserialize, Default, Clone, PartialEq), GlitchOdds { system_plays: Option<i32> }}`.
- `damage::{DamageTarget::{Unit { instance: CardInstance }, Hero { player }}, lose_health(&mut EngineSink,
  PlayerId, i32)}`.
- `faces::card_type_of(&GameState, &CardInstance) -> CardType`.
- `restrictions::{unaffected_by(&EffectContext, &CardInstance) -> bool, can_go_berserk(&GameState,
  &CardInstance) -> bool}`.
- `stays::{exit_mark(&GameState) -> u32, left_field_after(&GameState, u32, &str) -> bool,
  event_mark(&GameEvent) -> Option<u32>}`.
- `zones::`: `ZoneSlot { player, row, lane: i32 }` (built with a struct literal; a `type ZoneSlot =
  ZoneRef` works too), `adjacent(&ZoneSlot) -> Vec<ZoneSlot>`, `card_at(&GameState, &ZoneSlot) ->
  Option<&CardInstance>`, `carried_units_of(&GameState, PlayerId) -> Vec<&CardInstance>`,
  `active_units_of(..) -> Vec<&CardInstance>`, `zone_contents(&GameState, &ZoneSlot) ->
  Vec<&CardInstance>`, `pile_at(&GameState, &ZoneSlot) -> Option<&Pile>`, `is_buried`, `is_carried`,
  `row_size(Row) -> i32`, `slot_of(&GameState, &CardInstance) -> Option<ZoneSlot>`, `slots_of(PlayerId,
  Row) -> Vec<ZoneSlot>`, `is_locked`/`is_reserved(&GameState, &ZoneSlot)`, `lock_zone`/`unlock_zone(&mut
  GameState, &ZoneSlot)`, `place_on_field(&mut GameState, CardInstance, &ZoneSlot,
  PlaceOnFieldOptions { stack: Option<bool> }) -> bool`, `remove_from_field(&mut GameState,
  &CardInstance, RemoveFromFieldOptions { with_pile: Option<bool> }) -> bool`, `cease_to_exist(&mut
  GameState, &mut CardInstance)`, `replace_in_zone(&mut GameState, &CardInstance, CardInstance) -> bool`,
  `move_to_zone(&mut GameState, CardInstance, OffFieldZone, MoveToZoneOptions { position:
  Option<MoveToZonePosition>, keep_state, .. }: Default)`, `MoveToZonePosition::Index(usize)` (TS
  `"top" | "bottom" | number`), `OffFieldZone::{Hand, Library, Graveyard, Exile}` (Copy, PartialEq),
  `zone_of(&ZoneSlot) -> Zone`.
- `work::{remember_on(&mut IndexMap<String, Value>, &IndexMap<String, Value>, &str, Value), EVENT_KEY}`.
- `modifiers::{add_modifier(&mut EngineSink, PlayerId, ModifierExpiry, ModifierKind), cut_turn_short(&mut
  EngineSink, PlayerId, i32, Option<String>)}` — TS's `DistributiveOmit<PlayerModifier, "id">` passed as
  its two halves.
- `combat::{switch_position(&mut EngineSink, &CardInstance, SwitchPositionOptions { spend_exertion:
  Option<bool>, to: Option<Position> }: Default) -> _, enter_new_side(&mut EngineSink, &CardInstance,
  PlayerId)}`.
- `draw::{add_to_hand(&mut EngineSink, CardInstance), shuffle_into_library(&mut EngineSink,
  CardInstance, bool, Option<&str>)}`.
- `enchantments::{united_enchantments(&[CardInstance]) -> Option<Vec<Enchantment>>,
  add_enchantment(&mut CardInstance, Enchantment) -> bool}`.
- `tuning::{copy_tuning(Option<&Tuning>) -> Option<Tuning>, TUNED_FLOOR, X_KEY, add_step(Option<&IndexMap<
  String, i32>>, &str, i32) -> IndexMap<String, i32>, tidy_tuning(&mut CardInstance), tuned_count(&CardInstance,
  &str, i32) -> i32` (three arguments: no TS caller passes `min`), `tuning_of(&mut CardInstance) -> &mut
  Tuning`, `x_of(&CardInstance) -> i32}`.
- `own_library::hide_from_owner(&mut CardInstance)`.
- `layers::{unit_has(&GameState, &CardInstance, KeywordKind), card_keywords, printed_keywords_of ->
  Vec<Keyword>, stats_with_buffs -> { attack, max_health }, unclamped_attack -> i32, unit_view ->
  UnitView { keywords, health, .. }}`.
- `temporary::is_temporary_card`, `mana::is_x_cost`, `brittle_count::active_brittle_count(&CardInstance)
  -> Option<i32>`.
- `numbers::`: `NumberRef` (tagged on `kind`: `Cost`, `Attack`, `Health`, `Keyword { key }` with
  `key.as_str()`, `Param { key: String }`; Clone, PartialEq, Serialize, Deserialize), `current_stats ->
  Option<{ attack, health }>`, `number_key(&NumberRef) -> String`, `number_on(.., &NumberRef) ->
  Option<i32>`, `number_ref_id(&NumberRef) -> String`, `numbered_keywords_on -> Vec<{ key (as_str), value,
  better (as_str "up"/"down"), printed: Option<i32> }>`, `numbers_on -> Vec<NumberOnCard { id, ref_, label,
  value }>` (field `ref` as `ref_`), `own_cost -> Option<i32>`, `parse_number_ref(&str) -> Option<NumberRef>`.
- `params::{set_param(&mut CardInstance, &str, i32), step_param(&mut CardInstance, &str, i32),
  steppable_params(&GameState, &CardInstance, effects::tune::TuneDirection) -> Vec<{ param: Param, steps,
  delta }>}`.
- `prompts::{OpenPromptArgs { player, kind, aim, prompt, options, min, max, budget, owner, resume },
  open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>, resume_self(&EffectContext, &str,
  IndexMap<String, Value>) -> Resume}`.
- `subsystems::fuse::rebuild_fused_def(&mut GameState, &str, PlayerId) -> Option<CardDef or &CardDef>`,
  `subsystems::last_boards::last_board_candidates(&GameState, PlayerId, &[String]) -> Vec<LastBoardEntry>`,
  `subsystems::rotation::{RotationArgs { direction, perspective, radiant: Option<bool> }, rotate_rings(&mut
  EngineSink, &RotationArgs)}`.

Effects of the other files (part 6, part 7 chunk 2):
- `effects::add_to_hand::add_to_hand`, `effects::buff::buff` (both called with `json_as` of the TS
  literal, so their argument structs' Rust names do not matter), `effects::buff::random_pool_keywords() ->
  Vec<Keyword>`, `effects::choose::chosen_options(&EffectContext) -> Vec<String>`.
- `effects::card_scope::{CardScope (Serialize, Deserialize, Clone, PartialEq), CardsInCardScopeOptions {
  whole_hidden_piles: Option<bool> }, cards_in_card_scope(&EffectContext, &CardScope, CardsInCardScopeOptions)
  -> Vec<ScopedCard { card, matches, .. }>, unreadable_by(&GameState, &CardInstance) -> Vec<PlayerId>}`.
  `CardScope.side` should be `effects::targets::ScopeSide` (TS `BoardScope["side"]`, `sides_of` takes it).
- `effects::move_::bounce_card(&mut EngineSink, CardInstance)`, `effects::summon::clone_of(&mut
  EffectContext, &CardInstance, PlayerId, &SummonCopyArgs) -> CardInstance` (part 7 chunk 2).

## Decisions
- **Live instances.** TS returned live `CardInstance` objects and wrote through them. Every function
  of mine that names a card answers with an owned clone of it as it stands now, and every write finds
  the card again by id (`state::find_instance_mut`); TS reads of `ctx.self` use `EffectContext::live_self()`
  (part 1's), since `slot_of` and the menus read the instance's own fields. Readers I call are assumed to
  return references (`Option<&CardInstance>`, `Vec<&CardInstance>`), cloned with `.cloned()`; functions that
  put a card somewhere (`place_on_field`, `move_to_zone`, `draw::add_to_hand`, `shuffle_into_library`,
  `replace_in_zone`'s replacement, `bounce_card`) take it owned; others take `&CardInstance`.
- **`targets.rs` signatures** (every effect file calls these): `player_of(&EffectContext, PlayerSpec) ->
  PlayerId`, `resolve_target(&EffectContext, &TargetSpec) -> Option<DamageTarget>`,
  `instance_on_its_stay(&EffectContext, &str) -> Option<CardInstance>`, `self_on_its_stay(&EffectContext)
  -> Option<CardInstance>`, `stands_since_script_began(&EffectContext, &str) -> bool`,
  `instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>`, `sides_of(&EffectContext,
  Option<ScopeSide>) -> Vec<PlayerId>`, `matches_scope(&EffectContext, &CardInstance, &BoardScope) -> bool`,
  `cards_in_scope(&EffectContext, &BoardScope) -> Vec<CardInstance>`, `adjacent_to(&EffectContext,
  &TargetSpec, &BoardScope) -> Vec<CardInstance>`. TS's `scope = {}` default is `&BoardScope::default()`.
- **Type names.** `TargetSpec::{SelfCard, SelfHero, EnemyHero, Instance { instance_id }, Chosen { index:
  Option<usize> }}` (tag `of`; "self" renamed); `PlayerSpec::{SelfSide, Enemy}`; `ScopeSide::{Any, SelfSide,
  Enemy}` for `BoardScope["side"]` (also `ZoneScope.side`); `SelfSide` follows part 1's `DrawLimitPlayer`.
  TS anonymous argument objects are `<FnName>Args` (`LoseHealthArgs`, `RememberArgs`, `SwapArgs`,
  `TransformArgs`, `SetNumberArgs`, …, `Default` when every field is optional); TS named argument types keep
  their names (`TransformTarget`, `TuneArgs`, `ZoneScope`, `LaneSpec`, `TuneDirection`, `TuneRow`). An
  anonymous options object of another module's function is `<FnName>Options` (`PlaceOnFieldOptions`,
  `RemoveFromFieldOptions`, `MoveToZoneOptions`, `CardsInCardScopeOptions`). TS unions with literals:
  `LaneSpec { Lane(i32), Card(TargetSpec) }` untagged, `TransformRadiant { Flag(bool), Keep(KeepFace) }`,
  `NumberWhich { Ref(NumberRef), Id(String) }` ("random" is `Id("random")`), `SwitchAllPositionsSide
  { SelfSide, Enemy, Both }`. `addPlayerModifier`'s `mod` is `PlayerModifierSpec { expiry, #[flatten] kind }`
  (field `mod_`, serialised "mod").
- `TuneDirection` and `TuneRow` are plain serde enums with `as_str`, not `string_union!` (which would add
  a ts-rs export of a non-wire type). `SwapWhat` is `pub use crate::wire::SwapWhat` (part 1's note).
  `LAST_BOARD_DISCOVER_OPTIONS`/`LAST_BOARD_CARD_COST` are not redefined: config has them and
  `effects/mod.rs` re-exports them.
- **Cross-part argument structs I cannot see** (`buff`'s, `add_to_hand`'s, `SummonCopyArgs`, a literal
  `CatalogQueryArgs`) are built with `prelude::json_as` from the TS literal, which every effect argument
  type must accept (SURFACE §6.6), so a different Rust field layout still works.
- **tune.rs menus are data.** TS's menu rows and X items were closures over the live card; here they are
  enums (`MenuApply`, `XWrite`) that `apply_row`/`write_x` write onto the card found by id, with the
  same draws in the same order (row, then item, then the stats split). `tune_once` re-reads the card by
  id on every application (TS's live card), so a second application draws from the menu the first
  left. `pick_one` panics with TS's message on an empty row, as TS threw.
- **transform.rs** `replace_off_field` keeps TS's hand reorder exactly, including JS's `splice(-1, 1)`
  (the hand's last card goes) when the replacement is not in the hand. The retired card handed to
  `cease_to_exist` is my clone, as TS's was an object already out of its pile.
- `swap_board_now` calls `enter_new_side` with the card as it now stands in its new zone (TS wrote
  through the object it had placed). `place_contents` panics with TS's message.
- Where a call needs two parts of the context at once, I borrow the fields (`ctx.sink.rng` with
  `ctx.data`, `&mut *ctx.state`) or copy `ctx.controller` into a local first.
- `lock_played_zone` reads the captured event out of `ctx.data[EVENT_KEY]` with
  `serde_json::from_value::<GameEvent>`; a value that is not an event locks nothing (TS's object guard).
- `transform_random` passes `pick_generated` a `GlitchOdds { system_plays }` built from the state (TS
  passed the state, which satisfied `GlitchOdds` structurally).
- TS default arguments: `options = {}` → a struct with `Default`; `tuneOnce`'s `matches = true` and
  `resumeSelf`'s `data = {}` are passed explicitly; `TuneArgs` stands in for `reachedCards`' anonymous
  `{ target?, instanceId?, scope?, random? }`.
