//! Daily games (SPEC §9.13, #3727): a game played a move at a time over plain HTTPS, with nothing in
//! memory between moves, lost on time (R3727000), holding no seat (R3727004), pinned to the catalog it
//! started on (R3727005), its open challenges cancelled by a block (R3727007), holding its accounts
//! from deletion while it runs (R3727008), and "your move" sent by email and Web Push, to a browser's
//! push service only.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use indexmap::IndexMap;
use ring::rand::{SecureRandom, SystemRandom};
use ring::{agreement, signature};
use serde_json::{Value, json};
use tokio::sync::Semaphore;

use jackioh_engine::testkit::{clear_overrides, register_catalog, register_shipped};
use jackioh_engine::{CardCost, CardDefs, ShippedCatalog};
use jackioh_server::api::catalog::{LoadCatalogOptions, load_catalog};
use jackioh_server::api::queue::try_pair;
use jackioh_server::app::App;
use jackioh_server::config::{DAILY_MAX_ACTIVE_GAMES, MS_PER_DAY};
use jackioh_server::db::store::{DailyNotifyRow, MatchStatus, PushSubRow};

use crate::support::deps::{
    TestAppOptions, add_user, call, json_request, send as send_request, test_app_with,
};
use crate::support::engine::{TEST_GLITCH_VOID, fake_deck, install_test_cards};
use crate::support::store::{Account, account, befriend, fail_on, fake, fake_lock, stop_failing};

// Helpers

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

fn code_of(body: &Value) -> &str {
    body["error"]["code"].as_str().unwrap_or_default()
}

/// The test app over an empty store, or over `db` as a restart finds it, with `options`' channels.
async fn app_with(db: Option<jackioh_server::db::store::Db>, options: TestAppOptions) -> Arc<App> {
    jackioh_cards::register_all();
    test_app_with(TestAppOptions {
        skip_fixtures: true,
        db,
        ..options
    })
    .await
}

/// A new process over `app`'s store: no actor, no cache, nothing in memory carried over, and the
/// accounts signed in again (the fixture auth provider is the process's own).
async fn restart(app: &Arc<App>, people: &mut [&mut Account]) -> Arc<App> {
    let restarted = app_with(Some(app.db.clone()), TestAppOptions::default()).await;
    for person in people.iter_mut() {
        person.token = sign_in_again(&restarted, person);
    }
    restarted
}

/// `person`'s user (`account`'s `user-<n>`, its profile id `uuid(n)`), signed in to `app`'s auth
/// provider again.
fn sign_in_again(app: &App, person: &Account) -> String {
    let n: u64 = person
        .id
        .rsplit('-')
        .next()
        .and_then(|digits| digits.parse().ok())
        .expect("an id `account` made");
    let email = if n == NO_EMAIL {
        String::new()
    } else {
        format!("user-{n}@example.test")
    };
    add_user(app, &format!("user-{n}"), &email, true)
}

/// The account with no email address below.
const NO_EMAIL: u64 = 4;

/// `account`.
async fn person(app: &Arc<App>, n: u32, base: &str) -> Account {
    account(app, n, base).await
}

/// Two friends on a fresh app.
async fn two_friends() -> (Arc<App>, Account, Account) {
    let app = app_with(None, TestAppOptions::default()).await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;
    (app, ada, bo)
}

/// An All Random daily challenge at `days` per move.
fn random_challenge(friend: &Account, days: i64) -> Value {
    json!({ "friendId": friend.id, "mode": "random", "perMoveDays": days })
}

