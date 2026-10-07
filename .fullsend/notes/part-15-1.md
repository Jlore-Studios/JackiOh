# Slice: part 15 (cards lane 7: Classic+ #19–#49), chunk 1 of 4 (#403, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All fourteen are full ports: every TS function in TS order with its header and rule comments, every
`it` a `#[test]` (counts checked by grep against the TS files), every `describe` a `mod`. No `todo!`,
`unimplemented!` or `// TODO`.
- `crates/cards/src/ky_test_bank.rs` ← `kyTestBank.ts`. The night bot's port (commit 0180138) was
  complete: its 74 problems were checked against the TS text by script (ids, statements, answers and
  wrong options identical, same order). Kept as it stood.
- `crates/cards/src/scripts/classic_plus/c020_mushroom_power.rs` ← the night bot's half port, finished:
  `param(&*ctx, …)`, closure parameter types written out, `crate::register_all()` in every scenario
  test, `card_mut` in place of a `Scenario::card_mut` the testkit does not have, the outer `describe`
  as a `mod`.
- New: `c019_1_top_loser.rs` (17 tests), `c019_2_jungle_loser.rs` (17), `c019_3_mid_loser.rs` (13),
  `c019_4_support_loser.rs` (11), `c019_5_bot_loser.rs` (17), `c019_league_of_losers.rs` (14),
  `c020_mushroom_power.rs` (9), `c021_whirlwind.rs` (13), `c022_blood_moon.rs` (22),
  `c023_dropshipping.rs` (12), `c024_crushing_walls.rs` (11), `c025_soul_shot.rs` (13),
  `c026_tommy_tempo.rs` (17), all under `crates/cards/src/scripts/classic_plus/`.

## SURFACE
§7.1 shape in every file: the TS header as `//!`, `use jackioh_engine::prelude::*;`, `pub const ID`,
`pub fn script() -> CardScripts`; the TS test header as `//` above `#[cfg(test)] mod tests`, which
does `use super::*; use jackioh_engine::testkit::*;`. Testkit calls follow part 5.1's notes (the
testkit's author): `unit`/`backrow`/`hand` answer owned copies, `card()` a `&CardInstance`,
`state_mut()`, seats as `PlayerId`.

