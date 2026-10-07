# Slice: part 24, chunk 1 — engine tests 1 (effect verbs), files 1–10
BUILDS-RUN: 0

## FILES
`crates/engine/tests/rules/{effects_after_check, effects_animate, effects_boardwide, effects_brittle,
effects_buff, effects_card_scope, effects_cast_chaos, effects_cast, effects_choose_where,
effects_choose}.rs` — every TS `it` (94) and `describe` (23) ported in TS order, header comments and
ruling/§ comments kept. Notes: this file, `part-24-1.assumptions`, `spec-gaps-part-24-1.md`.

## GAPS
Tests not ported: none (see `spec-gaps-part-24-1.md` for the assertions written against the nearest
Rust observable).

Names called that another part provides (module path, the shape these files assume):

Fixtures (part 24, other chunks):
- `crate::rules::fixtures::harness`: `new_game(seed: &str, decks: Option<(Vec<String>, Vec<String>)>)
  -> GameState`; `put(&mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance` (three arguments:
  the `{ radiant }` option is never passed by these files); `slot(PlayerId, Row, i32) -> ZoneSlot`;
  `in_hand(&mut GameState, def_id: &str, player: PlayerId, count) -> Vec<CardInstance>` (count
  always passed, TS default 1); `set_library(&mut GameState, PlayerId, &[String]) ->
  Vec<CardInstance>`; `events_of_type(&[GameEvent], GameEventType)` returning a `Vec` (only `.len()`
  and `.is_empty()` are used; field reads match `GameEvent` directly).
- `fixtures::combat::{plain, shielded, spikey_pillow, taunter, zero_attack, stacker, armoured,
  indestructible}`, `fixtures::field::{banner, cover, golem, springer, tower}`,
  `fixtures::instance_data::{brittle_unit, brittle_trap, blood_moon, echo_bolt, tesla}`,
  `fixtures::play_pipeline_b::{ask_target, cast_field, cast_trap, discover_spell, grave_spell,
  jogg_box, mode_spell, named_caster, solarius, target_spell, their_choice, tyrant, x_spell,
  x_target}`: each TS `export const x: CardDef` as `pub fn x() -> CardDef` (called `x().id`).
- `fixtures::field::playing(seed) -> GameState`, `fixtures::instance_data::instance_game(seed) ->
  GameState`, `fixtures::catalog::token_def(name) -> CardDef`.
- `fixtures::play_pipeline_b::{DISCOVER_POOL, RANDOM_POOL}: &[&str]`, `pb_playing(seed) ->
  GameState`, `pb_act(&GameState, body: Value) -> GameState`, `pb_reduce(&GameState, body: Value) ->
  ReduceResult` (body = the TS `ActionInput` literal), `in_graveyard(&mut GameState, def_id: &str,
  PlayerId) -> CardInstance`. `only` and `roundTrip` are written locally (trivial).

Engine (parts 2–8):
- `resolve::make_context(EngineSink, Option<CardInstance>, HookOptions) -> EffectContext`;
  `resolve::HookOptions { controller, targets, modes, data }` all `Option`, `Default`;
  `resolve::apply_effects(&[Effect], &mut EffectContext)`; `resolve::cast_card(&mut EngineSink,
  &CardInstance, CastOptions)` with `CastOptions: Default + Deserialize`.
- `triggers::settle(&mut EngineSink, SettleOptions)` (`Default`); `state_check::state_check(&mut
  EngineSink)`, `state_check::DEATHS_WORK`; `work::owed_work(&GameState, Option<&str>) ->
  Vec<WorkItem>`.
- `prompts::open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>` with
  `OpenPromptArgs { player, kind, aim, prompt, options, min, max, budget, owner, resume }` (TS
  fields, `Option` for the optional ones); `prompts::resume_self(&EffectContext, step: &str,
  data: IndexMap<String, Value>) -> Resume`.
- `query::zone_cards(&GameState, PlayerId, ZoneName)` (TS `OffFieldZone` taken as the wire
  `ZoneName`).
- `zones::{ZoneSlot, card_at(&GameState, ZoneSlot) -> Option<&CardInstance>, place_on_field(&mut
  GameState, CardInstance, ZoneSlot, options) -> bool, move_to_zone(&mut GameState, &CardInstance,
  ZoneName, options), active_units_of(&GameState, PlayerId)}`; the option bags are `Default +
  Deserialize` (`{ stack }`, `{ position, keepState }`).
