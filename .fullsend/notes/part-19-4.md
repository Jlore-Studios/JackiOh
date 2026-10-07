# Slice: part 19, chunk 4 of 6 (server 2: the results, series-rules and series tests)
BUILDS-RUN: 0

## FILES
- `crates/server/tests/api/series_rules.rs` (new): `apps/server/test/api/series-rules.test.ts` whole (39 `it`s, one
  `mod` per `describe`, TS order), with `trio`, `fresh`, `play`, `lcg`, `refusalOf`, `viewOf`.
- `crates/server/tests/api/results.rs` (new): `apps/server/test/api/results.test.ts` whole (20 `it`s), with `play`,
  `move` (`rating_move`), `toTheTurnCap`, `scenario`, `record`, `expectOneEnding`, and the series section's `trio`,
  `seriesRow`, `playNext`, `finish`, `seriesScenario`.
- `crates/server/tests/api/series.rs` (new): `apps/server/test/api/series.test.ts` whole (20 `it`s), with `trio`,
  `move`, `activeProfile`, `harness`, `getSeries`, `pick`, `forfeit`, `pickBoth`, `row`, `seatsOf`, `finishGame`,
  `ratings`, `inMatch`, `flush`, `advanceSweeping`.

## SURFACE
- §11.2: `support::deps::{test_app, call}`, `App { db, matches, .. }`, `Db::Fake(Arc<tokio::sync::Mutex<FakeData>>)`,
  `Db::begin(None)` / `Tx::<substore>_<method>` / `Tx::commit`, `Registry::{start(&self, &Arc<App>, StartMatchInput),
  has, stop}`, `api::series::run_sweeper(app)`. Time is `#[tokio::test(start_paused = true)]` + `tokio::time::advance`.
- §5.1: rows, views and inputs are built from TS's object literals with `serde_json::from_value(json!(…))` and read back
  as `serde_json::to_value(&x)` (as part 20.2 did), so the cases depend on TS's JSON keys, not on Rust field types.
  Typed arguments only where a call needs one (`PlayerId`, `Winner`, `GameOverReason`, slots through a generic
  `int::<T: TryFrom<i64>>`).
- §4.4.9: a transition TS threw `SeriesRefusal` from returns `Result<SeriesRow, SeriesRefusal>`; tests read `.reason`.

