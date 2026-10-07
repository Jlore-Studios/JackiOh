# Slice: part 8 (engine 7: subsystems), chunk 2 of 3 (#396)
BUILDS-RUN: 0

## FILES
All six are full ports of their TS files, every function in TS order, every R-id and § citation kept
(checked by script against the TS sources):
- `crates/engine/src/subsystems/call_to_chaos.rs` ← `callToChaos.ts`
- `crates/engine/src/subsystems/copied_text.rs` ← `copiedText.ts`
- `crates/engine/src/subsystems/hero_power.rs` ← `heroPower.ts`
- `crates/engine/src/subsystems/ky_test.rs` ← `kyTest.ts`
- `crates/engine/src/subsystems/lethal.rs` ← `lethal.ts`
- `crates/engine/src/subsystems/scorer.rs` ← `scorer.ts`

## SURFACE
Matched §4 (paths, names, types), §4.4 (stable sorts, insertion-ordered maps, presence, RNG order),
§6.5 (`&mut EngineSink`; scorer's `dryRunning` on the sink) and §6.6 (effects as `Effect::new`
closures). Constants come from `crate::config`; none is redefined (subsystems/mod.rs re-exports them).

## DEPENDS-ON
Names called in other parts' modules, by TS name snake_cased, with the shape assumed:
- `crate::resolve::{lazy_part(kind: &'static str, expand: impl Fn(&mut EffectContext, &Memo) -> EffectPart + Send + Sync + 'static) -> Effect, cast_card(&mut EngineSink, CardInstance, &CastOptions), CastOptions: Default}`
- `crate::triggers::{settle(&mut EngineSink, &SettleOptions), SettleOptions: Default}`
- `crate::play_steps::run_play_steps(&mut EngineSink, PlayerId, &PlayAction) -> Result<(), EngineError>`
- `crate::play_choices::{PlayAction, play_actions_for(&GameState, PlayerId, &CardInstance) -> Vec<PlayAction>}`
- `crate::catalog::{def_by_index(SetName, &str) -> Option<CardDef or &CardDef>, query(&CatalogQueryArgs) -> Vec<CardDef>, query_cost(&CardDef) -> i32, pick_generated(&mut Rng, &[CardDef], Option<_>) -> Option<CardDef or &CardDef>, fused_id_parts(&str) -> Option<Vec<String>>, def_of(Option<&GameState>, &str)}`
- `crate::scripts::{script_of(&GameState, &str) -> CardScripts (part 2's brief), registered_scripts() -> map with .get(&str) -> Option<&CardScripts>}`
- `crate::damage::{DamageTarget::{Unit { instance }, Hero { player }}, hero_hit_amount(&GameState, PlayerId, i32, bool) -> i32, pierces(&GameState, &CardInstance) -> bool}`; `crate::combat::{AttackTarget (= DamageTarget), can_attack(&GameState, &CardInstance, &AttackTarget) -> bool}`
- `crate::layers::unit_view(&GameState, &CardInstance) -> UnitView { attack, max_health, health, keywords, armor, .. }`
- `crate::zones::{active_units_of, slot_of, adjacent(&ZoneSlot), card_at(&GameState, &ZoneSlot), first_free_zone(&GameState, PlayerId, Row), slots_of(PlayerId, Row), is_locked(&GameState, &ZoneSlot), is_reserved(&GameState, &ZoneSlot)}` (`ZoneSlot` with a `lane: i32` field)
- `crate::faces::card_type_of(&GameState, &CardInstance) -> CardType`; `crate::mana::cost_now(&GameState, &CardInstance) -> i32`; `crate::params::param(&EffectContext, &str) -> i32`; `crate::stays::left_field_after(&GameState, u32, &str) -> bool`
- `crate::subsystems::activate::{ACTIVATIONS_MEMORY_KEY, abilities_of(&GameState, &CardInstance) -> Vec<ActivationDecl>, uses_allowed(&CardInstance, &ActivationDecl) -> i32, uses_this_turn(&GameState, &CardInstance) -> i32}` (chunk 1 or 3 of part 8)
- effects (parts 6–7), each taking its argument struct built with `json_as(json!({…}))` from TS's literal: `summon`, `summon_random`, `recruit`, `heal`, `draw`, `gain_mana`, `add_to_hand`, `add_random_from_catalog`, `set_radiant`, `set_cost_mod`, `damage`, `set_number`, `draw_from_opponent`, `fuse_cards`, `gain_hero_armor`, `add_player_modifier`, `discover_from_catalog`, `choose_mode`, `choose_answer`; and `TargetSpec::Chosen { index: None }`, `resolve_target(&EffectContext, &TargetSpec) -> Option<DamageTarget>`, `cards_in_scope(&EffectContext, &BoardScope) -> Vec<CardInstance or &CardInstance>`, `chosen_options(&EffectContext) -> Vec<String>`, `answered_correctly(&EffectContext) -> bool`, `after_state_check(impl Fn(ctx) -> Vec<Effect> + Send + Sync + 'static) -> Effect`.

## GAPS
- None of the six files leaves a function out.
- `PlayAction` is assumed to be `ActionBody` (TS `Extract<ActionBody, { type: "play" }>` has no struct
  in the frozen wire, whose `Play` is a struct variant). `scorer.rs` reads the action through one
  private accessor, `play_fields`, so a struct `PlayAction` from part 4 changes only that function.
- TS `SettleSink = EngineSink & { dispatched? }`: the dry run hands `settle` a plain `EngineSink`
  (the frozen sink has no `dispatched`); part 3 decides where that position lives.
- The effect argument structs are reached only through `json_as`, so their names are free; a
  field that holds a function (`forEachCard`'s `cards`/`each`) is avoided: `hero_power.rs` carries a
  private copy of `effects::forEachCard` (rule 5), same kind (`"forEachCard"`) and memo.
- The Classic+ #73 table (`call_to_chaos_plus.rs`, another chunk) must be a
  `&'static [ChaosEffectDef]` with `fn() -> Effect` builders to pass as `CallToChaosArgs.table`.
