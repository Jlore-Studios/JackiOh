# Slice: part 13 (cards lane 5: Classic #33–#68), chunk 2 of 4 (#401, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
Every file is the whole port of its TS script and TS test (every function in TS order, the headers and
every rule-stating comment kept; one `#[test]` per TS `it`, counts checked against the TS files):
- `crates/cards/src/scripts/classic/c044_back_from_the_gy.rs` (new; 22 tests)
- `crates/cards/src/scripts/classic/c045_nature_titan.rs` (new; 19)
- `crates/cards/src/scripts/classic/c046_divine_favor.rs` (the night bot's port, finished; 15)
- `crates/cards/src/scripts/classic/c047_recurring_felinor.rs` (new; 17)
- `crates/cards/src/scripts/classic/c048_hired_shrimp.rs` (new; 20)
- `crates/cards/src/scripts/classic/c049_anti_greed_machine.rs` (new; 18)
- `crates/cards/src/scripts/classic/c050_voidwalker.rs` (new; 24)
- `crates/cards/src/scripts/classic/c051_back_breaker.rs` (new; 11)
- `crates/cards/src/scripts/classic/c052_final_gambit.rs` (the night bot's port, finished; 31)
- `crates/cards/src/scripts/classic/c053_plague_crawler.rs` (new; 21)
Nothing left. No `todo!`, `unimplemented!` or `// TODO`.

## SURFACE
Each file: `pub const ID`, `pub fn script() -> CardScripts`, a `#[cfg(test)] mod tests` (SURFACE §7.1, §7.3).
Part 1's frozen types are used as compiled: hooks take `&mut EffectContext` (`hook(|ctx| …)`), triggers are
`TriggerDef::new(id, &[GameEventType::…], |ctx, event| …)` with `.with_when(…)`, `ReplacementDef` /
`ReplacementInstead` / `ReplacementWhen` / `replacement_when`, `DrawLimit` / `DrawLimitPlayer` with
`read_hook`, `target_check`, `condition_hook` for `preview`, `StaticFlags`, `PreviewValue`.

## GAPS (names called in other parts' modules; the shape assumed)
- `params::param(&impl ParamContext, &str) -> i32` (part 2.1): passed `&*ctx` (an `&EffectContext`), `&args`
  (`HookArgs`, `ConditionContext`) or a `&EffectContext` closure argument. `params::step_param(&mut
  CardInstance, &str, i32)` in tests, on `state::find_instance_mut(s.state_mut(), id)`.
- `query::{zone_count(&GameState, PlayerId, OffFieldZone) -> i32, zone_cards(..) -> slice or Vec<&_>
  (only iterated), hero_of(&GameState, PlayerId).health, cards_played_this_turn(&GameState, PlayerId) -> i32}`;
  `zones::OffFieldZone::{Hand, Library, Graveyard}`; `catalog::def_of(Option<&GameState>, &str)` (`.type_`,
  `.loc`; part 2.2's owner shape); `mana::effective_cost(&GameState, &CardInstance, Default::default()) -> i32`;
  `replacements::replacement_of(&EffectContext) -> Option<ReplacementRecord { redirected_to: Option<PlayerId>, .. }>`.
- Effects (built with `json_as` of the TS literal, so only serde shape matters): `choose_pick`, `exile`,
  `summon`, `draw`, `heal`, `add_to_hand` (`{ instance: { of: "self" }, costOverride? }`), `destroy`,
  `destroy_all` (a `BoardScope`), `exile_matching` (`{ zones, player }`), `place_plague` (`{ target, amount }`).
- Effects whose args hold functions, built as struct literals (part 6.2 says they derive `Clone` only):
  `draw_while(DrawWhileArgs { more: Arc<dyn Fn(&EffectContext) -> bool>, player: Option<PlayerSpec> })`;
  `for_each_card(ForEachCardArgs { cards: Arc<dyn Fn(&EffectContext) -> Vec<String>>, each: Arc<dyn
  Fn(&str) -> Effect> })`; `cast_new(CastNewArgs { def: CastNewDef::Def(CastDef { def_id, radiant:
  Option<bool> }), radiant: Option<bool>, how: CastHow })` — the riskiest: if `CastNewArgs` names its
  flattened `CastHow` field otherwise (or derives `Deserialize`), C #47's `cry` is the one line to fix.
- `effects::targets::{TargetSpec::Chosen { index: None }, instance_of(&EffectContext, &TargetSpec) ->
  Option<CardInstance>}`; `effects::card_scope::unreadable_by(&GameState, &CardInstance) -> Vec<PlayerId>`.
- Tests: `effects::tune::{applicable_changes(&GameState, &CardInstance, TuneDirection) -> Vec<TuneRow>,
  TuneDirection::{Upgrade, Degrade}, TuneRow::Number}` (C #46, the bot's call, kept);
  `animated::animate_card(&mut EngineSink, &CardInstance, Default::default()) -> bool` (C #51; part 2.2's
  owner shape is an options struct, part 6.1 assumed `Option<Position>`: `Default::default()` fits both);
  `subsystems::fuse::fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>` with `FuseArgs` from
  `json_as` (part 8.1 says it derives serde); `legal_actions`, `reduce(&GameState, &Action)`,
  `crate::card_def(id).loc` (cards lib.rs), `registered_catalog()`.
- Testkit (part 5.1): `Scenario::{state_mut, unit/backrow -> Option<CardInstance>, hand/pile -> Vec<_>,
  card(ref) -> &CardInstance, stats, view, expect_*}`; a card ref may be `&str`, `&String` or
  `&CardInstance`. No `card_mut`: the bot's `s.card_mut(..)` calls were replaced by `find_instance_mut`.

## Decisions
- The night bot's C #46 and C #52: kept their tests (every `it` was there) and fixed what broke the
  rules or the frozen types: `preview` now `condition_hook` (by-value `ConditionContext`, a `PreviewValue`
  literal); C #52's `ReplacementDef` is a struct literal (it holds a hook, so it has no serde) and its
  test compares the declaration field by field; `DrawCount` is an `Arc<dyn Fn>` (no fn pointers);
  `Option<CardInstance>` reads borrow with `as_ref()`; test names use `s` for `§` (`s2_4_…`).
- TS module consts holding functions (`cry`, `baseLimit`, `onAttack`, `drawOnPlacement`, `targets`) are
  private `fn`s, called once per face; `radiant = base.clone()` where TS wrote `radiant = base`, and the
  TS test's `expect(radiant).toBe(base)` is ported as "the same flags, hooks and trigger ids".
- C #44's `graveyardUnits` keeps TS's shape (the cards), and the `for_each_card` closure maps them to ids
  (part 6.2: `cards` answers ids).
- C #50's Radiant `when` is `matches!(event, ToGraveyard { owner, .. } if owner != controller)`; the
  other moments never reach it (`ReplacementContext.event` is always the declared moment).
- C #45's `toThrow(/tribute/i)` is `expect_refused_with(…, "ribute")`, which holds whichever case the
  message's first letter takes (no regex crate).
- TS tests that mutate a live card (`stepParam(s.card(x), …)`, `stepParam(s.unit(…), …)`) write through
  `find_instance_mut(s.state_mut(), id)`; `s.state.x = …` is `s.state_mut().x = …`; a TS sink
  `{ state, events, rng }` is `EngineSink::new(s.state_mut(), &mut events, &mut Rng::new(seed, cursor))`,
  its cursor not written back (TS's local rng was dropped too).
- Comparisons against TS object literals go through a private `js()` (serde_json of the engine value);
  `new Set(...)` equality is `IndexSet` equality (order-insensitive).
