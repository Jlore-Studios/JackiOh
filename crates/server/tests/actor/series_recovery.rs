//! SPEC §11 R263: a Best-of-3 series survives a restart.
//!
//! A series is a database row written by compare-and-set; a game's result, the series' record of it
//! and (when the game ends the series) the rating move commit in one transaction; and a sweeper
//! starts any game whose picks are in but whose match is not running, once the row has sat unchanged
//! for `SERIES_START_GRACE_SECONDS`. Each of those is checked here the way it can fail:
//!
//!  - a crash between "both picked" and "match started" (the playing row written, no start);
//!  - a restart: a second app over the same store — a fresh registry, fresh ids — carries on a
//!    series the first one began;
//!  - a store failure in the middle of the result's transaction, which must take the series advance
//!    and the rating move down with the result row;
//!  - two starts racing for the same game.
//!
//! Port of `apps/server/test/match/series-recovery.test.ts`. TS's processes each had a fake match
//! directory that recorded starts and could be told to throw; the Rust server has the one real
//! registry (SURFACE §11.2), so here a start is the match row it wrote (read off the shared store),
//! a "process" is an app over the same store whose predecessor's actors are stopped (the crash),
//! a start that fails is the store refusing the match row's insert (TS's `store.onCall`), and every
//! trio holds legal decks of real cards so the real engine can begin the game. TS's three
//! compare-and-set races (`store.series.update` replaced to lose, or to land a rival write first)
//! need a seam the fake store does not have; they are listed under GAPS in
//! `.fullsend/notes/part-19-6.md`.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use jackioh_engine::wire::{GameOverReason, PlayerId, Winner};
use jackioh_server::api::results::record_result;
use jackioh_server::api::series::{ensure_series_game, start_series, sweep_series};
use jackioh_server::api::series_rules::{game_ended, pick_deck};
use jackioh_server::app::App;
use jackioh_server::config::{
    MATCH_CEILING_MINUTES, RATING_DEVIATION_START, RATING_VOLATILITY_START, SERIES_START_GIVE_UP_SECONDS,
    SERIES_START_GRACE_SECONDS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, SeriesRow, StoreError};
use jackioh_server::ranked::glicko2::{rate_game, Glicko};

use crate::support::deps::{add_user, call, test_app, test_app_with, TestAppOptions};

const ALICE: &str = "profile-alice";
const BOB: &str = "profile-bob";
const SERIES_ID: &str = "series-1";
const FIRST_MATCH: &str = "match-1";

fn grace_ms() -> i64 {
    SERIES_START_GRACE_SECONDS as i64 * 1000
}

fn give_up_ms() -> i64 {
    SERIES_START_GIVE_UP_SECONDS as i64 * 1000
}

/// One store call in its own transaction, as TS's `deps.store.<sub>.<method>(…)` was.
macro_rules! store {
    ($app:expr, $t:ident => $call:expr) => {{
        let mut $t = $app.db.begin(None).await.expect("the fake store opens a transaction");
        let out = $call;
        $t.commit().await.expect("the fake store commits");
        out
    }};
}

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a serialisable value")
}

/// The fake store behind the test app (SURFACE §11.2's `Db::Fake`).
fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// One table of the fake store, as TS's `deps.store.tables.<name>` read it: rows as JSON.
async fn table(app: &App, pick: impl Fn(&FakeData) -> Value) -> Vec<Value> {
    let data = fake(app);
    let data = data.lock().await;
    match pick(&data) {
        Value::Array(rows) => rows,
        other => panic!("not a table: {other}"),
    }
}

/// TS's `store.onCall`: every store method named `method` fails with `message` until cleared.
async fn fail_on(app: &App, method: &'static str, message: &'static str) {
    fake(app).lock().await.on_call = Some(Arc::new(move |called: &str| {
        if called == method { Err(StoreError::Other(message.to_string())) } else { Ok(()) }
    }));
}

async fn stop_failing(app: &App) {
    fake(app).lock().await.on_call = None;
}

// ---------------------------------------------------------------------------------------------
// Time and logs: TS's manual `Timers` and recording `Logger`
// ---------------------------------------------------------------------------------------------

/// TS's `deps.timers.now()`: the server's clock, read as the epoch-ms stamp it wrote on the series
/// row at `begin` plus the tokio time (paused, moved only by `advance`) since.
#[derive(Clone, Copy)]
struct Clock {
    base_ms: i64,
    at: tokio::time::Instant,
}

