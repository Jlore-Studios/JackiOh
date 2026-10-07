# Slice: part 16 (cards lane 8: Classic+ #50–#78 and the AI tokens), chunk 1 of 3 (#404): C+ #50–#65.1
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All complete, each the whole TS script (every function in TS order, its header and doc comments,
every comment that states a rule or cites a ruling) with the whole TS test file as its
`#[cfg(test)] mod tests` (the test file's header above it, one `mod` per `describe`, one `#[test]`
per `it`; the `it` counts match the TS files one for one). No `todo!`, `unimplemented!` or `// TODO`.

- `crates/cards/src/scripts/classic_plus/c050_adaptive_growth.rs` — the night bot's half-done port
  (commit 0180138) read against both TS files and finished in place: its `ConditionHook` took
  `&ConditionContext` (part 1's takes the bundle by value: now `condition_hook(|c| …)`), its def check
  read `registered_catalog()[ID].r#type` (now `crate::card_def(ID).type_`), its pile read iterated an
  `Option<Pile>` as if it were the pile, its lanes were `usize` and it called a `card_mut` the testkit
  does not have (now `find_instance_mut(s.state_mut(), …)`). There were no bot notes for part 16
  (`.fullsend/notes/part-16-bot.md` absent).
- `c051_jlockheeds_j15_fighter.rs`, `c052_jlockheeds_permanent_defense_contract.rs`,
  `c053_book_of_tokens.rs`, `c054_book_of_books.rs`, `c055_book_of_greed.rs`, `c056_book_of_pain.rs`,
  `c057_book_of_stats.rs`, `c058_fruit_basket.rs`, `c059_all_purpose_apple.rs`, `c060_doctors_orders.rs`,
  `c061_bauble_bubble.rs`, `c062_kys_papaya.rs`, `c063_fruit_tree.rs`, `c064_mulch_muncher.rs`,
  `c065_1_rotten_grape.rs` — new.

Nothing in this chunk's list is left.

## SURFACE
- Every card file: `pub const ID`, `pub fn script() -> CardScripts`, private helpers and constants.
  Nothing else public. TS's `def` export is not ported (SURFACE §7.1); a test reads the def with
  `crate::card_def(super::ID)`.

## DEPENDS-ON
- Part 1 (frozen, matched): `Script`, `CardScripts`, `StaticFlags`, `hook`, `condition_hook`,
  `cost_hook`, `ConditionHook`, `ConditionZone`, `CostArgs`, `HookArgs`, `ActivationDecl`,
  `ActivationUses::Count`, `EffectContext` (`live_self`, `data`, `controller`, Deref to the sink's
  `state`/`rng`), `EngineSink::new`, `Effect`, `TargetDecl::target`, `TargetAim::Help`, `fill_params`,
  `FaceKind`, `opponent_of`, `PlayerId::opponent`, `Rng::{int, lucky}`, `create_rng`, `find_instance_mut`,
  `CreateGameOptions`, `Action::new`, `ActionBody`, `Selection`, `GameEvent` variants, `PlayerModifier`,
  `ModifierKind::{StartOfTurnEffect, HealToDamage}`, `ModifierExpiry::Never`, `HandView`, `CardView`,
  `Keyword`, `KeywordKind::as_str`, `Tag`, `Rarity`, `PrintedRarity`, `SetName`, `CardCost`, `Zone`,
  `ZoneName`, `PromptKind::Cell`, `Winner`, `AttackHealth`, `json_as`; `crate::{card_def, register_all}`.

## GAPS
### Called in other parts' modules (TS name snake_cased at its TS module's path; the shape assumed)
- `params::param(&impl ParamContext, &str) -> i32` with `ParamContext` for `EffectContext` (called as
  `param(&*ctx, …)`) and for `HookArgs` (C+ #64's cost hook: TS `param({ state, self: instance,
  radiant }, …)` is `param(&HookArgs { state, self_: instance, radiant }, …)`), as part 2.1's notes say.
  `params::step_param(&mut CardInstance, &str, i32)` (tests, on the card under its id).
- `zones::active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>` (`.len()` only).
- `numbers::{numbered_keywords_on(&GameState, &CardInstance) -> Vec<NumberedKeyword>, NumberedKey::Lucky}`,
  `NumberedKeyword { key, value: i32, .. }` (C+ #53; matched with `matches!`, so no `PartialEq` needed).
- `catalog::{query_cost(&CardDef) -> i32, def_of(Option<&GameState>, &str), query(&CatalogQueryArgs) ->
  Vec<&CardDef>}` with `CatalogQueryArgs: Deserialize` (tests build it with `json_as`); `def_of`'s
  answer is read by field and `.clone()`d, so `&CardDef` or `CardDef` both fit.
- `graveyard_play::playable_from_graveyard(&GameState, &CardInstance) -> bool`;
  `query::played_this_game_with_tag(&GameState, PlayerId, Tag) -> i32` (C+ #64).
- `mana::{effective_cost(&GameState, &CardInstance, <options>: Default) -> i32, cost_now(&GameState,
  &CardInstance) -> i32}` (tests; the options passed as `Default::default()`, part 4's three arguments).
- `combat::random_attack_targets(&GameState, &CardInstance, AttackAmong) -> Vec<AttackTarget>` with
  `AttackAmong: Deserialize` from `"enemies"`/`"enemyUnits"` (built with `json_as`) and `AttackTarget`
  matched as `damage::DamageTarget::{Hero { player }, Unit { instance: CardInstance }}` (C+ #51's test).
  If `AttackTarget` is not an alias of `DamageTarget`, the two `match`es name the wrong enum.
- `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext,
  HookOptions { controller: Option<PlayerId>, .. }: Default, apply_effects(&[Effect], &mut EffectContext)}`;
  `triggers::{settle(&mut EngineSink, SettleOptions), SettleOptions: Default}` (tests of #62, #64, #65.1
  that build a sink by hand, as TS did).
- `reduce::{reduce(&GameState, &Action) -> ReduceResult, begin_game(&GameState) -> ReduceResult}`,
  `create_game(&CreateGameOptions)`, `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`,
  `replay::{hash_state(&GameState) -> String, fold(&FoldArgs) -> FoldResult { state, errors: Vec<_> }}` —
  `FoldArgs` (= `ReplayInput`) must derive `Deserialize` with camelCase keys: C+ #62's replay test builds
  it with `json_as(json!({ seed, decks, log }))`.
- `subsystems::papaya::{papaya_begin() -> Vec<Effect>, papaya_answered(&mut EffectContext) -> Vec<Effect>,
  PAPAYA_STEP: &'static str}` (part 8's notes; the card wraps the fn in `hook(|ctx| …)`).
