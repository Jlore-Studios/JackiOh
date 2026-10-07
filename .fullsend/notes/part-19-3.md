# Slice: part 19, chunk 3 of 6 (#407): server 2 — rooms, the WebSocket adapter, and the queue/rematch/game-records tests
BUILDS-RUN: 0

## FILES
- `crates/server/src/actor/rooms.rs` ← `apps/server/src/match/rooms.ts` (whole): handlers `create` (`POST /api/rooms`)
  and `join` (`POST /api/rooms/:code/join`), `e2e_room_seed_count`, the R143 seed map and every private helper.
- `crates/server/src/actor/ws_server.rs` ← `apps/server/src/match/wsServer.ts`: `WS_PATH`, `WS_SUBPROTOCOL`,
  `WS_CLOSE: WsClose`, `Socket` (+ `SocketFrame`), `socket_from_ws`, `handle_match_socket` (TS
  `createMatchSocketHandler`), `AttachOptions` (+ `for_app`), `handle` (SURFACE §11.2), `handle_with` (TS
  `attachWebSocketServer`'s upgrade listener), `close_all` (TS `AttachedSockets.close`).
- `crates/server/tests/api/game_records.rs` ← `test/api/game-records.test.ts` (6 of 7 `it`s; below).
- `crates/server/tests/api/rematch.rs` ← `test/api/rematch.test.ts` (18 of 19 `it`s; below).
- `crates/server/tests/api/queue.rs` ← `test/api/queue.test.ts` (all 35 `it`s, two recast; below).

## SURFACE
- §11.2: `pub async fn handle(app: Arc<App>, req: axum::extract::Request) -> Response`; `Socket` lives at
  `actor::ws_server::Socket`, as §11.2's `Registry::attach` names it (TS's `Socket` type was in contracts.ts:
  `actor/contracts.rs` must not define a second one).
- §11.3: the token comes from `?token=` only (`subprotocolToken` and the `Authorization` path are dropped); `jackioh.v1`
  is echoed when offered (`WebSocketUpgrade::protocols`); a frame over `MAX_FRAME_BYTES` closes with 1009 (axum's
  `max_message_size`/`max_frame_size` plus one close frame on the capacity error); no heartbeats; the origin check
  (raw 403) and the per-address cap (raw 429) run before the upgrade, in TS's order.
- Handler shape `pub async fn <name>(app: &App, req: Req) -> ApiResult`, except `join`: see GAPS.

## DEPENDS-ON (names I call that other parts write)
- `crate::app` (18.1): `App { env, db, auth, matches, catalog.version }`, `now_ms() -> i64`, `browser_origins(&Env) ->
  Vec<String>`, `ROUTES` (tests: tuples `(method, path, AuthLevel, Handler)`).
