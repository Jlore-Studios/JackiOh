//! The Conquest series through the server (`src/api/series.rs`, `src/api/results.rs`, SPEC §9.5,
//! R330–R336, R259–R262). `series_rules.rs` holds the exhaustive rule tables; this file shows each ruling
//! holding end to end: a series made the way pairing makes it (`start_series`), picks and forfeits
//! through the router, games started through the match directory, and games finished through the
//! same `record_result` the actor calls.
//!
//! The match directory here writes the match row (`createFakeMatchDirectory(store)` in TS), as the real
//! registry does, so "the game started" is a row in the store and a result can finish it. Time is the
//! manual clock, so the pick clock (R333) is driven by `timers.advance` and the sweeper.
//!
//! Port of `apps/server/test/api/series.test.ts`. What changed with the port:
//!
//! - The match directory is the real registry (`app.matches`), which writes the row and runs the
//!   game's actor on the real engine. So the trios' decks are real: six disjoint slices of the
//!   catalog's playable cards (`cards`), where TS made up two ids per deck. TS's record of the starts
//!   (`deps.matches.started`) is read back off the rows the registry wrote (`started`).
//! - The router is the whole app's (`support::deps::call`), and the three players sign in as the E2E
//!   fixture accounts (`e2e-token-p1`, `e2e-token-p2`, `e2e-token-pending`, SURFACE §11.2's
//!   `test_app()`), whose profiles this file seeds after emptying the fixtures.
//! - The manual clock is tokio's paused clock (`start_paused`, `tokio::time::advance`), which the
//!   server's clock follows (SURFACE §11.3: `Timers` → `tokio::time`). The sweeper is
//!   `run_sweeper` spawned as a task, and stopping it is aborting the task.
//! - TS wrapped `matches.discardOpen` to record its argument, and set `onCall` to fail a method; the
//!   fake's one seam is `on_call`, which sees the method's name, so the harness counts the calls
//!   (the only id a series releases is its reserved game 1 id) and forwards to a test's fault.
//! - Rows and views are compared as their JSON (TS's keys).

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::Duration;

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::{GameOverReason, Tag, Winner};
use jackioh_server::actor::contracts::{RecordResultInput, TerminalOutcome};
use jackioh_server::api::results::record_result;
use jackioh_server::api::series::{run_sweeper, start_series, sweep_series};
use jackioh_server::api::series_rules::NewSeriesInput;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    RATING_DEVIATION_START, RATING_VOLATILITY_START, SERIES_MAX_GAMES, SERIES_PICK_SECONDS,
    SERIES_SWEEP_INTERVAL_SECONDS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchSeat, StoreError};
use jackioh_server::ranked::glicko2::{Glicko, rate_game};
use serde::Serialize;
use serde_json::{Value, json};

use crate::support::deps::{call, test_app};

const ALICE: &str = "profile-alice";
const BOB: &str = "profile-bob";
const STRANGER: &str = "profile-stranger";
const SERIES_ID: &str = "series-1";
const FIRST_MATCH: &str = "match-1";
const PICK_MS: i64 = SERIES_PICK_SECONDS * 1000;
const SWEEP_MS: i64 = SERIES_SWEEP_INTERVAL_SECONDS * 1000;

/// The E2E fixture accounts (`src/api/e2e.ts`'s `E2E_ACCOUNTS`, a contract with
/// `e2e/support/config.ts`): the user id a profile is seeded under, and the token that signs in as it.
const ALICE_ACCOUNT: (&str, &str) = ("e2e-p1", "e2e-token-p1");
const BOB_ACCOUNT: (&str, &str) = ("e2e-p2", "e2e-token-p2");
const STRANGER_ACCOUNT: (&str, &str) = ("e2e-pending", "e2e-token-pending");

// ---------------------------------------------------------------------------
// The harness's small reads (private copies, CLAUDE.md's fullsend rule 5)
// ---------------------------------------------------------------------------

/// Every store call in this file: one transaction, committed when the call answers.
macro_rules! store {
    ($app:expr, |$t:ident| $call:expr) => {{
        let mut $t = $app.db.begin(None).await.expect("a store transaction begins");
        let out = $call
            .await
            .expect(concat!("the store answers ", stringify!($call)));
        $t.commit().await.expect("the store transaction commits");
        out
    }};
}

/// The JSON a row, a view or any port value serialises to (TS's object).
fn j<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("the value serialises")
}

/// TS's `toMatchObject`: every key `expected` names holds the same value in `actual` (objects
/// partially, arrays element for element), and nothing else is compared.
fn assert_match(actual: &Value, expected: &Value) {
    fn matches(actual: &Value, expected: &Value) -> bool {
        match (actual, expected) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches(got, value))),
            (Value::Array(have), Value::Array(want)) => {
                have.len() == want.len() && have.iter().zip(want).all(|(got, value)| matches(got, value))
            }
            _ => actual == expected,
        }
    }
    assert!(matches(actual, expected), "expected {actual} to match {expected}");
}

/// TS's `array.map(f)` over a JSON array.
fn map(array: &Value, f: impl Fn(&Value) -> Value) -> Value {
    Value::Array(
        array
            .as_array()
            .map(|items| items.iter().map(&f).collect())
            .unwrap_or_default(),
    )
}

/// The fake store behind `test_app()`.
fn fake(app: &App) -> &Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => data,
        _ => panic!("support::deps::test_app() runs on the fake store"),
    }
}

/// One read of the fake's raw tables (TS's `store.tables`), as JSON.
async fn table(app: &App, read: impl Fn(&FakeData) -> Value) -> Value {
    let data = fake(app).lock().await;
    read(&data)
}

async fn profile(app: &App, id: &str) -> Value {
    j(&store!(app, |t| t.profiles_get_by_id(id)))
}

