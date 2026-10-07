# Slice: part 9 (cards lane 1: Core #1–#48), chunk 1 of 4 (#397): query.rs and Core #1–#18
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All complete, each the whole TS script (every function in TS order, its header and doc comments,
every comment that states a rule or cites a ruling) with the whole TS test file as its
`#[cfg(test)] mod tests` (the test file's header above it, one `mod` per `describe`, one `#[test]`
per `it`, an `it` generated in a loop is one `#[test]` per generated title). No `todo!`,
`unimplemented!` or `// TODO`.

- `crates/cards/src/query.rs` ← `packages/cards/src/query.ts`.
- `crates/cards/src/scripts/core/c001_big_d_fender.rs` … `c018_bread_and_butter.rs` ← Core #1–#18.

The night bot's half-done ports of `query.rs`, #1–#4, #10, #11 and #17 (commit 0180138) were read
against both TS files and rewritten in place: they used names part 1 did not freeze (`AuraArgs`, an
`Arc` for `AuraEntry.applies`, `usize` lanes, `expect_refused` closures returning `()`), never called
`register_all()`, and `query.rs` returned owned `CardDef`s where part 2.2's `catalog::query` hands out
`&'static CardDef`. There were no bot notes for part 9 (`.fullsend/notes/part-09-bot.md` absent).

