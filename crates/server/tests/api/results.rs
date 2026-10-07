//! BUILD M7-T2: "one integration test per reason".
//!
//! §2.5's seven endings each get a test — `hero-death`, `both-heroes-dead`, `concede`,
//! `draw-accepted`, `turn-cap`, `disconnect`, `match-ceiling` — and each one drives the scripted
//! engine to produce the outcome rather than hand-writing it, so the reason strings under test are
//! the ones `reduce` really emits. Then the two things §9.5 asks of the writer itself: it is
//! idempotent, and the reaper resolves anything past the ceiling.
//!
//! Port of `apps/server/test/api/results.test.ts`. What changed with the port:
//!
//! - "The scripted engine" is the real one (SURFACE §11.2: the server has no `Engine` trait):
//!   `support::engine` installs `test/fakes/engine.ts`'s scripted cards as real scripts with the
//!   testkit override, and `play` drives `jackioh_engine::reduce` from a testkit `scenario()` that
//!   puts them in p1's hand, so every outcome and turn count is still one the engine emitted.
//! - The match directory is the real registry (`app.matches`). TS's default fake directory only
//!   recorded a start; the registry writes the match row and runs an actor, so a scenario starts its
//!   match through the registry instead of writing the row first. A match backdated past its ceiling
//!   is written straight to the store with no actor, which is the crashed-actor case §9.5's reaper
//!   exists for (an actor would resolve its own ceiling first).
//! - The store is the fake behind `support::deps::test_app()`, emptied of the E2E fixtures first;
//!   `tables`, `seed_profile` and `on_call` are TS's `tables`, `seedProfile` and `onCall`. The
//!   recording logger is a `tracing` subscriber that keeps this thread's JSON lines.
//! - Rows are compared as their JSON (TS's keys), so the cases depend on the wire shape of the
//!   port types, not on Rust field types.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use jackioh_engine::config::TURN_CAP_PLAYER_TURNS;
use jackioh_engine::{Action, GameOverReason, PlayerId, Winner};
use jackioh_server::actor::clock::{initial_clocks, match_ceiling_at};
use jackioh_server::actor::contracts::{RecordResultInput, TerminalOutcome};
use jackioh_server::api::results::{reap_stuck_matches, record_result};
use jackioh_server::api::series::{ensure_series_game, start_series};
use jackioh_server::api::series_rules::{NewSeriesInput, pick_deck};
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{MATCH_CEILING_MINUTES, RATING_DEVIATION_START, RATING_VOLATILITY_START};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{
    Db, MatchClocks, MatchRow, MatchSeat, ResultRow, SeriesRow, StartMatchInput, StoreError, Ticket,
};
use jackioh_server::ranked::glicko2::{Glicko, rate_game};
use jackioh_server::ranked::season::season_id_of;
use serde::Serialize;
use serde_json::{Value, json};

use crate::support::deps::test_app;
use crate::support::engine::{fake_deck, install_test_cards};

const MINUTE: i64 = 60 * 1000;
/// A minute past the ceiling (R79, R389), whatever it is.
const PAST_CEILING_MS: i64 = (MATCH_CEILING_MINUTES + 1) * MINUTE;
const MATCH_ID: &str = "match-1";
const A: &str = "profile-a";
const B: &str = "profile-b";

// ---------------------------------------------------------------------------
// The harness's small reads (private copies, CLAUDE.md's fullsend rule 5)
// ---------------------------------------------------------------------------

/// Every store call in this file: one transaction, committed when the call answers.
macro_rules! store {
    ($app:expr, |$t:ident| $call:expr) => {{
        let mut $t = $app.db.begin(None).await.expect("a store transaction begins");
        let out = $call.await.expect(concat!("the store answers ", stringify!($call)));
        $t.commit().await.expect("the store transaction commits");
        out
    }};
}

/// The JSON a row or any port value serialises to (TS's object).
fn j<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("the value serialises")
}

/// TS's `toMatchObject`: every key `expected` names holds the same value in `actual` (objects
/// partially, arrays element for element), and nothing else is compared.
fn assert_match(actual: &Value, expected: &Value) {
    fn matches(actual: &Value, expected: &Value) -> bool {
        match (actual, expected) {
            (Value::Object(have), Value::Object(want)) => {
                want.iter().all(|(key, value)| have.get(key).is_some_and(|got| matches(got, value)))
            }
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
    Value::Array(array.as_array().map(|items| items.iter().map(&f).collect()).unwrap_or_default())
}

/// A slot in whatever integer type the series rules take (TS: `number`).
fn int<T: TryFrom<i64>>(n: i64) -> T
where
    T::Error: std::fmt::Debug,
{
    T::try_from(n).expect("a slot the rules' signature can hold")
}

/// The fake store behind `test_app()`.
fn fake(app: &App) -> &Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => data,
        _ => panic!("support::deps::test_app() runs on the fake store"),
    }
}

/// TS's `createTestDeps()` began on an empty memory store; `test_app()` holds the E2E fixtures
/// (R144), so they go first and every row a test reads is one it wrote.
async fn fresh_store(app: &App) {
    fake(app).lock().await.reset();
}

