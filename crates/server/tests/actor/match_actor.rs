//! BUILD M6-T4 acceptance, items 1, 3 and 4 (item 2, crash recovery, is `recovery.rs`):
//!
//!  1. "two WebSocket clients complete a scripted game"
//!  3. "an action with a reused nonce returns the original ack"
//!  4. "the opponent's socket never receives the other hand's `defId`s (protocol-level test)"
//!
//! plus the properties SPEC §9 asks for around them: a client cannot act as the other player
//! (§9.1), a malformed frame is answered rather than fatal, the per-match flood limit rejects the
//! overflow (§9.8), and a disconnect starts the grace both clients render (§9.5).
//!
//! Item 4 is made twice. Most of this file runs on the scripted cards of `test/fakes/engine.ts`,
//! which can be driven to any situation in one action; the block near the bottom runs the same
//! acceptance item on the **real** §8 catalog, because a redaction cannot be proved against cards
//! the test suite wrote. That block's own header says what only it can see.
//!
//! The clock is the real one here, and the results writer too. TS stubbed the clock on purpose —
//! the actor took `CreateMatchClock` through `ActorDeps`, and half its tests fired an expiry *on
//! demand* (`clock.expire({ kind: "grace", player: "p1" })`) — and stubbed the results writer the
//! same way. The Rust actor takes neither: its clock, its results writer and its store all come
//! from the `App` (SURFACE §11.2, §11.3: no ports, no traits), so every expiry here is a deadline
//! passing on tokio's paused clock, and every ending is written by `api::results` over the fake
//! store, where the TS stub's call count is read back as the row it wrote.
//!
//! Port of `apps/server/test/match/actor.test.ts`. How the TS test's seams map, all in one place:
//!
//!  - `createFakeEngine()` is `support::engine::install_test_cards()`: the scripted cards
//!    (`test-prompt-self`, `test-prompt-enemy`, `test-lethal`) as real engine scripts under this
//!    thread's testkit override (SURFACE §8, §11.2). The real engine shuffles a deck and opens on both
//!    mulligans (R265), where the scripted port dealt `fakeDeck`'s extras first and opened on turn 1,
//!    so the harness deals under the first seed whose opening hands hold every extra, and walks the
//!    game to rest on p1's turn 1 (both seats' R345 automatic turn end off, as the scripted port never
//!    ended a turn by itself; both mulligans keeping the whole hand) through the actor's own `submit`
//!    before any socket attaches. The rows that walk wrote are `Harness.base`: a seq TS counted from 1
//!    is counted from `base + 1`.
//!  - `enginePort()` (the real port) is the real §8 catalog (`jackioh_cards::register_all()`, no
//!    override), dealt under `"seed-actor"` exactly as TS dealt it, and left on both mulligans.
//!  - `createFakeSocket()` is `support::socket::create_fake_socket()`, behind `Client`; every frame
//!    is read as the JSON the wire carries (SURFACE §5.1).
//!  - `deps.store.tables.*` are the fake store's rows, read through the store's own methods.
//!  - `deps.log.entries` is a `tracing` layer of this file's (`log_capture`): the server logs through
//!    `tracing` with TS's event names (SURFACE §11.3).
//!  - `deps.timers` is tokio's paused clock (`start_paused = true`); `advance` sleeps on it.
//!  - The `ws` adapter's block runs a real listener and a minimal WebSocket client of this file's,
//!    since `socketFromWs` and `createMatchSocketHandler` are `actor::ws_server::handle`'s inside now.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::{
    Action, ActionBody, COIN_DEF_ID, CardCost, CardDef, CreateGameArgs, DEFAULT_PORTRAIT, EMOTE_COOLDOWN_MS,
    EMOTE_HAND_SIZE, EMOTE_IDS, EMOTE_WINDOW_MAX, EMOTE_WINDOW_MS, GameOverReason, GameResult, PLAYER_IDS,
    Phase, PlayerId, Winner, deal_emote_hand, emote_gate, opponent_of,
};
use jackioh_server::actor::match_actor::MatchActor;
use jackioh_server::actor::ws_server::{Socket, WS_CLOSE, WS_PATH};
use jackioh_server::app::{App, router};
use jackioh_server::config::{
    DISCONNECT_GRACE_SECONDS, MATCH_ACTIONS_PER_SECOND, MATCH_CEILING_MINUTES, MULLIGAN_CLOCK_SECONDS,
    PROMPT_CLOCK_SECONDS, RATING_DEVIATION_START, RATING_VOLATILITY_START, TURN_CLOCK_SECONDS,
};
use jackioh_server::db::store::{Db, MatchRow, StartMatchInput};
use jackioh_server::ranked::glicko2::{Glicko, rate_game};
use serde_json::{Value, json};
use tracing::subscriber::DefaultGuard;

use self::log_capture::Recorder;
use crate::support::deps::{empty_test_app, test_app};
use crate::support::engine::{fake_deck, install_test_cards};
use crate::support::socket::{FakeSocket, create_fake_socket};

/// R603: the move one ranked game makes between two players new to rating.
fn rating_move(rating_a: f64, rating_b: f64, score_a: f64) -> (f64, f64) {
    let fresh = |rating: f64| Glicko {
        rating,
        deviation: float(RATING_DEVIATION_START),
        volatility: float(RATING_VOLATILITY_START),
    };
    let next = rate_game(&fresh(rating_a), &fresh(rating_b), score_a);
    (next.a.rating, next.b.rating)
}

// ---------------------------------------------------------------------------
// TS's deterministic stand-in for `src/match/clock.ts` (`stubClocks`) and its results writer stub
// (`resultWriter`) are not ported: the Rust actor takes no clock and no results writer to stub (see
// the header). Their tests drive the real ones.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Scaffolding
// ---------------------------------------------------------------------------

const MATCH_ID: &str = "match-1";

/// TS `TEST_CATALOG_VERSION` (`test/fakes/deps.ts`): the catalog version the test matches carry.
const TEST_CATALOG_VERSION: &str = "test-1";

/// The seed every match here is dealt under (TS `"seed-actor"`); a scripted deal searches from it.
const SEED: &str = "seed-actor";

/// How many seeds a scripted deal tries before it gives up on putting its extras in hand.
const SEED_SEARCH_LIMIT: usize = 5000;

/// How many times `settle` yields: enough for every task woken at this instant to run.
const SETTLE_YIELDS: usize = 8;

/// Unit conversion only.
const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;

/// A config number as milliseconds' arithmetic wants it, whatever integer type `config.rs` gives it.
fn int(value: impl Into<i64>) -> i64 {
    value.into()
}

/// A config number as a rating wants it.
fn float(value: impl Into<f64>) -> f64 {
    value.into()
}

fn turn_ms() -> i64 {
    int(TURN_CLOCK_SECONDS) * SECOND
}

fn prompt_ms() -> i64 {
    int(PROMPT_CLOCK_SECONDS) * SECOND
}

fn mulligan_ms() -> i64 {
    int(MULLIGAN_CLOCK_SECONDS) * SECOND
}

fn grace_ms() -> i64 {
    int(DISCONNECT_GRACE_SECONDS) * SECOND
}

fn ceiling_ms() -> i64 {
    int(MATCH_CEILING_MINUTES) * MINUTE
}

/// The server's own epoch-millisecond clock, the one the actor and the match clock read (TS
/// `deps.timers.now()`).
fn now() -> i64 {
    jackioh_server::app::now_ms()
}

/// Lets every task woken at this instant run: the actor, the clock's timers, the sockets' channels.
async fn settle() {
    for _ in 0..SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

/// TS `deps.timers.advance(ms)`: sleeps on the paused clock, so every timer due by then fires in
/// deadline order, then lets what it woke run.
async fn advance(ms: i64) {
    tokio::time::sleep(Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0))).await;
    settle().await;
}

/// TS `toMatchObject`: every key `expected` names is in `actual` and matches, recursively; arrays
/// match element by element and have the same length; anything else is equal.
fn is_match(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| is_match(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(found, value)| is_match(found, value))
        }
        _ => actual == expected,
    }
}

#[track_caller]
fn assert_match(actual: &Value, expected: &Value) {
    assert!(is_match(actual, expected), "{actual} does not match {expected}");
}

fn result_of(winner: Winner, reason: GameOverReason) -> Option<GameResult> {
    Some(GameResult { winner, reason })
}

/// One client's end of a fake socket (`support::socket`), read as the JSON frames the server sent.
/// Every call this file makes on the fake goes through here.
struct Client(FakeSocket);

impl Client {
    fn new() -> Client {
        Client(create_fake_socket())
    }

    /// The half the actor holds. TS handed the fake itself to `attach`; Rust's `Socket` is a struct.
    fn socket(&self) -> Socket {
        self.0.socket()
    }

    /// Every frame the server sent, in order, as text.
    fn sent(&self) -> Vec<String> {
        self.0.sent()
    }

    /// Every frame the server sent, parsed.
    fn messages(&self) -> Vec<Value> {
        self.sent()
            .iter()
            .map(|text| serde_json::from_str(text).expect("every frame is JSON"))
            .collect()
    }

    /// Frames of one `type`, parsed.
    fn of_type(&self, kind: &str) -> Vec<Value> {
        self.messages()
            .into_iter()
            .filter(|message| message["type"] == kind)
            .collect()
    }

    /// Simulate the client sending a frame.
    fn receive(&self, text: &str) {
        self.0.receive(text);
    }

    /// Simulate the client sending JSON.
    fn receive_json(&self, value: Value) {
        self.receive(&value.to_string());
    }

    /// Simulate the transport dropping.
    fn drop_transport(&self) {
        self.0.drop();
    }

    fn clear(&self) {
        self.0.clear();
    }

    fn is_open(&self) -> bool {
        self.0.is_open()
    }
}

/// TS `deps.log`: the server's `tracing` events on this thread, by level and event name.
mod log_capture {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::subscriber::DefaultGuard;
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

    /// TS `RecordingLogger.entries[n]`, by event name: TS's `level` is not read by any test here.
    #[derive(Clone, Debug, PartialEq)]
    pub struct LogEntry {
        pub event: String,
    }

    #[derive(Clone, Default)]
    pub struct Recorder {
        entries: Arc<Mutex<Vec<LogEntry>>>,
    }

    impl Recorder {
        /// Records the server's events on this thread for as long as the guard lives. A
        /// `#[tokio::test]` runs its actor tasks on this thread too (its runtime is current-thread).
        pub fn install() -> (Recorder, DefaultGuard) {
            let recorder = Recorder::default();
            let guard =
                crate::support::deps::set_log_default(tracing_subscriber::registry().with(recorder.clone()));
            (recorder, guard)
        }

        pub fn entries(&self) -> Vec<LogEntry> {
            self.entries.lock().expect("the log").clone()
        }

        pub fn events(&self) -> Vec<String> {
            self.entries().into_iter().map(|entry| entry.event).collect()
        }
    }

    /// The event's name: its `event` field (TS's first argument), else its message.
    #[derive(Default)]
    struct EventName {
        event: Option<String>,
        message: Option<String>,
    }

    impl Visit for EventName {
        fn record_str(&mut self, field: &Field, value: &str) {
            match field.name() {
                "event" => self.event = Some(value.to_string()),
                "message" => self.message = Some(value.to_string()),
                _ => {}
            }
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            let text = format!("{value:?}").trim_matches('"').to_string();
            match field.name() {
                "event" if self.event.is_none() => self.event = Some(text),
                "message" => self.message = Some(text),
                _ => {}
            }
        }
    }

    impl<S: Subscriber> Layer<S> for Recorder {
        fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
            if !event.metadata().target().starts_with("jackioh_server") {
                return;
            }
            let mut name = EventName::default();
            event.record(&mut name);
            let Some(event_name) = name.event.or(name.message) else {
                return;
            };
            self.entries
                .lock()
                .expect("the log")
                .push(LogEntry { event: event_name });
        }
    }
}

// --- the fake store, read and written as TS's `deps.store.tables` --------------------------------

/// TS `deps.store.seedProfile(...)`: a profile row written straight into the fake store.
async fn seed_profile(app: &App, profile: Value) {
    let Db::Fake(data) = &app.db else {
        panic!("the test app runs on the fake store")
    };
    data.lock().await.seed_profile(profile);
}

/// TS `deps.store.tables.matchActions` for one match: its `match_actions` rows, as JSON.
async fn match_actions(app: &App, match_id: &str) -> Vec<Value> {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let rows = tx.matches_actions(match_id).await.expect("matches.actions");
    tx.commit().await.expect("commit");
    rows.iter()
        .map(|row| serde_json::to_value(row).expect("MatchActionRow serialises"))
        .collect()
}

/// TS `deps.store.tables.matches[0]`: the match row, as JSON.
async fn match_row(app: &App, match_id: &str) -> Value {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let row = tx
        .matches_get(match_id)
        .await
        .expect("matches.get")
        .expect("the match row");
    tx.commit().await.expect("commit");
    serde_json::to_value(&row).expect("MatchRow serialises")
}

/// TS `deps.store.matches.create(...)`: a match row written as TS's object literal.
async fn create_match_row(app: &App, row: Value) {
    let row: MatchRow = serde_json::from_value(row).expect("a MatchRow");
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    tx.matches_create(&row).await.expect("matches.create");
    tx.commit().await.expect("commit");
}

/// TS `deps.store.tables.results` for one match (one row at most: §9.5, R611), as JSON. Where TS
/// counted the stub writer's calls, the writer's row is what is counted here.
async fn result_row(app: &App, match_id: &str) -> Option<Value> {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let row = tx
        .results_get_by_match(match_id)
        .await
        .expect("results.getByMatch");
    tx.commit().await.expect("commit");
    row.map(|row| serde_json::to_value(&row).expect("ResultRow serialises"))
}

/// TS `deps.store.profiles.getMany(...)`, as JSON.
async fn profiles(app: &App, ids: &[&str]) -> Vec<Value> {
    let ids: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let rows = tx.profiles_get_many(&ids).await.expect("profiles.getMany");
    tx.commit().await.expect("commit");
    rows.iter()
        .map(|row| serde_json::to_value(row).expect("Profile serialises"))
        .collect()
}

// --- decks -------------------------------------------------------------------------------------

/// TS `catalog.isToken` (`src/api/catalog.ts`): a token by flag or by tag.
fn is_token(def: &CardDef) -> bool {
    def.token || def.tags.iter().any(|tag| tag.as_str() == "Token")
}

/// TS `loadCatalog()`'s `cardIds`, tokens filtered out: every deckable §8 id, in catalog order.
fn deckable_pool() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| !is_token(def))
        .map(|(id, _)| id.clone())
        .collect()
}

fn game_args(seed: &str, decks: &(Vec<String>, Vec<String>)) -> CreateGameArgs {
    CreateGameArgs {
        seed: seed.to_string(),
        decks: decks.clone(),
        ..CreateGameArgs::default()
    }
}

/// `test/fakes/engine.ts`'s `decksTheEngineAccepts`, a private copy (it is the engine's own
/// refusal that sizes the decks, so the size is never restated): the slices of the pool grow until
/// `create_game` stops refusing them, and "refused every deck size" names an unregistered catalog.
fn decks_the_engine_accepts(pool: &[String], seed: &str) -> (Vec<String>, Vec<String>) {
    let mut refusals: Vec<String> = Vec::new();
    let mut size = 1;
    while size * 2 <= pool.len() {
        let decks = (pool[..size].to_vec(), pool[size..size * 2].to_vec());
        let args = game_args(seed, &decks);
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            jackioh_engine::create_game(&args)
        })) {
            Ok(_) => return decks,
            Err(payload) => {
                let refusal = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_string()))
                    .unwrap_or_default();
                if !refusals.contains(&refusal) {
                    refusals.push(refusal);
                }
            }
        }
        size += 1;
    }
    panic!(
        "the real engine refused every deck size built from the catalog:\n  {}",
        refusals.join("\n  ")
    );
}

/// `decksTheEngineAccepts`, narrowed to the decks whose opening deal, under `seed`, opens straight
/// onto both mulligans (§2.1, R265) — what a test about the window both seats share needs from its
/// very first frame.
///
/// A deal that asks its seat something makes setup wait for the answer before it opens the mulligans
/// (R224). No cast-on-draw card is dealt (R635), so no cast asks; a start-of-game clause as its card
/// arrives in a hand still can (R151). Which cards the deal draws is the seed's to say, so whether a
/// deal asks is read off the real engine, not off card text. The first deck stays the pool's first
/// slice; the second slides along the pool a card at a time until the deal asks nothing, so a seed
/// whose deal already opened on the mulligans keeps exactly the decks `decks_the_engine_accepts`
/// gives it.
fn decks_that_open_on_the_mulligans(pool: &[String], seed: &str) -> (Vec<String>, Vec<String>) {
    let (first, _) = decks_the_engine_accepts(pool, seed);
    let size = first.len();
    let mut from = size;
    while from + size <= pool.len() {
        let decks = (first.clone(), pool[from..from + size].to_vec());
        let begun = jackioh_engine::begin_game(&jackioh_engine::create_game(&game_args(seed, &decks))).state;
        if begun.phase == Phase::Mulligan && jackioh_engine::mulligan_owed(&begun).len() == 2 {
            return decks;
        }
        from += 1;
    }
    panic!(
        "under seed {seed} no second deck lets the deal open on both mulligans: p1's own deal asks first (R224)"
    );
}

