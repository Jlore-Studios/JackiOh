# Slice: part 27 (engine tests 4: play pipeline and cross-card rules), chunk 8 of 8 (#415, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All seven were empty placeholders on `staging`; all are whole ports (every `describe` as a `mod`, every
`it` as a `#[test]`, in TS order, the header comment and every rule- or ruling-citing comment kept, the
TS title kept as a `///` line on each test). `#[test]` counts equal the TS `it` counts file by file
(68 in all):
- `crates/engine/tests/rules/mana.rs` ← `mana.test.ts` (11)
- `crates/engine/tests/rules/overflow_events.rs` ← `overflow-events.test.ts` (13)
- `crates/engine/tests/rules/play_pipeline_b_replay.rs` ← `play-pipeline-b-replay.test.ts` (1)
- `crates/engine/tests/rules/play_step3.rs` ← `play-step3.test.ts` (12)
- `crates/engine/tests/rules/play_choices_filters.rs` ← `playChoices-filters.test.ts` (14)
- `crates/engine/tests/rules/play_choices.rs` ← `playChoices.test.ts` (8)
- `crates/engine/tests/rules/play_counts.rs` ← `playCounts.test.ts` (9)
Notes: this file, `part-27-8.assumptions`, `spec-gaps-part-27-8.md`. Nothing left; no rebase conflicted.

## GAPS
Tests not ported: none (`spec-gaps-part-27-8.md` lists the assertions written against the nearest Rust
observable).

Names called that another part provides (TS name snake_cased at its TS module's Rust path; the shape
assumed, taken from the owner's notes where they give one, else from the other test chunks'):

Fixtures (part 24, `crate::rules::fixtures::<file>`):
- `harness::{new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState, setup_catalog(), put(&mut
  GameState, &str, ZoneSlot) -> CardInstance (a copy), slot(PlayerId, Row, i32) -> ZoneSlot, in_hand(&mut
  GameState, &str, PlayerId, i32) -> Vec<CardInstance>, set_library(&mut GameState, PlayerId, &[String])}`
  (the majority shapes of parts 24.1–24.4 and 25.1). `sinkFor` and `eventsOfType` are not called.
- `scripts::{x_bolt, going_long, heroic_power, cn_virus, infinite_reserves}() -> CardDef`;
  `combat::{plain, stacker}() -> CardDef`.
- `play_pipeline_a::{PA, with_play_a(GameState) -> GameState}`: `PA` is TS's constant object, kept
  under its name (SURFACE §4.2) as a static that derefs to a struct whose fields are the TS keys
  snake_cased, each a `CardDef` (`PA.pact.id`, `PA.hidden_field_trap.id`, `PA.ai_card.id`,
  `PA.echo_copy.id`, `PA.counter_trap.id`); e.g. `pub static PA: LazyLock<Pa>`. Keys used: bolt, ping,
  crier, refusal, counter_trap, hidden_trap, hidden_field_trap, pact, book, produce, apple, pear,
  echo_copy, ai_card, field.
- `play_pipeline_b::{RANDOM_POOL: &[&str], register_pipeline_b(), second_wind, plantation, jogg_box,
  solarius, forever, trickster, toe_cracker, monkey, tax, grave_spell, x_target, target_spell,
  mode_spell, discover_spell, their_choice, titan, named_caster, tyrant, cast_trap, grave_unit}` (the
  defs as `fn() -> CardDef`).

Engine (parts 2–8), with the owners' shapes:
- `mana::{effective_cost(&GameState, &CardInstance, CostOptions) -> i32, printed_cost, can_afford(&GameState,
  &CardInstance), gain_mana(&mut PlayerState, i32), spend_mana(&mut PlayerState, i32), refresh_mana(&mut
  PlayerState), max_mana_for(&PlayerState) -> i32}`, `CostOptions: Default` (part 4.1).
- `modifiers::add_modifier(&mut EngineSink, PlayerId, ModifierExpiry, ModifierKind)` (part 3.2), the two
  halves built from TS's literal with `json_as`.
- `draw::{draw(&mut EngineSink, PlayerId, i32), draw_one(&mut EngineSink, PlayerId, None) -> DrawOutcome
  (Fatigue, Token, Burned matched with `matches!`), shuffle_into_library(&mut EngineSink, &mut CardInstance,
  bool, Option<&str>) -> ShuffleInOutcome (Dropped)}` (part 4.2).
- `play_choices::{declared_targets(&GameState, &CardInstance) -> Vec<TargetDecl>, declared_modes(..) ->
  Vec<ModeDecl>, legal_selections_for(&GameState, PlayerId, &CardInstance, &TargetDecl) -> Vec<Selection>,
  play_choice_combinations(&GameState, PlayerId, &CardInstance, None) -> Vec<PlayChoices>, PlayChoices {
  targets: Option<Vec<Selection>>, modes: Option<Vec<String>> }, why_choices_refused(&GameState, PlayerId,
  &CardInstance, &PlayAction) -> Result<(), EngineError>, PlayAction (built by `json_as` from the TS
  `{ type: "play", … }` literal: part 4.2 serialises it through `ActionBody`), resolving_face(..) ->
  CardInstance, play_actions_for(..) -> Vec<PlayAction> (`.targets`)}` (part 4.2; `declared_*` take the state
  as its owner wrote them, TS took the card alone).
