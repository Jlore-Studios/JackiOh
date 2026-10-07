# Slice: part 12 (cards lane 4), chunk 3 of 4 — Classic #18–#27 (#400, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All ten were absent on `staging` (no bot port, no `part-12-bot.md` notes); each is a full port of its TS
script and test file (every function in TS order, the headers as `//!` / `//` lines, every R-id and
§-id the TS cites — checked by grep: the R-id sets of each Rust file equal its TS pair's; one `#[test]`
per TS `it`, counts checked: 20, 17, 26, 23, 19, 21, 11, 20, 12, 11). No `todo!`, `unimplemented!` or
`// TODO`.
- `crates/cards/src/scripts/classic/c018_glitch_in_the_system.rs` (classic-018)
- `crates/cards/src/scripts/classic/c019_lizards_breath.rs` (classic-019)
- `crates/cards/src/scripts/classic/c020_the_power_to_punish.rs` (classic-020)
- `crates/cards/src/scripts/classic/c021_turtinator.rs` (classic-021)
- `crates/cards/src/scripts/classic/c022_mid_runner.rs` (classic-022)
- `crates/cards/src/scripts/classic/c023_devils_pact.rs` (classic-023)
- `crates/cards/src/scripts/classic/c024_book_of_knowledge.rs` (classic-024)
- `crates/cards/src/scripts/classic/c025_lag_in_the_system.rs` (classic-025)
- `crates/cards/src/scripts/classic/c026_rapid_draw.rs` (classic-026)
- `crates/cards/src/scripts/classic/c027_pestilent_slime.rs` (classic-027)