/// The def ids a seat holds once the deal is done, read through its own view (§10.8).
fn dealt_def_ids(state: &jackioh_engine::GameState, player: PlayerId) -> Vec<String> {
    let view = serde_json::to_value(jackioh_engine::view_for(state, player)).expect("PlayerView serialises");
    hand(&view).into_iter().map(|card| card.def_id).collect()
}

/// TS's `fakeDeck(extra)` put `extra` in the opening hand; the real engine shuffles every deck with
/// the match rng, so the scripted decks are dealt under the first of `seed-actor`, `seed-actor-1`, …
/// whose opening hands hold every seat's extras. The search reads the real engine's own deal.
fn seed_dealing(decks: &(Vec<String>, Vec<String>), wants: [&[&str]; 2]) -> String {
    for attempt in 0..SEED_SEARCH_LIMIT {
        let seed = if attempt == 0 {
            SEED.to_string()
        } else {
            format!("{SEED}-{attempt}")
        };
        let begun = jackioh_engine::begin_game(&jackioh_engine::create_game(&game_args(&seed, decks))).state;
        let holds = |player: PlayerId, wanted: &[&str]| {
            let dealt = dealt_def_ids(&begun, player);
            wanted
                .iter()
                .all(|def_id| dealt.iter().any(|held| held == def_id))
        };
        if holds(P1, wants[0]) && holds(P2, wants[1]) {
            return seed;
        }
    }
    panic!("no seed of {SEED_SEARCH_LIMIT} deals {wants:?} into the opening hands");
}

// --- the harness -------------------------------------------------------------------------------

/// Which cards the two seats play.
#[derive(Clone)]
enum Decks {
    /// `support::engine`'s scripted cards: `fake_deck(extra)` per seat (TS `createFakeEngine()`).
    Scripted {
        p1: Vec<&'static str>,
        p2: Vec<&'static str>,
    },
    /// Real §8 ids (TS `engine: enginePort()`), dealt under `SEED`.
    Real(Vec<String>, Vec<String>),
}

/// How far the harness walks the game before any socket attaches.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Walk {
    /// Not at all: the game is on both mulligans, as the real port opened it.
    AsDealt,
    /// R345's automatic turn end off for both seats, the mulligans left open (TS
    /// `createFakeEngine({ mulligan: true })`, which never ended a turn by itself).
    AutoEndOff,
    /// `AutoEndOff`, then both mulligans keeping the whole hand: at rest on p1's turn 1, where TS's
    /// `createFakeEngine()` opened.
    ToTurnOne,
}

struct Options {
    decks: Decks,
    /// R642: the portraits dealt to the two seats, written onto the match row `registry.start` makes.
    portraits: Option<[&'static str; 2]>,
    attach: bool,
    walk: Walk,
    /// The two profiles' ratings. TS seeded profiles only for the tests about what an ending records
    /// (its stub writer needed none); the Rust actor always writes through the real `api::results`,
    /// so both profiles are always seeded, at 1000 unless a test says otherwise.
    ratings: [f64; 2],
}

impl Default for Options {
    fn default() -> Options {
        Options {
            decks: Decks::Scripted {
                p1: vec!["test-prompt-self"],
                p2: vec!["test-prompt-enemy", "test-lethal"],
            },
            portraits: None,
            attach: true,
            walk: Walk::ToTurnOne,
            ratings: [1000.0, 1000.0],
        }
    }
}

/// The real catalog's decks, left as dealt (TS `harness({ engine: enginePort(), p1Deck, p2Deck })`).
fn real(decks: (Vec<String>, Vec<String>)) -> Options {
    Options {
        decks: Decks::Real(decks.0, decks.1),
        walk: Walk::AsDealt,
        ..Options::default()
    }
}

struct Harness {
    app: Arc<App>,
    actor: MatchActor,
    p1: Client,
    p2: Client,
    /// The rows the harness's walk wrote before the test began (see the header).
    base: usize,
    log: Recorder,
    _log: DefaultGuard,
}

impl Harness {
    /// TS `actor.idle()`: the frames sent so far have been handled and what they pushed has arrived.
    async fn idle(&self) {
        settle().await;
        self.actor.idle().await;
        settle().await;
    }

    fn socket(&self, player: PlayerId) -> &Client {
        match player {
            PlayerId::P1 => &self.p1,
            PlayerId::P2 => &self.p2,
        }
    }

    /// The rows the test itself caused: everything after the harness's walk.
    async fn rows(&self) -> Vec<Value> {
        match_actions(&self.app, MATCH_ID)
            .await
            .into_iter()
            .skip(self.base)
            .collect()
    }

    async fn last_row(&self) -> Value {
        self.rows().await.last().cloned().expect("a log row")
    }

    /// TS `actor.clocks()`, as `MatchClocks`' JSON.
    async fn clocks(&self) -> Value {
        serde_json::to_value(self.actor.clocks()).expect("MatchClocks serialises")
    }

