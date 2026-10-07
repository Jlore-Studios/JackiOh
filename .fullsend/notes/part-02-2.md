# Slice: part 2 (engine 1: the model and the board), chunk 2 of 2
BUILDS-RUN: 0

## FILES
All new, each the whole TS file (every function in TS order, its doc comments and every comment
that states a rule or cites a ruling):
`crates/engine/src/{animated, brittle, carriers, catalog, enchantments, kill_credit, layers, numbers,
own_library, ownership, preview, query, restrictions, temporary, times_played}.rs`.
Nothing left.

## GAPS

### Not ported (by the brief or SURFACE)
- `catalog.ts`: the process-global digest table (`digestIngredients`) and `registerFusedIngredients`
  (brief step 2, SURFACE §6.6). A digest id's ingredients are read off its fused definition's
  `ingredients` (found by `find_def(state, id)`). Part 8's fuse must not call `register_fused_ingredients`.
- `restrictions.ts`: `registerAttackBar`, `AttackBar`, `AttackBarAnswer`, the `bars` map and `barred`
  (brief step 5): `cannot_attack`/`cannot_be_attacked` are their flags alone (the registry was always empty).

### Called in other modules (TS name snake_cased at its TS module's path; signature I assumed)
Part 2 chunk 1:
- `scripts::script_of(&GameState, def_id: &str) -> CardScripts` (SURFACE §6.6). Every file here that
  read `scriptOf(instance)`/`flagsOf(instance)` has a private `running_script(state, instance)` (Vanilla →
  `empty_script()`, else the face by `instance.radiant`) over it, so no file depends on how chunk 1 ports
  `scriptOf`/`flagsOf` themselves.
- `faces::card_type_of(&GameState, &CardInstance) -> CardType` (numbers, preview, restrictions, query).
  `restrictions::effect_is_from_spell` and `animated::face_type_of` compute a face's type from
  `def_of` themselves (TS passed `{ defId, radiant }`).
- `brittle_count::{active_brittle_count(&CardInstance) -> Option<i32>, gain_brittle_count,
  give_brittle_count, printed_brittle_of, start_brittle_on_field}` (re-exported by `brittle.rs` as TS did).
- `tuning::tuned_keywords(&[Keyword], &CardInstance) -> Vec<Keyword>`, `tuning::x_of(&CardInstance) -> i32`
  (layers). `numbers.rs` has private copies of `tuned_count` (3 args, floor `TUNE_MIN_AMOUNT`) and
  `numbered_sum`, so TS's optional `min` argument's port does not matter to it.
- `params::params_of(&GameState, &str)` (anything with `.iter()` over `Param`),
  `params::param_value(&GameState, Option<&CardInstance>, key: &str, options) -> i32` with the options
  argument passed as `Default::default()` (works for `Option<_>` or a `Default` options struct).
- `plague::{permanents_on_field, plague_multiplier_of, plague_on, plague_on_field}` (re-exported by `query.rs`).
- `announce::is_announce_live(&GameState, &str) -> bool`; `stays::left_field_after(&GameState, u32, &str) -> bool`.
- `zones`: `ZoneSlot { player, row, lane }` (a struct, or an alias of `wire::ZoneRef`);
  `OffFieldZone::{Hand, Library, Graveyard, Exile}`; `slots_of(PlayerId, Row) -> Vec<ZoneSlot>`;
  `card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>`; `slot_of(&GameState, &CardInstance) ->
  Option<ZoneSlot>`; `active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>` (owned also works);
  `is_buried`, `acts_on_field`, `is_carried` `(&GameState, &CardInstance) -> bool`;
  `is_open(&GameState, &ZoneSlot) -> bool`; `first_entry_zone(&GameState, PlayerId, Row) -> Option<ZoneSlot>`;
  `home_of(&GameState, &str) -> Option<&HomeZone>` (owned also works); `reserve_home(&mut GameState,
  &ZoneSlot, &str)`; `release_home(&mut GameState, &str)`; `step_into_unit_zone(&mut GameState,
  &CardInstance, &ZoneSlot) -> bool`; `step_into_backrow(&mut GameState, &CardInstance, &ZoneSlot, stack: bool) -> bool`
  (TS `{ stack?: boolean }` taken as a bool). The two step functions are expected to find the card by
  its id (the caller holds a copy).