## DEPENDS-ON (names I call that other parts write)
- `jackioh_server::api::series_rules` (part 19): `SeriesRefusal { reason, message }` (Debug), `SeriesRefusalReason`
  (`NotPicking, Over, PickClosed, SlotOutOfRange, SlotWon, PickSealed, StalePick, PickOpen, NotPlaying, PicksMissing`;
  `Copy + PartialEq + Debug`), `NewSeriesInput` (`Deserialize`, TS's camelCase), `RatingMove { before: (f64, f64),
  after: (f64, f64) }` (TS's anonymous `{ before, after }` move), and, each returning `Result<_, SeriesRefusal>` where
  TS could throw: `new_series(&NewSeriesInput, i64) -> SeriesRow`, `pick_deck(&SeriesRow, PlayerId, slot, i64,
  Option<game_no>)`, `begin_game(&SeriesRow, i64)`, `game_ended(&SeriesRow, Winner, GameOverReason, i64, &str)`,
  `timeout_picks(&SeriesRow, i64)`, `forfeit_series(&SeriesRow, PlayerId, i64)`, `game_seats(&SeriesRow) ->
  Result<GameSeats { seats: (MatchSeat, MatchSeat), seed: String }, _>`, `rate_series(&SeriesRow, Option<RatingMove>)
  -> SeriesRow`, `series_score(&SeriesRow) -> Option<f64>` (Serialize), `won_slots(PlayerId, &[SeriesGame]) -> <a set,
  Serialize, .len()>`, `unwon_slots(&SeriesRow, PlayerId, Option<&[SeriesGame]>) -> Vec<slot>`, `first_unwon(…same) ->
  Option<slot>`, `both_picked(&SeriesRow) -> bool`, `already_picked(&SeriesRow, PlayerId, slot, Option<game_no>) ->
  bool`, `project_series(&SeriesRow, &str, i64) -> Option<SeriesView>` (Serialize).
- `jackioh_server::api::results` (part 19): `record_result(&Arc<App>, RecordResultInput) -> Result<ResultRow, _>` (the
  brief's name for what TS's `createRecordResult(deps)` returned), `reap_stuck_matches(&Arc<App>) -> Result<Vec<String>, _>`.
- `jackioh_server::api::series` (part 19): `start_series(&Arc<App>, NewSeriesInput, Option<&mut Tx>) -> Result<SeriesRow,
  _>` (TS's third parameter `store = deps.store`), `ensure_series_game(&Arc<App>, &SeriesRow) -> Result<(), _>`,
  `sweep_series(&Arc<App>) -> Result<SeriesSweep { timed_out, started, abandoned: Vec<String> }, _>`,
  `run_sweeper(Arc<App>)` (a `Send` future that loops; it logs `series.sweeper_failed` as TS did).
- `jackioh_server::actor::contracts` (part 19): `RecordResultInput { match_id: String, seats: (MatchSeat, MatchSeat),
  outcome: TerminalOutcome, turns: i32, at: i64, last_boards: Option<…> }`, `TerminalOutcome { winner: Winner, reason:
  GameOverReason }` (a struct or an alias of `GameResult`).
- `jackioh_server::actor::clock` (part 19): `initial_clocks(i64) -> MatchClocks` (Serialize), `match_ceiling_at(i64) -> i64`
  (TS's `config` argument is the constants now).
- `jackioh_server::app::now_ms() -> i64` (part 18): the server's epoch-ms clock (TS `deps.timers.now()`). It must follow
  tokio's clock, so `tokio::time::pause()`/`advance()` move it (SURFACE §11.3: `Timers` → `tokio::time`); series.rs's
  pick-clock tests depend on it.
- `crate::support::deps::{test_app() -> Arc<App>, call(&Arc<App>, &str, &str, Option<&str>, Option<Value>) -> (u16,
  HeaderMap, Value)}` (part 18); `test_app()` signs in with `E2eAuth`'s three fixture tokens.
- `crate::support::engine` (part 19): `create_fake_engine(FakeEngineOptions) -> <anything; held for the test>`, which
  installs the scripted cards for the calling thread (testkit override); `FakeEngineOptions: Default`;
  `fake_deck(&[&str]) -> Vec<String>` (TS `fakeDeck`). The installed cards must include `test-lethal` (ends the game,
  the player who played it wins by `hero-death`) and `test-mutual-lethal` (both heroes die: `both-heroes-dead`), playable
  from hand with 10 mana and no targets, and the 20 `test-card-<i>` fillers.
- `jackioh_server::db::fake::FakeData` (part 20): `reset(&mut self)`, `seed_profile(Value)`, `tables.{results,
  season_ranks, rated_games, matches}` (Serialize, `results` a `Vec<ResultRow>`), `on_call: Option<Arc<dyn Fn(&str) ->
  Result<(), StoreError> + Send + Sync>>` called with TS's method names (`"results.insert"`, `"series.update"`,
  `"series.active"`, `"matches.discardOpen"`) while the transaction holds the store.
- `jackioh_server::db::store` (part 20): `MatchSeat`, `StartMatchInput`, `MatchRow`, `MatchClocks`, `ResultRow` (Clone),
  `SeriesRow` (Clone), `FrozenTrio`, `Ticket` (all serde with TS's camelCase), `StoreError::{Duplicate(String),
  Other(String)}`; `Tx` methods `profiles_get_by_id, matches_{create, get, set_clocks, mode_of}, series_{get, update,
  active}, tickets_{insert, open_for_profile}`.
- `jackioh_server::ranked::glicko2::{Glicko { rating, deviation, volatility }, rate_game(&Glicko, &Glicko, f64) -> { a:
  Glicko, b: Glicko }}`, `jackioh_server::ranked::season::season_id_of(&str) -> String` (part 18).
- `jackioh_server::config::{MATCH_CEILING_MINUTES, RATING_DEVIATION_START, RATING_VOLATILITY_START, SERIES_MAX_GAMES,
  SERIES_PICK_SECONDS, SERIES_SWEEP_INTERVAL_SECONDS, SERIES_WINS_NEEDED}` (any numeric type: read through `as`).
- `jackioh_engine::validator::TRIO_DECKS`, `jackioh_engine::testkit::scenario` (`"turn"`, `"seed"`, `p1.hand`, `p1.mana`;
  `card(&str) -> &CardInstance`, `state()`), `jackioh_cards::{register_all, CATALOG, catalog_version}`.

## GAPS
- Every name under DEPENDS-ON that another chunk or part writes is a guess at its Rust shape; part 31 reconciles the call
  sites, not the behaviour. The likeliest misses: `record_result` vs a `create_record_result(app)` factory, the
  `Option` trailing parameters for TS's optional/default ones, `RatingMove`'s name, `rate_game`'s by-reference
  arguments and `{ a, b }` answer, `on_call`'s signature, `call`'s argument types.
- `jackioh_server::app::now_ms` must exist and follow tokio's paused clock; with a wall-clock server the five
  `start_paused` tests in series.rs never reach the pick deadline.
- The racing-writer test (results.rs) needs `on_call` to run while the losing transaction holds the fake's
  `tokio::sync::Mutex`, and that mutex to be fair (FIFO): its racing writer is a thread queued on `blocking_lock`, which
  writes after the rollback and before the retry. TS rewrote `getByMatch`; the fake has no per-method seam.
- `matches.discardOpen`: TS recorded the id it was asked; `on_call` sees only the method name, so series.rs counts calls
  (a series releases only its reserved game 1 id).
- Not ported: series-rules' `pickDeck(…, 1.5)` and `pickDeck(…, NaN)` (an integer slot cannot hold them; the route's
  body check refuses them, asserted in series.rs); results' `deps.matches.started` and series' `timers.pending`
  (read back off the match rows the registry writes, and off the sweeper task's `JoinHandle`).

## Decisions
- The match directory is the real registry: matches start through `app.matches.start` (row written, actor running), and
  TS's `started` list is the fake's `tables.matches` read back as `StartMatchInput`s. A results scenario backdated past
  its ceiling writes the row with `matches_create` and runs no actor (a live actor would resolve its own ceiling); the
  R263 reaper case stops game 5's actor and moves its ceiling to 0, where TS's fake rows carried a ceiling of 0.
- results.rs drives the real engine with `support::engine`'s scripted cards: a testkit `scenario()` on turn 1 with
  `test-lethal` and `test-mutual-lethal` in p1's hand (10 mana), inputs that name a card by id (`"card"`), both seats'
  R345 auto end switched off first. The turn cap is reached from `TURN_CAP_PLAYER_TURNS - 1` with two End turns (thirty
  real turns of empty libraries end in fatigue), so that test is named "the last player-turn is a draw".
- series.rs's trios are real: six disjoint 20-card slices of the sorted playable catalog (`jackioh_cards::CATALOG`), so
  the registry's real engine accepts every game and alice's cards can be looked for in bob's view as her name is.
- Players sign in as the E2E fixture accounts (`e2e-p1`, `e2e-p2`, `e2e-pending`), whose profiles each test seeds after
  `FakeData::reset()` empties the fixtures, keeping TS's profile ids.
- TS's recording logger is a thread-local `tracing_subscriber` JSON writer; a line is found by its quoted event name.
- TS default parameters are `Option` trailing arguments in Rust (`None` for TS's default); a TS default that no test
  overrides (`fresh(now)`, `play(…, now)`, `viewOf(…, now)`, `getSeries(…, id)`) is fixed instead.
- `#[test]`/`mod` names follow SURFACE §7.3: the title snake_cased, rulings leading; `§9.5` becomes `9_5`.