- Card #98 (cards lane) wires `resume` as `(POWER_RESUME, hook(subsystems::hero_power))`,
  `activations: subsystems::power_abilities(radiant)`, `start_of_game: hook(|_| vec![subsystems::roll_power()])`.
  Card #95/#73: `subsystems::call_to_chaos(CallToChaosArgs { radiant: Some(..), table: .. })`.
  Card #97: `subsystems::top_three(state, controller, &ScorerOptions { radiant: Some(..) })`.
  Classic+ #42: `subsystems::ky_test_script(&bank)` → `KyTestScript { cry, resume }`.
- `perfect_hand.rs` (another chunk) calls `score_def(state, viewer, def, &ScorerOptions, base.as_mut())`
  with `base = dry_run_base(state, viewer)` (an `Option<GameState>`).

## Decisions
- Tables are `const` slices of plain data with `fn` pointers (`HERO_POWERS: &[HeroPower]`,
  `CHAOS_EFFECTS: &[ChaosEffectDef]`), not `Arc` hooks, so they stay `const` and the WASM
  `engine_tables()` can read them; `HeroPower.targets` is `Option<fn() -> Vec<TargetDecl>>` (a
  `TargetDecl` holds `Vec`s, which a `const` cannot build). `HeroPowerName` and `ChaosEffectName`
  are hand-written enums (serde camelCase, `ALL`, `as_str`), not `string_union!`, so they export no
  client TS type (neither is a wire type; `HeroPowerView.name` is a string).
- `hero_power` (TS `heroPower: Hook`) is a plain `fn(&mut EffectContext) -> Vec<Effect>`; cards wrap
  it with `hook(...)`.
- TS wrote through the live `ctx.self` in `refreshPower` and `rollPower`: Rust writes the memory
  key to the live card (`find_instance_mut`) and to the context's own `self_` copy; reads use
  `ctx.live_self()`. `ensure_power(&mut EngineSink, &mut CardInstance)` keeps TS's shape; `roll_power`
  runs it on a clone of the live card and writes `memory.power` back.
- Scorer's `dryRunning`: the trial sink sets `EngineSink::dry_running = true` (brief step 3), and the
  re-entry guard reads the state: a state carrying `HIDDEN_CARD_DEF_ID` in `transient_defs` is a dry
  run's copy (every copy carries the stand-ins `concealFrom` writes; no real state does), so a nested
  `rank`/`top_three` from a card played in the dry run, whose context state is the trial, scores on
  printed data exactly as TS's flag made it. Public signatures stay TS's (no flag argument).
- Scorer's default `base = dryRunBase(state, viewer)` becomes an explicit `base: Option<&mut GameState>`
  on `dry_run` and `score_def` (shared and mutated across one ranking, as TS's object was: its
  `nextId` advances per candidate). `compare_scored` returns `Equal` when two index ranks are both
  infinite (TS returned `NaN`, which `sort` reads as 0).
- `copies_text(card)` keeps TS's one argument and reads the registry with `registered_scripts()`
  (a fused id answers false before any lookup, so no state is needed); every other read goes through
  `scripts::script_of(state, def_id)` with `scriptOf`'s Vanilla guard and face pick done locally.
- `fix_copied_text(state: &mut GameState, card_id: &str, record: Option<Option<PlayRecord>>)`: the card
  by id, written in place (both TS call sites hold the card in the state); `record` is TS's
  `record?: PlayRecord | null` (outer `None` = absent).
- `text_face_of` returns an owned `CardInstance`; `running_script_of` an owned `Script`.
- `ky_test_script(bank: &[KyTestProblem]) -> KyTestScript { cry: Hook, resume: IndexMap<&'static str, Hook> }`;
  `KyTestProblem` has `String` fields and serde camelCase (the bank may be JSON); `KyTestDifficulty`
  is `config`'s, `pub use`d. `json!` objects are BTreeMap-ordered (no `preserve_order`); every map the
  module writes is read by key and hashed canonically, so the order is not observable.
- `onInstance` swaps `ctx.targets` for the one selection around the inner `apply` and restores it
  (TS spread a fresh context object).
- `roll_chaos_effects(rng, radiant, table: Option<&[ChaosEffectDef]>)`; drawn entries are matched by
  name (unique within a list) where TS matched identity.
- Lethal and the scorer take whatever `active_units_of`/`card_at` hand back (owned or borrowed): the
  code passes them to functions (deref coercion) and clones where it keeps one.
