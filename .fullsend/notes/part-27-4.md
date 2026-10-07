# Slice: part 27 (engine tests 4), chunk 4 of 8
BUILDS-RUN: 0

## FILES
- `crates/cards/tests/cross/paused_sequences.rs` ← `packages/cards/test/paused-sequences.test.ts` (45 tests, 42 mods)
- `crates/cards/tests/cross/play_choices.rs` ← `packages/cards/test/play-choices.test.ts` (7 tests, 6 mods)
- `crates/cards/tests/cross/plays_and_casts.rs` ← `packages/cards/test/plays-and-casts.test.ts` (12 tests, 6 mods)
- `.fullsend/notes/spec-gaps-part-27-4.md`, this file, `part-27-4.assumptions`

Every TS `it` is one `#[test]`, every `describe` one `mod`, in TS order, with the header comment and
every rule-stating comment kept. Ruling ids in a title are `r<n>` tokens (SURFACE §7.3); a title that
opens with `§x.y` opens with `sx_y`, a `§` inside a title is its numbers.

## GAPS
Names these files call that other parts provide (TS name snake_cased at its TS module's Rust path,
SURFACE §4). Signatures are this chunk's guesses where SURFACE and part 1 fix none.

Testkit (part 5, `testkit::scenario`):
- `scenario(Value) -> Scenario`; `Scenario::{state() -> &GameState, state_mut() -> &mut GameState,
  events() / last_events() -> &[GameEvent] (or &Vec), play(&str, Value), answer(Value), attack(&str,
  &str), activate(&str, Value), start_turn(), end_turn(), card(&str) -> &CardInstance, stats(&str) ->
  UnitView, unit(PlayerId, lane) / backrow(PlayerId, lane) -> Option<&CardInstance> (read with
  `.cloned()`), hand(PlayerId) and pile(PlayerId, &str) (a Vec or a slice; only iterated and measured),
  view(PlayerId) -> PlayerView, expect_in_zone(&str, &str), expect_stats(&str, Value),
  expect_health(PlayerId, i32)}`. A card ref is always a `&str`: a catalog id or an instance id.
- `register_scripts(IndexMap<String, CardScripts>)` (the thread-local override). **Needed:**
  `scripts::registered_scripts()` must return the override once one is set, since a test here registers
  several fixtures in a row (`{ ...registeredScripts(), [id]: … }`), and `scripts::script_of` must look a
  transient def id up in the registry (override first) before composing fused scripts from
  `state.transient_defs`: these fixtures are transient defs with registered scripts, "held like a
  fusion's" (TS `fixture`, `fixtureFaces`, `fixtureCard`).

Cards (part 1): `jackioh_cards::register_all()`, called first in every test.

Engine (parts 2–8), by module:
- `catalog::query(&CatalogQueryArgs) -> Vec<CardDef>` (`CatalogQueryArgs: Deserialize`, built by `json_as`).
- `scripts::registered_scripts()` (cloned into an `IndexMap<String, CardScripts>`).
- `zones::{place_on_field(&mut GameState, CardInstance, ZoneSlot, <options>: Default) -> bool,
  ZoneSlot { player, row, lane: i32 }, active_units_of(&GameState, PlayerId) -> Vec<_>}`.
- `modifiers::schedule_delayed(&mut EngineSink, PlayerId, DelayedAt, Resume, Option<String> /* watch */,
  Option<i32> /* notBefore */) -> DelayedEffect`.
- `prompts::{answer_prompt(&mut EngineSink, AnswerInput) -> Result<(), EngineError>, AnswerInput {
  player_id, choice_id, selection: Vec<Selection> }, RESUME_HOOK: &str}`.
- `layers::unit_view(&GameState, &CardInstance) -> UnitView { attack, health, max_health, .. }`.
- `mana::{effective_cost(&GameState, &CardInstance, CostOptions) -> i32, CostOptions: Default}`.
- `subsystems::{CHAOS_EFFECTS (a slice whose items have `name`), roll_chaos_effects(&mut Rng, bool,
  <table>) -> Vec<_> (items with `name`), POWER_KEY: &str, power_of(&CardInstance) -> Option<_> (with
  `x: i32`, `name`), power_ability_of(&GameState, &CardInstance) -> Option<_> (with `id`),
  used_this_turn(&GameState, &CardInstance) -> bool, fuse(&mut EngineSink, FuseArgs) ->
  Option<CardInstance>}` (`FuseArgs: Deserialize`, built by `json_as` from the TS literal with the
  instances serialised).
- `effects::{choose_target, choose_mode, choose_from_hand, damage, damage_all, buff, heal, destroy,
  sacrifice, bounce, discard, exile, draw, add_to_hand, set_cost_mod, set_radiant, steal, summon, delay,
  forced_attacks, rotate}`, each taking one argument struct that `json_as` builds from the TS literal
  (`Deserialize`, SURFACE §6.6), and `effects::chosen_options(&EffectContext) -> Vec<String>`.

## Decisions
- TS's optional parameters are passed explicitly, never dropped: `roll_chaos_effects(rng, radiant,
  subsystems::CHAOS_EFFECTS)`, `effective_cost(state, card, CostOptions::default())`,
  `schedule_delayed(.., None, None)`. Other chunks differ (`pools_and_randomness.rs` calls
  `roll_chaos_effects` with two arguments, part 25.1's notes `effective_cost` with two); part 31 picks one.
- Every argument object the TS wrote as a literal (effect args, `FuseArgs`, `CatalogQueryArgs`,
  `StaticFlags`, `TargetDecl`, `CardDef`, `Action`, `ActionBody`) is `json_as(json!(<the literal>))`, so the
  test reads as the TS did and depends only on serde shapes. `AnswerInput` and `Resume`/`DelayedAt` are
  struct literals (their fields are TS's three and six).
- TS `sinkFor(s)` is `with_sink(&mut s, |sink| …)`: a sink over `s.state_mut()` whose events and rng
  draws are discarded, as TS's were (it wrote the state in place and dropped the rest).
- A TS write through a live instance the test held (`grudge.buffs.attack += 3`, `memory.power = …`,
  `grantedKeywords.push`) is `find_instance_mut(s.state_mut(), id)`; `placeFixture` returns the placed
  card as `s.card(id)` reads it.
- TS `findIndex`/`indexOf` comparisons keep their -1 (`i64` helpers), so `a > b` reads as in TS when one
  is missing.
- `expect(() => s.play(x)).not.toThrow()` is the call itself: the harness panics on a refusal.
- `play_choices`' module counter `let nonce = 0` is a `static AtomicU32`.
- The TS fixture helpers' default parameters: `stats = {2/2}` → `Option<Value>`, `options = {}` → a
  `Value`, `cost = 0` → an explicit `i32` at every call. `type: CardType` is a `&str` (the def is JSON).
- `LONG_PLAY_TIMEOUT_MS` (vitest's per-test timeout) is not ported: a `#[test]` has none. Its comment is
  kept beside the constants.