/// TS's `store.seedProfile(input)`: a profile written without the API (status `active` unless set).
async fn seed_profile(app: &App, input: Value) {
    let _ = fake(app).lock().await.seed_profile(input);
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

async fn reap(app: &Arc<App>) -> Vec<String> {
    reap_stuck_matches(app).await.expect("the reaper sweeps")
}

/// The catalog version every match here is created at (TS's `deps.catalog.version`).
fn catalog_version() -> &'static str {
    jackioh_cards::catalog_version()
}

/// Starts a match through the registry, which writes its row and runs its actor (R79, R417).
async fn start(app: &Arc<App>, input: Value) {
    let input: StartMatchInput = serde_json::from_value(input).expect("a StartMatchInput");
    app.matches.start(app, input).await.expect("the match starts");
}

/// The seats of a match as its row records them: index 0 is p1 (`results.ts`'s `seatsOf`). TS read
/// them off the fake directory's record of the start, which held the same profiles and decks.
async fn started_seats(app: &App, match_id: &str) -> (MatchSeat, MatchSeat) {
    let row = match_row(app, match_id).await;
    if row.is_null() {
        panic!("{match_id} was never started");
    }
    (
        seat_of(row["players"][0].clone(), "p1", row["decks"][0].clone()),
        seat_of(row["players"][1].clone(), "p2", row["decks"][1].clone()),
    )
}

fn seat_of(profile_id: Value, player: &str, deck: Value) -> MatchSeat {
    serde_json::from_value(json!({ "profileId": profile_id, "player": player, "deck": deck })).expect("a MatchSeat")
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

    fn lines(&self) -> Vec<String> {
        let bytes = self.0.lock().expect("the log buffer").clone();
        String::from_utf8_lossy(&bytes).lines().map(str::to_owned).collect()
    }

    /// TS's `log.entries.find((entry) => entry.event === event)`: the first line naming it.
    fn find(&self, event: &str) -> Option<String> {
        let quoted = format!("\"{event}\"");
        self.lines().into_iter().find(|line| line.contains(&quoted))
    }
}

// ---------------------------------------------------------------------------
// The file's own helpers, in TS order
// ---------------------------------------------------------------------------

/// The two seats every scenario plays.
fn seats() -> (MatchSeat, MatchSeat) {
    // Hand slot 0 is `test-lethal` (one hero dies) and slot 1 `test-mutual-lethal` (both do), so the
    // two §2.5 endings that differ only in how many heroes the state check finds dead are reachable
    // from the same fixture and differ by nothing but which card is played.
    (
        seat_of(json!(A), "p1", json!(fake_deck(&["test-lethal", "test-mutual-lethal"]))),
        seat_of(json!(B), "p2", json!(fake_deck(&[]))),
    )
}

/// Runs the engine with the scripted cards (`support/engine.rs`) to a terminal state and hands back
/// the outcome and turn count the actor would pass to `record_result`.
///
/// The game is a testkit `scenario()` on turn 1 with both scripted cards in p1's hand, which is
/// where TS's scripted engine began (it dealt no mulligan). An input names a card by its id
/// (`"card": "test-lethal"`) where TS named a hand slot (`"instanceId": "p1-h0"`); `play` puts the
/// instance id in. Before the inputs, both seats switch off R345's automatic end of turn, so a
/// turn ends only when an input ends it, as in the scripted engine.
fn play(inputs: &[Value]) -> (TerminalOutcome, i32) {
    play_from(1, inputs)
}

/// `play`, from a later turn (the turn cap is reached from the turn before it).
fn play_from(turn: i32, inputs: &[Value]) -> (TerminalOutcome, i32) {
    let s = jackioh_engine::testkit::scenario(json!({
        "seed": "seed-1",
        "turn": turn,
        "p1": { "hand": ["test-lethal", "test-mutual-lethal"], "mana": 10 },
        "p2": {},
    }));
    let mut state = s.state().clone();
    let mut actions = vec![
        json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" }),
        json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p2" }),
    ];
    actions.extend(inputs.iter().cloned());
    for (n, mut input) in actions.into_iter().enumerate() {
        if let Some(card) = input.get("card").and_then(Value::as_str).map(str::to_owned) {
            let instance_id = s.card(&card).id.clone();
            if let Some(fields) = input.as_object_mut() {
                fields.remove("card");
                fields.insert("instanceId".to_owned(), json!(instance_id));
            }
        }
        input["nonce"] = json!(format!("nonce-{}", n + 1));
        let kind = input["type"].clone();
        let action: Action = serde_json::from_value(input).expect("an Action");
        let result = jackioh_engine::reduce(&state, &action);
        if let Some(error) = result.error {
            panic!("{kind}: {error}");
        }
        state = result.state;
    }
    let Some(result) = state.result else {
        panic!("the scripted game did not end");
    };
    (TerminalOutcome { winner: result.winner, reason: result.reason }, state.turn)
}

/// R603: the Glicko-2 move one ranked game makes between two players new to it (each at a new
/// player's deviation and volatility), `score_a` being the first one's score. TS's `move`.
fn rating_move(rating_a: f64, rating_b: f64, score_a: f64) -> (f64, f64) {
    let fresh = |rating: f64| Glicko {
        rating,
        deviation: RATING_DEVIATION_START,
        volatility: RATING_VOLATILITY_START,
    };
    let next = rate_game(&fresh(rating_a), &fresh(rating_b), score_a);
    (next.a.rating, next.b.rating)
}

