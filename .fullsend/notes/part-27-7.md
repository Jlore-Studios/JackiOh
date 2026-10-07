# Slice: part 27 (engine tests 4: play pipeline and cross-card rules), chunk 7 of 8 (#415, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All eleven were empty placeholders on `staging`; each is now the whole TS file, every `describe` a
`mod` and every `it` a `#[test]`, in TS order, with the header comment and every comment that states a
rule or cites a ruling (`#[test]` counts equal the TS `it` counts file by file, 114 in all):
- `crates/cards/tests/cross/vanilla_and_positions.rs` ← `packages/cards/test/vanilla-and-positions.test.ts` (10)
- `crates/engine/tests/rules/announce.rs` ← `announce.test.ts` (21)
- `crates/engine/tests/rules/cost_rules.rs` ← `cost-rules.test.ts` (12)
- `crates/engine/tests/rules/counter_warning.rs` ← `counterWarning.test.ts` (4)
- `crates/engine/tests/rules/draw_complete.rs` ← `draw-complete.test.ts` (4)
- `crates/engine/tests/rules/draw_limit.rs` ← `draw-limit.test.ts` (15)
- `crates/engine/tests/rules/draw_pause.rs` ← `draw-pause.test.ts` (7)
- `crates/engine/tests/rules/draw.rs` ← `draw.test.ts` (14)
- `crates/engine/tests/rules/echo.rs` ← `echo.test.ts` (10)
- `crates/engine/tests/rules/graveyard_play.rs` ← `graveyard-play.test.ts` (14)
- `crates/engine/tests/rules/mana_before_play.rs` ← `mana-before-play.test.ts` (3)

Notes: this file, `part-27-7.assumptions`, `spec-gaps-part-27-7.md`. No `todo!`, `unimplemented!`,
`#[ignore]` or `// TODO`. Nothing outside these paths was written; no rebase conflicted.

## SURFACE
§4 (paths, names, types), §6.5/§6.6 (`EngineSink::new(state, events, rng)`, hooks over
`&mut EffectContext`, `Effect::new`, `(effect.apply)(&mut ctx)`, `TriggerDef::new(..).with_when(..)`,
`would_counter_hook`), §7.3 (test names), §8 (testkit: `scenario`, `json_as`, `json!`, the thread-local
`register_catalog`/`register_scripts`). Part 1's frozen types used as written: `GameState`,
`PlayerState`, `ManaState`, `DrawCount`, `CardInstance`, `Zone`, `ZoneName`, `Row`, `PlayerModifier`,
`ModifierKind::{EchoNextSpell, CostDiscount, CostRule, EnchantNextSpell}`, `ModifierExpiry`,
`Enchantment`, `GameEvent`, `GameEventType`, `Action`, `ActionBody`, `ActionType`, `Selection`,
`PromptKind`, `PromptOption`, `WorkItem`, `EchoItem`, `CardDef`, `CardScripts`, `Script`,
`StaticFlags`, `TriggerDef`, `Hook`, `PlayerView`, `HandView`, `CardView`, `ModifierView`, `Rng`,
`CreateGameOptions`.

## GAPS
Tests not ported: none (`spec-gaps-part-27-7.md` lists the assertions written against the nearest Rust
observable).

Names called that other parts provide (module path; the shape these files assume). Where an owner's
notes were on `staging` I followed them ("owner").

**Fixtures (part 24, `crate::rules::fixtures::…`):**
- `harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`; `setup_catalog()`;
  `put(&mut GameState, def_id: &str, at: ZoneSlot, options) -> CardInstance` (TS's `{ radiant? }`
  always passed, as `Default::default()` or `json_as(json!({ "radiant": true }))`, so a `Value` or a
  `Default + Deserialize` struct both compile; answers a copy); `slot(PlayerId, Row, i32) -> ZoneSlot`;
  `in_hand(&mut GameState, &str, PlayerId, count: i32) -> Vec<CardInstance>` (count always passed);
  `set_library(&mut GameState, PlayerId, &[String]) -> Vec<CardInstance>`;
  `events_of_type(&[GameEvent], GameEventType)` (a `Vec` of `&GameEvent` or `GameEvent`: only `.len()`,
  `.iter()` and `serde_json::to_value` of an element are used). `sinkFor` is not called (Decisions).
