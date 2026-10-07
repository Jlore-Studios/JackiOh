# Slice: part 14 (cards lane 6), chunk 2 of 4 (#402, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All fifteen were absent on `staging` (no night-bot half-port, no `part-14-bot.md`); all are full ports
of the TS script and its test file: every script function in TS order with its header and rule-citing
comments, one `#[test]` per TS `it` (counts checked against the TS: 17, 8, 7, 13, 10, 6, 7, 17, 13, 14,
10, 18, 5, 17, 24), one `mod` per TS `describe`, and the same R-ids as the TS pair (checked by grep).
No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/classic_plus/c012_8_frostspatula.rs` ← `012-8-frostspatula.ts` + test
- `crates/cards/src/scripts/classic_plus/c012_the_mother_pancake.rs` ← `012-the-mother-pancake.ts` + test
- `crates/cards/src/scripts/classic_plus/c013_mommy_barker.rs` ← `013-mommy-barker.ts` + test
- `crates/cards/src/scripts/classic_plus/c014_forever.rs` ← `014-forever.ts` + test
- `crates/cards/src/scripts/classic_plus/c015_conjure_rush_token.rs` ← `015-conjure-rush-token.ts` + test
- `crates/cards/src/scripts/classic_plus/c016_conjure_rush_token.rs` ← `016-conjure-rush-token.ts` + test
- `crates/cards/src/scripts/classic_plus/c017_conjure_rush_token.rs` ← `017-conjure-rush-token.ts` + test
- `crates/cards/src/scripts/classic_plus/c018_gullible_treatler.rs` ← `018-gullible-treatler.ts` + test
- `crates/cards/src/scripts/classic/c069_plague_charger.rs` ← `069-plague-charger.ts` + test
- `crates/cards/src/scripts/classic/c070_book_of_plague.rs` ← `070-book-of-plague.ts` + test
- `crates/cards/src/scripts/classic/c071_lane_eater.rs` ← `071-lane-eater.ts` + test
- `crates/cards/src/scripts/classic/c072_grand_counterspell.rs` ← `072-grand-counterspell.ts` + test
- `crates/cards/src/scripts/classic/c073_nurse_cleaver.rs` ← `073-nurse-cleaver.ts` + test
- `crates/cards/src/scripts/classic/c074_corpse_plantation.rs` ← `074-corpse-plantation.ts` + test
- `crates/cards/src/scripts/classic/c075_argusland.rs` ← `075-argusland.ts` + test

## SURFACE
SURFACE §7.1 shape in every file: `//!` header, `use jackioh_engine::prelude::*;`, the effect verbs
imported explicitly from `jackioh_engine::effects` (SURFACE §7.1's example; an explicit import also wins
over any same-named glob), `pub const ID`, `pub fn script() -> CardScripts`, tests at the bottom under
the TS test file's header comment. Part 1's frozen types used as compiled: `Script`, `CardScripts`,
`hook`, `read_hook`, `aura_hook`, `condition_hook`, `HookArgs`, `AuraEntry`, `StatMod`, `HeroGuard`,
`GraveyardPlayPermission`, `ActivationDecl`, `ActivationUses::Count`, `TriggerDef::new(..).with_when(..)`,
`ConditionZone`, `EffectContext` (`self_`, `controller`, `live_self()`, `events`/`state` through the
sink), `GameEvent`/`GameEventType` variants, `Keyword`, `KeywordKind`, `CardType`, `Tag`, `Row`,
`ZoneName`, `Enchantment::ReturnAfterResolve`, `Selection::Instance`, `ActionBody::Play`, `Action`,
`PLAYER_IDS`, `opponent_of`, `keyword_key`, `HAND_CAP`/`HERO_HEALTH: i32`, `RANDOM_KEYWORD_POOL: &[&str]`.
Testkit verbs as part 5.1's notes fix them (`scenario(Value)`, steps `&mut self -> &mut Scenario`,
`unit`/`backrow` → `Option<CardInstance>` copies, `hand`/`pile` → `Vec<CardInstance>`, `card(ref)` →
`&CardInstance`, `stats(ref)` → `layers::UnitView`, `view(seat)`, `state()`/`state_mut()`,
`events()`/`last_events()`, `expect_*`, `expect_refused(|s| s.…)`).

## DEPENDS-ON
Effects (parts 6/7), every argument built from TS's literal with `json_as`, so only the argument type
must `Deserialize` from it:
- `effects::memory::remember({ key, value })`, `effects::summon::summon({ defId, radiant })` and
  `summon({ defId, randomKeywords })`, `effects::add_to_hand::add_random_from_catalog({ query: { tags },
  count })`, `effects::cast::enchant_next_spell({ enchantment: { kind: "returnAfterResolve", floor } })`,
  `effects::destroy::{sacrifice({ target: { of: "self" } }), destroy({ target: { of: "instance",
  instanceId } })}`, `effects::plague::{place_plague_tokens({ count }), place_plague({ target: { of:
  "self" }, amount })}`, `effects::locks::lock_lane({})` and `lock_lane({ side: "enemy" })`,
  `effects::move_::counter_play({ to: "graveyard" | "thief", target })`.