## GAPS
Nothing left unported. Names called in other parts' modules (TS name snake_cased at its TS module's
path), with the shape assumed; the likeliest to need an edit in Wave 3 first:
- `effects::each::{for_each_card, ForEachCardArgs { cards, each }}` (part 6.2): `cards` is
  `Arc<dyn Fn(&mut EffectContext) -> Vec<String> + Send + Sync>` (`&mut`: Jungle Loser's and Soul
  Shot's lists draw from `ctx.rng`), `each` is `Arc<dyn Fn(&str) -> Effect + Send + Sync>`. Every
  closure handed to them has its parameter types written out (`|each: &mut EffectContext<'_>|`,
  `|instance_id: &str|`), so a `String` parameter for `each` is a one-token edit per file.
- `effects::kill_credit::{with_kill_credit, WithKillCreditArgs { killer: TargetSpec, pairs, transfer:
  bool, during: Effect, then: Option<…> }}` (part 6.1): `pairs` as `Arc<dyn Fn(&EffectContext,
  &CardInstance) -> Vec<KillCredit> + Send + Sync>`, `then` as `Arc<dyn Fn(&KillCredit) -> Vec<Effect>
  + Send + Sync>`; `kill_credit::KillCredit { victim_id, to_id }` built as a struct literal.
- `params::param(&impl ParamContext, &str) -> i32`, called `param(&*ctx, key)` (part 2.1).
- `effects::targets::{adjacent_to(&EffectContext, &TargetSpec, &BoardScope), cards_in_scope(
  &EffectContext, &BoardScope) -> Vec<CardInstance>, ScopeSide::{Any, Enemy}}` (part 7.1); `ScopeSide`
  must be `Copy` (Crushing Walls' list closure captures it inside an `Fn` hook) and `Serialize`.
- `combat::{random_attack_targets(&GameState, &CardInstance, AttackAmong) -> Vec<_>,
  AttackAmong::EnemyUnits}` (part 3).
- `zones::{slots_of(PlayerId, Row) -> Vec<ZoneSlot>, card_at(&GameState, &ZoneRef) ->
  Option<&CardInstance>, is_locked(&GameState, &ZoneRef), is_carried(&GameState, &CardInstance),
  slot_of(&GameState, &CardInstance) -> Option<ZoneSlot>, lock_zone/reserve_zone(&mut GameState,
  &ZoneRef), place_on_field(&mut GameState, &mut CardInstance, &ZoneRef, PlaceOnFieldOptions) -> bool}`
  (part 2.1); every slot is passed as `&ZoneRef`, which its `impl Into<ZoneSlot>` takes.
- `prompts::summoned_so_far(&EffectContext) -> Vec<String>`; `resolve::{make_context(&mut EngineSink,
  Option<&CardInstance>, HookOptions) -> EffectContext, HookOptions { controller: Option<PlayerId>, .. }:
  Default, apply_effects(&[Effect], &mut EffectContext)}` (part 3.2; tests only).
- `layers::{unit_view, card_keywords}`, `tuning::{numbered_sum(&[Keyword], KeywordKind) -> Option<i32>,
  tuning_of, add_step}`, `mana::cost_now`, `restrictions::is_berserk(&CardInstance)`,
  `cast_on_draw_now::is_cast_on_draw(&GameState, &CardInstance)`, `faces::card_type_of`,
  `params::{step_param, set_param}`, `scripts::registered_scripts() -> IndexMap<String, CardScripts>`,
  `catalog::{query(&CatalogQueryArgs) -> Vec<&'static CardDef>, pick_generated(&mut Rng, &[&CardDef],
  Option<&GameState>)}` (part 2).
- Effect verbs, each built with `json_as(json!({ …TS literal… }))`, so only their serde shape matters:
  `buff`, `next_turn_mana`, `heal`, `forced_attack_random`, `forced_attack_own_hero`, `go_berserk`,
  `summon`, `trigger_cry`, `damage_all`, `bounce`, `add_random_from_catalog`, `give_brittle`, `destroy`,
  `end_turn`, `end_turn_after_actions`, `set_health` (tests), `effects::draw` (tests, named by path:
  the bare name is ambiguous in a test module).
- Testkit (part 5): `scenario`, `Scenario::{play, attack, end_turn, start_turn, switch_position,
  activate, state, state_mut, events, last_events, view, unit, backrow, hand, card, stats,
  expect_in_zone, expect_stats, expect_health, expect_refused, expect_refused_with}`,
  `register_scripts` (the override). A TS test that wrote through a live instance (`s.card(x).…`,
  `stepParam(s.card(x), …)`, `tuningOf(top(s))`) writes through `find_instance_mut(s.state_mut(), id)`
  in a private `card_mut`/`top_mut`/`loser_mut` helper, since the testkit has no `card_mut`.

## Decisions
- TS factories (`jungle(transfer)`, `league(radiant)`, `dropship(radiant)`, `walls(side)`) are private
  fns returning `Script`; TS's module constant `onKill` is `fn on_kill() -> TriggerDef`, built once in
  `script()` and cloned onto both faces. TS named hooks (`attacks`, `cry`, `berserkAttack`,
  `addedByThisList`, `pick`) are private fns used as `hook(f)` or `Arc::new(f)`.
- `export const radiant = base` is `base.clone()`; its test `expect(radiant).toBe(base)` is
  `Arc::ptr_eq` on a shared hook, or equal static flags where the script has no hook (Top Loser).
- `ctx.self` (TS's live object) is `ctx.live_self()` where TS read the card's current zone, flags or
  keywords (Jungle Loser, Mid Loser, Bot Loser's Berserk, Whirlwind's return, Soul Shot's Lucky), and
  `ctx.self_` where only its id matters (Bot Loser's kill trigger, Tommy Tempo).
- Soul Shot's Lucky picks need the state (the R414 comparator) and the rng at once: they borrow the
  sink's two fields apart (`&*ctx.sink.state`, `&mut *ctx.sink.rng`). `Number.MAX_SAFE_INTEGER` for a
  card on no lane is `i32::MAX` (only compared with lanes 1–5).
- Crushing Walls' TS `side: "any" | "enemy"` parameter is `ScopeSide` (`Any`, `Enemy`).
- Blood Moon's replacements are `ReplacementDef` literals (no `Default`: it holds a hook).
- `ky_test_bank.rs`: `KY_TEST_BANK` stays a `LazyLock<Vec<KyTestProblem>>` (a problem holds `String`s,
  so no `const`), read-only after first use like the crate's `CATALOG`.
- Tests: names are the titles snake_cased (`§` → `s`, `#` → `n`, a leading digit `t_`), R-ids leading;
  the outer `describe("C+ #N …")` is a `mod c_nN_…`. Every scenario test calls `crate::register_all()`
  first (part 5.1). TS `{ ...defaults, ...overrides }` side setups are a private `spread` over JSON
  objects; `toMatchObject` a private `matches_object` over JSON; `indexOf`/`findIndex` keep TS's −1;
  `s.unit(p, lane) ?? ""` is a private `unit_or_blank`; TS regexes are hand checks (`has_token_index`
  for `/-\d{3}-\d/`). Dropshipping's `try { s.play(unit) } catch { continue }` is `catch_unwind` around
  the step. Seeds are TS's, and `createRng(seed, cursor)` is `Rng::new(&seed, cursor)`.