/// `from` challenges `to` to an All Random daily game and `to` accepts: its match id.
async fn daily_game(app: &Arc<App>, from: &Account, to: &Account, days: i64) -> String {
    let (status, body) = api(
        app,
        "POST",
        "/api/daily/challenges",
        &from.token,
        random_challenge(to, days),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let id = body["id"].as_str().expect("a challenge id").to_string();
    let (status, body) = api(
        app,
        "POST",
        &format!("/api/daily/challenges/{id}/accept"),
        &to.token,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    body["matchId"].as_str().expect("a match id").to_string()
}

/// `GET /api/daily/games/:id` for `who`, 200.
async fn read(app: &Arc<App>, who: &Account, match_id: &str) -> Value {
    let (status, body) = api(
        app,
        "GET",
        &format!("/api/daily/games/{match_id}"),
        &who.token,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

/// `POST …/actions` `{after, nonce, action}`.
async fn send(
    app: &Arc<App>,
    who: &Account,
    match_id: &str,
    after: i64,
    nonce: &str,
    action: Value,
) -> (u16, Value) {
    api(
        app,
        "POST",
        &format!("/api/daily/games/{match_id}/actions"),
        &who.token,
        json!({ "after": after, "nonce": nonce, "action": action }),
    )
    .await
}

fn legal_has(game: &Value, kind: &str) -> bool {
    game["legal"]
        .as_array()
        .is_some_and(|legal| legal.iter().any(|action| action["type"] == kind))
}

/// The `(attackerId, targetId)` of each attack `game`'s seat may declare, in `legal`'s order.
fn legal_attack_ids(game: &Value) -> Vec<(String, String)> {
    game["legal"]
        .as_array()
        .expect("legal")
        .iter()
        .filter(|action| action["type"] == "attack")
        .map(ids_of)
        .collect()
}

/// The `(attackerId, targetId)` of each entry of `game`'s forecast, in its order (R3855000).
fn forecast_ids(game: &Value) -> Vec<(String, String)> {
    game["forecast"]["attacks"]
        .as_array()
        .expect("a forecast's attacks")
        .iter()
        .map(ids_of)
        .collect()
}

fn ids_of(entry: &Value) -> (String, String) {
    let id = |key: &str| entry[key].as_str().expect("an id").to_string();
    (id("attackerId"), id("targetId"))
}

/// The move `game`'s seat makes: keep its whole hand at the mulligan, else end its turn, else the
/// first answer to its prompt.
fn next_move(game: &Value) -> Value {
    if legal_has(game, "mulligan") {
        let keep: Vec<Value> = game["view"]["you"]["hand"]
            .as_array()
            .map(|hand| hand.iter().map(|card| card["instanceId"].clone()).collect())
            .unwrap_or_default();
        return json!({ "type": "mulligan", "keep": keep });
    }
    if legal_has(game, "endTurn") {
        return json!({ "type": "endTurn" });
    }
    game["legal"]
        .as_array()
        .and_then(|legal| legal.iter().find(|action| action["type"] != "concede"))
        .cloned()
        .unwrap_or_else(|| panic!("no move to make: {game}"))
}

/// One move by whichever of `players` holds it (the first in seat order at the mulligan): who moved.
async fn one_move(app: &Arc<App>, players: [&Account; 2], match_id: &str, n: usize) -> usize {
    for (i, who) in players.iter().enumerate() {
        let game = read(app, who, match_id).await;
        if game["hasMove"] != true {
            continue;
        }
        let seq = game["seq"].as_i64().expect("a seq");
        let (status, body) = send(app, who, match_id, seq, &format!("move-{n}"), next_move(&game)).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["seq"], seq + 1, "{body}");
        return i;
    }
    panic!("nobody holds the move")
}

/// Both mulligans answered.
async fn past_the_mulligan(app: &Arc<App>, players: [&Account; 2], match_id: &str) {
    for n in 0..2 {
        let game = read(app, players[0], match_id).await;
        if game["view"]["phase"] != "mulligan" && !legal_has(&game, "mulligan") {
            let other = read(app, players[1], match_id).await;
            if !legal_has(&other, "mulligan") {
                return;
            }
        }
        one_move(app, players, match_id, 100 + n).await;
    }
}

/// The account holding the move now (after the mulligan, one only).
async fn holder<'a>(app: &Arc<App>, players: [&'a Account; 2], match_id: &str) -> (&'a Account, &'a Account) {
    let first = read(app, players[0], match_id).await;
    if first["hasMove"] == true {
        (players[0], players[1])
    } else {
        (players[1], players[0])
    }
}

/// A fake HTTP service recording every request's path and body, answering `status` for a path ending
/// in `/gone` and 200 for the rest.
async fn fake_service(status: StatusCode) -> (String, Arc<StdMutex<Vec<(String, HeaderMap, Vec<u8>)>>>) {
    type Seen = Arc<StdMutex<Vec<(String, HeaderMap, Vec<u8>)>>>;
    async fn take(
        State((seen, status)): State<(Seen, StatusCode)>,
        uri: Uri,
        headers: HeaderMap,
        body: axum::body::Bytes,
    ) -> StatusCode {
        seen.lock()
            .expect("the requests")
            .push((uri.path().to_string(), headers, body.to_vec()));
        if uri.path().ends_with("/gone") {
            status
        } else {
            StatusCode::CREATED
        }
    }
    let seen: Seen = Arc::new(StdMutex::new(Vec::new()));
    let router = Router::new()
        .route("/{*rest}", axum::routing::post(take))
        .with_state((Arc::clone(&seen), status));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a local port");
    let address = listener.local_addr().expect("its address");
    tokio::spawn(async move { axum::serve(listener, router).await });
    (format!("http://{address}"), seen)
}

/// What a fake service saw: each request's path, headers and body.
type Requests = Arc<StdMutex<Vec<(String, HeaderMap, Vec<u8>)>>>;

/// A fake HTTP service that records every request, then holds it until the returned gate is opened
/// (`add_permits(1)`: each request gives the permit back, so one opens it for good), and answers
/// `status` on every path.
async fn held_service(status: StatusCode) -> (String, Requests, Arc<Semaphore>) {
    type Held = (Requests, StatusCode, Arc<Semaphore>);
    async fn take(
        State((seen, status, gate)): State<Held>,
        uri: Uri,
        headers: HeaderMap,
        body: axum::body::Bytes,
    ) -> StatusCode {
        seen.lock()
            .expect("the requests")
            .push((uri.path().to_string(), headers, body.to_vec()));
        let _held = gate.acquire().await;
        status
    }
    let seen: Requests = Arc::new(StdMutex::new(Vec::new()));
    let gate = Arc::new(Semaphore::new(0));
    let router = Router::new()
        .route("/{*rest}", axum::routing::post(take))
        .with_state((Arc::clone(&seen), status, Arc::clone(&gate)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a local port");
    let address = listener.local_addr().expect("its address");
    tokio::spawn(async move { axum::serve(listener, router).await });
    (format!("http://{address}"), seen, gate)
}

/// How long a move may take to answer while a mail or push service holds its answer: under the 10 s a
/// send waits (`PLAYER_EMAIL_TIMEOUT_MS`, `PUSH_TIMEOUT_MS`), so a move that waited on a send fails it.
const ANSWERS_WITHIN: Duration = Duration::from_secs(5);

/// `future` (a request), failing the test when it waits on a held service instead of answering.
async fn answers_at_once<T>(what: &str, future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(ANSWERS_WITHIN, future)
        .await
        .unwrap_or_else(|_| panic!("{what} waited on the held service"))
}

/// Polls `done` until it holds, which the notifier's task makes happen after a move has answered.
async fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..1000 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never: {what}");
}

/// Waits for the notifier's task to decide for every profile a move queued (§9.13).
async fn notifier_idle(app: &Arc<App>) {
    tokio::time::timeout(Duration::from_secs(20), app.daily.notifier.idle())
        .await
        .expect("the notifier goes idle");
}

// The game: played over HTTPS, rebuilt from the store between moves

#[tokio::test]
async fn a_daily_game_is_played_to_its_end_across_a_restart() {
    let (mut app, mut ada, mut bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 3).await;

    // R3727002: at the mulligan the move is each seat's still owing one.
    let mut movers = Vec::new();
    for n in 0..8 {
        app = restart(&app, &mut [&mut ada, &mut bo]).await;
        movers.push(one_move(&app, [&ada, &bo], &match_id, n).await);
    }
    // Both players moved, the move passing between them.
    assert!(movers.contains(&0) && movers.contains(&1), "{movers:?}");

    app = restart(&app, &mut [&mut ada, &mut bo]).await;
    let (on_move, other) = holder(&app, [&ada, &bo], &match_id).await;
    let other_id = other.id.clone();
    let game = read(&app, on_move, &match_id).await;
    let seq = game["seq"].as_i64().expect("a seq");
    assert_eq!(seq, 8, "every move is in the log");
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        seq,
        "concede",
        json!({ "type": "concede" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["over"], true, "{body}");
    assert_eq!(body["lostOnTime"], false, "{body}");

    app = restart(&app, &mut [&mut ada, &mut bo]).await;
    let other = if ada.id == other_id { &ada } else { &bo };
    let theirs = read(&app, other, &match_id).await;
    assert_eq!(theirs["over"], true, "{theirs}");
    assert_eq!(theirs["holds"], json!([]), "{theirs}");
    let (status, list) = api(&app, "GET", "/api/daily", &other.token, Value::Null).await;
    assert_eq!(status, 200, "{list}");
    assert_eq!(list["games"][0]["result"]["outcome"], "win", "{list}");
    assert_eq!(list["waiting"], 0, "{list}");
    // §9.5: the result is recorded, so the replay opens.
    let (status, replay) = api(
        &app,
        "GET",
        &format!("/api/replays/{match_id}"),
        &other.token,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{replay}");
}

#[tokio::test]
async fn a_daily_game_never_builds_a_live_actor() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    assert!(app.matches.actor_for(&app, &match_id).await.is_err());
    // §9.13: and its rematch is a new daily challenge, never a live one.
    let (status, body) = api(
        &app,
        "POST",
        &format!("/api/matches/{match_id}/rematch"),
        &ada.token,
        json!({ "stakes": 1 }),
    )
    .await;
    assert_eq!((status, code_of(&body)), (409, "daily"), "{body}");
}

#[tokio::test]
async fn the_same_action_sent_twice_applies_once() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    let (on_move, _) = holder(&app, [&ada, &bo], &match_id).await;
    let game = read(&app, on_move, &match_id).await;
    let action = next_move(&game);
    let (first, once) = send(&app, on_move, &match_id, 0, "twice", action.clone()).await;
    let (second, again) = send(&app, on_move, &match_id, 0, "twice", action).await;
    assert_eq!((first, second), (200, 200), "{once} {again}");
    assert_eq!(once["seq"], 1);
    assert_eq!(again["seq"], 1, "the retry applied nothing: {again}");
}

#[tokio::test]
async fn a_stale_action_is_refused() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let (on_move, _) = holder(&app, [&ada, &bo], &match_id).await;
    let game = read(&app, on_move, &match_id).await;
    let seq = game["seq"].as_i64().expect("a seq");
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        seq - 1,
        "stale",
        json!({ "type": "endTurn" }),
    )
    .await;
    assert_eq!((status, code_of(&body)), (409, "stale"), "{body}");
    assert_eq!(
        body["game"]["seq"], seq,
        "the refusal carries the game as it stands: {body}"
    );
    // And the player who does not hold the move is offered none.
    let (_, waiting) = holder(&app, [&ada, &bo], &match_id).await;
    let theirs = read(&app, waiting, &match_id).await;
    assert_eq!(theirs["hasMove"], false);
    assert!(!legal_has(&theirs, "endTurn"), "{theirs}");
}

