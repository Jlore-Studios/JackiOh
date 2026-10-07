# Slice: part 19, chunk 2 of 6 (server 2: the match lifecycle — actor, clock, protocol, registry, engine seam, series)
BUILDS-RUN: 0

## FILES
- `crates/server/src/actor/protocol.rs` (new) ← `apps/server/src/match/protocol.ts`
- `crates/server/src/actor/contracts.rs` (new) ← `apps/server/src/match/contracts.ts`
- `crates/server/src/actor/clock.rs` (new) ← `apps/server/src/match/clock.ts`
- `crates/server/src/actor/engine.rs` (new) ← `apps/server/src/match/engine.ts` + `engine.real.ts`
- `crates/server/src/actor/match_actor.rs` (new) ← `apps/server/src/match/actor.ts`
- `crates/server/src/actor/registry.rs` (new) ← `apps/server/src/match/registry.ts`
- `crates/server/src/api/series.rs` (new) ← `apps/server/src/api/series.ts`

## SURFACE
- §11.2 `Registry` exactly: `new()`, `start(&self, &Arc<App>, StartMatchInput) -> Result<(), ApiError>`,
  `attach(&self, &Arc<App>, &str, &str, Socket) -> Result<(), AttachError>`, `has`, `stop` (async),
  `presence_of -> Option<PerPlayer<bool>>`; plus TS's `actor_for(&Arc<App>, &str)`, `live()`, and `bind`/`app` (below).
- §11.2 handler shape for the four series routes (`get_series`, `pick`, `forfeit`, `match_series`) and the
  loop name `api::series::run_sweeper(app: Arc<App>)` (first sweep after 5 s, then every 5 s after each sweep).
- §11.3: no `EnginePort`/trait; engine.rs is plain `pub fn`s over `jackioh_engine` (create_game, begin_game,
  reduce, legal_actions, view_for, fold, hash_state, snapshot, deal_random_deck, last_boards, summarize_game).
  `Timers` → tokio::time, `Logger` → tracing. The nonce is read only inside `action`; `joinRoom` is answered
  `malformed`.

