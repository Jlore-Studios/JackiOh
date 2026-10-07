# Slice: part 13 (cards lane 5: Classic #33–#68), chunk 1 of 4 — Classic #33–#43 (#401, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All eleven hold the whole port: the TS script (every function in TS order, its header and rule
comments) and, below it, its TS test file as `#[cfg(test)] mod tests` (one `#[test]` per `it`, one `mod`
per `describe`, the test file's header comment above it). No `todo!`, `unimplemented!` or `// TODO`.

- Night bot's half-done ports (commit 0180138), read against both TS files and finished in place:
  `crates/cards/src/scripts/classic/{c033_joro, c034_ancient_acquisition, c035_prep, c036_burn,
  c037_last_hurrah, c040_mc_tech, c041_state_of_the_game, c042_transmutable_toxins, c043_plague_nuke}.rs`.
  Every `it` was already there; what was fixed is listed under Decisions.
- New: `crates/cards/src/scripts/classic/{c038_jackiestan_auctioneer, c039_outbreak}.rs`.

Test counts (TS `it` = Rust `#[test]`): #33 23, #34 21, #35 12, #36 11, #37 13, #38 19, #39 19, #40 18,
#41 14, #42 18, #43 17.

## SURFACE
Every file: `pub const ID: &str = "classic-0NN"` and `pub fn script() -> CardScripts` (SURFACE §7.1);
nothing else public. Hooks are `hook(|ctx| …)` over `&mut EffectContext` (part 1); typed hooks use part
1's builders: `condition_hook(|c: ConditionContext| …)` for `condition_met` and `preview`, `aura_hook(f)`
with `fn aura<'a>(HookArgs<'a>) -> Vec<AuraEntry<'a>>`, `TriggerDef::new(id, &[GameEventType::…], run)
.with_when(…)`. `ReplacementDef` and `ActivationDecl` (not serde types) are struct literals.

## DEPENDS-ON
Called by TS name snake_cased at its TS module's Rust path (rule 6); the shape I assumed:
- `params::param(ctx, key) -> i32` taking each of `&EffectContext` (a `&mut` coerced), `&HookArgs` and
  `&ConditionContext` (TS's structural `{ state, self, radiant, data?, defId? }`); a trait or generic in
  `params.rs` (part 2/4). `params::step_param(&mut CardInstance, &str, i32)`.
- `query`: `recalled(&EffectContext, &str) -> Option<Value>` (part 2.2's notes), `cards_played_this_turn(
  &GameState, PlayerId) -> i32`, `zone_count(&GameState, PlayerId, OffFieldZone) -> i32`,
  `unspent_mana_of`/`max_mana_of(&GameState, PlayerId) -> i32`.
- `mana::{cost_now(&GameState, &CardInstance) -> i32, effective_cost(&GameState, &CardInstance,
  Default::default()) -> i32}`; `plague::plague_on(&CardInstance) -> i32`;
  `catalog::def_of(Option<&GameState>, &str) -> &CardDef` (part 2.2's notes; `.token` read).
- `zones::{active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>, slots_of(PlayerId, Row) ->
  Vec<ZoneSlot>, card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>, OffFieldZone::Library}`.
- Effects, args from TS's literals by `json_as`: `add_random_from_graveyard`, `add_player_modifier`,
  `damage`, `draw`, `discard_hand_at_turn_end`, `remember`, `place_plague`, `place_plague_random`,
  `steal` (`StealTarget`), `choose_target`, `gain_mana`, `summon`, `destroy_all(BoardScope)`;
  `animate(Default::default())` (`AnimateArgs: Default`).
- `effects::each::{for_each_card, ForEachCardArgs { cards: Arc<dyn Fn(&mut EffectContext) -> Vec<String>
  + Send + Sync>, each: Arc<dyn Fn(&str) -> Effect + Send + Sync> }}` (part 6.2's notes: ids, not TS's
  `CardInstance | string`; `&mut` because #40's `cards` shuffles with `ctx.rng`). Built from named fns.
- `effects::targets::{instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>,
  TargetSpec::Chosen { index: None }}`.
- `effects::after_check::after_state_check(impl Fn(&mut EffectContext) -> Vec<Effect> + Send + Sync +
  'static) -> Effect` (a closure, as parts 8.2 and 24.1 assumed; not a `Hook`).
- `subsystems::fuse(&mut EngineSink, FuseArgs /* Deserialize */) -> Option<CardInstance>` (#33's test).
- Testkit (part 5.1's notes): `scenario`, `Scenario::{play, attack, answer, activate, end_turn, state,
  state_mut, events, last_events, view, unit, hand, pile, card, stats, expect_in_zone, expect_stats,
  expect_events, expect_health, expect_mana, expect_refused}`; `crate::register_all()` and
  `crate::card_def(id) -> CardDef` (part 1's cards `lib.rs`); `state::find_instance_mut` (frozen).

## GAPS
- None of my functions left out. Names above that another part must provide with that shape; the
  likeliest misses: `param` on `HookArgs`/`ConditionContext` (if `params.rs` takes only
  `&EffectContext`), `ForEachCardArgs` field types, `after_state_check`'s argument type, `def_of`'s
  first argument (`Option<&GameState>` here; several other parts assumed `&GameState`).
- The testkit has no `card_mut`: the night bot's ports called `s.card_mut(x)` for TS's
  `stepParam(s.card(x), …)` and `s.card(x).grantedKeywords.push(…)`. I replaced every one with
  `find_instance_mut(s.state_mut(), &id)`; other bot-harvested card files (parts 9–16) likely still call
  `card_mut` and either need the same edit or a `Scenario::card_mut` added by part 31.

## Decisions
- Fixes to the night bot's ports: `ReplacementDef` (#33) built as a struct literal, not `json_as` (it has
  no serde; its test checks the fields, `instead` as JSON); `condition_met`/`preview` (#36, #40, #43)
  built with `condition_hook` over `ConditionContext` by value (the bot wrote `Arc::new(|ctx:
  &ConditionContext| …)`, the wrong signature); #42's aura rewritten on part 1's `HookArgs`/`AuraEntry`
  (the bot named `SelfArgs`/`AuraGrant`, which do not exist) and `uses: ActivationUses::Count(1)`; #40's
  `cards` callback takes `&mut EffectContext` and computes the permanents before `c.rng.shuffle` (a
  shared borrow of `c.state` inside the `DerefMut` call would not compile); #43's `after_state_check`
  takes the closure itself and `def_of` gets `Some(&*ctx.state)`; `card_mut` replaced (GAPS); TS
  `def.id`/`def.params` read with `crate::card_def(ID)` where the bot compared `ID` with itself.
- TS helpers typed `EffectContext | ConditionContext` (#36 `drawCondition`, #40 `enoughPermanents`)
  take the readings (`state`, `controller`, the threshold) instead: two Rust types; pure reads, so where
  they are taken changes nothing.
- #38's `"activate" | "sale" | null` is a private `enum Answer` in `Option`; `recalled(…) === true` is
  `.and_then(|v| v.as_bool()) == Some(true)` (works whether `recalled` hands back `Value` or `&Value`).
- #39's `outcome` returns a private `struct Outcome`; its two `forEachCard` callbacks and their `each`
  are named private fns (`stolen_ones`, `draws_owed`, `steal_it`, `draw_one`) so their signatures are
  higher-ranked without closure annotations.
- Test names: a title starting with `§x.y` gets `s` (`s2_2_…`, `s4_5_…`, renamed from the bot's `c2_2_…`);
  a title starting with a digit or `+` keeps the bot's `c` prefix (`c3_or_fewer_…`, `c2_2_for_each_…`);
  ruling tokens lead (`r97_r177_…`). TS's `toBe(base)` on scripts is ported as "both faces carry the same
  declarations" (identity has no Rust twin). `toMatchObject` is a private `matches_object` (#38).
- `expect(x.faceUp).toBe(false)` is `face_up == Some(false)` (TS's explicit false, SURFACE §4.4.7).