/// §9.13: only the player holding the move sends anything but a concede. The engine takes
/// `setAutoEndTurn` from either seat at any time; a daily game that took it from the other player
/// would let them append row after row, each making the holder's next move stale until it lost on
/// time.
#[tokio::test]
async fn a_player_not_holding_the_move_may_only_concede() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let (on_move, waiting) = holder(&app, [&ada, &bo], &match_id).await;
    let seq = read(&app, on_move, &match_id).await["seq"]
        .as_i64()
        .expect("a seq");

    for (n, action) in [
        json!({ "type": "setAutoEndTurn", "enabled": true }),
        json!({ "type": "endTurn" }),
    ]
    .into_iter()
    .enumerate()
    {
        let (status, body) = send(&app, waiting, &match_id, seq, &format!("not-mine-{n}"), action).await;
        assert_eq!((status, code_of(&body)), (422, "not_your_move"), "{body}");
        assert_eq!(body["game"]["seq"], seq, "nothing was logged: {body}");
    }
    let theirs = read(&app, waiting, &match_id).await;
    let offered: Vec<&Value> = theirs["legal"].as_array().expect("legal").iter().collect();
    assert_eq!(offered, vec![&json!({ "type": "concede" })], "{theirs}");

    // The holder's move, sent after the seq it read, still applies.
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        seq,
        "mine",
        json!({ "type": "endTurn" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["seq"], seq + 1, "{body}");
}

/// R3855000: a daily game's read, and every answer to an action, carries the engine's forecast of the
/// seat's attacks (R3477000): one entry per attack its `legal` lists, in its order, and none while the
/// other seat holds the move. The game is dealt from a random seed, so the test plays a card whenever
/// one is legal and ends its turn when none is, until a seat has an attack to forecast.
#[tokio::test]
async fn r3855000_a_daily_read_carries_the_forecast_of_its_seats_legal_attacks() {
    /// More than enough turns for a unit to be played and to be old enough to attack.
    const MOVES_TO_AN_ATTACK: usize = 60;
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;

    let mut found = None;
    for n in 0..MOVES_TO_AN_ATTACK {
        let (on_move, waiting) = holder(&app, [&ada, &bo], &match_id).await;
        let game = read(&app, on_move, &match_id).await;
        if legal_has(&game, "attack") {
            found = Some((on_move, waiting, game));
            break;
        }
        let action = game["legal"]
            .as_array()
            .and_then(|legal| legal.iter().find(|action| action["type"] == "play"))
            .cloned()
            .unwrap_or_else(|| next_move(&game));
        let seq = game["seq"].as_i64().expect("a seq");
        let (status, body) = send(&app, on_move, &match_id, seq, &format!("fc-{n}"), action).await;
        assert_eq!(status, 200, "{body}");
    }
    let (on_move, waiting, game) = found.expect("a seat with a legal attack");

    // The holder: the forecast's entries are its legal attacks, in order.
    let attacks = legal_attack_ids(&game);
    assert!(!attacks.is_empty(), "{game}");
    assert_eq!(forecast_ids(&game), attacks, "{game}");
    assert!(game["forecast"]["lethal"].is_boolean(), "{game}");

    // The other seat: no attack, and no lethal; a refusal's `game` says so too.
    let none = json!({ "attacks": [], "lethal": false });
    let theirs = read(&app, waiting, &match_id).await;
    assert_eq!(theirs["forecast"], none, "{theirs}");
    let seq = game["seq"].as_i64().expect("a seq");
    let (status, body) = send(
        &app,
        waiting,
        &match_id,
        seq,
        "fc-not-mine",
        json!({ "type": "setAutoEndTurn", "enabled": true }),
    )
    .await;
    assert_eq!((status, code_of(&body)), (422, "not_your_move"), "{body}");
    assert_eq!(body["game"]["forecast"], none, "{body}");

    // An action's answer carries the forecast as the new read of the holder's seat.
    let attack = game["legal"]
        .as_array()
        .and_then(|legal| legal.iter().find(|action| action["type"] == "attack"))
        .cloned()
        .expect("a legal attack");
    let (status, body) = send(&app, on_move, &match_id, seq, "fc-attack", attack).await;
    assert_eq!(status, 200, "{body}");
    assert!(body["forecast"].is_object(), "{body}");
    assert_eq!(forecast_ids(&body), legal_attack_ids(&body), "{body}");
}

/// Two requests under one nonce with different actions, both read at one seq: the store takes the
/// second append as a retry of the first, so the second's clock must not be written over the first's
/// logged move. Here the first (`setAutoEndTurn`, which keeps the move with its holder) commits after
/// the second (`endTurn`) read the game and before it writes, as a race on Postgres would have it.
#[tokio::test]
async fn one_nonce_sent_twice_at_once_with_two_actions_keeps_the_logged_one_s_clock() {
    use jackioh_engine::{Action, ActionBody, PlayerId};
    use jackioh_server::db::fake::{CallOutcome, FakeTables};
    use jackioh_server::db::store::MatchActionRow;
    use std::sync::atomic::AtomicBool;

    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let (on_move, waiting) = holder(&app, [&ada, &bo], &match_id).await;
    let game = read(&app, on_move, &match_id).await;
    let seq = game["seq"].as_i64().expect("a seq");
    let seat: PlayerId = serde_json::from_value(game["seat"].clone()).expect("a seat");
    let deadline = game["holds"][0]["deadline"].clone();

    let fired = Arc::new(AtomicBool::new(false));
    let (id, once) = (match_id.clone(), Arc::clone(&fired));
    fake(&app).lock().await.before_call = Some(Arc::new(move |method: &str, tables: &mut FakeTables| {
        // The last read of the second request's load, whose transaction commits what this writes.
        if method == "matches.checkpoint" && !once.swap(true, Ordering::SeqCst) {
            // The first request's transaction: its row, and its clock's seq.
            tables.match_actions.push(MatchActionRow {
                match_id: id.clone(),
                seq: seq + 1,
                action: Action::new(ActionBody::SetAutoEndTurn { enabled: true }, seat, "one-nonce"),
                at: 0,
            });
            if let Some(row) = tables.daily.games.iter_mut().find(|row| row.match_id == id) {
                row.seq = seq + 1;
            }
        }
        CallOutcome::Proceed
    }));
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        seq,
        "one-nonce",
        json!({ "type": "endTurn" }),
    )
    .await;
    fake(&app).lock().await.before_call = None;
    assert!(fired.load(Ordering::SeqCst), "the race ran");
    // Answered as a retry: the game as it stands, the first request's move logged.
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["seq"], seq + 1, "{body}");

    // The clock is the logged move's: the holder still holds the move, with the same deadline, and the
    // other player is offered nothing but a concede, so nobody loses on time for a move never logged.
    let mine = read(&app, on_move, &match_id).await;
    assert_eq!(mine["hasMove"], true, "{mine}");
    assert_eq!(mine["holds"][0]["deadline"], deadline, "{mine}");
    assert!(legal_has(&mine, "endTurn"), "{mine}");
    let theirs = read(&app, waiting, &match_id).await;
    assert_eq!(theirs["hasMove"], false, "{theirs}");
}

// R3727000: an expired move loses for its holder, whether the sweep or a read finds it first