- Effects (parts 6–7), each `fn(<args>) -> Effect` whose one argument deserialises from the TS literal
  (every call is `json_as(json!({ … }))`, imported by name from `jackioh_engine::effects` so a prelude
  glob collision cannot shadow it): `buff_all_units` (`side`, `attack`, `health`), `for_rest_of_game`
  (`step`, `label`, `data`), `add_random_from_catalog` (`query`, `count`, `radiant`, `costMod`,
  `costOverride`, `temporary`), `summon` (`defId`, `radiant`), `discard_random` (`count`, `player`),
  `buff` (`target`, `attack`, `health`), `heal` (`target: { of: "selfHero" }`, `amount`), `damage`
  (`to: { of: "chosen" }`, `amount`), `add_to_hand` (`defId`, `radiant`, `costOverride`), `lose_health`
  (`player`, `amount`); tests: `lock` (`zone`), `cast_new` (`def` as a string, `random`),
  `convert_healing` (TS's default `{}` passed as `json_as(json!({}))`).
- The testkit as part 5.1's notes give it: `scenario(Value)`, `Scenario::{state(), state_mut(), events(),
  last_events(), view(seat), unit(seat, i32), backrow(seat, i32) -> Option<CardInstance>, hand(seat),
  pile(seat, &str) -> Vec<CardInstance>, card(ref) -> &CardInstance, stats(ref) -> layers::UnitView {
  attack, health, keywords, .. }}`, the steps `play(card, Value)`, `attack(card, card | "hero")`,
  `answer(Value)` (an option key as a JSON string), `end_turn()`, `activate(card, Value)`,
  `switch_position(card)` returning `&mut Scenario`, and `expect_in_zone`, `expect_stats`,
  `expect_events(Value)` (an array of type names), `expect_health`, `expect_mana`,
  `expect_refused(|s| …)`, `expect_refused_with(|s| …, "text")`. A card ref is a `&str`/`&String` (id,
  catalog id or name) or a `&CardInstance`; a seat a `PlayerId`.

### Not ported
Nothing. TS `cardDef("core-t-rush").id`, `cardDef("classicplus-059").id`, `cardDef("core-005").id` used
as values are private `&str` constants (their load-time existence checks are the cards' tests).

## Decisions
- Test modules: `use jackioh_engine::testkit::*;` (plus the effect verbs a test applies by hand) and no
  `use super::*` at the top, so the prelude's verbs never meet the testkit's globs; nested `mod base` /
  `mod radiant` take `use super::*;`. Each test module defines a local `scenario(Value)` that calls
  `crate::register_all()` first and shadows the testkit's (TS's harness registers on import); tests that
  build state without a scenario call `register_all()` themselves.
- TS `stepParam(s.card(ref), …)` and `s.card(ref).grantedKeywords.push(…)` wrote through the live object:
  here `find_instance_mut(s.state_mut(), &id)` on the card's id (a local `step` helper).
- TS `s.unit(p, lane) ?? ""` is a local `unit_id` returning the id or `""`.
- TS option bags (`signed({ radiant?, seed?, contracts?, fillers? })`, …) are positional arguments, an
  `Option` for each TS default. A setup's `...(radiant ? { radiant: true } : {})` is written
  `"radiant": radiant` (the harness's default is `false`); a script's conditional spread stays
  conditional (#52's `delayed`, #63's Fruit), so the effect's arguments are exactly TS's.
- `expect(radiant).toBe(base)`: `Arc::ptr_eq` on the shared hook (#56 and #57 the Cry, #62 the Cry,
  #64 the cost hook); #51 (flags only) compares the two faces' `static_flags` and that neither has a
  Cry. `typeof base.cry === "function"` is `.is_some()`.
- `toEqual` on declarations and static flags compares their serde JSON with `json!` (key order free);
  `toMatchObject({ instanceId: "hidden", defId: "hidden" })` is a `matches!` with a guard;
  `JSON.stringify(view)` is `serde_json::to_string`.
- #50: `conditionMet` built once and shared by both faces (TS `radiant.conditionMet = base.conditionMet`).
- #52: `carried` reads the data bag with `Value::as_i64` (TS `typeof value === "number"`), else `param`;
  the cry's label is `fill_params(&crate::card_def(ID), face, Some(&{ cards, discount }))`.
- #53: `luckyOf` reads `ctx.live_self()` (TS's live `ctx.self`); the roll is a non-capturing closure
  over `&mut Rng`, handed to `Rng::lucky` or called once, so the draws are TS's in TS's order.
- #64: `PRINTED_COST` is computed once in `script()` from `crate::card_def(ID)` and moved into the hook.
- #62: TS's hand-built sink (`{ state, events: [], rng }`, `makeContext`, `applyEffects`, `settle`,
  `state.rngCursor = sink.rng.cursor`) is a local `by_hand` helper; #64 and #65.1 write it inline.
- Test names: the TS title snake_cased, `§` → `s`, `#` → `n`, punctuation dropped, ruling tokens leading
  (`r58_r70_…`, `r594_r386_…`).
