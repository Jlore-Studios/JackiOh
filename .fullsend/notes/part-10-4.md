# Slice: part 10 (cards lane 2: Core #49–#81), chunk 4 of 4 (#398, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
None of the four existed on `staging` (no bot half-port, no `.fullsend/notes/part-10-bot.md`); all four
are full ports, script and tests, every TS function and every `it`. No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/core/c078_fullsend.rs` ← `078-fullsend.ts` + `078-fullsend.test.ts` (18 `it` → 18 `#[test]`, 4 `describe` → 4 `mod`)
- `crates/cards/src/scripts/core/c079_twinspell.rs` ← `079-twinspell.ts` + `079-twinspell.test.ts` (10 → 10, 2 `mod`)
- `crates/cards/src/scripts/core/c080_zao_gao.rs` ← `080-zao-gao.ts` + `080-zao-gao.test.ts` (18 → 18, 3 `mod`)
- `crates/cards/src/scripts/core/c081_radiant_saintess.rs` ← `081-radiant-saintess.ts` + `081-radiant-saintess.test.ts` (17 → 17, 2 `mod`)

## SURFACE
§7.1 shape in every file: `//!` TS header, `use jackioh_engine::prelude::*;` plus the effect verbs named
from `jackioh_engine::effects`, `pub const ID`, `pub fn script() -> CardScripts`; tests in
`#[cfg(test)] mod tests` with the TS test header above it, `use super::*; use jackioh_engine::testkit::*;`.
Testkit calls follow part 5.1's notes (the testkit's author), where they differ from part 11's bot
conventions: seats as `PlayerId`, `unit`/`backrow` answering owned `Option<CardInstance>`, `hand`/`pile`
owned `Vec<CardInstance>`, `card()` a `&CardInstance` (cloned before the next step), `hand_glows(&s, id, P1)`.

## DEPENDS-ON (names called in other parts' modules, with the shape assumed)
- Effects (parts 6/7), each taking one args struct built with `json_as(json!({ …TS literal… }))`, so
  only the struct's serde keys matter, not its Rust name:
  `effects::refresh_mana({ amount })`, `effects::add_player_modifier({ player, mod })` (the field is
  serde-renamed `mod`; its value deserialises as TS's `DistributiveOmit<PlayerModifier, "id">`, i.e.
  `ModifierKind` flattened beside `expiry: ModifierExpiry`), `effects::delay({ at: { phase, player },
  step, hook })` (`at.player` a `PlayerSpec` string), `effects::exile_hand({ player })`,
  `effects::discard_random({ count })`, `effects::summon(SummonArgs { defId, radiant, randomKeywords })`,
  `effects::set_radiant(RadiantTarget { instanceId })`.
- `prompts::RESUME_HOOK: &str` (part 3/4), serialised into `delay`'s `hook` key.
- `zones::active_units_of(&GameState, PlayerId)` → a `Vec` of `CardInstance` or `&CardInstance` (only
  `.iter().map(|u| u.id.clone())` is used); `zones::OffFieldZone::Hand`;
  `query::zone_cards(&GameState, PlayerId, OffFieldZone) -> Vec<CardInstance>` (part 2).
- `layers::keywords_of(&GameState, &CardInstance) -> Vec<Keyword>` (tests).
- Testkit (part 5): `scenario(Value) -> Scenario`; `play(card, Value)`, `end_turn()`, `answer(Value)`
  returning `&mut Scenario`; `state()`, `events()`, `last_events()`, `view(PlayerId) -> PlayerView`,
  `unit(PlayerId, i32)`, `backrow(PlayerId, i32)`, `hand(PlayerId)`, `pile(PlayerId, &str)`, `card(ref)`;
  `expect_in_zone`, `expect_stats(ref, Value)`, `expect_events(Value)`, `expect_mana(PlayerId, i32)`;
  `glow::hand_glows(&Scenario, &str, PlayerId) -> bool`; a card ref from `&str` or `&CardInstance`.
- Part 1 (exist): `crate::card_def(&str) -> CardDef`, `crate::register_all()`, `GameEvent::event_type`,
  `Keyword::kind().as_str()`, `PromptKind::Discover`, `Tag::Cn`, `HandView::Cards`, `ModifierView`,
  `StaticFlags.echo_grant`, `RANDOM_KEYWORD_POOL: &[&str]`, `PromptOption: Clone` (checked in `state.rs`).

## GAPS
- None left unported. Every name above is a guess at another part's signature (TS name snake_cased at
  its TS module's Rust path); the ones most likely to need an edit in Wave 3:
  - `ctx.state` passed straight where a read helper takes `&GameState` (`active_units_of(ctx.state, …)`,
    `zone_cards(ctx.state, …)`): relies on the `&mut GameState` → `&GameState` coercion through
    `EffectContext`'s `Deref`; `&*ctx.state` if the compiler wants it spelled out.
  - `add_player_modifier`'s args field for TS's `mod` must be `#[serde(rename = "mod")]` (Rust's
    `mod` is a keyword, so the field is probably `mod_`).
  - `s.unit(…)` answering `Option<CardInstance>` (part 5.1) rather than `Option<&CardInstance>` (part
    11's bot conventions): the type annotation `Vec<Option<CardInstance>>` in c081's last test and the
    `expect_stats_or(&mut s, s.unit(…))` calls assume owned copies.

## Decisions
- TS `const x: Hook = (ctx) => […]` → a private `fn x(ctx: &mut EffectContext<'_>) -> Vec<Effect>`
  wrapped as `hook(x)` where the script names it (`death`, `radiant_death`, `exile_the_hand`). TS
  helpers that read the context (`comboDraw`, `radiateYourUnits`, `radiateYourHand`) take
  `&EffectContext<'_>`.
- `resume: { [EXILE_STEP]: exileTheHand }` → `IndexMap::from([(EXILE_STEP, hook(exile_the_hand))])`.
- Zao Gao's `RUSH_TOKEN = cardDef("core-t-rush").id` (read from the catalog, not restated) is
  `crate::card_def("core-t-rush").id`, read once in `script()` and moved into both faces' closures — no
  static (SURFACE §3). The test file's own `RUSH_TOKEN` stays a literal, as in TS.
- TS factories (`fullsend(withComboDraw)`, `twinspell(amount)`, `zaoGao(tokens)`) are private fns
  returning `Script`; `Tokens` is a private `Copy` struct with `BASE_TOKENS`/`RADIANT_TOKENS` consts.
  Twinspell's `1`/`2` stay literals at the call, as TS wrote them.
- `[...new Set(ids)]` → collect into `indexmap::IndexSet` and back (first-seen order, SURFACE §4.4.2).
- Tests: `toMatchObject` → a private `matches_object` over the modifier's JSON (`js` = `serde_json::to_value`);
  `mods.filter(kind === k)` reads the JSON `kind`; `indexOf`/`findIndex` keep TS's `-1` through a
  private `index_of`; `s.expectStats(card ?? ID, …)` → a private `expect_stats_or` so the fallback is
  TS's; `expect(radiant).not.toBe(base)` → the two faces' Cry hooks are not the same `Arc`.
- Test names: titles snake_cased, `§` → `s`, `#` → `n`, R-ids leading as `r<n>` tokens (part 11's lane
  convention); one `mod` per `describe`, R-ids of a describe title leading its mod name.
- Every test that builds a scenario starts with `crate::register_all()` (part 5.1's note: the TS
  globalSetup's job); Zao Gao's card-data test reads `card_def` only and does not.