#[tokio::test]
async fn r3727000_a_move_after_its_deadline_is_refused_and_lost_on_time() {
    for through_the_sweep in [true, false] {
        let (app, ada, bo) = two_friends().await;
        let match_id = daily_game(&app, &ada, &bo, 1).await;
        past_the_mulligan(&app, [&ada, &bo], &match_id).await;
        let (late, waiting) = holder(&app, [&ada, &bo], &match_id).await;
        let game = read(&app, late, &match_id).await;
        let seq = game["seq"].as_i64().expect("a seq");

        // A minute short of the deadline the move is still theirs.
        app.daily.set_offset(MS_PER_DAY - 60_000);
        assert_eq!(read(&app, waiting, &match_id).await["over"], false);
        app.daily.set_offset(MS_PER_DAY);
        if through_the_sweep {
            let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
            assert_eq!(status, 200, "{swept}");
            assert_eq!(swept["settled"], 1, "{swept}");
        }

        let (status, body) = send(
            &app,
            late,
            &match_id,
            seq,
            "too-late",
            json!({ "type": "endTurn" }),
        )
        .await;
        assert_eq!((status, code_of(&body)), (409, "time_up"), "{body}");
        let theirs = read(&app, waiting, &match_id).await;
        assert_eq!(theirs["over"], true, "{theirs}");
        assert_eq!(theirs["lostOnTime"], true, "{theirs}");
        assert_eq!(theirs["seq"], seq + 1, "one server action ends it: {theirs}");
        let (_, list) = api(&app, "GET", "/api/daily", &waiting.token, Value::Null).await;
        assert_eq!(
            list["games"][0]["result"],
            json!({ "outcome": "win", "reason": "disconnect" }),
            "{list}"
        );
        let (_, list) = api(&app, "GET", "/api/daily", &late.token, Value::Null).await;
        assert_eq!(list["games"][0]["result"]["outcome"], "loss", "{list}");
    }
}

// R3727004: a daily game holds no seat; the cap bounds them

#[tokio::test]
async fn r3727004_a_player_at_the_daily_cap_still_plays_a_live_game_and_is_refused_one_more_daily() {
    let (app, ada, bo) = two_friends().await;
    let cy = person(&app, 3, "Cy").await;
    let mut games = Vec::new();
    for _ in 0..DAILY_MAX_ACTIVE_GAMES {
        games.push(daily_game(&app, &ada, &bo, 14).await);
    }
    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/challenges",
        &ada.token,
        random_challenge(&bo, 14),
    )
    .await;
    assert_eq!((status, code_of(&body)), (409, "daily_full"), "{body}");
    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/seeks",
        &ada.token,
        json!({ "mode": "random", "perMoveDays": 1 }),
    )
    .await;
    assert_eq!((status, code_of(&body)), (409, "daily_full"), "{body}");

    // Queued for a live game while twenty daily games run.
    let (status, body) = api(
        &app,
        "POST",
        "/api/queue",
        &ada.token,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    // A daily game's ending leaves the live ticket alone.
    let game = read(&app, &ada, &games[0]).await;
    let seq = game["seq"].as_i64().expect("a seq");
    let (status, body) = send(
        &app,
        &ada,
        &games[0],
        seq,
        "concede-0",
        json!({ "type": "concede" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["over"], true, "{body}");
    let mut tx = app.db.begin(None).await.expect("a transaction");
    assert!(
        tx.tickets_open_for_profile(&ada.id)
            .await
            .expect("a read")
            .is_some()
    );
    tx.commit().await.expect("commit");

    let (status, body) = api(&app, "POST", "/api/queue", &cy.token, json!({ "mode": "random" })).await;
    assert_eq!(status, 200, "{body}");
    try_pair(&app).await.expect("a sweep");
    let (status, me) = api(&app, "GET", "/api/auth/me", &ada.token, Value::Null).await;
    assert_eq!(status, 200, "{me}");
    let live = me["currentMatchId"]
        .as_str()
        .expect("in a live match")
        .to_string();
    assert!(!games.contains(&live));
    assert!(app.matches.actor_for(&app, &live).await.is_ok());
    // The daily games go on beside it.
    assert_eq!(read(&app, &ada, &games[1]).await["over"], false);
}

// Challenges

#[tokio::test]
async fn a_daily_challenge_to_an_offline_friend_opens_and_to_a_stranger_is_refused() {
    let (app, ada, bo) = two_friends().await;
    let cy = person(&app, 3, "Cy").await;
    // Bo has sent no heartbeat: offline, which a live challenge refuses.
    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/challenges",
        &ada.token,
        random_challenge(&bo, 7),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let (_, theirs) = api(&app, "GET", "/api/daily", &bo.token, Value::Null).await;
    assert_eq!(
        theirs["challenges"]["incoming"][0]["from"]["profileId"],
        ada.id.as_str(),
        "{theirs}"
    );
    assert_eq!(theirs["challenges"]["incoming"][0]["perMoveDays"], 7, "{theirs}");

    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/challenges",
        &ada.token,
        random_challenge(&cy, 7),
    )
    .await;
    assert_eq!(status, 404, "{body}");
    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/challenges",
        &ada.token,
        random_challenge(&bo, 2),
    )
    .await;
    assert_eq!((status, code_of(&body)), (422, "bad_time"), "{body}");
}

#[tokio::test]
async fn r3727007_a_block_cancels_open_daily_challenges_and_a_running_game_goes_on() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/challenges",
        &ada.token,
        random_challenge(&bo, 7),
    )
    .await;
    assert_eq!(status, 201, "{body}");

    let (status, body) = api(
        &app,
        "POST",
        "/api/blocks",
        &bo.token,
        json!({ "profileId": ada.id }),
    )
    .await;
    assert!(status == 201 || status == 204, "{status} {body}");

    for who in [&ada, &bo] {
        let (_, list) = api(&app, "GET", "/api/daily", &who.token, Value::Null).await;
        assert_eq!(
            list["challenges"],
            json!({ "incoming": [], "outgoing": [] }),
            "{list}"
        );
        assert_eq!(list["games"].as_array().map(Vec::len), Some(1), "{list}");
    }
    let game = read(&app, &ada, &match_id).await;
    assert_eq!(game["over"], false, "{game}");
    let (on_move, _) = holder(&app, [&ada, &bo], &match_id).await;
    let game = read(&app, on_move, &match_id).await;
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        game["seq"].as_i64().expect("a seq"),
        "after-block",
        next_move(&game),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

// R3727008: a running daily game holds its account from deletion

#[tokio::test]
async fn r3727008_an_account_playing_a_daily_game_is_deleted_only_once_the_game_ends() {
    for by_time in [false, true] {
        let (app, ada, bo) = two_friends().await;
        let match_id = daily_game(&app, &ada, &bo, 1).await;

        // While the game runs, neither player's account goes, and the game is left as it was.
        for who in [&ada, &bo] {
            let (status, body) = api(&app, "DELETE", "/api/account", &who.token, Value::Null).await;
            assert_eq!((status, code_of(&body)), (409, "conflict"), "{body}");
        }
        let game = read(&app, &bo, &match_id).await;
        assert_eq!(game["over"], false, "{game}");

        if by_time {
            // Both mulligans ran out: the game holds nothing, since the delete ends it on time first.
            app.daily.set_offset(MS_PER_DAY);
        } else {
            let seq = read(&app, &ada, &match_id).await["seq"].as_i64().expect("a seq");
            let (status, body) = send(
                &app,
                &ada,
                &match_id,
                seq,
                "ada-concedes",
                json!({ "type": "concede" }),
            )
            .await;
            assert_eq!(status, 200, "{body}");
        }
        let (status, body) = api(&app, "DELETE", "/api/account", &ada.token, Value::Null).await;
        assert_eq!(status, 204, "by time {by_time}: {body}");
        let mut tx = app.db.begin(None).await.expect("a transaction");
        let row = tx
            .matches_get(&match_id)
            .await
            .expect("a read")
            .expect("the match stays");
        tx.commit().await.expect("commit");
        assert_eq!(row.status, MatchStatus::Finished, "by time {by_time}");
    }
}

