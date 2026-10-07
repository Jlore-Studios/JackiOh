# Slice: part 5 (engine 4), chunk 1 of 4
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All five were empty placeholders on `staging`; all are full ports. No `todo!`, `unimplemented!` or `// TODO`.
- `crates/engine/src/replay.rs` ← `packages/engine/src/replay.ts`: `canonical`, `fnv1a32_utf16`,
  `hash_state` (SURFACE §5.2), `ReplayInput`/`ReplayResult`/`FoldError` with SURFACE §6.1's names as
  aliases (`FoldArgs = ReplayInput`, `FoldResult = ReplayResult`), `fold(&ReplayInput) -> ReplayResult`.
  Two unit tests of its own (canonical text, FNV over UTF-16 units).
- `crates/engine/src/counter_warning.rs` ← `packages/engine/src/counterWarning.ts`:
  `countered_hand_cards(&GameState, PlayerId) -> IndexSet<String>` (what part 5.2's `view_for.rs` calls).
- `crates/engine/tests/rules/validator_drafts.rs` ← `packages/validator/test/drafts.test.ts` (every `it`,
  one `mod` per `describe`, R-ids leading the names).
- `crates/cards/tests/cross/game_summary.rs` ← `packages/cards/test/game-summary.test.ts`.
- `crates/engine/src/testkit/scenario.rs` ← `packages/cards/test/_harness.ts`, with
  `packages/cards/test/_harness.test.ts` as its `#[cfg(test)] mod tests` (every `it`).

## SURFACE (§8, the testkit API other parts call)
- `scenario(opts: Value) -> Scenario`; `Scenario::new(&ScenarioOptions)`; the option types are pub and
  `Deserialize` (`ScenarioOptions`, `SideSetup`, `PileSetup`/`PileEntry`, `FieldSetup`/`FieldEntry`,
  `BackrowSetup`, `DefRef`, `CostSetup`, `PlayOptions`, `ActivateOptions`).
- Steps (`&mut self -> &mut Scenario`): `play(card, Value)`, `attack(card, card_or_"hero")`,
  `answer(Value)`, `end_turn()`, `start_turn()`, `switch_position(card)`, `activate(card, Value)`.
- Reads (`&self`): `state() -> &GameState`, `state_mut() -> &mut GameState` (TS tests write
  `s.state.…`; part 5.2's `invariants.rs` uses it), `events()`/`last_events() -> &[GameEvent]`,
  `view(seat) -> PlayerView`, `unit(seat, lane: i32)`/`backrow(seat, lane) -> Option<CardInstance>`
  (copies), `hand(seat)`/`pile(seat, "library") -> Vec<CardInstance>` (copies), `card(ref) -> &CardInstance`,
  `stats(ref) -> layers::UnitView`.
- Assertions (`&mut self -> &mut Scenario`): `expect_in_zone(ref, "field")`, `expect_stats(ref, Value)`,
  `expect_events(Value)` (an array of type strings, or one string), `expect_health(seat, i32)`,
  `expect_mana(seat, i32)`, `expect_refused(|s| s.play(…))`, `expect_refused_with(|s| …, "text")`.
  The closure is `for<'a> FnOnce(&'a mut Scenario) -> &'a mut Scenario`, so a read inside one is
  written `|s| { s.card("x"); s }`.
- A card ref is `impl Into<CardRef>`: `&str`, `String`, `&String`, `CardInstance`, `&CardInstance`. A seat
  is `impl SeatRef`: `PlayerId`, `"p1"`/`"p2"`, `Option<PlayerId>` or `()` (the active player, TS's
  omitted argument).
- Added beside SURFACE: `expect_throw(|| …)` and `expect_throw_with(|| …, "text")` for a refusal that
  is not a step (`expect(() => scenario({…})).toThrow(/…/)`).
- `DEFAULT_SEED`, `DEFAULT_TURN` (TS's constants, kept in the testkit: they are not rules numbers).
- Registries: `register_catalog(CardDefs)` (version "test"), `register_catalog_as(CardDefs, &str)`,
  `register_scripts(IndexMap<String, CardScripts>)`, `clear_overrides()`, and the readers
  `catalog_override() -> Option<&'static CardDefs>` (the name part 2.2's `catalog.rs` calls),
  `catalog_version_override() -> Option<&'static str>`,
  `scripts_override() -> Option<&'static IndexMap<String, CardScripts>>`.

## GAPS
### Called in other modules (TS name snake_cased at its TS module's path; the shape I assumed)
- `reduce::{reduce(&GameState, &Action), begin_game(&GameState)} -> ReduceResult { state, events, error: Option<String> }`,
  `reduce::seat_to_act(&GameState) -> Option<PlayerId>` (part 5.2; matches its notes).
- `play_choices::offered_play_costs(&GameState, PlayerId, &CardInstance) -> Vec<i32>` (part 4).
- `preview::backrow_is_public(&GameState, &CardInstance, PlayerId) -> bool` (part 2.2).
- `triggers::{TriggerHolder { card: CardInstance, controller, zone: TriggerZone, is_trap, script, .. },
  TriggerZone::{Field, Backrow} (PartialEq), cards_in_trigger_order(&GameState) -> Vec<TriggerHolder>,
  triggers_on_event(&TriggerHolder, GameEventType) -> <anything with is_empty()>}` (part 3.3).
  `holder.script.would_counter: Option<WouldCounterHook>` (script.rs, frozen).
- `game_summary::summarize_game(&FoldArgs) -> Option<GameSummary>` and `wire::stats::{GameSummary,
  SeatSummary}` as part 5.4 wrote them; `subsystems::choose_action(&GameState, PlayerId, &mut Rng) ->
  Option<ActionBody>` (part 8).
- `validator` (part 5's validator chunk): `TRIO_DECKS` (compared with `3`),
  `check_deck_draft(&DeckDraftInput)`, `check_trio_draft(&TrioDraftInput)`,
  `check_import_room(&ImportRoomInput)`, `validate_deck(&DeckInput)`, `validate_loadout(&LoadoutInput)`,
  `validate_trio(&LoadoutInput)`, `trio_conflicts(&[T])` where `T: Deserialize` (a `LoadoutDeck` or a
  `{cards}` struct), `normalize_name(&str) -> String`. Every input but `DeckDraftInput` is built with
  `json_as` from TS's literal, so it must derive `Deserialize` (it must anyway, for the WASM
  `validator(call, json)` binding); every answer is compared as JSON, so it must `Serialize` exactly as
  TS's object (`{ ok: true }`, `{ ok: false, errors }`, `{ rule, message, deck?, cardId? }`,
  `{ cardId, decks }`, `{ ok: false, decksShort, triosShort, message }`).
  `DeckDraftInput { name: String, cards: Vec<String>, is_deckable: Arc<dyn Fn(&str) -> bool + Send + Sync>,
  name_max_length: usize, portrait: Option<String>, is_portrait: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>> }`
  (TS's `portrait?: string | null`: null and absent check alike, so one `Option`). Built in one helper,
  `draft_input`, so a different shape is one edit.
- `tests/rules/fixtures/validator_loadouts.rs` (part 5.2): `ARCHIVIST`, `CEASELESS_VOID`, `HIT_JOB`,
  `NOT_IN_CATALOG`, `SHEEP_TOKEN: &str`, `POOL_IDS: LazyLock<Vec<String>>`, `catalog()`/`collection()`
  (both `Serialize`), `legal_decks()` (indexable by deck, each indexable and sliceable to its ids).
- `wire::emotes::{is_portrait_id(&Value) -> bool, PORTRAIT_IDS: &[PortraitId] (Display)}` (part 5.2).
- Harness: `catalog::registered_catalog()`; `setup::create_in_hand(&mut EngineSink, PlayerId, &str)`
  (return ignored; the card is read back as the hand's last); `zones::{ZoneSlot { player, row, lane: i32 },
  place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot, PlaceOnFieldOptions { stack: Option<bool> })
  -> bool, move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, MoveOptions { position:
  Option<LibraryPosition>, .. }: Default) -> MoveResult, LibraryPosition::Bottom, MoveResult::{Moved,
  Vanished} (PartialEq), OffFieldZone::{Hand, Library, Graveyard, Exile}, remove_from_any_zone(&mut
  GameState, &CardInstance), beneath_at(&GameState, ZoneSlot)}` (part 2.1; these follow the majority
  of the other parts' notes); after a placement or a move the card is found again by id
  (`find_instance_mut`), so it does not matter whether the callee takes it by value or by reference;
  `mana::{refresh_mana(&mut PlayerState), effective_cost(&GameState, &CardInstance, Default::default())}`;
  `state_check::state_check(&mut EngineSink)`; `turn::start_turn(&mut EngineSink, PlayerId)` (part 5.4's
  shape); `layers::unit_view(&GameState, &CardInstance) -> UnitView { attack, health, max_health, armor,
  keywords, position: Position }`; `view_for::view_for(&GameState, PlayerId)`.
- `scripts.rs` (part 2.1) must consult `testkit::scenario::scripts_override()` under
  `#[cfg(feature = "testkit")]`, as part 2.2's `catalog.rs` consults `catalog_override()`.
### Not ported
- `_harness.ts`'s `PLACEHOLDERS` and `actionOrEngine` (the fallbacks for `reduce`'s long-gone placeholder
  refusals, brief step 7): `attack` and `answer` are plain actions.
- TS's `{ timeout: 120_000 }` on game-summary's one test (cargo test has no per-test timeout).
### For other parts
- A card test (parts 9–16) calls `jackioh_cards::register_all()` before `scenario(…)`: the engine's testkit
  cannot name the cards crate, so the TS harness's import-time `registerAll()` has no Rust twin. Without
  it the filler deck refuses with "did jackioh_cards::register_all() run?".
- SURFACE §6.1 names `FoldArgs`/`FoldResult` but no part defined them; they are `replay.rs`'s (above).

## Decisions
- `hash_state`: `serde_json::to_value(state)`, remove top-level `applied` and `opening`, `canonical`, FNV.
  `canonical` sorts keys by UTF-16 code units (TS's `<`), writes strings with `serde_json::to_string`
  (the same escapes as `JSON.stringify`), integers as digits, and an integral float as an integer (JS
  prints `2`, serde `2.0`; the state holds none, but the golden hashes over views and events use it too).
- `fold` panics where TS's `createGame` threw (a deck the handicaps do not allow), as `create_game` does.
- `counter_warning`: TS's `ReadonlySet<string>` is an `IndexSet<String>` in hand order.
- `validator_drafts.rs`: inputs from TS's literals with `json_as`; answers compared as JSON
  (`serde_json::to_value`), so the test pins the wire shape, not the Rust struct. `expect(validateTrio)
  .toBe(validateLoadout)` is ported as "both answer the same JSON" on a legal and a short trio.
  `isDeckable` reads the fixture catalog as JSON.
- `game_summary.rs`: the TS quirks of `splice(indexOf(x), 1)` and `splice(lastIndexOf(x), 1)` with a
  missing entry (they remove the last element) are kept as written.
- `scenario.rs`, the override: three `thread_local!` `Cell<Option<&'static …>>`s (a leaked box per
  registration), not a `RefCell` — so not even the one `RefCell` SURFACE §8 allows is needed.
- `scenario.rs`, failures: internal helpers return `Result<_, String>` with TS's message text and the
  public methods `panic!` with it (TS threw); `expect_refused*`/`expect_throw*` catch the panic with
  `catch_unwind(AssertUnwindSafe(…))` and match the message with `contains` (every TS regex in these
  tests is a literal). The default panic hook still prints the caught message.
- `scenario.rs`, reads: `unit`/`backrow`/`hand`/`pile` return owned copies so the result can be passed
  into the next `&mut` step (TS returned snapshot objects too); `card()` returns `&CardInstance` (SURFACE).
- `scenario.rs`, private copies (fullsend rule 5): `catalog.defOf` (transient defs first), the filler deck's
  `query({})` (non-token, not Glitch, `query`'s set/index/id order), `zones.cardAt`. `PileName` and
  `ZoneName` are `&str` arguments, not exported enums (`ZoneName` would collide with the wire's).
- `scenario.rs`, `turn`: an `f64`, refused by TS's message unless a non-negative whole number.
- `scenario.rs`'s own tests run inside the engine crate, which cannot link `jackioh_cards` (it would be a
  second copy of the engine), so they install `crates/cards/catalog.json` (`include_str!`) through the
  override and run with no card scripts; no case plays a card whose script would act.
- `_harness.test.ts`'s `expect(s.state).not.toBe(before)` (object identity) is ported as "the state
  changed"; `toBe(s)` as `std::ptr::eq` on the returned `&mut Scenario`; "accepts a fixture `as const`"
  as a literal built ahead and passed in unchanged.
