# Slice: part 26 (engine tests 3: view, turn, setup, combat), chunk 3 of 7 (#414)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All eight were empty placeholders on `staging`; all are full ports (every `describe` a `mod`, every `it` a
`#[test]`, in TS order, with the header comment and every rule- or ruling-citing comment kept):
- `crates/engine/tests/rules/game_summary.rs` ← `packages/engine/test/game-summary.test.ts` (6 tests, 1 mod)
- `crates/engine/tests/rules/generation_replay.rs` ← `generation-replay.test.ts` (1, 1)
- `crates/engine/tests/rules/glow_facts.rs` ← `glow-facts.test.ts` (9, 5)
- `crates/engine/tests/rules/handicap.rs` ← `handicap.test.ts` (66, 5)
- `crates/engine/tests/rules/hotseat_smoke.rs` ← `hotseat.smoke.test.ts` (1, 1)
- `crates/engine/tests/rules/instance_data.rs` ← `instance-data.test.ts` (8, 3)
- `crates/engine/tests/rules/kill_credit.rs` ← `kill-credit.test.ts` (7, 2)
- `crates/engine/tests/rules/layers.rs` ← `layers.test.ts` (23, 3)
- Notes: this file, `part-26-3.assumptions`, `spec-gaps-part-26-3.md`.

No file is left. No `todo!`, `unimplemented!`, `#[ignore]` or `// TODO`.

## GAPS
Tests not fully expressible: see `spec-gaps-part-26-3.md` (`NaN`, `Infinity` and `null` as a
`heroHealth` cannot be ported; fractions and wrong types are refused by serde instead of
`validateHandicap`; identity checks; dropped timeouts and `beforeAll`/`afterAll`).