/// The last two player-turns before the cap end the match in a draw (§2.5). TS ended thirty turns
/// of its scripted engine (`FAKE_TURN_CAP`); the real engine's cap is `TURN_CAP_PLAYER_TURNS`,
/// reached here from the turn before it (`play_from`), since thirty real turns of empty libraries
/// would end in fatigue first.
fn to_the_turn_cap() -> Vec<Value> {
    (0..2)
        .map(|i| {
            let player = if i % 2 == 0 { "p1" } else { "p2" };
            json!({ "type": "endTurn", "playerId": player })
        })
        .collect()
}

/// TS's scenario options: both ratings, how long ago the match started, and whether the queue
/// paired it.
#[derive(Default)]
struct ScenarioOptions {
    ratings: Option<(f64, f64)>,
    started_offset_ms: Option<i64>,
    ranked: Option<bool>,
}

/// A ranked match `MATCH_ID` between A and B, unless `ranked: false` makes it a room's (R604).
async fn scenario(options: ScenarioOptions) -> Arc<App> {
    let ranked = options.ranked.unwrap_or(true);
    let app = test_app().await;
    fresh_store(&app).await;
    let (rating_a, rating_b) = options.ratings.unwrap_or((1000.0, 1000.0));
    seed_profile(&app, json!({ "id": A, "rating": rating_a, "inMatchId": MATCH_ID })).await;
    seed_profile(&app, json!({ "id": B, "rating": rating_b, "inMatchId": MATCH_ID })).await;
    let (first, second) = seats();
    match options.started_offset_ms {
        Some(offset) => {
            // Started long ago and not answering: the row alone, as a crashed actor leaves it.
            let started_at = now_ms() - offset;
            let row: MatchRow = serde_json::from_value(json!({
                "id": MATCH_ID,
                "seed": "seed-1",
                "players": [A, B],
                "decks": [j(&first)["deck"], j(&second)["deck"]],
                "catalogVersion": catalog_version(),
                "ranked": ranked,
                "status": "live",
                "createdAt": started_at,
                "finishedAt": null,
                "clocks": j(&initial_clocks(started_at)),
            }))
            .expect("a MatchRow");
            store!(&app, |t| t.matches_create(&row));
        }
        None => {
            start(
                &app,
                json!({
                    "matchId": MATCH_ID,
                    "seed": "seed-1",
                    "catalogVersion": catalog_version(),
                    "ranked": ranked,
                    "seats": [j(&first), j(&second)],
                }),
            )
            .await;
        }
    }
    app
}

async fn record(app: &Arc<App>, inputs: &[Value]) -> ResultRow {
    let (outcome, turns) = play(inputs);
    record_result(app, RecordResultInput {
        match_id: MATCH_ID.to_owned(),
        seats: seats(),
        outcome,
        turns,
        at: now_ms(),
        last_boards: None,
    })
    .await
    .expect("the result is written")
}

/// What `expect_one_ending` checks: the winner (a profile, or `None` for a draw), the reason and
/// both ratings after.
struct Ending {
    winner: Option<&'static str>,
    reason: &'static str,
    rating_after: (f64, f64),
}

/// §9.5: "Every ending records a result and clears both players' in-match state."
async fn expect_one_ending(app: &App, expected: Ending) {
    let results = table(app, |data| j(&data.tables.results)).await;
    assert_eq!(results.as_array().map(Vec::len), Some(1));
    let row = &results[0];
    assert_eq!(row["matchId"], MATCH_ID);
    assert_eq!(row["players"], json!([A, B]));
    assert_eq!(row["winnerProfileId"], json!(expected.winner));
    assert_eq!(row["reason"], expected.reason);
    assert_eq!(row["ratingAfter"], json!([expected.rating_after.0, expected.rating_after.1]));

    let (profile_a, profile_b) = (profile(app, A).await, profile(app, B).await);
    assert_eq!(
        json!([profile_a["rating"], profile_b["rating"]]),
        json!([expected.rating_after.0, expected.rating_after.1]),
    );
    // Both, for every reason — M7-T1's "both can queue again".
    assert!(profile_a["inMatchId"].is_null());
    assert!(profile_b["inMatchId"].is_null());

    assert_eq!(match_row(app, MATCH_ID).await["status"], "finished");
}

mod results_m7_t2 {
    use super::*;

    /// R603: two new players at 1000, equally rated, so the winner gains exactly what the loser gives.
    fn win_loss() -> (f64, f64) {
        rating_move(1000.0, 1000.0, 1.0)
    }

    #[tokio::test]
    async fn hero_death_the_winner_is_rated_up_and_the_loser_down() {
        install_test_cards();
        let (win, loss) = win_loss();
        let app = scenario(ScenarioOptions::default()).await;
        let row = j(&record(&app, &[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]).await);
        assert_eq!(row["reason"], "hero-death");
        assert_eq!(row["turns"], json!(1));
        assert_eq!(row["ratingBefore"], json!([1000.0, 1000.0]));
        expect_one_ending(&app, Ending { winner: Some(A), reason: "hero-death", rating_after: (win, loss) }).await;
    }

