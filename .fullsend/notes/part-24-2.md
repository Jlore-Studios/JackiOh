# Slice: part 24, chunk 2 — engine tests 1 (effect verbs), files 11–18
BUILDS-RUN: 0

## FILES
`crates/engine/tests/rules/{effects_combat, effects_core, effects_cost, effects_counters, effects_cry,
effects_damage, effects_datacenter, effects_delay}.rs` — every TS `describe` (as a `mod`) and every
`it` (as a `#[test]`) in TS order, 112 of 115; the three dropped are effects-core's source-reading
"M3-T1 structural acceptance" tests (`spec-gaps-part-24-2.md`). Header comments and every comment
that states a rule or cites a ruling kept. Notes: this file, `part-24-2.assumptions`,
`spec-gaps-part-24-2.md`.

## GAPS
Tests not ported: `effects-core.test.ts:189`, `:225`, `:240` (they read the source tree; see the
spec-gaps file).

Names called that another part provides (module path, the shape these files assume):

Fixtures (part 24, other chunks), each TS `export const x: CardDef` as `pub fn x() -> CardDef`:
- `crate::rules::fixtures::harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) ->
  GameState`; `put(&mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance` (three arguments,
  as chunk 1 assumes; the two `{ radiant: true }` calls are a local `put_radiant`); `slot(PlayerId,
  Row, i32) -> ZoneSlot`; `in_hand(&mut GameState, &str, PlayerId, count) -> Vec<CardInstance>`
  (count always passed, `HAND_CAP` included); `set_library(&mut GameState, PlayerId, &[String]) ->
  Vec<CardInstance>`; `events_of_type(&[GameEvent], GameEventType)` returning a `Vec` (of
  `GameEvent` or `&GameEvent`: both read the same here).