## DEPENDS-ON (names I call that other parts write)
- `crate::app::App { db, matches: Registry, .. }` (part 18), `Send + Sync`.
- `crate::api::http` (part 18): `ApiError { code: ApiErrorCode, message: String, details: Option<Value>,
  retry_after_ms: Option<i64> }` (built by struct literal, SURFACE §11.2's fields), `ApiErrorCode::{BadRequest,
  NotFound, Conflict, Internal}`, `ApiResult`, `Req { caller: Option<Caller { profile, .. }>, params, body, .. }`,
  `json(u16, Value) -> Response`.
- `crate::api::results` (part 19, another chunk), both async, error `E: Display`:
  `record_result(&App, RecordResultInput) -> Result<ResultRow, E>` and `void_match(&App, VoidMatchInput) ->
  Result<(), E>` (the actor passes `&Arc<App>`, which coerces). TS's ports `createRecordResult`/`createVoidMatch`
  become these two direct calls. `RecordResultInput` and `VoidMatchInput` are defined in `actor::contracts` (TS's
  home), so results.rs imports them from there.
- `crate::api::series_rules` (part 19, another chunk): `SeriesRefusal { reason: SeriesRefusalReason, message:
  String }`, `SeriesRefusalReason::{SlotOutOfRange, SlotWon, PickSealed, NotPicking, StalePick}`, `NewSeriesInput`,
  `RatingMove { before: (f64, f64), after: (f64, f64) }` (my name for TS's anonymous `{ before, after }`),
  `new_series(&NewSeriesInput, i64) -> SeriesRow`, `pick_deck(&SeriesRow, SeriesSeat, i32, i64, Option<i32>)`,
  `timeout_picks(&SeriesRow, i64)`, `abandon_unstarted(&SeriesRow, i64)`, `forfeit_series(&SeriesRow,
  SeriesSeat, i64)`, `game_ended(&SeriesRow, Winner, GameOverReason, i64, &str)` (each `-> Result<SeriesRow,
  SeriesRefusal>`), `rate_series(&SeriesRow, Option<RatingMove>) -> SeriesRow`, `game_seats(&SeriesRow) ->
  Result<GameSeats { seats: (MatchSeat, MatchSeat), seed: String }, SeriesRefusal>` (read by field name),
  `already_picked(&SeriesRow, SeriesSeat, i32, Option<i32>) -> bool`, `project_series(&SeriesRow, &str, i64) ->
  Option<SeriesView: Serialize>`, `seat_of(&SeriesRow, &str) -> Option<SeriesSeat>`, `series_score(&SeriesRow) ->
  Option<f64>`.
- `crate::api::ranked` (part 18): `RankedGameInput { id, kind: RatedGameKind, catalog_version, sides:
  (RankedSideInput, RankedSideInput), winner_side: Option<usize>, reason: RatedReason, at: i64, stake: Option<_> }`,
  `RankedSideInput::Player { profile_id }`, `RankedPlan { row: RatedGameRow { sides: (s, s) with s.before.rating,
  s.after.rating: f64 }, .. }`, `plan_ranked_game(&mut Tx<'_>, &App, &RankedGameInput)` and
  `commit_ranked_game(&mut Tx<'_>, &RankedPlan, i64)` (async, error `E: Display`); `RatedReason: From<SeriesEnd>`.
- `crate::db::store` (part 20): `Db::begin(None)`, `Tx::commit`, `StoreError: Display`, and the Tx methods
  `matches_{create, get, append_actions(&[MatchActionRow]), actions, set_clocks(&str, &MatchClocks), finish(&str,
  i64), mode_of, discard_open}`, `last_boards_{get(&str, LastBoardKind), sample_others(&[String], usize)}`,
  `series_{create, get, update -> bool, with_game, active}`, `tickets_{open_for_profile, cancel(&str, i64)}`,
  `profiles_{get_many(&[String]), set_in_match(&str, Option<&str>)}`. Types (with serde + Clone + Debug +
  PartialEq): `MatchRow { id, seed, players: (String, String), decks: (Vec<String>, Vec<String>),
  catalog_version, ranked: Option<bool>, mode: Option<QueueMode>, stake: Option<_>, status: MatchStatus,
  created_at: i64, finished_at: Option<i64>, clocks: MatchClocks, last_boards: Option<(Vec<LastBoardEntry>,
  Vec<LastBoardEntry>)>, glitch_boards: <same>, portraits: Option<(PortraitId, PortraitId)> }`, `MatchClocks {
  turn_deadline: Option<i64>, prompt_deadline: Option<i64>, grace_deadline: PerPlayer<Option<i64>>, ceiling_at:
  i64 }`, `MatchStatus::{Live, Finished}`, `MatchSeat { profile_id, player: PlayerId, deck: Vec<String>, portrait:
  Option<String> }`, `MatchActionRow { match_id, seq: i64, action: Action, at: i64 }`, `StartMatchInput {
  match_id, seed, catalog_version, ranked: bool, seats: (MatchSeat, MatchSeat), mode: Option<QueueMode>, stake:
  Option<_> }`, `QueueMode::Random`, `LastBoardKind::Server`, `SeriesRow { id, sides: (side, side) with
  .profile_id, catalog_version, ranked: Option<bool>, status: SeriesStatus, games: Vec<_>, next_match_id,
  pick_deadline: Option<i64>, winner, end_reason: Option<SeriesEnd>, rating_before, rating_after, updated_at: i64,
  ended_at: Option<i64> }`, `SeriesStatus::{Picking, Playing, Over}`, `SeriesSeat` (= `PlayerId`, Copy),
  `RatedGameKind::Series`, `Profile { id, in_match_id: Option<String> }`, `Ticket { id }`.
- `jackioh_engine` (parts 1, 5, 8): `FoldArgs { seed, decks, log, catalog, handicaps, dealt, last_boards,
  glitch_boards }` and `FoldResult { state, errors: Vec<_: Debug> }` by SURFACE §6.1's names (TS `ReplayInput` /
  `ReplayResult`), `ReduceResult`, `create_game`, `begin_game`, `reduce`, `legal_actions`, `view_for`, `fold`,
  `hash_state`, `mulligan_owed`, `seats_swapped`, `last_board_for`, `summarize_game`; wire `parse_aim`, `aim_key`,
  `emote_gate`, `is_emote_id`, `portrait_or_default` (all read).