## SURFACE
- Every card file: `pub const ID`, `pub fn script() -> CardScripts`, private helpers. Nothing else public.
- `crate::query` (re-exported at the crate root by part 1's `lib.rs`): `type CardQuery =
  CatalogQueryArgs`, `query(&CardQuery) -> Vec<&'static CardDef>`, `pool(&str, &CardQuery) ->
  Vec<&'static CardDef>`, `TRAP_TYPES: &[CardType]`, `pub use catalog::query_cost`, and the TS
  `catalog` object as `pub const catalog: CatalogSurface { trap_types }` with methods
  `catalog.query(&args)`, `catalog.pool(own_id, &args)`, `catalog.cost(&def)` (for 067 and 083).

## DEPENDS-ON
- Part 1 (frozen, matched): `Script`, `CardScripts`, `hook`, `aura_hook`, `HookArgs`, `AuraEntry { applies:
  Box<…>, mod_ }`, `StatMod`, `condition_hook`, `ConditionContext`, `ConditionZone`, `ConditionHook`,
  `PreviewHook`, `PreviewValue`, `TriggerDef::new(id, &[GameEventType], |ctx, event| …).with_when(…)`,
  `TargetDecl::target(min, max, json!)`, `ModeDecl`, `PromptKind`, `GameEvent`/`GameEventType`,
  `Exertion`, `CardInstance`, `Zone`, `Row`, `Position`, `Keyword`, `CardCost`, `Tag`, `PendingView`,
  `PendingPromptView`, `HandView`, `CardView`, `opponent_of`, `json_as`; `crate::card_def(id) -> CardDef`
  and `crate::register_all()` (part 1's `crates/cards/src/lib.rs`).

## GAPS
### Called in other parts' modules (TS name snake_cased at its TS module's path; the shape assumed)
- `jackioh_engine::catalog::{query(&CatalogQueryArgs) -> Vec<&'static CardDef>, query_cost(&CardDef) -> i32}`
  and `CatalogQueryArgs: Serialize + Deserialize` with camelCase keys (part 2.2's notes say so).
  `pool` extends `excludeDefId` on the arguments' JSON, so it works whatever Rust type holds that field.
- Effects (part 6/7), each `fn(<args>) -> Effect` whose one argument deserialises from the TS object
  literal (every call is `json_as(json!({ … }))`): `destroy`, `destroy_all`, `destroy_adjacent_to`,
  `summon` (`defId`, `player`, `statsOverride`, `armorOverride`), `summon_copy` (`of`, `player`),
  `flip_coins` (`target`, `coins`, `perHeads`, `perTails`), `flip_coin_keyword` (`target`,
  `headsKeyword`, `tailsKeyword`), `draw` (`count`), `heal` (`target`, `amount`), `gain_mana`
  (`amount`; the prelude resolves the name to the effect), `discover_from_catalog` (`step`, `query`,
  `count`), `add_to_hand` (`defId`, `costMod`), `forced_attacks_on` (`target`, `attackers`),
  `damage_all` (`side`, `amount`, `heroes`), `bounce_all` (`side`).
- `effects::choose::chosen_options(ctx) -> Vec<String>` — called with a `&mut EffectContext`, so it
  compiles whether it takes `&EffectContext` or `&mut EffectContext`.
- `query::played_earlier(&GameState, PlayerId, impl Into<CardOrId>)` with an `Option<&CardInstance>`
  (`ctx.live_self()`, TS's live `ctx.self`) and a `&CardInstance` (part 2.2's notes);
  `query::unspent_mana_of(&GameState, PlayerId) -> i32`.
- Tests: `layers::keywords_of(&GameState, &CardInstance) -> Vec<Keyword>` (c003);
  `state::find_instance_mut` (part 1); the testkit as part 5.1's notes give it (`scenario(Value)`,
  `unit(seat, i32)`/`backrow(seat, i32) -> Option<CardInstance>`, `hand(seat)`/`pile(seat, &str) ->
  Vec<CardInstance>`, `card(ref) -> &CardInstance`, `stats(ref)` → `layers::UnitView { keywords, .. }`,
  `state()`, `state_mut()`, `view(seat) -> PlayerView`, `events()`/`last_events() -> &[GameEvent]`,
  `expect_refused(|s| s.step(..))` with the closure returning `&mut Scenario`); the glow helpers of
  part 5.3 as `backrow_glows(&Scenario, lane, PlayerId) -> bool`, `opponent_sees_glow(&Scenario,
  lane, PlayerId) -> bool`, `hand_glows(&Scenario, &str, PlayerId) -> bool` (lane an integer literal).

### Not ported
Nothing. TS `def` exports are not ported (SURFACE §7.1); TS `cardDef("core-t-rush").id` used as a
value is a private `const RUSH_TOKEN: &str` (its load-time existence check becomes the card's tests).

## Decisions
- Test modules: `use jackioh_engine::testkit::*;` only (no `use super::*` at the top of `mod tests`,
  so the prelude's effect verbs never meet the testkit's engine-root globs); a test that needs the
  card's own items names them (`use super::{ID, script};`, c011). Nested `mod`s per `describe` take
  `use super::*;`. Every `#[test]` starts with `crate::register_all();` (part 5.1: the testkit cannot
  name the cards crate).
- Names: the title snake_cased, `§` → `s`, `#` → `n`, `×`/punctuation dropped, a leading digit → `t_`,
  ruling tokens kept leading (`r64_r83_…`); a `describe` "#7 Jewelosco Scarab (§8.1 row 7)" →
  `mod n7_jewelosco_scarab_s8_1_row_7`; nested "base"/"radiant" → `mod base`/`mod radiant`.
- TS's "make the hand copy radiant" harness workaround writes through
  `find_instance_mut(s.state_mut(), &id)`; seeded buffs/damage (c012) likewise.
- `toEqual({ attacked: false, switched: false })` on an exertion is `== Exertion::default()` (TS's
  `toEqual` ignores the absent `attacks`); a backrow view compared to a literal is compared as JSON.
- TS's `expect(base).toEqual({})` (c011) is an exhaustive destructuring of `Script` checking every
  field empty, so a field added to `Script` breaks that test until it is listed.
- c018: `TrapTrigger` is part 1's one `TriggerDef` (it carries `when`); the face constants are a
  private `Face { multiplier, formula }` with `BASE`/`RADIANT`; `×` is written `\u{00d7}`.
- c010: the cry passes `ctx.live_self()` (TS read the live `ctx.self`); the glow passes `ctx.self_`.
- Helpers that take the context take `&mut EffectContext<'_>` (c007, c017), so the effect readers they
  call may take either `&` or `&mut`.
- `query.rs`'s `catalog` object: a struct with the one data field (`trap_types`) and the three
  functions as methods, not fn-pointer fields (SURFACE §16 `hook.style`: no fn pointers).