async fn match_row(app: &App, id: &str) -> Value {
    j(&store!(app, |t| t.matches_get(id)))
}

/// The catalog version the series is made at (TS's `deps.catalog.version`).
fn catalog_version() -> &'static str {
    jackioh_cards::catalog_version()
}

/// The catalog's playable ids (no token), sorted: what the real engine accepts in a deck.
fn pool() -> &'static [String] {
    static POOL: OnceLock<Vec<String>> = OnceLock::new();
    POOL.get_or_init(|| {
        jackioh_cards::register_all();
        let mut ids: Vec<String> = jackioh_cards::CATALOG
            .iter()
            .filter(|(_, def)| !def.token && !def.tags.contains(&Tag::Token))
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        ids
    })
}

/// The cards of `owner`'s deck in trio slot `slot`: TS's `${owner}-card-${slot}a` and `…b`, now a
/// legal deck of real cards, and no card in two of the six decks, so a deck is told by its cards and
/// alice's cards are as easy to look for in bob's view as her name.
fn cards(owner: &str, slot: usize) -> Vec<String> {
    let first = match owner {
        "alice" => 0,
        "bob" => 3,
        other => panic!("no trio is built for {other}"),
    };
    let size = DECK_SIZE as usize;
    let at = (first + slot) * size;
    pool()[at..at + size].to_vec()
}

/// R336: nothing of alice's trio — its name, its deck names, its cards — is in `view`.
fn assert_hides_alice(view: &Value) {
    let text = view.to_string();
    assert!(!text.contains("alice"), "{text}");
    for slot in 0..3 {
        for card in cards("alice", slot) {
            assert!(
                !text.contains(&format!("\"{card}\"")),
                "alice's {card} is in {text}"
            );
        }
    }
}

/// TS's recording logger: every `tracing` line this thread writes (SURFACE §11.3: `Logger` →
/// `tracing` JSON lines with the same `event` names), kept for the test to read.
#[derive(Clone, Default)]
struct Logs(Arc<StdMutex<Vec<u8>>>);

impl std::io::Write for Logs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("the log buffer").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Logs {
    fn capture() -> (Logs, tracing::subscriber::DefaultGuard) {
        let logs = Logs::default();
        let writer = logs.clone();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_max_level(tracing::Level::TRACE)
            .with_writer(move || writer.clone())
            .finish();
        (logs, crate::support::deps::set_log_default(subscriber))
    }

    /// Whether any line names `event` (TS: `log.entries.some((entry) => entry.event === event)`).
    fn has(&self, event: &str) -> bool {
        let bytes = self.0.lock().expect("the log buffer").clone();
        let quoted = format!("\"{event}\"");
        String::from_utf8_lossy(&bytes)
            .lines()
            .any(|line| line.contains(&quoted))
    }
}

/// A fault a test injects (TS: a throwing `store.onCall`).
type Fault = Box<dyn Fn(&str) -> Result<(), StoreError> + Send>;

// ---------------------------------------------------------------------------
// The file's own helpers, in TS order
// ---------------------------------------------------------------------------

/// The tokens the three players sign in with.
struct Tokens {
    alice: &'static str,
    bob: &'static str,
    stranger: &'static str,
}

struct Harness {
    /// TS's `deps` and `router` both: the app holds the store and the registry, and serves the routes.
    app: Arc<App>,
    tokens: Tokens,
    /// How many times `matches.discardOpen` was asked (TS kept the ids it was asked to release).
    discarded: Arc<StdMutex<usize>>,
    fault: Arc<StdMutex<Option<Fault>>>,
    logs: Logs,
    _logging: tracing::subscriber::DefaultGuard,
}

impl Harness {
    /// TS's `h.deps.store.onCall = (method) => { … throw … }`.
    fn fail_on(&self, fault: impl Fn(&str) -> Result<(), StoreError> + Send + 'static) {
        *self.fault.lock().expect("the fault slot") = Some(Box::new(fault));
    }

    /// TS's `h.deps.store.onCall = () => undefined`.
    fn heal(&self) {
        *self.fault.lock().expect("the fault slot") = None;
    }

    /// TS's `h.discarded`, counted.
    fn discarded(&self) -> usize {
        *self.discarded.lock().expect("the discard count")
    }
}

fn trio(owner: &str) -> Value {
    let deck = |slot: usize| json!({ "name": format!("{owner} deck {slot}"), "cards": cards(owner, slot) });
    json!({ "name": format!("{owner}'s trio"), "decks": [deck(0), deck(1), deck(2)] })
}

/// R603: the move one rated game makes between two players new to rating (a series is one, R262).
/// TS's `move`.
fn rating_move(rating_a: f64, rating_b: f64, score_a: f64) -> (f64, f64) {
    let fresh = |rating: f64| Glicko {
        rating,
        deviation: RATING_DEVIATION_START,
        volatility: RATING_VOLATILITY_START,
    };
    let next = rate_game(&fresh(rating_a), &fresh(rating_b), score_a);
    (next.a.rating, next.b.rating)
}

/// An active profile signed in as one of the E2E fixture accounts; answers its token.
async fn active_profile(
    app: &App,
    id: &str,
    account: (&'static str, &'static str),
    rating: f64,
) -> &'static str {
    let (user_id, token) = account;
    let _ = fake(app).lock().await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "email": format!("{id}@example.test"),
        "status": "active",
        "rating": rating,
    }));
    token
}

