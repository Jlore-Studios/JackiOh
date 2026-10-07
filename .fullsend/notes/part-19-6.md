# Slice: part 19, chunk 6 of 6 (server 2: six match-lifecycle test ports)
BUILDS-RUN: 0

## FILES
- `crates/server/tests/actor/ws_server.rs` (new) ← `apps/server/test/match/ws-server.test.ts`
- `crates/server/tests/actor/glitch.rs` (new) ← `apps/server/test/match/glitch.test.ts`
- `crates/server/tests/actor/last_boards.rs` (new) ← `apps/server/test/match/last-boards.test.ts`
- `crates/server/tests/actor/recovery.rs` (new) ← `apps/server/test/match/recovery.test.ts`
- `crates/server/tests/actor/rooms.rs` (new) ← `apps/server/test/match/rooms.test.ts`
- `crates/server/tests/actor/series_recovery.rs` (new) ← `apps/server/test/match/series-recovery.test.ts`

One TS `describe` is one `mod`, one `it` one `#[tokio::test]` named by SURFACE §7.3 (ruling tokens
first). Every helper of each TS file is ported in TS order except those under GAPS.

## SURFACE
- §11.2/§11.3 as written: the real `Registry` on `App.matches` (`start`, `attach -> Result<(), _>`,
  `has`, `stop`, `presence_of`), `support::deps::{test_app, call}`, the engine called directly
  (`jackioh_engine::{create_game, begin_game, view_for, legal_actions, hash_state, validate_deck,
  registered_catalog}`), `Db::Fake(Arc<tokio::sync::Mutex<FakeData>>)`, one `Tx` per store call
  (`db.begin(None)` … `commit`, the `store!` macro in each file).
- §11.3's deltas are what the ports hold the server to: the token from `?token=` only, `jackioh.v1`
  echoed, 1009 past `MAX_FRAME_BYTES`, the legacy `{ deckIndex }` body gone.