- `crate::api::http` (18.2): `Req { caller: Option<Caller { profile, .. }>, params, body }`, `ApiError { code, message,
  details, retry_after_ms }` (Debug), `ApiErrorCode::{NotFound, Conflict, AlreadyInMatch, UpdateRequired, Unauthorized,
  Unavailable, Internal}` (PartialEq), `ApiResult`, `json(u16, Value) -> Response`, `assert_active(&Profile) ->
  Result<(), ApiError>`, `client_address(&HeaderMap, Option<&str>, usize) -> String`, `rate_limit_address(&str, u32) ->
  String` (TS's default `IPV6_RATE_LIMIT_PREFIX_BITS` passed explicitly), `AuthLevel::Active`, and
  `impl From<StoreError> for ApiError` (500 `internal` "something went wrong", as TS's router answered a store throw):
  rooms.rs uses `?` on every store call.
- `crate::api::crypto` (18.2): `normalize_code(&str) -> String`, `is_well_formed_code(&str, usize) -> bool`,
  `random_code(usize) -> String`.
- `crate::api::decks` (18.x): `read_mode_choice(&Value) -> Result<ModeChoiceInput, ApiError>`, `freeze_choice(&App, &str,
  &ModeChoiceInput) -> Result<FrozenChoice, ApiError>`, `assert_not_in_series(&App, &str) -> Result<(), ApiError>`,
  `ModeChoiceInput::{Bo1 { .. }, Bo3 { .. }, Random}`, `FrozenChoice::{Bo1 { deck: FrozenDeck }, Bo3 { trio: FrozenTrio },
  Random}`.
- `crate::api::queue` (19.1): `seed_override_of(&App, &Value) -> Result<Option<String>, ApiError>`.
- `crate::api::series` (19.2): `start_series(&App, &NewSeriesInput, &mut Tx<'_>) -> Result<SeriesRow, E>` with
  `ApiError: From<E>` (identity when E is `ApiError`).
- `crate::api::series_rules` (19.1): `NewSeriesInput { series_id, first_match_id, sides: (NewSeriesSide, NewSeriesSide),
  seed_base, catalog_version, ranked }`, `NewSeriesSide { profile_id, trio: FrozenTrio }`.
- `crate::actor::engine` (19.2): `deal_random_deck(&str) -> Vec<String>`.
- `crate::actor::contracts` (19.2): `SocketHandlers { message, close }`, callable as `(h.message)(text: String)` and
  `(h.close)()`, `Send + Sync` (e.g. `Box<dyn Fn(String) + Send + Sync>`); `Socket::attach` takes it by value.
- `crate::actor::protocol` (19.2): `encode(&ServerMessage) -> String`, `error_message(SocketErrorCode, &str, Option<&str>)`
  returning `ServerMessage` or a type with `From<_> for ServerMessage` (called as `encode(&error_message(..).into())`),
  `SocketErrorCode::{Forbidden, Malformed}`, `MAX_FRAME_BYTES: usize`.
- `crate::actor::registry` (19.2): `Registry::{start(&self, &Arc<App>, StartMatchInput) -> Result<(), ApiError>,
  attach(&self, &Arc<App>, &str, &str, Socket) -> Result<(), AttachError>, has, stop, presence_of -> Option<PerPlayer<bool>>}`;
  `AttachError::Api(ApiError)` for TS's thrown `ApiError` (404 → 4404, else 4403) and `Display` for the rest (1011).
- `crate::auth` (18.1): `Auth::verify(&str) -> Result<AuthUser { user_id, .. }, AuthError>` (any `Err` is TS's `null`).
- `crate::config` (18.3): `ROOM_CODE_LENGTH: usize`, `ROOM_CODE_TTL_SECONDS: i64`, `WS_MAX_CONNECTIONS_PER_ADDRESS: usize`,
  `DEFAULT_TRUSTED_PROXY_HOPS: usize`, `IPV6_RATE_LIMIT_PREFIX_BITS: u32`; `Env.trusted_proxy_hops: usize`.
- `crate::db::store` (20.4): `Db::begin(None)`, `Tx::commit`, `Tx::{rooms_get(&str) -> Option<Room>, rooms_create(&Room) ->
  bool, rooms_claim(&str, &str, &str, i64) -> Option<Room>, profiles_get_by_id, profiles_get_by_user_id,
  profiles_set_in_match(&str, Option<&str>), series_active_for, matches_discard_open}`; `Room { code, host_profile_id,
  mode, host_deck, host_portrait: Option<Option<String>>, host_trio: Option<FrozenTrio>, catalog_version, created_at: i64,
  expires_at: i64, guest_profile_id: Option<String>, match_id: Option<String> }`; `FrozenDeck { cards, portrait:
  Option<Option<String>> }` (20.4's `absent_or_null`; chunk 1 assumed `Option<String>` — the `.flatten()`s in rooms.rs go
  if 20.4 changes it); `MatchSeat { profile_id, player, deck, portrait: Option<String> }`; `StartMatchInput { match_id, seed,
  catalog_version, ranked, seats: (MatchSeat, MatchSeat), mode: Option<QueueMode>, stake: Option<_> }`; `QueueMode::{Bo1,
  Bo3, Random}` (`Copy`, `PartialEq`, `Serialize`, `as_str()`); `StoreError: Display`, `StoreError::Other(String)`.
- `jackioh_engine::{pick_portrait_from_seed, PlayerId, DEFAULT_PORTRAIT}` (part 1's wire, frozen).
- Tests: `crate::support::deps` (18.1): `test_app().await -> Arc<App>` with `env.e2e == true` (the seed tests send R143's
  `seed`), `call(&Arc<App>, method, path, Option<&str>, Value) -> (u16, HeaderMap, Value)` (`Value::Null` = no body),
  `add_user(&App, user_id: &str, email: &str) -> String` (sync; TS `deps.auth.addUser`, the token). `crate::db::fake`
  (20.3): `FakeData::seed_profile(&mut self, Value)`, `FakeData.tables.{profiles, tickets, matches, series, collection,
  game_records, match_actions}` (Serialize rows), `FakeData.on_call: Option<Box<dyn Fn(&str) -> Result<(), StoreError> +
  Send + Sync>>` charged with TS's port names (`"matches.create"`, `"matches.discardOpen"`, `"series.create"`,
  `"profiles.setInMatch"`, `"gameRecords.insert"`). `api::queue::{try_pair(&Arc<App>) -> Result<_, ApiError>,
  e2e_seed_count() -> usize}`, `api::results::record_result(&Arc<App>, RecordResultInput) -> Result<_, _>`,
  `api::ranked::rate_ranked_game(&mut Tx, &App, &RankedGameInput) -> Result<RatedGameRow, _>` (by reference; chunk 1
  assumed by value — one call site), `api::game_records::record_live_game(&App, &str) -> Option<GameRecord>`,
  `api::collection::grant_entire_catalog(&App, &str, Option<&str>) -> Result<(), _>`, `api::rematch::{STAKE_NORMAL,
  STAKE_DOUBLE}` (Serialize, PartialEq), `config::{rating_window(f64) -> f64, MAX_SAVED_DECKS, MAX_SAVED_TRIOS,
  REMATCH_OFFER_TTL_MS}`; every input built from TS's JSON (`Ticket`, `MatchRow`, `SeriesRow`, `SavedDeck`, `SavedTrio`,
  `RecordResultInput`, `RankedGameInput`, `ProfileStatus`) needs `Deserialize`, every row read back `Serialize`.

## GAPS
### SURFACE conflicts for part 31
- Handlers are `(app: &App, req)` but `Registry::start` takes `&Arc<App>`: `rooms::join` takes `app: &Arc<App>` (as chunk 1's
  `queue::enqueue` and `rematch::offer_rematch` do); `rooms::create` keeps `&App`. `h!` must hand handlers an `&Arc<App>`
  (deref coercion then serves both shapes).
- `Socket` is defined here (SURFACE's path); if `actor/contracts.rs` also defines `Socket`, delete that copy (or make it
  `pub use crate::actor::ws_server::Socket`). `SocketHandlers` is taken from contracts.rs.
- `ws_server::CLOSE_HANDSHAKE_TIMEOUT` (30 s: the `ws` library's own close timeout, which TS inherited unstated) and the
  RFC close codes 1000/1001/1009 are named constants in ws_server.rs, not config.rs (CLAUDE.md rule 9); move the timeout
  to config.rs if rule 9 is read as covering library defaults.
### Not ported
- rooms.ts `createRoomRoutes()` (the table is `app::ROUTES`: `POST /api/rooms` active → `actor::rooms::create`,
  `POST /api/rooms/:code/join` active → `actor::rooms::join`).
- wsServer.ts `subprotocolToken`, the `authorization` header token path (SURFACE §11.3), the `path` option (the router
  routes `/ws/match` here), and the types `UpgradeRequest`, `MatchSocketDeps`, `UpgradableServer`, `Duplexish`,
  `AttachedSockets` (axum's request and `close_all` replace them).
- game-records.test.ts: "nor without a recorder" (the second half of "files nothing for a match no ticket…") and "is
  bound at the composition root" — Rust has no optional `GameRecorder` (chunk 1: the record is always filed under
  `catalog_version()`); `loadCurrentPatch`'s temp-file cases (the patch is compiled in; the list-order half is kept).
- rematch.test.ts "keeps a seat taken during the start in its game, with its tickets": it needs B to take another game
  between the pre-check and the flagging, which TS arranged by replacing `deps.matches.start`. No Rust seam can interleave
  there (`on_call` sees only a method name and cannot write the store it is called from); part 35 may add a test hook.
### Assumptions others must meet
- Every server clock reads `app::now_ms()` (or the same tokio-anchored formula): rematch's offer TTL and the queue's
  rating window are tested by `tokio::time::advance`.
- The upgrade's peer address is `ConnectInfo<SocketAddr>` (18.1's `serve`); without it the count falls back to the
  forwarded entry or `client_address`'s unknown address.
- `too_large` recognises tungstenite's capacity error by its text ("Message too long" / "Space limit exceeded"); the
  error type belongs to a crate the server does not name.
- The fake store's `tickets_claim_pair` must not charge `"matches.create"` (queue.rs's failing-start test refuses that
  call to make the registry's start fail after the claim).

## Decisions
- `Socket` is a cloneable handle (`Arc`) with an outgoing channel of `SocketFrame::{Text, Close { code, reason }}` and
  three constructors: `new(out, incoming)` (chunk 1's support socket: a reader task forwards client text to the handlers
  and runs the close handler when `incoming`'s sender drops), `detached(out)` (the axum pump calls `receive` /
  `transport_closed` itself) and `channel()`. `close()` runs the close handler once, at once (TS's fake socket and `ws`'s
  close event alike); `send` after close goes nowhere (as `ws` on a closing socket). Equality is identity (TS `===`).
- The axum pump is one `tokio::select!` loop over `ws.recv()`, the outgoing channel and the closing-handshake timeout (no
  `futures` crate to split the socket); a binary frame is answered `malformed` directly, as TS's adapter did.
- Per-address counts and live sockets are a process static keyed by the `App`'s address (TS kept them per
  `attachWebSocketServer` call); an `AddressSlot` guard releases the count when the connection ends or the handshake
  never completes. `handle` reads its options from the app (`AttachOptions::for_app`: `browser_origins`, the env's hop
  count); `handle_with` lets a test lower `max_connections_per_address`.
- rooms.rs: ids are `uuid::Uuid::new_v4()` and seeds 16 `getrandom` bytes as hex (TS `systemIds`); codes are
  `api::crypto::random_code` (TS `systemIds.code`). Each TS store call is its own short transaction; `startSeries` and
  its cancels share one, committed before the cleanup runs, and the cleanup's own failure is logged
  `room.series_cleanup_failed`. TS's two `setInMatch` calls stay two transactions. TS's thrown wiring faults (a non-bo1
  choice in a bo1 room, a Conquest room with no trio) log `handler.threw` and answer 500 "something went wrong".
- Logs: `tracing` with TS's event names (`event = "room.created"`) and TS's camelCase keys; `alert` is `error!`.
- Tests: driven only through SURFACE's surfaces (`test_app`, `call`, `Registry::start/attach/has/presence_of/stop`, `Tx`
  methods) and the wire (protocol frames over in-memory `Socket`s), with TS's literals built through serde (`from(json!)`)
  and rows read back as JSON. Real decks everywhere: disjoint `DECK_SIZE` slices of the catalog's playable ids, and every
  player owns the whole catalog. A started match is read off its row where TS read the fake directory's `started`.
  `#[tokio::test(start_paused = true)]` throughout; `tokio::time::advance` is TS's `timers.advance`.
