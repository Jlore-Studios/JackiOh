# Slice: part 9 (cards lane 1: Core #1–#48), chunk 4 of 4 (#397)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All seven were absent on `staging` (no night-bot half-port, no `part-09-bot.md` notes); all are full
ports of the TS script and its test file, one `#[test]` per TS `it` (counts checked against the TS:
14, 9, 14, 13, 10, 8, 8), one `mod` per TS `describe`. No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/core/c042_eugenics.rs` ← `042-eugenics.ts` + test
- `crates/cards/src/scripts/core/c043_big_felinor.rs` ← `043-big-felinor.ts` + test
- `crates/cards/src/scripts/core/c044_true_strike.rs` ← `044-true-strike.ts` + test
- `crates/cards/src/scripts/core/c045_deft_duelist.rs` ← `045-deft-duelist.ts` + test
- `crates/cards/src/scripts/core/c046_suppressive_aura.rs` ← `046-suppressive-aura.ts` + test
- `crates/cards/src/scripts/core/c047_fig_of_life.rs` ← `047-fig-of-life.ts` + test
- `crates/cards/src/scripts/core/c048_5pek_controller.rs` ← `048-5pek-controller.ts` + test

## SURFACE
Matched SURFACE §7 (one file per card: `//!` header, `use jackioh_engine::prelude::*;`, `pub const ID`,
`pub fn script() -> CardScripts`, tests at the bottom) and §8's testkit verbs as part 5.1's notes fix
them (`scenario(Value) -> Scenario`, steps `&mut self -> &mut Scenario`, `unit`/`backrow`/`pile`
return owned copies, `card(ref) -> &CardInstance`, card refs `&str`/`&String`/`&CardInstance`, seats
`PlayerId`, `expect_refused_with(|s| s.attack(…), "text")`, `state_mut()`).

## DEPENDS-ON
- Effects (parts 6/7), each built from TS's literal with `json_as`, so only the parameter type must
  `Deserialize` from it: `effects::{exile_random_from_library({count}), radiant_chance({zone, chance,
  lucky?}), destroy_all(BoardScope {side, notTags}), damage({to, amount, ignoreArmor}), exile({target}),
  heal({target, amount}), switch_all_positions({side: "both" | "enemy"}),
  chosen_options(&EffectContext) -> Vec<String>}`. `kind` strings read by #42's test:
  `"exileRandomFromLibrary"`, `"radiantChance"`.
- Testkit (part 5.1): as SURFACE above. `jackioh_cards::register_all()` (part 1) before each scenario.
- `damage::{deal_damage(&mut EngineSink, DamageArgs) -> i32, DamageArgs { source: Option<CardInstance>,
  target, amount: i32, flags }, DamageTarget::Unit { instance: CardInstance }}` (part 3.1's notes);
  `flags` written `Default::default()` so `Option<DamageFlags>` or a `Default` struct both pass.
- `layers::unit_view(&GameState, &CardInstance) -> UnitView { keywords, .. }` (part 2).
- `zones::{move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, MoveToZoneOptions: Default)
  -> MoveResult, OffFieldZone::Graveyard}` (part 2.1's notes).
- Frozen (part 1): `Script`, `CardScripts`, `hook`, `aura_hook`, `HookArgs`, `AuraEntry`, `StatMod`,
  `TargetDecl::target`, `ModeDecl`, `PromptKind::Mode`, `EffectContext::new`, `EngineSink::new`, `Rng::new`,
  `CardDef`/`CardFace` fields, `CardInstance` fields, `GameEvent::Destroyed {..}`, `event_type()`,
  `Position`, `Exertion`, `crate::card_def`.

## GAPS
- None of my functions were left out.
- Names I call that other parts provide (above): the eight effect verbs; `damage::deal_damage`/
  `DamageArgs`/`DamageTarget`; `layers::unit_view`; `zones::move_to_zone`/`OffFieldZone`; the testkit's
  `Scenario` methods `state_mut`, `expect_refused_with`, `last_events`, `view`, `backrow`.
- If `radiant_chance`'s argument spells `zone` as `zones`, or takes it as a list only, #42's literal
  `{ "zone": "library" }` stops deserialising (TS's `RadiantZone | RadiantZone[]`: a single string
  must be accepted, e.g. `OneOrMany<RadiantZone>`).

## Decisions
- A Radiant hand card (TS: `s.card("core-0NN").radiant = true` after the build) is set up with the
  harness's own pile entry `{ "def": …, "radiant": true }` instead: `placePile` sets the same flag on the
  same freshly created hand card, and it needs no mutable accessor. Same assertions.
- Each test module shadows `scenario` with a local wrapper that calls `crate::register_all()` first (TS's
  harness registered on import; part 5.1's notes ask card tests to register).
- TS's `def` (`cardDef(id)`) in tests is a local `def()` over `crate::card_def(ID)`.
- Object-shape assertions on wire types (keywords, `targets`, `modes`, `cost`, `type`, `tags`,
  `ownLibrary`) compare `serde_json::to_value(…)` with TS's literal, so they pin the JSON, not the Rust
  enum variant names.
- TS `toBeUndefined()` on an optional array (`targets`, `modes`) is `is_empty()`; on a hook
  (`cry`) `is_none()`; `typeof aura === "function"` is `aura.is_some()`. #45's
  `staticFlags?.deftDuelist` (not ported, SURFACE §7.2) is "no `deftDuelist` key in `flags()`'s JSON".
- #42's `effectKinds` calls the hook on a context built at rest (`EffectContext::new`) over a
  scenario's state, since a Rust hook needs a real `&mut EffectContext` (TS passed `undefined`).
- #44's `dealDamage({ state: s.state, events: [] }, …)` runs on a clone of the state (nothing reads it
  afterwards) with a fresh `EngineSink`.
- #46's "restored on leaving" moves a copy of the backrow card through `move_to_zone(g.state_mut(), …)`;
  the mover refreshes it from the state by id (part 2.1).
- #46's penalties are a private `Penalties { paid2, paid4 }` struct const (TS's object constants); the
  two auras are plain `fn`s handed to `aura_hook`, and `applies` closures annotate `&CardInstance`.
- Test names: the TS title snake_cased, R-ids moved to the front (`r49_r6_…`, `r81_r65_…`, `r81_radiant_…`),
  `§x.y` written `sx_y`, a long trailing parenthetical citation dropped, a leading digit spelled out
  (`nine_through_armor_7_…`); the exact TS title is the doc comment above each test and each `mod`.
- Helper arguments TS defaulted (`seed = "eugenics"`, `indestructibleEnemy = false`) are
  explicit parameters.