async fn harness(ratings: (f64, f64)) -> Harness {
    let (logs, logging) = Logs::capture();
    let app = test_app().await;
    // `test_app()` holds the E2E fixtures (R144); the series' players replace them.
    fake(&app).lock().await.reset();
    // `matches.discardOpen` is a no-op in memory (there are no `open` rows), so record what it is asked.
    let discarded = Arc::new(StdMutex::new(0));
    let fault: Arc<StdMutex<Option<Fault>>> = Arc::new(StdMutex::new(None));
    {
        let discarded = discarded.clone();
        let fault = fault.clone();
        fake(&app).lock().await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
            if method == "matches.discardOpen" {
                *discarded.lock().expect("the discard count") += 1;
            }
            match fault.lock().expect("the fault slot").as_ref() {
                Some(fault) => fault(method),
                None => Ok(()),
            }
        }));
    }
    let tokens = Tokens {
        alice: active_profile(&app, ALICE, ALICE_ACCOUNT, ratings.0).await,
        bob: active_profile(&app, BOB, BOB_ACCOUNT, ratings.1).await,
        stranger: active_profile(&app, STRANGER, STRANGER_ACCOUNT, 1000.0).await,
    };
    let input: NewSeriesInput = serde_json::from_value(json!({
        "seriesId": SERIES_ID,
        "firstMatchId": FIRST_MATCH,
        "sides": [
            { "profileId": ALICE, "trio": trio("alice") },
            { "profileId": BOB, "trio": trio("bob") },
        ],
        "seedBase": "seed-base",
        "catalogVersion": catalog_version(),
        "ranked": true,
    }))
    .expect("a NewSeriesInput");
    let mut tx = app.db.begin(None).await.expect("store.tx");
    start_series(&app, input, &mut tx)
        .await
        .expect("the series starts");
    tx.commit().await.expect("the series commits");
    Harness {
        app,
        tokens,
        discarded,
        fault,
        logs,
        _logging: logging,
    }
}

/// TS's `harness()`: both players at 1000.
async fn harness_at_1000() -> Harness {
    harness((1000.0, 1000.0)).await
}

/// One request through the app's router: its status and its JSON body.
async fn request(h: &Harness, method: &str, path: &str, token: &str, body: Option<Value>) -> (u16, Value) {
    let (status, _headers, body) = call(&h.app, method, path, Some(token), body.unwrap_or(Value::Null)).await;
    (status, body)
}

async fn get_series(h: &Harness, token: &str) -> (u16, Value) {
    get_series_at(h, token, SERIES_ID).await
}

/// TS's `getSeries(h, token, id)` with an id other than the series'.
async fn get_series_at(h: &Harness, token: &str, id: &str) -> (u16, Value) {
    request(h, "GET", &format!("/api/series/{id}"), token, None).await
}

async fn pick(h: &Harness, token: &str, slot: Value) -> (u16, Value) {
    request(
        h,
        "POST",
        &format!("/api/series/{SERIES_ID}/pick"),
        token,
        Some(json!({ "slot": slot })),
    )
    .await
}

async fn forfeit(h: &Harness, token: &str) -> (u16, Value) {
    request(
        h,
        "POST",
        &format!("/api/series/{SERIES_ID}/forfeit"),
        token,
        None,
    )
    .await
}

async fn pick_both(h: &Harness, slots: [i64; 2]) -> Value {
    assert_eq!(pick(h, h.tokens.alice, json!(slots[0])).await.0, 200);
    let (status, answer) = pick(h, h.tokens.bob, json!(slots[1])).await;
    assert_eq!(status, 200);
    answer
}

/// The series row, as its JSON.
async fn row(h: &Harness) -> Value {
    let series = store!(h.app, |t| t.series_get(SERIES_ID));
    if series.is_none() {
        panic!("the series is gone");
    }
    j(&series)
}

/// TS's `h.deps.matches.started`: every match the directory started, in order, as the start was
/// asked for (`StartMatchInput`). The registry writes each one's row as it starts it, so the rows say it.
async fn started(h: &Harness) -> Vec<Value> {
    let rows = table(&h.app, |data| j(&data.tables.matches)).await;
    rows.as_array()
        .map(|rows| rows.iter().map(start_input_of).collect())
        .unwrap_or_default()
}

/// A match row read back as the `StartMatchInput` that made it: index 0 of `players` and `decks` is p1.
fn start_input_of(row: &Value) -> Value {
    json!({
        "matchId": row["id"],
        "seed": row["seed"],
        "catalogVersion": row["catalogVersion"],
        "ranked": row["ranked"].as_bool().unwrap_or(false),
        "seats": [
            { "profileId": row["players"][0], "player": "p1", "deck": row["decks"][0] },
            { "profileId": row["players"][1], "player": "p2", "deck": row["decks"][1] },
        ],
    })
}

async fn seats_of(h: &Harness, match_id: &str) -> (MatchSeat, MatchSeat) {
    let Some(started) = started(h)
        .await
        .into_iter()
        .find(|input| input["matchId"] == match_id)
    else {
        panic!("{match_id} was never started");
    };
    let seat = |at: usize| -> MatchSeat {
        serde_json::from_value(started["seats"][at].clone()).expect("a MatchSeat")
    };
    (seat(0), seat(1))
}

/// Ends the game in play the way the actor does: `record_result`, with the match's own seats.
/// `reason` defaults as TS's did: `draw-accepted` for a draw, `hero-death` otherwise.
async fn finish_game(h: &Harness, winner: &str, reason: Option<GameOverReason>) {
    let reason = reason.unwrap_or(if winner == "draw" {
        GameOverReason::DrawAccepted
    } else {
        GameOverReason::HeroDeath
    });
    let series = row(h).await;
    assert_eq!(series["status"], "playing");
    let match_id = series["nextMatchId"].as_str().unwrap_or_default().to_owned();
    let seats = seats_of(h, &match_id).await;
    let winner = if winner == "draw" {
        Winner::Draw
    } else {
        let player = [j(&seats.0), j(&seats.1)]
            .into_iter()
            .find(|seat| seat["profileId"] == winner)
            .map(|seat| seat["player"].clone())
            .unwrap_or(json!("p1"));
        serde_json::from_value(player).expect("a seat")
    };
    record_result(
        &h.app,
        RecordResultInput {
            match_id,
            seats,
            outcome: TerminalOutcome { winner, reason },
            turns: 9,
            at: now_ms(),
            last_boards: None,
            final_hash: None,
        },
    )
    .await
    .expect("the result is written");
}