- `fixtures::combat::{plain, big_body, taunter, armoured, shielded, indestructible, spikey_pillow,
  combat_catalog(CardDefs) -> CardDefs, scripts() -> IndexMap<String, CardScripts>}` (TS
  `COMBAT_SCRIPTS` as `scripts()`, by the brief's step 2).
- `fixtures::scripts::{x_bolt, mana_well, anti_oneshot, stockpile, fixture_catalog(CardDefs) ->
  CardDefs, scripts()}`; `fixtures::catalog::vanilla_catalog(count: i32, from: i32) -> CardDefs`.
- `fixtures::datacenter::{runner, field, hard_field, dying_field, animated_field, trap, field_trap,
  free, one, two, x_cost, cast_on_draw, guard, DYING_FIELD_DAMAGE: i32, GUARD_CAP: i32,
  datacenter_catalog(CardDefs) -> CardDefs, scripts()}`.
- `fixtures::field::{act_result(&GameState, ActionInput) -> ReduceResult, flush(&mut GameState,
  PlayerId, mana: i32), playing(&str) -> GameState, tower}`.
- `fixtures::prompts::{aimer, asker, crier, grave_rewind, moder, rewind, spark, tribute_crier,
  quickdraw_of(&CardDef) -> CardDef}`.
- `fixtures::prompt_harness`: `board(&str) -> GameState`; `resolving_card(&mut GameState, &str,
  PlayerId, radiant: bool) -> CardInstance`; `cast_now(&mut GameState, &str, PlayerId, radiant:
  bool)` (its return is not read); `must<T>(Option<T>, &str) -> T`; `answer_keys(&mut GameState,
  &[&str])` returning a value with `pub events: Vec<GameEvent>` (TS `{ sink, error }`; only the
  events are read); `open_as(&GameState, PromptKind, PlayerId) -> PendingChoice`; `round_trip(&GameState)
  -> GameState`; `event_types(&[GameEvent]) -> Vec<String>`; `act(&GameState, ActionInput,
  Option<&mut Vec<Action>>) -> GameState`; `replayable(&str, &[String], &[String])` returning a
  value with `pub state: GameState, pub log: Vec<Action>, pub decks: (Vec<String>, Vec<String>)`;
  `expect_replays(&str, &(Vec<String>, Vec<String>), &[Action], &GameState)`; `hand_card(&GameState,
  PlayerId, &str) -> CardInstance`.

Engine (parts 2–8), the same shapes chunk 1 assumed wherever both call a name:
- `resolve::{make_context(EngineSink, Option<CardInstance>, HookOptions) -> EffectContext,
  HookOptions { controller, targets, modes, data }` (all `Option`, `Default`), `apply_effects(&[Effect],
  &mut EffectContext)`, `cast_card(&mut EngineSink, &CardInstance, CastOptions: Default)}`.
- `zones::{ZoneSlot { player, row, lane } (pub fields), card_at(&GameState, ZoneSlot) ->
  Option<&CardInstance>, place_on_field(&mut GameState, CardInstance, ZoneSlot, options) -> bool,
  move_to_zone(&mut GameState, &CardInstance, ZoneName, options) -> MoveResult (PartialEq + Debug,
  `MoveResult::Moved`), is_locked(&GameState, ZoneSlot), is_open(&GameState, ZoneSlot),
  first_free_zone(&GameState, PlayerId, Row) -> Option<ZoneSlot>}`; option bags `Default +
  Deserialize` (`{ stack: true }` is passed as `json_as(json!(…))`). TS `OffFieldZone` taken as the
  wire `ZoneName`, as chunk 1 did.
- `mana::{effective_cost(&GameState, &CardInstance) -> i32, modifier_is_live(&GameState,
  &PlayerModifier) -> bool, max_mana_for(&PlayerState) -> i32, refresh_mana(&mut PlayerState)}`.
- `modifiers::{add_modifier(&mut EngineSink, PlayerId, mod)` with `mod` the modifier without its id,
  `Deserialize` from the TS literal (`{ kind, …, expiry }`), `due_delayed(&GameState, Phase, PlayerId)`
  returning a `Vec` of `DelayedEffect` (or refs), `expire_modifiers(&mut EngineSink, PlayerId)}`.
- `combat::{AttackTarget = damage::DamageTarget { Unit { instance: CardInstance }, Hero { player } },
  declare_attack(&mut EngineSink, &CardInstance, &AttackTarget)` (result bound to `_`: TS's
  `CombatResult` is a `Result` by SURFACE §4.4.9), `force_attack(&mut EngineSink, &CardInstance,
  &AttackTarget)}`.
- `prompts::{open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>` with
  `OpenPromptArgs: Deserialize` (built by `json_as`), `run_hook_resumable(&mut EngineSink,
  &CardInstance, &str, options)` with its TS options bag (`controller`, `targets`, `modes`, `data`,
  `exitsFrom`) `Deserialize`, `RESUME_HOOK}`; `triggers::settle(&mut EngineSink, SettleOptions:
  Default)`; `state_check::state_check(&mut EngineSink)`; `animated::animate_card(&mut EngineSink,
  &CardInstance, options: Default) -> bool`; `cry_trigger::TRIGGER_CRY_HOOK`;
  `work::REMEMBERED_KEY: &str`; `scripts::registered_scripts()` and `catalog::registered_catalog()`
  (`.clone()`d to an owned map); testkit `register_catalog(CardDefs)`, `register_scripts(IndexMap<String,
  CardScripts>)`; `view_for`, `hash_state`, `reduce`, `begin_game`, `clone_state` as SURFACE §6.1.

Effects (parts 6–7):
- Every verb takes one argument struct that deserialises from the TS literal (`json_as(json!(…))`):
  `set_cost_mod`, `set_cost_override` (`inHandOnly`), `plague`, `clear_plague`, `lock` (the
  `ZoneSpec` union `{ of: "chosen" | "self" | "lane", player?, row, lane }`), `damage` (`ignoreArmor`,
  `combat`, `lifesteal`), `draw_while_cheap`, `destroy_field_spells_and_hit`, `delay` (`at.player`
  `"self" | "enemy" | THIS_TURN`, `hook`, `data`), `add_player_modifier` (`{ player, mod }`),
  `forced_attacks_on`, `forced_attacks`, `ai_plays_out_turn` (`settleFirst`), `summon`,
  `trigger_cry` (`{ instanceId }`), `draw`, `add_to_hand`, `shuffle_into`, `shuffle_copies_of_self`,
  `lose_health`, `gain_mana`, `refresh_mana`, `next_turn_mana`, `remember`, `remember_random`,
  `switch_position_of`, `switch_all_positions`. A TS call with the argument left out
  (`cancelAttack()`, `clearPlague()`, `addRandomFromGraveyard()`) passes `Default::default()`.
- `effects::{TargetSpec: Deserialize, resolve_target(&EffectContext, &TargetSpec) ->
  Option<DamageTarget>, player_of(&EffectContext, PlayerSpec) -> PlayerId` (`PlayerSpec` from
  `json_as(json!("self"))`), `has_triggerable_cry(&GameState, &CardInstance) -> bool`,
  `field_spells_doomed(&SweepReader, FieldSpellSide) -> Vec<CardInstance>`, `SweepReader<'a> {
  state: &'a GameState, self_: Option<&'a CardInstance>, def_id: Option<&'a str>, radiant: bool,
  controller: PlayerId }` (TS's unexported `Pick<EffectContext, …>`, made pub), `FieldSpellSide {
  Any, Enemy }`, `DELAYED_HOOK`, `THIS_TURN: &str}`.

## Decisions
- No harness `sink_for`: TS `sinkFor(state)` returns a sink that owns its event list and rng, which
  a Rust `EngineSink` cannot. Each file has a private `Bench { state: &mut GameState, events, rng }`
  with `sink() -> EngineSink` and a local `sink_for(&mut GameState)`, built only from part 1's
  frozen `EngineSink::new` and `Rng::new(&state.seed, state.rng_cursor)` — the same choice as
  chunk 1, so the fixtures' harness need not export a sink at all. Where TS wrote the cursor back
  after a run (combat, cry, datacenter, delay), so does the port; where it did not (core, cost,
  counters, damage), neither does it.
- TS's live card objects: read back by id (`find_instance`), written through `find_instance_mut`,
  and refreshed by id before a context is built, so `ctx.self_` is the card as it stands (TS's
  `ctx.self`). A context TS kept across several runs (effects-damage's `ctxFor`) is kept in Rust too
  and the state read through it (`ctx.state`); contexts TS made one after another are scoped one
  after another. effects-core's `context(state, options)`, which returned a context, is
  `with_context(state, options, |ctx| …)`.
- Event assertions: whole lists and objects by serialised JSON; single fields by matching the frozen
  `GameEvent` variants or by the serialised field.
- `it.fails` → `#[should_panic]`; `toMatchObject` → a local `matches_object` over JSON.
- Test names by SURFACE §7.3: R-ids kept as `r<n>` tokens (leading where the title leads with them),
  `§x.y` → `sx_y`, `#n` → `cn`, apostrophes dropped, camelCase split; `describe` titles as `mod`
  names the same way.
- Engine calls take a TS `CardInstance` argument as `&CardInstance`, except `make_context` and
  `place_on_field`, which take the card by value (chunk 1's split).