- Every port value is built from TS's own object literal with `serde_json::from_value(json!(…))`
  and read back through `serde_json::to_value`, so the tests depend on TS's JSON (camelCase keys),
  not on Rust field spellings; direct field access only on `MatchSnapshot` (`phase`,
  `mulligan_owed`, `pending_for`, `active`, `result`, by §4.2) and `CardDef` (`token`, `tags`,
  `set`, `type_`, part 1's).

## DEPENDS-ON (names I call that other parts write)
- `support::deps` (part 18): `test_app() -> Arc<App>` with E2E mode **off** (TS `createTestDeps`'
  default); `test_app_with(TestAppOptions { e2e: bool, db: Option<Db> }) -> Arc<App>` with
  `TestAppOptions: Default` (`e2e` = TS `deps.e2e = true`; `db` = a second "process" over the same
  store, TS `createTestDeps({ store, timers })`); `call(&Arc<App>, method: &str, path: &str,
  token: Option<&str>, body: Option<Value>) -> (u16, HeaderMap, Value)`; `add_user(&Arc<App>,
  user_id: &str, email: &str) -> String` (TS `FakeAuth.addUser`: the token that verifies as a new
  user; the test `Auth` must accept users added at run time).
- `support::engine` (part 19, another chunk): `create_fake_engine(FakeEngineOptions { mulligan:
  false })` returning a `'static` value (unit or a guard; held for the test as `Box<dyn Any>`)
  that installs the test catalog and scripts on this thread (SURFACE §8 override);
  `fake_deck(&[&str]) -> Vec<String>` (20 test-catalog ids, the extras first). Cards used:
  `test-prompt-self`, `test-prompt-enemy` (prompts answerable with `selection: [{pick:"none"}]`),
  `test-lethal`, `test-glitch-swap`, `test-glitch-void`, fillers `test-card-0`…`test-card-19`.
- `support::socket` (part 19, another chunk): `create_fake_socket() -> FakeSocket` and, on it,
  `socket() -> actor::ws_server::Socket` (the end handed to `attach`), `of_type(&str) -> Vec<Value>`
  (every frame of that `type`, parsed, oldest first), `receive_json(Value)`, `clear()`, `drop()`
  (TS `drop`: the transport drops, 1006), `is_open() -> bool`, `close_code() -> Option<u16>`; all sync.
- `actor::registry::Registry` (part 19): beyond SURFACE §11.2, `async fn actor_for(&self, &Arc<App>,
  &str) -> Result<MatchActor, ApiError>` (TS `actorFor`: rebuilds by `fold`, one fold for
  concurrent callers) and `fn live(&self) -> Vec<String>`.
- `actor::match_actor::MatchActor` (part 19): a `Clone` handle with async `idle()`,
  `submit(PlayerId, &str, ActionBody) -> ServerMessage` (Serialize: `{type:"ack",nonce,seq}` /
  `{type:"error",code,message,nonce}`), `view_for(PlayerId) -> PlayerView` (with `clockMs`),
  `snapshot() -> MatchSnapshot` (`PartialEq + Debug`), `engine_state() -> GameState`,
  `clocks() -> MatchClocks` (Serialize), `attach(PlayerId, Socket)`, `detach(PlayerId)`.
- `actor::ws_server::{WS_PATH, WS_SUBPROTOCOL}`; `actor::protocol::MAX_FRAME_BYTES`;
  `actor::rooms::e2e_room_seed_count()` (any integer); `actor::engine::deal_random_deck(&str) ->
  Vec<String>` (TS `EnginePort.dealRandomDeck`, R258).
- `api::results` (part 19): `record_result(&App, RecordResultInput)`, `void_match(&App,
  VoidMatchInput)`, `reap_stuck_matches(&App) -> Result<Vec<String>, _>`; every error `Debug`.
- `api::series` (part 19): `start_series(&App, NewSeriesInput) -> Result<SeriesRow, _>` (TS's
  default-store form), `sweep_series(&App) -> Result<SeriesSweep, _>` (Serialize as `{timedOut,
  started, abandoned}`), `ensure_series_game(&App, &SeriesRow) -> Result<(), _>`.
- `api::series_rules` (part 19): `pick_deck(&SeriesRow, SeriesSeat, slot, now: i64, Option<_>) ->
  Result<SeriesRow, _>`, `game_ended(&SeriesRow, Winner, GameOverReason, i64, &str) ->
  Result<SeriesRow, _>`.
- `ranked::glicko2` (part 18): `Glicko { rating, deviation, volatility }` (f64),
  `rate_game(&Glicko, &Glicko, 1.0) -> { a: Glicko, b: Glicko }`.
- `config` (part 18): `GLITCH_BOARDS_SAMPLED`, `MATCH_VOIDED_CLOSE_CODE`,
  `WS_MAX_CONNECTIONS_PER_ADDRESS`, `DISCONNECT_GRACE_SECONDS`, `MATCH_CEILING_MINUTES`,
  `PROMPT_CLOCK_SECONDS`, `TURN_CLOCK_SECONDS`, `ROOM_CODE_LENGTH`, `ROOM_CODE_TTL_SECONDS`,
  `MAX_SAVED_DECKS`, `MAX_SAVED_TRIOS`, `SERIES_START_GRACE_SECONDS`,
  `SERIES_START_GIVE_UP_SECONDS`, `RATING_DEVIATION_START`, `RATING_VOLATILITY_START` (any numeric
  type: every use casts with `as`).
- `db::fake::FakeData` (part 20.3): `seed_profile(Value)`, `tables.{matches, match_actions,
  results, profiles, last_boards, game_records, rated_games, rooms, series, decks}` (Serialize rows
  in TS's JSON), `on_call: Option<Arc<dyn Fn(&str) -> Result<(), StoreError> + Send + Sync>>`
  called with TS's method names (`"matches.create"`, `"lastBoards.put"`, `"series.update"`,
  `"profiles.setGlicko"`).
- `db::store` (part 20.4): `Db: Clone`, `StoreError::Other(String)`, `CollectionEntry`, `SeriesRow`,
  every input type `Deserialize`; the `Tx` methods of part 20's notes (`matches_get`,
  `matches_create`, `matches_mode_of`, `matches_finish`, `last_boards_get/put`, `results_get_by_match`,
  `series_get/create/update/active_for`, `profiles_get_by_id/set_in_match`, `decks_get/upsert`,
  `trios_upsert`, `collection_upsert_quantities`, `tickets_open_for_profile`); a cap or epoch-ms
  argument is passed `as _`.
- `app::router(Arc<App>) -> axum::Router` serving `/ws/match` (SURFACE §11.2).