impl Clock {
    fn now(&self) -> i64 {
        self.base_ms + i64::try_from(self.at.elapsed().as_millis()).expect("milliseconds fit")
    }
}

/// TS's `deps.timers.advance(ms)`: every timer due fires, and the woken tasks get to run.
async fn advance(ms: i64) {
    tokio::time::advance(Duration::from_millis(u64::try_from(ms).expect("time moves forward"))).await;
    tokio::task::yield_now().await;
}

/// Every `tracing` line the server writes on this thread while the guard lives, as JSON.
#[derive(Clone, Default)]
struct Logs(Arc<std::sync::Mutex<Vec<u8>>>);

struct LogWriter(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("the log buffer").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> LogWriter {
        LogWriter(Arc::clone(&self.0))
    }
}

impl Logs {
    fn record() -> (Logs, tracing::subscriber::DefaultGuard) {
        let logs = Logs::default();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_max_level(tracing::Level::TRACE)
            .with_writer(logs.clone())
            .finish();
        let guard = tracing::subscriber::set_default(subscriber);
        (logs, guard)
    }

    /// Whether any line names `event` (SURFACE §11.3 keeps TS's event names).
    fn has(&self, event: &str) -> bool {
        let bytes = self.0.lock().expect("the log buffer").clone();
        String::from_utf8_lossy(&bytes)
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|entry| event_of(&entry) == Some(event))
    }
}

fn event_of(entry: &Value) -> Option<&str> {
    ["event", "message"].iter().find_map(|key| {
        entry.get(*key).or_else(|| entry.get("fields").and_then(|fields| fields.get(*key))).and_then(Value::as_str)
    })
}

// ---------------------------------------------------------------------------------------------
// Trios, processes and the series
// ---------------------------------------------------------------------------------------------

/// Every playable catalog id, in `catalog.json` order.
fn playable() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| !def.token && !to_json(&def.tags).as_array().is_some_and(|tags| tags.contains(&json!("Token"))))
        .map(|(id, _)| id.clone())
        .collect()
}

/// The cards of `owner`'s deck in `slot`: a legal deck of real ids, disjoint from every other
/// deck here (TS: `<owner>-card-<slot>a`, `<owner>-card-<slot>b`).
fn trio_deck(owner: &str, slot: usize) -> Vec<String> {
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    let first = if owner == "alice" { 0 } else { 3 } + slot;
    playable()[first * size..(first + 1) * size].to_vec()
}

fn trio(owner: &str) -> Value {
    let deck = |slot: usize| json!({ "name": format!("{owner} deck {slot}"), "cards": trio_deck(owner, slot) });
    json!({ "name": format!("{owner}'s trio"), "decks": [deck(0), deck(1), deck(2)] })
}

/// One server process. TS also gave each its own prefixed `Ids`, so a second process minted no id
/// twice; the Rust server's ids are its own random ones.
struct Process {
    app: Arc<App>,
    alice: String,
    bob: String,
    clock: Clock,
    /// How many matches had started in the store when this process booted: its own starts follow.
    started_before: usize,
}

/// Boots a process. With `from`, "restarts" over the database `from` left: `from`'s actors leave
/// memory first (a crashed process holds none), and the new app shares `from`'s store and clock.
async fn boot(from: Option<&Process>) -> Process {
    let app = match from {
        None => {
            let app = test_app().await;
            let data = fake(&app);
            let mut data = data.lock().await;
            for id in [ALICE, BOB] {
                data.seed_profile(json!({ "id": id, "userId": format!("user-{id}"), "status": "active", "rating": 1000 }));
            }
            drop(data);
            app
        }
        Some(previous) => {
            for match_id in previous.app.matches.live() {
                previous.app.matches.stop(&match_id).await;
            }
            test_app_with(TestAppOptions { db: Some(previous.app.db.clone()), ..TestAppOptions::default() }).await
        }
    };
    let alice = add_user(&app, &format!("user-{ALICE}"), "alice@example.test");
    let bob = add_user(&app, &format!("user-{BOB}"), "bob@example.test");
    let started_before = started(&app).await.len();
    let clock = from.map(|previous| previous.clock).unwrap_or(Clock { base_ms: 0, at: tokio::time::Instant::now() });
    Process { app, alice, bob, clock, started_before }
}