async fn ratings(h: &Harness) -> Value {
    json!([
        profile(&h.app, ALICE).await["rating"],
        profile(&h.app, BOB).await["rating"]
    ])
}

async fn in_match(h: &Harness) -> Value {
    json!([
        profile(&h.app, ALICE).await["inMatchId"],
        profile(&h.app, BOB).await["inMatchId"]
    ])
}

/// Lets the sweeper's task run to its next wait: the fake store never waits on anything else.
async fn flush() {
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
}

/// TS's `h.deps.timers.advance(ms)`: tokio's paused clock, which the server's clock follows.
async fn advance(ms: i64) {
    tokio::time::advance(Duration::from_millis(u64::try_from(ms).unwrap_or(0))).await;
}

/// Advances the manual clock one sweep interval at a time, letting each sweep finish.
async fn advance_sweeping(ms: i64) {
    let mut elapsed = 0;
    while elapsed < ms {
        advance(SWEEP_MS.min(ms - elapsed)).await;
        flush().await;
        elapsed += SWEEP_MS;
    }
}

async fn sweep(h: &Harness) -> Value {
    let swept = sweep_series(&h.app).await.expect("the sweep runs");
    json!({ "timedOut": swept.timed_out, "started": swept.started, "abandoned": swept.abandoned })
}

mod r259_the_series_through_the_api {
    use super::*;

    #[tokio::test]
    async fn r259_get_api_series_id_answers_each_players_own_view_and_404_to_a_stranger_or_an_unknown_id() {
        let h = harness_at_1000().await;

        let (_, alice) = get_series(&h, h.tokens.alice).await;
        assert_match(
            &alice,
            &json!({ "id": SERIES_ID, "status": "picking", "gameNo": 1, "currentMatchId": null }),
        );
        assert_match(
            &alice["you"],
            &json!({ "seat": "p1", "trioName": "alice's trio", "pick": null }),
        );
        let (_, bob) = get_series(&h, h.tokens.bob).await;
        assert_eq!(bob["you"]["seat"], "p2");
        assert_hides_alice(&bob);

        let (status, stranger) = get_series(&h, h.tokens.stranger).await;
        assert_eq!(status, 404);
        assert_eq!(stranger["error"]["code"], "not_found");
        let (status, unknown) = get_series_at(&h, h.tokens.alice, "no-such-series").await;
        assert_eq!(status, 404);
        // One answer for both, so an id tells a stranger nothing.
        assert_eq!(unknown, get_series(&h, h.tokens.stranger).await.1);

        // A stranger can neither pick nor forfeit in it.
        assert_eq!(pick(&h, h.tokens.stranger, json!(0)).await.0, 404);
        assert_eq!(forfeit(&h, h.tokens.stranger).await.0, 404);
    }

    #[tokio::test]
    async fn r259_a_pick_stays_hidden_until_both_are_in_then_game_1_starts_with_the_right_seats_decks_and_seed()
     {
        let h = harness_at_1000().await;

        let (_, after_alice) = pick(&h, h.tokens.alice, json!(1)).await;
        assert_match(
            &after_alice,
            &json!({ "status": "picking", "you": { "pick": 1 } }),
        );
        assert!(started(&h).await.is_empty());

        let (_, bob_sees) = get_series(&h, h.tokens.bob).await;
        assert_eq!(
            bob_sees["opponent"],
            json!({
                "wins": 0,
                "decks": [
                    { "slot": 0, "won": false },
                    { "slot": 1, "won": false },
                    { "slot": 2, "won": false },
                ],
                "picked": true,
            }),
        );

        let (_, after_bob) = pick(&h, h.tokens.bob, json!(2)).await;
        assert_match(
            &after_bob,
            &json!({ "status": "playing", "currentMatchId": FIRST_MATCH, "gameNo": 1 }),
        );
        assert_eq!(
            started(&h).await,
            vec![json!({
                "matchId": FIRST_MATCH,
                "seed": "seed-base:1",
                "catalogVersion": catalog_version(),
                "ranked": true,
                "seats": [
                    { "profileId": ALICE, "player": "p1", "deck": cards("alice", 1) },
                    { "profileId": BOB, "player": "p2", "deck": cards("bob", 2) },
                ],
            })],
        );
        assert_eq!(in_match(&h).await, json!([FIRST_MATCH, FIRST_MATCH]));
        assert_eq!(match_row(&h.app, FIRST_MATCH).await["status"], "live");
    }

    #[tokio::test]
    async fn r330_a_pick_is_refused_400_for_a_slot_out_of_range_or_a_deck_that_has_won_409_while_a_game_is_played()
     {
        let h = harness_at_1000().await;

        for slot in [json!("0"), json!(3), json!(-1), json!(0.5), Value::Null] {
            let (status, refused) = pick(&h, h.tokens.alice, slot).await;
            assert_eq!(status, 400);
            assert_eq!(refused["error"]["code"], "bad_request");
        }

        pick_both(&h, [0, 0]).await;
        let (status, during) = pick(&h, h.tokens.alice, json!(1)).await;
        assert_eq!(status, 409);
        assert_eq!(during["error"]["code"], "conflict");

        finish_game(&h, ALICE, None).await;
        let (status, locked) = pick(&h, h.tokens.alice, json!(0)).await;
        assert_eq!(status, 400);
        // TS: `toMatch(/already won .* locked/u)`.
        let message = locked["error"]["message"].as_str().unwrap_or_default();
        let won = message.find("already won ");
        assert!(
            won.is_some_and(|at| message[at..].contains(" locked")),
            "{message}"
        );
        // Bob lost with deck 0, and may play it again.
        assert_eq!(pick(&h, h.tokens.bob, json!(0)).await.0, 200);
        assert_eq!(pick(&h, h.tokens.alice, json!(1)).await.0, 200);
    }