- `catalog::vanilla_deck(size: i32, from: i32) -> Vec<String>`.
- `scripts::{hinder, cn_virus, anti_oneshot, infinite_reserves}() -> CardDef`.
- `turn`: `turn_catalog(CardDefs) -> CardDefs`, `turn_scripts() -> IndexMap<String, CardScripts>` (TS
  `TURN_SCRIPTS`), `notes(&GameState) -> Vec<String>`, `LOG_LANE: i32`, and `fn() -> CardDef` for
  `log_card`, `palantir`, `anti_greed`, `draw_two`, `cast_spell`, `cast_unit`, `plain`, `taxman`.
- `play_pipeline_a`: `PA` as TS's constant object with snake_cased fields, each a `CardDef`, read as
  `PA.crier.id` (e.g. `pub static PA: LazyLock<Pa>`; part 27.6 assumed the same). Fields used: `crier`,
  `bolt`, `ping`, `counter_trap`, `counter_trap2`, `echo_bolt`, `chalice`, `field`, `exile_trap`,
  `refusal`, `shredder`, `steal_trap`, `palantir`, `watcher`, `hidden_trap`, `hidden_field_trap`,
  `q_bolt`, `q_palantir`. `register_play_a()`, `with_play_a(GameState) -> GameState`.
- `play_pipeline_b`: `pb_playing(&str) -> GameState`; `pb_act(&GameState, body) -> GameState` and
  `pb_reduce(&GameState, body) -> ReduceResult` (body built with `json_as(json!(TS literal))`, so
  `ActionInput` or `Value` both compile); `in_graveyard(&mut GameState, &str, PlayerId) -> CardInstance`;
  `only<T: Clone>(&[T]) -> T` (TS's "at least one", the first); `round_trip(&GameState) -> GameState`;
  `plays_of(&GameState, &str, PlayerId)` (a `Vec` of anything that serialises as TS's `{ type: "play",
  … }`: `ActionBody` or `PlayAction`); `QUEST_REWARD_KEY: &str`; `fn() -> CardDef` for `second_wind`,
  `plantation`, `quest_card`, `grave_unit`, `grave_spell`, `zero_spell`, `grave_trap`, `discover_spell`,
  `x_spell`, `cast_trap`, `forever`, `toe_cracker`, `lobbyist`, `monkey`, `trickster`, `tax`,
  `three_spell`, `five_unit`, `two_field`, `x_unit`, `embiggen_field`, `titan`.

**Engine (through `jackioh_engine::testkit::*` or the path given):**
- `reduce::{reduce(&GameState, &Action), begin_game(&GameState), legal_actions(&GameState, PlayerId) ->
  Vec<ActionBody>}`, `ReduceResult { state, events, error: Option<String> }` (SURFACE §6.1);
  `replay::{hash_state, fold(&FoldArgs) -> FoldResult { state, errors }}` — `FoldArgs` built with
  `json_as(json!({ seed, decks, log }))`, so it must derive `Deserialize` (part 5.1's `ReplayInput`).
- `catalog::registered_catalog()` and `scripts::registered_scripts()` (`.clone()`d, so owned or
  `&'static` both work); the testkit's `register_catalog(CardDefs)` and
  `register_scripts(IndexMap<String, CardScripts>)`.
- `state::{new_instance, find_instance, find_instance_mut, create_game}` (frozen).
- `draw` (part 4.2, owner): `draw(&mut EngineSink, PlayerId, i32) -> Vec<DrawOutcome>`, `draw_one(&mut
  EngineSink, PlayerId, Option<ChainLinkOrCount>) -> DrawOutcome` (TS's numeric link passed as
  `Some(n.into())`, `From<i32>`), `DrawOutcome::{Drawn, Cast, Fatigue, Limited}` (`PartialEq + Debug`),
  `shuffle_into_library(&mut EngineSink, &mut CardInstance, existing: bool, Option<&str>) ->
  ShuffleInOutcome::{Library, Dropped}` (checked with `matches!`), `draws_this_turn(&GameState,
  PlayerId) -> i32`, `draw_limit_of(&GameState, PlayerId) -> Option<i32>`, `draw_blocked(&mut
  EngineSink, PlayerId) -> bool` (a `&EngineSink` parameter also takes the call), `DRAW_CHAIN_WORK`,
  `DRAW_COUNT_WORK: &str`, `owed_draw_chain_of(&Resume) -> Option<OwedDrawChain { player, chain, owns }>`,
  `owed_draw_count_of(&Resume) -> Option<OwedDrawCount { player, count }>` (fields read by name).
- `ownership::draw_from_library_of(&mut EngineSink, PlayerId, PlayerId, LibraryEnd::Bottom) ->
  Option<DrawOutcome>` (part 2.2, owner).
- `counter_warning::countered_hand_cards(&GameState, PlayerId) -> IndexSet<String>` (part 5.1, owner).
- `announce::open_announces(&GameState) -> &[AnnounceRecord]` (part 2.1, owner).
- `zones::{card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>, active_units_of(&GameState,
  PlayerId), move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone::Exile, Default::default())}`
  (part 2.1, owner: a slot as `&slot(..)`, which its `impl Into<ZoneSlot>` takes).
- `mana` (part 4.1, owner): `effective_cost(&GameState, &CardInstance, CostOptions)` (passed
  `Default::default()`), `play_cost(&GameState, &CardInstance) -> i32`, `cost_now(&GameState,
  &CardInstance) -> i32`.
- `modifiers::add_modifier(&mut EngineSink, PlayerId, ModifierExpiry, ModifierKind)` (part 3.2, owner).
- `resolve` (part 3.2, owner): `make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) ->
  EffectContext`, `HookOptions { controller: Option<PlayerId>, .. }: Default`, `apply_effects(&[Effect],
  &mut EffectContext)`, `cast_card(&mut EngineSink, &CardInstance, CastOptions)` (options
  `Default::default()`).
- `triggers::settle(&mut EngineSink, SettleOptions)` (`Default::default()`).
- `prompts` (part 3.2): `open_prompt(&mut EngineSink, OpenPromptArgs { player, kind, aim, prompt,
  options: Vec<PromptOption>, min, max, budget, owner, resume }) -> Option<PendingChoice>` (all ten
  fields written out), `resume_self(&EffectContext, &str, data)` (data `Default::default()`, so an
  `IndexMap` or an `Option<IndexMap>` both compile).
- `work::owed_work(&GameState, Option<&str>) -> Vec<WorkItem>`; `play_steps::PLAY_WORK_KIND: &str`.
- `view_for::{view_for, HIDDEN_ID: &str}`.
- Effects (parts 6–7), data arguments built with `json_as(json!(TS literal))`: `damage`, `choose_mode`,
  `discover_from_catalog` (a plain `query` object must deserialise), `add_to_hand` (imported as
  `add_to_hand_effect`), `draw` (as `draw_effect`), `draw_from_library`, `add_cost_rule`, `cast`
  (`{ target, afterward }`, `CastHow` flattened), `discard`; `chosen_options(&EffectContext) ->
  Vec<String>`. `cast_new` by struct literal, as part 24.4: `CastNewArgs { def:
  CastNewDef::from(String), radiant: None, how: Default::default() }` (part 6.2 says it derives `Clone`
  only); if its fields differ, part 31 fixes `cost_rules.rs`'s one helper, `cast_new_of`.
- Cards crate: `jackioh_cards::register_all()` before `scenario(…)` (part 5.1's note).
- Testkit (part 5.1, owner): `Scenario::{play, attack(card, "hero"), end_turn, switch_position,
  expect_refused_with, expect_in_zone, state, state_mut, last_events, view, unit, card, stats}`, card
  refs as `&str` / `&CardInstance`, seats as `PlayerId`; `stats(..).keywords: Vec<Keyword>`.

## Decisions
- **Sinks.** TS's `sinkFor(state[, events])` held the state while tests read and wrote it between
  calls; an `EngineSink` borrows the state. Each file keeps a private `Bench { events, rng }` (the rng
  from `(state.seed, state.rng_cursor)`) that lends itself and the state to one engine call at a time
  (`draw_one(&mut bench.sink(&mut state), …)`), so the state is read between calls and the events and
  rng stay the one sink's. The cursor is written back only where TS wrote it (`cost-rules`' `run`).
  Where TS kept one context across effects (draw-limit's named draw, cost-rules' discard) the context
  lives across them and the state is read through `ctx.state` meanwhile.
- **Live objects.** TS read and wrote cards through the objects helpers handed back. Every read after
  an engine call goes back to the state by id (`find_instance`, a pile, a view); every write before a
  run goes through `find_instance_mut` (`monkeyCard.radiant = true`, `wind.vanilla = true`,
  `field.counters.plague = 2`, `quest.memory[KEY] = true`, `rock.grantedKeywords.push(…)`,
  `enchanted.enchantments = […]`, `big.embiggened = true`). `{ ...x, x: 2 }` is a clone with `x` set.
- **Matchers.** Events are compared as their JSON (`of_type`, `pluck`, `first_field`), so TS's `toEqual`
  literals port key for key and absent optionals compare as TS's `undefined`; views likewise.
  `toMatchObject` is a local `matches_object` (objects by subset, arrays by length and element);
  `toMatch(/literal/)` is `contains` on the refusal text; the one real regex,
  `/no card .* in p1's hand/`, is a small hand check (`no_card_in_hand`). `toBeUndefined` on an
  optional is `None`; `indexOf`/`findIndex` keep TS's -1.
- **Literals.** Actions are `json_as::<Action>(json!(…))` with the nonce added by each file's own
  `static NONCE: AtomicU32` (TS's module `let nonce`) and the prefix TS used (`cw`, `dl`, `dr`, `ec`,
  `pa-an-`, `rp`, `fold-`). Local fixture defs are `json_as::<CardDef>(json!(…))` with TS's module
  `nextIndex` written out per def (counterWarning from 1481, draw-complete 5601, draw-pause 2701, echo
  1501); echo's `{ ...base, ...extra }` is a shallow JSON merge.
- **TS defaults** are passed explicitly (`new_game(seed, None)` with TS's default seed `"engine-test"`
  where TS called `newGame()`, `in_hand(…, 1)`, `draw_one(…, None)`, `plays_of(…, PlayerId::P1)`,
  `LibraryEnd::Bottom`); a local helper's default (`drawFrom`'s `count = 1`, `handCard`'s `player =
  "p1"`, `run`'s `controller = "p1"`) is an explicit argument at every call.
- **Names**: every `R<n>` of a title is a leading `r<n>_` token, several in title order (`R46 and R91` →
  `r46_r91_…`; a ruling in a trailing parenthesis moves to the front: `(R3)` → `r3_…`); `§x.y` is
  `sx_y`; `#N` is `cN` or a trailing number word (`classic_22`); `C+` is `classic_plus`; punctuation
  is dropped. The title's possessive `R65's`, `R226's`, `R48's` lose the `'s` so the token stays `r65`.
- graveyard-play's `try … finally` restore of the registry: the override is the test thread's own
  (SURFACE §8), so the restore simply runs after the assertions.
- echo's structural `echoQueue(state)` read is `state.echo_queue` (part 1 froze the field), and
  `Object.hasOwn(state, "echoQueue")` reads the state's JSON.
- announce's `paused(seed)` and `setting(seed, trap)` (functions inside a `describe`) are private fns
  inside that `describe`'s `mod`, answering tuples for TS's `{ state, bolt, palantir }` / `{ state, trap }`.
