# Slice: part 12 (cards lane 4), chunk 2 of 4 (#400): Classic #9–#17
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All nine hold the whole port: the TS header as `//!`, `pub const ID`, `pub fn script() -> CardScripts`
(every TS function, exported or not, in TS order), and the TS test file as `#[cfg(test)] mod tests`
(its header above it, one `#[test]` per `it`, one `mod` per inner `describe`). No `todo!`,
`unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/classic/c009_income_tax.rs` (new; 27 tests)
- `crates/cards/src/scripts/classic/c010_exile.rs` (new; 25 tests)
- `crates/cards/src/scripts/classic/c011_mind_melt.rs` (new; 16 tests)
- `crates/cards/src/scripts/classic/c012_book_of_blood.rs` (the night bot's port from 0180138, finished; 13 tests)
- `crates/cards/src/scripts/classic/c013_boots_on_the_ground.rs` (new; 18 tests)
- `crates/cards/src/scripts/classic/c014_shadowstep.rs` (new; 24 tests)
- `crates/cards/src/scripts/classic/c015_nose_hunter.rs` (new; 20 tests)
- `crates/cards/src/scripts/classic/c016_book_of_flame.rs` (new; 11 tests)
- `crates/cards/src/scripts/classic/c017_counterspell.rs` (the night bot's port from 0180138, finished; 19 tests)

There were no `.fullsend/notes/part-12-bot.md` notes. The bot's two files were read against both TS
files: every function and every `it` was there; what was fixed is under Decisions.

## SURFACE
SURFACE §7.1–§7.3 and part 1's frozen `script.rs` (which wins where SURFACE differs): hooks are
`hook(|ctx| …)` over `&mut EffectContext`; triggers are `TriggerDef::new(id, &[GameEventType::…],
|ctx, event| …).with_when(|ctx, event| …)` (TS's `ctx.event` is the second argument);
`ActivationDecl`, `ReplacementDef`, `ReplacementInstead`, `ActivationCost`, `ActivationUses` as part 1
wrote them; `resume` is `IndexMap::from([("step", hook(…))])`; `radiant: base.clone()` for TS's
`export const radiant = base`. Testkit calls follow part 5.1's notes (`unit`/`backrow`/`hand`/`pile`
return copies, `card` a reference, `expect_refused(|s| s.play(…))` returns the scenario, lanes `i32`).

## DEPENDS-ON
Every name is the TS name snake_cased at its TS module's path (rule 6), reached through
`jackioh_engine::prelude::*` in the scripts and `jackioh_engine::testkit::*` in the tests.

## GAPS
No function was left out. Names I call that other parts write, with the shape I assumed:
- `params::param(&impl ParamContext, &str) -> i32` (part 2.1): called `param(&*ctx, …)` from a
  `&mut EffectContext` hook and `param(ctx, …)` from a `&EffectContext` helper or `when`.
  `params::step_param(&mut CardInstance, &str, i32)` in tests, on `find_instance_mut(s.state_mut(), id)`.
- `query::zone_count(&GameState, PlayerId, zones::OffFieldZone) -> i32` (part 2.2), with `OffFieldZone::Hand`.
- `mana::cost_now(&GameState, &CardInstance) -> i32`; `mana::effective_cost(&GameState, &CardInstance,
  CostOptions)` called with `Default::default()` as the third argument (part 4.1).
- `replacements::replacement_of(&EffectContext) -> Option<ReplacementRecord>` with
  `flickered: Option<Vec<FlickeredCard { def_id: String, radiant: bool, .. }>>` (part 3.3). The code
  reads it as `.map(|record| record.flickered.clone().unwrap_or_default())`, so an owned record or a
  reference both compile.
- `effects::targets::cards_in_scope(&EffectContext, &BoardScope) -> Vec<CardInstance>` (owned, part
  7.1's shape; part 6.1's notes say `Option<&BoardScope>`: part 31 picks one) and
  `effects::targets::BoardScope: Deserialize` from `{ side, rows }`.
- `effects::each::{for_each_card(ForEachCardArgs) -> Effect, ForEachCardArgs { cards, each }}` (part
  6.2), built as `cards: Arc::new(move |at: &mut EffectContext<'_>| -> Vec<String>)` (ids, as part 6.2
  ported TS's `CardInstance | string`) and `each: Arc::new(|instance_id: &str| -> Effect)`. If part 6.2's
  `ForEachCardEach` takes `String`, Classic #10's closure annotation changes.
- Effect constructors, each taking one argument struct built with `json_as(json!({ …TS literal… }))`, so
  only the struct's serde keys matter: `damage { to, amount, lifesteal }`, `counter_play { to?, target }`,
  `add_to_hand { defId, radiant, costOverride }` and `{ instance, costOverride }`, `choose_from_hand { of,
  by?, count, step, prompt, data? }`, `give_from_hand { from, cards: "unchosen", costMod? }`, `exile {
  target }` (`{ of: "chosen", index }` and `{ of: "instance", instanceId }`), `choose_cost_in_hand { of,
  step, prompt }`, `exile_matching { zones, player, cost }`, `draw { count }` (the effect, `effects::draw`),
  `recruit { player }`, `exile_bottom_of_library { player, count }`, `exile_random_from_hand { player,
  count }`. `effects::choose::chosen_number(&EffectContext) -> Option<i32>`.
- Engine reads in tests: `reduce::{reduce(&GameState, &Action) -> ReduceResult { state, events, error }}`,
  `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>` (read as JSON), `state::find_instance_mut`,
  `layers::UnitView.keywords: Vec<Keyword>` (`s.stats(…)`).
- Testkit (part 5): `scenario(Value) -> Scenario`, the steps returning `&mut Scenario` (chained where TS
  chained), `state_mut()`, `view(PlayerId) -> PlayerView`, `expect_events(Value)` with an array or one
  string, `expect_in_zone(ref, "gone")` for a token that ceased to exist (Classic #14, R11).
- `crate::{register_all, card_def, CATALOG}` (part 1's `crates/cards/src/lib.rs`).

## Decisions
- The night bot's files: c012's tests called `s.card_mut(…)` (no such testkit method), took lanes as
  `usize`, and passed `expect_refused` a closure returning `()`; its hook passed `&mut EffectContext` to
  the generic `param`. c017 built `TriggerDef { when: Arc::new(|ctx: &EffectContext| … ctx.event …) }`,
  a shape part 1 did not freeze (the event is the second argument), and its tests used `card_mut` and
  `.cloned()` on owned backrow copies. All fixed; its comments, names and assertions kept.
- TS `Extract<GameEvent, { type: "cardAnnounced" }>` is a private `Announced` struct holding the fields
  the card reads (#10, #17); TS's `passOf(ctx, answered: Destroyed)` takes the answered id beside the
  event (#14). TS module constants that cannot be Rust `const`s (`ENEMY_PERMANENTS: BoardScope`,
  `targets: TargetDecl[]`) are private functions building the same value.
- #10's `budgetPicks` tests "already picked" by id (TS by identity on the same live objects): the same
  answer, since `cards_in_scope` hands back copies. `left` is lowered before the push; the order of rng
  draws is TS's (one `pick` per loop, none after the break).
- #9's `discountOf` keeps TS's `typeof value === "number" && Number.isInteger(value)` guard over the
  `data` bag (SURFACE §4.4.10). Its `data` is `{}` on the base face and `{ discount }` on the Radiant one,
  as TS wrote it.
- Tests: each file shadows `scenario` with a local one that calls `crate::register_all()` first (TS's
  vitest globalSetup; part 5.1's notes). `stepParam(s.card(x), …)` is a private `step` helper writing
  through `find_instance_mut(s.state_mut(), id)`; TS's `chalice.x = 3` / `delete none.x` the same way.
  View and event comparisons against TS literals go through `serde_json::to_value`; `toMatchObject` is
  `assert_subset` (#17) or a field read (#10); TS's option objects (`taxBoard(opts)`, `wipe(…, opts)`,
  `setup(p1, p2)` spreads) take a `Value` with TS's own keys. `expect(radiant).toBe(base)` is
  `Arc::ptr_eq` on the cry hook plus equal targets; `toBeTypeOf("function")` is `is_some()`; an
  undefined list is `is_empty()`.
- Test names: SURFACE §4.2 snake case of the title, ruling tokens kept where they stand (`r386_…`,
  `r53_r78_…`), `§` written `sec` (as the bot's two files did), every other run of punctuation one `_`.
