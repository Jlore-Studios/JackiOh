# Slice: part-11 (cards lane 3: Core #82–#100 and the Core tokens to Ghoul)
BUILDS-RUN: 0

## FILES
(filled in at the end)

## SURFACE
Every file: `pub const ID: &str` and `pub fn script() -> CardScripts` (SURFACE §7.1). Nothing else is public.

## DEPENDS-ON
(filled in at the end)

## GAPS
(filled in at the end)

## DECISIONS
(filled in at the end)

## CONVENTIONS (every file in this lane follows these; the reference port is `core/t_ghoul.rs`)

### Script half

- Header: the TS file's header comment, verbatim, as `//!` lines (the `import`s dropped).
- Imports: `use jackioh_engine::prelude::*;`, then `use jackioh_engine::effects::{…};` naming each effect
  constructor the file calls, `use jackioh_engine::subsystems;` when it calls one, `use std::sync::Arc;`
  when it builds a typed hook. Never `use` anything from another card file.
- `pub const ID: &str = "<the id cardDef(...) names>";`. TS `def.id` in a script is `ID`; a TS
  `cardDef("core-093-1").id` used as a value is a private `const COMBO_FODDER: &str = "core-093-1";`.
- `pub fn script() -> CardScripts { let base = …; let radiant = …; CardScripts { base, radiant } }`.
  `export const radiant: Script = base;` is `radiant: base.clone()`. A TS factory (`virus(face)`) is a private
  `fn virus(face: Face) -> Script`. TS constant objects (`{ base: 1, radiant: 2 } as const`) become private
  `const`s or a small private `#[derive(Clone, Copy)] struct` with `const BASE: Face = Face { … }`.
- `Script { cry: Some(hook(…)), ..Script::default() }`. Field names snake_cased from TS. List-valued fields are
  `Vec` (`targets`, `modes`, `triggers`, `hand_triggers`, `activations`, `replacements`); `resume` is
  `IndexMap<&'static str, Hook>` (`IndexMap::from([("copies", hook(…))])`); every other field is an `Option`
  (`static_flags: Some(StaticFlags { cast_on_draw: Some(true), ..StaticFlags::default() })`).
- Hooks: `hook(|ctx| vec![…])` / `hook(move |ctx| …)` with `ctx: &EffectContext` (SURFACE §6.6). Typed hooks are
  `Some(Arc::new(|ctx: &ConditionContext| -> bool { … }))` for `condition_met`,
  `Some(Arc::new(|ctx: &ConditionContext| -> Vec<PreviewValue> { … }))` for `preview`,
  `Some(Arc::new(|args: &CostArgs| -> i32 { … }))` for `cost` (TS `{ state, instance }`), and so on: the closure
  argument is a reference to the TS argument type by its TS name (an anonymous TS object type gets the name
  `<Field>Args`, e.g. `CostArgs`, `SetStatArgs`), and the field type is the alias part 1 writes.
- Triggers (`TriggerDef`, and `TrapTrigger` which is the same type):
  `TriggerDef { id: "fed-fauci-plague".into(), on: vec![GameEventType::Damage],
  when: Some(Arc::new(|ctx: &TriggerContext| -> bool { … })), run: Arc::new(|ctx: &TriggerContext| -> Vec<Effect> { … }) }`.
  `TriggerContext` is TS's `EffectContext & { event: GameEvent }`: `ctx.event` is the `GameEvent`, and every
  `EffectContext` field reads straight off `ctx` (it derefs to `EffectContext`).
- Context fields: `ctx.state` (read it as `&ctx.state` wherever a `&GameState` is wanted), `ctx.controller`
  (`PlayerId`, Copy), `ctx.self_` (TS `self`: `Option<CardInstance>`; in `ConditionContext` it is a plain
  `CardInstance`), `ctx.radiant`, `ctx.targets` (`Vec<Selection>`), `ctx.modes` (`Vec<String>`), `ctx.x` (`i32`),
  `ctx.data` (`IndexMap<String, Value>`), `ctx.zone` (`ConditionZone::Hand | Field`), `ctx.your_turn`.
- Events: `GameEvent` is a `#[serde(tag = "type")]` enum; a variant is the TS `type` in PascalCase, fields snake_cased:
  `if let GameEvent::Damage { target_id, .. } = &ctx.event { … }`. `GameEventType::<PascalCase>` names a type.
- String unions are enums with PascalCase variants of the literal: `OffFieldZone::Graveyard`, `CardType::FieldSpell`,
  `PlayerId::P1`. A literal used where TS passes it inside an object literal stays a string inside `json!`.
- Effects: every constructor takes its args struct; build it from the TS object literal with
  `json_as(json!({ …the TS literal, keys as in TS (camelCase)… }))`. A TS spread `...(cond ? {} : { k: v })` is a
  `let mut args = json!({…}); if !cond { args["k"] = json!(v); }`. An argument field that holds a function
  (a filter callback) is set in Rust on the struct after `json_as` (`let mut a: XArgs = json_as(…); a.filter = Some(Arc::new(…));`).
