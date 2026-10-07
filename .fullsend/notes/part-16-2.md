# Slice: part 16 (cards lane 8: Classic+ #50–#78 and the AI tokens), chunk 2 of 3 (#404, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
None of the fourteen existed on `staging` (no bot half-port, no `.fullsend/notes/part-16-bot.md`); all are
full ports, script and tests, every TS function and every `it` (the `#[test]` count of each file equals
its TS `it` count). No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/classic_plus/c065_2_normal_grape.rs` (17 tests)
- `crates/cards/src/scripts/classic_plus/c065_3_large_grape.rs` (10)
- `crates/cards/src/scripts/classic_plus/c065_4_golden_grape.rs` (17)
- `crates/cards/src/scripts/classic_plus/c065_5_mythic_grape.rs` (9)
- `crates/cards/src/scripts/classic_plus/c065_two_grapes.rs` (15)
- `crates/cards/src/scripts/classic_plus/c066_vine_of_grapes.rs` (8)
- `crates/cards/src/scripts/classic_plus/c067_pear.rs` (11)
- `crates/cards/src/scripts/classic_plus/c068_organic_produce.rs` (13)
- `crates/cards/src/scripts/classic_plus/c069_buff_billy.rs` (10)
- `crates/cards/src/scripts/classic_plus/c070_chaos_machine.rs` (14)
- `crates/cards/src/scripts/classic_plus/c071_book_of_buff.rs` (10)
- `crates/cards/src/scripts/classic_plus/c072_book_of_nerf.rs` (11)
- `crates/cards/src/scripts/classic_plus/c073_1_classic_golem.rs` (15)
- `crates/cards/src/scripts/classic_plus/c073_call_to_chaos_classic_edition.rs` (27)

## SURFACE
§7.1 shape in every file: the TS header as `//!`, `use jackioh_engine::prelude::*;` plus the effect verbs
named from `jackioh_engine::effects`, `pub const ID`, `pub fn script() -> CardScripts`; the TS test header
as `//` lines above `#[cfg(test)] mod tests { use super::*; use jackioh_engine::testkit::*; … }`.
Testkit calls follow part 5.1's notes (the testkit's author), as part 10.4 did.

## GAPS
None left unported. Names called in other parts' modules, with the shape assumed (from the owners' notes
where they say; the TS name snake_cased at its TS module otherwise):
- Effects (parts 6/7), every argument built with `json_as(json!({ …TS literal… }))`, so only the serde
  shape matters: `damage_enemy_or_heal_friend({ amount })`, `draw_priced({ costMod | costOverride })`,
  `replace_hand_with_random({ query, costOverride, radiant? })`, `add_rolled_grapes({ count, radiant? })`,
  `set_radiant({ target } | { instanceId })`, `summon_random({ query, radiant? })`,
  `add_random_from_catalog({ query, count, costOverride? })`, `upgrade`/`degrade(TuneArgs { target, times }
  | { scope, random })`, `transform_random({ target, query, radiant?, readyToAttack })` (its `radiant` must
  take a JSON `true`), `summon_copy({ of, lane })`, `recruit({ filter })`, `cast_new({ def, random })`
  (a string `def` must deserialise).
