# Slice: part 10 (cards lane 2), chunk 2 of 4 (#398): Core #55–#65
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run. Python ran only to
snake_case test titles and to make mechanical edits to the two bot files.

## FILES
All twelve hold the full port: `pub const ID`, `pub fn script()`, every TS function in TS order with
its doc and rule comments, and a `mod tests` with one `#[test]` per TS `it` (one `mod` per `describe`).
No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/core/c055_lava_golem.rs` (22 tests)
- `crates/cards/src/scripts/core/c056_jilliax.rs` — the night bot's port (commit 0180138), finished: 10 tests
- `crates/cards/src/scripts/core/c057_conjure_ky.rs` (12)
- `crates/cards/src/scripts/core/c058_rush_token_farm.rs` (10)
- `crates/cards/src/scripts/core/c059_unbiased_immigration.rs` (13)
- `crates/cards/src/scripts/core/c060_bear_honeypot.rs` (29: the TS `for (radiant of [false, true])`
  loop's two `it`s are one `#[test]` per face)
- `crates/cards/src/scripts/core/c061_prejudiced_postdoc.rs` (10)
- `crates/cards/src/scripts/core/c062_friend_of_felinors.rs` — the night bot's port, finished: 6 tests
- `crates/cards/src/scripts/core/c063_plastic_surgery.rs` (10)
- `crates/cards/src/scripts/core/c064_gifted_program.rs` (12)
- `crates/cards/src/scripts/core/c065_1_spikey_pillow.rs` (11)
- `crates/cards/src/scripts/core/c065_masochism_mask.rs` (11)

Files left: none.

## SURFACE
§4.1 paths, §4.2 names, §7.1 card-file shape, §8 testkit verbs. Where part 1's frozen code differs from
SURFACE, part 1's was used: hooks are `hook(|ctx| …)` over `&mut EffectContext`; triggers are
`TriggerDef::new(id, &[GameEventType::…], |ctx, event| …).with_when(|ctx, event| …)`; `conditionMet`
is `condition_hook(|c| …)` over the `Copy` `ConditionContext`; auras are `aura_hook(|args| vec![AuraEntry
{ applies: Box::new(…), mod_: StatMod { … } }])`; `resume` is `IndexMap::from([(STEP, hook(…))])`.
The testkit is used as part 5.1's notes describe it (below).

