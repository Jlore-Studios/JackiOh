# Slice: part 14 (cards lane 6), chunk 4 of 4 (#402): Classic #89, #90 and the Glitch token
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All three were absent on `staging` (no night-bot port at commit 0180138, no `part-14-bot.md`); all three
are full ports: the TS script whole (every function in TS order, its header as `//!`, every doc comment
and every comment that states a rule or cites a ruling) and the TS test file whole as the
`#[cfg(test)] mod tests` (its header above it, one `mod` per `describe`, one `#[test]` per `it`:
19, 45 and 10, the TS counts). No `todo!`, `unimplemented!` or `// TODO`. Every R-id the TS files cite
is in the Rust file.
- `crates/cards/src/scripts/classic/c089_paul_allens_ghost.rs` ← `089-paul-allens-ghost.ts` + test
- `crates/cards/src/scripts/classic/c090_in_too_deep.rs` ← `090-in-too-deep.ts` + test
- `crates/cards/src/scripts/classic/t_glitch_glitch.rs` ← `t-glitch-glitch.ts` + test

## SURFACE
Each file: `pub const ID`, `pub fn script() -> CardScripts`; everything else private. Part 1's frozen
names used as written: `Script`, `StaticFlags`, `TriggerDef::new(id, &[GameEventType], |ctx, event| …)`
(two arguments, `&mut EffectContext`), `hook`, `aura_hook`, `read_hook`, `AuraEntry { applies: Box<…>,
mod_ }`, `StatMod`, `GraveyardPlayPermission`, `QuestBook`/`QuestDef`/`QuestRewardDef`/`QuestGoal`
(script.rs), `Keyword::Indestructible`, `GameEvent::{QuestCompleted, QuestProgressed, Glitched, …}`,
`PromptKind::Reward`, `BackrowView::Public(PublicBackrowView { quest, .. })`, `GlitchOutcome`,
`GameResult { winner: Winner::Draw, reason: GameOverReason::Voided }`, `GLITCH_DEF_ID`,
`GLITCH_ODDS_DENOMINATOR`, `GLITCH_OUTCOMES`, `LIBRARY_CAP`, `find_instance_mut`.

## DEPENDS-ON
Part 1 (frozen): the names above, `crate::register_all()`, `crate::card_def(id) -> CardDef`.

## GAPS
### Not ported
Nothing. TS `def` exports are not ported (SURFACE §7.1).