- `effects::targets` (part 7.1's notes): `TargetSpec` (Deserialize from `{ "of": "chosen" }`),
  `BoardScope: Default`, `instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>`,
  `adjacent_to(&EffectContext, &TargetSpec, &BoardScope) -> Vec<CardInstance>`.
- `params::param(&impl ParamContext, &str) -> i32` called as `param(ctx, "…")` with a
  `&mut EffectContext` (part 2.1: it coerces); `params::step_param(&mut CardInstance, &str, i32)`.
- `query::zone_cards(&GameState, PlayerId, OffFieldZone) -> Vec<CardInstance>` (C+ #65.4, passed
  `ctx.state` through the context's `Deref`; `&*ctx.state` if the compiler wants it spelled out).
- `combat::after_attack_of(&EffectContext) -> Option<AfterAttackFacts { target_id, destroyed_ids:
  Vec<String>, survived, forced }>` (C+ #73.1).
- `subsystems::{call_to_chaos(CallToChaosArgs { radiant: Option<bool>, table: Option<&'static
  [ChaosEffectDef]> }), CHAOS_PLUS_EFFECTS: &'static [ChaosEffectDef { name, label: &'static str, .. }],
  roll_chaos_effects(&mut Rng, bool, &[ChaosEffectDef]) -> Vec<_ with .name>, CHAOS_CHAIN_KEY: &str,
  fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>}` (`FuseArgs` Deserialize, `handPrice: "fused"`).
  If `CallToChaosArgs` grows a field, add `..Default::default()` in C+ #73's two hooks.
- Tests only: `catalog::{def_of(Option<&GameState>, &str), query(&CatalogQueryArgs), roll_grape(&mut Rng,
  i32) -> String}` (named by path: `jackioh_engine::catalog::…`), `numbers::numbers_on` (read as JSON),
  `mana::{cost_now(&GameState, &CardInstance) -> i32, effective_cost(&GameState, &CardInstance,
  Default::default()) -> i32}`, `resolve::{make_context(&mut EngineSink, Option<&CardInstance>,
  HookOptions { controller: Option<PlayerId>, .. }: Default) -> EffectContext, apply_effects(&[Effect],
  &mut EffectContext)}`, `triggers::{settle(&mut EngineSink, SettleOptions), SettleOptions: Default}`,
  `state_check::state_check(&mut EngineSink)`, `reduce::reduce(&GameState, &Action) -> ReduceResult`,
  `view_for::HIDDEN_ID: &str`, `testkit::invariants::create_invariant_monitor(&GameState)` with
  `after(&mut self, &[GameEvent], &GameState)` and `before(&mut self, &GameState, PlayerId, &ActionBody)`,
  each `-> Vec<String>`; `crate::{card_def, CATALOG, register_all}` (part 1's cards `lib.rs`).
- Testkit (part 5.1): `scenario(Value)`, steps returning `&mut Scenario`, `unit`/`backrow` →
  `Option<CardInstance>` (owned), `hand`/`pile` → `Vec<CardInstance>`, `card(ref) -> &CardInstance`,
  `state_mut()`, a card ref from `&str`/`&String`/`&CardInstance`, `expect_refused`/`expect_refused_with`.

## Decisions
- The outer `describe("C+ #NN …")` is `mod tests` itself; every inner `describe` is a `mod` (C+ #73's
  "base, the ten entries" → `base_the_ten_entries`, "R436 what was rolled is public" → `r436_…`).
  Names: the title lower-cased, `§` → `s`, other non-alphanumeric runs → `_`, ruling tokens leading;
  a title starting with a digit gets `t_` (C+ #73.1's "20/20 kills …" → `t_20_20_…`).
- TS module constants that are not `const`-able (`targets`, `CLASSIC_UNITS`, `staticFlags`) are private
  fns; TS factories (`twoGrapes(radiant)`, `pear(radiant)`, `afterItAttacks(radiant)`, …) are private fns
  returning `Script`/`Hook`. A TS spread `...(radiant ? { radiant: true } : {})` is a key set on the
  `json!` value when true. Shared TS hooks (`tick`, Book of Nerf's `cry`) are one `Hook` cloned, so
  `toBe` identity ports as `Arc::ptr_eq`; `radiant = base` is `base.clone()`.
- Tests read engine values as JSON (`js` = `serde_json::to_value`) wherever TS compared against a literal
  or used `toMatchObject` (defs, events, legal actions, tunings), so they pin the wire shape, not a Rust
  struct. `stepParam(s.card(x), …)` and `s.card(x).field = …` go through `state_mut()` +
  `find_instance_mut`. TS's `indexOf`/`lastIndexOf` keep −1 (and `indexOf(x, from)`'s negative `from`).
- Engine internals in tests are named by module path (`jackioh_engine::resolve::make_context`, …) so the
  two globs (`super::*` = the prelude, `testkit::*` = the whole engine) never meet on a name.
- Every test that builds a scenario starts with `crate::register_all()`.