- Engine reads go through the read helpers by their TS names snake_cased (`zone_cards(&ctx.state, p, OffFieldZone::Hand)`,
  `def_of(&ctx.state, &card)`, `find_instance(&ctx.state, id)`), `&GameState` first, ids as `&str`, cards as
  `&CardInstance`, players by value. Never index `state.players` (CLAUDE.md rule 5). `param(&args, "cap")`.
  Subsystems: `subsystems::<ts_name_snake>(…)`. Constants keep their TS names (`RESUME_HOOK`, `THIS_TURN`).
- `cards/src/query.ts` is `crate::query::{query, pool, TRAP_TYPES, …}` (part 9); its `catalog.query(...)`/`catalog.pool(...)`
  object methods are the module functions `crate::query::query(...)`/`crate::query::pool(...)`.
- Numbers: §4.3 and §4.4 (game numbers `i32`, `Math.max(0, x)` → `x.max(0)`, truthiness made explicit).

### Test half (`#[cfg(test)] mod tests`)

- The TS test file's header comment as `//` lines directly above `#[cfg(test)]`.
- `use super::*; use jackioh_engine::testkit::*;` plus the effect constructors the tests call.
  `const P1: PlayerId = PlayerId::P1; const P2: PlayerId = PlayerId::P2;` and the TS test's own constants.
- One `#[test]` per TS `it` (an `it.each` row is one `#[test]` per row), one nested `mod` per `describe`
  (`mod x { use super::*; … }`). Names: the title lower-cased, `§` → `s`, `#` → `n`, every other run of
  non-alphanumerics → one `_`, trimmed; a name that would start with a digit gets `t_`. Ruling tokens stay leading
  (`it("R90 R81 …")` → `fn r90_r81_…`).
- Every `#[test]` body starts with `crate::register_all();` (idempotent; the TS globalSetup's job).
- TS `def` (from the card file or `cardDef(id)`) is `crate::card_def(id)` → `&'static CardDef` (part 1's lib.rs,
  where `catalog-data.ts` ports). TS `CATALOG` is `registered_catalog()` (`&'static CardDefs`, an
  `IndexMap<String, CardDef>`). TS `base`/`radiant` imported from the script are `script().base`/`script().radiant`.
- Scenario (SURFACE §8): `let mut s = scenario(json!({ … }));` with the TS options literal copied in.
  `s.play(card, json!({…}))` (`json!({})` for none), `s.attack(&a, &b)` / `s.attack(&a, "hero")`, `s.answer(json!(…))`,
  `s.end_turn()`, `s.start_turn()`, `s.switch_position(&c)`, `s.activate(&c, json!({…}))`. Each returns `&mut Scenario`
  so TS chains port as chains. A card argument is a `&str` (id, index or name) or a `&CardInstance`.
  Reads: `s.state()` (`&GameState`), `s.state_mut()`, `s.events()` / `s.last_events()` (`&[GameEvent]`),
  `s.view(Some(P1))` (`PlayerView`; `None` = the TS no-argument call), `s.unit(P1, 1)` / `s.backrow(P1, 1)`
  (`Option<&CardInstance>`: take `.cloned().unwrap()` before the next mutation), `s.hand(Some(P1))` /
  `s.pile(P1, "graveyard")` (`Vec<CardInstance>`), `s.card(r)` (`&CardInstance`), `s.stats(&c)` (`UnitView`).
  Assertions: `s.expect_in_zone(&c, "graveyard")`, `s.expect_stats(&c, json!({…}))`,
  `s.expect_events(json!(["damage", …]))`, `s.expect_health(P2, 27)`, `s.expect_mana(P1, 3)`,
  `s.expect_refused(|s| { s.play(…); })`, `s.expect_refused_with(|s| { … }, "text")`.
- Glow (`_glow.ts`): `glow::hand_glows(&s, &id, P1)`, `glow::backrow_glows(&s, lane, P1)`,
  `glow::opponent_sees_glow(&s, lane, P1)` (TS default arguments written out).
- `expect(x).toBe(y)` / `toEqual` → `assert_eq!`; on engine values compare their JSON with a private
  `fn js<T: serde::Serialize>(v: &T) -> Value` when the TS compares against an object literal.
  `toMatchObject` → a private `fn matches_object(actual: &Value, pattern: &Value) -> bool` (copy t_ghoul's).
  `toContain` → `.contains(&…)`; `toHaveLength(n)` → `.len() == n`; `toBeUndefined`/`toBeNull` → `is_none()`.
  A TS failure message (`expect(x, "msg")`) is the `assert!` message.
- Engine internals in tests (`makeContext`, `applyEffects`, `createRng`, `placeOnField`, `newInstance`,
  `registerScripts`) are the same names snake_cased from the testkit, which re-exports the whole engine.
  `createRng(seed)` → `Rng::new(seed, 0)`; `createRng(seed, cursor)` → `Rng::new(seed, cursor)`.
  A TS `run(s, effects)` helper (sink + `makeContext` + `applyEffects`) ports as t_ghoul's private `run`.
- Seeds stay exactly as in TS (the Rust RNG is bit-identical, SURFACE §6.3).
