# Slice: part 5 (engine 4), chunk 2 of 4
BUILDS-RUN: 0

## FILES
- `crates/engine/src/wire/emotes.rs` ← `packages/shared/src/emotes.ts`, with `packages/shared/test/emotes.test.ts` as `#[cfg(test)] mod tests` (one `mod` per `describe`, one `#[test]` per `it`).
- `crates/engine/src/condition.rs` ← `packages/engine/src/condition.ts`.
- `crates/engine/src/reduce.rs` ← `packages/engine/src/reduce.ts` (no `rng` argument, no `syncFusedScripts`).
- `crates/engine/src/view_for.rs` ← `packages/engine/src/viewFor.ts` (no `syncFusedScripts`).
- `crates/engine/tests/rules/fixtures/validator_loadouts.rs` ← `packages/validator/test/fixtures/loadouts.ts`.
- `crates/cards/tests/cross/invariants.rs` ← `packages/cards/test/invariants.test.ts` (313 lines today, not the 55 the table says; every `it` ported).

All files were empty placeholders on `staging`; all are full ports. No `todo!`, `unimplemented!` or `// TODO`.

## SURFACE
Matched: `reduce(&GameState, &Action) -> ReduceResult`, `begin_game(&GameState) -> ReduceResult`,
`legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`, `view_for(&GameState, PlayerId) -> PlayerView`,
`seat_to_act(&GameState) -> Option<PlayerId>` (always `Some`, as TS always names a seat).
`ReduceResult { state, events, error: Option<String> }` is defined in `reduce.rs` (camelCase serde, `error` skipped when `None`).

## DEPENDS-ON (names called in other modules, TS name snake_cased at the TS module's path)
reduce.rs:
- `combat::{AttackTarget (enum: Unit { instance: CardInstance } | Hero { player }), ExertionKind::{Attack, Switch}, attack_targets(&GameState, &CardInstance) -> Vec<AttackTarget>, declare_attack(&mut EngineSink, &CardInstance, &AttackTarget) -> Result<_, EngineError>, has_exertion(&GameState, &CardInstance, ExertionKind) -> bool, switch_position(&mut EngineSink, &CardInstance) -> Result<_, EngineError>}`
- `game_over::end_game(&mut EngineSink, Winner, GameOverReason)`
- `modifiers::remove_modifier(&mut EngineSink, PlayerId, &str)`
- `play_choices::{play_actions_for, graveyard_play_actions_for}(&GameState, PlayerId, &CardInstance) -> Vec<PlayAction>` with `ActionBody: From<PlayAction>` (identity if `PlayAction = ActionBody`)
- `play_steps::run_play_steps(&mut EngineSink, PlayerId, &ActionBody) -> Result<_, EngineError>`
- `prompts::{AnswerInput { player_id, choice_id, selection }, answer_prompt(&mut EngineSink, &AnswerInput) -> Result<_, EngineError>, prompt_answers(&PendingChoice) -> Vec<_ into ActionBody>}`
- `scripts::flags_of(&GameState, &CardInstance) -> StaticFlags` (state added: fused scripts are looked up from it, SURFACE §6.6)
- `setup::{answer_mulligan(&mut EngineSink, PlayerId, &[String]), begin_setup(&mut EngineSink), mulligan_owed(&GameState) -> Vec<PlayerId>, mulligan_prompt_for(&GameState, PlayerId) -> Option<&PendingChoice>, why_mulligan_refused(&GameState, PlayerId, &[String]) -> Result<(), EngineError>}`
- `subsystems::activate::{activate_ability(&mut EngineSink, PlayerId, &ActionBody) -> Result<_, EngineError>, activate_actions_for(&GameState, PlayerId, &CardInstance) -> Vec<_ into ActionBody>}`
- `subsystems::ai_policy::play_out_turn(&mut EngineSink, PlayerId) -> PlayoutResult { actions }`
- `subsystems::glitch::reset_match(&mut EngineSink)`, `subsystems::hero_power::power_ability_of(&GameState, &CardInstance) -> Option<&ActivationDecl>`
- `triggers::settle(&mut EngineSink)` (one argument)
- `turn::{answer_draw(&mut EngineSink, PlayerId, bool), can_offer_draw, has_standing_draw_offer (&GameState, PlayerId) -> bool, concede(&mut EngineSink, PlayerId), end_turn(&mut EngineSink), offer_draw(&mut EngineSink, PlayerId)}`
- `zones::{active_units_of, carried_units_of (&GameState, PlayerId) -> Vec<&CardInstance>, card_at(&GameState, ZoneRef) -> Option<&CardInstance>, slots_of(PlayerId, Row) -> Vec<ZoneRef>}`

condition.rs: `query::{granted_combo_live(&GameState, PlayerId), gifted_would_make_radiant(&GameState, PlayerId, &CardInstance)} -> bool`; `subsystems::copied_text::{running_script_of(&GameState, &CardInstance) -> Script (or &Script), text_face_of(&GameState, &CardInstance) -> CardInstance (or &)}`.