    /// TS `actor.viewFor(player)`, as JSON.
    async fn view(&self, player: PlayerId) -> Value {
        view_json(&self.actor, player).await
    }
}

async fn view_json(actor: &MatchActor, player: PlayerId) -> Value {
    serde_json::to_value(actor.view_for(player)).expect("PlayerView serialises")
}

/// One action through `submit`, which must be acked.
async fn submit_ok(actor: &MatchActor, player: PlayerId, nonce: &str, body: Value) {
    let body: ActionBody = serde_json::from_value(body).expect("an ActionBody");
    let reply = actor.submit(player, nonce.to_string(), body).await;
    let reply = serde_json::to_value(&reply).expect("ServerMessage serialises");
    assert_eq!(
        reply["type"],
        json!("ack"),
        "the walk's {nonce} was refused: {reply}"
    );
}

/// Brings the game to where TS's scripted port held it, through the actor's own `submit`, before any
/// socket attaches (no socket sees a frame of it). Answers how many log rows that wrote.
async fn walk_setup(actor: &MatchActor, walk: Walk) -> usize {
    if walk == Walk::AsDealt {
        return 0;
    }
    let mut rows = 0;
    for player in PLAYER_IDS {
        submit_ok(
            actor,
            player,
            &format!("setup-auto-{player}"),
            json!({ "type": "setAutoEndTurn", "enabled": false }),
        )
        .await;
        rows += 1;
    }
    if walk == Walk::ToTurnOne {
        for player in PLAYER_IDS {
            let keep = hand_ids(&view_json(actor, player).await);
            submit_ok(
                actor,
                player,
                &format!("setup-mulligan-{player}"),
                json!({ "type": "mulligan", "keep": keep }),
            )
            .await;
            rows += 1;
        }
        let at = actor.snapshot();
        assert_eq!(
            (at.phase, at.turn, at.active, at.pending_for),
            (Phase::Main, 1, P1, None),
            "the walk did not come to rest on p1's turn 1"
        );
    }
    rows
}

async fn harness(options: Options) -> Harness {
    let (log, log_guard) = Recorder::install();
    let (decks, seed) = match &options.decks {
        Decks::Scripted { p1, p2 } => {
            install_test_cards();
            let decks = (fake_deck(p1), fake_deck(p2));
            let seed = seed_dealing(&decks, [p1.as_slice(), p2.as_slice()]);
            (decks, seed)
        }
        Decks::Real(p1, p2) => {
            jackioh_cards::register_all();
            ((p1.clone(), p2.clone()), SEED.to_string())
        }
    };
    let app = empty_test_app().await;
    let [first, second] = options.ratings;
    seed_profile(
        &app,
        json!({ "id": "profile-1", "rating": first, "inMatchId": MATCH_ID }),
    )
    .await;
    seed_profile(
        &app,
        json!({ "id": "profile-2", "rating": second, "inMatchId": MATCH_ID }),
    )
    .await;

    let mut seats = json!([
        { "profileId": "profile-1", "player": "p1", "deck": decks.0 },
        { "profileId": "profile-2", "player": "p2", "deck": decks.1 },
    ]);
    if let Some([p1_portrait, p2_portrait]) = options.portraits {
        seats[0]["portrait"] = json!(p1_portrait);
        seats[1]["portrait"] = json!(p2_portrait);
    }
    let input: StartMatchInput = serde_json::from_value(json!({
        "matchId": MATCH_ID,
        "seed": seed,
        "catalogVersion": TEST_CATALOG_VERSION,
        // R604: as the queue starts one, so an ending that is rated is rated.
        "ranked": true,
        "seats": seats,
    }))
    .expect("a StartMatchInput");
    app.matches.start(&app, input).await.expect("registry.start");

    let actor = app.matches.actor_for(&app, MATCH_ID).await.expect("the actor");
    actor.idle().await;
    let base = walk_setup(&actor, options.walk).await;
    let h = Harness {
        app,
        actor,
        p1: Client::new(),
        p2: Client::new(),
        base,
        log,
        _log: log_guard,
    };
    if options.attach {
        h.actor.attach(P1, h.p1.socket());
        h.actor.attach(P2, h.p2.socket());
        h.idle().await;
    }
    h
}

/// TS `send(actor, socket, nonce, body, smuggled)`: an action frame `{ ...body, ...smuggled, nonce }`.
async fn send_smuggling(h: &Harness, socket: &Client, nonce: &str, body: Value, smuggled: Value) {
    let mut action = body;
    if let (Some(fields), Some(extra)) = (action.as_object_mut(), smuggled.as_object()) {
        for (key, value) in extra {
            fields.insert(key.clone(), value.clone());
        }
    }
    action["nonce"] = json!(nonce);
    socket.receive_json(json!({ "type": "action", "action": action }));
    h.idle().await;
}

async fn send(h: &Harness, socket: &Client, nonce: &str, body: Value) {
    send_smuggling(h, socket, nonce, body, json!({})).await;
}

fn views(socket: &Client) -> Vec<Value> {
    socket
        .of_type("view")
        .into_iter()
        .map(|frame| frame["view"].clone())
        .collect()
}

fn last_view(socket: &Client) -> Value {
    views(socket).last().cloned().expect("no view frame was sent")
}

/// One card of a seat's own hand, as its view lists it.
#[derive(Clone, Debug, PartialEq)]
struct HandCard {
    instance_id: String,
    def_id: String,
}

fn hand(view: &Value) -> Vec<HandCard> {
    view["you"]["hand"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .map(|card| HandCard {
                    instance_id: card["instanceId"].as_str().unwrap_or_default().to_string(),
                    def_id: card["defId"].as_str().unwrap_or_default().to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn hand_ids(view: &Value) -> Vec<String> {
    hand(view).into_iter().map(|card| card.instance_id).collect()
}

/// TS `firstInHand(socket)`: TS's scripted deck put its extras first in hand, so "the first card"
/// was always the scripted card the test meant. The real deal is shuffled, so it is named.
fn in_hand(socket: &Client, def_id: &str) -> String {
    hand(&last_view(socket))
        .into_iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.instance_id)
        .unwrap_or_else(|| panic!("{def_id} is not in the hand"))
}

fn open_choice(socket: &Client) -> String {
    let pending = last_view(socket)["pending"].clone();
    assert!(
        pending["forYou"] == json!(true),
        "no prompt is open for this player"
    );
    pending["choiceId"].as_str().expect("a choice id").to_string()
}

// `of_type` yields whole frames, discriminator included: every server message is a member of
// `ServerMessage`'s tagged union (protocol.rs) and an `ack` is no exception.
fn errors(socket: &Client) -> Vec<Value> {
    socket.of_type("error")
}

fn acks(socket: &Client) -> Vec<Value> {
    socket.of_type("ack")
}

fn legal_of(socket: &Client) -> Vec<Value> {
    let frame = socket
        .of_type("view")
        .last()
        .cloned()
        .expect("no view frame was sent");
    frame["legal"]
        .as_array()
        .cloned()
        .expect("the view frame's legal array")
}

fn legal_types(socket: &Client) -> Vec<String> {
    legal_of(socket)
        .iter()
        .map(|action| action["type"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The answer to the seat's open prompt. TS sent `{ type: "answer", choiceId, selection: [{ pick:
/// "none" }] }`, the scripted port's one answer; the scripted card's prompt is the engine's now, so
/// its answer is read off the seat's own legal array, where TS's was too.
fn answer_of(socket: &Client) -> Value {
    legal_of(socket)
        .into_iter()
        .find(|action| action["type"] == "answer")
        .expect("the seat may answer its prompt")
}

fn seq_of(frame: &Value) -> i64 {
    frame["seq"].as_i64().expect("a seq")
}

/// Keys that only exist on a `GameState`, never on a `PlayerView` (§10.1 vs §10.8). A frame naming
/// one of them is a state that escaped, whatever its values happen to be.
const STATE_ONLY_KEYS: &[&str] = &[
    "library",
    "libraries",
    "hands",
    "decks",
    "seed",
    "rngCursor",
    "triggerQueue",
    "echoQueue",
    "applied",
    "pendingChoice",
    "state",
];

/// Every value that appears anywhere in a frame, with its key path, for the leak scan.
fn walk(value: &Value, key: &str, visit: &mut dyn FnMut(&str, &Value)) {
    visit(key, value);
    match value {
        Value::Array(entries) => {
            for entry in entries {
                walk(entry, key, visit);
            }
        }
        Value::Object(fields) => {
            for (child_key, child) in fields {
                walk(child, child_key, visit);
            }
        }
        _ => {}
    }
}

/// The leak scan over the frames a socket received: secret strings found, and state-only keys.
fn scan(sent: &[String], secrets: &[String]) -> (Vec<String>, Vec<String>) {
    let mut leaked = Vec::new();
    let mut state_shaped = Vec::new();
    for frame in sent {
        let parsed: Value = serde_json::from_str(frame).expect("every frame is JSON");
        walk(&parsed, "$", &mut |key, value| {
            if STATE_ONLY_KEYS.contains(&key) {
                state_shaped.push(format!("{key} in {}", frame.chars().take(40).collect::<String>()));
            }
            if let Some(text) = value.as_str()
                && secrets.iter().any(|secret| secret == text)
            {
                leaked.push(text.to_string());
            }
        });
    }
    (leaked, state_shaped)
}

// ---------------------------------------------------------------------------
// M6-T4 acceptance 1
// ---------------------------------------------------------------------------

mod m6_t4_the_match_actor {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn two_websocket_clients_complete_a_scripted_game() {
        let h = harness(Options::default()).await;

        // Both clients get a full view the moment they attach (§9.5, §10.8).
        assert_eq!(views(&h.p1).len(), 1);
        assert_eq!(views(&h.p2).len(), 1);
        assert_eq!(last_view(&h.p1)["viewer"], json!("p1"));
        assert_eq!(last_view(&h.p2)["viewer"], json!("p2"));

        // p1 plays a card that opens a prompt for itself, answers it, and ends the turn.
        send(
            &h,
            &h.p1,
            "n1",
            json!({ "type": "play", "instanceId": in_hand(&h.p1, "test-prompt-self") }),
        )
        .await;
        assert_match(&last_view(&h.p1)["pending"], &json!({ "forYou": true }));
        assert_match(
            &last_view(&h.p2)["pending"],
            &json!({ "forYou": false, "pendingFor": "p1" }),
        );
        assert_eq!(answer_of(&h.p1)["choiceId"], json!(open_choice(&h.p1)));
        send(&h, &h.p1, "n2", answer_of(&h.p1)).await;
        send(&h, &h.p1, "n3", json!({ "type": "endTurn" })).await;
        assert_eq!(last_view(&h.p2)["active"], json!("p2"));

        // p2 plays a card that opens a prompt for p1 (a trap firing on your turn, R79), p1 answers,
        // and p2 then plays lethal.
        send(
            &h,
            &h.p2,
            "n4",
            json!({ "type": "play", "instanceId": in_hand(&h.p2, "test-prompt-enemy") }),
        )
        .await;
        assert_match(&last_view(&h.p1)["pending"], &json!({ "forYou": true }));
        send(&h, &h.p1, "n5", answer_of(&h.p1)).await;
        send(
            &h,
            &h.p2,
            "n6",
            json!({ "type": "play", "instanceId": in_hand(&h.p2, "test-lethal") }),
        )
        .await;

        // Both sockets see the game through to its result.
        assert_eq!(
            last_view(&h.p1)["result"],
            json!({ "winner": "p2", "reason": "hero-death" })
        );
        assert_eq!(
            last_view(&h.p2)["result"],
            json!({ "winner": "p2", "reason": "hero-death" })
        );

        // §9.3: one append-only row per accepted action, gapless.
        let base = i64::try_from(h.base).expect("a small base");
        let log = h.rows().await;
        let seqs: Vec<i64> = log.iter().map(seq_of).collect();
        assert_eq!(seqs, (1..=6).map(|n| base + n).collect::<Vec<i64>>());
        let nonces: Vec<Value> = log.iter().map(|row| row["action"]["nonce"].clone()).collect();
        assert_eq!(
            nonces,
            vec![
                json!("n1"),
                json!("n2"),
                json!("n3"),
                json!("n4"),
                json!("n5"),
                json!("n6")
            ]
        );
        assert_eq!(
            acks(&h.p1).iter().map(seq_of).collect::<Vec<i64>>(),
            vec![base + 1, base + 2, base + 3, base + 5]
        );
        assert_eq!(
            acks(&h.p2).iter().map(seq_of).collect::<Vec<i64>>(),
            vec![base + 4, base + 6]
        );

        // §9.5: every ending records exactly one result.
        let result = result_row(&h.app, MATCH_ID).await.expect("the result row");
        assert_match(
            &result,
            &json!({ "matchId": MATCH_ID, "winnerProfileId": "profile-2", "reason": "hero-death" }),
        );
        assert_eq!(match_row(&h.app, MATCH_ID).await["status"], json!("finished"));
    }

    // -------------------------------------------------------------------------
    // M6-T4 acceptance 3
    // -------------------------------------------------------------------------

    #[tokio::test(start_paused = true)]
    async fn an_action_with_a_reused_nonce_returns_the_original_ack() {
        let h = harness(Options::default()).await;
        // Read the hand off the attach view *before* clearing the buffer the counts below are taken
        // from: `clear()` drops the frames already sent, `in_hand` needs one of them.
        let body = json!({ "type": "play", "instanceId": in_hand(&h.p1, "test-prompt-self") });
        h.p1.clear();

        send(&h, &h.p1, "same-nonce", body.clone()).await;
        let first_ack = acks(&h.p1).last().cloned().expect("an ack");
        let views_after_first = views(&h.p1).len();
        let base = i64::try_from(h.base).expect("a small base");
        assert_eq!(
            first_ack,
            json!({ "type": "ack", "nonce": "same-nonce", "seq": base + 1 })
        );

        send(&h, &h.p1, "same-nonce", body).await;

        // The identical ack came back...
        assert_eq!(acks(&h.p1), vec![first_ack.clone(), first_ack]);
        // ...and nothing else happened: no second log row and no second view push.
        assert_eq!(h.rows().await.len(), 1);
        assert_eq!(views(&h.p1).len(), views_after_first);
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
    }

    // -------------------------------------------------------------------------
    // M6-T4 acceptance 4
    // -------------------------------------------------------------------------

    #[tokio::test(start_paused = true)]
    async fn the_opponents_socket_never_receives_the_other_hands_def_ids() {
        // TS dealt two disjoint decks of made-up ids (`p1-secret-<n>`, `p2-secret-<n>`) to its
        // scripted port, since `fakeDeck()` gives both seats the same filler ids and the scan below
        // would pass for the wrong reason. The real engine deals only catalog ids, so the two disjoint
        // decks are real §8 ones, walked to p1's turn 1 as the scripted port opened.
        let pool = deckable_pool();
        let decks = decks_that_open_on_the_mulligans(&pool, SEED);
        let h = harness(Options {
            walk: Walk::ToTurnOne,
            ..real(decks)
        })
        .await;

        let p1_hand: Vec<String> = hand(&last_view(&h.p1))
            .into_iter()
            .map(|card| card.def_id)
            .collect();
        let p2_hand: Vec<String> = hand(&last_view(&h.p2))
            .into_iter()
            .map(|card| card.def_id)
            .collect();
        assert!(!p1_hand.is_empty());
        assert!(!p2_hand.is_empty());

        // Drive the match without playing anything, so every card in both hands stays hidden (a card
        // that has been played is public in the graveyard, §10.8).
        h.p1.clear();
        h.p2.clear();
        send(&h, &h.p1, "t1", json!({ "type": "endTurn" })).await;
        send(&h, &h.p2, "t2", json!({ "type": "endTurn" })).await;
        send(&h, &h.p1, "t3", json!({ "type": "offerDraw" })).await;
        h.p1.receive_json(json!({ "type": "hello" }));
        h.idle().await;

        let mut leaked = Vec::new();
        let mut state_shaped = Vec::new();
        for (socket, secrets) in [(&h.p2, &p1_hand), (&h.p1, &p2_hand)] {
            let (found, shaped) = scan(&socket.sent(), secrets);
            leaked.extend(found);
            state_shaped.extend(shaped);
        }

        assert_eq!(leaked, Vec::<String>::new());
        assert_eq!(state_shaped, Vec::<String>::new());
        // What the opponent does get is a count (§10.8), and it is the other hand's size now (the real
        // engine draws at each turn's start, where the scripted port drew nothing after the deal).
        assert_eq!(
            last_view(&h.p2)["opponent"]["hand"],
            json!({ "count": hand(&last_view(&h.p1)).len() })
        );
        assert_eq!(
            last_view(&h.p1)["opponent"]["hand"],
            json!({ "count": hand(&last_view(&h.p2)).len() })
        );
        assert!(last_view(&h.p2)["opponent"]["libraryCount"].as_i64().unwrap_or(0) > 0);
    }

    // -------------------------------------------------------------------------
    // §9.1: the seat is the server's
    // -------------------------------------------------------------------------

    #[tokio::test(start_paused = true)]
    async fn a_client_cannot_submit_an_action_as_the_other_player() {
        let h = harness(Options::default()).await;

        // p1 puts p2's seat on the wire and asks to play one of p1's own cards.
        send_smuggling(
            &h,
            &h.p1,
            "spoof",
            json!({ "type": "play", "instanceId": in_hand(&h.p1, "test-prompt-self") }),
            json!({ "playerId": "p2" }),
        )
        .await;

        let row = h.last_row().await;
        assert_eq!(row["action"]["playerId"], json!("p1"));
        assert_eq!(h.actor.snapshot().active, P1);

        // And the other seat cannot act out of turn: the reducer's own refusal is relayed (§9.3). TS's
        // scripted port said "it is not your turn" here; the real reducer checks the open prompt first,
        // so the sentence relayed is whatever it says, read off the reducer itself.
        let refusal = jackioh_engine::reduce(
            &h.actor.engine_state(),
            &Action::new(ActionBody::EndTurn, P2, "out-of-turn"),
        )
        .error
        .expect("the reducer refuses p2's endTurn");
        send(&h, &h.p2, "out-of-turn", json!({ "type": "endTurn" })).await;
        assert_eq!(
            errors(&h.p2).last(),
            Some(
                &json!({ "type": "error", "code": "illegal_action", "message": refusal, "nonce": "out-of-turn" })
            )
        );
        assert_eq!(h.rows().await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r79_refuses_a_server_only_action_sent_by_a_client() {
        let h = harness(Options::default()).await;
        h.p1.receive_json(
            json!({ "type": "action", "action": { "type": "ceilingReached", "nonce": "cheat" } }),
        );
        h.idle().await;

        let last = errors(&h.p1).last().cloned().expect("an error frame");
        assert_eq!(last["code"], json!("malformed"));
        assert!(
            last["message"]
                .as_str()
                .unwrap_or_default()
                .contains("server-only")
        );
        assert_eq!(h.rows().await, Vec::<Value>::new());
        assert_eq!(h.actor.snapshot().result, None);
    }

    #[tokio::test(start_paused = true)]
    async fn answers_a_malformed_frame_and_stays_alive() {
        let h = harness(Options::default()).await;

        h.p1.receive("this is not JSON");
        h.p1.receive_json(json!({ "type": "action", "action": { "type": "play" } }));
        h.p1.receive_json(json!({ "type": "nonsense" }));
        h.idle().await;

        let codes: Vec<Value> = errors(&h.p1).iter().map(|error| error["code"].clone()).collect();
        assert_eq!(
            codes,
            vec![json!("malformed"), json!("malformed"), json!("malformed")]
        );

        // The actor still works.
        send(&h, &h.p1, "after-garbage", json!({ "type": "endTurn" })).await;
        let base = i64::try_from(h.base).expect("a small base");
        assert_eq!(
            acks(&h.p1).last(),
            Some(&json!({ "type": "ack", "nonce": "after-garbage", "seq": base + 1 }))
        );
    }

    // -------------------------------------------------------------------------
    // §9.8: per-match flood limit
    // -------------------------------------------------------------------------

    /// The action the flood tests spend their budget on: R345's preference, which the real reducer
    /// accepts from either seat at any moment and which changes nothing on the board. TS spent it on
    /// `offerDraw`, which its scripted port accepted unconditionally; the real one is R36's, once a turn.
    fn filler() -> Value {
        json!({ "type": "setAutoEndTurn", "enabled": false })
    }

    #[tokio::test(start_paused = true)]
    async fn r137_rejects_the_overflow_of_a_seats_action_budget_without_touching_the_opponents() {
        let h = harness(Options::default()).await;
        h.p1.clear();
        h.p2.clear();

        let budget = MATCH_ACTIONS_PER_SECOND as i64;
        for i in 0..=budget {
            send(&h, &h.p1, &format!("flood-{i}"), filler()).await;
        }

        let overflow = errors(&h.p1);
        assert_eq!(overflow.len(), 1);
        assert_match(
            &overflow[0],
            &json!({ "code": "rate_limited", "nonce": format!("flood-{budget}") }),
        );
        assert_eq!(i64::try_from(h.rows().await.len()).expect("a small log"), budget);
        // §9.8: reject the overflow, do not drop the socket.
        assert!(h.p1.is_open());

        // R137: one shared per-match counter would refuse the *victim* here, which is the abuse the
        // ruling is about. Each seat holds its own window, so the opponent's click still lands while
        // p1 is over budget — and the match's aggregate ceiling is twice R109's number, not once.
        send(&h, &h.p2, "opponent-click", filler()).await;
        assert_eq!(errors(&h.p2), Vec::<Value>::new());
        assert_eq!(
            acks(&h.p2).last().map(|ack| ack["nonce"].clone()),
            Some(json!("opponent-click"))
        );

        // A second later the budget is back.
        advance(SECOND).await;
        send(&h, &h.p1, "after-window", filler()).await;
        assert_eq!(
            acks(&h.p1).last().map(|ack| ack["nonce"].clone()),
            Some(json!("after-window"))
        );
    }

    // -------------------------------------------------------------------------
    // §9.5: disconnect grace, and the clock as the actor drives it
    // -------------------------------------------------------------------------

    #[tokio::test(start_paused = true)]
    async fn starts_the_grace_countdown_on_a_drop_and_clears_it_on_reconnect() {
        let h = harness(Options::default()).await;

        h.p1.drop_transport();
        h.idle().await;

        assert!(!h.clocks().await["graceDeadline"]["p1"].is_null());
        // §9.5: "the grace countdown is stored on the match so both clients show it".
        let stored = match_row(&h.app, MATCH_ID).await["clocks"]["graceDeadline"]["p1"].clone();
        assert!(!stored.is_null());
        let shown = h.p2.of_type("clock").last().cloned().expect("a clock frame");
        assert_eq!(shown["clocks"]["graceDeadline"]["p1"], stored);

        // Reconnect: a fresh full view, never a log replay.
        let revived = Client::new();
        h.actor.attach(P1, revived.socket());
        h.idle().await;

        assert!(h.clocks().await["graceDeadline"]["p1"].is_null());
        assert_eq!(views(&revived).len(), 1);
        assert_eq!(last_view(&revived)["viewer"], json!("p1"));
        assert!(match_row(&h.app, MATCH_ID).await["clocks"]["graceDeadline"]["p1"].is_null());
    }

    #[tokio::test(start_paused = true)]
    async fn r79_r146_dispatches_every_clock_expiry_as_a_logged_server_action_each_stamped_per_r146() {
        let h = harness(Options::default()).await;

        // A turn expiry ends the active player's turn. (TS fired its stub clock's `expire`; the real
        // clock fires when the deadline passes.)
        advance(turn_ms()).await;
        h.idle().await;
        assert_match(
            &h.last_row().await["action"],
            &json!({ "type": "timeout", "playerId": "p1" }),
        );
        assert_eq!(h.actor.snapshot().active, P2);

        // A prompt expiry answers only that prompt (the holder is the non-active player here).
        send(
            &h,
            &h.p2,
            "prompt-enemy",
            json!({ "type": "play", "instanceId": in_hand(&h.p2, "test-prompt-enemy") }),
        )
        .await;
        assert_eq!(h.actor.snapshot().pending_for, Some(P1));
        advance(prompt_ms()).await;
        h.idle().await;
        assert_match(
            &h.last_row().await["action"],
            &json!({ "type": "timeout", "playerId": "p1" }),
        );
        assert_eq!(h.actor.snapshot().pending_for, None);
        // R79: the non-active player's prompt clock does not end the active player's turn.
        assert_eq!(h.actor.snapshot().active, P2);

        // Grace expiry is a loss for the disconnected player. The real clock runs a grace only for a
        // seat whose socket has gone (TS fired one on demand with p1 still attached), so p1 drops; p2's
        // turn clock, resumed with the time it had banked (R79), outlasts the grace.
        h.p1.drop_transport();
        h.idle().await;
        advance(grace_ms()).await;
        h.idle().await;
        // R146: p2 is the active player by now, and the loss is still p1's — the result is stamped
        // with the seat it belongs to, never with whoever happened to be active.
        assert_eq!(h.actor.snapshot().active, P2);
        assert_match(
            &h.last_row().await["action"],
            &json!({ "type": "disconnectExpired", "player": "p1" }),
        );
        assert_eq!(
            h.actor.snapshot().result,
            result_of(Winner::P2, GameOverReason::Disconnect)
        );
        assert!(result_row(&h.app, MATCH_ID).await.is_some());
        // The clock stopped: no deadline is armed any more (TS read its stub's `stopped` flag).
        let clocks = h.clocks().await;
        assert!(clocks["turnDeadline"].is_null() && clocks["promptDeadline"].is_null());
        assert_eq!(clocks["graceDeadline"], json!({ "p1": null, "p2": null }));
        // p1's socket is gone, so the view its seat is owed is read off the actor.
        assert!(!h.view(P1).await["result"].is_null());
        assert!(!last_view(&h.p2)["result"].is_null());

        // Seqs stayed gapless across client and server actions alike (§9.3).
        let base = i64::try_from(h.base).expect("a small base");
        let seqs: Vec<i64> = h.rows().await.iter().map(seq_of).collect();
        assert_eq!(seqs, vec![base + 1, base + 2, base + 3, base + 4]);
    }

    #[tokio::test(start_paused = true)]
    async fn r79_r146_ends_the_match_in_a_draw_when_the_ceiling_expires_stamped_with_the_active_seat() {
        // TS fired its stub clock's ceiling on demand. The real clock measures the ceiling from the
        // match's `createdAt` (R79), and a game played out to it would end on the turn cap first
        // (R389), so the match here is a stored row that began a ceiling ago, less a second: the
        // registry rebuilds its actor from it, whose clock then has one second left.
        let (_log, _guard) = Recorder::install();
        install_test_cards();
        let app = empty_test_app().await;
        seed_profile(
            &app,
            json!({ "id": "profile-1", "rating": 1000, "inMatchId": MATCH_ID }),
        )
        .await;
        seed_profile(
            &app,
            json!({ "id": "profile-2", "rating": 1000, "inMatchId": MATCH_ID }),
        )
        .await;
        let created_at = now() - ceiling_ms() + SECOND;
        create_match_row(
            &app,
            json!({
                "id": MATCH_ID,
                "seed": SEED,
                "players": ["profile-1", "profile-2"],
                "decks": [fake_deck(&[]), fake_deck(&[])],
                "catalogVersion": TEST_CATALOG_VERSION,
                "ranked": true,
                "status": "live",
                "createdAt": created_at,
                "finishedAt": null,
                "clocks": {
                    "turnDeadline": null,
                    "promptDeadline": null,
                    "graceDeadline": { "p1": null, "p2": null },
                    "ceilingAt": created_at + ceiling_ms(),
                },
            }),
        )
        .await;
        let actor = app
            .matches
            .actor_for(&app, MATCH_ID)
            .await
            .expect("the rebuilt actor");
        actor.idle().await;
        let active = actor.snapshot().active;

        advance(SECOND).await;
        actor.idle().await;

        // R146: the ceiling belongs to neither player, so it is stamped with the active seat as a
        // convention — the log never has to guess whose action this was when it is folded back.
        let log = match_actions(&app, MATCH_ID).await;
        assert_match(
            &log.last().cloned().expect("a log row")["action"],
            &json!({ "type": "ceilingReached", "playerId": active.as_str() }),
        );
        assert_eq!(
            actor.snapshot().result,
            result_of(Winner::Draw, GameOverReason::MatchCeiling)
        );
        assert!(result_row(&app, MATCH_ID).await.is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn answers_an_action_after_the_result_with_match_over_not_a_log_row() {
        let h = harness(Options::default()).await;
        send(&h, &h.p1, "concede", json!({ "type": "concede" })).await;
        let rows = h.rows().await.len();
        send(&h, &h.p2, "too-late", json!({ "type": "endTurn" })).await;
        assert_match(
            &errors(&h.p2).last().cloned().expect("an error frame"),
            &json!({ "code": "match_over", "nonce": "too-late" }),
        );
        assert_eq!(h.rows().await.len(), rows);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_a_socket_that_room_joins_are_an_http_call() {
        let h = harness(Options::default()).await;
        h.p1.receive_json(json!({ "type": "joinRoom", "roomCode": "ABCDEF" }));
        h.idle().await;
        // SURFACE §11.3: a `joinRoom` frame is answered `malformed` (TS answered `unsupported`).
        assert_match(
            &errors(&h.p1).last().cloned().expect("an error frame"),
            &json!({ "code": "malformed" }),
        );
    }
}

// ---------------------------------------------------------------------------
// R384 — an Activate ability over the socket (#491)
// ---------------------------------------------------------------------------

/// SPEC §10.2 lists `activate` among the actions a client sends, and `legal_actions` lists every
/// Activate ability (R384) and, since R752, every Heroic Power's power as one. The socket's whitelist
/// (`protocol::CLIENT_ACTION_TYPES`) had only the `activatePower` alias, so online every activation a
/// client sent back from its own legal array was answered `malformed` and applied nothing (#491).
mod r384_activate_over_the_socket {
    use super::*;
    use jackioh_engine::ActionType;
    use jackioh_server::actor::protocol::{
        ActionMessage, CLIENT_ACTION_TYPES, ClientMessage, SERVER_ONLY_ACTION_TYPES, parse_client_message,
    };

    /// p1's turns the walk below waits through at most for the rolled power's price: Heroic Power's
    /// dearest power costs (3), p1's third turn's mana.
    const TURNS_TO_AFFORD_ANY_POWER: usize = 3;

    #[test]
    fn r384_every_action_type_but_the_server_only_ones_is_one_a_client_may_send() {
        for kind in ActionType::ALL {
            assert_ne!(
                CLIENT_ACTION_TYPES.contains(kind),
                SERVER_ONLY_ACTION_TYPES.contains(kind),
                "{kind} must be either a client's action or the server's own (§10.2, R79)"
            );
        }
    }

    #[test]
    fn r384_parses_an_activate_field_by_field_and_drops_a_smuggled_player_id() {
        let parsed = parse_client_message(
            &json!({
                "type": "action",
                "action": {
                    "type": "activate",
                    "instanceId": "c7",
                    "ability": "eat",
                    "targets": [{ "pick": "hero", "player": "p2" }],
                    "modes": ["mana"],
                    "tributes": ["c3"],
                    "playerId": "p2",
                    "nonce": "act-1",
                },
            })
            .to_string(),
        );
        assert_eq!(
            parsed,
            Ok(ClientMessage::Action(ActionMessage {
                nonce: "act-1".to_string(),
                body: serde_json::from_value(json!({
                    "type": "activate",
                    "instanceId": "c7",
                    "ability": "eat",
                    "targets": [{ "pick": "hero", "player": "p2" }],
                    "modes": ["mana"],
                    "tributes": ["c3"],
                }))
                .expect("an activate body"),
            }))
        );

        // Only the instance is required, as the reducer reads it (an `activate` naming no ability is
        // the card's only one); a field of the wrong shape is malformed, never passed on.
        let bare = parse_client_message(
            &json!({ "type": "action", "action": { "type": "activate", "instanceId": "c7", "nonce": "act-2" } })
                .to_string(),
        );
        assert!(matches!(bare, Ok(ClientMessage::Action(_))), "{bare:?}");
        for wrong in [
            json!({ "type": "activate", "nonce": "bad" }),
            json!({ "type": "activate", "instanceId": "c7", "ability": 3, "nonce": "bad" }),
            json!({ "type": "activate", "instanceId": "c7", "targets": ["c1"], "nonce": "bad" }),
            json!({ "type": "activate", "instanceId": "c7", "modes": [1], "nonce": "bad" }),
            json!({ "type": "activate", "instanceId": "c7", "tributes": "c3", "nonce": "bad" }),
        ] {
            let parsed = parse_client_message(&json!({ "type": "action", "action": wrong }).to_string());
            assert!(parsed.is_err(), "{wrong} parsed as {parsed:?}");
        }
    }

    /// The `activate` p1's own legal array lists on `card`, if any.
    fn listed_activation(socket: &Client, card: &str) -> Option<Value> {
        legal_of(socket)
            .into_iter()
            .find(|action| action["type"] == "activate" && action["instanceId"] == card)
    }

    #[tokio::test(start_paused = true)]
    async fn r384_r752_the_activate_a_seats_legal_array_lists_is_sent_back_verbatim_and_applied() {
        let h = harness(Options {
            decks: Decks::Scripted {
                p1: vec!["core-098"],
                p2: vec![],
            },
            ..Options::default()
        })
        .await;

        // #98 Heroic Power is Quickdraw, so it opens in p1's hand, and costs (0).
        let power = in_hand(&h.p1, "core-098");
        let play = legal_of(&h.p1)
            .into_iter()
            .find(|action| action["type"] == "play" && action["instanceId"] == power)
            .expect("Heroic Power is playable on turn 1");
        send(&h, &h.p1, "play-power", play).await;
        assert_eq!(errors(&h.p1), Vec::<Value>::new());

        // The power it rolled costs (1) to (3): p1's turns pass until the mana pays it.
        let mut activation = listed_activation(&h.p1, &power);
        for round in 0..TURNS_TO_AFFORD_ANY_POWER {
            if activation.is_some() {
                break;
            }
            send(
                &h,
                &h.p1,
                &format!("p1-end-{round}"),
                json!({ "type": "endTurn" }),
            )
            .await;
            send(
                &h,
                &h.p2,
                &format!("p2-end-{round}"),
                json!({ "type": "endTurn" }),
            )
            .await;
            activation = listed_activation(&h.p1, &power);
        }
        let activation = activation.expect("the rolled power is listed as an `activate` once p1 can pay it");

        send(&h, &h.p1, "activate-power", activation.clone()).await;
        assert_eq!(errors(&h.p1), Vec::<Value>::new(), "the activation was refused");
        assert_eq!(
            acks(&h.p1).last().map(|ack| ack["nonce"].clone()),
            Some(json!("activate-power"))
        );
        let row = h.last_row().await;
        assert_eq!(row["action"]["type"], json!("activate"));
        assert_eq!(row["action"]["playerId"], json!("p1"));
        assert_eq!(row["action"]["ability"], activation["ability"]);

        // Applied: the power's one use this turn is spent, so it is listed no more.
        let state = h.actor.engine_state();
        let card = jackioh_engine::find_instance(&state, &power).expect("Heroic Power is on the field");
        assert_eq!(
            jackioh_engine::subsystems::activate::uses_this_turn(&state, card),
            1
        );
        assert_eq!(listed_activation(&h.p1, &power), None);
    }
}

// ---------------------------------------------------------------------------
// R643 — the emote protocol and its shared rate limit (§9.5, §10.10)
// ---------------------------------------------------------------------------

/// §10.10's emotes through the actor (R643): a top-level `emote` frame, never an `ActionBody`, so it
/// reaches neither `reduce`, the append-only log nor a rejected-action log line; the actor relays it
/// to the opponent alone, drops it silently past the shared limit, and answers an unknown id as
/// `malformed`.
///
/// Every number here is the shared module's (`jackioh_engine::wire::emotes`): the client greys its
/// menu off the same `emote_gate` the actor drops with, so the constants the test measures against
/// are the ones the wire actually enforces rather than this file's restatement of them.
///
/// The clock here is the real one, so a test that waits out windows sees the turn clock end turns
/// (R79) as it waits. Those `timeout`s are the clock's rows (`srv-` nonces, R270); "no row" below
/// means none of an emote's.
mod r643_emotes_through_the_actor_9_5_10_10 {
    use super::*;

    fn relays(socket: &Client) -> Vec<Value> {
        socket.of_type("emote")
    }

    /// R1342: the hand the account's last portraits frame dealt it — the only emotes it may send.
    fn hand(socket: &Client) -> Vec<String> {
        let frame = socket
            .of_type("portraits")
            .last()
            .cloned()
            .expect("a portraits frame");
        frame["emotes"]
            .as_array()
            .expect("the frame's emote hand")
            .iter()
            .map(|id| id.as_str().expect("an emote id").to_string())
            .collect()
    }

    /// The `n`th emote of the hand `socket` was dealt.
    fn dealt(socket: &Client, n: usize) -> String {
        hand(socket).get(n).cloned().expect("a hand of EMOTE_HAND_SIZE")
    }

    /// The rows anything but the clock wrote since the walk (see the module comment).
    async fn client_rows(h: &Harness) -> Vec<Value> {
        h.rows()
            .await
            .into_iter()
            .filter(|row| {
                !row["action"]["nonce"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("srv-")
            })
            .collect()
    }

    #[tokio::test(start_paused = true)]
    async fn r643_relays_a_valid_emote_to_the_opponent_alone_stamped_with_the_senders_seat() {
        let h = harness(Options::default()).await;
        let (p1_emote, p2_emote) = (dealt(&h.p1, 4), dealt(&h.p2, 1));
        h.p1.clear();
        h.p2.clear();

        h.p1.receive_json(json!({ "type": "emote", "emote": p1_emote }));
        h.idle().await;

        assert_eq!(
            relays(&h.p2),
            vec![json!({ "type": "emote", "from": "p1", "emote": p1_emote })]
        );
        // The sender sees their own locally: nothing comes back — no relay, no ack, no error.
        assert_eq!(relays(&h.p1), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(acks(&h.p1), Vec::<Value>::new());

        // …and in the other direction, stamped with p2's seat. The two seats keep their own rate-limit
        // windows, so p2's send immediately after p1's is still inside no cooldown.
        h.p2.receive_json(json!({ "type": "emote", "emote": p2_emote }));
        h.idle().await;
        assert_eq!(
            relays(&h.p1),
            vec![json!({ "type": "emote", "from": "p2", "emote": p2_emote })]
        );
        assert_eq!(relays(&h.p2).len(), 1);
        assert_eq!(errors(&h.p2), Vec::<Value>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn r643_r1342_relays_every_emote_of_the_senders_hand_and_writes_none_of_them_to_the_log() {
        let h = harness(Options::default()).await;
        let held = hand(&h.p1);
        assert_eq!(held.len(), EMOTE_HAND_SIZE);
        h.p1.clear();
        h.p2.clear();

        let mut sent_at: Vec<i64> = Vec::new();
        for emote in &held {
            h.p1.receive_json(json!({ "type": "emote", "emote": emote }));
            sent_at.push(now());
            h.idle().await;
            // A whole window between sends, so the eight are admitted on their own merits.
            advance(EMOTE_WINDOW_MS).await;
        }

        let expected: Vec<Value> = held
            .iter()
            .map(|emote| json!({ "type": "emote", "from": "p1", "emote": emote }))
            .collect();
        assert_eq!(relays(&h.p2), expected);
        // PREMISE: every send really did land inside its own gate — none of the eight was a silent
        // drop this test happened not to look at.
        assert_eq!(sent_at.len(), EMOTE_HAND_SIZE);
        // §9.3, R643: no `ActionBody`, so no seq is spent, no ack is owed, no row is written and no
        // rejected-action entry is logged.
        assert_eq!(client_rows(&h).await, Vec::<Value>::new());
        assert_eq!(acks(&h.p1), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert!(
            !h.log
                .events()
                .iter()
                .any(|event| event == "match.action.rejected")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r643_answers_an_id_outside_the_ten_as_malformed_relays_nothing_and_logs_no_rejected_action() {
        let h = harness(Options::default()).await;
        h.p1.clear();
        h.p2.clear();
        let entries_before = h.log.entries().len();

        for frame in [
            json!({ "type": "emote", "emote": "flex" }),
            json!({ "type": "emote", "emote": "Gary" }),
            json!({ "type": "emote", "emote": 7 }),
            json!({ "type": "emote", "emote": null }),
            json!({ "type": "emote" }),
        ] {
            h.p1.receive_json(frame);
        }
        h.idle().await;

        // The malformed answer is the actor's usual one — code and reason back to the sender alone.
        let malformed = errors(&h.p1);
        assert_eq!(malformed.len(), 5);
        for error in &malformed {
            assert_eq!(error["code"], json!("malformed"));
            assert_eq!(error["message"], json!("\"emote\" must be a known emote id"));
            assert!(error.get("nonce").is_none());
        }
        assert_eq!(relays(&h.p2), Vec::<Value>::new());
        assert_eq!(relays(&h.p1), Vec::<Value>::new());

        // Each bad frame is logged as a malformed frame — and an emote is never a rejected *action*
        // (R643's drop does not count toward the rejected-action alert, and neither does its refusal).
        let logged: Vec<String> = h.log.events().into_iter().skip(entries_before).collect();
        assert_eq!(logged.len(), 5);
        assert!(logged.iter().all(|event| event == "match.frame.malformed"));
        assert!(
            !h.log
                .events()
                .iter()
                .any(|event| event == "match.action.rejected")
        );
        assert_eq!(client_rows(&h).await, Vec::<Value>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn r643_drops_an_emote_sent_inside_the_cooldown_silently_and_resumes_after_it() {
        let h = harness(Options::default()).await;
        let (first, second) = (dealt(&h.p1, 0), dealt(&h.p1, 5));
        h.p1.clear();
        h.p2.clear();

        h.p1.receive_json(json!({ "type": "emote", "emote": first }));
        h.idle().await;
        assert_eq!(
            relays(&h.p2),
            vec![json!({ "type": "emote", "from": "p1", "emote": first })]
        );

        // Inside EMOTE_COOLDOWN_MS the shared gate says no: nothing reaches the opponent, no error
        // reaches the sender, and nothing is written — silence is what a drop needs (R643).
        let entries_before = h.log.entries().len();
        h.p1.receive_json(json!({ "type": "emote", "emote": second }));
        h.idle().await;
        assert_eq!(relays(&h.p2).len(), 1);
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(h.log.entries().len(), entries_before);
        assert_eq!(client_rows(&h).await, Vec::<Value>::new());

        // The gate's own `retry_after_ms` is the wait — derived from the shared constant, not restated.
        let gate = emote_gate(&[now()], now());
        assert!(!gate.ok);
        advance(if gate.ok {
            0
        } else {
            gate.retry_after_ms.unwrap_or(0)
        })
        .await;
        h.p1.receive_json(json!({ "type": "emote", "emote": second }));
        h.idle().await;
        let sent: Vec<Value> = relays(&h.p2).iter().map(|frame| frame["emote"].clone()).collect();
        assert_eq!(sent, vec![json!(first), json!(second)]);
    }

    #[tokio::test(start_paused = true)]
    async fn r643_relays_emote_window_max_spaced_emotes_and_silently_drops_the_next_inside_the_window() {
        let h = harness(Options::default()).await;
        let emote = dealt(&h.p1, 2);
        h.p1.clear();
        h.p2.clear();

        // One past the cooldown between sends, so all EMOTE_WINDOW_MAX are admitted: the window is the
        // thing being measured, not the pause.
        let mut sent_at: Vec<i64> = Vec::new();
        for n in 0..EMOTE_WINDOW_MAX {
            h.p1.receive_json(json!({ "type": "emote", "emote": emote }));
            sent_at.push(now());
            h.idle().await;
            assert_eq!(relays(&h.p2).len(), n + 1);
            advance(EMOTE_COOLDOWN_MS + 1).await;
        }
        // PREMISE: all five really did land inside one window, so the next send has nowhere to go.
        assert!(now() - sent_at.first().copied().unwrap_or(0) < EMOTE_WINDOW_MS);

        let entries_before = h.log.entries().len();
        h.p1.receive_json(json!({ "type": "emote", "emote": emote }));
        h.idle().await;
        assert_eq!(relays(&h.p2).len(), EMOTE_WINDOW_MAX);
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(h.log.entries().len(), entries_before);
        assert_eq!(client_rows(&h).await, Vec::<Value>::new());

        // After the gate's own retry_after_ms — the moment the oldest send leaves the window — the next
        // one relays again.
        let gate = emote_gate(&sent_at, now());
        assert!(!gate.ok);
        advance(if gate.ok {
            0
        } else {
            gate.retry_after_ms.unwrap_or(0)
        })
        .await;
        h.p1.receive_json(json!({ "type": "emote", "emote": emote }));
        h.idle().await;
        assert_eq!(relays(&h.p2).len(), EMOTE_WINDOW_MAX + 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r643_spends_none_of_the_seats_9_8_action_budget_a_flooded_seats_emote_still_relays() {
        let h = harness(Options::default()).await;
        let emote = dealt(&h.p1, 3);
        h.p1.clear();
        h.p2.clear();

        // Put p1 over the per-second action budget, so the next action frame is the flood refusal.
        // (R345's preference stands in for TS's `offerDraw`, as in the flood test above.)
        for n in 0..=MATCH_ACTIONS_PER_SECOND as i64 {
            h.p1.receive_json(json!({
                "type": "action",
                "action": { "type": "setAutoEndTurn", "enabled": false, "nonce": format!("flood-{n}") },
            }));
        }
        h.idle().await;
        assert!(errors(&h.p1).iter().any(|error| error["code"] == "rate_limited"));

        // The emote never counted and is not counted now: it is no ActionBody (R643), so §9.8's limit
        // does not see it and its own limit — the shared gate — still admits the first send.
        h.p1.receive_json(json!({ "type": "emote", "emote": emote }));
        h.idle().await;
        assert_eq!(
            relays(&h.p2),
            vec![json!({ "type": "emote", "from": "p1", "emote": emote })]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r1342_refuses_an_emote_of_the_pool_outside_the_senders_hand_silently_and_spends_no_limit() {
        let h = harness(Options::default()).await;
        let held = hand(&h.p1);
        let outside: Vec<&'static str> = EMOTE_IDS
            .iter()
            .map(|id| id.as_str())
            .filter(|id| !held.iter().any(|kept| kept == id))
            .collect();
        // PREMISE: the pool is bigger than the hand, so there is something to refuse.
        assert_eq!(outside.len(), EMOTE_IDS.len() - EMOTE_HAND_SIZE);
        let inside = dealt(&h.p1, 0);
        h.p1.clear();
        h.p2.clear();

        for emote in &outside {
            h.p1.receive_json(json!({ "type": "emote", "emote": emote }));
        }
        h.idle().await;
        // Refused: never relayed, no error (the id is in the pool, so the frame parsed), no row.
        assert_eq!(relays(&h.p2), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(client_rows(&h).await, Vec::<Value>::new());
        assert!(
            h.log
                .events()
                .iter()
                .any(|event| event == "match.emote.outside_hand")
        );
        assert!(
            !h.log
                .events()
                .iter()
                .any(|event| event == "match.action.rejected" || event == "match.frame.malformed")
        );

        // …and the refusals spent none of the limit: a dealt emote right after them, with no time
        // passing, is admitted as the first of the window.
        h.p1.receive_json(json!({ "type": "emote", "emote": inside }));
        h.idle().await;
        assert_eq!(
            relays(&h.p2),
            vec![json!({ "type": "emote", "from": "p1", "emote": inside })]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r1342_each_seat_is_held_to_its_own_hand_not_the_opponents() {
        let h = harness(Options::default()).await;
        let (mine, theirs) = (hand(&h.p1), hand(&h.p2));
        // An emote only p2 was dealt is outside p1's hand: p1's send of it is refused.
        let only_theirs = theirs
            .iter()
            .find(|id| !mine.contains(id))
            .cloned()
            .expect("PREMISE: the seats' hands differ (they agree about 1 match in 10^5)");
        h.p1.clear();
        h.p2.clear();

        h.p1.receive_json(json!({ "type": "emote", "emote": only_theirs }));
        h.idle().await;
        assert_eq!(relays(&h.p2), Vec::<Value>::new());

        h.p2.receive_json(json!({ "type": "emote", "emote": only_theirs }));
        h.idle().await;
        assert_eq!(
            relays(&h.p1),
            vec![json!({ "type": "emote", "from": "p2", "emote": only_theirs })]
        );
    }
}

// ---------------------------------------------------------------------------
// R642 — the portraits frame (§9.5, §10.11)
// ---------------------------------------------------------------------------

/// A match's two portraits are the match row's data (R642): fixed when the seats are, sent to each
/// client on attach and again on a reconnect's `hello`, in a frame of their own — a portrait is
/// never part of `PlayerView`. The frame carries the receiving account's own emote hand (R1342),
/// dealt from the match seed (R1341).
mod r642_the_portraits_frame_9_5 {
    use super::*;

    fn portraits_sent(socket: &Client) -> Vec<Value> {
        socket.of_type("portraits")
    }

    /// R1341: the hand the seat's account is dealt in this match, as the wire spells it.
    async fn hand_dealt(h: &Harness, seat: PlayerId) -> Value {
        let seed = match_row(&h.app, MATCH_ID).await["seed"]
            .as_str()
            .expect("the match row's seed")
            .to_string();
        json!(
            deal_emote_hand(&seed, seat)
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
        )
    }

    #[tokio::test(start_paused = true)]
    async fn r642_r1342_sends_both_portraits_and_the_accounts_own_hand_on_attach_never_on_the_view() {
        let h = harness(Options {
            portraits: Some(["gary", "shredder"]),
            ..Options::default()
        })
        .await;

        // PREMISE: the row really did freeze the pair the seats were dealt, seat order like `decks`.
        assert_eq!(
            match_row(&h.app, MATCH_ID).await["portraits"],
            json!(["gary", "shredder"])
        );

        for seat in PLAYER_IDS {
            let socket = h.socket(seat);
            assert_eq!(
                portraits_sent(socket),
                vec![json!({
                    "type": "portraits",
                    "p1": "gary",
                    "p2": "shredder",
                    "emotes": hand_dealt(&h, seat).await,
                })]
            );
            // …and the view frame carries none of it: `PlayerView` is a rules surface (R641, R642,
            // R1342).
            let view = socket.of_type("view").last().cloned().expect("a view frame");
            assert!(view["view"].get("portrait").is_none());
            assert!(view["view"].get("portraits").is_none());
            assert!(view["view"].get("emotes").is_none());
        }
        // Each account hears its own hand and never the opponent's.
        assert_ne!(
            portraits_sent(&h.p1)[0]["emotes"],
            portraits_sent(&h.p2)[0]["emotes"]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r642_r1341_sends_them_again_on_a_reconnects_fresh_view_and_on_hello_with_the_same_hand() {
        let h = harness(Options {
            portraits: Some(["timmy", "dfender"]),
            ..Options::default()
        })
        .await;
        assert_eq!(portraits_sent(&h.p1).len(), 1);
        let frame = json!({
            "type": "portraits",
            "p1": "timmy",
            "p2": "dfender",
            "emotes": hand_dealt(&h, P1).await,
        });
        assert_eq!(portraits_sent(&h.p1), vec![frame.clone()]);

        h.p1.drop_transport();
        h.idle().await;
        let revived = Client::new();
        h.actor.attach(P1, revived.socket());
        h.idle().await;
        // §9.5's fresh full view rides with the portraits again, as on the first attach, and R1341's
        // hand is the one the first attach dealt.
        assert_eq!(views(&revived).len(), 1);
        assert_eq!(portraits_sent(&revived), vec![frame.clone()]);

        revived.receive_json(json!({ "type": "hello" }));
        h.idle().await;
        assert_eq!(portraits_sent(&revived), vec![frame.clone(), frame]);
    }

    #[tokio::test(start_paused = true)]
    async fn r642_reads_a_match_row_that_predates_portraits_as_vanilla_vanilla() {
        // A row written before migration 0019 carries no `portraits`; both seats read as the default
        // (R641's `null`-is-`vanilla`, one level up at the row).
        let (_log, _guard) = Recorder::install();
        install_test_cards();
        let app = empty_test_app().await;
        let now = now();
        create_match_row(
            &app,
            json!({
                "id": "legacy-match",
                "seed": "seed-legacy",
                "players": ["profile-1", "profile-2"],
                "decks": [fake_deck(&[]), fake_deck(&[])],
                "catalogVersion": TEST_CATALOG_VERSION,
                "status": "live",
                "createdAt": now,
                "finishedAt": null,
                "clocks": {
                    "turnDeadline": null,
                    "promptDeadline": null,
                    "graceDeadline": { "p1": null, "p2": null },
                    "ceilingAt": now + ceiling_ms(),
                },
            }),
        )
        .await;

        let socket = Client::new();
        app.matches
            .attach(&app, "legacy-match", "profile-1", socket.socket())
            .await
            .expect("registry.attach");
        let actor = app
            .matches
            .actor_for(&app, "legacy-match")
            .await
            .expect("the actor");
        settle().await;
        actor.idle().await;
        settle().await;

        let default = serde_json::to_value(DEFAULT_PORTRAIT).expect("PortraitId serialises");
        // R1341: an old row still has its seed, so its seats are dealt their hands like any other.
        let hand: Vec<&str> = deal_emote_hand("seed-legacy", P1)
            .iter()
            .map(|id| id.as_str())
            .collect();
        assert_eq!(
            portraits_sent(&socket),
            vec![json!({ "type": "portraits", "p1": default, "p2": default, "emotes": hand })]
        );
    }
}

// ---------------------------------------------------------------------------
// BUILD M5-T2 / §10.2: the array the client greys the board out with
// ---------------------------------------------------------------------------

/// "The client never computes legality itself; it asks `legalActions` and greys out the rest"
/// (BUILD M5-T2), and §10.2 makes `legalActions(state, playerId)` "what both the client UI and the
/// My Pawn AI consume". The socket is the client's only source for it, so it travels on the `view`
/// frame (`protocol.rs` `ViewMessage` says why there and not in a frame of its own).
///
/// The property under test is not "an array arrives" but **whose** array arrives. `legalActions`
/// enumerates the `play`s of the cards in the player's hand, so the other seat's array would hand
/// over every instance id in the opponent's hand — §9.1 lists that first under "Hidden".
mod the_legal_action_array_on_the_view_frame_build_m5_t2_10_2 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn gives_each_seat_its_own_array_and_never_the_other_seats() {
        let h = harness(Options::default()).await;

        // The real `legalActions` names the active player's plays (one per way to play each card it
        // can afford: lanes, targets, R81), plus `endTurn` and `concede`; `[{ type: "concede" }]` for
        // the other seat. TS's scripted port offered one `play` per hand card.
        let p1_hand = hand_ids(&last_view(&h.p1));
        let p2_hand = hand_ids(&last_view(&h.p2));
        assert!(!p1_hand.is_empty());
        assert!(!p2_hand.is_empty());

        let mine = legal_of(&h.p1);
        let plays: Vec<String> = mine
            .iter()
            .filter(|action| action["type"] == "play")
            .map(|action| action["instanceId"].as_str().unwrap_or_default().to_string())
            .collect();
        assert!(plays.contains(&in_hand(&h.p1, "test-prompt-self")));
        assert!(plays.iter().all(|instance_id| p1_hand.contains(instance_id)));
        assert!(legal_types(&h.p1).contains(&"endTurn".to_string()));

        // p2 is not the active player, so its own array is the one the engine gives p2 — and it names
        // none of p1's cards. This is the assertion that would fail if `push_view` ever passed the
        // wrong player to `legal_actions`.
        assert_eq!(legal_of(&h.p2), vec![json!({ "type": "concede" })]);
        let p2_sees = Value::Array(legal_of(&h.p2)).to_string();
        for instance_id in &p1_hand {
            assert!(!p2_sees.contains(instance_id.as_str()));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn re_derives_the_array_after_every_action_for_both_seats() {
        let h = harness(Options::default()).await;

        assert!(legal_types(&h.p1).contains(&"endTurn".to_string()));
        assert_eq!(legal_of(&h.p2), vec![json!({ "type": "concede" })]);

        send(&h, &h.p1, "hand-over", json!({ "type": "endTurn" })).await;

        // The turn moved, so the two arrays swapped — neither client had to work that out.
        assert_eq!(legal_of(&h.p1), vec![json!({ "type": "concede" })]);
        assert!(legal_types(&h.p2).contains(&"endTurn".to_string()));
    }

    #[tokio::test(start_paused = true)]
    async fn an_open_prompt_leaves_the_seat_that_does_not_hold_it_with_nothing_to_do_10_6() {
        let h = harness(Options::default()).await;

        // `test-prompt-self` opens a prompt for the player who played it.
        send(
            &h,
            &h.p1,
            "prompt",
            json!({ "type": "play", "instanceId": in_hand(&h.p1, "test-prompt-self") }),
        )
        .await;

        let choice_id = open_choice(&h.p1);
        // The holder's array is that prompt's answers and nothing else but concede, which R211 offers
        // both seats at every moment of a live game (TS's scripted port offered the answer alone).
        let mine = legal_of(&h.p1);
        assert!(mine.iter().any(|action| action["type"] == "answer"));
        assert!(mine.iter().all(|action| (action["type"] == "answer"
            && action["choiceId"] == json!(choice_id))
            || *action == json!({ "type": "concede" })));
        // §10.6: the opponent "sees only that a prompt is open". Not even the choiceId reaches it —
        // concede, R211's way out, is the whole of what p2 may do (TS's scripted port offered nothing).
        assert_eq!(legal_of(&h.p2), vec![json!({ "type": "concede" })]);
        assert!(
            !Value::Array(legal_of(&h.p2))
                .to_string()
                .contains(choice_id.as_str())
        );
    }

    #[tokio::test(start_paused = true)]
    async fn reconnecting_socket_gets_the_array_with_its_fresh_view_9_5() {
        let h = harness(Options::default()).await;
        let before = legal_of(&h.p1);

        h.p1.drop_transport();
        h.idle().await;

        let revived = Client::new();
        h.actor.attach(P1, revived.socket());
        h.idle().await;

        // The attach pushed one view, and it carries the array: a board rebuilt after a reload is
        // interactive without waiting for the next action to happen (spec 05 reloads mid-prompt).
        assert_eq!(views(&revived).len(), 1);
        assert_eq!(legal_of(&revived), before);

        // `hello` means "push me a fresh full view" (§9.5), and that view is no different.
        revived.receive_json(json!({ "type": "hello" }));
        h.idle().await;
        assert_eq!(views(&revived).len(), 2);
        assert_eq!(legal_of(&revived), before);
    }

    #[tokio::test(start_paused = true)]
    async fn is_empty_once_the_match_has_a_result() {
        let h = harness(Options::default()).await;
        send(&h, &h.p1, "gg", json!({ "type": "concede" })).await;
        assert!(!last_view(&h.p1)["result"].is_null());
        assert_eq!(legal_of(&h.p1), Vec::<Value>::new());
        assert_eq!(legal_of(&h.p2), Vec::<Value>::new());
    }
}

// ---------------------------------------------------------------------------
// M6-T4 acceptance 4 again, with the REAL catalog under the actor
// ---------------------------------------------------------------------------

/// The same acceptance item, against the real §8 catalog.
///
/// Everything above runs on `support::engine`'s scripted cards, whose behaviour this suite wrote —
/// so it proves that the actor and the protocol add no leak *on top of* a redaction. That is fine for
/// the clock, the log and the flood limit, which is what the scripted cards are for, but CLAUDE.md
/// rule 7 ("the client ... never sees hidden information") is the one property where the component
/// under test and the component asserted must not be the same file.
///
/// So this block deals two decks of real §8 card ids through `registry.start`, and runs the leak scan
/// over the bytes the sockets actually received. Three things only exist here:
///
///  - the hands are dealt by `beginGame`'s real opening draw of real cards;
///  - `PlayerView.events` is a real redacted stream (R97) — the `drawn` event for every card in the
///    opponent's opening hand carries that card's `defId` in the state, and R97's `HIDDEN_ID` is the
///    only reason it does not reach the other socket;
///  - the open prompts are two real mulligans, open at once (R265), whose options name each chooser's
///    own hand card by card (`setup.rs` `mulligan_prompt`), so R81's "only the viewer's own prompt
///    carries its options" is carrying real ids rather than the scripted port's `{ key: "none" }`.
mod m6_t4_acceptance_4_with_the_real_engine_10_8_claude_md_rule_7 {
    use super::*;

    /// The real §8 catalog and two disjoint decks of real ids, whose deal opens straight onto both
    /// mulligans (R265): no card it casts asks a question first (R224).
    fn real_engine() -> (Vec<String>, (Vec<String>, Vec<String>)) {
        let pool = deckable_pool();
        let decks = decks_that_open_on_the_mulligans(&pool, SEED);
        (pool, decks)
    }

    /// Keeps the whole hand, which is what makes the scan below sound: nothing leaves a hand.
    fn keep_everything(socket: &Client) -> Value {
        let cards = hand_ids(&last_view(socket));
        assert!(!cards.is_empty());
        json!({ "type": "mulligan", "keep": cards })
    }

    #[tokio::test(start_paused = true)]
    async fn the_opponents_socket_never_receives_the_other_hands_def_ids_through_the_real_view_for() {
        let (pool, decks) = real_engine();
        let (p1_deck, p2_deck) = decks.clone();

        // PREMISE: the two decks are real §8 ids and share none, so a defId found on the wrong socket
        // can only have come from the other player's deck.
        assert_eq!(p1_deck[0], pool[0]);
        assert!(p1_deck.iter().all(|card_id| !p2_deck.contains(card_id)));

        let h = harness(real(decks)).await;

        // §2.1, R265: the game opens on both mulligans at once. Both keep everything, so every card
        // dealt is still in the hand it was dealt to when the scan runs — a card returned to the library
        // would be one the scan could not reason about, and a card played would be public (§10.8). p2
        // answers first, so the scan also covers the frames sent while p2's answer was sealed (R266).
        assert_match(
            &last_view(&h.p1)["pending"],
            &json!({ "forYou": true, "kind": "mulligan" }),
        );
        assert_match(
            &last_view(&h.p2)["pending"],
            &json!({ "forYou": true, "kind": "mulligan" }),
        );
        send(&h, &h.p2, "mull-2", keep_everything(&h.p2)).await;
        assert_match(
            &last_view(&h.p1)["pending"],
            &json!({ "forYou": true, "kind": "mulligan" }),
        );
        assert_eq!(
            last_view(&h.p2)["pending"],
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        send(&h, &h.p1, "mull-1", keep_everything(&h.p1)).await;

        // Out of setup and into the first real turn (§2.1, R10: p1 draws), then a reconnect-style full
        // view push, so the scan covers a view built after play has started as well as the attach ones.
        assert_eq!(h.actor.snapshot().phase, Phase::Main);
        send(&h, &h.p1, "t1", json!({ "type": "endTurn" })).await;
        h.p2.receive_json(json!({ "type": "hello" }));
        h.idle().await;

        let p1_hand: Vec<String> = hand(&last_view(&h.p1))
            .into_iter()
            .map(|card| card.def_id)
            .collect();
        let p2_hand: Vec<String> = hand(&last_view(&h.p2))
            .into_iter()
            .map(|card| card.def_id)
            .collect();
        // PREMISE: both hands really hold real cards. Empty sets would make every scan below vacuous.
        assert!(!p1_hand.is_empty());
        assert!(!p2_hand.is_empty());
        // Every one is a deck card, but for The Coin setup deals p2, who goes second (§2.1, R244), which
        // is as much a secret of p2's hand as any deck card in it.
        for def_id in p1_hand.iter().chain(&p2_hand) {
            assert!(pool.contains(def_id) || def_id == COIN_DEF_ID, "{def_id}");
        }
        assert!(p2_hand.iter().any(|def_id| def_id == COIN_DEF_ID));
        assert!(!p1_hand.iter().any(|def_id| def_id == COIN_DEF_ID));

        // PREMISE: the frames really carry the events the scripted port never produced, so the R97 half
        // of the scan is exercising something.
        assert!(
            !last_view(&h.p1)["events"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .is_empty()
        );

        let mut leaked = Vec::new();
        let mut state_shaped = Vec::new();
        for (socket, secrets) in [(&h.p2, &p1_hand), (&h.p1, &p2_hand)] {
            let (found, shaped) = scan(&socket.sent(), secrets);
            leaked.extend(found);
            state_shaped.extend(shaped);
        }

        assert_eq!(leaked, Vec::<String>::new());
        assert_eq!(state_shaped, Vec::<String>::new());

        // What the opponent does get is a count (§10.8), and the count is right.
        assert_eq!(
            last_view(&h.p2)["opponent"]["hand"],
            json!({ "count": p1_hand.len() })
        );
        assert_eq!(
            last_view(&h.p1)["opponent"]["hand"],
            json!({ "count": p2_hand.len() })
        );
        assert!(last_view(&h.p2)["opponent"]["libraryCount"].as_i64().unwrap_or(0) > 0);
        // §9.1: a library is a count for both players — the viewer's own included.
        assert!(last_view(&h.p2)["you"]["libraryCount"].as_i64().unwrap_or(0) > 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r81_r265_each_seats_real_mulligan_prompt_reaches_only_that_seat_with_both_open_at_once() {
        // The scripted port's prompt carried one `{ key: "none" }` option, so this is the assertion it
        // could not make: the real mulligan prompt names every card in the chooser's hand by instance
        // id and by `defId` (`setup.rs` `mulligan_prompt`, `view_for.rs` `option_view`). R265 opens both
        // at once, so each seat is at the same time the holder of one prompt and the other's opponent.
        let (_pool, decks) = real_engine();
        let h = harness(real(decks)).await;

        for (holder, opponent) in [(&h.p1, &h.p2), (&h.p2, &h.p1)] {
            let pending = last_view(holder)["pending"].clone();
            assert!(pending["forYou"] == json!(true), "a seat holds no prompt");
            // PREMISE: the options really do name the cards, so the negative below is about redaction
            // and not about an empty option list.
            assert_eq!(pending["kind"], json!("mulligan"));
            let options = pending["options"].as_array().cloned().unwrap_or_default();
            assert!(!options.is_empty());
            let option_defs: Vec<String> = options
                .iter()
                .map(|option| option["defId"].as_str().unwrap_or_default().to_string())
                .collect();
            let hand_defs: Vec<String> = hand(&last_view(holder))
                .into_iter()
                .map(|card| card.def_id)
                .collect();
            assert_eq!(option_defs, hand_defs);

            // §10.6, R81: the other seat is shown its own prompt, never this one — not the choiceId, not
            // an option, not a card.
            let theirs = last_view(opponent)["pending"].clone();
            assert_match(&theirs, &json!({ "forYou": true, "kind": "mulligan" }));
            assert_ne!(theirs["choiceId"], pending["choiceId"]);

            // Whole values, never substrings: real instance ids are `c<n>`, so `c2` is a prefix of the
            // other seat's own `c21` and a substring search would report a leak that is not one.
            let mut secrets: Vec<String> = vec![pending["choiceId"].as_str().unwrap_or_default().to_string()];
            for option in &options {
                for key in ["key", "instanceId", "defId"] {
                    if let Some(text) = option[key].as_str() {
                        secrets.push(text.to_string());
                    }
                }
            }
            let (seen, _) = scan(&opponent.sent(), &secrets);
            assert_eq!(seen, Vec::<String>::new());
        }
    }
}

// ---------------------------------------------------------------------------
// The concurrent mulligan through the actor (R265, R266, R268)
// ---------------------------------------------------------------------------

/// Both seats' mulligans are open at once (R265) and live outside `pending`, so they are the one time
/// two seats owe an answer together. What the actor adds to the engine here is its own: the prompt
/// frames each seat is sent, the one mulligan clock (R268) and the timeouts its expiry becomes. The
/// rules — what an answer may keep, that it is sealed, what a timeout keeps — are the engine's, and
/// `crates/engine` proves them.
///
/// These run the real engine and the real clock: the fields under test are the real `view_for`'s
/// (`pending`, `mulligan`), and the claims about the deadline are the clock's own.
mod the_concurrent_mulligan_through_the_actor_r265_r266_r268 {
    use super::*;

    /// Decks whose deal opens straight onto both mulligans, so each test starts in the window (R224).
    fn real_decks() -> (Vec<String>, Vec<String>) {
        decks_that_open_on_the_mulligans(&deckable_pool(), SEED)
    }

    async fn mulligan_match() -> Harness {
        harness(real(real_decks())).await
    }

    fn prompts(socket: &Client) -> Vec<Value> {
        socket.of_type("prompt")
    }

    fn ids(socket: &Client) -> Vec<String> {
        hand_ids(&last_view(socket))
    }

    fn choice_of(socket: &Client) -> String {
        let pending = last_view(socket)["pending"].clone();
        assert!(
            pending["forYou"] == json!(true) && pending["kind"] == json!("mulligan"),
            "this seat owes no mulligan"
        );
        pending["choiceId"].as_str().expect("a choice id").to_string()
    }

    fn rows_by_type_and_seat(rows: &[Value]) -> Vec<(String, String)> {
        rows.iter()
            .map(|row| {
                (
                    row["action"]["type"].as_str().unwrap_or_default().to_string(),
                    row["action"]["playerId"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect()
    }

    fn pair(kind: &str, seat: &str) -> (String, String) {
        (kind.to_string(), seat.to_string())
    }

    #[tokio::test(start_paused = true)]
    async fn r265_the_actor_accepts_mulligans_from_both_seats_in_either_order_and_each_push_fits_each_seat() {
        for first in [P2, P1] {
            let second = opponent_of(first);
            let h = mulligan_match().await;
            let deadline = h.clocks().await["promptDeadline"].clone();

            // Both prompts are open from the start, each seat holding its own, with its own answers and
            // concede on its legal array (R211) — and one clock between them (R268).
            for seat in PLAYER_IDS {
                let socket = h.socket(seat);
                assert_match(
                    &last_view(socket)["pending"],
                    &json!({ "forYou": true, "kind": "mulligan" }),
                );
                assert_eq!(
                    last_view(socket)["mulligan"],
                    json!({ "youReady": false, "opponentReady": false })
                );
                assert_eq!(last_view(socket)["clockMs"].as_i64(), Some(mulligan_ms()));
                let legal = legal_of(socket);
                assert!(legal.iter().any(|action| action["type"] == "mulligan"));
                assert!(legal.contains(&json!({ "type": "concede" })));
                for action in legal.iter().filter(|action| action["type"] == "mulligan") {
                    for id in action["keep"].as_array().cloned().unwrap_or_default() {
                        assert!(ids(socket).contains(&id.as_str().unwrap_or_default().to_string()));
                    }
                }
            }
            let at = h.actor.snapshot();
            assert_eq!((at.phase, at.turn, at.pending_for), (Phase::Mulligan, 0, None));
            assert_eq!(at.mulligan_owed, vec![P1, P2]);

            // The first answer, which returns two cards, is sealed: it is in, and nothing moves (R266).
            let first_hand = ids(h.socket(first));
            let first_keep: Vec<String> = first_hand.iter().skip(2).cloned().collect();
            let second_choice = choice_of(h.socket(second));
            h.p1.clear();
            h.p2.clear();
            send(
                &h,
                h.socket(first),
                &format!("mull-{first}"),
                json!({ "type": "mulligan", "keep": first_keep }),
            )
            .await;

            assert_eq!(
                last_view(h.socket(first))["pending"],
                json!({ "forYou": false, "pendingFor": second.as_str() })
            );
            assert_eq!(
                last_view(h.socket(first))["mulligan"],
                json!({ "youReady": true, "opponentReady": false, "kept": first_keep })
            );
            assert_eq!(ids(h.socket(first)), first_hand);
            assert_eq!(legal_of(h.socket(first)), vec![json!({ "type": "concede" })]);
            assert_match(
                &last_view(h.socket(second))["pending"],
                &json!({ "forYou": true, "choiceId": second_choice }),
            );
            // That the other seat is ready is all the second seat is told: no `kept` of anyone's.
            assert_eq!(
                last_view(h.socket(second))["mulligan"],
                json!({ "youReady": false, "opponentReady": true })
            );
            assert_eq!(h.actor.snapshot().mulligan_owed, vec![second]);

            // Each seat's prompt frame is the one that fits it, and both carry the one deadline, unmoved.
            assert_eq!(
                prompts(h.socket(first)),
                vec![
                    json!({ "type": "prompt", "forYou": false, "pendingFor": second.as_str(), "deadline": deadline })
                ]
            );
            assert_eq!(
                prompts(h.socket(second)),
                vec![json!({
                    "type": "prompt",
                    "forYou": true,
                    "pendingFor": second.as_str(),
                    "choiceId": second_choice,
                    "kind": "mulligan",
                    "deadline": deadline,
                })]
            );
            assert_eq!(h.clocks().await["promptDeadline"], deadline);

            // The second answer resolves both, in seat order, and the game begins (R265, §2.1). A turn
            // with nothing to do ends itself inside the same action (§2.5, R82), so whose turn is at rest
            // depends on the hands dealt — which is why the seats below are read, not assumed.
            let keep = ids(h.socket(second));
            send(
                &h,
                h.socket(second),
                &format!("mull-{second}"),
                json!({ "type": "mulligan", "keep": keep }),
            )
            .await;
            let active = h.actor.snapshot().active;
            for seat in PLAYER_IDS {
                let view = last_view(h.socket(seat));
                assert_match(
                    &view,
                    &json!({ "phase": "main", "active": active.as_str(), "pending": null }),
                );
                assert!(view["turn"].as_i64().unwrap_or(0) >= 1);
                assert!(view.get("mulligan").is_none());
            }
            assert_eq!(h.actor.snapshot().mulligan_owed, Vec::<PlayerId>::new());
            // The first seat's two returned cards were replaced: same count, two new ids.
            assert!(
                ids(h.socket(first))
                    .iter()
                    .filter(|id| !first_keep.contains(*id))
                    .count()
                    >= 2
            );

            // One row per answer, in the order they came, stamped with the seat that sent it (§9.1, §9.3).
            assert_eq!(
                rows_by_type_and_seat(&h.rows().await),
                vec![
                    pair("mulligan", first.as_str()),
                    pair("mulligan", second.as_str())
                ]
            );
            // The mulligan clock is gone, and the active seat's turn clock runs from full.
            let clocks = h.clocks().await;
            assert!(clocks["promptDeadline"].is_null());
            assert_eq!(clocks["turnDeadline"].as_i64(), Some(now() + turn_ms()));
            assert_eq!(last_view(h.socket(active))["clockMs"].as_i64(), Some(turn_ms()));
            assert!(last_view(h.socket(opponent_of(active)))["clockMs"].is_null());
            h.actor.stop().await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r266_a_sealed_answer_never_reaches_the_other_seat_its_view_changes_by_the_ready_flag_alone() {
        let h = mulligan_match().await;
        let before = last_view(&h.p1);
        let p2_hand = hand(&last_view(&h.p2));
        let p2_keep: Vec<String> = p2_hand
            .iter()
            .take(2)
            .map(|card| card.instance_id.clone())
            .collect();
        assert!(p2_hand.len() > 2);

        send(
            &h,
            &h.p2,
            "sealed",
            json!({ "type": "mulligan", "keep": p2_keep }),
        )
        .await;

        // Everything p1 is shown is what it was shown before, but that p2 is ready (and the public
        // `promptAnswered` that says so). No part of the view depends on what p2 kept.
        let after = last_view(&h.p1);
        let strip = |view: &Value| {
            let mut rest = view.clone();
            if let Some(fields) = rest.as_object_mut() {
                fields.remove("mulligan");
                fields.remove("events");
            }
            rest
        };
        assert_eq!(strip(&after), strip(&before));
        assert_eq!(
            after["mulligan"],
            json!({ "youReady": false, "opponentReady": true })
        );
        assert_match(
            &after["events"]
                .as_array()
                .and_then(|events| events.last().cloned())
                .unwrap_or_default(),
            &json!({ "type": "promptAnswered", "player": "p2" }),
        );

        // And no frame p1's socket ever received names one of p2's cards, by instance or by definition.
        let secrets: Vec<String> = p2_hand
            .iter()
            .flat_map(|card| [card.instance_id.clone(), card.def_id.clone()])
            .collect();
        let (leaked, _) = scan(&h.p1.sent(), &secrets);
        assert_eq!(leaked, Vec::<String>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn r268_the_mulligan_clock_is_one_deadline_for_both_seats_not_re_armed_when_one_answers_and_times_out_the_seat_left()
     {
        let h = mulligan_match().await;
        let armed_at = now();
        let deadline = h.clocks().await["promptDeadline"].as_i64();

        // One deadline, stored on the match so both clients render it (§9.5), and no turn clock under it.
        assert_eq!(deadline, Some(armed_at + mulligan_ms()));
        assert!(h.clocks().await["turnDeadline"].is_null());
        assert_eq!(
            match_row(&h.app, MATCH_ID).await["clocks"]["promptDeadline"].as_i64(),
            deadline
        );

        // p1 answers 10 s in. Both seats are still on the same deadline, the one that answered included.
        advance(10 * SECOND).await;
        send(
            &h,
            &h.p1,
            "p1-first",
            json!({ "type": "mulligan", "keep": ids(&h.p1) }),
        )
        .await;
        assert_eq!(h.clocks().await["promptDeadline"].as_i64(), deadline);
        for socket in [&h.p1, &h.p2] {
            let shown = socket.of_type("clock").last().cloned().expect("a clock frame");
            assert_eq!(shown["clocks"]["promptDeadline"].as_i64(), deadline);
        }
        assert_eq!(
            last_view(&h.p1)["clockMs"].as_i64(),
            Some(mulligan_ms() - 10 * SECOND)
        );
        assert_eq!(
            last_view(&h.p2)["clockMs"].as_i64(),
            Some(mulligan_ms() - 10 * SECOND)
        );

        let pending = last_view(&h.p2)["pending"].clone();
        assert!(pending["forYou"] == json!(true), "p2 owes no mulligan");
        let offered: Vec<String> = pending["options"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|option| option["key"].as_str().unwrap_or_default().to_string())
            .collect();

        // Nothing happens a millisecond early; at the deadline p2, the one seat still owing, times out.
        advance(mulligan_ms() - 10 * SECOND - 1).await;
        h.idle().await;
        assert_eq!(h.rows().await.len(), 1);
        advance(1).await;
        h.idle().await;

        let log = h.rows().await;
        assert_eq!(log.len(), 2);
        assert_match(
            &log[1]["action"],
            &json!({ "type": "timeout", "playerId": "p2", "nonce": "srv-mulligan-2" }),
        );
        // R268: a timed-out mulligan keeps the whole hand — every card p2 was offered is still there.
        for id in &offered {
            assert!(ids(&h.p2).contains(id));
        }
        let at = h.actor.snapshot();
        assert_eq!(
            (at.phase, at.mulligan_owed.len(), at.result),
            (Phase::Main, 0, None)
        );

        // And the mulligan clock hands over to the ordinary turn clock, from full.
        let clocks = h.clocks().await;
        assert!(clocks["promptDeadline"].is_null());
        assert_eq!(clocks["turnDeadline"].as_i64(), Some(now() + turn_ms()));
        assert_eq!(
            last_view(h.socket(at.active))["clockMs"].as_i64(),
            Some(turn_ms())
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r270_a_client_may_not_send_a_nonce_the_server_mints_so_it_cannot_swallow_the_clocks_own_timeout()
    {
        let h = mulligan_match().await;
        // p1 answers with the very nonce the mulligan expiry would mint for p2's timeout (seq 2).
        send(
            &h,
            &h.p1,
            "srv-mulligan-2",
            json!({ "type": "mulligan", "keep": ids(&h.p1) }),
        )
        .await;
        let refused = errors(&h.p1).last().cloned().expect("an error frame");
        assert_match(&refused, &json!({ "code": "malformed" }));
        assert!(refused["message"].as_str().unwrap_or_default().contains("R270"));
        assert_eq!(h.rows().await.len(), 0);
        assert_eq!(h.actor.snapshot().mulligan_owed, vec![P1, P2]);

        // Under any other nonce the answer stands, and the expiry still times out the seat left.
        send(
            &h,
            &h.p1,
            "p1-ready",
            json!({ "type": "mulligan", "keep": ids(&h.p1) }),
        )
        .await;
        assert_eq!(h.actor.snapshot().mulligan_owed, vec![P2]);
        advance(mulligan_ms()).await;
        h.idle().await;
        assert_match(
            &h.last_row().await["action"],
            &json!({ "type": "timeout", "playerId": "p2", "nonce": "srv-mulligan-2" }),
        );
        let at = h.actor.snapshot();
        assert_eq!((at.mulligan_owed.len(), at.result), (0, None));
        assert_eq!(errors(&h.p2), Vec::<Value>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn r268_when_neither_seat_answers_the_expiry_times_out_both_in_seat_order_each_its_own_row() {
        let h = mulligan_match().await;
        let hands = (ids(&h.p1), ids(&h.p2));

        advance(mulligan_ms()).await;
        h.idle().await;

        let log = h.rows().await;
        assert_eq!(log.len(), 2);
        assert_match(
            &log[0]["action"],
            &json!({ "type": "timeout", "playerId": "p1", "nonce": "srv-mulligan-1" }),
        );
        assert_match(
            &log[1]["action"],
            &json!({ "type": "timeout", "playerId": "p2", "nonce": "srv-mulligan-2" }),
        );
        assert_eq!(log.iter().map(seq_of).collect::<Vec<i64>>(), vec![1, 2]);
        // Both kept everything (R268); what else is in each hand is turn 1's draw (R10) and The Coin (R244).
        for id in &hands.0 {
            assert!(ids(&h.p1).contains(id));
        }
        for id in &hands.1 {
            assert!(ids(&h.p2).contains(id));
        }
        let at = h.actor.snapshot();
        assert_eq!(
            (at.phase, at.mulligan_owed.len(), at.result),
            (Phase::Main, 0, None)
        );
        assert_eq!(h.clocks().await["turnDeadline"].as_i64(), Some(now() + turn_ms()));
    }

    #[tokio::test(start_paused = true)]
    async fn r268_the_turn_clock_that_follows_the_mulligan_clock_is_p1s_on_turn_1_from_full() {
        // The scripted port never ended a turn by itself, so turn 1 was at rest when the window closed
        // and the hand-over could be read exactly: the real clock, the actor's two timeouts, then p1's
        // turn. The scripted cards run under the real engine here, so the walk turns R345's automatic
        // turn end off for both seats first (its two rows are `base`).
        let h = harness(Options {
            walk: Walk::AutoEndOff,
            ..Options::default()
        })
        .await;
        assert_eq!(last_view(&h.p1)["clockMs"].as_i64(), Some(mulligan_ms()));
        assert_eq!(last_view(&h.p2)["clockMs"].as_i64(), Some(mulligan_ms()));

        advance(mulligan_ms()).await;
        h.idle().await;

        assert_eq!(
            rows_by_type_and_seat(&h.rows().await),
            vec![pair("timeout", "p1"), pair("timeout", "p2")]
        );
        let at = h.actor.snapshot();
        assert_eq!(
            (at.phase, at.turn, at.active, at.mulligan_owed.len()),
            (Phase::Main, 1, P1, 0)
        );
        let clocks = h.clocks().await;
        assert!(clocks["promptDeadline"].is_null());
        assert_eq!(clocks["turnDeadline"].as_i64(), Some(now() + turn_ms()));
        assert_eq!(last_view(&h.p1)["clockMs"].as_i64(), Some(turn_ms()));
        assert!(last_view(&h.p2)["clockMs"].is_null());

        // And it is an ordinary turn clock: it runs out in `turnClockSeconds` and ends p1's turn.
        advance(turn_ms()).await;
        h.idle().await;
        assert_match(
            &h.last_row().await["action"],
            &json!({ "type": "timeout", "playerId": "p1" }),
        );
        let at = h.actor.snapshot();
        assert_eq!((at.turn, at.active), (2, P2));
    }

    #[tokio::test(start_paused = true)]
    async fn r268_an_expiry_reads_who_still_owes_when_it_runs_so_a_seat_that_answered_gets_no_timeout() {
        // The scripted cards, left on both mulligans with the automatic turn end off (TS
        // `createFakeEngine({ mulligan: true })`). TS's stub clock fired the expiry on demand; the real
        // one fires at its deadline, and once only.
        let h = harness(Options {
            walk: Walk::AutoEndOff,
            ..Options::default()
        })
        .await;
        // The actor hands the clock the window, both seats owing, before anyone acts.
        let at = h.actor.snapshot();
        assert_eq!((at.mulligan_owed.clone(), at.pending_for), (vec![P1, P2], None));

        send(&h, &h.p2, "p2-first", json!({ "type": "mulligan", "keep": [] })).await;
        assert_eq!(h.actor.snapshot().mulligan_owed, vec![P1]);
        advance(mulligan_ms()).await;
        h.idle().await;
        assert_eq!(
            rows_by_type_and_seat(&h.rows().await),
            vec![pair("mulligan", "p2"), pair("timeout", "p1")]
        );
        let at = h.actor.snapshot();
        assert_eq!((at.mulligan_owed.len(), at.phase, at.turn), (0, Phase::Main, 1));

        // No second alarm, once both are in: the window's clock fires once (R268) and nothing more is
        // written. (TS fired a late alarm by hand here; with the real clock nothing can.)
        advance(mulligan_ms()).await;
        h.idle().await;
        assert_eq!(h.rows().await.len(), 2);
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(errors(&h.p2), Vec::<Value>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn r265_refuses_anything_but_a_mulligan_or_a_way_out_while_the_mulligans_are_open_relaying_the_engine()
     {
        let h = mulligan_match().await;

        send(&h, &h.p1, "too-soon", json!({ "type": "endTurn" })).await;
        assert_match(
            &errors(&h.p1).last().cloned().expect("an error frame"),
            &json!({ "code": "illegal_action", "nonce": "too-soon" }),
        );
        // A seat cannot answer twice: its answer is sealed, not open to revision (R266).
        send(&h, &h.p2, "once", json!({ "type": "mulligan", "keep": [] })).await;
        send(&h, &h.p2, "twice", json!({ "type": "mulligan", "keep": [] })).await;
        assert_match(
            &errors(&h.p2).last().cloned().expect("an error frame"),
            &json!({ "code": "illegal_action", "nonce": "twice" }),
        );
        let nonces: Vec<Value> = h
            .rows()
            .await
            .iter()
            .map(|row| row["action"]["nonce"].clone())
            .collect();
        assert_eq!(nonces, vec![json!("once")]);
    }

    #[tokio::test(start_paused = true)]
    async fn r265_a_seat_may_concede_during_the_mulligan_and_the_loss_is_recorded_exactly_once_9_5() {
        let h = harness(Options {
            ratings: [1000.0, 1000.0],
            ..real(real_decks())
        })
        .await;

        send(&h, &h.p1, "p1-ready", json!({ "type": "mulligan", "keep": [] })).await;
        send(&h, &h.p2, "gg", json!({ "type": "concede" })).await;
        assert_eq!(
            last_view(&h.p1)["result"],
            json!({ "winner": "p1", "reason": "concede" })
        );
        assert_eq!(
            last_view(&h.p2)["result"],
            json!({ "winner": "p1", "reason": "concede" })
        );

        // Once, however many more times anyone asks.
        send(&h, &h.p2, "gg-again", json!({ "type": "concede" })).await;
        send(&h, &h.p1, "gg-too", json!({ "type": "concede" })).await;
        assert_match(
            &errors(&h.p2).last().cloned().expect("an error"),
            &json!({ "code": "match_over", "nonce": "gg-again" }),
        );
        assert_match(
            &errors(&h.p1).last().cloned().expect("an error"),
            &json!({ "code": "match_over", "nonce": "gg-too" }),
        );

        let (won_a, won_b) = rating_move(1000.0, 1000.0, 1.0);
        let result = result_row(&h.app, MATCH_ID).await.expect("the result row");
        assert_match(
            &result,
            &json!({
                "matchId": MATCH_ID,
                "winnerProfileId": "profile-1",
                "reason": "concede",
                "ratingBefore": [1000.0, 1000.0],
                "ratingAfter": [won_a, won_b],
            }),
        );
        assert_eq!(match_row(&h.app, MATCH_ID).await["status"], json!("finished"));
        // No clock is left to fire into a finished match: a whole ceiling's worth of time writes nothing
        // (TS counted the manual timers' queue empty; tokio keeps no such count).
        let rows = h.rows().await.len();
        advance(ceiling_ms()).await;
        h.idle().await;
        assert_eq!(h.rows().await.len(), rows);
    }
}

// ---------------------------------------------------------------------------
// Draw offers and concede through the actor (R36, R269, §9.5)
// ---------------------------------------------------------------------------

/// A draw offer is an engine rule end to end (R36): who may offer (the active player, in the main
/// phase), how often (`DRAW_OFFERS_PER_TURN`), how long a decline blocks (`DRAW_OFFER_BLOCK_TURNS`)
/// and how long an unanswered offer stands (R269) are all `crates/engine`'s, because hotseat and
/// practice play them with no server at all. The actor only relays: an offer is an action like any
/// other, its refusal is the reducer's sentence, and an accepted draw is one more ending for the
/// results writer. So these run the real engine and the real results writer over the in-memory
/// store, so "rated 0.5 each" is the real rating move.
mod draw_offers_and_concede_through_the_actor_r36_r269_9_5 {
    use super::*;

    struct DrawHarness {
        h: Harness,
        /// The seat at rest in its main phase once the mulligans are done, who may offer (R36).
        offerer_seat: PlayerId,
    }

    impl DrawHarness {
        fn offerer(&self) -> &Client {
            self.h.socket(self.offerer_seat)
        }

        /// The other seat, who answers.
        fn answerer(&self) -> &Client {
            self.h.socket(opponent_of(self.offerer_seat))
        }
    }

    async fn draw_match(ratings: Option<[f64; 2]>) -> DrawHarness {
        let decks = decks_that_open_on_the_mulligans(&deckable_pool(), SEED);
        let h = harness(Options {
            ratings: ratings.unwrap_or([1000.0, 1000.0]),
            ..real(decks)
        })
        .await;
        // Past both mulligans (R265), keeping everything, into the first main phase at rest.
        for player in PLAYER_IDS {
            let socket = h.socket(player);
            let keep = hand_ids(&last_view(socket));
            send(
                &h,
                socket,
                &format!("mull-{player}"),
                json!({ "type": "mulligan", "keep": keep }),
            )
            .await;
        }
        // A turn with nothing to do ends itself (§2.5, R82), so whose turn is at rest depends on the
        // hands dealt. Whose it is does not matter to R36, so it is read rather than assumed.
        let at = h.actor.snapshot();
        assert_eq!((at.phase, at.pending_for, at.result), (Phase::Main, None, None));
        DrawHarness {
            offerer_seat: at.active,
            h,
        }
    }

    /// Ends the turn of whoever is active, and says so if the engine refused.
    async fn end_turn(h: &Harness, nonce: &str) {
        let active = h.actor.snapshot().active;
        let socket = h.socket(active);
        send(h, socket, nonce, json!({ "type": "endTurn" })).await;
        let refused: Vec<Value> = errors(socket)
            .into_iter()
            .filter(|error| error["nonce"] == nonce)
            .collect();
        assert_eq!(refused, Vec::<Value>::new());
        assert_eq!(h.actor.snapshot().pending_for, None);
    }

    /// Ends turns until the offerer's next turn is at rest (a turn with nothing to do ends itself).
    async fn to_offerers_next_turn(d: &DrawHarness) {
        let from = d.h.actor.snapshot().turn;
        for step in 0..4 {
            end_turn(&d.h, &format!("to-next-{step}")).await;
            let at = d.h.actor.snapshot();
            if at.active == d.offerer_seat && at.turn > from {
                return;
            }
        }
        panic!("the offerer's next turn never came to rest");
    }

    #[tokio::test(start_paused = true)]
    async fn r269_a_standing_offer_is_on_both_seats_views_and_accepting_it_ends_the_match_as_a_draw_rated_0_5_each()
     {
        let ratings = [1200.0, 1000.0];
        let d = draw_match(Some(ratings)).await;
        let h = &d.h;

        // R36: only the active player may offer, so only the offerer's array carries it.
        assert!(legal_types(d.offerer()).contains(&"offerDraw".to_string()));
        assert!(!legal_types(d.answerer()).contains(&"offerDraw".to_string()));
        assert!(last_view(d.offerer()).get("drawOffer").is_none());

        send(h, d.offerer(), "offer", json!({ "type": "offerDraw" })).await;
        // R269: public to both seats, so the offerer can see it is waiting and the other seat can answer.
        assert_eq!(
            last_view(d.offerer())["drawOffer"],
            json!({ "by": d.offerer_seat.as_str() })
        );
        assert_eq!(
            last_view(d.answerer())["drawOffer"],
            json!({ "by": d.offerer_seat.as_str() })
        );
        assert!(legal_types(d.answerer()).contains(&"answerDraw".to_string()));

        send(
            h,
            d.answerer(),
            "accept",
            json!({ "type": "answerDraw", "accept": true }),
        )
        .await;
        assert_eq!(
            last_view(d.offerer())["result"],
            json!({ "winner": "draw", "reason": "draw-accepted" })
        );
        assert_eq!(
            last_view(d.answerer())["result"],
            json!({ "winner": "draw", "reason": "draw-accepted" })
        );
        assert!(last_view(d.offerer()).get("drawOffer").is_none());

        // §9.5: one result, through the writer every ending goes through, rated as a draw — 0.5 each,
        // which from unequal ratings is a real move toward each other (R603).
        let (drawn_a, drawn_b) = rating_move(ratings[0], ratings[1], 0.5);
        assert!(drawn_a < ratings[0]);
        assert!(drawn_b > ratings[1]);
        let result = result_row(&h.app, MATCH_ID).await.expect("the result row");
        assert_match(
            &result,
            &json!({
                "matchId": MATCH_ID,
                "winnerProfileId": null,
                "reason": "draw-accepted",
                "ratingBefore": ratings,
                "ratingAfter": [drawn_a, drawn_b],
            }),
        );
        let rows: Vec<(Value, Value)> = profiles(&h.app, &["profile-1", "profile-2"])
            .await
            .iter()
            .map(|profile| (profile["rating"].clone(), profile["inMatchId"].clone()))
            .collect();
        assert_eq!(
            rows,
            vec![(json!(drawn_a), Value::Null), (json!(drawn_b), Value::Null)]
        );
        assert_eq!(match_row(&h.app, MATCH_ID).await["status"], json!("finished"));
    }

    #[tokio::test(start_paused = true)]
    async fn r36_a_declined_offer_leaves_the_match_live_and_blocks_the_offerer_by_the_engines_rule_and_in_its_words()
     {
        let d = draw_match(None).await;
        let h = &d.h;

        send(h, d.offerer(), "offer", json!({ "type": "offerDraw" })).await;
        send(
            h,
            d.answerer(),
            "decline",
            json!({ "type": "answerDraw", "accept": false }),
        )
        .await;
        assert_eq!(h.actor.snapshot().result, None);
        assert!(last_view(d.offerer()).get("drawOffer").is_none());
        assert!(last_view(d.answerer()).get("drawOffer").is_none());
        assert!(!legal_types(d.answerer()).contains(&"answerDraw".to_string()));

        // The offerer can no longer offer: not on its array, and refused by the reducer if sent anyway,
        // with the reducer's own sentence relayed and no row written (§9.3, §9.8).
        assert!(!legal_types(d.offerer()).contains(&"offerDraw".to_string()));
        let rows = h.rows().await.len();
        send(h, d.offerer(), "again", json!({ "type": "offerDraw" })).await;
        assert_eq!(
            errors(d.offerer()).last(),
            Some(&json!({
                "type": "error",
                "code": "illegal_action",
                "message": "you cannot offer a draw right now",
                "nonce": "again",
            }))
        );
        assert_eq!(h.rows().await.len(), rows);

        // R36's block outlasts the turn: the offerer's next turn still offers no draw.
        to_offerers_next_turn(&d).await;
        assert!(!legal_types(d.offerer()).contains(&"offerDraw".to_string()));
        assert!(result_row(&h.app, MATCH_ID).await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn r269_an_unanswered_offer_lapses_when_its_offerers_turn_ends_and_blocks_nothing() {
        let d = draw_match(None).await;
        let h = &d.h;

        send(h, d.offerer(), "offer", json!({ "type": "offerDraw" })).await;
        assert_eq!(
            last_view(d.answerer())["drawOffer"],
            json!({ "by": d.offerer_seat.as_str() })
        );

        end_turn(h, "end-1").await;
        // Gone from both views and from the array of the seat that could have answered it.
        assert!(last_view(d.offerer()).get("drawOffer").is_none());
        assert!(last_view(d.answerer()).get("drawOffer").is_none());
        assert!(!legal_types(d.answerer()).contains(&"answerDraw".to_string()));
        send(
            h,
            d.answerer(),
            "late",
            json!({ "type": "answerDraw", "accept": true }),
        )
        .await;
        assert_match(
            &errors(d.answerer()).last().cloned().expect("an error frame"),
            &json!({ "code": "illegal_action", "message": "there is no draw offer to answer", "nonce": "late" }),
        );
        assert_eq!(h.actor.snapshot().result, None);

        // A lapse is not a decline (R269): the offerer may offer again on its next turn.
        to_offerers_next_turn(&d).await;
        assert!(legal_types(d.offerer()).contains(&"offerDraw".to_string()));
    }

    #[tokio::test(start_paused = true)]
    async fn concede_records_the_loss_once_and_rates_it_as_one_9_5() {
        let d = draw_match(Some([1000.0, 1000.0])).await;
        let h = &d.h;

        // Conceded by p1, whoever's turn it is (concede is open to both seats at all times, §2.5).
        send(h, &h.p1, "resign", json!({ "type": "concede" })).await;
        assert_eq!(
            last_view(&h.p2)["result"],
            json!({ "winner": "p2", "reason": "concede" })
        );
        send(h, &h.p1, "resign-again", json!({ "type": "concede" })).await;
        assert_match(
            &errors(&h.p1).last().cloned().expect("an error frame"),
            &json!({ "code": "match_over", "nonce": "resign-again" }),
        );

        let (lost_a, lost_b) = rating_move(1000.0, 1000.0, 0.0);
        let result = result_row(&h.app, MATCH_ID).await.expect("the result row");
        assert_match(
            &result,
            &json!({ "winnerProfileId": "profile-2", "reason": "concede", "ratingAfter": [lost_a, lost_b] }),
        );
    }
}

// ---------------------------------------------------------------------------
// The `ws` adapter: a real listener and the smallest WebSocket client (TS `fakeWs`)
// ---------------------------------------------------------------------------

/// One frame off the wire.
#[derive(Debug)]
enum Frame {
    Text(String),
    Binary,
    Close(Option<u16>),
    Ping,
    Pong,
}

/// A WebSocket client of the smallest kind (RFC 6455): enough to open `/ws/match`, send text and
/// binary frames, and read the server's frames and its close. No crate in the stack is one, and the
/// `ws` adapter TS faked (`socketFromWs`) is inside `actor::ws_server::handle` now, so the adapter is
/// reached the way a browser reaches it.
struct WsClient {
    stream: tokio::net::TcpStream,
    buffer: Vec<u8>,
}

/// How long a test waits for the server's next frame before it says so.
const WS_WAIT: Duration = Duration::from_secs(10);

impl WsClient {
    /// Opens `path_and_query` with the upgrade a browser sends. `Err` carries a refused handshake's
    /// HTTP status.
    async fn connect(addr: SocketAddr, path_and_query: &str) -> Result<WsClient, u16> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut stream = tokio::net::TcpStream::connect(addr)
            .await
            .expect("the test server listens");
        let request = format!(
            "GET {path_and_query} HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("the upgrade request is written");
        let mut buffer: Vec<u8> = Vec::new();
        let head_end = loop {
            if let Some(at) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                break at + 4;
            }
            let mut chunk = [0u8; 4096];
            let read = tokio::time::timeout(WS_WAIT, stream.read(&mut chunk))
                .await
                .expect("the handshake is answered in time")
                .expect("the handshake reads");
            assert!(read > 0, "the server closed the connection during the handshake");
            buffer.extend_from_slice(&chunk[..read]);
        };
        let head = String::from_utf8_lossy(&buffer[..head_end]).to_string();
        let status: u16 = head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0);
        if status != 101 {
            return Err(status);
        }
        buffer.drain(..head_end);
        Ok(WsClient { stream, buffer })
    }

    /// `n` more bytes off the stream, or `None` at its end.
    async fn take(&mut self, n: usize) -> Option<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        while self.buffer.len() < n {
            let mut chunk = [0u8; 4096];
            let read = tokio::time::timeout(WS_WAIT, self.stream.read(&mut chunk))
                .await
                .expect("the server answers in time")
                .unwrap_or(0);
            if read == 0 {
                return None;
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
        Some(self.buffer.drain(..n).collect())
    }

    /// The next frame, or `None` when the connection ended.
    async fn next_frame(&mut self) -> Option<Frame> {
        let head = self.take(2).await?;
        let opcode = head[0] & 0x0f;
        let masked = head[1] & 0x80 != 0;
        let mut len = usize::from(head[1] & 0x7f);
        if len == 126 {
            let bytes = self.take(2).await?;
            len = usize::from(u16::from_be_bytes([bytes[0], bytes[1]]));
        } else if len == 127 {
            let bytes = self.take(8).await?;
            let mut wide = [0u8; 8];
            wide.copy_from_slice(&bytes);
            len = usize::try_from(u64::from_be_bytes(wide)).expect("a frame that fits in memory");
        }
        let mask = if masked { Some(self.take(4).await?) } else { None };
        let mut payload = self.take(len).await?;
        if let Some(mask) = mask {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[i % 4];
            }
        }
        Some(match opcode {
            0x1 => Frame::Text(String::from_utf8(payload).expect("a text frame is UTF-8")),
            0x8 => Frame::Close((payload.len() >= 2).then(|| u16::from_be_bytes([payload[0], payload[1]]))),
            0x9 => Frame::Ping,
            0xA => Frame::Pong,
            _ => Frame::Binary,
        })
    }

    /// One frame from the client: always masked (RFC 6455 §5.3).
    async fn send_frame(&mut self, opcode: u8, payload: &[u8]) {
        use tokio::io::AsyncWriteExt;

        let mut frame = vec![0x80 | opcode];
        let len = payload.len();
        if len < 126 {
            frame.push(0x80 | u8::try_from(len).expect("under 126"));
        } else if let Ok(short) = u16::try_from(len) {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&short.to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&u64::try_from(len).expect("a length").to_be_bytes());
        }
        let mask = [0x37u8, 0xfa, 0x21, 0x3d];
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
        self.stream.write_all(&frame).await.expect("the frame is written");
    }

    async fn send_text(&mut self, text: &str) {
        self.send_frame(0x1, text.as_bytes()).await;
    }

    async fn send_binary(&mut self, bytes: &[u8]) {
        self.send_frame(0x2, bytes).await;
    }

    async fn close(&mut self, code: u16) {
        self.send_frame(0x8, &code.to_be_bytes()).await;
    }

    /// Reads until a text frame of `kind` arrives, and answers it parsed.
    async fn next_of_type(&mut self, kind: &str) -> Value {
        loop {
            match self.next_frame().await {
                Some(Frame::Text(text)) => {
                    let message: Value = serde_json::from_str(&text).expect("every frame is JSON");
                    if message["type"] == kind {
                        return message;
                    }
                }
                Some(Frame::Close(code)) => panic!("closed ({code:?}) before a {kind} frame"),
                None => panic!("the connection ended before a {kind} frame"),
                Some(_) => {}
            }
        }
    }

    /// Reads to the server's close: every text frame on the way, and the close code.
    async fn until_close(&mut self) -> (Vec<String>, Option<u16>) {
        let mut texts = Vec::new();
        loop {
            match self.next_frame().await {
                Some(Frame::Text(text)) => texts.push(text),
                Some(Frame::Close(code)) => return (texts, code),
                None => return (texts, None),
                Some(_) => {}
            }
        }
    }
}

/// The test app behind a real listener on a free port, as `app::serve` runs it.
struct Listening {
    app: Arc<App>,
    addr: SocketAddr,
}

async fn listen() -> Listening {
    jackioh_cards::register_all();
    let app = test_app().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a free port");
    let addr = listener.local_addr().expect("the listener's address");
    let service = router(Arc::clone(&app)).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move {
        axum::serve(listener, service)
            .await
            .expect("the test server runs");
    });
    Listening { app, addr }
}

/// The E2E fixture account's profile id (R144: reseeded at boot under `E2E=1`).
async fn profile_id_of(app: &App, user_id: &str) -> String {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let profile = tx
        .profiles_get_by_user_id(user_id)
        .await
        .expect("profiles.getByUserId")
        .expect("the E2E fixture's profile");
    tx.commit().await.expect("commit");
    profile.id
}

async fn set_in_match(app: &App, profile_id: &str, match_id: Option<&str>) {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    tx.profiles_set_in_match(profile_id, match_id)
        .await
        .expect("profiles.setInMatch");
    tx.commit().await.expect("commit");
}

/// A live match between the two active E2E fixtures, both pointed at it, on two real decks.
async fn live_fixture_match(app: &Arc<App>, match_id: &str) -> (String, String) {
    let first = profile_id_of(app, "e2e-p1").await;
    let second = profile_id_of(app, "e2e-p2").await;
    let pool = deckable_pool();
    let input: StartMatchInput = serde_json::from_value(json!({
        "matchId": match_id,
        "seed": SEED,
        "catalogVersion": TEST_CATALOG_VERSION,
        "ranked": false,
        "seats": [
            { "profileId": first, "player": "p1", "deck": pool[..20].to_vec() },
            { "profileId": second, "player": "p2", "deck": pool[20..40].to_vec() },
        ],
    }))
    .expect("a StartMatchInput");
    app.matches.start(app, input).await.expect("registry.start");
    set_in_match(app, &first, Some(match_id)).await;
    set_in_match(app, &second, Some(match_id)).await;
    (first, second)
}

/// A close code as the tests compare it, whatever integer type `WS_CLOSE`'s fields have.
fn code(value: impl Into<i64>) -> i64 {
    value.into()
}

// ---------------------------------------------------------------------------
// R82, R345: the automatic turn end
// ---------------------------------------------------------------------------

mod the_automatic_turn_end_is_each_players_to_turn_off_r82_r345 {
    use super::*;

    /// Two decks the real engine accepts, p1's of cards that each cost two or more and none of which
    /// casts on draw. Turn 1 has one mana (§2.3), so once both keep their hands p1 has nothing to play
    /// on it and R82 would end the turn by itself, which is what these tests need to see the preference
    /// change. (#21 Hinder's cast used to do it, by lowering p1's first refresh to 0, but setup no
    /// longer deals a cast-on-draw card, R635.) Setup asks nothing, so the tests start, as their names
    /// say, in the mulligan window (R265).
    async fn real_match() -> Harness {
        let pool = deckable_pool();
        let (first, _) = decks_the_engine_accepts(&pool, SEED);
        let dear: Vec<String> = pool
            .iter()
            .filter(|card_id| {
                jackioh_cards::CATALOG.get(*card_id).is_some_and(|def| {
                    matches!(def.cost, CardCost::Fixed(cost) if cost >= 2)
                        && !def.tags.iter().any(|tag| tag.as_str() == "Quickdraw")
                        && !format!("{} {}", def.base.text, def.radiant.text)
                            .to_lowercase()
                            .contains("cast on draw")
                })
            })
            .cloned()
            .collect();
        let p1_deck: Vec<String> = dear.iter().take(first.len()).cloned().collect();
        let p2_deck: Vec<String> = pool
            .iter()
            .filter(|card_id| !p1_deck.contains(*card_id))
            .take(first.len())
            .cloned()
            .collect();
        assert_eq!(p1_deck.len(), first.len(), "enough cards that cost two or more");
        let h = harness(real((p1_deck, p2_deck))).await;
        // PREMISE: the deal asks nothing (R635), so both mulligans are open.
        let at = h.actor.snapshot();
        assert_eq!(
            (at.phase, at.pending_for, at.mulligan_owed.clone()),
            (Phase::Mulligan, None, vec![P1, P2])
        );
        h
    }

    async fn keep_hands(h: &Harness) {
        for player in PLAYER_IDS {
            let socket = h.socket(player);
            let keep = hand_ids(&last_view(socket));
            send(
                h,
                socket,
                &format!("mull-{player}"),
                json!({ "type": "mulligan", "keep": keep }),
            )
            .await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r345_a_seat_that_turned_it_off_during_the_mulligan_keeps_a_turn_it_has_nothing_to_do_on_and_only_its_own_view_says_so()
     {
        // With these decks p1 has nothing to do on turn 1 (`real_match`), so R82 would end that turn by
        // itself.
        let h = real_match().await;
        send(
            &h,
            &h.p1,
            "auto-off",
            json!({ "type": "setAutoEndTurn", "enabled": false }),
        )
        .await;
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        keep_hands(&h).await;

        let at = h.actor.snapshot();
        assert_eq!(
            (at.phase, at.active, at.pending_for, at.result),
            (Phase::Main, P1, None, None)
        );
        // PREMISE: nothing to do but what R82 discounts, so it is the preference alone that keeps the turn.
        assert!(legal_types(&h.p1).contains(&"endTurn".to_string()));
        let others: Vec<String> = legal_types(&h.p1)
            .into_iter()
            .filter(|kind| !["endTurn", "concede", "offerDraw"].contains(&kind.as_str()))
            .collect();
        assert_eq!(others, Vec::<String>::new());
        assert_eq!(last_view(&h.p1)["autoEndTurn"], json!(false));
        assert!(last_view(&h.p2).get("autoEndTurn").is_none());

        send(&h, &h.p1, "end", json!({ "type": "endTurn" })).await;
        assert_eq!(h.actor.snapshot().active, P2);
    }

    #[tokio::test(start_paused = true)]
    async fn r345_without_it_the_same_turn_1_ends_by_itself_and_a_malformed_preference_is_answered_not_applied()
     {
        let h = real_match().await;
        send(
            &h,
            &h.p1,
            "bad",
            json!({ "type": "setAutoEndTurn", "enabled": "no" }),
        )
        .await;
        assert_eq!(
            errors(&h.p1).last().map(|error| error["code"].clone()),
            Some(json!("malformed"))
        );
        keep_hands(&h).await;
        let at = h.actor.snapshot();
        assert_eq!((at.phase, at.active), (Phase::Main, P2));
        assert!(last_view(&h.p1).get("autoEndTurn").is_none());
    }
}

// ---------------------------------------------------------------------------
// The `ws` adapter (smoke only: every protocol guarantee above is proven on the same `Socket`)
// ---------------------------------------------------------------------------

mod the_ws_adapter {
    use super::*;

    #[tokio::test]
    async fn turns_a_ws_connection_into_the_actors_socket_text_frames_only() {
        let server = listen().await;
        live_fixture_match(&server.app, MATCH_ID).await;
        let mut ws = WsClient::connect(
            server.addr,
            &format!("{WS_PATH}?token=e2e-token-p1&matchId={MATCH_ID}"),
        )
        .await
        .expect("the upgrade is accepted");

        // The socket is open, and what the actor sends reaches the client as text: the attach's view.
        let view = ws.next_of_type("view").await;
        assert_eq!(view["view"]["viewer"], json!("p1"));

        // A text frame reaches the actor: `hello` is answered with a fresh view.
        ws.send_text(r#"{"type":"hello"}"#).await;
        ws.next_of_type("view").await;

        // A binary frame does not: it is answered, and nothing is delivered.
        ws.send_binary(&[1, 2, 3]).await;
        let refused = ws.next_of_type("error").await;
        assert!(refused.to_string().contains("text frames only"));

        // The close reaches the actor as the socket going: the seat is no longer present (R672).
        ws.close(1000).await;
        let mut present = true;
        for _ in 0..100 {
            present = server
                .app
                .matches
                .presence_of(MATCH_ID)
                .is_some_and(|presence| presence.p1);
            if !present {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(!present, "the actor still holds the closed socket");
    }

    #[tokio::test]
    async fn r148_refuses_a_socket_that_is_not_in_the_match_it_asked_for_mirroring_the_http_status() {
        let server = listen().await;
        let profile = profile_id_of(&server.app, "e2e-p1").await;
        set_in_match(&server.app, &profile, Some("other")).await;

        let mut refused = WsClient::connect(
            server.addr,
            &format!("{WS_PATH}?token=e2e-token-p1&matchId={MATCH_ID}"),
        )
        .await
        .expect("the upgrade happens before the checks");
        let (refused_texts, refused_code) = refused.until_close().await;
        assert_eq!(refused_code.map(i64::from), Some(code(WS_CLOSE.forbidden)));
        assert!(!server.app.matches.has(MATCH_ID));

        let mut anonymous = WsClient::connect(server.addr, WS_PATH)
            .await
            .expect("the upgrade happens");
        let (anonymous_texts, anonymous_code) = anonymous.until_close().await;
        assert_eq!(anonymous_code.map(i64::from), Some(code(WS_CLOSE.unauthorized)));

        // R148: 4401 and 4403 are the private-use mirrors of 401 and 403, and only the close code
        // varies — §9.1: a socket learns that it may not have this match, never which check said so.
        assert_eq!(code(WS_CLOSE.unauthorized), 4401);
        assert_eq!(code(WS_CLOSE.forbidden), 4403);
        for texts in [&refused_texts, &anonymous_texts] {
            assert!(
                texts
                    .last()
                    .is_some_and(|text| text.contains("\"code\":\"forbidden\""))
            );
        }

        // In the match it asks for, the same account is attached to its seat (TS: the registry's
        // `attach` was called with `match-1` and the profile): its seat's view arrives.
        let (first, _) = live_fixture_match(&server.app, MATCH_ID).await;
        assert_eq!(first, profile);
        let mut accepted = WsClient::connect(
            server.addr,
            &format!("{WS_PATH}?matchId={MATCH_ID}&token=e2e-token-p1"),
        )
        .await
        .expect("the upgrade is accepted");
        let view = accepted.next_of_type("view").await;
        assert_eq!(view["view"]["viewer"], json!("p1"));
        assert!(
            server
                .app
                .matches
                .presence_of(MATCH_ID)
                .is_some_and(|presence| presence.p1)
        );
    }

    #[tokio::test]
    async fn gives_a_pending_account_no_socket_9_4() {
        let server = listen().await;
        let profile = profile_id_of(&server.app, "e2e-pending").await;
        set_in_match(&server.app, &profile, Some(MATCH_ID)).await;

        let mut ws = WsClient::connect(
            server.addr,
            &format!("{WS_PATH}?token=e2e-token-pending&matchId={MATCH_ID}"),
        )
        .await
        .expect("the upgrade happens before the checks");
        let (_, close) = ws.until_close().await;
        assert_eq!(close.map(i64::from), Some(code(WS_CLOSE.forbidden)));
        // Refused before the registry was asked: no actor was made for the match.
        assert!(!server.app.matches.has(MATCH_ID));
    }
}

mod r1044_face_down_play_over_the_socket {
    use super::*;
    use jackioh_server::actor::protocol::{ActionMessage, ClientMessage, parse_client_message};

    #[test]
    fn r1044_parses_a_face_down_play_and_refuses_a_bad_timing() {
        let parsed = parse_client_message(
            &json!({
                "type": "action",
                "action": {
                    "type": "play",
                    "instanceId": "c7",
                    "zone": { "row": "backrow", "lane": 2 },
                    "faceDown": "startOfNextTurn",
                    "nonce": "set-1",
                },
            })
            .to_string(),
        );
        assert_eq!(
            parsed,
            Ok(ClientMessage::Action(ActionMessage {
                nonce: "set-1".to_string(),
                body: serde_json::from_value(json!({
                    "type": "play",
                    "instanceId": "c7",
                    "zone": { "row": "backrow", "lane": 2 },
                    "faceDown": "startOfNextTurn",
                }))
                .expect("a play body"),
            }))
        );
        // Anything but a reveal timing is malformed, never passed on.
        for bad in [json!("tomorrow"), json!(3), json!(true)] {
            let parsed = parse_client_message(
                &json!({ "type": "action", "action": {
                    "type": "play",
                    "instanceId": "c7",
                    "faceDown": bad,
                    "nonce": "bad",
                } })
                .to_string(),
            );
            assert!(parsed.is_err(), "{bad} parsed as {parsed:?}");
        }
    }
}