    #[tokio::test]
    async fn r331_a_pick_is_sealed_another_slot_is_refused_with_409_and_the_same_slot_again_answers_as_the_success_it_was()
     {
        let h = harness_at_1000().await;
        assert_eq!(pick(&h, h.tokens.alice, json!(1)).await.0, 200);

        let (status, other) = pick(&h, h.tokens.alice, json!(2)).await;
        assert_eq!(status, 409);
        assert!(
            other["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("final"),
            "{other}"
        );
        assert_eq!(row(&h).await["sides"][0]["pick"], json!(1));

        // A retry of the same pick, its first answer lost: 200, and nothing is written twice.
        let version = row(&h).await["version"].clone();
        let (status, retry) = pick(&h, h.tokens.alice, json!(1)).await;
        assert_eq!(status, 200);
        assert_eq!(retry["you"]["pick"], json!(1));
        assert_eq!(row(&h).await["version"], version);

        // The same after the pick that completed both began the game.
        pick(&h, h.tokens.bob, json!(0)).await;
        let (status, late) = pick(&h, h.tokens.bob, json!(0)).await;
        assert_eq!(status, 200);
        assert_match(
            &late,
            &json!({ "status": "playing", "currentMatchId": FIRST_MATCH }),
        );
        assert_eq!(pick(&h, h.tokens.bob, json!(2)).await.0, 409);
    }

    #[tokio::test]
    async fn r331_a_late_duplicate_of_game_1s_pick_answers_as_game_1s_and_never_becomes_game_2s() {
        let h = harness_at_1000().await;
        let path = format!("/api/series/{SERIES_ID}/pick");
        let pick_for = |token: &'static str, slot: i64, game_no: i64| {
            let path = path.clone();
            let h = &h;
            async move {
                request(
                    h,
                    "POST",
                    &path,
                    token,
                    Some(json!({ "slot": slot, "gameNo": game_no })),
                )
                .await
            }
        };
        assert_eq!(pick_for(h.tokens.alice, 0, 1).await.0, 200);
        assert_eq!(pick_for(h.tokens.bob, 1, 1).await.0, 200);
        finish_game(&h, BOB, None).await;
        assert_eq!(row(&h).await["status"], "picking");

        let (status, _) = pick_for(h.tokens.alice, 0, 1).await;
        assert_eq!(status, 200);
        assert!(
            row(&h).await["sides"][0]["pick"].is_null(),
            "game 2's pick is still Alice's to make"
        );
        assert_eq!(pick_for(h.tokens.alice, 2, 1).await.0, 409);
        assert_eq!(pick_for(h.tokens.alice, 2, 2).await.0, 200);
        for game_no in [json!(0), json!("2"), json!(1.5)] {
            let (status, _) = request(
                &h,
                "POST",
                &path,
                h.tokens.bob,
                Some(json!({ "slot": 1, "gameNo": game_no })),
            )
            .await;
            assert_eq!(status, 400);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r331_the_same_pick_sent_again_after_the_deadline_answers_as_the_success_it_was() {
        let h = harness_at_1000().await;
        assert_eq!(pick(&h, h.tokens.alice, json!(1)).await.0, 200);
        advance(PICK_MS).await;
        assert_eq!(pick(&h, h.tokens.alice, json!(1)).await.0, 200);
        assert_eq!(pick(&h, h.tokens.alice, json!(2)).await.0, 409);
    }

    #[tokio::test]
    async fn r331_a_pick_is_in_the_database_before_it_is_acknowledged_and_the_opponent_learns_only_that_it_is_in()
     {
        let h = harness_at_1000().await;
        // The store fails the write: the pick is refused, and nothing says it was made.
        h.fail_on(|method| {
            if method == "series.update" {
                Err(StoreError::Other("the database went away".to_owned()))
            } else {
                Ok(())
            }
        });
        assert_eq!(pick(&h, h.tokens.alice, json!(2)).await.0, 500);
        h.heal();
        assert!(row(&h).await["sides"][0]["pick"].is_null());
        assert_eq!(
            get_series(&h, h.tokens.bob).await.1["opponent"]["picked"],
            json!(false)
        );

        assert_eq!(pick(&h, h.tokens.alice, json!(2)).await.0, 200);
        assert_eq!(row(&h).await["sides"][0]["pick"], json!(2));
        let (_, bob) = get_series(&h, h.tokens.bob).await;
        assert!(bob.to_string().contains("\"picked\":true"), "{bob}");
        assert_hides_alice(&bob);
    }

    #[tokio::test]
    async fn r335_game_2_puts_series_p2_first_with_its_own_seed_and_the_decks_picked_for_it() {
        let h = harness_at_1000().await;
        pick_both(&h, [0, 1]).await;
        finish_game(&h, ALICE, None).await;

        let game2 = pick_both(&h, [2, 0]).await;
        let game2_id = game2["currentMatchId"].as_str().unwrap_or_default().to_owned();
        assert_ne!(game2_id, FIRST_MATCH);
        // R376: a game of the series is filed as Conquest's, whatever made the series.
        assert_eq!(j(&store!(h.app, |t| t.matches_mode_of(&game2_id))), json!("bo3"));
        assert_eq!(
            j(&store!(h.app, |t| t.matches_mode_of(FIRST_MATCH))),
            json!("bo3")
        );
        assert_eq!(
            started(&h).await.get(1).cloned().unwrap_or(Value::Null),
            json!({
                "matchId": game2_id,
                "seed": "seed-base:2",
                "catalogVersion": catalog_version(),
                "ranked": true,
                "seats": [
                    { "profileId": BOB, "player": "p1", "deck": cards("bob", 0) },
                    { "profileId": ALICE, "player": "p2", "deck": cards("alice", 2) },
                ],
            }),
        );
        finish_game(&h, BOB, None).await;
        let (_, view) = get_series(&h, h.tokens.bob).await;
        assert_match(
            &view,
            &json!({ "status": "picking", "gameNo": 3, "currentMatchId": null }),
        );
        assert_eq!(
            map(&view["games"], |game| json!([
                game["yourSlot"],
                game["opponentSlot"],
                game["youWentFirst"],
                game["result"]
            ])),
            json!([[1, 0, false, "loss"], [0, 2, true, "win"]]),
        );
    }

    #[tokio::test]
    async fn r332_a_players_last_deck_is_picked_for_them_and_when_both_are_down_to_one_the_game_starts_by_itself()
     {
        let h = harness_at_1000().await;
        pick_both(&h, [0, 0]).await;
        finish_game(&h, ALICE, None).await;
        pick_both(&h, [1, 0]).await;
        finish_game(&h, ALICE, None).await;

        // Alice has won with decks 0 and 1: deck 2 is hers, picked for her; Bob picks.
        let (_, alice) = get_series(&h, h.tokens.alice).await;
        assert_match(&alice["you"], &json!({ "pick": 2, "autoPick": true }));
        assert_eq!(
            get_series(&h, h.tokens.bob).await.1["opponent"]["picked"],
            json!(true)
        );
        assert_eq!(pick(&h, h.tokens.bob, json!(0)).await.0, 200);
        finish_game(&h, BOB, None).await;
        pick_both(&h, [2, 1]).await;
        finish_game(&h, BOB, None).await;

        // Two wins each: game 5 needs no pick and has started.
        let series = row(&h).await;
        assert_eq!(map(&series["sides"], |side| side["wins"].clone()), json!([2, 2]));
        assert_eq!(series["status"], "playing");
        assert_match(
            &series["games"][4],
            &json!({ "gameNo": 5, "slots": [2, 2], "first": "p1" }),
        );
        assert_eq!(
            started(&h).await.get(4).cloned().unwrap_or(Value::Null),
            json!({
                "matchId": series["nextMatchId"],
                "seed": "seed-base:5",
                "catalogVersion": catalog_version(),
                "ranked": true,
                "seats": [
                    { "profileId": ALICE, "player": "p1", "deck": cards("alice", 2) },
                    { "profileId": BOB, "player": "p2", "deck": cards("bob", 2) },
                ],
            }),
        );
        assert_eq!(
            in_match(&h).await,
            json!([series["nextMatchId"], series["nextMatchId"]])
        );
    }

    #[tokio::test]
    async fn r259_get_api_matches_match_id_series_names_the_series_a_game_belongs_to_for_its_players_only() {
        let h = harness_at_1000().await;
        async fn read(h: &Harness, token: &str, match_id: &str) -> Value {
            let (status, answer) =
                request(h, "GET", &format!("/api/matches/{match_id}/series"), token, None).await;
            assert_eq!(status, 200);
            answer
        }

        // Not a game yet: game 1's id is only reserved while its players pick.
        assert!(read(&h, h.tokens.alice, FIRST_MATCH).await["series"].is_null());

        pick_both(&h, [0, 0]).await;
        finish_game(&h, BOB, None).await;
        let after_game1 = read(&h, h.tokens.alice, FIRST_MATCH).await;
        assert_match(
            &after_game1["series"],
            &json!({ "id": SERIES_ID, "status": "picking", "gameNo": 2 }),
        );
        assert_match(
            &after_game1["series"]["games"][0],
            &json!({ "matchId": FIRST_MATCH, "result": "loss" }),
        );

        assert!(read(&h, h.tokens.stranger, FIRST_MATCH).await["series"].is_null());
        assert!(read(&h, h.tokens.alice, "some-other-match").await["series"].is_null());
        // An id that is not even valid percent-encoding names nothing either: never a 500.
        assert!(read(&h, h.tokens.alice, "%E0%A4%A").await["series"].is_null());
        assert_eq!(get_series_at(&h, h.tokens.alice, "%ZZ").await.0, 404);
        assert_eq!(
            request(
                &h,
                "POST",
                "/api/series/%/pick",
                h.tokens.alice,
                Some(json!({ "slot": 0 }))
            )
            .await
            .0,
            404
        );
    }
}

mod r334_endings_inside_a_series {
    use super::*;

    #[tokio::test]
    async fn r334_a_concede_or_a_disconnect_loses_the_game_not_the_series_r261() {
        let h = harness_at_1000().await;
        pick_both(&h, [0, 0]).await;
        finish_game(&h, ALICE, Some(GameOverReason::Concede)).await;
        assert_eq!(row(&h).await["status"], "picking");
        pick_both(&h, [1, 1]).await;
        finish_game(&h, BOB, Some(GameOverReason::Disconnect)).await;
        let series = row(&h).await;
        assert_eq!(series["status"], "picking");
        assert_eq!(map(&series["sides"], |side| side["wins"].clone()), json!([1, 1]));
        assert_eq!(
            map(&series["games"], |game| game["reason"].clone()),
            json!(["concede", "disconnect"])
        );
    }

    #[tokio::test]
    async fn r334_a_drawn_game_counts_for_neither_side_at_the_game_cap_equal_wins_is_a_series_draw_rated_once()
     {
        let h = harness((1200.0, 1000.0)).await;
        pick_both(&h, [0, 0]).await;
        finish_game(&h, "draw", None).await;
        assert_eq!(
            map(&row(&h).await["sides"], |side| side["wins"].clone()),
            json!([0, 0])
        );
        for _game in 2..=SERIES_MAX_GAMES as i64 - 2 {
            pick_both(&h, [0, 0]).await;
            finish_game(&h, "draw", None).await;
        }
        pick_both(&h, [1, 1]).await;
        finish_game(&h, ALICE, None).await;
        pick_both(&h, [2, 1]).await;
        finish_game(&h, BOB, None).await;
        assert_eq!(
            row(&h).await["games"].as_array().map(Vec::len),
            Some(SERIES_MAX_GAMES as usize)
        );

        let series = row(&h).await;
        assert_match(
            &series,
            &json!({ "status": "over", "winner": "draw", "endReason": "exhausted" }),
        );
        let expected = rating_move(1200.0, 1000.0, 0.5);
        assert_eq!(ratings(&h).await, json!([expected.0, expected.1]));
        assert_eq!(series["ratingBefore"], json!([1200.0, 1000.0]));
        assert_eq!(series["ratingAfter"], json!([expected.0, expected.1]));
        let (_, view) = get_series(&h, h.tokens.bob).await;
        assert_eq!(
            view["result"],
            json!({ "outcome": "draw", "endReason": "exhausted", "ranked": true })
        );
        assert_eq!(in_match(&h).await, json!([null, null]));
    }

    #[tokio::test]
    async fn r334_between_games_a_player_may_forfeit_the_other_side_wins_and_the_series_is_rated_once_r261_r262()
     {
        let h = harness_at_1000().await;
        pick_both(&h, [0, 0]).await;
        finish_game(&h, BOB, None).await;
        // Bob leads 1–0 and forfeits anyway: alice takes the series.
        let (status, view) = forfeit(&h, h.tokens.bob).await;
        assert_eq!(status, 200);
        let expected = rating_move(1000.0, 1000.0, 1.0);
        assert_match(
            &view,
            &json!({ "status": "over", "currentMatchId": null, "pickDeadline": null }),
        );
        assert_eq!(
            view["result"],
            json!({ "outcome": "loss", "endReason": "forfeit", "ranked": true })
        );
        assert_eq!(ratings(&h).await, json!([expected.0, expected.1]));
        // A game was played, so no reserved id is released.
        assert_eq!(h.discarded(), 0);
        // Over is over: a second forfeit or a pick is a conflict.
        assert_eq!(forfeit(&h, h.tokens.alice).await.0, 409);
        assert_eq!(pick(&h, h.tokens.alice, json!(1)).await.0, 409);
    }

    #[tokio::test]
    async fn r334_a_forfeit_before_game_1_releases_the_match_id_the_pairing_reserved_r261_r263() {
        let h = harness_at_1000().await;
        assert_eq!(forfeit(&h, h.tokens.alice).await.0, 200);
        // TS: `discarded` is `[FIRST_MATCH]`; the one id a series with no game played releases.
        assert_eq!(h.discarded(), 1);
        assert!(started(&h).await.is_empty());
        assert_match(
            &row(&h).await,
            &json!({ "status": "over", "winner": "p2", "endReason": "forfeit", "games": [] }),
        );
    }

    #[tokio::test]
    async fn r334_a_forfeit_is_refused_with_409_while_a_game_is_being_played_concede_the_game_instead_r261() {
        let h = harness_at_1000().await;
        pick_both(&h, [0, 0]).await;
        let (status, body) = forfeit(&h, h.tokens.alice).await;
        assert_eq!(status, 409);
        assert_eq!(body["error"]["code"], "conflict");
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("concede"),
            "{body}"
        );
        assert_eq!(row(&h).await["status"], "playing");
    }
}

mod r262_how_a_series_is_rated {
    use super::*;