Names called that another part provides (TS name snake_cased at its TS module's Rust path, SURFACE §4).
The shapes are the ones these files assume; where the owning part's notes give a signature, it is used.

### Fixtures (part 24), as `crate::rules::fixtures::<file>`
- `harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState` (TS's optional `decks`
  passed explicitly, as part 24-1 has it); `setup_catalog()`; `put(&mut GameState, &str, ZoneSlot) ->
  CardInstance` (three arguments; the `{ radiant: true }` form is a local `put_radiant` built from
  `new_instance` + `place_on_field`, the radiant flag set before placing as TS's `put` does);
  `slot(PlayerId, Row, i32) -> ZoneSlot`; `in_hand(&mut GameState, &str, PlayerId, count) ->
  Vec<CardInstance>` (count always passed); `set_library(&mut GameState, PlayerId, &[String]) ->
  Vec<CardInstance>`; `play_random_game(&str, Option<(Vec<String>, Vec<String>)>) -> X` with fields
  `state: GameState`, `log: Vec<Action>`, `decks: (Vec<String>, Vec<String>)`. `events_of_type` and
  `sink_for` are not called (events are matched on `GameEvent` variants; sinks are built from frozen types).
- `catalog`: `vanilla_deck(i32, i32) -> Vec<String>`, `vanilla_catalog(i32, i32) -> CardDefs`,
  `token_def(&str) -> CardDef`.
- `combat`: `plain, big_body, zero_attack, taunter, shielded, armoured, indestructible, poisonous,
  trampler, stacker, big_dfender, spikey_pillow` — each TS `export const x: CardDef` a `pub fn x() -> CardDef`.
- `call_to_chaos_plus::immutable()`; `scripts::{hinder, going_long, heroic_power}()`.
- `generation`: `plague_book, slime, crawler, toxins, outbreak, dusting, scatter, charger, quiet_trap, body,
  big_body, fuse_a, fuse_b, lab, slop, deck_fusion, mutate, fuser, juhan, pile_on` (zero-arg fns),
  `register_generation()`.
- `instance_data`: `body, constant, military, nerfer, numbered` (zero-arg fns), `CONSTANT_STEP: &str`,
  `instance_game(&str) -> GameState`, `instance_deck(PlayerId) -> Vec<String>`,
  `register_instance_fixtures()` (return ignored).
- `kill_credit`: `bot, jungle, tempo` (zero-arg fns), `register_kill_credit()`.
- `damage_combat`: `grunt, death_asker` (zero-arg fns), `playing(&str) -> GameState`, `answer(&GameState)
  -> ReduceResult`, `round_trip(&GameState) -> GameState`, `replays_to(&GameState, &[Action], &GameState)
  -> bool`, `recorder(&GameState) -> Recorder` with `play(&mut self, ActionInput) -> ReduceResult`,
  `state(&self)` (a `&GameState` or an owned one: only `.clone()`d), and pub fields `start: GameState`,
  `log: Vec<Action>`.

### Engine (parts 2–8, 5 for the testkit)
- Root (SURFACE §6.1): `create_game(&CreateGameOptions)`, `begin_game`, `reduce(&GameState, &Action) ->
  ReduceResult`, `legal_actions -> Vec<ActionBody>`, `seat_to_act -> Option<PlayerId>`, `view_for`,
  `hash_state`, `fold(&FoldArgs) -> FoldResult { state, errors: Vec<_> }` with `FoldArgs: Default`
  (`seed, decks, log, catalog: Option<CardDefs>, handicaps: Option<PerPlayerOpt<Handicap>>, ..`; part 5.1
  names it `ReplayInput` with `FoldArgs` an alias), `summarize_game(&FoldArgs) -> Option<GameSummary>`
  (`GameSummary: PartialEq + Serialize`, wire). `create_game` and `fold` panic with TS's message on a deck
  or handicap TS refused (part 1, part 5.1); the tests catch the panic and read its `String`/`&str` payload.
- `catalog::{registered_catalog() -> &'static CardDefs, def_of(Option<&GameState>, &str)}` (`.tags`
  read); `scripts::registered_scripts()` (owned `IndexMap<String, CardScripts>`); testkit
  `register_catalog(CardDefs)`, `register_scripts(IndexMap<String, CardScripts>)`.
- `layers::{unit_view(&GameState, &CardInstance) -> layers::UnitView { attack, max_health, health,
  keywords, armor, position }, face_of(&GameState, &CardInstance)` (a `Serialize` value written
  `{ attack, health, keywords }`), `unit_has(&GameState, &CardInstance, KeywordKind) -> bool}`.
- `zones::{ZoneSlot` (`Copy`, fields `player`, `row`, `lane`), `place_on_field(&mut GameState, &mut
  CardInstance, ZoneSlot, PlaceOnFieldOptions { stack: Option<bool> }: Default) -> bool`,
  `remove_from_field(&mut GameState, &CardInstance, Default) -> bool`, `move_to_zone(&mut GameState, &mut
  CardInstance, OffFieldZone, Default)`, `OffFieldZone::{Hand, Graveyard}`, `active_units_of` and
  `dormant_units_of(&GameState, PlayerId)` (references or owned: only `.id` is read, or the item is passed
  on as `&_`)`}` — part 2.1's shapes.
- `state_check::state_check(&mut EngineSink)`; `resolve::{make_context(&mut EngineSink,
  Option<&CardInstance>, HookOptions) -> EffectContext, HookOptions { controller: Option<PlayerId>, .. }:
  Default}` (part 3.2's shape); `EffectContext: DerefMut<Target = EngineSink>` (part 1).
- `effects::{buff, grant_keyword, buff_cards, grant_keyword_cards, recruit}`: one argument each, built with
  `json_as(json!(TS literal))`.
- `draw::{draw(&mut EngineSink, PlayerId, i32), draw_one(&mut EngineSink, PlayerId, None) -> DrawOutcome,
  DrawOutcome::Cast (PartialEq + Debug), DRAW_COUNT_WORK: &str, owed_draw_count_of(&Resume) ->
  Option<OwedDrawCount { player, count }>}`; `work::owed_work(&GameState, Option<&str>) -> Vec<WorkItem>`.
- `prompts::{open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>, OpenPromptArgs {
  player, kind, aim, prompt: String, options: Vec<PromptOption>, min, max, budget, owner, resume }`
  (every optional an `Option`), `resume_self(&EffectContext, &str, IndexMap<String, Value>) -> Resume}`.
- `setup::{mulligan_prompt_for(&GameState, PlayerId) -> Option<&PendingChoice>, opening_hand_size(&GameState,
  PlayerId) -> i32}`; `mana::{max_mana_for(&PlayerState) -> i32, refresh_mana(&mut PlayerState)}`;
  `damage::{heal_hero, heal_hero_up_to}(&mut EngineSink, PlayerId, i32) -> i32`.
- `subsystems::ai_policy::choose_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>` (part 8.1).
- `query::{granted_combo_live(&GameState, PlayerId), gifted_would_make_radiant(&GameState, PlayerId,
  &CardInstance)} -> bool`, `query::lethal_attackers_of(&GameState, PlayerId)` and
  `query::fusable_permanents_of(&GameState, PlayerId, Option<&str>)` (lists; only `.id` read);
  `condition::condition_active(&GameState, &CardInstance, PlayerId, ConditionZone) -> bool`.
- `kill_credit::{KILL_CREDIT_KEY: &str, credited_killer_id(&CardInstance, victim_id: &str)}` (TS's
  `Pick<CardInstance, "id">` taken as the id; the answer compared with a `String` and a `&str`);
  `restrictions::is_berserk(&CardInstance) -> bool`.

## Decisions
- Module paths are named for every engine function (`layers::unit_view`, `zones::place_on_field`, …):
  the testkit's `pub use crate::*` brings the modules too, and a qualified call cannot hit a root-glob
  ambiguity. Types and SURFACE §6.1's entry points are named bare.
- **Sinks** are built from frozen types, as parts 24-1 and 25-1 do: `Rng::new(&state.seed,
  state.rng_cursor)` + `EngineSink::new`. The cursor is written back exactly where TS wrote it back
  (instance-data's `run`) and nowhere else (`sinkFor` in a draw, a state check, kill-credit's `drawOne`).
  The heal test's TS sink had no rng; the Rust sink carries one that neither heal draws from.
- **Live objects**: writes go through `find_instance_mut` by id (`edit` in layers), reads take the card back
  from the state (`live`) before each engine call and assertion. A card passed to `move_to_zone` or
  `remove_from_field` is that live copy.
- **Fixture values**: TS `CardDef` constants are zero-arg fns (`plain().id`); local defs are JSON literals
  through `json_as::<CardDef>`, each with the `index` TS's module counter gave it, written out.
- **Assertions**: `toMatchObject` on a `UnitView` is the named fields as a tuple; on a hand `CardView` the
  `Option` fields. `toEqual` on a view part, a summary or a face is JSON equality (`serde_json::to_value`).
  Events are matched on `GameEvent` variants (frozen). Regexes are `contains`, or `in_order` for `/a.*b/`.
- **Errors**: `validate_handicap`/`validate_deck` are `Result<(), EngineError>`, read through
  `e.message`; `create_game` and `fold` refusals are caught panics (`refusal`, `catch_unwind` with
  `AssertUnwindSafe`; the testkit's thread-local registries survive it, being on the same thread).
- **Nonces**: TS's module-level counters (instance-data, handicap) are `static AtomicU32`s.
- **Policies**: the random-policy loops (generation-replay, instance-data) port TS's ternary with its
  short-circuit: `policy.chance` is drawn only when the other actions are non-empty and an endTurn exists.
- game-summary keeps TS's `splice(lastIndexOf(x), 1)` quirk (a missing entry takes the last one) and
  `slice(-1)` for a step with no `turnStarted`, as part 5.1's cards-side port does.
- Test names: an `R<n>` leading a title leads the Rust name (`r376_…`, `r42_r412_…` on mods); `§x.y`
  becomes `sx_y`, `#46` `c46`. Titles without a ruling get none added.
