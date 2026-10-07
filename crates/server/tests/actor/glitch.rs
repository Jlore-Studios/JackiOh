//! The Glitch Easter egg on the server (issue #170, SPEC §7, R676–R679): what the server does with
//! the swap, boards and void outcomes the engine announces. The scripted cards of
//! `support::engine` (`test/fakes/engine.ts`'s) reach each on demand — `test-glitch-swap` and
//! `test-glitch-void` — under the real registry, actor and results writer, over the fake store.
//! The reset outcome (R676) needs nothing of the server: the engine rebuilds its own state, and
//! `(seed, log)` still folds to it.
//!
//! The trust model these prove: only the engine's state moves an account to the other seat or voids
//! a match. A client frame reaches either only as a legal `play` of the card.
//!
//! Port of `apps/server/test/match/glitch.test.ts`. What the Rust server changed underneath it
//! (SURFACE §11.3): the engine is the real one with the test cards installed (no scripted port to
//! wrap, so "the rebuild folds with the same boards" is read off the rebuilt state), it opens on
//! both mulligans (answered keeping everything before the sockets attach, which is where TS's
//! scripted game began), it shuffles (the seed is searched so the scripted cards are dealt into
//! p1's opening hand, where `fakeDeck` put them), and its instance ids are its own (a card is
//! named by its definition, as the TS comments name it, not by `p1-h0`).

use std::any::Any;
use std::sync::Arc;

use serde_json::{json, Value};

use jackioh_engine::wire::PlayerId;
use jackioh_engine::{CreateGameArgs, LastBoardEntry};
use jackioh_server::actor::match_actor::MatchActor;
use jackioh_server::api::results::void_match;
use jackioh_server::api::series::start_series;
use jackioh_server::app::App;
use jackioh_server::config::{GLITCH_BOARDS_SAMPLED, MATCH_VOIDED_CLOSE_CODE};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;

use crate::support::deps::{add_user, call, test_app};
use crate::support::engine::{fake_deck, install_test_cards};
use crate::support::socket::{create_fake_socket, FakeSocket};

const MATCH_ID: &str = "match-glitch";
const P1: &str = "profile-1";
const P2: &str = "profile-2";

/// How many seeds `seed_dealing` tries before it gives up.
const SEED_SEARCH: usize = 5_000;

/// When the boards below were stored; nothing reads it back.
const BOARD_AT: i64 = 1_700_000_000_000;

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

// ---------------------------------------------------------------------------------------------
// The log recorder (TS `createRecordingLogger`)
// ---------------------------------------------------------------------------------------------

/// Every `tracing` line the server writes on this thread while the guard lives, as JSON. On the
/// current-thread runtime `#[tokio::test]` builds, the actor's tasks run on this thread too.
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

    fn entries(&self) -> Vec<Value> {
        let bytes = self.0.lock().expect("the log buffer").clone();
        String::from_utf8_lossy(&bytes).lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
    }

    /// The lines that name `event` (SURFACE §11.3 keeps TS's event names), whichever field the
    /// server wrote the name in.
    fn of(&self, event: &str) -> Vec<Value> {
        self.entries().into_iter().filter(|entry| event_of(entry) == Some(event)).collect()
    }
}

fn event_of(entry: &Value) -> Option<&str> {
    ["event", "message"].iter().find_map(|key| {
        entry.get(*key).or_else(|| entry.get("fields").and_then(|fields| fields.get(*key))).and_then(Value::as_str)
    })
}

// ---------------------------------------------------------------------------------------------
// The world
// ---------------------------------------------------------------------------------------------

/// A live, ranked match on the real registry: account `P1` began in p1 with `p1_deck`'s scripted
/// cards dealt into its hand, `P2` in p2. `a` is P1's socket and `b` is P2's, attached the way
/// `ws_server.rs` attaches them.
struct World {
    app: Arc<App>,
    a: FakeSocket,
    b: FakeSocket,
    /// The rows the opening's two mulligans wrote, ahead of everything a test sends.
    opening: usize,
    /// Whatever `install_test_cards` hands back (the testkit override), held for the test.
    _engine: Box<dyn Any>,
}