view_for.rs: `animated::is_animated`, `announce::announced_face_down_to(&GameState, &CardInstance, PlayerId)`, `catalog::registered_catalog`, `combat::has_exertion`, `cost_rules::{cost_rule_text(&CostRule, bool) -> String, enchant_next_spell_label(&Enchantment) -> String}`, `counter_warning::countered_hand_cards(&GameState, PlayerId) -> IndexSet<String>`, `damage::hero_armor_of(&GameState, PlayerId) -> i32`, `echo::echo_grant_of(&GameState, PlayerId, &PlayerModifier) -> i32`, `faces::card_type_of(&GameState, &CardInstance) -> CardType`, `instance_view::{instance_data_view(&GameState, &CardInstance) -> InstanceData { type_, brittle, params, tuning, enchantments }, hand_keywords_view(&GameState, &CardInstance) -> Option<Vec<Keyword>>}`, `layers::{unit_view(&GameState, &CardInstance) -> layers::UnitView { attack, max_health, health, keywords, armor, position }, stats_with_buffs(&GameState, &CardInstance) -> { attack, max_health }}`, `mana::{NEXT_REFRESH_MODIFIER_ID, effective_cost(&GameState, &CardInstance) -> i32, modifier_is_live(&GameState, &PlayerModifier) -> bool}`, `marks::marks_on(&GameState, &str) -> Vec<CardMark>`, `own_library::own_library_view(&GameState, PlayerId) -> LibraryView`, `params::params_view(&GameState, &CardInstance) -> Option<IndexMap<String, i32>>`, `preview::{backrow_is_public(&GameState, &CardInstance, PlayerId), is_face_down(&GameState, &CardInstance), preview_of(&GameState, &CardInstance, PlayerId, ConditionZone) -> Option<Vec<PreviewValue>>}`, `setup::{mulligan_prompt_for, returned_awaiting_shuffle(&GameState) -> Vec<String>}`, `subsystems::activate::activation_views_for(&GameState, PlayerId, &CardInstance) -> Option<Vec<ActivationView>>`, `subsystems::combo_index::grade_name(i32) -> Grade (Display)`, `subsystems::copied_text::{copied_text_of(&GameState, &CardInstance) -> Option<PlayRecord>, text_face_of}`, `subsystems::hero_power::{power_of(&CardInstance) -> Option<HeroPower { name, x, .. }>, power_title_of(&HeroPower, bool) -> String, used_this_turn(&GameState, &CardInstance) -> bool}`, `subsystems::quests::quest_view_of(&GameState, &CardInstance) -> Option<QuestView>`, `turn::standing_draw_offer(&GameState) -> Option<PlayerId>`, `zones::{beneath_at(&GameState, ZoneSlot) -> Vec<CardInstance>, carried_at(&GameState, ZoneRef) -> Option<&CardInstance>, carried_units_of, home_of(&GameState, &str) -> Option<&HomeZone>, is_reserved(&GameState, ZoneRef) -> bool, slot_of(&GameState, &CardInstance) -> Option<ZoneSlot>, slots_of}`.

validator_loadouts.rs: `validator::{CardId (= String), CatalogSnapshot { version, cards: CardDefs, banned: Option<Vec<CardId>> }, Collection (= IndexMap<String, i32>), LOADOUT_DECKS, LoadoutDeck { name: Option<String>, cards: Vec<CardId> }, LoadoutInput { decks, catalog, collection }, LoadoutRule::{L1..L6}}`, all `Clone`.

invariants.rs (cards cross test): `testkit::{scenario(Value) -> Scenario, Scenario::{state(), state_mut(), play(&str, Value), end_turn(), last_events()}, create_invariant_monitor(&GameState) -> Monitor with after(&[GameEvent], &GameState) -> Vec<String> and hidden(&GameState) -> Vec<String>, hidden_information_violations(&GameState, PlayerId, &PlayerView, &[ActionBody]) -> Vec<String>}`, `zones::cease_to_exist(&mut GameState, &CardInstance)`, `jackioh_cards::register_all()`.

