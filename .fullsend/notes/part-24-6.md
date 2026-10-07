# Slice: part 24 (engine tests 1), chunk 6 of 7: the first fifteen engine test fixtures
BUILDS-RUN: 0

## FILES
All under `crates/engine/tests/rules/fixtures/`, each a whole port of its `packages/engine/test/fixtures/*.ts`
(every export, every rule- or ruling-citing comment, the TS header comment as `//!`):
`activate.rs`, `board_history.rs`, `call_to_chaos_plus.rs`, `catalog.rs`, `combat.rs`, `copied_text.rs`,
`core_patches.rs`, `damage_combat.rs`, `datacenter.rs`, `field.rs`, `fruit.rs`, `generation.rs`, `harness.rs`,
`instance_data.rs`, `kill_credit.rs`. Fixture files hold no `describe`/`it`, so no test is ported here and
none is dropped (`spec-gaps-part-24-6.md`: `TESTS-IN: 0`, empty list).

## SURFACE
Each file starts `use jackioh_engine::testkit::*;` and imports the effect verbs it uses from
`jackioh_engine::effects` (the testkit does not glob them). Registration goes through the testkit's
thread-local `register_catalog(CardDefs)` / `register_scripts(IndexMap<String, CardScripts>)` on top of
`registered_catalog().clone()` / `registered_scripts().clone()` (TS `{ ...registered(), ...mine }`, same key
order: an existing key keeps its place). Names are TS's snake_cased (SURFACE §4.2).

## DEPENDS-ON
See GAPS: every engine and sibling-fixture name these files call, with the shape assumed.

## GAPS
No test was left unported (these files hold none).

### Names called that another part provides (TS name snake_cased at its TS module's Rust path)
- Sibling fixture, part 24 chunk 7 (`fixtures/scripts.rs`): `FIXTURE_SCRIPTS` (anything whose `.clone()` is
  an `IndexMap<String, CardScripts>`; this chunk wrote its own maps as `LazyLock<IndexMap<…>>` statics) and
  `fixture_catalog(CardDefs) -> CardDefs` (TS `fixtureCatalog(base)`, the base passed explicitly). Used by
  `harness::setup_catalog`, `harness::catalog`, `harness::scripts`.
- testkit (part 5): `register_catalog(CardDefs)`, `register_scripts(IndexMap<String, CardScripts>)`.
- `catalog::registered_catalog() -> &CardDefs`; `scripts::registered_scripts()` (owned or `&`: called with `.clone()`).
- `zones::ZoneSlot { player, row, lane: i32 }` (built with a struct literal), `zones::place_on_field(&mut
  GameState, &mut CardInstance, &ZoneSlot, <options: Default>) -> bool`, `zones::slots_of(PlayerId, Row) ->
  Vec<ZoneSlot>`, `zones::card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>`,
  `zones::active_units_of(&GameState, PlayerId)` (`Vec<&CardInstance>` or `Vec<CardInstance>`; both compile).