## SURFACE
§7.1 shape: `pub const ID`, `pub fn script() -> CardScripts`, `use jackioh_engine::prelude::*;` plus
an explicit `use jackioh_engine::effects::{…}` of every verb the file calls (an explicit import beats the
prelude's globs). Part 1's frozen types used as compiled: `Script`, `ActivationDecl`, `ActivationUses`,
`ActivationCost`, `ModeDecl`/`PromptKind`, `TargetDecl::target` / `json_as::<TargetDecl>`, `PreviewValue`,
`condition_hook`, `read_hook`, `hook`, `EffectContext` (hooks take `&mut EffectContext`), `Zone`, `Row`,
`GameEvent`/`GameEventType`, `Action::new`, `ActionBody::{Play, Activate, Answer}`, `CardInstance`
fields, `PlayerState.{backrow, turn_log, mana}`, `GameState.{pending, active, marks, work, turn}`.

## GAPS
Names I call that other parts provide (TS name snake_cased at its TS module's path), with the shape
I assumed. Part 31 reconciles.

Effects (parts 6, 7):
- `effects::each::{for_each_card, ForEachCardArgs { cards, each }}` with `cards: Arc<dyn Fn(&mut
  EffectContext) -> Vec<String> + Send + Sync>` (ids, per part 6.2's notes) and `each: Arc<dyn Fn(&str)
  -> Effect + Send + Sync>`, built by struct literal with `Arc::new(…)`. `&mut` because #22's set draws
  from `ctx.rng` (TS `c.rng.shuffle(…)`); if part 6.2 made it `&EffectContext`, #22 must take the rng
  from a `&mut` (the other three files only read). #26 passes a fn item (`Arc::new(whole_hand_if_no_choice)`).
- `effects::targets::{cards_in_scope(&EffectContext, &BoardScope) -> Vec<CardInstance>, sides_of(
  &EffectContext, Option<ScopeSide>) -> Vec<PlayerId>, BoardScope: Deserialize, ScopeSide::Enemy}`.
- Built from TS's literal with `json_as(json!(…))`, so only `Deserialize` is assumed: `exile`, `bounce`,
  `discard` (`{ target: { of: "instance", instanceId } }`), `sacrifice` (`{ target: { of: "self" } }`),
  `damage` (`{ to: { of: "chosen" }, amount }`), `draw` (`{ count }`), `gain_mana` (`{ amount }`),
  `discard_random` (`{ count, player? }`), `discard_hand` (`{ player: "self" }`),
  `destroy_at_next_turn_start` (`{ target, mark }` / `{ scope, mark }`), `add_player_modifier`
  (`{ mod: { kind: "replacePlays", defId, radiant, expiry: { until: "thisTurn", turn } } }`).
Engine (parts 2–5, 8):
- `params::param(&impl ParamContext, &str) -> i32` (part 2.1's notes), called as `param(&*ctx, …)` on a
  hook's `&mut EffectContext`, `param(ctx, …)` on a `&EffectContext`, `param(&args, …)` on `HookArgs`
  and `ConditionContext`. #22's private `enough_mana(ctx: &impl ParamContext, …)` names the trait
  `ParamContext` (part 2.1's name), as TS's `EffectContext | ConditionContext` argument.
- `params::step_param(&mut CardInstance, &str, i32)` (tests, on `find_instance_mut(s.state_mut(), id)`).
- `query::{zone_cards(&GameState, PlayerId, OffFieldZone) -> Vec<CardInstance>, zone_count(&GameState,
  PlayerId, OffFieldZone) -> i32, unspent_mana_of(&GameState, PlayerId) -> i32, hero_of(&GameState,
  PlayerId).health}`; `zones::{OffFieldZone::{Hand, Library, Graveyard, Exile}, slot_of(&GameState,
  &CardInstance) -> Option<ZoneSlot> (.lane: i32), midlane_lanes_of(&GameState, PlayerId) -> Vec<i32>,
  midlane_lanes(<integer>) -> Vec<<integer>>}`. `zone_count` is assumed `i32` (SURFACE §4.3's count);
  #19, #23 and #26 compare it with `param`'s `i32` and #19 puts it in `PreviewValue.value`.
- `mana::cost_now(&GameState, &CardInstance) -> i32`.
- `subsystems::activate::{activation_paid(&EffectContext) -> ActivationPaid { tributed: Vec<TributedUnit
  { attack: i32, .. }> }, ACTIVATIONS_MEMORY_KEY: &str}` (as `subsystems::…`); `config::ACTIVATE_UNLIMITED_CAP`.
- Testkit (part 5): `scenario`, `Scenario::{play, activate, answer, attack, end_turn, state, state_mut,
  events, last_events, view, unit, backrow, hand, pile, card, expect_*}` as part 5.1's notes give them;
  `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`, `reduce(&GameState, &Action) ->
  ReduceResult`, `find_instance`, `find_instance_mut` through `testkit::*`.
- `crate::register_all()` and `crate::card_def(id) -> CardDef` (part 1's cards `lib.rs`).

Not ported: nothing. TS's `def` export is not ported (SURFACE §7.1: the def comes from the catalog by
`ID`); tests read it as `crate::card_def(ID)`.

## Decisions
- `expect(radiant).toBe(base)` (#21, #22, #24, #26, #27) is `Arc::ptr_eq` on a hook of the two faces:
  `radiant: base.clone()` shares the closures, which is the Rust form of "the same script object".
- TS default arguments are written out (`eat(s, tribute, targets, who)`, `setup(p1, p2, radiant)`,
  `hand_defs(s, P1)`); TS spreads of a side setup over defaults (`{ …defaults, ...p1 }`) are a private
  `spread(defaults, side)` that inserts the side's keys over the defaults.
- TS helpers whose names would shadow the card file's own functions in the test module are renamed:
  #20's test `punish(s, mode, targets?)` is `activate_punish`. A `stepParam(s.card(x), …)` write on the
  live instance is a private `step(s, card, key, steps)` over `find_instance_mut(s.state_mut(), id)`;
  `s.card(x).x = 3` and `.memory[KEY] = …` likewise.
- TS `ctx.self` in #22's Cry reads the live card (`ctx.live_self()`); `self.zone.z === "field" &&
  self.zone.row === "units"` is `matches!(zone, Zone::Field { row: Row::Units, .. })`.
- #22's shuffle draws through `c.sink.rng` (two sink fields at once, part 7's convention); TS
  `slice(0, bounces)` is `take(bounces.max(0) as usize)`.
- #19's ranking sorts with the stable `sort_by` (SURFACE §4.4.1); `PileName` is a private enum (TS's
  string union) with `as_str` for the preview's `display`.
- #18's `GLITCH_OPTIONS`/`NUMBER_CHOICE` are private fns (a `Vec<String>` cannot be a `const`).
- Test assertions on TS object literals compare JSON (`serde_json::to_value`) against `json!(…)`;
  `toMatchObject` on an event is a `match` on its `GameEvent` variant; `expect.arrayContaining` is a
  private `contains_all`; `Object.keys(event).sort()` reads the serialised event's keys.
- One nested `mod` per TS `describe` (the file's outer describe included), `use super::*;` in each;
  test names by the part 11 convention (`§` → `s`, `#` → `n`, R-ids leading).