/// The matches started in the store, oldest first (every row past an `open` reservation).
async fn started(app: &App) -> Vec<Value> {
    table(app, |data| json!(data.tables.matches)).await.into_iter().filter(|row| row["status"] != "open").collect()
}

/// A started row as TS's directory recorded the `StartMatchInput` (seat order is the row's).
fn as_start(row: &Value) -> Value {
    json!({
        "matchId": row["id"],
        "seed": row["seed"],
        "catalogVersion": row["catalogVersion"],
        "ranked": row["ranked"],
        "seats": [
            { "profileId": row["players"][0], "player": "p1", "deck": row["decks"][0] },
            { "profileId": row["players"][1], "player": "p2", "deck": row["decks"][1] },
        ],
    })
}

impl Process {
    /// TS's `deps.matches.started`: the starts this process made.
    async fn started(&self) -> Vec<Value> {
        started(&self.app).await.iter().skip(self.started_before).map(as_start).collect()
    }
}

/// Starts the series, and reads the server's clock off the row it wrote.
async fn begin(process: &mut Process) -> Value {
    let at = tokio::time::Instant::now();
    let series = start_series(
        &process.app,
        from(json!({
            "seriesId": SERIES_ID,
            "firstMatchId": FIRST_MATCH,
            "sides": [
                { "profileId": ALICE, "trio": trio("alice") },
                { "profileId": BOB, "trio": trio("bob") },
            ],
            "seedBase": "seed-base",
            "catalogVersion": jackioh_cards::catalog_version(),
            "ranked": true,
        })),
    )
    .await
    .expect("the series starts");
    let series = to_json(&series);
    process.clock = Clock { base_ms: series["createdAt"].as_i64().expect("the row's creation"), at };
    series
}

async fn row(app: &Arc<App>) -> Value {
    let series = store!(app, t => t.series_get(SERIES_ID).await.expect("series.get"));
    to_json(&series.unwrap_or_else(|| panic!("the series is gone")))
}

async fn pick(process: &Process, token: &str, slot: usize) -> (u16, Value) {
    let (status, _headers, body) =
        call(&process.app, "POST", &format!("/api/series/{SERIES_ID}/pick"), Some(token), Some(json!({ "slot": slot }))).await;
    (status, body)
}

async fn pick_both(process: &Process, slots: [usize; 2]) -> Value {
    let (status, body) = pick(process, &process.alice, slots[0]).await;
    assert_eq!(status, 200, "{body}");
    let (status, answer) = pick(process, &process.bob, slots[1]).await;
    assert_eq!(status, 200, "{answer}");
    answer
}

/// The game in play ends as the actor would report it, from the seats in the match row — which a
/// restarted process reads from the store, not from its own memory. `Err` carries the writer's
/// refusal.
async fn finish_game(process: &Process, winner: &str) -> Result<(), String> {
    let series = row(&process.app).await;
    let match_id = series["nextMatchId"].as_str().expect("a next match id").to_string();
    let found = store!(process.app, t => t.matches_get(&match_id).await.expect("matches.get"));
    let found = to_json(&found.unwrap_or_else(|| panic!("{match_id} has no match row")));
    let seats = json!([
        { "profileId": found["players"][0], "player": "p1", "deck": found["decks"][0] },
        { "profileId": found["players"][1], "player": "p2", "deck": found["decks"][1] },
    ]);
    let seat = if winner == "draw" {
        "draw"
    } else if found["players"][1] == winner {
        "p2"
    } else {
        "p1"
    };
    record_result(
        &process.app,
        from(json!({
            "matchId": match_id,
            "seats": seats,
            "outcome": { "winner": seat, "reason": "hero-death" },
            "turns": 5,
            "at": process.clock.now(),
        })),
    )
    .await
    .map(|_| ())
    .map_err(|error| format!("{error:?}"))
}

/// Both picks written the way two requests write them — one compare-and-set each — and nothing
/// after: the state a process leaves when it dies between committing the second pick and starting
/// the match.
async fn write_picks_only(app: &Arc<App>, series: &Value, slots: [usize; 2], now: i64) -> Value {
    let series: SeriesRow = from(series.clone());
    let one = pick_deck(&series, PlayerId::P1, slots[0] as _, now, None).expect("p1's pick");
    assert!(store!(app, t => t.series_update(&one).await.expect("series.update")));
    let both = pick_deck(&one, PlayerId::P2, slots[1] as _, now, None).expect("p2's pick");
    assert!(store!(app, t => t.series_update(&both).await.expect("series.update")));
    let both = to_json(&both);
    assert_eq!(both["status"], "playing");
    both
}