    #[tokio::test]
    async fn both_heroes_dead_both_heroes_dying_in_the_same_check_is_a_draw_and_rates_as_one_2_5() {
        // The seventh reason `api/results.rs`'s own header names and `0004_matches.sql`'s `reason`
        // CHECK allows. It is the one ending that is a *draw produced by lethal damage*, so the thing
        // to prove is that the writer scores it 0.5/0.5 and names no winner — `scoreForSeat` decides
        // that on `outcome.winner === "draw"` alone, and a writer that read the reason instead (or
        // that treated "somebody died" as a win) would name A here.
        install_test_cards();
        let app = scenario(ScenarioOptions { ratings: Some((1200.0, 1000.0)), ..Default::default() }).await;
        let row = j(&record(&app, &[json!({ "type": "play", "card": "test-mutual-lethal", "playerId": "p1" })]).await);

        // PREMISE: the scripted engine really produced this reason, so the assertions below are about
        // the writer and not about a string this file typed out.
        assert_eq!(row["reason"], "both-heroes-dead");
        assert_eq!(row["turns"], json!(1));
        assert_eq!(row["ratingBefore"], json!([1200.0, 1000.0]));

        // The same move a draw gets anywhere else (R603): the favourite gives, the underdog takes.
        let expected = rating_move(1200.0, 1000.0, 0.5);
        assert!(expected.0 < 1200.0);
        assert!(expected.1 > 1000.0);
        expect_one_ending(&app, Ending { winner: None, reason: "both-heroes-dead", rating_after: expected }).await;
    }

    #[tokio::test]
    async fn concede_the_conceding_player_loses_2_5() {
        install_test_cards();
        let (win, loss) = win_loss();
        let app = scenario(ScenarioOptions::default()).await;
        record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await;
        expect_one_ending(&app, Ending { winner: Some(A), reason: "concede", rating_after: (win, loss) }).await;
    }

    #[tokio::test]
    async fn draw_accepted_a_draw_moves_both_ratings_toward_each_other() {
        install_test_cards();
        let app = scenario(ScenarioOptions { ratings: Some((1200.0, 1000.0)), ..Default::default() }).await;
        let row = j(&record(
            &app,
            &[
                json!({ "type": "offerDraw", "playerId": "p1" }),
                json!({ "type": "answerDraw", "accept": true, "playerId": "p2" }),
            ],
        )
        .await);
        let expected = rating_move(1200.0, 1000.0, 0.5);
        assert_eq!(row["ratingBefore"], json!([1200.0, 1000.0]));
        // The favourite gives points away on a draw; the underdog takes them.
        assert!(expected.0 < 1200.0);
        assert!(expected.1 > 1000.0);
        expect_one_ending(&app, Ending { winner: None, reason: "draw-accepted", rating_after: expected }).await;
    }

    /// TS: "turn-cap: the 30th player-turn is a draw" (its scripted engine's cap, `FAKE_TURN_CAP`).
    #[tokio::test]
    async fn turn_cap_the_last_player_turn_is_a_draw() {
        install_test_cards();
        let app = scenario(ScenarioOptions::default()).await;
        let (outcome, turns) = play_from(TURN_CAP_PLAYER_TURNS - 1, &to_the_turn_cap());
        let row = j(&record_result(&app, RecordResultInput {
            match_id: MATCH_ID.to_owned(),
            seats: seats(),
            outcome,
            turns,
            at: now_ms(),
            last_boards: None,
        })
        .await
        .expect("the result is written"));
        assert!(row["turns"].as_i64().unwrap_or(0) >= i64::from(TURN_CAP_PLAYER_TURNS));
        expect_one_ending(&app, Ending { winner: None, reason: "turn-cap", rating_after: (1000.0, 1000.0) }).await;
    }

    #[tokio::test]
    async fn disconnect_the_disconnected_player_loses_9_5() {
        install_test_cards();
        let (win, loss) = win_loss();
        let app = scenario(ScenarioOptions::default()).await;
        record(&app, &[json!({ "type": "disconnectExpired", "player": "p1", "playerId": "p1" })]).await;
        expect_one_ending(&app, Ending { winner: Some(B), reason: "disconnect", rating_after: (loss, win) }).await;
    }

    #[tokio::test]
    async fn match_ceiling_an_actor_resolved_ceiling_is_a_draw_with_the_ordinary_rating_move_r112() {
        install_test_cards();
        let app = scenario(ScenarioOptions { ratings: Some((1200.0, 1000.0)), ..Default::default() }).await;
        let row = j(&record(&app, &[json!({ "type": "ceilingReached", "playerId": "p1" })]).await);
        let expected = rating_move(1200.0, 1000.0, 0.5);
        assert_eq!(row["turns"], json!(1));
        expect_one_ending(&app, Ending { winner: None, reason: "match-ceiling", rating_after: expected }).await;
    }