- `query::{played_this_turn_of_type(&GameState, PlayerId, &[CardType]) -> i32, played_this_game_with_tag(
  &GameState, PlayerId, Tag) -> i32, last_spell_played(&GameState) -> Option<PlayRecord>,
  last_face_up_played(&GameState, PlayerId) -> Option<FaceUpRecord>}` (part 2.2).
- `resolve::{cast_card(&mut EngineSink, &CardInstance, CastOptions), CastOptions: Default, make_context(&mut
  EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext, HookOptions { controller, .. }: Default}`
  (part 3.2's `make_context`; `cast_card` as the test chunks assume); `triggers::{settle(&mut EngineSink,
  SettleOptions), SettleOptions: Default}` (part 3.3).
- `zones::{card_at(&GameState, impl Into<ZoneSlot>) -> Option<&CardInstance>, lock_zone(&mut GameState,
  ZoneSlot), place_on_field(&mut GameState, &mut CardInstance, ZoneSlot, PlaceOnFieldOptions { stack, .. })
  -> bool, move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, MoveToZoneOptions: Default),
  active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>}`, `OffFieldZone::{Hand, Graveyard}` (part 2.1).
- `replay::{fold(&FoldArgs) -> FoldResult { state, errors }, FoldArgs { seed, decks: (Vec<String>,
  Vec<String>), log: Vec<Action>, .. }: Default, hash_state}` (part 5.1); `view_for::{view_for, HIDDEN_ID}`
  (part 5.2); `reduce`, `begin_game`, `legal_actions`, `seat_to_act -> Option<PlayerId>` (SURFACE §6.1).
- `catalog::registered_catalog()`, `scripts::registered_scripts()` (both `.clone()`d to an owned map);
  testkit `register_catalog(CardDefs)`, `register_scripts(IndexMap<String, CardScripts>)` (SURFACE §8).
- Effects, each argument built from TS's literal with `json_as(json!(…))`: `effects::{damage, shuffle_into,
  discover_from_catalog (its `query` as the plain value), add_to_hand, chosen_options(&EffectContext) ->
  Vec<String>}`.

## Decisions
- **The sink.** TS's `sinkFor(state)` handed back a sink holding the state; an `EngineSink` borrows it. Each
  file that needs one keeps a private `Sink { events, rng }` (the rng `Rng::new(&state.seed,
  state.rng_cursor)`, as `sinkFor` built it) that lends itself and the state to one engine call at a time, or
  builds an `EngineSink` in a block; no file writes the cursor back, as no TS test here did. Where TS created
  the sink before a `newInstance` or a `moveToZone`, the sink is made after it (neither draws from the rng).
- **Live objects.** TS read and wrote cards through the objects its helpers returned. Here the helpers'
  copies are read back by id (`find_instance`) and written through `find_instance_mut`; an engine call that
  took the live object gets the copy (`&CardInstance`, or `&mut CardInstance` where the owner's notes say the
  call updates it). `mana.test.ts`'s `handCard` answers the id.
- **Matchers.** Events, views and `turnLog.playedByType` are compared as JSON (`serde_json::to_value`), so
  TS's literals port key for key and an absent optional is TS's `undefined`; `toMatchObject` is a local
  `matches_object`; `toBe` on a state (identity) is value equality; `toMatch(/text/)` is `contains` (every
  pattern here is a literal); `whyChoicesRefused`'s `null` is `Ok(())` read as `None`.
- **Names.** SURFACE §7.3: every `R<n>` of a title leads, in title order (`r703_r90_…`); `§x.y` in a `mod`
  name is `sx_y`, in a test name dropped (kept in the `///` title), except the one test whose title starts
  with `§9.1` (`s9_1_…`); `#N` is `cN`, `Classic+` is `classic_plus`; apostrophes dropped, camelCase split.
- **Fixture defs local to a file** (`playChoices*`'s `def`/`unit`, overflow's secret trap) are JSON literals
  through `json_as::<CardDef>`, TS's spread as a shallow merge, TS's `nextIndex` counter written out as the
  index each def took (1101–1120, 1451–1458).
- **Nonces.** TS's module counters are `static AtomicU32`s (unique across parallel tests; not banned by
  clippy.toml).
- `play-pipeline-b-replay`: the policy keeps TS's short-circuit draw order (`chance` only when there are
  other actions and an end turn); the JSON round trip at each step is `serde_json::to_value` then
  `from_value::<GameState>`; TS's `TEST_TIMEOUT_MS` has no Rust counterpart (no timeout).
- `playCounts`' `play` helper adds the card to a clone of the state it is handed and returns the state
  after the play; TS added it to the caller's object, which every caller then replaced, so the results read
  the same.