- Readers I copied privately rather than call (rule 5; their comments name the zones reader they mirror):
  `layers` (unit-zone top, aura sources straight off the rows as `active_units_of` + backrow),
  `brittle` (ticked cards), `carriers` (`carried_at`, backrow top, `is_carrier`).

Part 3:
- `resolve::make_context(EngineSink<'a>, self_: Option<CardInstance>, HookOptions) -> EffectContext<'a>` and
  `resolve::HookOptions { controller: Option<PlayerId>, .. }: Default` (brittle's crumble).
- `damage::DamageTarget::{Hero { player }, Unit { instance }}` as struct variants (restrictions, query).
- `work::part_memory_key(&IndexMap<String, Value>, &str) -> String` (query::recalled).
Part 4:
- `draw::add_to_hand(&mut EngineSink, &CardInstance)` (its return is not read: `take_into_hand` decides
  hand/burned by TS's own `hand.length >= HAND_CAP` test before the call); `draw::draw_blocked(sink, PlayerId)
  -> bool` (sink passed as `&mut`, coerces to `&`); `draw::complete_draw(&mut EngineSink, PlayerId,
  CardInstance /* owned: spliced out of the library */, None /* link */) -> DrawOutcome`;
  `draw::DrawOutcome::Limited`.
- `mana::{is_x_cost, printed_cost(&GameState, &CardInstance) -> i32, play_cost, modifier_is_live(&GameState,
  &PlayerModifier) -> bool}`; `play_choices::gifted_makes_radiant(&GameState, PlayerId, i32) -> bool`.
Part 5: `testkit::scenario::catalog_override() -> Option<&'static CardDefs>` (SURFACE §8's thread-local
  override; `'static`, e.g. a leaked box per registration, since `registered_catalog()` hands out
  `&'static`). Called only under `#[cfg(feature = "testkit")]`. `catalog_version()` answers TS's "test"
  while it is in force.
Part 6: `effects::destroy::destroy(<args deserialisable from { target: { of: "instance", instanceId } }>) -> Effect`
  (built with `json_as`, so the Rust arg type's name does not matter).
Part 7: `effects::move_::discard_from_hand(&mut EngineSink, &CardInstance)`.
Part 8: `subsystems::copied_text::{running_script_of(&GameState, &CardInstance) -> Script,
  text_face_of(&GameState, &CardInstance) -> CardInstance}` (a `&CardInstance` return also compiles);
  `subsystems::lethal::is_lethal(&GameState, &CardInstance, &AttackTarget) -> bool`.

### Signatures other parts call that differ from TS's arity (part 31: fix call sites, not these)
- `catalog::{fused_id_specs, fused_id_parts, self_def_ids}(state: Option<&GameState>, def_id)` and
  `catalog::excluding_def_id(state: Option<&GameState>, &CatalogQueryArgs, def_id: Option<&str>)`: the
  state is the digest table's replacement. Callers: params, targeting, play_counts, effects/{transform,
  shuffle_random, add_to_hand, last_board, cast, summon, choose, fruit, fuse}, subsystems/{last_boards,
  glitch, perfect_hand, copied_text, fuse}. Pass `Some(state)` (or `None` where TS's caller had no state:
  then a digest id reads as no fused id).
- `catalog::{find_def, def_of}(Option<&GameState>, &str)` (brief); `catalog::query(&CatalogQueryArgs)`
  (TS's `{}` default is `&CatalogQueryArgs::default()`); `catalog::pick_generated(&mut Rng, &[&CardDef],
  Option<&GameState>)`; `catalog::roll_grape(&mut Rng, lucky: i32)` (TS default 0).
- A TS function that wrote through a live card and read the state takes the state (or sink) mutably
  and `card: &CardInstance`, the card as the caller holds it, re-read and written by id:
  `times_played::count_play(&mut GameState, &CardInstance)`, `animated::{animate_card(sink, &card,
  AnimateOptions), return_home, animate_on_entry}`, `ownership::{change_owner, take_into_hand}`.
  Pure card writes keep `&mut CardInstance`: `own_library::{show_to_owner, hide_from_owner}`,
  `enchantments::add_enchantment(&mut CardInstance, &Enchantment)`.
- Readers TS gave only the card but whose flags need the state (fused scripts): 
  `restrictions::attackable_only_from_lane(&GameState, &CardInstance)`.
- `query::{played_earlier, was_played_this_turn}(state, player, impl Into<CardOrId>)` take a
  `&CardInstance`, `Option<&CardInstance>`, `&str` or `&String` (TS `CardInstance | string | null`).
- `query::played_this_turn_of_type(state, player, &[CardType])` (TS one or several);
  `query::fusable_permanents_of(state, player, Option<&str>)`; `query::left_field_since_resolved(state,
  &GameEvent)` (false for any event but `CardResolved`); `query::recalled(&EffectContext, key) -> Option<Value>`.
- `ownership::take_into_hand -> Option<TakenTo>` (`Hand`/`Burned`); `ownership::draw_from_library_of(sink,
  drawer, from, LibraryEnd) -> Option<DrawOutcome>` (`LibraryEnd::default()` is TS's "bottom").
- `restrictions::{effect_is_from_spell, unaffected_by}(&EffectContext, …)`; `attack_restriction(state,
  attacker, &DamageTarget) -> Option<&'static str>`; `is_spell_source`/`spell_cannot_reach` take
  `source: Option<&CardInstance>`.
- `enchantments::{enchantments_of_kind, has_enchantment}(card, EnchantmentKind)` (a new enum for TS's
  `Enchantment["kind"]`, with `enchantment_kind(&Enchantment)`); `united_enchantments(impl IntoIterator
  of Borrow<CardInstance>) -> Option<Vec<Enchantment>>`.
- `preview::preview_of(state, card, viewer, ConditionZone) -> Option<Vec<PreviewValue>>`.

## Decisions
- `catalog`: one `OnceLock<{defs, version}>`; `register_catalog` ignores a second call (TS replaced the
  registry; tests use the testkit override). Before registration `registered_catalog()` is an empty map
  and `catalog_version()` "0", as TS. `query` returns `Vec<&'static CardDef>` (no clones; the sort is
  `sort_by`, stable, TS's comparator: set rank, index as a float compared not subtracted, index, id).
  `index_rank` is `parse::<f64>()` with a non-finite or failed parse as `+∞` (SURFACE §4.4.4).
  `CatalogQueryArgs` writes the wire `CatalogQuery`'s fields out inline (camelCase serde, so a card's
  `json!` literal deserialises) plus `def_id`, `token`; `From<CatalogQuery>`. `GlitchOdds` is a type
  alias of `GameState` (every TS caller passed the state). `TransientHolder` likewise.
- `fused_id_specs`' readable-id parse walks bytes (ids are ASCII at every split point); `/^t-\d+:/` is
  `fused_head_len`.
- `layers`: computed on read, never cached (SPEC §10.4); TS's anonymous returns are named `FaceStats`
  (`face_of`) and `BuffedStats` (`stats_with_buffs`). `UnitView`/`FaceStats`/`BuffedStats` derive serde but
  no `TS` (not wire; `UnitView` would collide with the wire one's generated file).
- Enums that are TS string unions but not wire (`NumberedKey`, `AnimatedKind`, `EnchantmentKind`,
  `TakenTo`, `LibraryEnd`) are written by hand with serde renames, not with `wire::string_union!`, which
  would add `ts-rs` exports to `apps/web/src/wire/generated/` and break the V20 diff.
- `NumberRef` is `#[serde(tag = "kind")]`; `NumberOnCard.ref_` serialises as `ref`; `NumberedKeyword.printed`
  is `Option<i32>` serialised `null` (TS `number | null`).
- `kill_credit::credited_killer_id` reads the memory bag defensively (array of `{victimId, toId}`; a
  non-string `toId` is no credit).
- `own_library::compare_text` compares UTF-16 code units (names may carry non-ASCII).
- `carriers`: `delete memory[key]` is `IndexMap::shift_remove` (keeps the other keys' order).
- `brittle`: the tick reads the ticked cards once as copies, re-reads each by id before it ticks, and
  writes by id; `crumble` builds its context with `make_context(sink.reborrow(), None, …)`.
- `ownership::draw_from_library_of` keeps TS's second `draw_blocked` check (a pure read, no rng).
- `restrictions`/`query`: `ctx.live_self()` stands in for TS's live `ctx.self` (a card's type and memory as
  they are now).
