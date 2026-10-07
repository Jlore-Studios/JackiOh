# Spec gaps found in Wave 2 (part 31)

## engine tests

- **Testkit seams for `vi.mock` (part 26.6, `crates/engine/tests/rules/turn_wiring.rs`).** The file
  imports `jackioh_engine::testkit::{mock_brittle_tick, mock_animate_at_turn_start,
  mock_return_at_cleanup}`, each taking `impl Fn(&mut EngineSink<'_>, PlayerId)` and standing in for
  `brittle::brittle_tick`, `animated::animate_at_turn_start` and the end-of-turn cleanup return, the
  way TS's `vi.mock` of those modules did. The engine has none of them; SURFACE §8 names only the
  thread-local registry override. Decision taken here: the tests are kept as written (not deleted);
  the engine-src reconciler (or the orchestrator) decides whether these three thread-local hooks
  exist, `#[cfg(feature = "testkit")]` beside the registry override. Until then: 1 E0432 in
  turn_wiring.rs, and the `rules` binary does not build while it stands (its 9 tests).
  **DECIDED (orchestrator; built in part 32, `# engine green` in reconcile-decisions.md):** the three
  doubles exist in `testkit::seams` with the names and argument this file calls them with; `turn.rs`
  runs a set double in place of its stage under `testkit` only.
- **A test-made work handler (part 25.3, `crates/engine/tests/rules/pauses.rs`).** Two TS tests
  ("§9.3 resumes a three-step sequence at the step after the one that asked", "§9.3 finishes a pause
  nested inside an owed sequence before the sequence's own tail") drain work owed under a hook name the
  test registers with `registerWorkHandler`; SURFACE §6.6 replaces registration with `work.rs`'s
  closed `match`, so a test cannot add one. Part 25.3 left an empty module in their place. Decision
  taken here: nothing deleted or added; the engine-src reconciler decides whether the testkit gets a
  thread-local `register_work_handler(hook, fn(&mut EngineSink, &WorkItem))` consulted by the
  dispatcher's fallthrough under `#[cfg(feature = "testkit")]` (the same exception SURFACE §8 makes
  for the registries); if it does, the two tests are ported back from `pauses.test.ts`.
  **DECIDED (orchestrator; built in part 32, `# engine green` in reconcile-decisions.md):**
  `testkit::register_work_handler(hook, WorkHandler) -> Option<WorkHandler>` (the previous one, as
  TS) and `unregister_work_handler(hook)`, `WorkHandler = fn(&mut EngineSink<'_>, &WorkItem)`;
  `work::run_work_item`'s fallthrough and `work::can_resume` consult it under `testkit` only. The
  engine-tests owner ports the two tests back.
- **The testkit exports no `Arc`** (SURFACE §8 lists `json!`, `Value`, `serde_json`, `IndexMap`,
  `IndexSet`, `json_as`), while effect argument structs that hold functions (`ForEachCardArgs`,
  `ChooseTargetWhereArgs`, `DrawWhileArgs`, `CastEachArgs`) need `Arc::new` at the call. Decision:
  four test files import `std::sync::Arc` themselves; no SURFACE change needed unless the orchestrator
  prefers the testkit to re-export it as the prelude does.

## ai (part 31)

- **SURFACE §4.3 / §9: the AI's node counts are not classified.** §4.3 sorts TS `number` into game
  quantities (`i32`), indexes (`usize`), cursors (`u32`) and fractions (`f64`), and part 1 added
  "bounds on Rust loops and collection lengths `usize`"; a search budget fits none of them cleanly,
  and part 17's chunks split on it (17.1 `i32` "every AI count", 17.2 `usize`). Decision taken:
  `SearchBudget`'s fields, `SearchStats.{nodes, determinizations, lines, sim_errors}`,
  `NodeCounter::{used, limit}`, `find_lethal`'s `limit` and `MatchRecord.nodes` are `usize`
  (`crates/ai/src/types.rs`); the AI's config counts (`AI_SEARCH`, `AI_REPLY`, `AI_MULLIGAN`) stay
  `i32` and are cast where they meet a node count. JSON is unchanged (both serialise as numbers),
  so `ai_decide`'s answer and `constants()`'s `AI_BUDGET` read the same in the web. Suggested line
  for §4.3: "`number` (an AI node budget or count) → `usize`".