- `layers::{unit_view(&GameState, &CardInstance) -> UnitView, unit_has(&GameState, &CardInstance,
  KeywordKind) -> bool}`; `damage::{deal_damage(&mut EngineSink, DamageArgs) -> i32, DamageArgs {
  source: Option<CardInstance>, target: DamageTarget, amount, flags: Option<_> },
  DamageTarget::Unit { instance }}`.
- `view_for::{view_for, HIDDEN_ID}`; `scripts::registered_scripts()` (owned or `&IndexMap<String,
  CardScripts>`, `.clone()`d); testkit `register_catalog(CardDefs)`, `register_scripts(IndexMap<String,
  CardScripts>)`.
- `random_cast::{cast_modes_of(&GameState), prefer_enemies, prefer_friends}` — the last two as
  `(&GameState, PlayerId, &[T], impl Fn(&T) -> Selection, required) -> Vec<T>`.
- `subsystems::call_to_chaos::{cast_random_call_to_chaos() -> Effect, chaos_chain_of(Option<&CardInstance>)
  -> i32}`.

Effects (parts 6–7):
- Every data-argument verb takes one struct that deserialises from the TS literal (`json_as(json!(…))`):
  `destroy`, `destroy_all`, `destroy_adjacent_to`, `damage`, `damage_all`, `heal`, `gain_mana`,
  `summon`, `animate`, `buff`, `buff_all_units`, `grant_keyword`, `grant_random_keywords`,
  `give_brittle`, `gain_brittle` (the `{instanceId}`/`{target}`/`{scope}` union), `bounce_all`,
  `exile_all`, `exile_adjacent_to`, `discard_hand`, `discard_random`, `exile_hand`, `cast`, `cast_new`,
  `cast_random`, `choose_mode`, `choose_target`, `choose_from_hand`, `choose_cell`,
  `discover_from_catalog`, `discover_from_graveyard`. Fields TS types as value-or-function
  (`castRandom.query`/`count`, `castNew.def`, `discoverFromCatalog.query`) must accept the plain value.
- Closure-carrying args by struct literal: `ForEachCardArgs { cards: Arc<dyn Fn(&mut EffectContext)
  -> Vec<String>>, each: Arc<dyn Fn(&str) -> Effect> }` (TS `(CardInstance | string)[]` taken as ids);
  `ChooseTargetWhereArgs { step: String, scope: Option<TargetScope>, where_: Arc<dyn
  Fn(&EffectContext, Option<&CardInstance>) -> bool>, prompt: Option<String>, data: Option<…> }`.
- `after_state_check(impl Fn(&mut EffectContext) -> Vec<Effect>)` (a closure, not a `Hook`).
- `effects::{TargetScope, CardScope}` (`Default`/`Deserialize`), `targets_in_scope(&EffectContext,
  &TargetScope) -> Vec<Selection>`, `chosen_options(&EffectContext) -> Vec<String>`,
  `cards_in_card_scope(&EffectContext, &CardScope, options) -> Vec<ScopedCard>` with `.card`,
  `.readers` (serialises "everyone"/"owner"/"nobody"), `.matches`; `readers_of`, `unreadable_by`.

## Decisions
- TS `sinkFor(state)` cannot return a sink that owns its rng, so each file builds one from the
  frozen types: `Rng::new(&state.seed, state.rng_cursor)` + `EngineSink::new`. Where TS kept one
  sink for a whole test (buff, boardwide), a local `Bench`/`Runner` owns the state, the event list
  and the rng; where TS wrote the cursor back after a run (brittle, cast, cast-chaos, boardwide), so
  does the port, and where it did not (choose, choose-where, card-scope, animate), neither does it.
- An instance a test hands to the engine is passed as an owned `CardInstance` to `make_context` (the
  context keeps a snapshot) and to `place_on_field` (it is not in the state yet), and as
  `&CardInstance` to `move_to_zone`/`cast_card` (it is). TS's live object is always read back from
  the state first, so the snapshot is the card as it stands.
- Effect argument structs are built with `json_as(json!(TS literal))`, so the tests do not depend on
  the Rust names of TS's anonymous argument types. Empty option bags pass `Default::default()`.
- Event assertions: counts through `events_of_type`; fields by matching `GameEvent` variants (frozen);
  `toEqual` on event lists and view parts by serialised JSON.
- Test names: R-ids in a title are kept as `r<n>` tokens where the title has them (leading where the
  title leads with them); `§x.y` becomes `sx_y` in `mod` names and is kept in a comment; `#26`
  becomes `c26`.
- `nextIndex` counters (TS module `let`s) are written out as the index each def gets.
- Nonces: `effects-after-check`'s module counter is a `static AtomicU32` (not banned by clippy.toml).
