# Slice: part 26, chunk 7 of 7 (engine tests 3: view, turn, setup, combat), #414 under #306
BUILDS-RUN: 0

## FILES
All three ported whole (every `describe` as a `mod`, every `it` as a `#[test]`, in TS order, header
and rule/ruling comments kept); `#[test]` counts equal the TS `it` counts file by file (46 in all):
`crates/engine/tests/rules/{view_for (21), windfury (6), zones (19)}.rs`. Notes: this file,
`part-26-7.assumptions`, `spec-gaps-part-26-7.md`. Nothing outside these paths was written; no rebase
conflicted.

## GAPS
One assertion is written against a name SURFACE does not fix (also in `spec-gaps-part-26-7.md`):
`viewFor.test.ts` l.512 calls `viewFor(state, "p1", 75_000)` (TS's optional `clockMs`, R79), while
SURFACE §6.1 fixes a two-argument `view_for`. Ported as
`view_for_with_clock(&GameState, PlayerId, Option<i32>) -> PlayerView` (the name part 5.2's notes give
it); l.513's two-argument call stays `view_for(..).clock_ms == None`.

Names called that other parts provide (the signature each call assumes; part 31 reconciles):

**Fixtures, part 24** (`crate::rules::fixtures::<x>`):
- `harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`; `put(&mut GameState,
  def_id: &str, at: ZoneSlot, options /* { radiant? } */) -> CardInstance`, always passed
  `Default::default()`; `slot(PlayerId, Row, i32) -> ZoneSlot`; `in_hand(&mut GameState, &str, PlayerId,
  Option<i32>) -> Vec<CardInstance>`; `set_library(&mut GameState, PlayerId, &[String]) ->
  Vec<CardInstance>`. (`sinkFor` is not called: see Decisions.)
- `combat`: card fns `plain()`, `windfurier()`, `deft_duelist()` returning `CardDef`.
- `catalog`: `spell_def(i32, Value) -> CardDef`, `unit_def(i32, Value) -> CardDef`, `token_def(&str,
  Option<Vec<Tag>>) -> CardDef`, `vanilla_catalog(Option<i32>, Option<i32>) -> CardDefs`,
  `vanilla_deck(Option<i32>, Option<i32>) -> Vec<String>`.

**Engine** (through `jackioh_engine::testkit::*`):
- `view_for` (part 5): `view_for`, `view_for_with_clock` (above), `HIDDEN_ID: &str`.
- `catalog` (part 2): `def_of(Option<&GameState>, &str) -> &CardDef` (read for `.type_`),
  `registered_catalog() -> &'static CardDefs`; the testkit's one-argument `register_catalog(CardDefs)`.
- `prompts` (part 3): `open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>`,
  `OpenPromptArgs { player, kind: PromptKind, aim: Option<TargetAim>, prompt: String, options:
  Vec<PromptOption>, min: Option<i32>, max: Option<i32>, budget: Option<i32>, owner: Option<PlayerId>,
  resume: Resume }` built as a full struct literal.
- `resolve` (part 3): `make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) ->
  EffectContext`; `HookOptions { controller: Option<PlayerId>, .. }: Default`.
- `modifiers` (part 3): `add_modifier(&mut EngineSink, PlayerId, ModifierExpiry, ModifierKind) ->
  PlayerModifier` (part 3.2's split of TS's `DistributiveOmit<PlayerModifier, "id">`).
- `mana` (part 4): `effective_cost(&GameState, &CardInstance, options: Default) -> i32`.
- `effects::steal::steal(<Deserialize from { instanceId }>) -> Effect` (built with `json_as`).
- `combat` (part 3): `attacks_per_turn(&GameState, &CardInstance) -> i32`, `has_exertion(&GameState,
  &CardInstance, ExertionKind) -> bool` with `ExertionKind::{Attack, Switch}`,
  `switch_position(&mut EngineSink, &CardInstance, SwitchPositionOptions { spend_exertion:
  Option<bool>, to: Option<Position> }) -> Result<(), EngineError>` (`Debug` on the error).
- `reduce` (part 5): `reduce`, `begin_game`, `legal_actions`, `ReduceResult` (SURFACE §6.1).
- `zones` (part 2): `ZoneSlot` (= `ZoneRef`, built as `ZoneSlot { player, row, lane }`, passed by
  value, `Copy + PartialEq + Debug`); `adjacent(ZoneSlot) -> Vec<ZoneSlot>`; `ring_order(Row, PlayerId)
  -> Vec<ZoneSlot>`; `ring_neighbor(ZoneSlot, RotationDirection, PlayerId) -> ZoneSlot`;
  `first_free_zone(&GameState, PlayerId, Row) -> Option<ZoneSlot>`; `open_zones`, `fill_board_zones`
  `-> Vec<ZoneSlot>`; `slots_of(PlayerId, Row) -> Vec<ZoneSlot>`; `is_open`, `takes_move(&GameState,
  ZoneSlot) -> bool`; `card_at(&GameState, ZoneSlot) -> Option<&CardInstance>`; `lock_zone`,
  `reserve_zone(&mut GameState, ZoneSlot)`; `place_on_field(&mut GameState, &mut CardInstance,
  ZoneSlot, PlaceOnFieldOptions { stack: Option<bool> }: Default) -> bool`; `remove_from_field(&mut
  GameState, &CardInstance, Default) -> bool`; `active_units_of`, `dormant_units_of(&GameState,
  PlayerId) -> Vec<&CardInstance>`; `move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone,
  MoveToZoneOptions) -> MoveResult` with `OffFieldZone::{Hand, Library, Graveyard, Exile}`,
  `MoveResult::{Moved, Vanished}` (`PartialEq + Debug`), `MoveToZoneOptions { position:
  Option<LibraryPosition>, keep_state }: Default`, `LibraryPosition::At(i32)`.

## Decisions
- **Sinks.** TS's bare `sinkFor(state)` (openPrompt, addModifier, switchPosition) is a local
  `with_sink(state, |sink| …)` built from the frozen `Rng::new` and `EngineSink::new`, so these files
  do not depend on how the harness's `sink_for` owns its events and rng. As in TS, it does not write
  the cursor back; viewFor's `run()` does (TS wrote `state.rngCursor = sink.rng.cursor`).
- **Live objects.** A test holds an owned copy for its id and reads the card back with
  `find_instance` (`live`), writing with `find_instance_mut` (`live_mut`, windfury's `unit_at_mut`).
  Every assertion TS made on a live object is made on the card as it stands in the state.
- **Expectations.** `toEqual` on views, events, prompts and results compares serde JSON with `json!`
  (absent `Option`s are absent keys, as TS's `undefined`); `toMatchObject` is a local
  `matches_object` (subset on objects, element-wise at the same length on arrays);
  `not.toHaveProperty` and `Object.keys` read the serialised object's keys; `JSON.stringify(view)` is
  `serde_json::to_string`. Typed comparisons where the type is frozen (`HandView`, `ManaView`,
  `ModifierView`, `Exertion`, `AttackHealth`, `Counters`, `Zone`). `toMatch(/already acted/)` is
  `contains` on `ReduceResult.error`, failing when there is none.
- **Literals.** Events and `GameResult` are `json_as(json!(TS literal))`; card defs are built from the
  TS literal with a local shallow `spread` (TS's `...extra`) and `json_as::<CardDef>`; actions are the
  frozen `ActionBody` variants with `ActionInput::with_nonce`. TS's anonymous option bags are passed as
  the owners' named structs (`PlaceOnFieldOptions`, `MoveToZoneOptions`, `SwitchPositionOptions`,
  `HookOptions`) or `Default::default()` when TS omitted them. viewFor's `install` splits TS's
  modifier literal into `ModifierExpiry` and `ModifierKind` with `json_as`.
- **Catalog registration.** zones' `createGame({ …, catalog })` registered the catalog in TS; part 1's
  `create_game` only validates against it, so `game()` calls the testkit's `register_catalog` first.
- **Module `let`s.** viewFor's `nextIndex` is a counter inside `defs()`, which builds every definition
  in TS's module order (so each keeps TS's index); windfury's `nonce` is a `static AtomicU32`.
- **Local helpers kept to their callers.** viewFor's `game(seed = "view-test")` always takes a seed (no
  caller omits it); `must`/`at` are `expect`/indexing; zones' `put(…, stack = false)` takes a plain
  `bool` (every caller passes one). `ringNeighbor`'s `"left" | "right"` is `wire::RotationDirection`,
  the same literal set.
- **Names.** An `R<n>` anywhere in a title is a leading `r<n>_` token, several in title order (`… (R6)`
  at the end of an R636 title gives `r636_r6_…`; `(R13)` after a `§3.2` title gives `r13_section_3_2_…`);
  a leading `§x.y` is `section_x_y_…`, one inside a describe title `…_x_y`; punctuation dropped.
- **Exposure.** Read: the TS tests and fixtures, TS sources' signatures, SURFACE.md, the part brief,
  README §3, part 1's frozen Rust (`state`, `script`, `config`, `rng`, `prelude`, `lib`, `wire/*`,
  `testkit/mod.rs`, the test `mod.rs` files, the engine manifest) and other parts' notes files for the
  owners' signatures. No other Rust under `crates/` was opened.
