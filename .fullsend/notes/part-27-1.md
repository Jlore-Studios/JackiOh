# Slice: part 27 (engine tests 4: play pipeline and cross-card rules), chunk 1 of 8 (#415 under #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All six were empty placeholders on `staging`; each is now the whole TS file, every `describe` a `mod`
and every `it` a `#[test]`, in TS order, with the header comment and every comment that states a rule
or cites a ruling kept. `#[test]` counts per file (TS `it`s, plus the iterations of a `for` that
generates `it`s):

- `crates/cards/tests/cross/after_resolution.rs` ← `after-resolution.test.ts` (12)
- `crates/cards/tests/cross/combat_windows.rs` ← `combat-windows.test.ts` (22)
- `crates/cards/tests/cross/condition_active.rs` ← `condition-active.test.ts` (66: 62 `it`s, two of
  them inside a two-face `for` in each of two describes, so 4 more)
- `crates/cards/tests/cross/control_change_carry.rs` ← `control-change-carry.test.ts` (2)
- `crates/cards/tests/cross/control_change.rs` ← `control-change.test.ts` (20)
- `crates/cards/tests/cross/costs_and_mana.rs` ← `costs-and-mana.test.ts` (7)

Notes: this file, `part-27-1.assumptions`, `spec-gaps-part-27-1.md` (no gaps). Nothing outside these
paths was written; no rebase conflicted. No `todo!`, `unimplemented!`, `#[ignore]` or `// TODO`.

## GAPS
No test was left unported (`spec-gaps-part-27-1.md` lists none).

Names these files call that other parts provide (TS name snake_cased at its TS module's Rust path,
SURFACE §4; the shape each call assumes):