    #[tokio::test]
    async fn r262_a_series_decided_at_three_wins_moves_the_rating_once_from_the_ratings_before_game_1_its_games_are_unrated()
     {
        let h = harness((1200.0, 1000.0)).await;
        pick_both(&h, [0, 0]).await;
        finish_game(&h, BOB, None).await;
        // Game 1's row: recorded, not rated.
        let results = table(&h.app, |data| j(&data.tables.results)).await;
        assert_eq!(results.as_array().map(Vec::len), Some(1));
        assert_match(
            &results[0],
            &json!({
                "matchId": FIRST_MATCH,
                "winnerProfileId": BOB,
                "ratingBefore": [1200.0, 1000.0],
                "ratingAfter": [1200.0, 1000.0],
            }),
        );
        assert_eq!(ratings(&h).await, json!([1200.0, 1000.0]));
        assert_eq!(in_match(&h).await, json!([null, null]));

        pick_both(&h, [1, 1]).await;
        // Game 2: bob is the match's p1, so the row's seats are in his order.
        finish_game(&h, BOB, None).await;
        let results = table(&h.app, |data| j(&data.tables.results)).await;
        assert_eq!(results.as_array().map(Vec::len), Some(2));
        assert_match(
            &results[1],
            &json!({
                "winnerProfileId": BOB,
                "players": [BOB, ALICE],
                "ratingBefore": [1000.0, 1200.0],
                "ratingAfter": [1000.0, 1200.0],
            }),
        );
        assert_eq!(ratings(&h).await, json!([1200.0, 1000.0]));

        // Game 3: Bob's last deck is picked for him (R332); Alice's pick starts it, and Bob wins.
        assert_eq!(pick(&h, h.tokens.alice, json!(2)).await.0, 200);
        finish_game(&h, BOB, None).await;
        assert_eq!(
            table(&h.app, |data| json!(data.tables.results.len())).await,
            json!(3)
        );

        // The series: one move, scored as one match that bob won.
        let expected = rating_move(1200.0, 1000.0, 0.0);
        assert_eq!(ratings(&h).await, json!([expected.0, expected.1]));
        let series = row(&h).await;
        assert_match(
            &series,
            &json!({
                "status": "over",
                "winner": "p2",
                "endReason": "decided",
                "ratingBefore": [1200.0, 1000.0],
                "ratingAfter": [expected.0, expected.1],
            }),
        );
        assert_eq!(started(&h).await.len(), 3);

        let (_, alice) = get_series(&h, h.tokens.alice).await;
        assert_eq!(
            alice["result"],
            json!({ "outcome": "loss", "endReason": "decided", "ranked": true })
        );
        assert_eq!(alice["gameNo"], json!(3));

        // A late second report of the deciding game changes nothing.
        let deciding = series["games"][2]["matchId"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let seats = seats_of(&h, &deciding).await;
        record_result(
            &h.app,
            RecordResultInput {
                match_id: deciding,
                seats,
                outcome: TerminalOutcome {
                    winner: Winner::P2,
                    reason: GameOverReason::Concede,
                },
                turns: 1,
                at: now_ms(),
                last_boards: None,
                final_hash: None,
            },
        )
        .await
        .expect("the result is written");
        assert_eq!(ratings(&h).await, json!([expected.0, expected.1]));
        assert_eq!(row(&h).await["version"], series["version"]);
    }
}

mod r333_the_pick_clock {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r333_at_the_deadline_the_sweeper_gives_a_player_who_has_not_picked_their_first_deck_that_has_not_won_and_the_game_starts()
     {
        let h = harness_at_1000().await;
        assert_eq!(pick(&h, h.tokens.alice, json!(2)).await.0, 200);

