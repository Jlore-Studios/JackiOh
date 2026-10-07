# Slice: part 19 (server 2), chunk 5 of 6: the match actor's tests (actor, aim, clock, dealt-deck, engine.real)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All five were empty placeholders on `staging`; all are full ports. No `todo!`, `unimplemented!` or `// TODO`.
- `crates/server/tests/actor/match_actor.rs` ← `apps/server/test/match/actor.test.ts`: every `it` (46), one `mod` per
  `describe`, in TS order; plus a `tracing` log recorder (`log_capture`) and a minimal RFC 6455 client (`WsClient`).
- `crates/server/tests/actor/aim.rs` ← `apps/server/test/match/aim.test.ts` (8 `it`s).
- `crates/server/tests/actor/clock.rs` ← `apps/server/test/match/clock.test.ts` (22 `it`s, two `mod`s).
- `crates/server/tests/actor/dealt_deck.rs` ← `apps/server/test/match/dealt-deck.test.ts` (3 `it`s).
- `crates/server/tests/actor/engine_real.rs` ← `apps/server/test/match/engine.real.test.ts` (5 `it`s).

## SURFACE
- §11.2 as frozen: `App.matches: Registry` with `start(&Arc<App>, StartMatchInput)`, `attach(&Arc<App>, &str, &str,
  actor::ws_server::Socket) -> Result<(), AttachError>`, `has`, `stop`, `presence_of -> Option<PerPlayer<bool>>`;
  `app::router(Arc<App>)`; `support::deps::test_app()`; `Db::Fake(Arc<tokio::sync::Mutex<FakeData>>)`, `Db::begin`,
  `Tx::commit` and the `Tx` method names of part 20's notes.