- **SURFACE §9: `NodeCounter`'s receiver.** §9 names no counter; the decision (`&dyn
  NodeCounter`, `&self` methods, tallies in `Cell`) is `types.rs`'s. `Cell` is allowed by §3's
  `clippy.toml` (only `RefCell` is banned); if §3 means "no interior mutability" generally, the
  counter is the one exception and should be named there.

## tools and wasm

- **SURFACE §14.2, the arena's records' `pilots`.** §14.2 says the records name the two agents in
  `pilots`, but R376's `GameRecord.pilots` is `PerPlayer<Pilot>` with `Pilot` = `"human" | "ai"`, and
  `jackioh-server stats-import` parses records as `GameRecord`. Decision kept (part 29.1): both
  pilots are `"ai"`, and the agents are named in the record id,
  `dev:<patch>:arena:<a label>-vs-<b label>:<seed>` (labels `self`, `random`, `bin-gen<N>`), which
  also keeps a promotion's two series (same seeds) distinct. Suggested SURFACE wording: "`pilots`
  both `ai`; the agents are named in the record's `id`".
- **SURFACE §10.1 / §5.1, the web's engine constants.** `apps/web/src/wire/engineConfig.ts` carries
  13 constants, not 12: `LIBRARY_CAP` was added because `apps/web/src/game/animations.window.test.ts`
  imports it (part 21). Kept; §5.1's list should name it.
- **SURFACE §10.1, `validator("checkDeckDraft", …)`.** "The input and output are those TS
  functions' argument and result, as JSON" cannot hold for `checkDeckDraft`: its argument
  (`DeckDraftInput`) holds two predicates (`is_deckable`, `is_portrait`). Decision kept (part 21):
  it crosses as `{ name, cards, deckable: string[], portrait?, portraitKnown?, nameMaxLength }`, the
  page answering the predicates for exactly the ids and portrait the rules ask about, and the
  binding rebuilds them as closures.
- **SURFACE §10.1, the bindings' Rust return types.** §10.1 writes `-> String` (and `ai_to_act ->
  bool`); the bindings return `Result<String, JsError>` (`Result<bool, JsError>`), which
  wasm-bindgen exposes with the same JS return type and throws an `Error` with the engine's message
  where TS threw (bad JSON, a bad seat, an unknown validator call, a setup `create_game` would panic
  on). Kept; §10.1 should say so.

## engine src (part 31)

- **SURFACE §6.5, `EngineSink`'s fields.** §6.5 lists `state`, `events`, `rng`, `converting`,
  `dry_running`. Two more are needed and are now in `script.rs`: `frontier:
  triggers::FrontierSlot<'a>` (TS told an action's events apart by object identity — `SettleSink.
  dispatched` and `triggers.ts`'s two module `WeakSet`s; Rust keeps positions in the action's event
  list, and every sink over one list must share them, so `reborrow` hands a nested sink its parent's
  `Frontier`), and `owed_behind: Option<IndexSet<String>>` (TS `work.ts`'s `DrainSink.owedBehind`,
  R117, cloned by `reborrow`). Decision taken: both added; `converting`/`dry_running` stay copied down
  (every change to `converting` is undone before its call returns). Suggested §6.5 wording: list the
  two fields and say "`reborrow` shares `frontier` and copies the rest".