Engine reads (prelude globs):
- `query::recalled(&EffectContext, &str) -> Option<Value>` (part 2.2's notes).
- `params::param(&impl ParamContext, &str) -> i32` for `EffectContext` (`param(&*ctx, …)`) and
  `HookArgs` (`param(&args, …)`) (part 2.1).
- `plague::plague_on(&CardInstance) -> i32` (TS `Pick<CardInstance, "counters">`).
- `zones::{ZoneSlot (struct literal; an alias of `ZoneRef` works), slots_of(PlayerId, Row) ->
  Vec<ZoneSlot>, card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>, slot_of(&GameState,
  &CardInstance) -> Option<ZoneSlot>, lock_zone(&mut GameState, &ZoneSlot), is_locked(&GameState,
  &ZoneSlot)}` — every slot is passed by reference, which `impl Into<ZoneSlot>` (part 2.1) accepts too.
- `faces::card_type_of(&GameState, &CardInstance) -> CardType`.
Tests (through `jackioh_engine::testkit::*`): `params::step_param(&mut CardInstance, &str, i32)` on the
live card via `state::find_instance_mut`; `state::find_instance`; `mana::effective_cost(&GameState,
&CardInstance, Default::default())` (part 4.1's three arguments); `reduce::{reduce(&GameState, &Action)
-> ReduceResult, legal_actions(&GameState, PlayerId) -> Vec<ActionBody>}`; `replay::hash_state`;
`query::{cards_played_this_turn(&GameState, PlayerId) -> i32 (compared with 0), last_spell_played(&GameState)
(only .is_none())}`; `jackioh_cards::{register_all, card_def}` (part 1).

## GAPS
- No function was left out.
- Names called that other parts provide: the effect verbs and engine reads above. The riskiest
  shapes: `recalled`'s return (`Option<Value>`; an `Option<&Value>` would need `.cloned()` in
  `c012_8_frostspatula.rs`'s `kills_of`), `plague_on` taking `&CardInstance` (not `&Counters`),
  `card_at` returning a reference, `effective_cost`'s third `Default` argument.
- `counter_play`'s `to` is written as the string literal ("graveyard" / "thief") into its JSON
  argument, so `effects::move_::CounterDestination` must deserialise from those literals.

## Decisions
- Each test module shadows `scenario` with a local wrapper that calls `crate::register_all()` first (TS's
  harness registered on import; part 5.1's notes ask card tests to register).
- `export const radiant: Script = base` is `radiant: base.clone()`; TS's `expect(radiant).toBe(base)`
  ports as `Arc::ptr_eq` on each hook the face carries (the clone shares the `Arc`s). C+ #14's two
  identical faces stay two scripts, as in TS.
- Frostspatula's kills ride memory as `{ id, defId, radiant }` through a private `Kill` struct (serde
  camelCase); an entry that does not parse as one is dropped (TS cast without checking; only this card
  writes the key). The trigger and Death read the context through a shared `&EffectContext` view.
- Lane Eater's zones are `(PlayerId, Row)` tuples (TS `{ player, row }`); it reads its slot off
  `ctx.live_self()` (TS's live `ctx.self`). Grand Counterspell's `answers(to)` takes the
  `CounterDestination` literal as `&'static str`.
- `RADIANT_TOKENS`/the token loop count is `usize`; game numbers stay `i32`.
- Assertions on wire shapes compare JSON (`serde_json::to_value`): views (`you.reserved.backrow[1]`,
  `units[0].conditionActive`, `pending`, `opponent.hand`), zones, `def.params`, `def.cost`, events.
  `toMatchObject` is a private `matches_object` (subset for objects, element-wise for arrays).
  `{ ...base, ...over }` is a private `merged` on JSON objects.
- TS's `s.answer(card.id)` is `s.answer(json!(card.id))`; `expect(() => …).toThrow()` is
  `s.expect_refused(|s| …)`; `stepParam(s.card(X), …)` is a private `step` through `state_mut()`.
- C #74's `send` numbers its nonce `plantation-<applied.len() + 1>` instead of TS's module counter (no
  mutable statics, SURFACE §3); unique along each test's chain of states. Its `tokenOptions` keeps JS's
  comparator-less sort (values compared as strings, `null` as "null"). `playFromGraveyard`'s
  `{ from, tokens }` is two arguments.
- Test names: TS title snake_cased, R-ids leading, `§x.y` → `sx_y`, `#` → `n`, a leading number spelled
  out (`twenty_6_…`); the exact title is each test's doc comment. C+ #18's `describe("R195 conditionMet
  …")` is `mod r195_condition_met_…`, its `it`s named by their own titles.