// One sweep at a time

/// §9.13: a sweep while another runs (the Worker's cron landing beside the awake server's own loop)
/// does nothing, so no notice goes twice, and the next sweep does the work.
#[tokio::test]
async fn a_sweep_while_another_runs_does_nothing() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    app.daily.set_offset(MS_PER_DAY);
    let running = app.daily.begin_sweep().expect("no sweep runs yet");
    let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(status, 200, "{swept}");
    assert_eq!(swept["settled"], 0, "{swept}");
    assert_eq!(match_status(&app, &match_id).await, MatchStatus::Live);
    drop(running);
    let (_, _, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(swept["settled"], 1, "{swept}");
    assert_eq!(match_status(&app, &match_id).await, MatchStatus::Finished);
}

// A game that fails to start

/// §9.13: an accept and its game are written in one transaction, so an accept whose game fails to
/// start leaves the challenge open to accept again; and a seek that a failed start claimed goes back
/// to the queue, so the other player's seek is never lost without a word.
#[tokio::test]
async fn a_daily_game_that_fails_to_start_leaves_its_challenge_or_seek_open() {
    let (app, ada, bo) = two_friends().await;
    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/challenges",
        &ada.token,
        random_challenge(&bo, 1),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let id = body["id"].as_str().expect("a challenge id").to_string();
    let accept = format!("/api/daily/challenges/{id}/accept");
    fail_on(&app, "dailyGames.create", "the store cannot answer").await;
    let (status, body) = api(&app, "POST", &accept, &bo.token, json!({ "mode": "random" })).await;
    stop_failing(&app).await;
    assert_eq!(status, 500, "{body}");
    let (_, list) = api(&app, "GET", "/api/daily", &bo.token, Value::Null).await;
    assert_eq!(list["games"], json!([]), "{list}");
    assert_eq!(
        list["challenges"]["incoming"].as_array().map(Vec::len),
        Some(1),
        "the challenge is still open: {list}"
    );
    let (status, body) = api(&app, "POST", &accept, &bo.token, json!({ "mode": "random" })).await;
    assert_eq!(status, 201, "{body}");

    // The queue: Ada's seek waits, Bo's claims it and fails to start, and Ada's goes back.
    let seek = json!({ "mode": "random", "perMoveDays": 3 });
    let (status, body) = api(&app, "POST", "/api/daily/seeks", &ada.token, seek.clone()).await;
    assert_eq!(status, 201, "{body}");
    assert!(body["seekId"].is_string(), "{body}");
    fail_on(&app, "dailyGames.create", "the store cannot answer").await;
    let (status, body) = api(&app, "POST", "/api/daily/seeks", &bo.token, seek.clone()).await;
    stop_failing(&app).await;
    assert_eq!(status, 500, "{body}");
    let (_, list) = api(&app, "GET", "/api/daily", &ada.token, Value::Null).await;
    assert_eq!(
        list["seeks"].as_array().map(Vec::len),
        Some(1),
        "Ada's seek is back: {list}"
    );
    let (status, body) = api(&app, "POST", "/api/daily/seeks", &bo.token, seek).await;
    assert_eq!(status, 201, "{body}");
    assert!(body["matchId"].is_string(), "it pairs with Ada's seek: {body}");
}

// A result whose write failed

/// The match row's status now.
async fn match_status(app: &Arc<App>, match_id: &str) -> MatchStatus {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let row = tx
        .matches_get(match_id)
        .await
        .expect("a read")
        .expect("the match row");
    tx.commit().await.expect("commit");
    row.status
}

/// §9.13: a game over whose result's write failed is written by the next sweep, so its match is
/// never left `live` with no result, which nothing else would read again.
#[tokio::test]
async fn a_daily_result_whose_write_failed_is_written_by_the_next_sweep() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    let seq = read(&app, &ada, &match_id).await["seq"].as_i64().expect("a seq");
    fail_on(&app, "results.insert", "the store cannot answer").await;
    let (status, body) = send(
        &app,
        &ada,
        &match_id,
        seq,
        "ada-concedes",
        json!({ "type": "concede" }),
    )
    .await;
    stop_failing(&app).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["over"], true, "{body}");
    assert_eq!(match_status(&app, &match_id).await, MatchStatus::Live);

    let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(status, 200, "{swept}");
    assert_eq!(swept["settled"], 1, "{swept}");
    assert_eq!(match_status(&app, &match_id).await, MatchStatus::Finished);
    let (_, _, again) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(again["settled"], 0, "nothing is left to write: {again}");
}

// R679: a game a Glitch voids never happened

/// R679: a daily game a Glitch voids is forgotten as a live one is: no result, no match row, log or
/// daily game left, out of both lists, and neither account held from deletion by it.
#[tokio::test]
async fn r679_a_daily_game_a_glitch_voids_is_forgotten_and_holds_neither_account() {
    install_test_cards();
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    // Before anyone moves, both decks become ones the void opens in (Quickdraw), so whichever seat
    // takes the first turn holds it; every request folds the game from its row.
    {
        let mut data = fake_lock(&app).await;
        let row = data
            .tables
            .matches
            .iter_mut()
            .find(|row| row.id == match_id)
            .expect("the match row");
        row.decks = (fake_deck(&[TEST_GLITCH_VOID]), fake_deck(&[TEST_GLITCH_VOID]));
    }
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let (on_move, _) = holder(&app, [&ada, &bo], &match_id).await;
    let game = read(&app, on_move, &match_id).await;
    let glitch = game["view"]["you"]["hand"]
        .as_array()
        .and_then(|hand| hand.iter().find(|card| card["defId"] == TEST_GLITCH_VOID))
        .and_then(|card| card["instanceId"].as_str())
        .unwrap_or_else(|| panic!("the void opens in hand: {game}"))
        .to_string();
    let seq = game["seq"].as_i64().expect("a seq");
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        seq,
        "the-void",
        json!({ "type": "play", "instanceId": glitch }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["over"], true, "{body}");
    assert_eq!(
        body["view"]["result"],
        json!({ "winner": "draw", "reason": "voided" }),
        "{body}"
    );

    {
        let data = fake_lock(&app).await;
        let tables = &data.tables;
        assert!(
            tables.matches.iter().all(|row| row.id != match_id),
            "the match row is forgotten"
        );
        assert!(
            tables.match_actions.iter().all(|row| row.match_id != match_id),
            "and its log"
        );
        assert!(
            tables.daily.games.iter().all(|row| row.match_id != match_id),
            "and its daily game"
        );
        assert!(tables.results.is_empty(), "no result is written");
    }
    for who in [&ada, &bo] {
        let (status, list) = api(&app, "GET", "/api/daily", &who.token, Value::Null).await;
        assert_eq!(status, 200, "{list}");
        assert_eq!(list["games"], json!([]), "{list}");
        let (status, body) = api(
            &app,
            "GET",
            &format!("/api/daily/games/{match_id}"),
            &who.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 404, "{body}");
    }
    let (status, body) = api(&app, "DELETE", "/api/account", &bo.token, Value::Null).await;
    assert_eq!(status, 204, "{body}");
    clear_overrides();
}

// R3727005: a game keeps the catalog it started on

/// The catalog the game below started on, which the next build carries.
const PLAYED_ON: &str = "v-daily-pinned";

