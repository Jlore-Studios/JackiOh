# Slice: part 27, chunk 6 of 8 (engine tests 4: play pipeline and cross-card rules), #415 under #306
BUILDS-RUN: 0

## FILES
All six ported whole (every `describe` as a `mod`, every `it` as a `#[test]`, in TS order, the header
and every comment that states a rule or cites a ruling kept); 64 `#[test]`s in all:
`crates/cards/tests/cross/{setup_and_mulligan (9), stacks_and_reborn (8), tributes (7), trigger_stays
(13), turn_clock_and_legality (14), turn_stages (13)}.rs`. Notes: this file, `part-27-6.assumptions`,
`spec-gaps-part-27-6.md`. Nothing outside these paths was written; no rebase conflicted.

## GAPS
Tests not ported as written (also in `spec-gaps-part-27-6.md`): none dropped. turn-clock-and-legality
l.296 (lane 2.5) asserts its refusal at `Action`'s `Deserialize`, since `ZoneChoice.lane` is `i32`;
turn-clock-and-legality l.423's `expect.soft` is a hard `assert_eq!`.

Names called that other parts provide (the signature each call assumes; part 31 reconciles):

**Testkit, part 5** (`jackioh_engine::testkit`, SURFACE §8):
- `scenario(Value) -> Scenario`; reads `state() -> &GameState`, `events()`, `last_events()` (both used
  as `&[GameEvent]`), `unit(PlayerId, i32)` and `backrow(PlayerId, i32) -> Option<&CardInstance>`
  (`.cloned()`, `.map(|c| …)`), `hand(PlayerId)` (a list: `.iter()`, `.len()`, `.last()`),
  `pile(PlayerId, &str)`, `card(&str) -> &CardInstance`, `stats(&str) -> UnitView`,
  `view(PlayerId) -> PlayerView`; steps `play(&str, Value)`, `attack(&str, &str)` ("hero" or an id),
  `answer(Value)`, `end_turn()`, `start_turn()`; checks `expect_in_zone(&str, &str)`,
  `expect_stats(&str, Value)`, `expect_health(PlayerId, i32)`, `expect_mana(PlayerId, i32)`.