    #[tokio::test]
    async fn writes_one_row_and_rates_once_when_it_is_called_twice_for_the_same_match_9_5() {
        install_test_cards();
        let (win, loss) = win_loss();
        let app = scenario(ScenarioOptions::default()).await;
        let first = record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await;
        let again = record_result(&app, RecordResultInput {
            match_id: MATCH_ID.to_owned(),
            seats: seats(),
            // Even a different outcome cannot rewrite history: the row already written is returned.
            outcome: TerminalOutcome { winner: Winner::P2, reason: GameOverReason::HeroDeath },
            turns: 99,
            at: now_ms() + 1,
            last_boards: None,
        })
        .await
        .expect("the result is written");
        assert_eq!(j(&again), j(&first));
        expect_one_ending(&app, Ending { winner: Some(A), reason: "concede", rating_after: (win, loss) }).await;
    }

    #[tokio::test]
    async fn returns_the_racing_first_writers_row_when_two_writes_collide_on_the_result_key_9_5() {
        install_test_cards();
        let app = scenario(ScenarioOptions::default()).await;
        // Two writers — an actor and the reaper — pass `getByMatch` together, each inside its own
        // transaction. The first commits; the second's insert meets `results_pkey`, which the port
        // reports as `StoreError::Duplicate` (TS's DuplicateResultError). Simulate exactly that: this
        // store's first `results.insert` refuses as if the collision had happened, and the racing
        // writer's row lands the moment the loser's transaction lets the store go, so the next
        // `getByMatch` — run inside the retry's fresh transaction — sees the row the winner committed.
        //
        // TS rewrote `getByMatch` to push the row; the fake's one seam is `on_call`, called with the
        // store held, so the racing writer is a thread queued on the store's lock instead: the lock
        // is fair, so it writes after the loser rolls back and before the loser's retry reads.
        let winner_row: ResultRow = serde_json::from_value(json!({
            "matchId": MATCH_ID,
            "players": [A, B],
            "winnerProfileId": null,
            "reason": "match-ceiling",
            "turns": 0,
            "endedAt": now_ms(),
            "ratingBefore": [1000.0, 1000.0],
            "ratingAfter": [1000.0, 1000.0],
        }))
        .expect("a ResultRow");
        let collided = Arc::new(AtomicBool::new(false));
        {
            let data = fake(&app).clone();
            let collided = collided.clone();
            let winner_row = winner_row.clone();
            fake(&app).lock().await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
                if method != "results.insert" || collided.swap(true, Ordering::SeqCst) {
                    return Ok(());
                }
                // The racing writer's commit: visible once the loser's transaction rolls back.
                let (queued, waiting) = std::sync::mpsc::channel::<()>();
                let data = data.clone();
                let row = winner_row.clone();
                std::thread::spawn(move || {
                    let _ = queued.send(());
                    data.blocking_lock().tables.results.push(row);
                });
                let _ = waiting.recv();
                // Long enough for the writer to be queued on the lock this transaction holds.
                std::thread::sleep(Duration::from_millis(50));
                Err(StoreError::Duplicate(MATCH_ID.to_owned()))
            }));
        }

        let row = record_result(&app, RecordResultInput {
            match_id: MATCH_ID.to_owned(),
            seats: seats(),
            outcome: TerminalOutcome { winner: Winner::P1, reason: GameOverReason::HeroDeath },
            turns: 12,
            at: now_ms(),
            last_boards: None,
        })
        .await
        .expect("the result is written");
        assert!(collided.load(Ordering::SeqCst), "the first insert collided");
        assert_eq!(j(&row), j(&winner_row));
        // One effective write: the winner's row, and the loser added nothing — no second row, no
        // rating move of its own, and the match still finished exactly once.
        assert_eq!(table(&app, |data| json!(data.tables.results.len())).await, json!(1));
        assert_eq!(profile(&app, A).await["rating"].as_f64(), Some(1000.0));
        assert_eq!(profile(&app, B).await["rating"].as_f64(), Some(1000.0));
        assert_eq!(table(&app, |data| j(&data.tables.season_ranks)).await, json!([]));
        assert_eq!(table(&app, |data| j(&data.tables.rated_games)).await, json!([]));
    }

    #[tokio::test]
    async fn clears_a_stray_open_queue_ticket_so_both_players_can_queue_again_9_5() {
        install_test_cards();
        let app = scenario(ScenarioOptions::default()).await;
        let ticket: Ticket = serde_json::from_value(json!({
            "id": "ticket-a",
            "profileId": A,
            "rating": 1000.0,
            "mode": "bo1",
            "deck": j(&seats().0)["deck"],
            "trio": null,
            "catalogVersion": catalog_version(),
            "enqueuedAt": now_ms(),
            "status": "open",
            "matchId": null,
        }))
        .expect("a Ticket");
        store!(&app, |t| t.tickets_insert(&ticket));
        record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await;
        assert!(store!(&app, |t| t.tickets_open_for_profile(A)).is_none());
    }

    mod ranked_and_unranked_r604_r611 {
        use super::*;

        #[tokio::test]
        async fn r604_a_room_challenge_moves_neither_rating_nor_rank_and_records_both_ratings_unchanged() {
            install_test_cards();
            let app = scenario(ScenarioOptions {
                ratings: Some((1200.0, 1000.0)),
                ranked: Some(false),
                ..Default::default()
            })
            .await;
            let row = j(&record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await);
            assert_match(
                &row,
                &json!({ "winnerProfileId": A, "ratingBefore": [1200.0, 1000.0], "ratingAfter": [1200.0, 1000.0] }),
            );
            expect_one_ending(&app, Ending { winner: Some(A), reason: "concede", rating_after: (1200.0, 1000.0) })
                .await;
            assert_eq!(table(&app, |data| j(&data.tables.season_ranks)).await, json!([]));
            assert_eq!(table(&app, |data| j(&data.tables.rated_games)).await, json!([]));
            // Deviation and volatility do not move either.
            assert_eq!(profile(&app, A).await["ratingDeviation"].as_f64(), Some(RATING_DEVIATION_START));
        }

        #[tokio::test]
        async fn r604_a_ranked_match_moves_both_hidden_ratings_their_deviations_and_both_players_seasons() {
            install_test_cards();
            let app = scenario(ScenarioOptions::default()).await;
            record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await;
            let profile_a = profile(&app, A).await;
            assert!(profile_a["ratingDeviation"].as_f64().unwrap_or(f64::INFINITY) < RATING_DEVIATION_START);
            let ranks = table(&app, |data| j(&data.tables.season_ranks)).await;
            assert_eq!(
                map(&ranks, |rank| json!([rank["profileId"], rank["games"], rank["wins"], rank["losses"]])),
                json!([[A, 1, 1, 0], [B, 1, 0, 1]]),
            );
        }

        #[tokio::test]
        async fn r611_records_the_rated_game_version_pilots_result_and_both_ratings_and_ranks_before_and_after() {
            install_test_cards();
            let app = scenario(ScenarioOptions { ratings: Some((1200.0, 1000.0)), ..Default::default() }).await;
            record(&app, &[json!({ "type": "disconnectExpired", "player": "p1", "playerId": "p1" })]).await;
            let expected = rating_move(1200.0, 1000.0, 0.0);
            let games = table(&app, |data| j(&data.tables.rated_games)).await;
            assert_eq!(games.as_array().map(Vec::len), Some(1));
            let game = &games[0];
            // TS ran as `TEST_PATCH_VERSION` (season `v0.1`); the Rust server rates in the season of
            // the patch it was built at (SURFACE §11.3).
            let patch = catalog_version();
            assert_match(
                game,
                &json!({
                    "id": MATCH_ID,
                    "kind": "match",
                    "seasonId": season_id_of(patch),
                    "patchVersion": patch,
                    "catalogVersion": catalog_version(),
                    // R603: a disconnect is a loss like any other.
                    "winnerSide": 1,
                    "reason": "disconnect",
                }),
            );
            assert_eq!(
                map(&game["sides"], |side| json!([side["profileId"], side["botId"], side["pilot"]])),
                json!([[A, null, "human"], [B, null, "human"]]),
            );
            assert_eq!(
                map(&game["sides"], |side| json!([side["before"]["rating"], side["after"]["rating"]])),
                json!([[1200.0, expected.0], [1000.0, expected.1]]),
            );
            assert_eq!(map(&game["sides"], |side| side["rankAfter"]["tier"].clone()), json!(["raisin", "raisin"]));
        }
    }

    mod the_reaper_9_5_r112 {
        use super::*;

        #[tokio::test]
        async fn resolves_a_match_past_its_ceiling_as_a_draw_and_leaves_both_ratings_unchanged_r112() {
            install_test_cards();
            let app = scenario(ScenarioOptions {
                ratings: Some((1200.0, 1000.0)),
                started_offset_ms: Some(PAST_CEILING_MS),
                ..Default::default()
            })
            .await;
            let started_at = now_ms() - PAST_CEILING_MS;
            assert!(match_ceiling_at(started_at) < now_ms());

            assert_eq!(reap(&app).await, vec![MATCH_ID.to_owned()]);

            let results = table(&app, |data| j(&data.tables.results)).await;
            let row = &results[0];
            // R112: "records `turns = 0` and leaves both ratings unchanged".
            assert_eq!(row["turns"], json!(0));
            assert_eq!(row["ratingBefore"], json!([1200.0, 1000.0]));
            expect_one_ending(&app, Ending { winner: None, reason: "match-ceiling", rating_after: (1200.0, 1000.0) })
                .await;
            // The in-memory actor is dropped; the log stays.
            assert!(!app.matches.has(MATCH_ID));
        }

        #[tokio::test]
        async fn leaves_a_match_that_has_not_reached_its_ceiling_alone() {
            install_test_cards();
            let app = scenario(ScenarioOptions::default()).await;
            assert_eq!(reap(&app).await, Vec::<String>::new());
            assert_eq!(table(&app, |data| json!(data.tables.results.len())).await, json!(0));
            assert!(app.matches.has(MATCH_ID));
        }

        #[tokio::test]
        async fn is_a_no_op_once_the_actor_has_already_recorded_the_ending() {
            install_test_cards();
            let (win, loss) = win_loss();
            let app = scenario(ScenarioOptions { started_offset_ms: Some(PAST_CEILING_MS), ..Default::default() }).await;
            record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await;
            assert_eq!(reap(&app).await, Vec::<String>::new());
            expect_one_ending(&app, Ending { winner: Some(A), reason: "concede", rating_after: (win, loss) }).await;
        }

        #[tokio::test]
        async fn keeps_its_own_row_when_an_actor_reports_the_same_match_afterwards() {
            install_test_cards();
            let app = scenario(ScenarioOptions {
                ratings: Some((1200.0, 1000.0)),
                started_offset_ms: Some(PAST_CEILING_MS),
                ..Default::default()
            })
            .await;
            reap(&app).await;
            let late = j(&record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await);
            assert_eq!(late["reason"], "match-ceiling");
            expect_one_ending(&app, Ending { winner: None, reason: "match-ceiling", rating_after: (1200.0, 1000.0) })
                .await;
        }
    }

    mod a_game_of_a_conquest_series_r262_r263 {
        use super::*;

        const SERIES_ID: &str = "series-1";

        /// A trio whose decks are named after their owner and slot. Their cards are the scripted
        /// filler deck (TS: one made-up card each), so the real engine accepts every game they play.
        fn trio(owner: &str) -> Value {
            let deck = |slot: i64| json!({ "name": format!("{owner} {slot}"), "cards": fake_deck(&[]) });
            json!({ "name": owner, "decks": [deck(0), deck(1), deck(2)] })
        }

        async fn series_row(app: &App) -> SeriesRow {
            match store!(app, |t| t.series_get(SERIES_ID)) {
                Some(series) => series,
                None => panic!("the series is gone"),
            }
        }

        /// The picks a side still owes, one compare-and-set each, then the game started as the pick
        /// route starts it. A side whose last deck was picked for it (R332) is not asked.
        async fn play_next(app: &Arc<App>, slots: [i64; 2]) -> SeriesRow {
            let now = now_ms();
            let mut row = series_row(app).await;
            for (index, seat) in [PlayerId::P1, PlayerId::P2].into_iter().enumerate() {
                let view = j(&row);
                if view["status"] != "picking" || !view["sides"][index]["pick"].is_null() {
                    continue;
                }
                let next = pick_deck(&row, seat, int(slots[index]), now, None).expect("the pick applies");
                let _ = store!(app, |t| t.series_update(&next));
                row = next;
            }
            ensure_series_game(app, &row).await.expect("the game starts");
            row
        }

        /// The game in play ends with `winner` (a profile, or "draw"), reported with the match's own seats.
        async fn finish(app: &Arc<App>, winner: &str) {
            let series = series_row(app).await;
            let match_id = j(&series)["nextMatchId"].as_str().unwrap_or_default().to_owned();
            let started = started_seats(app, &match_id).await;
            let outcome = if winner == "draw" {
                TerminalOutcome { winner: Winner::Draw, reason: GameOverReason::DrawAccepted }
            } else {
                let player = [&started.0, &started.1]
                    .into_iter()
                    .map(j)
                    .find(|seat| seat["profileId"] == winner)
                    .map(|seat| seat["player"].clone())
                    .unwrap_or(json!("p1"));
                let winner: Winner = serde_json::from_value(player).expect("a seat");
                TerminalOutcome { winner, reason: GameOverReason::HeroDeath }
            };
            record_result(app, RecordResultInput {
                match_id,
                seats: started,
                outcome,
                turns: 3,
                at: now_ms(),
                last_boards: None,
            })
            .await
            .expect("the result is written");
        }

        /// A in series seat p1 at 1200, B in p2 at 1000, game 1 (match `MATCH_ID`) being played.
        ///
        /// TS swapped in `createFakeMatchDirectory(deps.store)`, which writes the match row; the
        /// registry behind `app.matches` writes it the same way, and runs the game's actor besides.
        async fn series_scenario() -> Arc<App> {
            let app = test_app().await;
            fresh_store(&app).await;
            seed_profile(&app, json!({ "id": A, "rating": 1200.0 })).await;
            seed_profile(&app, json!({ "id": B, "rating": 1000.0 })).await;
            let input: NewSeriesInput = serde_json::from_value(json!({
                "seriesId": SERIES_ID,
                "firstMatchId": MATCH_ID,
                "sides": [
                    { "profileId": A, "trio": trio("a") },
                    { "profileId": B, "trio": trio("b") },
                ],
                "seedBase": "seed",
                "catalogVersion": catalog_version(),
                "ranked": true,
            }))
            .expect("a NewSeriesInput");
            let mut tx = app.db.begin(None).await.expect("store.tx");
            start_series(&app, input, &mut tx).await.expect("the series starts");
            tx.commit().await.expect("the series commits");
            play_next(&app, [0, 0]).await;
            app
        }

        async fn ratings(app: &App) -> Value {
            json!([profile(app, A).await["rating"], profile(app, B).await["rating"]])
        }

        #[tokio::test]
        async fn r262_a_series_games_row_leaves_both_ratings_unchanged_and_the_series_records_the_game_in_the_same_write() {
            install_test_cards();
            let (logs, _logging) = Logs::capture();
            let app = series_scenario().await;
            // Game 1: series p1 (A) goes first, so the match's p1 is A, as in `seats`.
            let row = j(&record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await);

            assert_match(
                &row,
                &json!({
                    "winnerProfileId": A,
                    "reason": "concede",
                    "ratingBefore": [1200.0, 1000.0],
                    "ratingAfter": [1200.0, 1000.0],
                }),
            );
            assert_eq!(ratings(&app).await, json!([1200.0, 1000.0]));
            let series = j(&series_row(&app).await);
            assert_eq!(series["status"], "picking");
            assert_match(&series["games"][0], &json!({ "matchId": MATCH_ID, "winner": "p1", "reason": "concede" }));
            assert_eq!(map(&series["sides"], |side| side["wins"].clone()), json!([1, 0]));
            // Everything else a result does, it still does.
            assert!(profile(&app, A).await["inMatchId"].is_null());
            assert_eq!(match_row(&app, MATCH_ID).await["status"], "finished");
            let ended = logs.find("match.ended").expect("the result logs match.ended");
            assert!(ended.contains("unchanged"), "{ended}");
            assert!(ended.contains(SERIES_ID), "{ended}");
        }

        #[tokio::test]
        async fn r263_the_reapers_ceiling_draw_counts_for_neither_side_and_a_next_game_both_sides_last_decks_begin_is_started_r332() {
            install_test_cards();
            let app = series_scenario().await;
            finish(&app, A).await;
            play_next(&app, [1, 0]).await;
            finish(&app, B).await;
            play_next(&app, [1, 1]).await;
            finish(&app, A).await;
            play_next(&app, [2, 1]).await;
            finish(&app, B).await;
            // Two wins each: game 5 began by itself, on each side's last deck.
            let game5 = j(&series_row(&app).await);
            assert_eq!(game5["status"], "playing");
            let game5_id = game5["nextMatchId"].as_str().unwrap_or_default().to_owned();
            assert_match(&game5["games"][4], &json!({ "gameNo": 5, "slots": [2, 2] }));

            // TS: "The fake directory's rows carry a ceiling of 0, so game 5 is long past it." The
            // registry's row carries the real one (R79), so game 5's actor is stopped, as a crashed
            // one would be, and its ceiling moved to 0: the reaper is for a match nobody answers for.
            app.matches.stop(&game5_id).await;
            let clocks = {
                let mut clocks = match_row(&app, &game5_id).await["clocks"].clone();
                clocks["ceilingAt"] = json!(0);
                serde_json::from_value::<MatchClocks>(clocks).expect("MatchClocks")
            };
            store!(&app, |t| t.matches_set_clocks(&game5_id, &clocks));
            assert_eq!(reap(&app).await, vec![game5_id.clone()]);
            let results = table(&app, |data| j(&data.tables.results)).await;
            let reaped = results
                .as_array()
                .and_then(|rows| rows.iter().find(|result| result["matchId"] == game5_id.as_str()))
                .cloned()
                .unwrap_or(Value::Null);
            assert_match(
                &reaped,
                &json!({
                    "reason": "match-ceiling",
                    "turns": 0,
                    "ratingBefore": [1200.0, 1000.0],
                    "ratingAfter": [1200.0, 1000.0],
                }),
            );

            let series = j(&series_row(&app).await);
            assert_eq!(map(&series["sides"], |side| side["wins"].clone()), json!([2, 2]));
            assert_match(&series["games"][4], &json!({ "winner": "draw", "reason": "match-ceiling" }));
            assert_eq!(series["status"], "playing");
            assert_match(&series["games"][5], &json!({ "gameNo": 6, "slots": [2, 2], "first": "p2" }));
            let game6_id = series["nextMatchId"].as_str().unwrap_or_default().to_owned();
            let game6 = match_row(&app, &game6_id).await;
            assert_match(&game6, &json!({ "seed": "seed:6" }));
            assert_eq!(game6["players"], json!([B, A]));
            assert_eq!(profile(&app, B).await["inMatchId"], json!(game6_id));
            assert_eq!(ratings(&app).await, json!([1200.0, 1000.0]));
        }

        #[tokio::test]
        async fn r262_the_game_that_ends_the_series_moves_both_ratings_once_from_the_ratings_at_the_time_it_ends() {
            install_test_cards();
            let app = series_scenario().await;
            record(&app, &[json!({ "type": "concede", "playerId": "p2" })]).await;
            play_next(&app, [1, 1]).await;
            // Game 2: B is the match's p1. A wins it.
            finish(&app, A).await;
            // Game 3: A's last deck is picked for A (R332); B picks, and A wins with every deck.
            let game3 = j(&play_next(&app, [2, 2]).await);
            assert_eq!(game3["games"][2]["slots"], json!([2, 2]));
            finish(&app, A).await;
            let expected = rating_move(1200.0, 1000.0, 1.0);
            assert_eq!(ratings(&app).await, json!([expected.0, expected.1]));
            assert_match(
                &j(&series_row(&app).await),
                &json!({
                    "status": "over",
                    "winner": "p1",
                    "endReason": "decided",
                    "ratingBefore": [1200.0, 1000.0],
                    "ratingAfter": [expected.0, expected.1],
                }),
            );
            // Every game row is unrated.
            let results = table(&app, |data| j(&data.tables.results)).await;
            assert_eq!(results.as_array().map(Vec::len), Some(3));
            for result in results.as_array().into_iter().flatten() {
                assert_eq!(result["ratingAfter"], result["ratingBefore"]);
            }
        }
    }
}