#[tokio::test]
async fn r3727005_a_daily_game_plays_on_its_catalog_after_a_patch_ships_and_its_replay_hashes() {
    // As `actor/pinned.rs` deploys: the cards as they shipped, then the next build's, every card
    // priced past any mana (an All Random game deals from the whole catalog).
    jackioh_cards::register_all();
    let shipped: CardDefs = jackioh_engine::catalog::registered_catalog().clone();
    register_shipped(vec![ShippedCatalog::new(
        PLAYED_ON,
        shipped.clone(),
        IndexMap::new(),
    )]);
    let mut priced_out = shipped;
    for def in priced_out.values_mut() {
        def.cost = CardCost::Fixed(jackioh_engine::config::MAX_MANA + 1);
    }
    register_catalog(priced_out);

    let mut catalog = load_catalog(LoadCatalogOptions {
        version: None,
        json: None,
    })
    .await
    .expect("the catalog");
    catalog.version = PLAYED_ON.to_string();
    let first = test_app_with(TestAppOptions {
        skip_fixtures: true,
        catalog: Some(catalog),
        ..TestAppOptions::default()
    })
    .await;
    let mut ada = person(&first, 1, "Ada").await;
    let mut bo = person(&first, 2, "Bo").await;
    befriend(&first, &ada, &bo).await;
    let match_id = daily_game(&first, &ada, &bo, 1).await;

    // The patch ships: the next build, over the same store.
    let app = test_app_with(TestAppOptions {
        skip_fixtures: true,
        db: Some(first.db.clone()),
        ..TestAppOptions::default()
    })
    .await;
    for who in [&mut ada, &mut bo] {
        who.token = sign_in_again(&app, who);
    }
    assert_ne!(app.catalog.version, PLAYED_ON);
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    // A card is played at its old price, which the new build would refuse: turns end until a seat
    // can pay for one.
    let mut found = None;
    for n in 0..12 {
        let (on_move, _) = holder(&app, [&ada, &bo], &match_id).await;
        let game = read(&app, on_move, &match_id).await;
        assert_eq!(game["catalogVersion"], PLAYED_ON, "{game}");
        assert_eq!(game["olderCatalog"], true, "{game}");
        let play = game["legal"]
            .as_array()
            .and_then(|legal| legal.iter().find(|action| action["type"] == "play").cloned());
        if let Some(play) = play {
            found = Some((on_move, game, play));
            break;
        }
        one_move(&app, [&ada, &bo], &match_id, 200 + n).await;
    }
    let (on_move, game, play) = found.expect("a card the started-on catalog lets a seat play");
    let seq = game["seq"].as_i64().expect("a seq");
    let (status, body) = send(&app, on_move, &match_id, seq, "pinned-play", play).await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        seq + 1,
        "pinned-concede",
        json!({ "type": "concede" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["over"], true, "{body}");

    // R768: its replay folds under the started-on catalog to the hash its result recorded.
    let (status, replay) = api(
        &app,
        "GET",
        &format!("/api/replays/{match_id}"),
        &ada.token,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{replay}");
    clear_overrides();
}

// Telling a player it's their move

#[tokio::test]
async fn the_sweep_emails_once_per_account_and_none_when_off_or_without_email() {
    // The games start on a server with no mail key, so only the sweep below can email.
    let (quiet, ada, bo) = two_friends().await;
    let cy = person(&quiet, 3, "Cy").await;
    fake(&quiet).lock().await.seed_profile(json!({
        "id": "00000000-0000-4000-8000-000000000004",
        "userId": "user-4",
        "status": "active",
        "username": "Dee",
        "usernameTag": 4,
        "usernamePrompted": true,
        "email": "",
    }));
    let dee = Account {
        id: "00000000-0000-4000-8000-000000000004".to_string(),
        name: "Dee#4".to_string(),
        token: add_user(&quiet, "user-4", "", true),
    };
    befriend(&quiet, &cy, &dee).await;
    daily_game(&quiet, &ada, &bo, 1).await;
    daily_game(&quiet, &ada, &bo, 3).await;
    daily_game(&quiet, &cy, &dee, 1).await;
    // Bo turns the emails off.
    let (status, body) = api(
        &quiet,
        "PUT",
        "/api/daily/settings",
        &bo.token,
        json!({ "emailOn": false }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["emailOn"], false, "{body}");

    let (mail_api, seen) = fake_service(StatusCode::OK).await;
    let app = app_with(
        Some(quiet.db.clone()),
        TestAppOptions {
            mail_api: Some(format!("{mail_api}/emails")),
            ..TestAppOptions::default()
        },
    )
    .await;
    let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(status, 200, "{swept}");
    assert_eq!(swept["emailed"], 2, "Ada and Cy, once each: {swept}");

    let mails: Vec<(HeaderMap, Value)> = seen
        .lock()
        .expect("the requests")
        .iter()
        .map(|(_, headers, body)| {
            (
                headers.clone(),
                serde_json::from_slice(body).expect("a JSON mail"),
            )
        })
        .collect();
    let to: Vec<&str> = mails
        .iter()
        .filter_map(|(_, mail)| mail["to"][0].as_str())
        .collect();
    assert_eq!(to.len(), 2, "{to:?}");
    assert!(
        to.contains(&format!("{}@example.test", ada.id).as_str()),
        "{to:?}"
    );
    assert!(to.contains(&format!("{}@example.test", cy.id).as_str()), "{to:?}");
    let (headers, to_ada) = mails
        .iter()
        .find(|(_, mail)| mail["to"][0] == format!("{}@example.test", ada.id).as_str())
        .expect("Ada's mail");
    assert_eq!(headers["authorization"], "Bearer re_test_player_mail");
    // One email lists every game waiting, each with its link.
    assert_eq!(
        to_ada["subject"], "Your move in 2 JackiOh daily games",
        "{to_ada}"
    );
    assert_eq!(
        to_ada["text"]
            .as_str()
            .map(|text| text.matches("- Your game against Bo#2: http").count()),
        Some(2),
        "{to_ada}"
    );
    assert!(
        to_ada["headers"]["List-Unsubscribe"]
            .as_str()
            .is_some_and(|value| value.contains("/api/daily/unsubscribe?token="))
    );
    assert_eq!(
        to_ada["headers"]["List-Unsubscribe-Post"],
        "List-Unsubscribe=One-Click"
    );

    // The gap holds: a second sweep at once sends none.
    let (_, _, again) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(again["emailed"], 0, "{again}");

    // Opening the link changes nothing (RFC 8058 §1: a mail scanner follows every link): its page's
    // button posts the one-click.
    let token = to_ada["headers"]["List-Unsubscribe"]
        .as_str()
        .and_then(|value| value.split("token=").nth(1))
        .map(|token| token.trim_end_matches('>').to_string())
        .expect("a token");
    let link = format!("/api/daily/unsubscribe?token={token}");
    let (status, _headers, page) = call(&app, "GET", &link, None, Value::Null).await;
    assert_eq!(status, 200, "{page}");
    assert!(
        page.as_str().is_some_and(|page| page.contains("method=\"post\"")
            && page.contains("name=\"List-Unsubscribe\" value=\"One-Click\"")),
        "{page}"
    );
    let ada_token = sign_in_again(&app, &ada);
    let (_, settings) = api(&app, "GET", "/api/daily/settings", &ada_token, Value::Null).await;
    assert_eq!(settings["emailOn"], true, "{settings}");

    // RFC 8058's one-click unsubscribe, form-encoded, turns Ada's off.
    let mut one_click = json_request(
        "POST",
        &format!("/api/daily/unsubscribe?token={token}"),
        &Value::Null,
        None,
        None,
    );
    one_click.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    *one_click.body_mut() = axum::body::Body::from("List-Unsubscribe=One-Click");
    let (status, _headers, body) = send_request(&app, one_click).await;
    assert_eq!(status, 200, "{body}");
    let (_, settings) = api(&app, "GET", "/api/daily/settings", &ada_token, Value::Null).await;
    assert_eq!(settings["emailOn"], false, "{settings}");

    // And as §3.1 asks, `multipart/form-data`.
    let (status, settings) = api(
        &app,
        "PUT",
        "/api/daily/settings",
        &ada_token,
        json!({ "emailOn": true }),
    )
    .await;
    assert_eq!((status, &settings["emailOn"]), (200, &json!(true)), "{settings}");
    let mut multipart = json_request("POST", &link, &Value::Null, None, None);
    multipart.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("multipart/form-data; boundary=jackioh"),
    );
    *multipart.body_mut() = axum::body::Body::from(
        "--jackioh\r\nContent-Disposition: form-data; name=\"List-Unsubscribe\"\r\n\r\nOne-Click\r\n--jackioh--\r\n",
    );
    let (status, _headers, body) = send_request(&app, multipart).await;
    assert_eq!(status, 200, "{body}");
    let (_, settings) = api(&app, "GET", "/api/daily/settings", &ada_token, Value::Null).await;
    assert_eq!(settings["emailOn"], false, "{settings}");
}

/// A raw P-256 pair, base64url, as `daily::webpush`'s own tests cut it from a PKCS#8 document.
fn vapid_pair() -> (String, String) {
    let pkcs8 = signature::EcdsaKeyPair::generate_pkcs8(
        &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        &SystemRandom::new(),
    )
    .expect("a document");
    let bytes = pkcs8.as_ref();
    (
        URL_SAFE_NO_PAD.encode(&bytes[36..68]),
        URL_SAFE_NO_PAD.encode(&bytes[bytes.len() - 65..]),
    )
}

/// A browser's subscription keys: its P-256 public key and its 16-byte auth secret.
fn browser_keys() -> (String, String) {
    let rng = SystemRandom::new();
    let private = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng).expect("a key");
    let public = private.compute_public_key().expect("its public key");
    let mut auth = [0_u8; 16];
    rng.fill(&mut auth).expect("random bytes");
    (
        URL_SAFE_NO_PAD.encode(public.as_ref()),
        URL_SAFE_NO_PAD.encode(auth),
    )
}

#[tokio::test]
async fn a_push_subscription_answered_410_is_deleted() {
    let (push_api, seen) = fake_service(StatusCode::GONE).await;
    let app = app_with(
        None,
        TestAppOptions {
            push_keys: Some(vapid_pair()),
            ..TestAppOptions::default()
        },
    )
    .await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;
    let (status, settings) = api(&app, "GET", "/api/daily/settings", &ada.token, Value::Null).await;
    assert_eq!(status, 200, "{settings}");
    assert!(settings["vapidPublicKey"].is_string(), "{settings}");
    for (n, path) in [(1, "gone"), (2, "live")] {
        let (p256dh, auth) = browser_keys();
        let mut tx = app.db.begin(None).await.expect("a transaction");
        tx.push_subs_add(&PushSubRow {
            id: format!("sub-{n}"),
            profile_id: ada.id.clone(),
            endpoint: format!("{push_api}/push/{n}/{path}"),
            p256dh,
            auth,
            created_at: n,
        })
        .await
        .expect("a subscription");
        tx.commit().await.expect("commit");
    }

    // The game starts waiting on Ada: both devices are pushed, and the one gone is deleted.
    daily_game(&app, &bo, &ada, 1).await;
    notifier_idle(&app).await;
    let pushed: Vec<(String, HeaderMap)> = seen
        .lock()
        .expect("the requests")
        .iter()
        .map(|(path, headers, _)| (path.clone(), headers.clone()))
        .collect();
    assert_eq!(pushed.len(), 2, "{pushed:?}");
    for (_, headers) in &pushed {
        assert_eq!(headers["content-encoding"], "aes128gcm");
        assert!(
            headers["authorization"]
                .to_str()
                .is_ok_and(|value| value.starts_with("vapid t="))
        );
    }
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let left: Vec<String> = tx
        .push_subs_of(&ada.id)
        .await
        .expect("a read")
        .into_iter()
        .map(|row| row.endpoint)
        .collect();
    tx.commit().await.expect("commit");
    assert_eq!(left, vec![format!("{push_api}/push/2/live")]);
}

/// The endpoints `who` holds push subscriptions for, oldest first.
async fn push_endpoints(app: &Arc<App>, who: &Account) -> Vec<String> {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let held = tx.push_subs_of(&who.id).await.expect("a read");
    tx.commit().await.expect("commit");
    held.into_iter().map(|row| row.endpoint).collect()
}

/// §9.8: `POST /api/daily/push` keeps an endpoint only at a browser's push service, so a player
/// cannot aim the server's pushes at an address, a local host or a name of their own.
#[tokio::test]
async fn a_push_endpoint_not_at_a_browsers_push_service_is_refused() {
    let app = app_with(None, TestAppOptions::default()).await;
    let ada = person(&app, 1, "Ada").await;
    let (p256dh, auth) = browser_keys();
    let keys = json!({ "p256dh": p256dh, "auth": auth });
    for endpoint in [
        "https://169.254.169.254/latest/meta-data",
        "https://127.0.0.1:8787/daily-sweep",
        "https://localhost/daily-sweep",
        "https://push.example.net/send/abc",
        "https://fcm.googleapis.com.example.net/send/abc",
        "http://fcm.googleapis.com/fcm/send/abc",
    ] {
        let (status, body) = api(
            &app,
            "POST",
            "/api/daily/push",
            &ada.token,
            json!({ "endpoint": endpoint, "keys": keys }),
        )
        .await;
        assert_eq!(
            (status, code_of(&body)),
            (400, "bad_request"),
            "{endpoint}: {body}"
        );
    }
    assert!(push_endpoints(&app, &ada).await.is_empty());

    let browser = "https://fcm.googleapis.com/fcm/send/abc:APA91b";
    let (status, body) = api(
        &app,
        "POST",
        "/api/daily/push",
        &ada.token,
        json!({ "endpoint": browser, "keys": keys }),
    )
    .await;
    assert_eq!(status, 204, "{body}");
    assert_eq!(push_endpoints(&app, &ada).await, vec![browser.to_string()]);
}

/// §9.8: a push follows no redirect, so a push service's answer cannot send it on anywhere else.
#[tokio::test]
async fn a_push_follows_no_redirect() {
    let (elsewhere, seen) = fake_service(StatusCode::GONE).await;
    let landing = format!("{elsewhere}/landed");
    let hops = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&hops);
    let router = Router::new().route(
        "/{*rest}",
        axum::routing::post(move || {
            counted.fetch_add(1, Ordering::SeqCst);
            let landing = landing.clone();
            async move {
                (
                    StatusCode::TEMPORARY_REDIRECT,
                    [(axum::http::header::LOCATION, landing)],
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a local port");
    let hop = format!("http://{}/push/hop", listener.local_addr().expect("its address"));
    tokio::spawn(async move { axum::serve(listener, router).await });

    let app = app_with(
        None,
        TestAppOptions {
            push_keys: Some(vapid_pair()),
            ..TestAppOptions::default()
        },
    )
    .await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;
    let (p256dh, auth) = browser_keys();
    let mut tx = app.db.begin(None).await.expect("a transaction");
    tx.push_subs_add(&PushSubRow {
        id: "sub-hop".to_string(),
        profile_id: ada.id.clone(),
        endpoint: hop.clone(),
        p256dh,
        auth,
        created_at: 1,
    })
    .await
    .expect("a subscription");
    tx.commit().await.expect("commit");

    // The game starts waiting on Ada: her device is pushed, and the redirect it is answered is not
    // followed. A redirect is no 404 or 410, so the subscription stays.
    daily_game(&app, &bo, &ada, 1).await;
    notifier_idle(&app).await;
    assert_eq!(hops.load(Ordering::SeqCst), 1, "the push service is asked");
    assert!(seen.lock().expect("the requests").is_empty());
    assert_eq!(push_endpoints(&app, &ada).await, vec![hop]);
}

/// Adds a push subscription of `who`'s at `endpoint`.
async fn subscribe(app: &Arc<App>, who: &Account, n: i64, endpoint: String) {
    let (p256dh, auth) = browser_keys();
    let mut tx = app.db.begin(None).await.expect("a transaction");
    tx.push_subs_add(&PushSubRow {
        id: format!("sub-{n}"),
        profile_id: who.id.clone(),
        endpoint,
        p256dh,
        auth,
        created_at: n,
    })
    .await
    .expect("a subscription");
    tx.commit().await.expect("commit");
}

/// `who`'s notification settings and when each channel last went.
async fn notify_row(app: &Arc<App>, who: &Account) -> DailyNotifyRow {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let row = tx.daily_notify_get(&who.id).await.expect("a read");
    tx.commit().await.expect("commit");
    row
}

/// §9.13: a move is answered before its email goes, and the email still arrives.
#[tokio::test]
async fn a_move_is_answered_before_its_email_goes_and_the_email_still_arrives() {
    let (mail_api, seen, gate) = held_service(StatusCode::OK).await;
    let app = app_with(
        None,
        TestAppOptions {
            mail_api: Some(format!("{mail_api}/emails")),
            ..TestAppOptions::default()
        },
    )
    .await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;

    // The accept answers with the first mail held; both players owe a mulligan, so both are mailed.
    let match_id = answers_at_once("the accept", daily_game(&app, &ada, &bo, 1)).await;
    eventually("the first mail reached the service", || {
        seen.lock().expect("the requests").len() == 1
    })
    .await;
    // The mulligans and the turn's end each answer while that mail is still held, and the one task
    // sends one mail at a time.
    answers_at_once("the moves", async {
        past_the_mulligan(&app, [&ada, &bo], &match_id).await;
        one_move(&app, [&ada, &bo], &match_id, 1).await;
    })
    .await;
    assert_eq!(seen.lock().expect("the requests").len(), 1);

    gate.add_permits(1);
    notifier_idle(&app).await;
    // Bo's mail, held since the mulligan when both owed a move, and the mail of whoever holds the move
    // now: the task reaches Ada only after the turn has passed, so it decides for the state it finds,
    // and she is mailed only when the move passed to her. The 6 hour gap holds back any repeat.
    let (holding, _) = holder(&app, [&ada, &bo], &match_id).await;
    let mailed = [&bo, holding];
    let mut expected: Vec<String> = mailed
        .iter()
        .map(|who| format!("{}@example.test", who.id))
        .collect();
    expected.sort();
    expected.dedup();
    let mut to: Vec<String> = seen
        .lock()
        .expect("the requests")
        .iter()
        .map(|(_, _, body)| {
            let mail: Value = serde_json::from_slice(body).expect("a JSON mail");
            mail["to"][0].as_str().unwrap_or_default().to_string()
        })
        .collect();
    to.sort();
    assert_eq!(to, expected);
    for who in mailed {
        assert!(notify_row(&app, who).await.last_email_at.is_some());
    }
}

/// §9.13: with the push service failing, the player a game waits on is not held up by it on any
/// action: each used to push inline, since a failed push is never marked.
#[tokio::test]
async fn a_failing_push_service_never_holds_the_actions_of_the_player_it_waits_on() {
    let (push_api, seen, gate) = held_service(StatusCode::INTERNAL_SERVER_ERROR).await;
    let app = app_with(
        None,
        TestAppOptions {
            push_keys: Some(vapid_pair()),
            ..TestAppOptions::default()
        },
    )
    .await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;
    subscribe(&app, &ada, 1, format!("{push_api}/push/1/ada")).await;

    // The first game starts waiting on Ada, and her push is held. Each start answers at once.
    answers_at_once("the first accept", daily_game(&app, &bo, &ada, 1)).await;
    eventually("Ada's push reached the service", || {
        seen.lock().expect("the requests").len() == 1
    })
    .await;
    let second = answers_at_once("the second accept", daily_game(&app, &bo, &ada, 3)).await;
    let third = answers_at_once("the third accept", daily_game(&app, &bo, &ada, 7)).await;
    // The first game still waits on Ada, so her push stays due at each action in the others.
    answers_at_once("Ada's first action", one_move(&app, [&ada, &bo], &second, 1)).await;
    answers_at_once("Ada's second action", one_move(&app, [&ada, &bo], &third, 2)).await;

    gate.add_permits(1);
    notifier_idle(&app).await;
    // A push the service refused is not marked, so the sweep sends it.
    assert_eq!(notify_row(&app, &ada).await.last_push_at, None);
}

/// §9.13: a profile waiting for the notifier's task is not queued again, so two moves before the
/// task reaches it decide for it once.
#[tokio::test]
async fn a_profile_queued_twice_before_the_task_reaches_it_is_notified_once() {
    let (push_api, seen, gate) = held_service(StatusCode::INTERNAL_SERVER_ERROR).await;
    let app = app_with(
        None,
        TestAppOptions {
            push_keys: Some(vapid_pair()),
            ..TestAppOptions::default()
        },
    )
    .await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;
    subscribe(&app, &ada, 1, format!("{push_api}/push/1/ada")).await;

    // The accept queues Ada and Bo; the task takes Ada, whose push is held, and Bo waits.
    let first = answers_at_once("the first accept", daily_game(&app, &bo, &ada, 1)).await;
    eventually("Ada's push reached the service", || {
        seen.lock().expect("the requests").len() == 1
    })
    .await;
    // Bo's move queues Ada again (the task has taken her off) and not Bo, who still waits; the
    // second game's start and its move queue neither.
    answers_at_once("Bo's move", one_move(&app, [&bo, &ada], &first, 1)).await;
    let second = answers_at_once("the second accept", daily_game(&app, &bo, &ada, 3)).await;
    answers_at_once("Bo's second move", one_move(&app, [&bo, &ada], &second, 2)).await;

    gate.add_permits(1);
    notifier_idle(&app).await;
    // The push that was held and one decision for Ada, which still has games waiting on her; each of
    // the four moves queueing her afresh would have made four.
    assert_eq!(seen.lock().expect("the requests").len(), 2);
}

#[tokio::test]
async fn the_social_read_counts_the_daily_games_waiting_on_you() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let (on_move, other) = holder(&app, [&ada, &bo], &match_id).await;
    let (_, mine) = api(&app, "GET", "/api/social", &on_move.token, Value::Null).await;
    let (_, theirs) = api(&app, "GET", "/api/social", &other.token, Value::Null).await;
    assert_eq!(
        (mine["dailyWaiting"].clone(), theirs["dailyWaiting"].clone()),
        (json!(1), json!(0))
    );
    // Past its deadline the move is lost, so it waits on nobody, sweep or no sweep.
    app.daily.set_offset(MS_PER_DAY);
    let (_, mine) = api(&app, "GET", "/api/social", &on_move.token, Value::Null).await;
    assert_eq!(mine["dailyWaiting"], 0);
}
