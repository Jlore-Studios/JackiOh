# Slice: part 11 (cards lane 3), chunk 2 of 4: Core #90, #91, #92, #93.1, #93, #94, #95.1
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All seven were absent on `staging` (no night-bot half-port, no placeholder); all are full ports, each
the whole TS script (every function in TS order, its doc comments and every comment that states a
rule or cites a ruling) plus, where TS had one, its whole test file as `#[cfg(test)] mod tests`
(one `#[test]` per TS `it`, one nested `mod` per `describe`, the header comment above the module).
No `todo!`, `unimplemented!` or `// TODO`.

| File | Script from | Tests from (TS `it` / Rust `#[test]`) |
|---|---|---|
| `crates/cards/src/scripts/core/c090_cn_viral_injection.rs` | `090-cn-viral-injection.ts` | `090-cn-viral-injection.test.ts` (30 / 30; it covers #90.1 too) |
| `crates/cards/src/scripts/core/c091_fed_fauci.rs` | `091-fed-fauci.ts` | `091-fed-fauci.test.ts` (12 / 12) |
| `crates/cards/src/scripts/core/c092_felinor_fiender.rs` | `092-felinor-fiender.ts` | `092-felinor-fiender.test.ts` (19 / 19) |
| `crates/cards/src/scripts/core/c093_1_combo_fodder.rs` | `093-1-combo-fodder.ts` | none in TS (its tests are #93's file) |
| `crates/cards/src/scripts/core/c093_combo_index.rs` | `093-combo-index.ts` | `093-combo-index.test.ts` (49 / 49; it covers #93.1 too) |
| `crates/cards/src/scripts/core/c094_genns_greed.rs` | `094-genns-greed.ts` | `094-genns-greed.test.ts` (21 / 21) |
| `crates/cards/src/scripts/core/c095_1_chaos_golem.rs` | `095-1-chaos-golem.ts` | none in TS |

Files left: none.

## SURFACE
Every file: `pub const ID: &str` and `pub fn script() -> CardScripts` (SURFACE §7.1); nothing else is
public. `use jackioh_engine::prelude::*;` only. Hooks take `&mut EffectContext` (part 1 wins over
SURFACE §6.6); triggers are `TriggerDef::new(id, &[types], run).with_when(when)` with `(ctx, event)`
(part 1's `TriggerDef`); typed hooks through part 1's builders (`read_hook` for `set_stat`,
`condition_hook` for `condition_met`/`preview`).

## DEPENDS-ON
Names called in other parts' modules (TS name snake_cased at its TS module's path), with the shape
assumed. Each matches the owner's notes where the owner wrote one.

- Part 2.1 `zones`: `active_units_of`, `dormant_units_of(&GameState, PlayerId) -> Vec<&CardInstance>`
  (`.into_iter().cloned()` is used, so they must yield references, as part 2.1's notes say);
  `OffFieldZone::Library`.
- Part 2.2 `catalog::def_of(Option<&GameState>, &str) -> &CardDef` (`.tags`); `layers::stats_with_buffs(
  &GameState, &CardInstance) -> BuffedStats { attack, max_health }`; `query::zone_cards(&GameState,
  PlayerId, OffFieldZone)` (iterated with `.iter()`, so owned or referenced items both work).
- Part 4.1 `mana::{is_x_cost(&GameState, &CardInstance) -> bool, effective_cost(&GameState,
  &CardInstance, CostOptions) -> i32}` (third argument passed as `Default::default()`).
- Part 6.2 `effects::each::{for_each_card(ForEachCardArgs) -> Effect, ForEachCardArgs { cards, each }}`
  with `cards: Arc<dyn Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync>` and `each: Arc<dyn
  Fn(&str) -> Effect + Send + Sync>` (#94 writes the closures with those parameter types annotated).
- Effects built with `json_as(json!({ …TS literal… }))`, so their argument types must deserialise
  from TS's camelCase literals: `shuffle_into` (`defId`, `count`, `player: "enemy"`, `radiant`),
  `plague` (`amount`), `gain_mana` (`amount`), `add_to_hand` (`defId`), `damage` (`to: { of:
  "chosen" }`, `amount`, `lifesteal`), `draw_from_library` (`instanceId`), `exile_matching` (`parity:
  "odd"`, `exemptXCost`).
- Part 8.3 `subsystems::combo_index::{start_grade(StartGradeArgs), StartGradeArgs: Default,
  combo_index_end_of_turn(<&EngineSink or &mut EngineSink>, &CardInstance) -> Vec<Effect>` (passed
  `ctx`, which coerces either way), `grade_rises(&GameState, &CardInstance) -> bool`, `grade_of(
  &CardInstance) -> i32`, `grade_name(i32) -> Grade` (`Display`), `is_terminal_grade(i32) -> bool}`.
  Called by the module path (`subsystems::combo_index::…`), not the barrel glob.
- Part 5.1 testkit (`Scenario`), as its notes give it: steps and assertions `&mut self -> &mut
  Scenario`; `unit`/`backrow(seat, lane: i32) -> Option<CardInstance>`, `hand(seat)`/`pile(seat,
  &str) -> Vec<CardInstance>`, `card(ref) -> &CardInstance`, `stats(ref) -> layers::UnitView`
  (`.keywords`, `.position`), `view(seat) -> PlayerView`, `events()`/`last_events() -> &[GameEvent]`,
  `state()`, `state_mut()`; `expect_refused*(|s| s.step(…), …)` closures returning `&mut Scenario`;
  a seat as `PlayerId` or `Some(PlayerId)`; a card ref as `&str` or `&CardInstance`.
- Part 9 `crate::query::query(&CatalogQueryArgs)` (TS `query(args: CardQuery = {})`, `CardQuery =
  CatalogQueryArgs`), answering a list whose items have `.id` (owned `CardDef` or `&CardDef`). Only the
  tests call it (`query_ids`). If it takes the args by value, drop the `&` in the two `query_ids`.
- Part 1 (frozen, present): `GameEvent::event_type() -> GameEventType` with `Display`, `Keyword::kind()`
  (`KeywordKind`, `Display`), `TargetDecl::target`, `find_instance_mut`, `PreviewValue`, `SetStat`,
  `PublicBackrowView`, `BackrowView::Public`, `HandView::Cards`, `crate::card_def` (owned `CardDef`).

## GAPS
- None in my files: every TS function and every `it` is ported.
- The signatures under DEPENDS-ON that no owner's notes pin exactly: `ForEachCardArgs`' two closure
  types (part 6.2 names the aliases `ForEachCardCards`/`ForEachCardEach` but not their parameters) and
  `crate::query::query`'s argument (part 9 left no notes). Part 31: if either differs, the fix is in
  `c094_genns_greed.rs::greed` (two closure annotations) and the two `query_ids` helpers
  (`c090_cn_viral_injection.rs`, `c093_combo_index.rs`).

## Decisions
- One test file in TS for a card and its token (#90 + #90.1, #93 + #93.1) stays one `mod tests`, in
  the parent card's file (the brief's table maps the test file there); the token files carry no tests.
- TS `cardDef("core-090-1")` / `cardDef("core-093-1").id` used as a value is a private `const &str`;
  `build.rs` proves every file's `ID` against the catalog, which is what `cardDef`'s throw was for.
- TS `{ base: …, radiant: … } as const` (and `MANA_PER_TOKEN[face]`, `FORMULA[face]`) is a private
  `PerFace` struct per file, with a private `Face` enum where TS indexed by `"base" | "radiant"` (#91).
  A TS `const targets: TargetDecl[]` is a private `fn targets()` (a `TargetDecl` is not `const`).
- `ctx.self` is `ctx.self_` (the instance as the context was built). Every read here is of the card the
  context was built for, at that moment (its id, its Plague Counters at start of turn), so it equals
  TS's live object.
- #94's `drawn_cards` answers instance ids (`Vec<String>`), which is what part 6.2's `forEachCard`
  keeps; TS answered the instances and `forEachCard` read their ids.
- #93's radiant face is `Script { start_of_turn, ..base.clone() }` ("same" plus one clause), as TS
  listed the base's four hooks again with `startOfTurn` added.
- Tests: `expect(x).toEqual(objectLiteral)` compares the value's JSON (`serde_json::to_value`) with
  the literal; `toMatchObject`/`expect.objectContaining` is a private `matches_object`; `expect.any(
  Number)` is dropped to the other fields' checks; `toEqual([])` is `is_empty()` (an empty `vec![]`
  cannot infer its type in `assert_eq!`). `let mut s` wherever an `expect_*` is called (they take
  `&mut self`). TS `token ?? "core-t-rush"` as an attacker is a `match` with both arms.
  Seeds are TS's, unchanged; `format!("{radiant}")` prints `false`/`true` as TS's `String(radiant)`.
- Test helpers added beside TS's (private, in `mod tests`): `js`, `matches_object`, `type_of`,
  `library_of`, `viruses_in` (#90); `kinds` (#91); `pile_in_lane_1` (#92); `instance_id_of`, `cards`
  (#93); `has` (#94); `query_ids` (#90, #93).