- **SURFACE §6.6, `TriggerWhen`.** §6.6 (and part 1's alias) give `TriggerDef.when` a `&EffectContext`;
  Classic+ #74's `when` writes its own card's memory, reveals it and gains Brittle while it declines
  (R99). Decision taken: `TriggerWhen = Arc<dyn Fn(&mut EffectContext, &GameEvent) -> bool>`, and
  `with_when` likewise.
- **SURFACE §8, the testkit's `Scenario` methods.** The table names `s.state()` but not
  `state_mut()` (TS tests assign `g.state.…`; `invariants.rs` and ~440 card-test calls use it) nor
  `card_mut()` (TS wrote through `s.card(…)`'s live object; 17 card files wrote a private copy).
  Decision taken: both are `Scenario` methods (`state_mut(&mut self) -> &mut GameState`,
  `card_mut(&mut self, impl Into<CardRef>) -> &mut CardInstance`). Suggested §8 row: "`s.state` written
  through → `s.state_mut()`; `s.card(ref)` written through → `s.card_mut(ref)`".
- **SURFACE §6.1, `view_for`'s clock.** TS's `viewFor(state, player, clockMs?)` (R79) has a third
  argument the server's actor passes; §6.1 fixes `view_for(state, player)`. Decision kept (part 5.2):
  `view_for_with_clock(&GameState, PlayerId, Option<i32>)` beside `view_for`. §6.1 should list it.
- **SURFACE §7.1, the prelude and the testkit in one card test.** A card's `mod tests` globs both
  (`use super::*; use jackioh_engine::testkit::*;`). The prelude's `catalog::*` brought
  `catalog::register_catalog`, the testkit its override of the same name, so naming it was ambiguous.
  Decision taken: the prelude lists catalog's names explicitly, without `register_catalog`. Still
  ambiguous where named bare (part 1's design, unchanged): the effect verbs `draw`, `add_to_hand`,
  `end_turn`, `gain_mana`, `refresh_mana`, `lose_health` against the engine functions, and the module
  names `damage`, `draw`, `combat`, `mana`, `plague`, `kill_credit`, `fuse` (`effects::…` vs the root).
- **Test seams asked for by the engine-tests reconciler (above): not added.** `mock_brittle_tick`,
  `mock_animate_at_turn_start`, `mock_return_at_cleanup` (TS `vi.mock`, part 26.6) and a test-made work
  handler (TS `registerWorkHandler`, part 25.3) would put thread-local hooks into engine code paths;
  SURFACE §6.6 removed the registration hooks and §8 allows exactly one override (the registries).
  Decision: none added (a reconciler adds no feature neither design had). For the orchestrator: port
  those tests against the real board (`turn_wiring.rs`'s 9, `pauses.rs`'s 2) or drop them; or patch §8
  to allow the seams, and the engine owner adds them in Wave 3. **DECIDED:** the orchestrator chose the
  seams; part 32 added them (`testkit::seams`, see the two entries under "engine tests").
- **SURFACE §6.6, closure fields of effect arguments.** "A field that holds a function is skipped by
  serde and set in Rust" leaves its signature open. Decision taken (the owners'): a field that
  builds a list or a query reads `&mut EffectContext` (`ForEachCardArgs.cards`, `CastEachArgs.cards`,
  `DrawWhileArgs.more`, `DiscoverFromCatalogArgs.query_fn`, `CastNewDef::Read`, `CastRandomQuery::Read`,
  `CastRandomCount::Read`); a filter reads `&EffectContext` (`ChooseTargetWhereArgs.where_`,
  `ChooseFromHandArgs.where_`). `forEachCard`/`castEach` answer card ids (`Vec<String>`).
- **`wire/codes.rs`'s NFKC tables** (~180 KB, hand-generated from Node's ICU, part 5.3) are left as
  they are; SURFACE §2 lists no Unicode crate, so a replacement is part 37's call.

## server (part 31)

- **SURFACE §11.2, the handlers' receiver.** It fixes every handler as `pub async fn <name>(app: &App,
  req: Req) -> ApiResult` and also `Registry::start(&self, app: &Arc<App>, ..)`: a handler that starts a
  match (queue pairing, a room's join, a rematch, a series pick) cannot get the `Arc` from `&App`.
  Decision taken: every handler, `app::Handler`, `h!` and `api::http::dispatch` take `&Arc<App>`;
  part 19.2's `Registry::bind`/`app()` workaround is deleted, so `app.rs`'s boot and the test app need
  no bind. Suggested §11.2 text: `pub async fn <name>(app: &Arc<App>, req: Req) -> ApiResult`.
- **SURFACE §2, `jsonwebtoken`.** 11.1 has no crypto provider by default and panics on the first
  verify. Decision taken: `features = ["rust_crypto"]` on the workspace entry (Cargo.lock updated).
- **SURFACE §4.3, `x?: T | null`.** The table has `x?: T` and `x: T | null` but not TS's three-state
  optional nullable (`Profile.displayName`, `FrozenDeck.portrait`, `Ticket.portrait`,
  `Room.hostPortrait`), which the store's rows keep apart (absent vs `null`). Decision taken (the
  store's, 20.4): `Option<Option<T>>` with `#[serde(default, skip_serializing_if = "Option::is_none",
  with = "absent_or_null")]`.
- **SURFACE §4.3, a store's counts and caps.** Not game quantities, indexes or cursors. Decision taken
  (store.rs): `i64` for every count, cap, position, sequence number and epoch ms at the `Tx` boundary;
  pg.rs keeps TS's `::int` SQL and binds `int4` behind a conversion.
- **Five TS tests dropped for want of a seam (19.6), for part 35.** `rooms.test.ts`: "R149 retries
  past a code that is already in use and mints the next one" and "R149 gives up after a bounded number
  of collisions and says no code is available" (they script `Ids.code`; the Rust server mints room
  codes itself). `series-recovery.test.ts`: "R263 a pick that loses the compare-and-set is re-applied
  to the row that won", "R263 a write that keeps losing is refused with 409 and writes nothing", "R263
  a result whose series write loses the compare-and-set re-reads the series and records the game"
  (they replace `store.series.update`; `FakeData.on_call` can only fail a call). Seams were not added
  (a reconciler adds no feature); part 35 decides: an injectable code source in `actor::rooms`, and a
  fake-store hook that may change the tables before a named method runs.
- **The test app is always `E2E=1`** (`tests/support/deps.rs`'s `test_env`; without it `load_env`
  demands Supabase and Postgres). TS's test deps were not E2E unless asked, so
  `tests/actor/rooms.rs`'s non-E2E harness (R143: a client-sent seed is ignored outside E2E) builds the
  same app as its E2E arm; part 19.6 asked for `TestAppOptions.e2e`. Not added; for part 35.
- **The test app's patch version.** TS's test deps rated games under `TEST_PATCH_VERSION = "v0.1.1"`;
  the Rust server rates in `jackioh_cards::catalog_version()`'s season (SURFACE §11.3), so
  `tests/store/season_start.rs` (now reading `TEST_CATALOG_VERSION`) expects a v0.1 season the server
  cannot open. For part 35: a `SeasonDeps` on `App` (a test seam), or recast the expectations.
- **TS's optional `tx` on `startSeries`.** 19.2 made `api::series::start_series` always take the
  caller's `&mut Tx` (both TS callers passed one); four tests that called it without one now open and
  commit their own. No SURFACE change needed.

## cards (part 31)

- **SURFACE §8 does not say how a card test registers the shipped cards.** TS did it once, in vitest's
  globalSetup; the engine's testkit cannot (it cannot name `jackioh_cards`), so Wave 1 wrote 88 local
  wrappers (`scenario`, `game`). Decision taken here: one registering `scenario()` per test binary in
  the cards crate — `crate::scenario` (`src/lib.rs`, `#[cfg(test)]`) for the card files' `mod tests`,
  `cross::scenario` (`tests/cross/mod.rs`) for the cross tests — beside `crate::{js, matches_object,
  merged, unit_or_blank}` for the JSON idioms TS wrote inline (`toMatchObject`, object spread). SURFACE
  §8's table could name them so a card added later uses them.