        advance(PICK_MS - 1).await;
        assert_eq!(
            sweep(&h).await,
            json!({ "timedOut": [], "started": [], "abandoned": [] })
        );
        assert!(started(&h).await.is_empty());

        advance(1).await;
        assert_eq!(
            sweep(&h).await,
            json!({ "timedOut": [SERIES_ID], "started": [], "abandoned": [] })
        );
        assert_eq!(
            started(&h)
                .await
                .first()
                .map(|input| input["seats"].clone())
                .unwrap_or(Value::Null),
            json!([
                { "profileId": ALICE, "player": "p1", "deck": cards("alice", 2) },
                { "profileId": BOB, "player": "p2", "deck": cards("bob", 0) },
            ]),
        );
        assert_eq!(in_match(&h).await, json!([FIRST_MATCH, FIRST_MATCH]));
    }

    #[tokio::test(start_paused = true)]
    async fn r333_a_pick_that_arrives_after_the_deadline_is_refused_with_409() {
        let h = harness_at_1000().await;
        advance(PICK_MS).await;
        let (status, late) = pick(&h, h.tokens.bob, json!(0)).await;
        assert_eq!(status, 409);
        assert_eq!(late["error"]["code"], "conflict");
    }

    #[tokio::test(start_paused = true)]
    async fn r333_with_no_pick_at_all_by_the_deadline_the_series_is_abandoned_no_winner_unrated_game_1s_id_released_r260()
     {
        let h = harness((1200.0, 1000.0)).await;
        advance(PICK_MS).await;
        sweep(&h).await;

        assert_match(
            &row(&h).await,
            &json!({
                "status": "over",
                "winner": null,
                "endReason": "abandoned",
                "ratingBefore": null,
                "ratingAfter": null,
            }),
        );
        assert_eq!(ratings(&h).await, json!([1200.0, 1000.0]));
        assert_eq!(h.discarded(), 1);
        assert!(started(&h).await.is_empty());
        let (_, view) = get_series(&h, h.tokens.alice).await;
        assert_eq!(
            view["result"],
            json!({ "outcome": "abandoned", "endReason": "abandoned", "ranked": false })
        );
        // Not active any more: the sweeper leaves it alone from now on.
        assert_eq!(j(&store!(h.app, |t| t.series_active())), json!([]));
    }