**Testkit (part 5, `testkit::scenario`)** — as part 5.1's notes give it:
- `scenario(Value) -> Scenario`; steps `play(card, Value)`, `attack(card, card_or_"hero")`,
  `answer(Value)` (a bare string matched as TS's), `end_turn()`, `start_turn()`, `switch_position(card)`,
  each `-> &mut Scenario` (chained: `s.end_turn().end_turn()`, `s.play(..).expect_health(..)`).
- Reads: `state() -> &GameState`, `state_mut() -> &mut GameState`, `events()`/`last_events() ->
  &[GameEvent]`, `view(seat) -> PlayerView`, `unit(seat, i32)`/`backrow(seat, i32) -> Option<CardInstance>`,
  `hand(seat)`/`pile(seat, "library" | "graveyard" | "exile") -> Vec<CardInstance>`, `card(ref) ->
  &CardInstance`, `stats(ref) -> layers::UnitView` (its `keywords: Vec<Keyword>` read).
- Assertions: `expect_in_zone(ref, &str)`, `expect_stats(ref, Value)`, `expect_health(seat, i32)`,
  `expect_mana(seat, i32)`, `expect_refused_with(|s| s.attack(..), "text")` (closure
  `for<'a> FnOnce(&'a mut Scenario) -> &'a mut Scenario`).
- A card ref is `impl Into<CardRef>` taking `&str`, `String`, `&String` and `&CardInstance`; a seat is
  `"p1"`/`"p2"`. `combat_windows.rs` names the type `CardRef` in its own `card_mut` helper.
- `register_scripts(IndexMap<String, CardScripts>)` (the thread-local override, which `testkit/mod.rs`
  re-exports over the production one).

**Cards crate (part 1, present):** `jackioh_cards::register_all()` (every file calls it before
`scenario`, through a local `setup`; the engine's testkit cannot name the cards crate) and
`jackioh_cards::scripts_of() -> IndexMap<String, CardScripts>` (TS `CARDS`, condition-active's last test).

**Engine (parts 2–8):**
- `scripts::registered_scripts()` returning the registry **including the testkit override when one is
  set** (owned or `&`; `.clone()`d into an `IndexMap<String, CardScripts>`). TS's `registeredScripts()`
  returned whatever was registered last, and after-resolution and combat-windows register two fixture
  cards in a row, each spreading the registry the previous one left. If it reads only the production
  `OnceLock`, the second registration drops the first fixture.
- `mana::effective_cost(&GameState, &CardInstance, CostOptions)` with `CostOptions: Default` (TS's
  default parameter, passed as `Default::default()`).
- `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>` at the root (SURFACE §6.1, `reduce.rs`).
- `zones::place_on_field(&mut GameState, &mut CardInstance, ZoneRef, PlaceOnFieldOptions) -> bool` with
  `PlaceOnFieldOptions: Default` (part 2.1's shape: the slot is `impl Into<ZoneSlot>`, `ZoneSlot =
  ZoneRef`). The placed card is then found again by id with `find_instance_mut` and written there
  (`faceUp = false`, `position = "ATK"`).
- `params::step_param(&mut CardInstance, &str, i32)` (on the live hand card, via `find_instance_mut`).
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs { ingredients:
  Vec<CardInstance>, to_hand: Option<PlayerId>, .. }: Default}`.
- Effects (parts 6–7), each data verb taking one struct that deserialises from TS's literal
  (`json_as(json!(…))`): `effects::{destroy, destroy_all, damage, steal, draw, summon, choose_mode}`;
  `effects::swap_board()` with no argument (TS `swapBoard()`).
- Frozen and used as part 1 wrote them: `new_instance`, `find_instance`, `find_instance_mut`,
  `EngineSink::new`, `Rng::new`, `EffectContext` (`ctx.state`, `ctx.controller`, `ctx.data`), `hook`,
  `TriggerDef::new(..).with_when(..)`, `Script` (`cry`, `triggers`, `resume`), `CardScripts`,
  `GameEvent::event_type`, `Zone::z`, `ModifierKind::EchoNextSpell`, `HandView`, `ManaView`, `CardView`,
  `Exertion`, `AttackHealth`, `Keyword`, `Position`, `PlayerId`, `Row`, `ZoneRef`, `Selection`.

## Decisions
- **Registration.** TS's harness import ran `registerAll()`; each file has a private `setup(Value) ->
  Scenario` that calls `jackioh_cards::register_all()` then `scenario(…)` (part 5.1's note for card tests).
- **Fixture cards** (after-resolution, combat-windows): the def is TS's object literal through
  `json_as::<CardDef>(json!(…))`, inserted into `state_mut().transient_defs`; the script is a `Script`
  literal whose triggers are `TriggerDef::new(id, &[GameEventType::…], run).with_when(when)` matching the
  frozen `GameEvent` variants; `resume` is collected into the `IndexMap<&'static str, Hook>`. TS's script
  constants are zero-arg fns (`vaporize()`, `slay_on_play()`): a closure cannot be a `const`.
  `String(ctx.data.attackerId)` reads the JSON string, or the value's text, or `"undefined"`.
- **Live objects.** TS wrote through `s.card(X)` (`grantedKeywords`, `exertion.switched`,
  `divineShieldSpent`) and through a hand card (`stepParam`). The ports find the card by id in
  `s.state_mut()` (`card_mut` helper / `find_instance_mut`) and write there, between the same two steps.
  Every read after a step goes through `s.card(..)`/`s.unit(..)`, never a held copy.
- **The fusion sink** (condition-active R196): TS built `{ state, events: [], rng: createRng(seed,
  cursor) }` and dropped its events and cursor; the port builds `EngineSink::new` over a fresh
  `Rng::new(&state.seed, state.rng_cursor)` and drops both the same way (no cursor written back).
- **Event and view assertions.** Field reads match the frozen `GameEvent` variants (with `..`);
  `toEqual`/`toContainEqual` on an object literal compare `serde_json::to_value` with `json!(…)`
  (absent optionals are skipped, as TS's `undefined`); `toMatchObject` checks only the named fields.
  `glows` reads the serialised card for the `conditionActive` key exactly as TS's `"conditionActive" in
  card`, asserting `true` when present. `JSON.stringify(view)` is `serde_json::to_string`.
- **Refusals.** `expect(() => g.attack(..)).toThrow(/text/)` → `expect_refused_with(|g| g.attack(..),
  "text")`; every regex in these files is a literal.
- **Generated `it`s.** A TS `for` over faces that declares `it`s becomes one shared private fn and one
  `#[test]` per iteration, named after the iteration's title, in TS order.
- **Names.** Every ruling a title cites leads the test name (SURFACE §7.3: `it("R65 … (R48)")` →
  `r65_r48_…`), `§x.y` becomes `sx_y`, `#26` becomes `c26`, "C #22"/"C+ #50" become `classic_c22`/
  `classic_plus_c50` in `mod` names; `mod` names follow their `describe` title as written.
- **Defaults.** A TS default parameter is passed explicitly (`fixture_unit(.., AttackHealth { attack: 2,
  health: 2 })`, `growth(mine, theirs, false)`, `Default::default()` for an options object).
