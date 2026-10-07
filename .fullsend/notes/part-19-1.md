# Slice: part 19, chunk 1 of 6 (#407): server 2 — queue, rematch, results, series rules, game records, test support
BUILDS-RUN: 0

## FILES
- `crates/server/src/api/series_rules.rs` ← `apps/server/src/api/series-rules.ts` (whole)
- `crates/server/src/api/game_records.rs` ← `apps/server/src/api/game-records.ts` (whole)
- `crates/server/src/api/results.rs` ← `apps/server/src/api/results.ts` (whole) + `run_reaper` (index.ts's reaper loop, SURFACE §11.2)
- `crates/server/src/api/queue.rs` ← `apps/server/src/api/queue.ts` (whole) + `run_matchmaker`
- `crates/server/src/api/rematch.rs` ← `apps/server/src/api/rematch.ts` (whole)
- `crates/server/tests/support/engine.rs` ← `apps/server/test/fakes/engine.ts` (test cards on the real engine)
- `crates/server/tests/support/socket.rs` ← `apps/server/test/fakes/socket.ts`

## GAPS

### Not ported (by design, or no Rust home)
- `createQueueRoutes()` / `createRematchRoutes()`: the route table is `app.rs`'s `ROUTES` (part 18). Its rows,
  in TS order: `POST /api/queue` active → `api::queue::enqueue`; `DELETE /api/queue` active →
  `api::queue::dequeue`; `GET /api/queue/population` user → `api::queue::population`;
  `POST /api/matches/:matchId/rematch` active → `api::rematch::offer_rematch`;
  `GET /api/matches/:matchId/rematch` active → `api::rematch::rematch_status`.
- `startMatchmaker` → `api::queue::run_matchmaker(app)` (a loop; `{stop}` is aborting its task).
- `createRecordResult(deps)` / `createVoidMatch(deps)` (factories of ports) → the functions themselves:
  `api::results::record_result(&Arc<App>, RecordResultInput) -> Result<ResultRow, ApiError>` and
  `api::results::void_match(&Arc<App>, VoidMatchInput) -> Result<(), ApiError>`. The actor calls these.
- `api::results::reap_stuck_matches(&Arc<App>) -> Result<Vec<String>, ApiError>`, `run_reaper(Arc<App>)`.
- fakes/engine.ts: `createFakeEngine` (no ScriptedEngine, SURFACE §11.2), `FAKE_HAND_SIZE` (real hands are
  `OPENING_DRAW`), the fake `dealRandomDeck`, and the cards `test-glitch-swap` / `test-glitch-void` (a Glitch's
  outcome is the engine's own draw; no effect forces one). Tests that used them (glitch.rs, parts of
  match_actor.rs) need another way to reach a swap/void — e.g. the real Glitch card under a seed that draws it.
- game-records.ts's "no recorder bound → null" branch: the Rust server always has the engine.

### SURFACE conflict for part 31
- SURFACE §11.2 fixes handlers as `pub async fn x(app: &App, req: Req)` AND `Registry::start(&self, app:
  &Arc<App>, ..)`. A handler that starts a match cannot reach an `Arc<App>` from `&App`. So `api::queue::enqueue`
  and `api::rematch::offer_rematch` take `app: &Arc<App>` (the rest take `&App`). `h!` should call
  `$f(&app, req)` with `app: Arc<App>` inside an `async move` block; deref coercion then serves both shapes.
  `try_pair`, `record_result`, `void_match`, `reap_stuck_matches` take `&Arc<App>` for the same reason.

### Names I call that other parts provide (guessed by the TS, snake_cased)
Part 18:
- `crate::app::App { env.e2e: bool, db, matches, catalog.version: String }`.
- `crate::api::http::{ApiError { code, message, details, retry_after_ms }, ApiErrorCode::{BadRequest, NotFound,
  AlreadyInMatch, AlreadyQueued, MatchNotFinished, SeriesGame, DoubleRequiresRanked, Internal}, ApiResult,
  Req { caller: Option<Caller{profile}>, params, body }, json(u16, Value)}`. I build `ApiError` by struct literal.
- `crate::config::{MATCHMAKER_SWEEP_INTERVAL_SECONDS, MATCH_REAPER_INTERVAL_SECONDS, RATING_START,
  REMATCH_OFFER_TTL_MS, RESULT_WRITE_ATTEMPTS, SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED,
  rating_window(f64) -> f64}` (integers cast with `as`, so any integer type works).
- `crate::api::decks::{read_mode_choice(&Value) -> Result<ModeChoiceInput, ApiError>,
  freeze_choice(&App, &str, &ModeChoiceInput) -> Result<FrozenChoice, ApiError>, assert_not_in_series(&App, &str)
  -> Result<(), ApiError>, FrozenChoice::{Bo1 { deck: FrozenDeck }, Bo3 { trio: FrozenTrio }, Random}}`.
- `crate::api::ranked::{rate_ranked_game(&mut Tx, &App, RankedGameInput) -> Result<RatedGameRow, E: Display>,
  RankedGameInput { id, kind, catalog_version, sides, winner_side: Option<usize>, reason, at, stake },
  RankedSideInput::Player { profile_id }}`.
Part 20 (`crate::db::store`):
- `Db::begin(None)`, `Tx::commit`; Tx methods `results_get_by_match, results_insert(&ResultRow), series_by_match,
  series_with_game, series_active, series_active_for, matches_get, matches_live, matches_mode_of, matches_actions,
  matches_finish(&str, i64), matches_forget_voided, matches_discard_open, ranked_lock_seasons,
  profiles_get_many(&[String]), profiles_set_in_match(&str, Option<&str>), last_boards_put(&str, LastBoardKind,
  &[LastBoardEntry], i64), tickets_insert(&Ticket), tickets_get, tickets_open_for_profile, tickets_list_open,
  tickets_claim_pair(&str, &str, &str, i64) -> bool, tickets_cancel(&str, i64), tickets_count_open,
  tickets_count_open_by_mode (any Serialize), game_records_insert(&GameRecord) -> bool`. Strings by `&str`.
- Types and fields (SURFACE §4.3: pairs are tuples, epoch ms `i64`, ratings `f64`): `Profile { id, rating,
  in_match_id }`, `Ticket {..}`, `TicketStatus::{Open, Matched}`, `QueueMode::{Bo1, Bo3, Random}`, `MatchRow {..,
  ranked: Option<bool>, stake: Option<i32>, status: MatchStatus, clocks.ceiling_at, last_boards, glitch_boards,
  portraits: Option<(P, P)> with P: Display}`, `MatchStatus::Finished`, `MatchSeat { profile_id, player, deck,
  portrait: Option<String> }`, `StartMatchInput { match_id, seed, catalog_version, ranked, seats, mode, stake }`,
  `ResultRow {..}`, `SeriesRow`/`SeriesSide`/`SeriesGame` (`slots: (usize, usize)`, `pick: Option<usize>`,
  `winner: Option<Winner>` — the wire `Winner` is TS's `SeriesSeat | "draw"`), `SeriesSeat::{P1, P2}` (its own
  enum or an alias of `PlayerId`: both compile), `SeriesStatus`, `SeriesEnd`, `FrozenTrio { name, decks: (D, D, D) }`,
  `FrozenDeck { name, cards, portrait: Option<String> }`, `LastBoardKind::Server`, `RatedGameKind::Match`,
  `RatedReason::Game(GameOverReason)` (`RatedGameRow.reason: GameOverReason | SeriesEnd`),
  `StoreError::DuplicateResult { .. }` (TS `DuplicateResultError`), `StoreError: Display`.
Part 19, other chunks:
- `crate::actor::contracts::{RecordResultInput { match_id, seats, outcome, turns, at, last_boards:
  Option<LastBoardInput> }, TerminalOutcome { winner: Winner, reason } (Clone), VoidMatchInput { match_id,
  players: (String, String), at }}`.
- `crate::actor::engine::deal_random_deck(&str) -> Vec<String>` (TS engine.real.ts `dealRandomDeck`).
- `crate::actor::ws_server::{Socket::new(mpsc::UnboundedSender<SocketFrame>, mpsc::UnboundedReceiver<String>),
  SocketFrame::{Text(String), Close { code: u16, reason: String }}}` — the shape `tests/support/socket.rs` builds.
- `crate::api::series::{start_series(&App, &NewSeriesInput, &mut Tx) -> Result<SeriesRow, E: Display>,
  advance_series_in_tx(&mut Tx, &App, &SeriesRow, SeriesGameResult) -> Result<SeriesRow, E: Display>,
  resume_series(&Arc<App>, Option<SeriesRow>), SeriesGameResult { match_id, seats, outcome, at }}`.
  `NewSeriesInput`/`NewSeriesSide` live in `series_rules.rs` (TS's home; series.ts re-exported it).
- What series.rs gets from series_rules.rs: `pick_deck(&SeriesRow, SeriesSeat, i32, i64, Option<i32>)`,
  `already_picked(&SeriesRow, SeriesSeat, i32, Option<i32>)`, `game_ended(&SeriesRow, Winner, GameOverReason, i64,
  &str)`, `timeout_picks`, `abandon_unstarted`, `forfeit_series`, `begin_game`, `new_series(&NewSeriesInput, i64)`,
  `rate_series(&SeriesRow, Option<RatingMove>)`, `game_seats -> Result<GameSeats { seats, seed }, SeriesRefusal>`,
  `series_score -> Option<f64>`, `seat_of`, `unwon_slots(&SeriesRow, SeriesSeat, &[SeriesGame])`, `first_unwon(..)`
  (TS's defaulted `games` is explicit), `project_series -> Option<SeriesView>`, every transition
  `Result<SeriesRow, SeriesRefusal { reason, message }>`.
Part 5: `jackioh_engine::wire::pick_portrait_from_seed(&str) -> PortraitId` (Display); `GameRecord { id, source,
mode, patch, pilots: PerPlayer<Pilot>, game }`, `GameSource::Live`, `GameMode::{Bo1, Bo3, Random}`, `Pilot::Human`;
`FoldArgs` Deserialize (camelCase, built from JSON); `summarize_game(&FoldArgs)`; testkit `register_catalog(CardDefs)`
and `register_scripts(IndexMap<String, CardScripts>)` (one argument each, SURFACE §8).
Part 7: `effects::{choose_mode, lose_health}` taking a Deserialize argument struct (built with `json_as`).

## Decisions
- Error model: every fallible path returns `ApiError`. A failure that was a plain `Error` in TS is
  `ApiErrorCode::Internal` with its own sentence (the actor, the reaper and the matchmaker log it); at each handler's
  edge `hide_internal` turns an internal error into TS's router answer — log `handler.threw`, answer 500 `internal`
  "something went wrong".
- Time: a private `now_ms()` per file (fullsend rule 5), identical bodies: system epoch ms shifted by how far
  tokio's clock runs ahead of `std::time::Instant`, so `tokio::time::pause()`/`advance()` move it like TS's manual
  timers and it is the plain system clock otherwise. Part 31 may fold the copies into one shared fn.
- Ids: `uuid::Uuid::new_v4()`; seeds 16 bytes of `getrandom::fill` as hex (TS `systemIds`).
- Logs: `tracing` with TS's event names (`event = "queue.paired"`) and TS's camelCase field names; `alert` is
  `tracing::error!`; lists are logged as their JSON text; optional fields are `Option` values (absent when None).
- Store calls TS made outside `store.tx` each run in their own short `Tx` (`begin(None)` … `commit`); a `Tx` is
  never held across a call into another module that may begin its own (the fake store's mutex would deadlock).
- Module-level maps (`queue.rs`'s E2E seeds, `rematch.rs`'s offers) are process statics behind a `Mutex`, never held
  across an await. Unlike vitest's per-file module instances they are shared by every test of the one test binary:
  a test asserting `e2e_seed_count()` or `rematch_offer_count()` must tolerate other tests' entries.
- Rematch's "is this still my entry" (`offersByMatch.get(id) === entry`) is a generation number on the entry.
- series_rules: TS `SeriesSeat | "draw"` is the wire `Winner`; slots and picks are `usize` in rows and `i32` at
  the request edge (a negative slot is `slot_out_of_range`); `game_seats`'s missing-deck throw is a panic (an
  impossible state, SURFACE §4.4.9); `SeriesView` and its parts are serde camelCase with TS's nulls.
- results: `RESULT_WRITE_ATTEMPTS` retries only `StoreError::DuplicateResult`; `seats_of` leaves portraits `None`;
  `run_reaper`'s first sweep is one interval after boot, as TS's `start()`.
- game_records: `patch` is `jackioh_cards::catalog_version()` (the newest patch, R388); `FoldArgs` is built from JSON.
- support/engine.rs: the test cards are real 0-cost Spells with Quickdraw (so the deal puts them in hand) installed
  over the whole real catalog with the testkit override (`install_test_cards`); prompts are `choose_mode` (kind
  `mode`, not TS's `target`); games open on both mulligans as real games do (`past_the_mulligans` answers them).
  Lethal is `lose_health` (no Armor, no hero cap, R18), mutual lethal two of them in one effect list.
- support/socket.rs: drains the outgoing channel on every read; adds `settle`, `next_frame`, `next_of_type` for
  the async actor; a server close drops the client's sender (TS's `handlers.close()`); `drop()` is 1006.