async fn in_match(app: &Arc<App>) -> Vec<Value> {
    let mut flags = Vec::new();
    for id in [ALICE, BOB] {
        let profile = store!(app, t => t.profiles_get_by_id(id).await.expect("profiles.getById"));
        flags.push(profile.map_or(Value::Null, |profile| to_json(&profile)["inMatchId"].clone()));
    }
    flags
}

async fn sweep(app: &Arc<App>) -> Value {
    to_json(&sweep_series(app).await.expect("the sweep runs"))
}

fn nothing_swept() -> Value {
    json!({ "timedOut": [], "started": [], "abandoned": [] })
}

async fn active_for(app: &Arc<App>, profile: &str) -> bool {
    store!(app, t => t.series_active_for(profile).await.expect("series.activeFor")).is_some()
}

async fn rating_of(app: &Arc<App>, profile: &str) -> Option<f64> {
    let row = store!(app, t => t.profiles_get_by_id(profile).await.expect("profiles.getById"))?;
    to_json(&row)["rating"].as_f64()
}

mod r263_a_series_survives_a_restart {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r263_a_series_whose_picks_landed_but_whose_match_never_started_is_started_by_the_sweeper_after_the_grace() {
        let mut a = boot(None).await;
        let series = begin(&mut a).await;
        // The crash: both picks are in the row, and the process died before `matches.start`.
        write_picks_only(&a.app, &series, [0, 1], a.clock.now()).await;

        // Inside the grace the sweeper leaves it to the request that may be starting it right now.
        advance(grace_ms() - 1).await;
        assert_eq!(sweep(&a.app).await, nothing_swept());
        assert_eq!(a.started().await, Vec::<Value>::new());

        advance(1).await;
        assert_eq!(sweep(&a.app).await, json!({ "timedOut": [], "started": [SERIES_ID], "abandoned": [] }));
        assert_eq!(
            a.started().await,
            vec![json!({
                "matchId": FIRST_MATCH,
                "seed": "seed-base:1",
                "catalogVersion": jackioh_cards::catalog_version(),
                "ranked": true,
                "seats": [
                    { "profileId": ALICE, "player": "p1", "deck": trio_deck("alice", 0) },
                    { "profileId": BOB, "player": "p2", "deck": trio_deck("bob", 1) },
                ],
            })]
        );
        assert_eq!(in_match(&a.app).await, vec![json!(FIRST_MATCH), json!(FIRST_MATCH)]);

