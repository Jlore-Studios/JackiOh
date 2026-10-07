# Slice: part 14 (cards lane 6), chunk 1 of 4 — Classic+ #1–#12.7 (#402, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
Each holds the whole port of its TS script and TS test file (every function in TS order, the header
as `//!`, every rule/ruling comment kept; one `#[test]` per TS `it`, one `mod` per inner `describe`,
the card's outer `describe` being `mod tests` itself; the TS test header above `#[cfg(test)]`):
- `crates/cards/src/scripts/classic_plus/c001_doom_shroom.rs` — the night bot's half-done port
  (0180138), finished in place: see Decisions.
- `crates/cards/src/scripts/classic_plus/c002_groom_shroom.rs`
- `crates/cards/src/scripts/classic_plus/c003_second_amendment_snake.rs`
- `crates/cards/src/scripts/classic_plus/c004_juhan_biggest_bat.rs`
- `crates/cards/src/scripts/classic_plus/c005_guy_att.rs`
- `crates/cards/src/scripts/classic_plus/c006_wrong_house_attacker.rs`
- `crates/cards/src/scripts/classic_plus/c007_the_house.rs`
- `crates/cards/src/scripts/classic_plus/c008_withering_storm.rs`
- `crates/cards/src/scripts/classic_plus/c009_silence.rs`
- `crates/cards/src/scripts/classic_plus/c010_new_wraps.rs`
- `crates/cards/src/scripts/classic_plus/c011_anime_armor.rs`
- `crates/cards/src/scripts/classic_plus/c012_1_devour.rs`
- `crates/cards/src/scripts/classic_plus/c012_2_death_boil.rs`
- `crates/cards/src/scripts/classic_plus/c012_3_fluffy_grip.rs`
- `crates/cards/src/scripts/classic_plus/c012_4_powder_spray.rs`
- `crates/cards/src/scripts/classic_plus/c012_5_anti_waffle_shell.rs`
- `crates/cards/src/scripts/classic_plus/c012_6_frozen_wastes.rs`
- `crates/cards/src/scripts/classic_plus/c012_7_legion_of_the_hungry.rs`

No `todo!`, `unimplemented!` or `// TODO` in any of them. Nothing left out.

## SURFACE
Every file: `use jackioh_engine::prelude::*;`, `pub const ID`, `pub fn script() -> CardScripts`
(SURFACE §7.1). `export const radiant: Script = base` is `base.clone()`. Hooks are `hook(|ctx| …)`
over part 1's `&mut EffectContext`; typed hooks use part 1's builders (`condition_hook` for `preview`,
`read_hook` for `heroGuard`, `aura_hook` for `aura`); triggers are `TriggerDef::new(id, &[type], run)
.with_when(when)`. Effect arguments are the TS object literals through `json_as(json!({ … }))`; a TS
`= {}` default is `Default::default()`.

## DEPENDS-ON / GAPS
Nothing was left out. Names called in other parts' modules, with the shape I called them in (TS name
snake_cased at its TS module's path, rule 6); part 31 reconciles:

Engine (through `prelude::*`):
- `combat::{attack_target_of(&GameState, &str) -> Option<AttackTarget>, AttackTarget}` with
  `AttackTarget = damage::DamageTarget` and variants `Hero { player }` / `Unit { instance }`, matched
  through the alias (C+ #1, #2). C+ #12.2 names `jackioh_engine::damage::DamageTarget` directly
  (the prelude leaves `damage` out).
- `zones::{fill_board_zones(&GameState, PlayerId) -> Vec<ZoneSlot>, card_at(&GameState, &ZoneSlot)
  -> Option<&CardInstance>, first_free_zone(&GameState, PlayerId, Row) -> Option<ZoneSlot>,
  active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>` (owned also compiles: only `.iter()` and
  `.id` are used)`, OffFieldZone::Library, ZoneSlot { player, row, lane }` (built as a struct literal,
  also in a `const`). Slots are passed as `&ZoneSlot` (works with part 2.1's `impl Into<ZoneSlot>`).
- `query::{zone_cards(&GameState, PlayerId, OffFieldZone) -> Vec<CardInstance>, zone_count(&GameState,
  PlayerId, OffFieldZone) -> i32}` (C+ #8, #12.6, #12.7; `zone_count`'s `i32` is assumed: it is
  `.min`ed with an `i32`).
- `params::param(&impl ParamContext, &str) -> i32` called as `param(&*ctx, …)` on a context and
  `param(&args, …)` on `HookArgs`.
- `layers::unit_view(&GameState, &CardInstance)` (`.health`, `.keywords`); `faces::card_type_of(
  &GameState, &CardInstance) -> CardType`; `wire::has_keyword(&[Keyword], KeywordKind)`.
- Effects: `exile_all(BoardScope)`, `destroy_all(BoardScope)`, `lock_own_zone()`, `summon_random`,
  `summon`, `for_each_card(ForEachCardArgs { cards: ForEachCardCards, each: ForEachCardEach })` (part
  6.2's names), `grant_keyword`, `place_plague`, `damage_split`, `transform_beneath(Default)`,
  `degrade(TuneArgs)`, `draw`, `applicable_changes(&GameState, &CardInstance, TuneDirection) -> Vec<_>`
  (`.is_empty()`), `TuneDirection::Degrade`, `vanilla(Default)`, `set_radiant(Default)`,
  `instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>`, `resolve_target(&EffectContext,
  &TargetSpec) -> Option<DamageTarget>`, `destroy`, `damage`, `heal`, `take_from_library`,
  `damage_all`, `exile`. Every argument type must deserialize from TS's literal (SURFACE §6.6).
- Wire: `PreviewValue: Deserialize` (built with `json_as` so its optional fields stay absent),
  `TargetDecl`, `Param`, `Keyword`, `ActionBody`, `PlayerView` and `GameEvent: Serialize` (the
  tests compare them as JSON).
Tests (`jackioh_engine::testkit`, part 5's notes): `scenario(Value) -> Scenario`; `Scenario::{play,
attack, end_turn, start_turn, expect_in_zone, expect_stats, expect_events, expect_health,
expect_refused(|s| s.play(…)), unit/backrow(seat, i32) -> Option<CardInstance>, hand(seat) /
pile(seat, &str) -> Vec<CardInstance>, card(ref) -> &CardInstance, stats(ref), view(seat) ->
PlayerView, state(), state_mut(), events()}`; `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`;
`state::find_instance_mut`; `params::step_param(&mut CardInstance, &str, i32)`; `catalog::def_of(
Option<&GameState>, &str)` (part 2.2's shape; `.clone()`d, so a reference or an owned def both work);
`mana::effective_cost(&GameState, &CardInstance, Default::default())`; `zones::{lock_zone,
reserve_zone}(&mut GameState, &ZoneSlot)`, `zones::beneath_at(&GameState, &ZoneSlot)` (`.to_vec()`,
`.first()`, `.is_empty()`: a slice or a `Vec` both work), `zones::carried_at(&GameState, &ZoneSlot)
-> Option<&CardInstance>`. Cards crate: `crate::register_all()`, `crate::card_def(&str) -> CardDef`.

## Decisions
- **C+ #1 (the bot's port), finished in place**: kept its script and test structure; added
  `crate::register_all()` to every test (part 5: the testkit cannot register the cards itself);
  `expect_refused`'s closure now returns the step (`|s| s.play(…)`, part 5's
  `FnOnce(&mut Scenario) -> &mut Scenario`); `unit()` answers owned copies, so the bot's
  `.unwrap().clone()` is `.unwrap()`; test names with apostrophes spelled by the rule below.
- Every `#[test]` starts with `crate::register_all();` (idempotent).
- Test names: the `it` title lower-cased, `§` → `s`, `#` → `n`, every other run of non-alphanumerics
  (apostrophes included) → `_`, ruling tokens leading; a name that would start with a digit takes `t_`.
- TS `expect(radiant).toBe(base)` (one object for both faces): `Arc::ptr_eq` on a hook both faces
  carry, since `radiant = base.clone()` shares the `Arc`s. C+ #6 has no hook: both faces are checked
  empty field by field (TS `toEqual({})`).
- A TS module constant holding one `Effect` (#10's `giveReborn`) is built once in `script()` and
  cloned into both faces' hooks; a module `Hook` constant (#7's `summonOne`) is built once and its
  `Arc` shared by `cry` and `startOfTurn`. Object constants with no function (`FELINOR_UNITS`) are a
  private `fn` returning the literal.
- `forEachCard`'s two callbacks go through private `cards_of`/`each_of` (typed `Arc::new`), so the
  closures' signatures are inferred; `cards` answers ids, as part 6.2's `ForEachCardCards` does.
- `slice(0, n)` with `n` a declared number is `take(n.max(0) as usize)`; #8's `Math.min(param, len)`
  likewise clamps at 0 (no catalog value is negative). `ctx.rng.shuffle` is always handed a list
  read before it (`zone_cards` first), so the draws are TS's in TS's order.
- Reads of `ctx.state` next to a draw from `ctx.rng` are taken into locals first (one borrow of the
  context at a time).
- #12.6's factory takes `impl Fn(PlayerId) -> PlayerId + Copy` (TS `(controller) => PlayerId`).
- Tests: `toMatchObject` → a private `matches_object` over JSON, or a `matches!` on `Zone::Field {
  row, lane, .. }`; `stepParam(s.card(x), …)` → a private `step` through `find_instance_mut(
  s.state_mut(), id)`; TS's `{ ...base, ...p1 }` → a private shallow `merged`; an optional TS `seed`
  is set on the options only when given; `JSON.parse(JSON.stringify(state))` equality (#8) compares
  the JSON of a serde round trip (no `GameState: PartialEq` needed). `s.stats(s.unit(…) ?? "")` passes
  the id or `""`, as TS did.
- #12.5's aura `applies` is part 1's one `Box` (`AuraEntry`), closing over an owned `IndexSet` of ids.
