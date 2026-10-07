# Slice: part 12 (cards lane 4), chunk 1 of 4 — Classic #1–#8 (#400, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All eight were absent on `staging` (no bot work to finish, no `part-12-bot.md`); all are full ports,
each the TS script (header as `//!`, every function in TS order with its doc and rule comments) and,
below it, the TS test file (header as `//` above `#[cfg(test)]`, one `mod` per `describe`, one `#[test]`
per `it`; counts checked against the TS: 21, 17, 11, 28, 21, 19, 21, 25). No `todo!`, `unimplemented!`
or `// TODO`.
- `crates/cards/src/scripts/classic/c001_curse_of_the_forgotten_classic.rs` (`classic-001`)
- `crates/cards/src/scripts/classic/c002_the_trickster.rs` (`classic-002`)
- `crates/cards/src/scripts/classic/c003_book_of_heal.rs` (`classic-003`)
- `crates/cards/src/scripts/classic/c004_palantir.rs` (`classic-004`)
- `crates/cards/src/scripts/classic/c005_tesla.rs` (`classic-005`)
- `crates/cards/src/scripts/classic/c006_cloaked_toe_cracker.rs` (`classic-006`)
- `crates/cards/src/scripts/classic/c007_infiniscepter.rs` (`classic-007`)
- `crates/cards/src/scripts/classic/c008_pickle.rs` (`classic-008`)

## SURFACE
Each file: `pub const ID: &str = "classic-00N";` and `pub fn script() -> CardScripts`; nothing else is
public. Part 1's frozen shapes are used where SURFACE differs (hooks take `&mut EffectContext`,
`TriggerDef::new(id, &[types], |ctx, event| …).with_when(|ctx, event| …)`, the `HookArgs`/
`ConditionContext`/`TargetCheckArgs` bundles by value, `read_hook`/`condition_hook`/`target_check`).