        // Running now: the next sweeps do nothing.
        advance(grace_ms()).await;
        assert_eq!(sweep(&a.app).await, nothing_swept());
        assert_eq!(a.started().await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r263_a_game_that_can_never_be_started_ends_the_series_abandoned_and_unrated_and_lets_both_players_go() {
        let mut a = boot(None).await;
        begin(&mut a).await;
        // Every start fails: the frozen decks no longer build a game (a catalog change, say) — here
        // the match row's insert is refused every time.
        fail_on(&a.app, "matches.create", "createGame refused the frozen decks").await;
        pick_both(&a, [1, 2]).await;
        assert!(active_for(&a.app, ALICE).await);

        // Inside the give-up window the sweeper keeps trying to start it.
        advance(give_up_ms() - 1).await;
        assert_eq!(sweep(&a.app).await, nothing_swept());
        assert_eq!(row(&a.app).await["status"], "playing");

        advance(1).await;
        assert_eq!(sweep(&a.app).await, json!({ "timedOut": [], "started": [], "abandoned": [SERIES_ID] }));
        let ended = row(&a.app).await;
        assert_eq!(ended["status"], "over");
        assert_eq!(ended["winner"], Value::Null);
        assert_eq!(ended["endReason"], "abandoned");
        assert_eq!(ended["ratingBefore"], Value::Null);
        // The game that never started is not on the record.
        assert_eq!(ended["games"], json!([]));
        assert!(!active_for(&a.app, ALICE).await);
        assert!(!active_for(&a.app, BOB).await);
        assert_eq!(rating_of(&a.app, ALICE).await, Some(1000.0));
    }

    #[tokio::test(start_paused = true)]
    async fn r263_never_gives_up_a_game_whose_match_did_start_however_long_it_has_run() {
        let mut a = boot(None).await;
        begin(&mut a).await;
        pick_both(&a, [0, 0]).await;
        // A restart: the match row is live in the store, its actor not yet rebuilt in memory.
        let b = boot(Some(&a)).await;
        advance(give_up_ms() * 2).await;
        assert_eq!(sweep(&b.app).await, nothing_swept());
        assert_eq!(row(&b.app).await["status"], "playing");
    }

    #[tokio::test(start_paused = true)]
    async fn r263_a_restart_between_both_picks_and_the_match_start_heals_itself_in_the_new_process() {
        let (logs, _recording) = Logs::record();
        let mut a = boot(None).await;
        begin(&mut a).await;
        // The first process accepts both picks and dies as it starts the match.
        fail_on(&a.app, "matches.create", "the process exited").await;
        let view = pick_both(&a, [2, 2]).await;
        // The picks were committed; the answer still names the game.
        assert_eq!(view["status"], "playing");
        assert_eq!(view["currentMatchId"], FIRST_MATCH);
        assert!(logs.has("series.start_failed"));
        assert!(store!(a.app, t => t.matches_get(FIRST_MATCH).await.expect("matches.get")).is_none());

        // The failure was the dead process's, not the database's.
        stop_failing(&a.app).await;
        let b = boot(Some(&a)).await;
        advance(grace_ms()).await;
        assert_eq!(sweep(&b.app).await, json!({ "timedOut": [], "started": [SERIES_ID], "abandoned": [] }));
        let first = b.started().await.remove(0);
        assert_eq!(first["matchId"], FIRST_MATCH);
        assert_eq!(first["seed"], "seed-base:1");
        assert_eq!(in_match(&b.app).await, vec![json!(FIRST_MATCH), json!(FIRST_MATCH)]);
    }

    #[tokio::test(start_paused = true)]
    async fn r263_a_new_process_over_the_same_store_carries_on_a_series_the_old_one_began() {
        let mut a = boot(None).await;
        begin(&mut a).await;
        pick_both(&a, [0, 0]).await;
        assert_eq!(a.started().await.len(), 1);

        // Restart: the match row is in the store, its actor is not in the new process's memory.
        let b = boot(Some(&a)).await;
        assert!(!b.app.matches.has(FIRST_MATCH));
        advance(grace_ms()).await;
        // The match exists, so the sweeper does not start it again: the registry folds its log back on
        // the first socket (§9.5).
        assert_eq!(sweep(&b.app).await, nothing_swept());
        assert_eq!(b.started().await, Vec::<Value>::new());

        // Its actor, rebuilt in the new process, reports the result.
        finish_game(&b, ALICE).await.expect("game 1's result");
        let after_game_1 = row(&b.app).await;
        assert_eq!(after_game_1["status"], "picking");
        // TS: an id the new process minted (`b-…`); here, a new id.
        assert_ne!(after_game_1["nextMatchId"], FIRST_MATCH);

        // The players pick through the new process and game 2 starts there.
        let game_2 = pick_both(&b, [1, 1]).await;
        assert_eq!(game_2["currentMatchId"], after_game_1["nextMatchId"]);
        let second = b.started().await.remove(0);
        assert_eq!(second["matchId"], after_game_1["nextMatchId"]);
        assert_eq!(second["seed"], "seed-base:2");
        assert_eq!(second["seats"][0]["profileId"], BOB);

        // Alice wins again: her last deck is picked for her (R332), and that pick is in the row, so a
        // third process finds it there and only waits for Bob's.
        finish_game(&b, ALICE).await.expect("game 2's result");
        let c = boot(Some(&b)).await;
        let (_, _, alice_sees) = call(&c.app, "GET", &format!("/api/series/{SERIES_ID}"), Some(c.alice.as_str()), None).await;
        assert_eq!(alice_sees["you"]["pick"], json!(2));
        assert_eq!(alice_sees["you"]["autoPick"], json!(true));
        assert_eq!(pick(&c, &c.bob, 0).await.0, 200);
        let third = c.started().await.remove(0);
        let leads: Vec<Value> = third["seats"].as_array().into_iter().flatten().map(|seat| seat["deck"][0].clone()).collect();
        assert_eq!(leads, vec![json!(trio_deck("alice", 2)[0]), json!(trio_deck("bob", 0)[0])]);
        finish_game(&c, ALICE).await.expect("game 3's result");
        let over = row(&c.app).await;
        assert_eq!(over["status"], "over");
        assert_eq!(over["winner"], "p1");
        assert_eq!(over["endReason"], "decided");
    }

    #[tokio::test(start_paused = true)]
    async fn r331_a_pick_made_before_a_restart_is_kept_by_the_next_process_and_still_sealed_and_hidden() {
        let mut a = boot(None).await;
        begin(&mut a).await;
        assert_eq!(pick(&a, &a.alice, 2).await.0, 200);

        let b = boot(Some(&a)).await;
        let (_, _, bob_sees) = call(&b.app, "GET", &format!("/api/series/{SERIES_ID}"), Some(b.bob.as_str()), None).await;
        assert_eq!(bob_sees["opponent"]["picked"], json!(true));
        assert!(!bob_sees.to_string().contains("alice"), "{bob_sees}");
        // Sealed across the restart too: Alice cannot change it in the new process.
        assert_eq!(pick(&b, &b.alice, 1).await.0, 409);
        assert_eq!(pick(&b, &b.bob, 1).await.0, 200);
        let first = b.started().await.remove(0);
        let leads: Vec<Value> = first["seats"].as_array().into_iter().flatten().map(|seat| seat["deck"][0].clone()).collect();
        assert_eq!(leads, vec![json!(trio_deck("alice", 2)[0]), json!(trio_deck("bob", 1)[0])]);
    }

    #[tokio::test(start_paused = true)]
    async fn r263_a_store_failure_inside_the_results_transaction_rolls_back_the_series_advance_with_the_result() {
        let mut a = boot(None).await;
        begin(&mut a).await;
        pick_both(&a, [0, 0]).await;
        let playing = row(&a.app).await;

        fail_on(&a.app, "series.update", "the connection dropped").await;
        let failed = finish_game(&a, BOB).await.expect_err("the series write fails the result");
        assert!(failed.contains("connection dropped"), "{failed}");

        // Nothing of the ending landed: no result row, the series still playing that game, the match
        // still live and both players still in it.
        assert_eq!(table(&a.app, |data| json!(data.tables.results)).await, Vec::<Value>::new());
        assert_eq!(row(&a.app).await, playing);
        let game = store!(a.app, t => t.matches_get(FIRST_MATCH).await.expect("matches.get")).expect("game 1's match");
        assert_eq!(to_json(&game)["status"], "live");
        assert_eq!(in_match(&a.app).await, vec![json!(FIRST_MATCH), json!(FIRST_MATCH)]);

        stop_failing(&a.app).await;
        finish_game(&a, BOB).await.expect("the result lands");
        assert_eq!(table(&a.app, |data| json!(data.tables.results)).await.len(), 1);
        let after = row(&a.app).await;
        assert_eq!(after["status"], "picking");
        assert_eq!(after["version"], json!(playing["version"].as_i64().expect("a version") + 1));
    }

    #[tokio::test(start_paused = true)]
    async fn r263_a_failure_writing_the_series_rating_move_rolls_back_the_deciding_games_result_too() {
        let mut a = boot(None).await;
        begin(&mut a).await;
        pick_both(&a, [0, 0]).await;
        finish_game(&a, ALICE).await.expect("game 1's result");
        pick_both(&a, [1, 1]).await;
        finish_game(&a, ALICE).await.expect("game 2's result");
        // Alice's last deck is picked for her (R332); Bob's pick starts the deciding game.
        assert_eq!(pick(&a, &a.bob, 0).await.0, 200);
        let deciding = row(&a.app).await;

        fail_on(&a.app, "profiles.setGlicko", "the connection dropped").await;
        let failed = finish_game(&a, ALICE).await.expect_err("the rating write fails the result");
        assert!(failed.contains("connection dropped"), "{failed}");
        assert_eq!(table(&a.app, |data| json!(data.tables.results)).await.len(), 2);
        assert_eq!(row(&a.app).await, deciding);
        assert_eq!(rating_of(&a.app, ALICE).await, Some(1000.0));

        stop_failing(&a.app).await;
        finish_game(&a, ALICE).await.expect("the deciding result lands");
        let fresh = Glicko {
            rating: 1000.0,
            deviation: RATING_DEVIATION_START as f64,
            volatility: RATING_VOLATILITY_START as f64,
        };
        let expected = rate_game(&fresh, &fresh, 1.0);
        assert_eq!(table(&a.app, |data| json!(data.tables.results)).await.len(), 3);
        assert_eq!(rating_of(&a.app, ALICE).await, Some(expected.a.rating));
        let over = row(&a.app).await;
        assert_eq!(over["status"], "over");
        assert_eq!(over["ratingAfter"], json!([expected.a.rating, expected.b.rating]));
        // R611: and the series is recorded once, as one rated game.
        let rated: Vec<Value> = table(&a.app, |data| json!(data.tables.rated_games))
            .await
            .iter()
            .map(|game| json!([game["id"], game["kind"]]))
            .collect();
        assert_eq!(rated, vec![json!([SERIES_ID, "series"])]);
    }
}

mod r263_every_write_is_a_compare_and_set {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r263_two_starts_racing_for_one_game_make_one_match_the_loser_finds_the_winners_row_and_stops() {
        let mut a = boot(None).await;
        let series = begin(&mut a).await;
        let now = a.clock.now();
        let playing = write_picks_only(&a.app, &series, [0, 0], now).await;