## DEPENDS-ON (names called, expected from other parts; shapes guessed)
- part 5 testkit (`jackioh_engine::testkit`): `scenario(Value) -> Scenario`; the steps `play(card, Value)`,
  `attack(card, card | "hero")`, `answer(Value)`, `end_turn()`, `start_turn()`, `switch_position(card)`,
  each `-> &mut Scenario`; reads `state()`, `state_mut()`, `events()`, `last_events()`, `view(PlayerId)`,
  `unit(PlayerId, i32) -> Option<CardInstance>` and `backrow(PlayerId, i32) -> Option<CardInstance>`
  (OWNED copies, as part 5.1's notes say: `.map(|unit| unit.def_id)` moves out of them), `hand(PlayerId)`,
  `pile(PlayerId, &str) -> Vec<CardInstance>`, `card(ref) -> &CardInstance`, `stats(ref) -> layers::UnitView`
  (`.keywords: Vec<Keyword>`); assertions `expect_in_zone`, `expect_stats(ref, Value)`, `expect_events(Value)`,
  `expect_health`, `expect_mana`, `expect_refused_with(|s| s.step(…), "text")` (closure returns the
  `&mut Scenario`). A card ref is a `&str`, `&String` or `&CardInstance`.
  `glow::{hand_glows(&Scenario, &str, PlayerId), backrow_glows(&Scenario, lane, PlayerId),
  opponent_sees_glow(&Scenario, lane, PlayerId)} -> bool` (part 5.3: TS's default viewer written out).
  `find_instance_mut` (state.rs, frozen), `RANDOM_KEYWORD_POOL` (config.rs, frozen), `Rng::new`,
  `EngineSink::new` (frozen).
- part 2: `catalog::def_of(Option<&GameState>, &str) -> &CardDef` (part 2.2's shape; `.type_` read);
  `zones::open_zones(&GameState, PlayerId, Row) -> Vec<ZoneSlot>` (`.is_empty()` only).
- part 3: `resolve::cast_card(&mut EngineSink, &CardInstance, CastOptions)` with `CastOptions: Default`
  (called with `Default::default()`; #63's R703 cast test).
- part 5: `reduce::legal_actions(&GameState, PlayerId) -> Vec<ActionBody>` (SURFACE §6.1).
- parts 6–7, the effects library (`jackioh_engine::effects::<verb>`), each argument built with
  `json_as(json!({ …the TS literal… }))`, so only the field names matter (TS's camelCase):
  `add_random_from_catalog({ query, count, radiant?, costOverride? })`, `summon({ defId })`,
  `fill_board({ defId })`, `forced_attacks({ attackers: { side, defId, summonedThisScript }, target:
  { instanceId } })`, `summon_copy({ of: { of: "chosen" }, vanilla, grantedKeywords })`,
  `buff({ target, attack, health })`, `grant_random_keywords({ target, count })`,
  `buff_all_units({ side, attack, health })`, `choose_mode({ options, step, prompt })`,
  `exile_bottom_of_library({ player: "self" })`, `lose_health({ player: "self", amount })`, and
  `chosen_options(&EffectContext) -> Vec<String>`.
- part 9: `crate::query::{pool(&str, &CardQuery), query(&CardQuery)}` returning a list of defs (`&CardDef`
  or `CardDef`; only `.id` is read), with `CardQuery` (= the engine's `CatalogQueryArgs`) built by
  `json_as` from TS's literal, so it must `Deserialize`.
- part 1 (cards `lib.rs`, frozen): `crate::register_all()`, `crate::card_def(&str) -> CardDef`,
  `crate::CATALOG`.

## GAPS
- None of my functions were left out.
- The calls above are the guesses part 31 checks. The likeliest misses: `cast_card`'s instance argument
  (`&CardInstance` here; other parts' notes guess `&mut` or owned), `query::pool`'s argument
  (`&CardQuery` here, not by value), and `chosen_options`' return (`Vec<String>`; `.into_iter().next()`
  is read).

## Decisions
- Every `#[test]` starts with `crate::register_all();` (part 5.1: without it the filler deck refuses).
- Test names: the TS title by §4.2 (`§` → `s`, `#` → `n`, other runs → `_`, leading ruling tokens kept).
  A `describe`'s `mod` is its title without the leading "#NN Card Name —" (the file already names the
  card): `describe("#55 Lava Golem — Tribute 3 (§6.3, R81, R90)")` → `mod tribute_3_s6_3_r81_r90`; a
  `describe` that is only the card's name keeps it (`mod prejudiced_postdoc`).
- TS object spreads in test setups (`{ ...BUSY, … }`, `[...FODDER, …]`) are small private `spread`/
  `concat` helpers over `Value`; TS constants that are object literals are private fns returning `Value`.
- A TS test that writes through `s.card(x)` (`.costOverride = 0`, `.radiant = true`) writes through
  `find_instance_mut(s.state_mut(), &id)`; `s.state.players.p1.locks…`/`s.state.reserved.push` write
  through `state_mut()`.
- `toMatchObject` is a private `matches_object` over the event's JSON (#55); `toEqual` against an object
  literal compares `serde_json::to_value` of the engine value (declarations, flags, buffs, exertion).
- #61's `toThrow(/Human|not a legal|option/i)` is a real regex: a private `refusal` helper catches the
  step's panic (`catch_unwind`) and the test checks the lower-cased message by hand.
- #60: TS `match` is `match_` (SURFACE §4.1's keyword rule); TS's `ResolvedPlay` (the narrowed
  `cardResolved` event) is a private struct of the three fields read after the match; `TrapTrigger` is
  `TriggerDef` (part 1 has no separate type).
- #57/#59: TS's `CatalogQueryArgs` constants are private fns returning the JSON literal, nested into the
  effect's `json_as` argument.
- #62 (bot): kept its `felinor_token()` (`crate::card_def(…).id`, read once as the script is built);
  #65's `SPIKEY_PILLOW` is a plain `const` (TS `cardDef("core-065-1").id`, part 11's convention).
- #61: TS's dev note that `summonCopy` "does not exist yet … this import is the report" is stale and states
  no rule, so it is not ported.
- The bot's #56 and #62 were finished in place: `expect_refused_with` closures now hand the scenario back,
  lanes are `i32`, unit reads are owned, every test registers the cards, #62's tests sit in their
  `describe`'s `mod`, and `view(Some(p))` is `view(p)`.