- §11.3: no engine port, no stub clock, no stub results writer. Every expiry is a deadline passing on tokio's paused
  clock (`#[tokio::test(start_paused = true)]`, `advance(ms)` = `tokio::time::sleep` + yields, so timers fire in
  deadline order as TS's manual timers did); every ending goes through the real `api::results` over the fake store.
- §5.1: every frame, view, row and clock snapshot is read as TS's JSON (`serde_json::to_value`), so the tests depend
  on the wire shapes, not on Rust field names. Inputs (`StartMatchInput`, `MatchRow`, `Ticket`, `FoldArgs`, `Aim`,
  `ActionBody`, a cut-down `PlayerView`) are built from TS's object literals with `serde_json::from_value`.
- §7.3/§15: test names are the `it` titles snake_cased, R-ids moved to the front as tokens (`r79_…`, `r79_r146_…`).

## DEPENDS-ON (names I call that other chunks write; the shape I assumed)
- `jackioh_server::actor::match_actor` (chunk 19.2): `MatchActor` with `attach(&self, PlayerId, Socket)` (sync),
  and async `idle()`, `snapshot() -> MatchSnapshot { turn: i32, active, pending_for: Option<PlayerId>,
  mulligan_owed: Vec<PlayerId>, phase: Phase, result: Option<GameResult>, .. }`, `view_for(PlayerId) -> PlayerView`,
  `engine_state() -> GameState`, `clocks() -> MatchClocks`, `submit(PlayerId, &str, ActionBody) -> ServerMessage`
  (Serialize), `stop()`. `aim_is_public(&Aim, PlayerId, &PlayerView) -> bool`.
- `jackioh_server::actor::registry::Registry::actor_for(&self, &Arc<App>, &str) -> Result<MatchActor, E: Debug>`
  (TS `actorFor`; not in SURFACE's list, the TS registry has it and the tests need it).
- `jackioh_server::actor::clock` (chunk 19.2): `create_match_clock(CreateMatchClockInput) -> MatchClock`,
  `initial_clocks(i64) -> MatchClocks`, `match_ceiling_at(i64) -> i64` (no `ServerConfig`: the numbers are
  `crate::config`'s), and **`now_ms() -> i64`, the server's epoch-ms clock, which must follow tokio's paused time**
  (an epoch base plus `tokio::time::Instant` elapsed), since every deadline assertion compares against it.
- `jackioh_server::actor::contracts` (chunk 19.2): `CreateMatchClockInput { started_at: i64, on_expire:
  Arc<dyn Fn(ClockExpiry) + Send + Sync> }`; `ClockView { turn: i32, active: PlayerId, pending_for: Option<PlayerId>,
  mulligan_owed: Vec<PlayerId>, over: bool }`; `ClockExpiry::{Turn { player }, Prompt { player }, Mulligan,
  Grace { player }, Ceiling }` deriving `Clone, Debug, PartialEq`; `MatchClock` methods on `&self`: `sync(ClockView)`
  (by value), `start_grace(PlayerId, Option<i64>)`, `clear_grace(PlayerId)`, `snapshot() -> MatchClocks`,
  `remaining_for(PlayerId) -> Option<i64>`, `stop()`.
- `jackioh_server::actor::engine` (chunk 19.2): `create_game(&CreateGameArgs) -> GameState` (panics as the engine
  does), `begin_game`, `reduce(&GameState, &Action) -> ReduceResult`, `legal_actions`, `view_for`, `hash_state`,
  `snapshot(&GameState) -> MatchSnapshot`, `deal_random_deck(&str) -> Vec<String>`, `summarize_game(&FoldArgs) ->
  Option<GameSummary>`.
- `jackioh_server::actor::protocol` (chunk 19.2): `parse_client_message(&str) -> ClientMessage` with a variant
  `ClientMessage::Aim { aim: Option<Aim> }` (malformed is a variant of the same enum, not an `Err`).
- `jackioh_server::actor::ws_server` (chunk 19.3): `Socket`, `WS_PATH: &str`, `WS_CLOSE: WsClose { unauthorized,
  forbidden, not_found, internal }` (integers `i64::from` takes); `handle` behind `app::router`, the binary-frame
  error text "text frames only: every message is JSON", the error frame before each refusal close.
- `crate::support::engine` (chunk 19.1): `create_fake_engine()` installs the scripted cards and their catalog on the
  calling thread (testkit override) and is called before any game is made; `fake_deck(&[&str]) -> Vec<String>` (TS
  `fakeDeck`: the extras then `test-card-<n>` fillers to 20). Assumed: the scripted cards cost 1 (playable on turn 1
  and two on turn 2), `test-prompt-self`/`test-prompt-enemy` open a prompt whose legal answer is in the holder's
  `legal`, `test-lethal` ends the game by `hero-death` for its player.
- `crate::support::socket` (chunk 19.1): `create_fake_socket() -> FakeSocket` with `socket() -> Socket` (the half
  the actor holds), `sent() -> Vec<String>` (every frame so far, in order), `receive(&str)`, `drop()` (TS `drop`:
  close code 1006, the actor told), `clear()`, `is_open() -> bool`. Every other read is computed from `sent()`.
- `crate::support::deps::test_app()` (part 18): the fake store with the E2E fixtures (accounts `e2e-p1`, `e2e-p2`,
  `e2e-pending`, tokens `e2e-token-*`), not on paused-time-sensitive timers.
- `jackioh_server::db::fake::FakeData::seed_profile(Value)` and `FakeData.tables.tickets: Vec<Ticket>` (part 20.3);
  `Tx::{matches_actions, matches_get, matches_create(&MatchRow), results_get_by_match, profiles_get_many(&[String]),
  profiles_get_by_user_id, profiles_set_in_match(&str, Option<&str>)}` (part 20.4).
- `jackioh_server::ranked::glicko2::{Glicko { rating, deviation, volatility }, rate_game(Glicko, Glicko, f64) ->
  { a: Glicko, b: Glicko }}` (part 18; TS `rateGame(a, b, scoreA)`), `config::{TURN_CLOCK_SECONDS,
  PROMPT_CLOCK_SECONDS, MULLIGAN_CLOCK_SECONDS, DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES,
  MATCH_ACTIONS_PER_SECOND, AIM_RELAY_INTERVAL_MS}` (integers `Into<i64>`), `RATING_DEVIATION_START`,
  `RATING_VOLATILITY_START` (`Into<f64>`).
- Engine (frozen or part 5): `create_game`, `begin_game`, `view_for`, `reduce`, `mulligan_owed`, `hash_state`,
  `registered_catalog`, `FoldArgs` (Deserialize), `emote_gate(&[i64], i64) -> EmoteGate { ok, retry_after_ms, .. }`,
  `EMOTE_IDS`, `EMOTE_WINDOW_MS`/`EMOTE_COOLDOWN_MS: i64`, `EMOTE_WINDOW_MAX: usize`, `DEFAULT_PORTRAIT`, `COIN_DEF_ID`.

## GAPS
- Not ported: TS `stubClocks` and `resultWriter` (actor.test.ts) — the Rust actor takes no `CreateMatchClock` and no
  `RecordResult` to inject; their tests drive the real clock and writer instead (see Decisions). TS `fakeWs` is
  replaced by `WsClient` over a real listener (no `socketFromWs`/`createMatchSocketHandler` in Rust).
- TS's `timers.pending` (the manual timers' queue length) has no tokio counterpart; the four assertions on it became
  "the deadlines are null" plus "advancing far fires nothing".
- TS's late on-demand mulligan alarm (R268 "an expiry reads who still owes when it runs") cannot be fired with the
  real clock; that test now asserts the clock fires once and a second window writes nothing.
- If the Rust clock reads `SystemTime` (not paused) or the actor's `now` differs from `actor::clock::now_ms()`, every
  deadline assertion in `clock.rs` and `match_actor.rs` breaks; part 31 should make that one function the clock.
- `support::socket::FakeSocket::drop` trips clippy's `should_implement_trait`; if chunk 19.1 named it otherwise, only
  `Client::drop_transport` (one line per file) changes.

## Decisions
- The scripted cards run on the real engine, which shuffles decks and opens on both mulligans (R265). The default
  harness therefore deals under the first of `seed-actor`, `seed-actor-1`, … whose opening hands hold every seat's
  `fake_deck` extras (searched with `create_game`/`begin_game`/`view_for` under the same override), then walks the game
  to rest on p1's turn 1 through `actor.submit` before any socket attaches: R345's automatic turn end off for both
  seats (TS's scripted port never ended a turn by itself), then both mulligans keeping the whole hand. Those rows are
  `Harness.base`; a seq TS counted from 1 is `base + n`. Tests that TS ran on `createFakeEngine({ mulligan: true })`
  walk only the automatic-turn-end half; real-catalog tests (TS `enginePort()`) are not walked and deal under
  `seed-actor`, as TS did.
- `firstInHand` (the scripted deck's extras were first in hand) is `in_hand(socket, defId)`; TS's literal answer
  `{ type: "answer", choiceId, selection: [{ pick: "none" }] }` is the first `answer` on the holder's legal array.
- Where TS's assertion was about the scripted port's own behaviour, the real engine's is asserted instead, with a
  comment: `offerDraw` as a filler action (the fake accepted it unconditionally; the real one is R36's) becomes R345's
  `setAutoEndTurn { enabled: false }`; the legal arrays include R211's `concede` for both seats during a prompt and
  name one `play` per playable way rather than per hand card; the out-of-turn refusal message is read off
  `jackioh_engine::reduce` itself (the real reducer checks the open prompt first); the opponent's hand count is
  compared with the other hand's current size (the real engine draws each turn); the leak test's made-up
  `p<n>-secret-<i>` decks are two disjoint real decks.
- Clock expiries TS fired on demand are reached by time: the turn (75 s), the prompt (30 s), the grace (p1 drops,
  60 s; its last view is read off the actor since its socket is gone), and the ceiling through a stored match row that
  began a ceiling ago less a second (a played game would end on the turn cap first, R389).
- Profiles `profile-1`/`profile-2` are always seeded (ratings 1000 unless a test gives others), with `inMatchId`,
  since the real results writer always runs; "recordResult called once" is the one `results` row of the match.
- R643's tests wait out emote windows on the real clock, so the turn clock writes `srv-` timeouts as they wait; "no
  row" there means no row but the clock's own.
- `deps.log.entries` is a thread-local `tracing` layer (`log_capture`) recording events whose target starts with
  `jackioh_server`, named by their `event` field, else their message.
- `engine_real.rs` and `match_actor.rs` carry private copies of `decksTheEngineAccepts` and
  `decksThatOpenOnTheMulligans` (catching `create_game`'s panic where TS caught its throw); the pool is
  `jackioh_cards::CATALOG` minus tokens (TS `loadCatalog().cardIds` minus `isToken`).
- `aim.rs`'s cut-down `PlayerView` (TS `as unknown as PlayerView`) is a real deal's view with `viewer`, both hands and
  both `locks` replaced from TS's literal, round-tripped through serde.
- The ws adapter block runs `app::router` on `127.0.0.1:0` under a real (not paused) clock, with a hand-written
  RFC 6455 client (no WebSocket client crate is in §2's stack); the token travels as `?token=` (SURFACE §11.3 drops the
  header path TS's accepted case used).