        // Another process won the start a moment earlier: its row is in the store, so this start's
        // insert of the same id is refused (TS: a directory that wrote the row, then threw).
        let winner = from(json!({
            "id": FIRST_MATCH,
            "seed": "seed-base:1",
            "players": [ALICE, BOB],
            "decks": [trio_deck("alice", 0), trio_deck("bob", 0)],
            "catalogVersion": jackioh_cards::catalog_version(),
            "ranked": true,
            "status": "live",
            "createdAt": now,
            "finishedAt": null,
            "clocks": {
                "turnDeadline": null,
                "promptDeadline": null,
                "graceDeadline": { "p1": null, "p2": null },
                "ceilingAt": now + MATCH_CEILING_MINUTES as i64 * 60_000,
            },
        }));
        store!(a.app, t => t.matches_create(&winner).await.expect("matches.create"));
        let playing: SeriesRow = from(playing);
        ensure_series_game(&a.app, &playing).await.expect("the loser stops on the winner's row");
        let game = store!(a.app, t => t.matches_get(FIRST_MATCH).await.expect("matches.get")).expect("the winner's row");
        assert_eq!(to_json(&game)["status"], "live");

        // A start that fails for any other reason is not swallowed.
        let mut broken = boot(None).await;
        let other = begin(&mut broken).await;
        let playing_too: SeriesRow = from(write_picks_only(&broken.app, &other, [0, 0], broken.clock.now()).await);
        fail_on(&broken.app, "matches.create", "the engine could not be loaded").await;
        let refused = ensure_series_game(&broken.app, &playing_too).await.expect_err("an engine failure stands");
        assert!(format!("{refused:?}").contains("engine"), "{refused:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn r263_a_started_match_whose_in_match_flags_were_lost_to_a_crash_gets_them_back_from_the_sweeper() {
        let mut a = boot(None).await;
        begin(&mut a).await;
        pick_both(&a, [0, 0]).await;
        for id in [ALICE, BOB] {
            store!(a.app, t => t.profiles_set_in_match(id, None).await.expect("profiles.setInMatch"));
        }

        let b = boot(Some(&a)).await;
        advance(grace_ms()).await;
        assert_eq!(sweep(&b.app).await, nothing_swept());
        assert_eq!(in_match(&b.app).await, vec![json!(FIRST_MATCH), json!(FIRST_MATCH)]);
    }

    #[tokio::test(start_paused = true)]
    async fn r263_a_series_game_the_store_says_has_ended_with_no_result_is_reported_never_replayed() {
        let (logs, _recording) = Logs::record();
        let mut a = boot(None).await;
        begin(&mut a).await;
        pick_both(&a, [0, 0]).await;
        // The actor's own `finish` landed but its result write did not.
        let now = a.clock.now();
        store!(a.app, t => t.matches_finish(FIRST_MATCH, now as _).await.expect("matches.finish"));

        let b = boot(Some(&a)).await;
        advance(grace_ms()).await;
        assert_eq!(sweep(&b.app).await, nothing_swept());
        assert_eq!(b.started().await, Vec::<Value>::new());
        assert!(logs.has("series.game_unrecorded"));
        // The rules still refuse to end a game twice.
        let series: SeriesRow = from(row(&b.app).await);
        let once = game_ended(&series, Winner::P1, GameOverReason::Concede, 0, "x").expect("the first ending");
        assert!(game_ended(&once, Winner::P1, GameOverReason::Concede, 0, "y").is_err());
    }
}
