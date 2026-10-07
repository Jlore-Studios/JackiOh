//! SPEC §9.11, R376–R378: the live half of the card statistics. A finished match is filed once,
//! after its result, under its mode, the build's patch and two human pilots, and nothing about it
//! can cost the result. The real engine plays the games, through the real registry, actor and
//! results writer, driven over the match protocol by two in-memory sockets.
//!
//! Port of `apps/server/test/api/game-records.test.ts`. What moved, and why (SURFACE §11.3: the
//! server links the real engine and has no `EnginePort` or `GameRecorder` port):
//!
//!  - TS ended its games with the scripted `test-lethal`; here the losing seat concedes once both
//!    mulligans are in, which ends a real game in one action and needs no scripted card. The record's
//!    `reason` is therefore `concede`, and its opening hands are the real deal's.
//!  - The recorder is no longer optional: every live record is filed under the compiled-in patch
//!    (`jackioh_cards::catalog_version()`, R388's newest), so TS's "nor without a recorder" half and
//!    its composition-root test (which proved the runtime bound the recorder) have nothing left to
//!    prove. `loadCurrentPatch`'s file reading went with them; its "the list's order is the order of
//!    versions" half is the patch-list test below.
//!  - Log lines are read off the `tracing` JSON the server writes (`log_lines`), not a recording
//!    logger.

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::{mpsc, MutexGuard};

use jackioh_engine::PlayerId;
use jackioh_server::actor::ws_server::{Socket, SocketFrame};
use jackioh_server::api::game_records::record_live_game;
use jackioh_server::api::results::record_result;
use jackioh_server::app::App;
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchSeat, StartMatchInput, StoreError};

use crate::support::deps::test_app;

const MATCH_ID: &str = "match-records";
const P1: &str = "profile-1";
const P2: &str = "profile-2";

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

/// A port value built from TS's own object literal, so the test depends on the JSON shape only.
fn from<T: DeserializeOwned>(value: Value) -> T {
    match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(error) => panic!("{error}: {value}"),
    }
}

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// One store call in its own transaction, as every TS `deps.store.<x>.<y>(…)` call was.
macro_rules! q {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut tx = $app.db.begin(None).await.expect("begin");
        let out = tx.$method($($arg),*).await.expect(stringify!($method));
        tx.commit().await.expect("commit");
        out
    }};
}

/// The fake store's tables, for what TS read off `deps.store.tables` and `seedProfile`.
async fn fake(app: &App) -> MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the server's unit tests run on the fake store"),
    }
}

/// The JSON lines the server's `tracing` writes while the guard is held (TS's recording logger).
#[derive(Clone, Default)]
struct LogLines(Arc<StdMutex<Vec<u8>>>);

impl std::io::Write for LogLines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("log buffer").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogLines {
    type Writer = LogLines;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl LogLines {
    /// How many lines name `event` (TS: `log.entries.filter((entry) => entry.event === name)`).
    fn count(&self, event: &str) -> usize {
        let text = String::from_utf8_lossy(&self.0.lock().expect("log buffer")).into_owned();
        let needle = format!("\"{event}\"");
        text.lines().filter(|line| line.contains(&needle)).count()
    }
}

/// Captures this thread's log lines; every task of a `#[tokio::test]` runs on it.
fn log_lines() -> (LogLines, tracing::subscriber::DefaultGuard) {
    let lines = LogLines::default();
    let subscriber = tracing_subscriber::fmt().json().with_writer(lines.clone()).finish();
    let guard = crate::support::deps::set_log_default(subscriber);
    (lines, guard)
}

/// Two legal decks of real cards: disjoint slices of the catalog's playable ids.
fn real_decks() -> (Vec<String>, Vec<String>) {
    jackioh_cards::register_all();
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    let pool: Vec<String> =
        jackioh_cards::CATALOG.values().filter(|def| !def.token).map(|def| def.id.clone()).collect();
    (pool[..size].to_vec(), pool[size..size * 2].to_vec())
}

// ---------------------------------------------------------------------------------------------
// A client on the match protocol
// ---------------------------------------------------------------------------------------------

/// One seat's end of an in-memory socket: what TS's `actor.submit` reached directly.
struct Client {
    seat: &'static str,
    socket: Socket,
    frames: mpsc::UnboundedReceiver<SocketFrame>,
    view: Option<Value>,
    sent: u32,
}

impl Client {
    fn new(seat: &'static str) -> Client {
        let (socket, frames) = Socket::channel();
        Client { seat, socket, frames, view: None, sent: 0 }
    }

    fn absorb(&mut self, text: &str) -> Value {
        let frame: Value = serde_json::from_str(text).expect("every frame is JSON");
        if frame["type"] == "view" {
            self.view = Some(frame["view"].clone());
        }
        frame
    }

    /// The next frame the server sent this seat.
    async fn frame(&mut self) -> Value {
        loop {
            match self.frames.recv().await {
                Some(SocketFrame::Text(text)) => return self.absorb(&text),
                Some(SocketFrame::Close { .. }) => {}
                None => panic!("{}'s socket is gone", self.seat),
            }
        }
    }