- `jackioh_ai::{build_ai_deck, AiDeckOptions { banned: Vec<String>, .. }: Default}` (the brief's form).
- `crate::config` (part 18), any integer type (every use is `as i64`/`as usize`/`as u16`): `TURN_CLOCK_SECONDS`,
  `PROMPT_CLOCK_SECONDS`, `MULLIGAN_CLOCK_SECONDS`, `DISCONNECT_GRACE_SECONDS`, `MATCH_CEILING_MINUTES`,
  `MATCH_ACTIONS_PER_SECOND`, `AIM_RELAY_INTERVAL_MS`, `MATCH_VOIDED_CLOSE_CODE`, `GLITCH_BOARDS_SAMPLED`,
  `SERIES_START_GIVE_UP_SECONDS`, `SERIES_START_GRACE_SECONDS`, `SERIES_SWEEP_INTERVAL_SECONDS`,
  `SERIES_WRITE_ATTEMPTS`.
- `jackioh_cards::register_all()`; crates `uuid` (v4), `thiserror`, `tokio`, `tracing`, `axum` (Response).

## GAPS
- **`Socket`'s home.** SURFACE §11.2 writes `actor::ws_server::Socket`; TS defined it in `contracts.ts` and
  `wsServer.ts` imported it, so I defined it in `actor/contracts.rs` and my files use that one. `ws_server.rs`
  should `pub use super::contracts::Socket;` (and drop any second definition). Its contract: `Socket::new(out:
  UnboundedSender<SocketFrame>)` or `Socket::channel() -> (Socket, UnboundedReceiver<SocketFrame>)`; the transport
  writes each `SocketFrame::Text(String)` to the peer and closes on `SocketFrame::Close { code, reason }`, calls
  `socket.receive(text)` for each text frame and `socket.gone()` when the connection ends; `is_open`, `send(String)
  -> Result<(), SocketSendError>`, `close(Option<u16>, Option<&str>)` (runs the close handler once), `attach(
  SocketHandlers { message, close })`, `id`, `same`. `tests/support/socket.rs` (part 19 chunk 1) should wrap the
  same type.
- **SURFACE §11.2 is inconsistent:** handlers get `&App`, but `Registry::start` takes `&Arc<App>` (the actor
  keeps an `Arc<App>` to reach the store and the results writer). I added `Registry::bind(&Arc<App>)` and
  `Registry::app() -> Option<Arc<App>>` (a `OnceLock<Weak<App>>`); `start`, `attach`, `actor_for` and
  `run_sweeper` bind it. **`app.rs`'s `build` (and `tests/support/deps.rs`'s `test_app`) should call
  `app.matches.bind(&app)` right after making the `Arc`**, or a series pick that completes both picks before any
  socket or loop bound it fails to start its game (logged `series.start_failed`; the sweeper retries). queue.rs,
  rooms.rs and rematch.rs (other chunks) face the same `&App` → `&Arc<App>` problem and can use `app.matches.app()`.
- `RatedReason: From<SeriesEnd>` (`reason: end_reason.into()` in `plan_series_rating`), or part 31 names the variant.
- `ApiError` must be `Debug` (`AttachError` derives `Debug`).
- Not ported (SURFACE §11.3): `EngineUnavailableError`, `REQUIRED_ENGINE_EXPORTS`, `setEnginePort`,
  `injectedEnginePort`, `loadEnginePort`, `EnginePort` (engine.ts); `JoinRoomMessage` (protocol.ts);
  `createSeriesRoutes` (the route table is `app.rs`'s ROUTES: the four handlers above, TS's order and auth
  `active`). `ServerConfig`'s injection is gone: clock lengths are `crate::config` constants, so a test advances
  the paused clock by the real values (TS shrank them).