#[derive(Default)]
struct WorldOptions {
    /// The scripted cards `fakeDeck` puts in p1's opening hand; TS's default deck when `None`.
    p1_deck: Option<Vec<&'static str>>,
    p2_deck: Option<Vec<&'static str>>,
    /// TS's `before`: last boards stored ahead of the start, `(profile, kind, board)`.
    boards: Vec<(&'static str, &'static str, Vec<Value>)>,
}

/// Whether `def_id` is in the seat's own hand, as its view lists it.
fn hand_holds(view: &Value, def_id: &str) -> bool {
    view["you"]["hand"].as_array().is_some_and(|cards| cards.iter().any(|card| card["defId"] == def_id))
}

/// The first seed `<prefix>-<k>` whose opening deal puts each `(seat, card)` in that seat's hand:
/// TS's scripted engine drew `fakeDeck`'s first cards, the real one shuffles them (§2.1).
fn seed_dealing(prefix: &str, decks: &(Vec<String>, Vec<String>), wanted: &[(PlayerId, &str)]) -> String {
    let (first, second) = decks;
    for k in 0..SEED_SEARCH {
        let seed = format!("{prefix}-{k}");
        let args: CreateGameArgs = from(json!({ "seed": seed, "decks": [first, second] }));
        let state = jackioh_engine::begin_game(&jackioh_engine::create_game(&args)).state;
        if wanted.iter().all(|(seat, def_id)| hand_holds(&to_json(&jackioh_engine::view_for(&state, *seat)), def_id)) {
            return seed;
        }
    }
    panic!("no seed under {prefix} deals {wanted:?}");
}

/// Both mulligans answered keeping the whole hand, in seat order: the real engine opens on them
/// (R265), TS's scripted one opened on turn 1. Answers how many rows they wrote.
async fn open_turn_one(actor: &MatchActor) -> usize {
    let owed = actor.snapshot().mulligan_owed;
    for seat in &owed {
        let keep = hand_ids(&to_json(&actor.view_for(*seat)));
        let reply = to_json(&actor.submit(*seat, format!("keep-{seat}"), from(json!({ "type": "mulligan", "keep": keep }))).await);
        assert_eq!(reply["type"], "ack", "the opening mulligan was refused: {reply}");
    }
    owed.len()
}