    /// This seat's latest view, waiting for the first one.
    async fn current_view(&mut self) -> Value {
        while let Ok(out) = self.frames.try_recv() {
            if let SocketFrame::Text(text) = out {
                self.absorb(&text);
            }
        }
        while self.view.is_none() {
            self.frame().await;
        }
        self.view.clone().unwrap_or(Value::Null)
    }

    /// Sends one action and waits for its `ack` (or its `error`).
    async fn act(&mut self, mut action: Value) -> Value {
        self.sent += 1;
        let nonce = format!("{}-n{}", self.seat, self.sent);
        action["nonce"] = json!(nonce);
        self.socket.receive(json!({ "type": "action", "action": action }).to_string());
        loop {
            let frame = self.frame().await;
            let kind = frame["type"].as_str().unwrap_or("");
            if (kind == "ack" || kind == "error") && frame["nonce"] == json!(nonce) {
                return frame;
            }
        }
    }

    /// R265: keep the whole opening hand.
    async fn keep_hand(&mut self) {
        let view = self.current_view().await;
        let keep: Vec<Value> = view["you"]["hand"]
            .as_array()
            .map(|hand| hand.iter().map(|card| card["instanceId"].clone()).collect())
            .unwrap_or_default();
        let answer = self.act(json!({ "type": "mulligan", "keep": keep })).await;
        assert_eq!(answer["type"], "ack", "{}'s mulligan: {answer}", self.seat);
    }
}

struct Harness {
    app: Arc<App>,
    decks: (Vec<String>, Vec<String>),
    p1: Client,
    p2: Client,
}

/// The two queue tickets a match in `mode` was paired from (§9.5): what tells the record its mode.
async fn paired_tickets(app: &App, mode: &str) {
    for profile_id in [P1, P2] {
        q!(
            app,
            tickets_insert(&from(json!({
                "id": format!("ticket-{profile_id}"),
                "profileId": profile_id,
                "rating": 1000,
                "mode": mode,
                "deck": [],
                "trio": null,
                "catalogVersion": app.catalog.version,
                "enqueuedAt": 0,
                "status": "matched",
                "matchId": MATCH_ID,
            })))
        );
    }
}

/// A live match on the real registry and the real results writer, paired from two `mode` tickets —
/// or, with `None`, from nothing a mode can be read off — with both seats' sockets attached.
async fn live_match(mode: Option<&str>) -> Harness {
    let app = test_app().await;
    {
        let mut data = fake(&app).await;
        data.seed_profile(json!({ "id": P1, "inMatchId": MATCH_ID }));
        data.seed_profile(json!({ "id": P2, "inMatchId": MATCH_ID }));
    }
    if let Some(mode) = mode {
        paired_tickets(&app, mode).await;
    }
    let decks = real_decks();
    app.matches
        .start(
            &app,
            StartMatchInput {
                match_id: MATCH_ID.to_string(),
                seed: "seed-records".to_string(),
                catalog_version: app.catalog.version.clone(),
                ranked: false,
                seats: (
                    MatchSeat { profile_id: P1.to_string(), player: PlayerId::P1, deck: decks.0.clone(), portrait: None },
                    MatchSeat { profile_id: P2.to_string(), player: PlayerId::P2, deck: decks.1.clone(), portrait: None },
                ),
                mode: None,
                stake: None,
            },
        )
        .await
        .expect("the match starts");
    let p1 = Client::new("p1");
    let p2 = Client::new("p2");
    app.matches.attach(&app, MATCH_ID, P1, p1.socket.clone()).await.expect("p1 attaches");
    app.matches.attach(&app, MATCH_ID, P2, p2.socket.clone()).await.expect("p2 attaches");
    Harness { app, decks, p1, p2 }
}

/// Lets the actor finish what the last action started: the result, the record and the finish,
/// which land after the action's `ack`.
async fn wait_finished(app: &App, match_id: &str) {
    for _ in 0..500 {
        let row = q!(app, matches_get(match_id));
        if row.as_ref().map(to_json).is_some_and(|row| row["status"] == "finished") {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("match {match_id} never finished");
}

/// Both seats keep their hands (R265), then p2 concedes: p1 wins.
async fn play_to_concession(h: &mut Harness) {
    h.p1.keep_hand().await;
    h.p2.keep_hand().await;
    let conceded = h.p2.act(json!({ "type": "concede" })).await;
    assert_eq!(conceded["type"], "ack", "the concession: {conceded}");
    wait_finished(&h.app, MATCH_ID).await;
}

async fn results_for(app: &App, match_id: &str) -> usize {
    usize::from(q!(app, results_get_by_match(match_id)).is_some())
}

async fn game_records(app: &App) -> Value {
    to_json(&fake(app).await.tables.game_records)
}

// ---------------------------------------------------------------------------------------------
// live game records (§9.11)
// ---------------------------------------------------------------------------------------------

mod live_game_records {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r376_files_a_finished_match_once_its_result_is_in_its_mode_the_patch_two_human_pilots_and_the_game() {
        let (logs, _guard) = log_lines();
        let mut h = live_match(Some("random")).await;
        play_to_concession(&mut h).await;

        assert_eq!(results_for(&h.app, MATCH_ID).await, 1);
        let records = game_records(&h.app).await;
        let records = records.as_array().expect("a list");
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record["id"], MATCH_ID);
        assert_eq!(record["source"], "live");
        assert_eq!(record["mode"], "random");
        assert_eq!(record["patch"], jackioh_cards::catalog_version());
        assert_eq!(record["pilots"], json!({ "p1": "human", "p2": "human" }));
        let game = &record["game"];
        assert_eq!(game["winner"], "p1");
        assert_eq!(game["reason"], "concede");
        assert!(game["first"] == "p1" || game["first"] == "p2", "first: {}", game["first"]);
        assert!(game["turns"].as_i64().is_some(), "turns: {}", game["turns"]);
        assert_eq!(game["seats"]["p1"]["deck"], json!(h.decks.0));
        assert_eq!(game["seats"]["p2"]["deck"], json!(h.decks.1));
        // Nobody played a card before the concession.
        assert_eq!(game["seats"]["p1"]["played"], json!([]));
        assert_eq!(game["seats"]["p2"]["played"], json!([]));
        assert_eq!(logs.count("game.recorded"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r376_files_a_game_once_however_many_times_its_result_is_written() {
        let mut h = live_match(Some("bo1")).await;
        play_to_concession(&mut h).await;
        let again = record_result(
            &h.app,
            from(json!({
                "matchId": MATCH_ID,
                "seats": [
                    { "profileId": P1, "player": "p1", "deck": h.decks.0 },
                    { "profileId": P2, "player": "p2", "deck": h.decks.1 },
                ],
                "outcome": { "winner": "p1", "reason": "concede" },
                "turns": 3,
                "at": 0,
            })),
        )
        .await;
        assert!(again.is_ok(), "a second write is a no-op, not an error");

        assert_eq!(results_for(&h.app, MATCH_ID).await, 1);
        let records = game_records(&h.app).await;
        assert_eq!(records.as_array().map(Vec::len), Some(1));
        assert_eq!(records[0]["mode"], "bo1");
        assert!(record_live_game(&h.app, MATCH_ID).await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn r376_never_costs_a_result_a_failed_record_is_logged_and_the_result_stands() {
        let (logs, _guard) = log_lines();
        let mut h = live_match(Some("bo1")).await;
        fake(&h.app).await.on_call = Some(Arc::new(|method: &str| {
            if method == "gameRecords.insert" {
                return Err(StoreError::Other("the records table is gone".to_string()));
            }
            Ok(())
        }));
        play_to_concession(&mut h).await;

        assert_eq!(results_for(&h.app, MATCH_ID).await, 1);
        let row = q!(h.app, matches_get(MATCH_ID)).map(|row| to_json(&row));
        assert_eq!(row.map(|row| row["status"].clone()), Some(json!("finished")));
        assert_eq!(game_records(&h.app).await, json!([]));
        assert_eq!(logs.count("game.record.failed"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r376_files_nothing_for_a_match_no_ticket_room_or_series_made() {
        let (logs, _guard) = log_lines();
        let mut h = live_match(None).await;
        play_to_concession(&mut h).await;

        assert_eq!(results_for(&h.app, MATCH_ID).await, 1);
        assert_eq!(game_records(&h.app).await, json!([]));
        assert_eq!(logs.count("game.record.skipped"), 1);
        assert!(record_live_game(&h.app, "no-such-match").await.is_none());
        assert_eq!(logs.count("game.record.skipped"), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn r376_alerts_on_a_log_that_does_not_reach_the_result_it_ended_with() {
        let (logs, _guard) = log_lines();
        let mut h = live_match(Some("bo1")).await;
        play_to_concession(&mut h).await;
        {
            let mut data = fake(&h.app).await;
            data.tables.game_records.clear();
            // The last action, the one that ended the game, goes missing from the log.
            data.tables.match_actions.pop();
        }

        assert!(record_live_game(&h.app, MATCH_ID).await.is_none());
        assert_eq!(logs.count("game.record.unfinished"), 1);
    }

    #[tokio::test]
    async fn r376_files_a_live_game_under_the_newest_patch_of_r388s_list() {
        let list: Vec<Value> =
            serde_json::from_str(include_str!("../../../cards/patches/patches.json")).expect("patches.json");
        // The list's order is the order of versions, never a comparison of the strings (R388): the
        // newest patch is the last entry, whatever it says.
        let newest = list.last().and_then(|entry| entry["version"].as_str()).expect("a newest version");
        assert_eq!(jackioh_cards::catalog_version(), newest);
    }
}