### Called in other parts' modules (TS name snake_cased at its TS path; the shape assumed)
- Effects (parts 6–7), each `fn(<Args>) -> Effect` whose one argument deserialises from TS's object
  literal (every call is `json_as(json!({ … }))`, so only the serde shape matters): `heal` (`target`,
  `amount`), `choose_target` (`step`, `scope { side, of }`, `prompt`), `return_random_from_graveyard`
  (`count`), `place_plague_tokens` (`count`), `buff_random_unit` (`attack`, `health`),
  `choose_from_hand` (`of`, `by`, `count`, `step`, `prompt`; its `where_` callback skipped by serde),
  `draw` (`count`), `next_turn_mana` (`amount`), `exile_bottom_of_library` (`player`, `count`),
  `choose_reward` (`step`, `rewards: [{ id, label }]`, `prompt`), `damage` (`to`, `amount`), `bounce`
  (`target`), `discard` (`target { of: "chosen", index }`); `recruit(Default::default())` (TS's
  `args = {}`); `chosen_options(&EffectContext) -> Vec<String>` (called with `&mut`, which coerces);
  `effects::glitch() -> Effect` (effects/mod.rs's re-export of `subsystems::glitch::glitch`).
- `subsystems::quests` (part 8.3's notes): `quest_def_of`/`quest_reward_of(&QuestBook, &str) ->
  Option<&…>`, `open_quest`/`hold_quest_aura(impl Into<String>) -> Effect`,
  `held_quest_auras(&CardInstance) -> Vec<String>`, `quest_memory_of(&CardInstance) ->
  Option<QuestMemory>` with public fields `active`, `progress: IndexMap<String, i32>`, `done`, `auras`,
  `waiting` (all `Serialize`; `QuestMemory` itself need not be), `QUEST_MEMORY_KEY: &str`.
- `params::param(&impl ParamContext, &str) -> i32` for `HookArgs` (part 2.1); `params::step_param(&mut
  CardInstance, &str, i32)`; `scripts::flags_of(&GameState, &CardInstance) -> StaticFlags` (part 2.1).
- `catalog::query(&CatalogQueryArgs)` returning `Vec<CardDef>` or `Vec<&CardDef>` (only `.id` is read),
  `CatalogQueryArgs: Deserialize` from `{ withTokens }` and `{ set, token }`.
- Engine entry points: `reduce(&GameState, &Action) -> ReduceResult { state, events, error }`,
  `begin_game`, `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`, `view_for`, `hash_state`,
  `create_game(&CreateGameOptions)` (built with `json_as`), `replay::fold(&ReplayInput) ->
  ReplayResult { state, errors }` with **`ReplayInput: Deserialize`** from `{ seed, decks, log }`
  (built with `json_as`; part 5.1's notes do not say it derives it), `seat_played_by` (root).
- The testkit as part 5.1's notes give it: `scenario(Value)`, `play`/`answer`/`attack`/`activate`/
  `end_turn` returning `&mut Scenario`, `answer(json!("A"))`, `answer(json!(id))`, `answer(json!([ids]))`
  and `answer(json!([selection]))` (TS's three answer forms), `attack(card, "hero")`, `unit`/`hand`/`pile`
  returning copies, `card(ref) -> &CardInstance` (refs: `&str`, `&String`, `&CardInstance`),
  `stats(ref) -> layers::UnitView { keywords, .. }`, `view(seat) -> PlayerView`, `state()`,
  `state_mut()`, `events()`, `last_events()`, `expect_in_zone`, `expect_stats`, `expect_health`,
  `expect_refused(|s| s.play(…))`.

## Decisions
- `IN_TOO_DEEP_QUESTS` (a TS module constant) is `fn in_too_deep_quests() -> QuestBook`, built with
  typed struct literals through two private builders (`quest`, `reward`) so a wrong variant is a
  compile error, not a `json_as` panic; a `QuestBook` owns `String`s, so it cannot be a `const`, and
  SURFACE §3 allows no other static. Each hook that reads the tree builds it (ten quests, thirteen
  rewards: cheap).
- `rewardOptions` returns `Vec<Value>` of `{ id, label }`, which goes straight into `choose_reward`'s
  `json!` literal.
- `completedHere` reads `ctx.self_` (only the id is read, which a live re-read would not change).
- Test modules follow part 9.1: `use super::{…}; use jackioh_engine::testkit::*;` (never `use super::*`
  at the top of `mod tests`, so the prelude's effect verbs never meet the testkit's globs), nested
  `mod`s `use super::*;`, every `#[test]` starts with `crate::register_all();`. Ambiguity-prone engine
  functions are named by path (`jackioh_engine::scripts::flags_of`, `jackioh_engine::params::step_param`,
  `jackioh_engine::catalog::query`).
- Names: the title lower-cased, `§` → `s`, `#` → `n`, apostrophes and other punctuation dropped,
  every other run of non-alphanumerics → `_`, ruling tokens leading (`b5_e5_r682_…`, `r471_r689_…`).
  Describes: `c_n89_paul_allens_ghost`, `c_n90_in_too_deep`, `t_glitch_glitch`, then `base`/`radiant`.
- TS `expect(radiant).toBe(base)` (identity) is `Arc::ptr_eq` of the face's one hook: `radiant` is
  `base.clone()`, so the `Arc`s are the same allocation.
- TS `!("discards" in play)`: `ActionBody` has no `discards` field, so the check is that the action's
  JSON has no `discards` key (it still guards a field added later).
- C #89's module-level `let nonce` is a per-test `&mut u32` counter passed to `send` (no mutable statics
  in a pure crate's tests either); `send` builds the `Action` from JSON (`json_as`).
- TS `toMatchObject` is a private `matches_object` over JSON with Jest's rules (objects partial,
  arrays the same length, element by element). The quest line is compared as JSON built from
  `QuestMemory`'s fields, so the test needs no `Serialize`/`PartialEq` on `QuestMemory`.
- C #90's `SPARE` spread is `spare(extra)`, a JSON merge; no side setup names a key SPARE also has, so
  TS's spread order is immaterial. `shown()` returns the `PublicBackrowView` (TS typed it `CardView`).
- TS `act` (a closure over `log` and `n`) is a nested `fn act(state, body, &mut n, &mut log)`.
- `Object.fromEntries(…)` compared with `toEqual` is a `BTreeMap` comparison (key order ignored, as TS).
- Glitch: `[...seen].sort()` against the sorted outcomes compares `Vec<Option<String>>` sorted; the
  seeds `t-glitch-<outcome>-<n>` use `GlitchOutcome`'s `Display` (its literal), as TS's template did.