## GAPS
- Not ported (no seam on the Rust server, SURFACE §11.3 removed TS's injected ports):
  - rooms: "R149 retries past a code that is already in use and mints the next one" and "R149 gives
    up after a bounded number of collisions and says no code is available" — they script
    `Ids.code` to collide; the Rust server mints codes itself. With them goes the `roomIds` helper.
    Part 31: an injectable code source (or a test hook in `actor::rooms`) would bring them back.
  - series-recovery: "R263 a pick that loses the compare-and-set is re-applied to the row that won",
    "R263 a write that keeps losing is refused with 409 and writes nothing", "R263 a result whose
    series write loses the compare-and-set re-reads the series and records the game" — they replace
    `store.series.update`; `FakeData.on_call` can only fail a call, not change its answer or land a
    rival write first. A fake-store hook that may mutate the tables before a named method runs
    would bring them back. `processIds` (prefixed `Ids`) has no Rust counterpart.
  - ws-server: the `subprotocolToken` unit test (SURFACE §11.3 drops the path); ported instead as
    `reads_the_token_from_the_query_and_never_from_the_protocol_header`.
- Behaviour the ports assume of other parts (beyond names):
  - `Registry::start` answers `Err` (never panics) for decks the engine refuses, carrying the
    engine's text (`deck must hold exactly …`): `create_game` panics, so start validates first.
  - `Registry::attach`'s refusal `Debug` contains `no such match`.
  - The actor answers `idle()` only after every socket frame received before it, and, after
    `tokio::time::advance` (plus one yield), after the expiry a fired timer queued (TS's one
    serialized queue).
  - The server's epoch-ms clock moves with tokio's paused clock: the tests read "now" as the
    stamp the server wrote (`createdAt` of the match or series row) plus tokio time elapsed.
  - The server logs TS's event names (`match.started`, `match.rebuilt`, `match.fold.errors`,
    `match.voided`, `series.start_failed`, `series.game_unrecorded`) as a `tracing` field
    `event` (or as the message); the tests read `tracing_subscriber`'s JSON lines with
    `flatten_event(true)`, `warn!` as level `WARN`.
  - `support::engine`'s test cards cost (0), so two can be played in one turn (TS's fake kept no
    mana), and `test-card-11`…`test-card-17` survive `freeze_last_boards` (in the test catalog).
  - `ws_server`: the per-address count reads `ConnectInfo<SocketAddr>` (the tests serve with
    `into_make_service_with_connect_info::<SocketAddr>()`); if the count is process-wide, only this
    file opens real sockets, and its tests run one at a time (`SERIAL`).
  - `actor::rooms`' R143 seed map is process-wide; `rooms.rs`'s tests take turns (`ROOMS`).
- Lines over rustfmt's 110 columns: formatting only; part 31/35 run `cargo fmt`.

## Decisions
- No scripted engine (SURFACE §11.3): the "scripted" blocks run the real engine with
  `support::engine`'s cards installed. Three consequences, each handled in the test file, not the
  server: (1) the real game opens on both mulligans — `open_turn_one` answers them keeping the whole
  hand before any socket attaches (where TS's scripted game began), and log offsets / `seq`s are
  shifted by the rows it wrote (`opening`); (2) the deal is shuffled — `seed_dealing` searches
  `<seed>-<k>` for the first seed whose opening hands hold the scripted cards (`fakeDeck` put them
  first); (3) instance ids are the engine's — a card is found by its definition (`hand_card`,
  `in_hand`; TS's `firstInHand`/`p1-h0`).
- No fake match directory: TS's `deps.matches.started` is the store's match rows past their `open`
  reservation (`started`), plus `match.started` log lines where TS counted repeat starts; TS's
  `matches.start = throw` is `FakeData.on_call` failing `"matches.create"`; a "process" restart is a
  new app over the same `Db` after the old one's actors are stopped (`boot`).
- No injected results writer: TS's `recordResult` spy is the real `record_result`, read back off
  `tables.results` (both profiles seeded so it can write; `lossOf(winner)` is the row's reason and
  winner); `deps.games = { summarize: () => null }` is dropped (a void summarises nothing).
- Real decks where the real registry, engine or validator would refuse TS's: rooms' `DECK` and
  trios, series trios and the Conquest void test's trios are runs of `DECK_SIZE` playable catalog
  ids (disjoint within a trio, R253), and every rooms profile owns every playable card (L5); TS's
  names (`"alice deck 2"`, `"host 1"`, …) are kept, so R331's "nothing names alice" still means
  something.
- The engine's `fold` spy (R678) became: the rebuilt state's `glitchBoards` equals the row's frozen
  pair, and the rebuilt hash equals the live one.
- `attach` answers `()` (SURFACE §11.2), not TS's seat: the seat is read off the socket's first
  view (`viewer`).
- Legacy `{ deckIndex: 0 }` bodies (SURFACE §11.3 drops them) became `{ mode: "bo1", deckId }` of
  the caller's saved deck; the R264 refusal loop drops its legacy entry.
- Minted seeds and ids are the server's own: R143's "mints a seed" asserts a non-empty seed (TS
  `/^seed-/`), series' "a new id" asserts `!= FIRST_MATCH` (TS `/^b-/`).
- The ws-server cap: TS lowered it per listener (`maxConnectionsPerAddress: 2`, `1`); Rust has the
  one constant, so the tests open `WS_MAX_CONNECTIONS_PER_ADDRESS` sockets (one account each, over
  `cap/2` live matches, so the registry replaces none). The client is a hand-rolled RFC 6455 client
  over `TcpStream` (no WebSocket client crate on SURFACE §2's list); a stream that ends without a
  close frame reads as 1006, as `ws` reported it.
- Ratings are compared as `f64` (`as_f64`), never as JSON numbers (`1000` ≠ `1000.0` in serde_json).
- A reaped match's ceiling "one millisecond ago" is written as `ceilingAt: 0` (long past by any
  clock), so the test needs no reading of the server's clock there.