- game_records.rs ends its games by a concession after both mulligans (no scripted `test-lethal`): the record's
  `reason` is `concede` and its opening hands are the real deal's, so the record is checked field by field (id, source,
  mode, patch = `catalog_version()`, pilots, winner, reason, decks, nothing played) rather than against a literal.
  Log lines are read off a thread-local `tracing` JSON subscriber.
- queue.rs: the legacy `{ deckIndex }` body is gone (SURFACE §11.3), so R165's "nothing saved" uses a deck id never
  saved (`DECK_GONE`, still `loadout_invalid`, still a sentence about a deck) and its control saves under that same id;
  R257's legacy half asserts the 400. "Declares both queue mutations `active`" reads `app::ROUTES`. The failing start is
  a store fault on `"matches.create"` (bo1) or on the call right after `"series.create"` (bo3, one shot so the cleanup runs).
  R253's expected issues are read off the refusal (first issue's rule, message = that issue's, the deck names in it)
  instead of being recomputed through the adapter. Tests that send a seed or count R143's map hold `SEED_MAP`, since the
  map is one process static shared by the whole test binary.
- rematch.rs: every test names its own profiles and matches (`names(tag)`) instead of clearing the process-wide offer map
  (TS's `clearRematchOffers()` would wipe a test running beside it). Presence is real: sockets attached to a live actor,
  `transport_closed()` for TS's `drop()`. The foreign-key test checks the store's call order (`matches.create` before
  `profiles.setInMatch`) where TS emulated the key by replacing `setInMatch`.
