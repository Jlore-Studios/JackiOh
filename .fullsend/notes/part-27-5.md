# Slice: part 27 (engine tests 4), chunk 5 of 8 — cross-card tests: preview, re-entry, resolving face, self-generation
BUILDS-RUN: 0 (no cargo, rustc, rustfmt, clippy, pnpm, tsc or test runner; a python one-liner
slugged the TS titles into test names)

## FILES
- `crates/cards/tests/cross/preview.rs` ← `packages/cards/test/preview.test.ts` (95 tests)
- `crates/cards/tests/cross/re_entry.rs` ← `packages/cards/test/re-entry.test.ts` (27 tests)
- `crates/cards/tests/cross/resolving_face.rs` ← `packages/cards/test/resolving-face.test.ts` (7 tests)
- `crates/cards/tests/cross/self_generation.rs` ← `packages/cards/test/self-generation.test.ts` (2 tests)
- `.fullsend/notes/spec-gaps-part-27-5.md`, this file, `part-27-5.assumptions`

## SURFACE
Written against SURFACE §8's testkit table and part 1's frozen types (`state.rs`, `script.rs`,
`wire/*.rs`, `lib.rs`, `prelude.rs`, `testkit/mod.rs`, `subsystems/mod.rs`, `crates/cards/src/lib.rs`).
No Rust under `crates/*/src/` other than those was read, and no other part's test file.

## DEPENDS-ON
- testkit (`crates/engine/src/testkit/scenario.rs`): `scenario(Value) -> Scenario` and the §8 verbs.
- `jackioh_cards::{register_all, scripts_of, card_def, CATALOG, CATALOG_IDS}` (part 1, present).
- `reduce`, `legal_actions`, `view_for` (through `s.view`) at the engine root (§6.1).

## GAPS
Names called that another part is expected to provide (none of them is in part 1's frozen files):
- `testkit::Scenario::state_mut(&mut self) -> &mut GameState` — TS tests write `s.state.…` in place
  (`players.p1.mana.current = 5`, `mods.push(…)`, a card's `timesPlayed`/`grantedKeywords` via
  `find_instance_mut`, `rngCursor`) and build an `EngineSink` over the live state. SURFACE §8 names
  `s.state()` only.
- testkit shapes assumed (SURFACE §8 leaves them open): card references are passed as `&str` (an
  instance id or a catalog id); `unit`/`backrow(PlayerId, lane)` return `Option<&CardInstance>` (the
  code is also correct for `Option<CardInstance>`); `hand(PlayerId)` a `Vec<CardInstance>` or a slice;
  `pile(PlayerId, "library")` takes the zone as `&str`; `view(PlayerId) -> PlayerView`;
  `stats(&str) -> layers::UnitView`; `events()`/`last_events()` a slice or `&Vec<GameEvent>`;
  `expect_refused(|s| { s.play(…); })` takes a closure returning `()`; `play(&str, Value)`,
  `answer(Value)`, `start_turn()`, `end_turn()`, `expect_in_zone(&str, &str)`,
  `expect_stats(&str, Value)`, `expect_health(PlayerId, i32)`, `expect_mana(PlayerId, i32)`; return
  values of the verbs are never chained.
- `jackioh_engine::subsystems::fuse::fuse(sink: &mut EngineSink, args: FuseArgs) -> Option<CardInstance>`
  (re-exported as `subsystems::fuse`), and `subsystems::fuse::FuseArgs` deriving `Default` with
  `ingredients: Vec<CardInstance>`, `target: Option<CardInstance>`, `to_hand: Option<PlayerId>`
  (TS `FuseArgs`; the other optional fields left to `..FuseArgs::default()`).
- `jackioh_engine::mana::{effective_cost(&GameState, &CardInstance, &CostOptions) -> i32, CostOptions}`
  with `CostOptions: Default` (TS's defaulted `options: CostOptions = {}`).
- `jackioh_engine::params::step_param(&mut CardInstance, &str, i32)`.
- `jackioh_engine::scripts::INGREDIENTS_KEY: &str` (`"__ingredients"`).
- Test-name tokens: `c4_5_…`, `c3_2_…`, `c7_…`, `c8_52_…` (module and test names from titles that
  start with `§`) carry no `r<n>` token of their own; every R-id in a title is kept as a token.

## Decisions
- Every test starts with `jackioh_cards::register_all();`: TS's harness registered the catalog and
  scripts on import, and the engine's testkit cannot (the engine does not depend on the cards crate).
- One `mod` per TS `describe`, one `#[test]` per TS `it`. An `it` inside a TS loop
  (`for (const [face, …] of CASES) it(…)`) becomes one `#[test]` per case, each calling a private fn
  holding the body, named from the title with the case's values filled in. TS describe-local helpers
  are private fns in their `mod`; file-level helpers stay at the file's top.
- Names: the TS title lowercased, apostrophes dropped, every other run of non-alphanumerics `_`; a
  name that would start with a digit gets `c` (SURFACE §4.1's slug rule: `§4.5 …` → `c4_5_…`,
  `#18 …` → `c18_…`). Every `R<n>` anywhere in a title stays an `r<n>` token, as TS's coverage
  counted an R-id anywhere in a title.
- Card references go to the harness as `&str` instance ids (`s.card(X).id.clone()`), never as a held
  `CardInstance`: harness rule 1 resolves an instance id first, and an id never goes stale.
- `s.view`, `s.hand` always name the seat (TS's optional player defaulted to the active one).
- `toEqual` on preview lists and view parts compares their JSON (`serde_json::to_value`), which treats
  an absent optional exactly as toEqual does; `JSON.stringify(x)` checks use `Value::to_string()`.
- `subsystems.fuse({ state: s.state, events: [], rng })` is `fuse_on(&mut s, args)`: a fresh
  `EngineSink` over `s.state_mut()`, its rng cursor not written back (as TS did not); re-entry's
  `craft` writes the cursor back, as TS's did.
- A TS write to a live instance (`s.card(x).timesPlayed = …`, `stepParam(s.card(x), …)`) is a write
  through `find_instance_mut(s.state_mut(), id)`.
- The preview "pure read" fences are perturbations, not throwing getters (spec-gaps file).
- `expect(() => g.play(…)).toThrow()` is `g.expect_refused(|g| { g.play(…); })`.
