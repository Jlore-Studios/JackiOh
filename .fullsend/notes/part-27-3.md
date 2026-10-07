# Slice: part 27 (engine tests 4), chunk 3 of 8 (#415 under #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All three were empty placeholders on `staging`; each is a whole port (every `describe` a `mod`, every
`it` a `#[test]`, in TS order, the header and every rule or ruling comment kept). `#[test]` counts
equal the TS `it` counts, `mod` counts the `describe` counts:
- `crates/cards/tests/cross/hidden_information.rs` ← `packages/cards/test/hidden-information.test.ts`
  (37 tests, 24 mods).
- `crates/cards/tests/cross/lasting_effects.rs` ← `packages/cards/test/lasting-effects.test.ts` (6, 3).
- `crates/cards/tests/cross/my_pawn.rs` ← `packages/cards/test/my-pawn.test.ts` (9, 6).
Notes: this file, `part-27-3.assumptions`, `spec-gaps-part-27-3.md` (no gaps). Nothing outside these
paths was written.

## GAPS
No test was left unported (`spec-gaps-part-27-3.md` lists none). Names called that other parts provide
(the shape each call assumes; part 31 reconciles):

**Testkit** (part 5, `jackioh_engine::testkit::*`): `scenario(Value) -> Scenario`; steps `play(card,
Value)`, `attack(card, card_or_"hero")`, `answer(Value)`, `end_turn()`, `start_turn()` (each `&mut self
-> &mut Scenario`); reads `state()`, `state_mut()`, `events()`, `last_events()`, `view(seat) ->
PlayerView`, `unit`/`backrow(seat, i32) -> Option<CardInstance>`, `hand(seat) -> Vec<CardInstance>`,
`pile(seat, "library" | "exile") -> Vec<CardInstance>`, `card(ref) -> &CardInstance`; assertions
`expect_in_zone(ref, zone)`, `expect_health(seat, i32)`. A card ref is a def id `&str`, a
`CardInstance` or a `&CardInstance`; a seat is `"p1"`/`"p2"` or a `PlayerId`.
`scripts_override() -> Option<&'static IndexMap<String, CardScripts>>` and the one-argument
`register_scripts(IndexMap<String, CardScripts>)` (both named in part 5's notes).

**Engine** (through the testkit's `pub use crate::*` unless a path is given):
- `jackioh_cards::register_all()`, called first in every test (part 5's note: the engine's testkit cannot
  name the cards crate).
- `scripts::registered_scripts()` (owned or `&'static`; the test `.clone()`s it).
- `catalog::def_of(Option<&GameState>, &str)` (part 2's shape) read for `.type_`.
- `mana::effective_cost(&GameState, &CardInstance, CostOptions)` with `Default::default()`.
- `setup::mulligan_owed(&GameState) -> Vec<PlayerId>`; `reduce::{begin_game, reduce}` and
  `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`, `view_for::view_for`, `view_for::HIDDEN_ID:
  &str`, `state::{create_game, new_instance, find_instance_mut}`, `rng::create_rng` (all SURFACE §6.1 /
  part 1).
- `zones::{ZoneSlot { player, row, lane }, place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot,
  PlaceOnFieldOptions) -> bool, PlaceOnFieldOptions: Default}` (the majority shape in the notes).
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs { ingredients:
  Vec<CardInstance>, target: Option<CardInstance>, to_hand: Option<PlayerId>, .. }: Default}` (part 8).
- `subsystems::hero_power::power_of(&CardInstance) -> Option<HeroPower or &HeroPower>`, read for
  `.name` through `to_string()` (a string union's `Display`, or a `&str`).
- `effects::{choose_mode, destroy, remember}`, each taking its argument struct, built with
  `json_as(json!(…))` from TS's literal (`{ options, step, prompt }`, `{ target: { of: "instance",
  instanceId } }`, `{ key, value }`), so each must derive `Deserialize` (SURFACE §6.6).
- Script plumbing as part 1 froze it: `TriggerDef::new(..).with_when(..)`, `hook`, `Script { triggers,
  resume, death, start_of_game, ..Script::default() }`, `CardScripts { base, radiant }`,
  `EffectContext::live_self()`, `EngineSink::new`.
- The view's JSON must serialise as TS's: a face-down backrow zone `{ "faceDown": true, "cost": 1 }` with
  nothing else when it has no plague, pile or marks; a hand card's `attack`/`health`/`cost`/`power` keys;
  no `hiddenFrom`, `arrivedDuring` or `formerId` key in a view that strips them (R177, R119, R227).

## Decisions
- **Card refs and live objects.** TS held live `CardInstance`s; a test here keeps an owned copy (`.clone()`
  of `card()`, or the copies `unit`/`backrow`/`hand` hand back) for its id and reads the card again
  through `s.card(&copy)` after every step. TS's one write through a live object (`sniper.faceUp = false`
  in my-pawn R212) is `find_instance_mut(s.state_mut(), &id).face_up = Some(false)`.
- **`registerScripts({ ...registeredScripts(), [id]: … })`** is a local `registered_scripts_now()` (the
  testkit's thread override when one is set, else the production registry) plus one entry, then the
  testkit's `register_scripts`. TS's `registeredScripts()` returns the registry as last set, and my-pawn's
  R44 test registers two fixtures in a row, so reading only the production registry would drop the
  first: the helper keeps TS's meaning whichever registry `registered_scripts()` reads.
- **Fixture defs** (`fixture`, `fixtureDef`) are `json_as::<CardDef>(json!({…}))` of TS's literal.
- **Sinks** (`{ state: g.state, events: [], rng: createRng(seed, cursor) }`): a local `Rng` from
  `create_rng(&seed, cursor)`, a local event list, and `EngineSink::new(g.state_mut(), …)`; as in TS the
  rng's cursor is not written back to the state.
- **Events.** `eventsOf` is `events_of(&PlayerView, GameEventType) -> Vec<GameEvent>`; fields are read by
  matching the frozen `GameEvent` variants. `toEqual` on events, views and backrow zones compares JSON
  (`serde_json::to_value`, absent `Option`s absent, as TS's `undefined`) or `PartialEq` on the view;
  `JSON.stringify(x)` is `serde_json::to_string(x)`; `named(event)` reads the event's JSON keys ending in
  `Id`/`Ids` as TS's `Object.entries` did. `findIndex` keeps TS's -1 (`i64`).
- **TS `{ order, events }` and `{ s, oldId, newId }`** are tuples (the `events` half is never read, and
  a struct field never read would warn under CI's `clippy -D warnings`).
- **TS's `string | { def; radiant }` argument** (`immutableLibraryGame`) is a `serde_json::Value`.
- **Module `let setupNonce`** is a `static AtomicU32`; `gameWithHidden`'s local nonce a `u32` handed to a
  local `act`.
- **Sets** (`new Set(…)`) are `IndexSet`s; `JSON.stringify` of their members for the messages.
- **Regex matches**: `not.toMatch(/Pawn|Sheepish|core-0/)` is a hand check over the three alternatives;
  `toMatch(/core-096\+core-096$/)` is `ends_with`.
- **Names**: a title's R-ids, in title order, lead the name (`r152_r44_…`); a `§x.y` leading a title
  without one is `section_x_y_…` (with one, after the R tokens); `#NN` is `NN`; the trailing
  parenthetical of citations is dropped (its R-ids are already in the name); punctuation is dropped. Mods
  follow the same rule from the `describe` title.
- **Constants** keep TS's names and places (TS's mid-file consts stay mid-file; a `describe`'s consts are
  the `mod`'s).