fn hand_ids(view: &Value) -> Vec<String> {
    view["you"]["hand"]
        .as_array()
        .map(|cards| cards.iter().filter_map(|card| card["instanceId"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// The instance of `def_id` in the seat's own hand (TS's `p1-h<slot>`).
fn hand_card(view: &Value, def_id: &str) -> String {
    view["you"]["hand"]
        .as_array()
        .and_then(|cards| cards.iter().find(|card| card["defId"] == def_id))
        .and_then(|card| card["instanceId"].as_str())
        .unwrap_or_else(|| panic!("{def_id} is not in this hand: {}", view["you"]["hand"]))
        .to_string()
}

async fn world(options: WorldOptions) -> World {
    let engine: Box<dyn Any> = Box::new(install_test_cards());
    let app = test_app().await;
    {
        let data = fake(&app);
        let mut data = data.lock().await;
        data.seed_profile(json!({ "id": P1, "rating": 1000, "inMatchId": MATCH_ID }));
        data.seed_profile(json!({ "id": P2, "rating": 1000, "inMatchId": MATCH_ID }));
    }
    for (profile, kind, board) in &options.boards {
        let board: Vec<LastBoardEntry> = from(json!(board));
        store!(app, t => t.last_boards_put(profile, from(json!(kind)), &board, BOARD_AT as _).await.expect("lastBoards.put"));
    }

    let p1_extras = options.p1_deck.unwrap_or_else(|| vec!["test-glitch-swap", "test-lethal"]);
    let p2_extras = options.p2_deck.unwrap_or_default();
    let decks = (fake_deck(&p1_extras), fake_deck(&p2_extras));
    let wanted: Vec<(PlayerId, &str)> = p1_extras.iter().map(|card| (PlayerId::P1, *card)).collect();
    let seed = seed_dealing("seed-glitch", &decks, &wanted);
    app.matches
        .start(
            &app,
            from(json!({
                "matchId": MATCH_ID,
                "seed": seed,
                "catalogVersion": jackioh_cards::catalog_version(),
                "ranked": true,
                "seats": [
                    { "profileId": P1, "player": "p1", "deck": decks.0 },
                    { "profileId": P2, "player": "p2", "deck": decks.1 },
                ],
            })),
        )
        .await
        .expect("the registry starts the match");

    let actor = app.matches.actor_for(&app, MATCH_ID).await.expect("the match's actor");
    let opening = open_turn_one(&actor).await;
    let a = create_fake_socket();
    let b = create_fake_socket();
    app.matches.attach(&app, MATCH_ID, P1, a.socket()).await.expect("P1 attaches");
    app.matches.attach(&app, MATCH_ID, P2, b.socket()).await.expect("P2 attaches");
    actor.idle().await;
    World { app, a, b, opening, _engine: engine }
}

/// One client frame, and the actor's queue drained behind it.
async fn frame(w: &World, socket: &FakeSocket, nonce: &str, body: Value) -> Option<Value> {
    let actor = w.app.matches.actor_for(&w.app, MATCH_ID).await.expect("the match's actor");
    socket.clear();
    let mut action = body;
    action["nonce"] = json!(nonce);
    socket.receive_json(json!({ "type": "action", "action": action }));
    actor.idle().await;
    let mut replies = socket.of_type("ack");
    replies.extend(socket.of_type("error"));
    replies.pop()
}

fn last_view(socket: &FakeSocket) -> Value {
    socket.of_type("view").last().map(|frame| frame["view"].clone()).expect("no view was sent")
}

/// The seat every logged action of the game was stamped with, after the opening.
async fn stamped(w: &World) -> Vec<Value> {
    let rows = table(&w.app, |data| json!(data.tables.match_actions)).await;
    rows.iter().skip(w.opening).map(|row| row["action"]["playerId"].clone()).collect()
}

async fn idle(w: &World) {
    w.app.matches.actor_for(&w.app, MATCH_ID).await.expect("the match's actor").idle().await;
}

async fn match_row(app: &Arc<App>, match_id: &str) -> Option<Value> {
    store!(app, t => t.matches_get(match_id).await.expect("matches.get")).map(|row| to_json(&row))
}

mod glitchs_swap {
    //! Glitch's swap (R677).
    use super::*;

    #[tokio::test]
    async fn r677_routes_each_accounts_socket_to_the_seat_it_now_plays_views_legal_actions_and_accepted_actions() {
        let w = world(WorldOptions::default()).await;
        assert_eq!(last_view(&w.a)["viewer"], "p1");

        // P1 plays Glitch as p1: the seats swap.
        let glitch = hand_card(&last_view(&w.a), "test-glitch-swap");
        let played = frame(&w, &w.a, "n1", json!({ "type": "play", "instanceId": glitch })).await.expect("a reply");
        assert_eq!(played["type"], "ack");
        assert_eq!(last_view(&w.a)["viewer"], "p2");
        assert_eq!(last_view(&w.b)["viewer"], "p1");
        // The legal actions travel with the view of the seat played now: P2 (p1, the active seat) may
        // play p1's hand, P1 (p2) only concede.
        let legal_of = |socket: &FakeSocket| -> Value {
            socket.of_type("view").last().map(|frame| frame["legal"].clone()).unwrap_or_else(|| json!([]))
        };
        let lethal = hand_card(&last_view(&w.b), "test-lethal");
        assert!(
            legal_of(&w.b)
                .as_array()
                .expect("a legal list")
                .iter()
                .any(|action| action["type"] == "play" && action["instanceId"] == lethal.as_str()),
            "P2 may play p1's hand"
        );
        assert_eq!(legal_of(&w.a), json!([{ "type": "concede" }]));

        // An action from P1 is stamped p2 now — it is not p2's turn — and the same from P2 is p1's.
        let refused = frame(&w, &w.a, "n2", json!({ "type": "endTurn" })).await.expect("a reply");
        assert_eq!(refused["type"], "error");
        assert_eq!(refused["code"], "illegal_action");
        assert_eq!(stamped(&w).await, vec![json!("p1")]);
        let ended = frame(&w, &w.b, "n3", json!({ "type": "endTurn" })).await.expect("a reply");
        assert_eq!(ended["type"], "ack");
        assert_eq!(stamped(&w).await, vec![json!("p1"), json!("p1")]);
        assert_eq!(last_view(&w.a)["active"], "p2");
    }

    #[tokio::test]
    async fn r677_a_client_frame_cannot_move_an_account_a_player_id_on_the_wire_is_discarded_before_and_after_a_swap() {
        let w = world(WorldOptions::default()).await;
        w.a.receive_json(json!({
            "type": "action",
            "action": { "type": "endTurn", "playerId": "p2", "nonce": "x1" },
            "playerId": "p2",
        }));
        idle(&w).await;
        assert_eq!(stamped(&w).await, vec![json!("p1")]);
        assert_eq!(last_view(&w.a)["viewer"], "p1");

        // p2's turn: P2 plays nothing that swaps, and P1's smuggled seat still does not move it.
        frame(&w, &w.b, "x2", json!({ "type": "endTurn" })).await;
        let glitch = hand_card(&last_view(&w.a), "test-glitch-swap");
        let played = frame(&w, &w.a, "x3", json!({ "type": "play", "instanceId": glitch })).await.expect("a reply");
        assert_eq!(played["type"], "ack");
        assert_eq!(last_view(&w.a)["viewer"], "p2");
        w.a.receive_json(json!({
            "type": "action",
            "action": { "type": "concede", "playerId": "p1", "nonce": "x4" },
            "playerId": "p1",
        }));
        idle(&w).await;
        // Stamped p2, the seat P1 plays now: p1 — P2 — wins.
        let actions = table(&w.app, |data| json!(data.tables.match_actions)).await;
        let last = &actions.last().expect("a logged action")["action"];
        assert_eq!(last["type"], "concede");
        assert_eq!(last["playerId"], "p2");
        let results = table(&w.app, |data| json!(data.tables.results)).await;
        assert_eq!(results[0]["winnerProfileId"], P2);
    }

    #[tokio::test]
    async fn r677_credits_the_result_by_the_seats_as_they_are_played_at_the_end_the_winning_seats_current_account_wins_elo_included() {
        let w = world(WorldOptions::default()).await;
        let glitch = hand_card(&last_view(&w.a), "test-glitch-swap");
        frame(&w, &w.a, "n1", json!({ "type": "play", "instanceId": glitch })).await;
        // P2 now plays p1, whose hand holds test-lethal: seat p1 wins.
        let lethal = hand_card(&last_view(&w.b), "test-lethal");
        let played = frame(&w, &w.b, "n2", json!({ "type": "play", "instanceId": lethal })).await.expect("a reply");
        assert_eq!(played["type"], "ack");

        let results = table(&w.app, |data| json!(data.tables.results)).await;
        let result = &results[0];
        assert_eq!(result["matchId"], MATCH_ID);
        assert_eq!(result["winnerProfileId"], P2);
        assert_eq!(result["reason"], "hero-death");
        // Seat p1's row is P2's now, and P2's rating moved up.
        assert_eq!(result["players"], json!([P2, P1]));
        let after = result["ratingAfter"][0].as_f64().expect("a rating");
        let before = result["ratingBefore"][0].as_f64().unwrap_or(f64::INFINITY);
        assert!(after > before);
        let profiles = table(&w.app, |data| json!(data.tables.profiles)).await;
        let profile = profiles.iter().find(|row| row["id"] == P2).expect("P2's profile");
        assert!(profile["rating"].as_f64().expect("a rating") > 1000.0);
    }

    #[tokio::test]
    async fn r677_a_disconnect_grace_runs_on_the_seat_the_account_plays_now() {
        let w = world(WorldOptions::default()).await;
        let glitch = hand_card(&last_view(&w.a), "test-glitch-swap");
        frame(&w, &w.a, "n1", json!({ "type": "play", "instanceId": glitch })).await;
        let actor = w.app.matches.actor_for(&w.app, MATCH_ID).await.expect("the match's actor");
        actor.detach(PlayerId::P1); // P1's connection: the account that began in p1
        actor.idle().await;
        let clocks = to_json(&actor.clocks());
        assert!(!clocks["graceDeadline"]["p2"].is_null());
        assert!(clocks["graceDeadline"]["p1"].is_null());
    }

    #[tokio::test]
    async fn r677_a_rebuilt_actor_reads_the_swap_off_the_folded_state() {
        let w = world(WorldOptions::default()).await;
        let glitch = hand_card(&last_view(&w.a), "test-glitch-swap");
        frame(&w, &w.a, "n1", json!({ "type": "play", "instanceId": glitch })).await;
        w.app.matches.stop(MATCH_ID).await;
        let fresh = create_fake_socket();
        // TS: the attach resolves with the seat P1 began in, "p1" (SURFACE §11.2 answers `()`).
        w.app.matches.attach(&w.app, MATCH_ID, P1, fresh.socket()).await.expect("P1 attaches to the rebuilt actor");
        idle(&w).await;
        assert_eq!(last_view(&fresh)["viewer"], "p2");
    }
}

mod glitchs_boards {
    //! Glitch's boards (R678).
    use super::*;

    /// The boards' cards are ids the test catalog holds, so the engine freezes them as given (an id
    /// it cannot rebuild is dropped, `subsystems::last_boards::freeze_last_boards`).
    const OWN_1: &str = "test-card-11";
    const OWN_2: &str = "test-card-12";
    const PRACTICE: &str = "test-card-13";
    const OTHER_5: &str = "test-card-15";
    const OTHER_6: &str = "test-card-16";
    const OTHER_7: &str = "test-card-17";

    fn board(def_id: &str) -> Vec<Value> {
        vec![json!({ "defId": def_id, "radiant": false })]
    }

    #[tokio::test]
    async fn r678_freezes_two_other_players_last_server_boards_on_the_match_never_either_seats_own() {
        let w = world(WorldOptions {
            boards: vec![
                (P1, "server", board(OWN_1)),
                (P2, "server", board(OWN_2)),
                ("profile-3", "practice", board(PRACTICE)),
                ("profile-4", "server", vec![]),
                ("profile-5", "server", board(OTHER_5)),
                ("profile-6", "server", board(OTHER_6)),
                ("profile-7", "server", board(OTHER_7)),
            ],
            ..WorldOptions::default()
        })
        .await;
        assert_eq!(GLITCH_BOARDS_SAMPLED as i64, 2);
        let row = match_row(&w.app, MATCH_ID).await.expect("the match row");
        let frozen = row["glitchBoards"].as_array().cloned().unwrap_or_default();
        assert_eq!(frozen.len(), 2);
        let others = [json!(board(OTHER_5)), json!(board(OTHER_6)), json!(board(OTHER_7))];
        for entry in &frozen {
            assert!(others.contains(entry), "{entry} is not another player's board");
        }
        assert_ne!(frozen[0], frozen[1]);

        // A rebuild folds with the same boards (§9.3): TS spied on the port's `fold`; the rebuilt
        // state holds exactly the boards it was folded with, and folds to the same game.
        let actor = w.app.matches.actor_for(&w.app, MATCH_ID).await.expect("the match's actor");
        let live = actor.engine_state();
        w.app.matches.stop(MATCH_ID).await;
        let rebuilt = w.app.matches.actor_for(&w.app, MATCH_ID).await.expect("the rebuilt actor").engine_state();
        assert_eq!(to_json(&rebuilt)["glitchBoards"], json!({ "p1": frozen[0], "p2": frozen[1] }));
        assert_eq!(jackioh_engine::hash_state(&rebuilt), jackioh_engine::hash_state(&live));
    }

    #[tokio::test]
    async fn r678_passes_what_exists_one_other_board_leaves_the_second_seats_empty_none_omits_the_field() {
        let one = world(WorldOptions { boards: vec![("profile-5", "server", board(OTHER_5))], ..WorldOptions::default() }).await;
        let row = match_row(&one.app, MATCH_ID).await.expect("the match row");
        assert_eq!(row["glitchBoards"], json!([board(OTHER_5), []]));

        let none = world(WorldOptions { boards: vec![(P1, "server", board(OWN_1))], ..WorldOptions::default() }).await;
        let row = match_row(&none.app, MATCH_ID).await.expect("the match row");
        assert!(row.get("glitchBoards").is_none_or(Value::is_null), "no other board, no field: {row}");
    }
}

mod glitchs_void {
    //! Glitch's void (R679).
    use super::*;

    fn void_decks() -> WorldOptions {
        WorldOptions { p1_deck: Some(vec!["test-glitch-void"]), ..WorldOptions::default() }
    }

    async fn play_the_void(w: &World) {
        let glitch = hand_card(&last_view(&w.a), "test-glitch-void");
        frame(w, &w.a, "n1", json!({ "type": "play", "instanceId": glitch })).await;
    }

    #[tokio::test]
    async fn r679_writes_no_result_no_rating_no_game_record_and_no_last_board_and_the_match_is_gone() {
        // TS also swapped in a `GameRecorder` whose summary is null; the Rust results writer
        // summarises directly (SURFACE §11.3), and a void reaches no summary at all.
        let w = world(void_decks()).await;
        play_the_void(&w).await;

        assert_eq!(table(&w.app, |data| json!(data.tables.results)).await, Vec::<Value>::new());
        assert_eq!(table(&w.app, |data| json!(data.tables.game_records)).await, Vec::<Value>::new());
        assert_eq!(table(&w.app, |data| json!(data.tables.last_boards)).await, Vec::<Value>::new());
        assert_eq!(table(&w.app, |data| json!(data.tables.rated_games)).await, Vec::<Value>::new());
        let profiles = table(&w.app, |data| json!(data.tables.profiles)).await;
        assert_eq!(profiles.iter().map(|row| row["rating"].as_f64()).collect::<Vec<_>>(), vec![Some(1000.0), Some(1000.0)]);
        // As if it never existed: no row, no log, and both players free to queue.
        assert_eq!(match_row(&w.app, MATCH_ID).await, None);
        assert_eq!(table(&w.app, |data| json!(data.tables.match_actions)).await, Vec::<Value>::new());
        assert_eq!(profiles.iter().map(|row| row["inMatchId"].clone()).collect::<Vec<_>>(), vec![Value::Null, Value::Null]);
        assert!(!w.app.matches.has(MATCH_ID));
    }

    #[tokio::test]
    async fn r679_closes_both_sockets_with_the_voided_close_code_after_a_last_view_that_shows_the_void() {
        let w = world(void_decks()).await;
        play_the_void(&w).await;
        for socket in [&w.a, &w.b] {
            assert!(!socket.is_open());
            assert_eq!(socket.close_code(), Some(MATCH_VOIDED_CLOSE_CODE as u16));
        }
        assert_eq!(last_view(&w.b)["result"], json!({ "winner": "draw", "reason": "voided" }));
        // A socket that arrives later finds no match.
        let late = w.app.matches.attach(&w.app, MATCH_ID, P1, create_fake_socket().socket()).await;
        let refusal = late.expect_err("a voided match takes no socket");
        assert!(format!("{refusal:?}").contains("no such match"), "{refusal:?}");
    }

    #[tokio::test]
    async fn r679_logs_one_line_naming_the_match_and_both_profiles() {
        let (logs, _recording) = Logs::record();
        let w = world(void_decks()).await;
        play_the_void(&w).await;
        let lines = logs.of("match.voided");
        assert_eq!(lines.len(), 1, "{lines:?}");
        let line = &lines[0];
        assert_eq!(line["level"], "WARN");
        let text = line.to_string();
        for named in [MATCH_ID, P1, P2] {
            assert!(text.contains(named), "{text} does not name {named}");
        }
    }

    #[tokio::test]
    async fn r679_a_voided_conquest_game_never_happened_the_series_plays_the_same_game_again() {
        let (logs, _recording) = Logs::record();
        let app = test_app().await;
        let mut tokens = Vec::new();
        for id in [P1, P2] {
            fake(&app).lock().await.seed_profile(json!({
                "id": id,
                "userId": format!("user-{id}"),
                "status": "active",
                "rating": 1000,
            }));
            tokens.push(add_user(&app, &format!("user-{id}"), &format!("{id}@example.test"), true));
        }
        // TS's trios held one-card decks that the scripted directory never built a game from; the
        // real registry starts the game, so each deck is a legal one of real cards.
        let pool = playable();
        let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
        let trio = |owner: &str, offset: usize| -> Value {
            let decks: Vec<Value> = (0..3)
                .map(|slot| {
                    let start = offset + slot * size;
                    json!({ "name": format!("{owner}-{slot}"), "cards": pool[start..start + size] })
                })
                .collect();
            json!({ "name": owner, "decks": decks })
        };
        let mut tx = app.db.begin(None).await.expect("store.tx");
        start_series(
            &app,
            from(json!({
                "seriesId": "series-glitch",
                "firstMatchId": MATCH_ID,
                "sides": [
                    { "profileId": P1, "trio": trio("a", 0) },
                    { "profileId": P2, "trio": trio("b", 3 * size) },
                ],
                "seedBase": "seed-base",
                "catalogVersion": jackioh_cards::catalog_version(),
                "ranked": true,
            })),
            &mut tx,
        )
        .await
        .expect("the series starts");
        tx.commit().await.expect("the series commits");
        for token in &tokens {
            let (status, _, body) =
                call(&app, "POST", "/api/series/series-glitch/pick", Some(token.as_str()), json!({ "slot": 0 })).await;
            assert_eq!(status, 200, "{body}");
        }
        assert_eq!(logs.of("match.started").len(), 1);
        let first_start = match_row(&app, MATCH_ID).await.expect("game 1's match");
        let before = series_row(&app).await;

        // The actor's void: the registry has let the actor go, then `voidMatch`.
        app.matches.stop(MATCH_ID).await;
        void_match(&app, from(json!({ "matchId": MATCH_ID, "players": [P1, P2], "at": first_start["createdAt"] })))
            .await
            .expect("the void lands");

        let after = series_row(&app).await;
        assert_eq!(after["status"], "playing");
        assert_eq!(after["games"], before["games"]);
        assert_eq!(after["nextMatchId"], MATCH_ID);
        assert_eq!(table(&app, |data| json!(data.tables.results)).await, Vec::<Value>::new());
        // Started again: the same id, seats and seed, and both players are in it.
        assert_eq!(logs.of("match.started").len(), 2);
        let again = match_row(&app, MATCH_ID).await.expect("the game's match, started again");
        for field in ["id", "seed", "players", "decks", "catalogVersion", "ranked"] {
            assert_eq!(again[field], first_start[field], "{field}");
        }
        assert_eq!(again["status"], "live");
        let profiles = table(&app, |data| json!(data.tables.profiles)).await;
        assert_eq!(profiles.iter().map(|row| row["inMatchId"].clone()).collect::<Vec<_>>(), vec![json!(MATCH_ID), json!(MATCH_ID)]);
    }

    /// Every playable catalog id, in `catalog.json` order.
    fn playable() -> Vec<String> {
        jackioh_cards::register_all();
        jackioh_cards::CATALOG
            .iter()
            .filter(|(_, def)| !def.token && !to_json(&def.tags).as_array().is_some_and(|tags| tags.contains(&json!("Token"))))
            .map(|(id, _)| id.clone())
            .collect()
    }

    async fn series_row(app: &Arc<App>) -> Value {
        to_json(&store!(app, t => t.series_get("series-glitch").await.expect("series.get")).expect("the series row"))
    }
}