- **`state_mut() -> &mut GameState`**, which SURFACE §8's table does not list: TS tests write the live
  state (`g.state.transientDefs[id] = …`, `newInstance(g.state, …)`, `placeOnField(g.state, …)`, a
  card's `summonedTurn`/`faceUp`/`grantedKeywords`/`memory`). pools_and_randomness.rs (another part)
  calls it too.
- `register_scripts(IndexMap<String, CardScripts>)` (the thread-local override).

**Engine** (through `jackioh_engine::testkit::*` unless a path is given):
- `scripts::registered_scripts()` (the map, or `&'static` to it; the tests `.clone()` it).
- `create_game(&CreateGameArgs)`, `begin_game(&GameState) -> ReduceResult`, `reduce(&GameState,
  &Action) -> ReduceResult`, `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`,
  `view_for(&GameState, PlayerId) -> PlayerView`, `hash_state(&GameState) -> String` (SURFACE §6.1).
- `setup::mulligan_owed(&GameState) -> Vec<PlayerId>`, `setup::mulligan_prompt_for(&GameState,
  PlayerId) -> Option<&PendingChoice>` (an owned `Option<PendingChoice>` compiles too: only `.kind` and
  `.id.clone()` are read).
- `catalog::query(&CatalogQueryArgs) -> list of CardDef` (the argument is `json_as(json!({}))`, so the
  type's name does not matter).
- `query::was_played_this_turn(&GameState, PlayerId, &str)` — TS's `card: CardInstance | string`,
  passed the id.
- `play_choices::tribute_value_of(&GameState, &CardInstance) -> i32`;
  `mana::effective_cost(&GameState, &CardInstance, CostOptions) -> i32` with `CostOptions: Default`.
- `zones::ZoneSlot { player, row, lane: i32 }` by value; `zones::place_on_field(&mut GameState, &mut
  CardInstance, ZoneSlot, options: Default) -> bool` (other ports pass the card by value: part 31
  picks one); `zones::card_at(&GameState, ZoneSlot) -> Option<&CardInstance>`;
  `zones::active_units_of(&GameState, PlayerId)` (a list whose items have `.id`).
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs {
  ingredients: Vec<CardInstance>, target: Option<CardInstance>, .. }: Default}`;
  `subsystems::POWER_KEY: &str`; `prompts::RESUME_HOOK: &str`.
- `layers::UnitView { attack, max_health, health, .. }` (`stats()`'s answer).

**Effects** (parts 6–7), every argument built with `json_as(json!(…))` from TS's literal (SURFACE
§6.6): `remember`, `choose_mode`, `cast`, `bounce`, `bounce_all`, `delay`, `exile`, `exile_hand`,
`exile_matching`, `damage`, `sacrifice`, `draw`, `steal`, `summon`, `destroy`, `destroy_all`, `buff`,
`buff_all_units`, `add_player_modifier` (TS's `mod` key as the JSON key). The one closure-carrying one by
struct literal: `for_each_card(ForEachCardArgs { cards: Arc<dyn Fn(&EffectContext) -> Vec<String>>,
each: Arc<dyn Fn(&str) -> Effect> })` — the `cards` closure is annotated `|c: &EffectContext<'_>|`, as
`crates/engine/tests/rules/effects_each.rs` does; part 24-1's notes say `&mut EffectContext`. Part 31
picks one.

**Cards, part 1**: `jackioh_cards::{register_all, CATALOG}`.

## Decisions
- **Registration.** Importing TS's harness ran `registerAll()`; every case here calls
  `jackioh_cards::register_all()` first (a local `game(setup)` wraps `scenario`). A fixture card is a
  transient def written into the state plus `register_scripts({ ...registered_scripts(), [id]: … })`
  through the thread-local override, exactly TS's shape. Fixture defs are built from TS's literal with
  `json_as::<CardDef>(json!({ … }))`.
- **Live objects.** A test holds an owned `CardInstance` copy for its id; every write TS made through a
  live object (`summonedTurn = 0`, `position = "ATK"`, `faceUp = false`, a pushed Reborn, a Heroic
  Power's `memory[POWER_KEY]`, a library `unshift`) goes through `find_instance_mut(s.state_mut(), id)`
  or the state's own vectors; every read after a step goes back through the harness or the state.
- **Card references** are `&str`: a catalog id, an index, a name (`"4-mana 7/7"`, kept as TS wrote it)
  or an instance's id where TS passed the `CardInstance` (the harness resolves an instance by its id
  first, so the two are the same reference).
- **Hooks.** TS's `ctx.self` reads, where the card's current fields matter (the asking clause's
  `memory.asked`, #23's return flag), are `ctx.live_self()`; `ctx.self?.id` for identity is
  `ctx.self_`. `ctx.event` is the trigger's second argument; a TS `when` is `.with_when`. TS's
  `sourceId !== self?.id` keeps its null semantics (only a source equal to this card is its own).
- **Module `let nonce`** counters are `static AtomicU32`s (part 26-1 did the same).
- **TS defaults**: `fixture(…, cost = 0)` and `fixture(…, stats = { 2/2 })` take a trailing `Option`;
  `unitFixture`'s anonymous options object is a local `UnitOpts` with `Default`; `perUnit: "byId" |
  "overTheBoard"` is a local enum, and the `for (const perUnit of …) it(…)` loop is one shared fn and
  two `#[test]`s, in TS's order.
- **Expectations.** `toEqual` on events, views, plays and lists is `assert_eq!` on the typed values (all
  derive `PartialEq`), an empty list as a typed `Vec::new()`; `toMatchObject` on one event is
  `matches!` on that variant's fields; `toMatch(/Tribute 3/)` is `contains`; `findIndex`'s `-1` is kept
  (`position` mapped to `i64`, `-1` when absent) so `toBeGreaterThan` reads as in TS; `JSON.stringify(x)
  .includes(…)` is `serde_json::to_string`; `"player" in event` reads the serialised event's key; TS's
  `new Map`/`new Set` are `IndexMap`/`IndexSet`.
- **Sinks.** `unitCarrying`'s `{ state: s.state, events: [], rng: createRng(seed, cursor) }` is an
  `EngineSink::new` over `s.state_mut()`, a local event list and a fresh `Rng`, whose cursor is not
  written back (TS did not either).
- **Names**: every `R<n>` in a title is a leading `r<n>_` token, in title order; a leading `§x.y` is
  `sx_y_`, an inline one `x_y`; punctuation dropped. Describe titles name the `mod`s the same way.
- **Exposure, recorded honestly**: one survey `grep` over `crates/*/tests/` also named
  `crates/cards/src` and printed about fifteen lines from card-script test modules there (local helper
  calls such as `run.hand(…)`, `b.view(…)`). Nothing else under `crates/*/src/` was opened beyond
  part 1's frozen files and `mod.rs`es; the tests do not lean on what it printed.