- `own_library::show_to_owner(&mut CardInstance)`.
- `reduce::{reduce(&GameState, &Action), begin_game(&GameState)} -> ReduceResult { state, events, error:
  Option<String> }`, `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`, `seat_to_act(&GameState) ->
  Option<PlayerId>` (SURFACE §6.1; `unwrap_or(state.active)`, TS's own last fallback).
- `replay::hash_state(&GameState) -> String`.
- `prompts::open_prompt(&mut EngineSink, OpenPromptArgs)` with `OpenPromptArgs: Deserialize` (built with
  `json_as` from TS's literal), `prompts::resume_self(&EffectContext, &str, <data: Default>) -> Resume`
  (TS's `data = {}` passed as `Default::default()`), `prompts::RESUME_HOOK: &str`.
- `params::param(&impl ParamContext, &str) -> i32` for `EffectContext` (`param(&*ctx, …)`) and
  `ConditionContext` (`param(&c, …)`).
- `query::recalled(&EffectContext, &str)` (`Option<Value>` or `Option<&Value>`; both compile),
  `query::killer_of(&GameState, Option<&CardInstance>) -> Option<&CardInstance>`.
- `work::part_memory_key(&IndexMap<String, Value>, &str) -> String`.
- `combat::after_attack_of(&EffectContext) -> Option<AfterAttackFacts { target_id, destroyed_ids:
  Vec<String>, survived: bool, forced: bool }>`.
- `replacements::replacement_of(&EffectContext) -> Option<ReplacementRecord { redirected_to:
  Option<PlayerId>, flickered: Option<Vec<FlickeredCard { instance_id }>> }>`.
- `restrictions::is_berserk(&CardInstance) -> bool`; `plague::plague_on(&CardInstance) -> i32`;
  `cast_on_draw_now::is_cast_on_draw(&GameState, &CardInstance) -> bool`;
  `kill_credit::KillCredit { victim_id: String, to_id: String }` (struct literal).
- `subsystems::activate::activation_paid(&EffectContext) -> ActivationPaid { tributed: Vec<TributedUnit {
  attack: i32 }> }`; `subsystems::copied_text::copied_text_of(&GameState, &CardInstance) -> Option<PlayRecord>`;
  `subsystems::hero_power::{hero_power (called as `hero_power(ctx)` with `&mut EffectContext`),
  power_abilities(bool) -> Vec<ActivationDecl>, roll_power() -> Effect, POWER_RESUME: &'static str,
  STEADY_SHOT_PARAM: &str}`.
- `effects::WithKillCreditArgs { killer: TargetSpec, pairs: Arc<dyn Fn(&EffectContext, &CardInstance) ->
  Vec<KillCredit> + Send + Sync>, transfer: bool, during: Effect, then: Option<Arc<dyn Fn(&KillCredit) ->
  Vec<Effect> + Send + Sync>> }` for `with_kill_credit` (TS's inline argument type, which holds functions,
  so it cannot come from JSON; the name is this chunk's guess).
- Every other effect verb is called with `json_as(json!(<TS literal>))` and so assumes its argument type
  derives `Deserialize` (SURFACE §6.6): `damage, damage_all, damage_split, destroy_all,
  destroy_at_next_turn_start, discard_random, draw, gain_mana, cast_new, choose_target, choose_mode,
  counter_play, heal, delay, animate, flicker, lock_lane, lock_random_zone, forced_attack_own_hero,
  forced_attack_random, set_health, steal, buff, go_berserk, remember, add_rolled_grapes,
  damage_enemy_or_heal_friend, draw_priced, replace_hand_with_random, discover_from_catalog, fuse_cards,
  fuse_generated, fuse_onto_your_card, fuse_random_into, place_plague, place_plague_each,
  place_plague_random, place_plague_tokens, buff_cards, degrade, discover_number, enchant, gain_brittle,
  give_brittle, grant_keyword_cards, set_number, upgrade, roll_back`. `cast_new`'s and
  `discover_from_catalog`'s TS arguments may also be functions; the JSON form is the one these fixtures use.
  A TS verb called with no argument where TS defaults it (`animate()`, `lockLane()`, `unlockAll()`,
  `lockPlayedZone()`, `cancelAttack()`, `transformBeneath()`, `recruitAll()`) is called with
  `Default::default()` (fits an args struct deriving `Default` or an `Option`); `lock_own_zone()` takes none
  (TS has no parameter).
- Read helpers: `effects::{chosen_number(&EffectContext) -> Option<i32>, chosen_options(&EffectContext) ->
  Vec<String>, chosen_tuning_number(&EffectContext) -> Option<{ instance_id: String, which: <Serialize> }>}`.
- `GameEvent::event_type()`, `ActionInput::with_nonce`, `Action::new`, `config::{MarkSpec, GRAPE_ODDS,
  ROLLBACK_MAX_TURNS, DECK_SIZE, AI_END_TURN_PROBABILITY}` are part 1's and exist.

### For part 31
- Lowercase `pub static` defs (`plain`, `body`, `charger`, …) block a `let`/parameter/pattern binding of
  the same name wherever they are imported (E0530). A ported test that imports `fixtures::<file>::*` and
  binds, say, `let body = …` must rename the binding (or the fixture must become a `fn`).
- Several fixture files export the same names (`note`, `notes`, `ask_controller`, `act`, `act_result`,
  `playing`, `answer`, `LOG_LANE`, `catalog`, `scripts`, and defs such as `asker`, `charger`, `phoenix`):
  a test that globs two of them must name the one it means, as TS's named imports did.

## Decisions
- Each TS `export const x = def(…)` is `pub static x: LazyLock<CardDef>` under TS's name snake_cased
  (`#![allow(non_upper_case_globals)]`), so a ported test reads `taunter.id` as TS does. TS's constant
  collections keep their names as `LazyLock` statics (`COMBAT_DEFS`, `COMBAT_SCRIPTS`, `DC_SCRIPTS`,
  `FIELD_DEFS`, `GEN_SCRIPTS`, `LAB_POOL`, `CT_DEFS`, …); number and string constants are `pub const`.
- TS's `CT` object of defs is `pub static CT: LazyLock<CtDefs>` with one field per key (`CT.x_bolt.id`) and
  `CtDefs::values()` for `Object.values(CT)`.
- TS's module index counters (`let nextIndex = N`, one per `def` call in file order) are written out: each
  def states the index the counter gave it (`activate` 4101–4121, `corePatches` 4401–4404, `instanceData`
  4401–4425, `damage-combat` 4701–4743, `field` 9401–9421, `generation` 4601–4637, `copiedText`
  5901–5914, `killCredit` 5801–5803), so the order the statics are first touched cannot change them.
- TS `Partial<CardDef>` overrides and other object-literal options are `serde_json::Value` (SURFACE §8):
  `unit_def(5, json!({ "attack": 3 }))`, `put(state, id, at, json!({ "radiant": true }))`, spread over the
  defaults key by key; `attack`/`health`/`keywords` are taken out of the overrides first where TS
  destructured them.
- A TS argument with a default is passed explicitly (`vanilla_catalog(40, 1)`, `new_game("engine-test",
  None)`, `combat_catalog(CardDefs::new())`, `token_def("rush", [Tag::Token])`, `flush(state, p, 10)`,
  `in_hand(state, id, p, 1)`, `hand_card(state, id, PlayerId::P1)`); an optional one is an `Option`
  (`decks`, `deck_pair`, generation's `answer(run, selection, None)`).
- TS's module mutable state (`let nonce`, generation's `askerAnswers`) is a `thread_local!` `Cell`
  (clippy.toml bans `RefCell`, not `Cell`): each Rust test runs on its own thread, as each TS test file ran
  in its own module. `askerAnswers` is read with `asker_answers()` and emptied with `clear_asker_answers()`.
- `sink_for(&mut state)` (TS's one-argument form, 404 of its 470 uses) leaks a fresh rng and event list so
  the sink borrows only the state; `sink_for_events(&mut state, &mut events)` is TS's two-argument form.
- Helpers that handed back TS's live card (`put`, `in_hand`, `set_library`, `hand_card`) return owned
  copies (`put` returns the card as it stands in the state after placing it).
- Action bodies (`act`, `act_result`, generation's `act`/`refusal`, `Recorder::play`) take `impl
  serde::Serialize` — an `ActionInput` or its JSON — and parse it as `ActionInput`. generation's local
  `ActionInput` type alias is not ported: it is the wire's.
- generation's run helpers borrow (`act(&run, …) -> Run`, `frozen(&run)`, `replayed(&run)`); damage-combat's
  `recorder(&start) -> Recorder` with `state()`, `play(body)`, `log`, `start`; harness's
  `play_random_game` returns `RandomGame { state, log, decks }`.
- damage-combat's `note(string | (ctx) => string)` is `note(impl Into<String>)` plus `note_with(fn)`.
- `ctx.self` reads: a Death or "after this attacks" hook reads `ctx.self_` (the card as it died or fought,
  R89, R426); Lockdown's "Tribute this" ability reads `ctx.live_self()` (TS's live object, moved by the
  cost); field's `note` writes the card in the state (TS wrote through the live `ctx.self`) and the
  context's copy.
- keeper's `canActivate` (TS `recalled({ self, data: {} }, KEEPER_KEY) !== undefined`) reads
  `self_.memory` at `part_memory_key(&{}, KEEPER_KEY)`, `recalled`'s own key for empty data, since a
  `ConditionContext` is no `EffectContext`.
- TS `String(x)`: `js_string` (a memory value, `"undefined"` for none) and `js_bool`.
- Part 24's brief step 2: every file also exports `pub fn catalog() -> CardDefs` and `pub fn scripts() ->
  IndexMap<String, CardScripts>` (its own defs and scripts; empty scripts for the script-less `catalog.rs`
  and `call_to_chaos_plus.rs`; harness's are what `setup_catalog` registers), beside the TS-named items.
- `TargetDecl` literals use part 1's constructors (`TargetDecl::target(1, 1, json!({…}))`, `Value::Null`
  for no filter) and struct update for `forModes`; `ReplacementDef` literals a local `replacement(id, on,
  instead)` plus struct update.