## GAPS
- `testkit::Scenario::state_mut() -> &mut GameState`: SURFACE §8 names `s.state()` only, but TS tests assign `g.state.applied`, `g.state.pending` and delete `knownAs`; `invariants.rs` calls `state_mut()` (testkit/scenario.rs, part 5 chunk with the testkit).
- `scripts::flags_of` is called with `(state, instance)`; TS's `flagsOf(instance)` took no state. If part 2 kept the one-argument form, drop `state` at `reduce.rs::can_switch`.
- Calls pass what the TS call site passed; a TS default argument (`switchPosition(sink, unit, options = {})`, `settle(sink, options = {})`, `effectiveCost(state, card, options = {})`, `playOutTurn(sink, player, options = {})`) is left to the callee (a one-argument-fewer wrapper, or `Default` at the call). Part 31: add the default where the callee kept the parameter.
- `ZoneRef`/`ZoneSlot` are passed by value (Copy) to `zones::{card_at, carried_at, is_reserved, beneath_at}`; switch to `&` if part 2 took references.
- Refusals from other modules are read as `Result<_, EngineError>` (SURFACE §4.4.9): `declare_attack`, `switch_position`, `run_play_steps`, `answer_prompt`, `activate_ability`, `why_mulligan_refused`. A module that kept TS's `Option<String>`/`{ error }` shape needs a `.map_err`/`ok_or` at those six call sites.
- `view_for_with_clock(state, player, clock_ms)` is added beside SURFACE's two-argument `view_for`: TS's third argument `clockMs` (R79) is what the server's actor passes; part 19 calls this one (or sets `view.clock_ms` itself).
- `HIDDEN_ID` and `HIDDEN_OPTION_LABEL` are defined in `view_for.rs` (part 1 left viewFor's sentinels there); nobody else may define them, or the root glob is ambiguous.

## Decisions
- `reduce.rs`: TS's generator `eachLegalAction` is a visitor returning `ControlFlow<()>`; `legal_actions` collects every action, `auto_end_due` breaks at the first that is not endTurn/concede/offerDraw, so it stops computing at the first producer that yields one (#188). Order is `eachLegalAction`'s exactly.
- `reduce.rs`: the `turnEnds` rider is read and decremented through a private copy of `modifiers::turnEndsOf` (TS mutated the live object `turnEndsOf` returned, which a Rust `&` return cannot do).
- `reduce.rs`: `mulligan_subsets` guards `2 ** n` against shift overflow (n ≥ 64 is past the cap anyway).
- `reduce.rs`: a refused action returns `state.clone()`, no events and TS's text; a replayed nonce returns `state.clone()` and the first events with `error: None`.
- `view_for.rs`: `redact_event` works on the event's JSON (`serde_json::to_value`, edit keys, `from_value`) so TS's spreads and destructures (`{ ...event, instanceId: HIDDEN_ID }`, removing `arrivedDuring`/`exitsFrom`/`formerId`/`radiant`/`copyOf`/`hiddenFrom`/`readableFrom`) port key for key; the match is over `event.event_type()` with no wildcard, so a new event type still fails to compile until decided. Events left unchanged are returned as `event.clone()`.
- `view_for.rs`: `match_defs_in` walks the view's `serde_json::Value`; serde_json without `preserve_order` visits object keys sorted, so `defs`' insertion order can differ from TS's (same set; the hash sorts keys).
- `view_for.rs`: private copies (fullsend rule 5) of `catalog.findDef` (`find_def_in`, transient defs first, then `registered_catalog()`) and `plague.plagueOn` (`plague_tokens_on`); `costRuleModifierLabel(mod)` is called as its body, `cost_rule_text(rule, expiry == Used)`, to avoid guessing its argument shape.
- `view_for.rs`: `readable_where_stolen` answers `false` for `ZoneName::Gone` (TS's switch had no case and returned `undefined`).
- `emotes.rs`: `EmoteId` and `PortraitId` are `string_union!` enums; `VOICE_EMOTE_IDS`, `EMOJI_EMOTE_IDS`, `EMOTE_IDS` (= `EmoteId::ALL`) and `PORTRAIT_IDS` (= `PortraitId::ALL`) are `&[EmoteId]`/`&[PortraitId]`; `VoiceEmoteId`/`EmojiEmoteId` are aliases of `EmoteId`. `is_emote_id`/`is_portrait_id` take `&serde_json::Value` (TS's `unknown`, SURFACE §4.3); a `&str` caller uses `str::parse::<EmoteId>()`. `portrait_or_default(Option<&str>)`. `PORTRAITS: &[(PortraitId, PortraitEntry)]` plus `portrait_entry(id)` for TS's `PORTRAITS[id]`. Milliseconds are `i64` (`EMOTE_COOLDOWN_MS`, `EMOTE_WINDOW_MS`, `emote_gate(&[i64], i64)`), `EMOTE_WINDOW_MAX: usize`. `EmoteGate` is one struct `{ ok, retry_after_ms: Option<i64>, sent_at }` whose JSON is TS's union exactly. The constants stay in `emotes.rs`, not `config.rs`: both ends of the wire read them from here (the TS header's own reason).
- `validator_loadouts.rs`: `POOL_IDS` is a `LazyLock<Vec<CardId>>` static (TS's computed constant array; a test file) with `pool_ids()` beside it; injections take `&LoadoutInput` and return a new one; `INJECTIONS: &[Injection]` holds `fn` pointers. `LOADOUT_DECKS` is cast `as usize` (its type is the validator's).
- `invariants.rs`: card refs are read straight off `s.state()` (`players[p].units[lane-1]`, `hand[0]`, `backrow[lane-1]`) rather than through `unit()`/`hand()`/`backrow()`, whose return types SURFACE does not fix; events, prompts and keywords are built from TS's literals with `json_as`. Each test calls `jackioh_cards::register_all()` first (the engine's testkit cannot depend on the cards crate). Titles that cite rulings lead with them (`r636_r44_…`).