## GAPS
Called in other parts' modules (TS name snake_cased at its TS module's path; the shape assumed). The
effect constructors all take their argument struct built with `json_as(json!(…TS literal…))`, so only
the serde shape (TS's) and the function name matter:
- effects (parts 6–7), imported by name from `jackioh_engine::effects`: `damage`, `draw`,
  `forced_attacks`, `recruit` (C #1); `add_cost_rule` (C #2); `heal` (C #3); `counter_play`,
  `remember`, `sacrifice` (C #4); `animate`, `damage` (C #5); `gain_mana` (C #6); `cast_new`, `exile`,
  `instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>` (TargetSpec built with
  `json_as(json!({ "of": "chosen" }))`), `remember` (C #7); `choose_mode`,
  `chosen_options(&EffectContext) -> Vec<String>`, `discard_random`, `exile_bottom_of_library`, `draw`
  (C #8).
- `effects::cast` (part 6.2): C #7 builds `CastNewArgs { def: CastNewDef::Read(Arc::new(|ctx:
  &EffectContext<'_>| -> Option<CastDef> { … })), radiant: None, how: CastHow::default() }`. Assumed:
  the `Read` closure takes `&EffectContext` (not `&mut`) and answers `Option<CastDef>`; `CastDef`
  derives `Deserialize` from `{ defId, radiant }` (built with `json_as`); `CastHow: Default`.
- `params` (part 2.1): `param(&impl ParamContext, &str) -> i32` with `ParamContext` implemented for
  `EffectContext`, `HookArgs`, `ConditionContext`; C #1's `curse_damage` names the trait
  `ParamContext` (through the prelude) for TS's `EffectContext | ConditionContext`.
  `step_param(&mut CardInstance, &str, i32)` in the tests.
- `query` (part 2.2): `zone_count(&GameState, PlayerId, OffFieldZone) -> i32`,
  `zone_cards(&GameState, PlayerId, OffFieldZone)` (a `Vec` of `CardInstance` or `&CardInstance`; read
  through `.iter()` and `CardInstance::clone(card)`, so either compiles), `recalled(&EffectContext,
  &str) -> Option<Value>` (C #4).
- `catalog` (part 2.2): `def_of(Option<&GameState>, &str) -> &CardDef`; `fused_id_parts(Option<&GameState>,
  &str)` (serialisable; compared as JSON) in C #7's test.
- `numbers::own_cost(&GameState, &CardInstance) -> Option<i32>` (C #4).
- `mana::effective_cost(&GameState, &CardInstance, CostOptions: Default) -> i32` (C #7's check, tests of
  C #2, #6).
- `work::part_memory_key(&IndexMap<String, Value>, &str) -> String` (C #7's `held_spell`, called by its
  full path `jackioh_engine::work::part_memory_key`; `work` is not in the prelude).
- `zones::OffFieldZone::{Hand, Library, Exile}` (through the prelude).
- testkit (part 5.1): `scenario(Value) -> Scenario`; `play/attack/answer/activate/end_turn` returning
  `&mut Scenario`; `state()`, `state_mut()`, `events()`, `last_events()`, `view(PlayerId)`,
  `unit/backrow(PlayerId, i32) -> Option<CardInstance>`, `hand(PlayerId)`/`pile(PlayerId, &str) ->
  Vec<CardInstance>`, `card(impl Into<CardRef>) -> &CardInstance` (passed `&str`, `&String`,
  `&CardInstance`), `stats(..).position: Position`; `expect_in_zone`, `expect_stats`, `expect_events`
  (a JSON array), `expect_health`, `expect_mana`, `expect_refused(|s| …)`. Through `testkit::*`:
  `legal_actions`, `reduce(&GameState, &Action) -> ReduceResult`, `hash_state`, `find_instance_mut`,
  `step_param`, `effective_cost`, `fused_id_parts`.
- `jackioh_cards::{register_all, card_def}` (part 1's lib.rs) as `crate::…` in every test.

Not ported: nothing.

## Decisions
- `export const radiant: Script = base` is `base.clone()`; TS's `expect(radiant).toBe(base)` is ported
  as `Arc::ptr_eq` on the shared hook(s) (the cry, the trigger's `run`, the `chosen` step), which a
  clone of the same `Script` shares, plus equal declarations where the TS compared them.
- TS module constants holding hooks (`const preview`, `const drawLimit`, `const aura`, `const zap`,
  `const stealBook`, `const pickle`) are private functions returning the hook/trigger/script, built once
  in `script()` and cloned where both faces share them.
- C #4 `answered` hands back the announced card's instance id (its one use), not the event; C #5
  `arrival` already did.
- C #4: `typeof before === "number"` is `Value::as_f64` (an integer JSON number reads too), truncated to
  `i32`. C #8 `numberIn` is `as_f64` plus a whole-number check.
- C #7 `heldSpell(Pick<EffectContext, "self" | "data">)` is `held_spell(Option<&CardInstance>, &data)`:
  the Activate's cast reads `ctx.live_self()` (TS's live `ctx.self`), `canActivate` its
  `ConditionContext.self_` with an empty data bag, as TS wrote. `withinLimit`'s
  `param({ state, self, radiant }, …)` is a `HookArgs` built from the `TargetCheckArgs`.
- C #8 keeps TS's `Numbers` / `ChainData` / `Pending` as private structs; `{ ...chain }` is
  `chain_data`, a `json!` object (key order is not TS's: `canonical` sorts keys, and `IndexMap`
  equality ignores order).
- Tests read events by matching `GameEvent` variants (frozen in `wire/events.rs`) and compare views,
  defs, targets, legal actions and memory as JSON (`serde_json::to_value`), so they pin the wire shape
  TS compared. `toMatchObject` is a private `matches_object` per file; TS `{ ...defaults, ...p1 }`
  side setups are a private `merged`. `stepParam(s.card(x), …)` steps the live card through
  `find_instance_mut(s.state_mut(), id)` (a private `step`).
- Every `#[test]` starts with `crate::register_all()` (part 5.1: the harness cannot call it).
- Test names: the title lower-cased, `§` → `s`, `#` → `n`, other runs of non-alphanumerics → `_`;
  R-ids stay as `r<n>` tokens.