## Decisions
- **Actor shape.** `MatchActor` is a cheap `Clone` handle on `Arc<Shared>`: the match row, the seats, the clock,
  the inbox sender and a `std::sync::Mutex<Core>` (state, sockets, acks, aims, flags). One tokio task per match
  drains the inbox (`Task::{Arm, Hello, Action, Submit, Expire, Disconnect, Attach, Idle}`), one at a time, each
  run as its own spawned task and awaited, so a panic is logged `match.task.threw` and the queue goes on (TS's
  `fireAndForget`). What TS ran synchronously outside its queue (socket swap on attach, frame parsing, malformed
  answers, emote and aim relays, a socket going away, `stop`'s socket closes) runs synchronously on the caller's
  thread under the core lock, which is never held across an `.await` nor while a socket is closed (a close calls
  back into the actor). The queue task holds only a `Weak<Shared>`; sockets' handlers and aim timers too, so the
  actor goes away when the registry drops it. Lock order: core → clock; the clock never takes the core lock.
- **Clock timers.** `clock::after(ms, f)` spawns a sleeping tokio task; `Timer` aborts it on drop (TS `cancel`).
  Each callback gets its timer's id and checks it is still the countdown's timer before acting, so a callback
  already running when it was cancelled is a no-op. The clock's state is its own `Arc<Mutex<ClockState>>`; a
  callback updates it under the lock and calls `on_expire` after letting go. `on_expire` puts `Task::Expire` on
  the actor's inbox through a weak sender.
- **Time.** `clock::now_ms()` (TS `timers.now()`) is epoch ms anchored once to `SystemTime` and advanced by
  `tokio::time::Instant` (signed offset), so a paused test clock moves it only by `advance`. All of my files use it.
- `clock::match_ceiling_at(started_at)` and `clock::initial_clocks(started_at)` drop TS's `config` argument.
  registry.rs keeps its own private `initial_clocks(now, ceiling_minutes)` as TS did.
- **Protocol.** `ServerMessage` is one serde enum tagged `type`, struct variants in TS key order; `prompt`'s two
  arms are one variant with `for_you` and optional `choiceId`/`kind` (absent on the opponent's arm, as TS).
  `parse_client_message(&str) -> Result<ClientMessage, MalformedMessage>`. `joinRoom` → malformed "join a room
  with POST /api/rooms/:code/join, not over the socket". The frame-size and nonce-length checks count UTF-16 code
  units as TS's `length` did (the transport's 1009 is the byte check). `SocketErrorCode` keeps `unsupported`
  (unused now). `CLIENT_MESSAGE_TYPES` drops `joinRoom`. The client `play` whitelist has no `plague`, as TS.
  `ack.seq` and `clock.now`/deadlines are `i64`; a view's `clockMs` is the clock's `i64` clamped into `i32`.
- **Store calls.** Each TS call outside a `tx` is one `begin … commit` (`contracts::one_tx!`); the registry's
  three reads before a start (both last boards, the Glitch sample, `mode_of`) share one transaction, as do
  `rebuild`'s three. No transaction is held across a call that opens another (the fake store's lock is held for
  a whole transaction).
- **Registry.** `AttachError::{Api(ApiError), Internal(String)}` (ws_server: `Api` with `not_found` → 4404, other
  `Api` → 4403, `Internal` → 1011). Concurrent rebuilds share one `tokio::sync::OnceCell`. An engine panic in
  `start` (TS: `createGame` threw) or in a rebuild's `fold` is caught (`catch_unwind`) and answered as an internal
  error, leaving nothing written. `forget` is `pub(crate)` (the actor's `on_voided`).
- `match_actor::last_boards_of(&MatchRow) -> (Option<LastBoards>, Option<LastBoards>)`; `aim_is_public` is pub.
  An actor built with no state folds `(seed, decks, log, boards)` without `dealt`, exactly as TS did (the registry
  always passes a state).
- **Logs.** `tracing` events with an `event = "<TS name>"` field and TS's field names (`matchId`, `seriesId`, …);
  TS `info`/`warn`/`alert` are `info!`/`warn!`/`error!`. No new event names.
- **Series.** `SeriesError::{Refusal, Api, Store, Other}`; handlers turn a refusal into 400/409 as TS did and any
  other non-API error into 500 "something went wrong" with a `handler.threw` warning (http.ts's catch).
  `start_series(&App, NewSeriesInput, &mut Tx)` always writes in the caller's transaction (both TS callers passed
  one). `sweep_series(&App) -> Result<SeriesSweep, StoreError>`; `start_series_sweeper(Arc<App>) -> SeriesSweeper
  { stop() }` wraps `run_sweeper`. `slot`/`gameNo` are read as whole JSON numbers into `i32`.