- **SURFACE §8's table has no `card_mut`.** TS tests wrote through `s.card(x)`'s live object
  (`stepParam(s.card(x), …)`, `s.card(x).radiant = true`); the engine reconciler added
  `Scenario::card_mut` and the cards' 142 shims around it are gone. The table should list
  `s.card_mut(ref) -> &mut CardInstance` beside `card()`.
- **`query::recalled`'s argument.** TS took `Pick<EffectContext, "self" | "data">`, so a pure-read hook
  (Classic #28's replacement `when`) could call it as `recalled({ self, data: {} }, key)`; Rust's takes
  `&EffectContext`, so #28 keeps a two-line private reader. Decision: left; the engine owner may widen
  it (`recalled(self_: Option<&CardInstance>, data: &IndexMap<…>, key)`), and then the copy goes.
- **naming.ts's tests live in the cards crate, its code in the tools binary** (port-map: naming.ts →
  `crates/tools/src/patches.rs`; its test → `crates/cards/tests/cross/registry.rs`). A test binary of
  `jackioh-cards` cannot import a binary crate, so part 22.3 copied `naming` into the test. Decision:
  left as is; moving the naming tests into `patches.rs`'s `#[cfg(test)]` (tools) deletes the copy.
- **A cross test reads a file outside the crate**: `tests/cross/radiant_standard.rs` does
  `include_str!("../../../../docs/radiant-audit.md")`. SURFACE §3 ("data reaches a pure crate only at
  compile time") allows it, but part 37's cull must keep `docs/radiant-audit.md` where it is.