    #[tokio::test(start_paused = true)]
    async fn r333_the_sweeper_runs_every_series_sweep_interval_seconds_on_its_own_and_a_failed_sweep_does_not_stop_it()
     {
        let h = harness_at_1000().await;
        let sweeper = tokio::spawn(run_sweeper(h.app.clone()));
        // The task's first wait is armed before the clock moves (TS armed it in `startSeriesSweeper`).
        flush().await;

        let failures = Arc::new(AtomicI32::new(1));
        h.fail_on(move |method| {
            if method == "series.active" && failures.load(Ordering::SeqCst) > 0 {
                failures.fetch_sub(1, Ordering::SeqCst);
                return Err(StoreError::Other("the database went away".to_owned()));
            }
            Ok(())
        });
        advance_sweeping(SWEEP_MS).await;
        assert!(h.logs.has("series.sweeper_failed"));

        assert_eq!(pick(&h, h.tokens.bob, json!(1)).await.0, 200);
        advance_sweeping(PICK_MS).await;
        // The pick clock ran out between two sweeps; the next one settled it.
        assert_eq!(row(&h).await["status"], "playing");
        assert_eq!(
            started(&h)
                .await
                .first()
                .map(|input| map(&input["seats"], |seat| seat["deck"][0].clone()))
                .unwrap_or(Value::Null),
            json!([cards("alice", 0)[0], cards("bob", 1)[0]]),
        );

        // TS: `sweeper.stop()`, after which no timer of it is pending, however far the clock moves.
        sweeper.abort();
        flush().await;
        assert!(sweeper.is_finished());
        advance(10 * SWEEP_MS).await;
        flush().await;
        assert!(sweeper.await.is_err_and(|stopped| stopped.is_cancelled()));
    }
}
