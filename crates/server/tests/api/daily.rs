//! Daily games (SPEC §9.13, #3727): a game played a move at a time over plain HTTPS, with nothing in
//! memory between moves, lost on time (R3727000), holding no seat (R3727004), pinned to the catalog it
//! started on (R3727005), its open challenges cancelled by a block (R3727007), holding its accounts
//! from deletion while it runs (R3727008), and "your move" sent by email and Web Push, to a browser's
//! push service only. A finished game whose moves the retention purge took opens as its stored result
//! and refuses every move (R3934000). A finished game stays for the other player when one of its
//! players deletes their account, naming that opponent by nobody (R4621000).

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

use jackioh_engine::config::RANDOM_ATTACK_TARGET;
use jackioh_engine::testkit::{clear_overrides, register_catalog, register_shipped};
use jackioh_engine::{CardCost, CardDefs, ShippedCatalog};
use jackioh_server::api::catalog::{LoadCatalogOptions, load_catalog};
use jackioh_server::api::queue::try_pair;
use jackioh_server::api::retention::purge_expired;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    DAILY_MAX_ACTIVE_GAMES, DAILY_PLAYER_EMAILS_PER_DAY, MATCH_ACTION_RETENTION_DAYS, MS_PER_DAY,
};
use jackioh_server::daily::notify::sweep;
use jackioh_server::db::store::{DailyNotifyRow, MatchStatus, ProfileStatus, PushSubRow};
use jackioh_shared::economy::rewards::utc_day_bounds;

use crate::support::deps::{
    TestAppOptions, add_user, call, json_request, record_logs, send as send_request, test_app_with,
};
use crate::support::engine::{TEST_GLITCH_VOID, fake_deck, install_test_cards};
use crate::support::store::{
    Account, account, befriend, fail_on, fake, fake_lock, record_calls, stop_failing,
};

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

/// Whether `GET /api/daily` lists `match_id` for `who` as holding the move. The list marks
/// nothing, unlike opening the board (R4233000), so tests read it to see a move pass without
/// dealing with what it is about.
async fn list_has_move(app: &Arc<App>, who: &Account, match_id: &str) -> bool {
    let (status, list) = api(app, "GET", "/api/daily", &who.token, Value::Null).await;
    assert_eq!(status, 200, "{list}");
    list["games"]
        .as_array()
        .expect("games")
        .iter()
        .find(|game| game["matchId"] == match_id)
        .expect("the game is listed")["hasMove"]
        == true
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

/// The attacks of `ids` the forecast has an entry for: every one but those whose target a Mayor draws
/// (R1200), which carry the sentinel target and are not forecast (R4188001).
fn forecast_of_attacks(ids: Vec<(String, String)>) -> Vec<(String, String)> {
    ids.into_iter()
        .filter(|(_, target_id)| target_id != RANDOM_ATTACK_TARGET)
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
/// one is legal and ends its turn when none is, until a seat has an attack to forecast. A Mayor dealt
/// onto the field there draws every attack's target (R1200), and such an attack has no entry (R4188001).
/// A deal can end its game before either seat has an attack (Olivian Expanse's KYS Last Exam on turn
/// 2), and then the test deals another.
#[tokio::test]
async fn r3855000_a_daily_read_carries_the_forecast_of_its_seats_legal_attacks() {
    /// More than enough turns for a unit to be played and to be old enough to attack.
    const MOVES_TO_AN_ATTACK: usize = 60;
    /// More deals than random decks ever end early in a row.
    const DEALS: usize = 5;
    let (app, ada, bo) = two_friends().await;
    let mut match_id = daily_game(&app, &ada, &bo, 1).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;

    let (mut found, mut deals, mut moves) = (None, 1, 0);
    for n in 0.. {
        let (on_move, waiting) = holder(&app, [&ada, &bo], &match_id).await;
        let game = read(&app, on_move, &match_id).await;
        if game["over"] == true {
            assert!(deals < DEALS, "{DEALS} deals ended before an attack: {game}");
            (deals, moves) = (deals + 1, 0);
            match_id = daily_game(&app, &ada, &bo, 1).await;
            past_the_mulligan(&app, [&ada, &bo], &match_id).await;
            continue;
        }
        if legal_has(&game, "attack") {
            found = Some((on_move, waiting, game));
            break;
        }
        if moves == MOVES_TO_AN_ATTACK {
            break;
        }
        moves += 1;
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
    assert_eq!(forecast_ids(&game), forecast_of_attacks(attacks), "{game}");
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
    assert_eq!(
        forecast_ids(&body),
        forecast_of_attacks(legal_attack_ids(&body)),
        "{body}"
    );
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

// R4390000: a lapsed invite is deleted whatever became of it; the game it started plays on

#[tokio::test]
async fn r4390000_the_sweep_deletes_lapsed_invites_and_the_game_an_accept_started_plays_on() {
    let (app, ada, bo) = two_friends().await;
    // 14 days a move: its deadline outlasts the invites' week.
    let match_id = daily_game(&app, &ada, &bo, 14).await;
    let (status, cancelled) = api(
        &app,
        "POST",
        "/api/daily/challenges",
        &ada.token,
        random_challenge(&bo, 3),
    )
    .await;
    assert_eq!(status, 201, "{cancelled}");
    let (status, body) = api(
        &app,
        "DELETE",
        &format!(
            "/api/daily/challenges/{}",
            cancelled["id"].as_str().expect("a challenge id")
        ),
        &ada.token,
        Value::Null,
    )
    .await;
    assert_eq!(status, 204, "{body}");
    let (status, seek) = api(
        &app,
        "POST",
        "/api/daily/seeks",
        &ada.token,
        json!({ "mode": "random", "perMoveDays": 3 }),
    )
    .await;
    assert_eq!(status, 201, "{seek}");
    // The accepted challenge, the cancelled one and the open seek.
    assert_eq!(fake_lock(&app).await.tables.daily.invites.len(), 3);

    let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(status, 200, "{swept}");
    assert_eq!(swept["purged"], 0, "none has lapsed yet: {swept}");
    assert_eq!(fake_lock(&app).await.tables.daily.invites.len(), 3);

    app.daily
        .set_offset(jackioh_server::config::DAILY_CHALLENGE_TTL_DAYS * MS_PER_DAY + 1);
    let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
    assert_eq!(status, 200, "{swept}");
    assert_eq!(swept["expired"], 1, "the open seek: {swept}");
    assert_eq!(swept["purged"], 3, "{swept}");
    assert_eq!(swept["settled"], 0, "the game's clock has days left: {swept}");
    assert!(fake_lock(&app).await.tables.daily.invites.is_empty());

    // The game the accepted challenge started opens, plays and ends.
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let (on_move, other) = holder(&app, [&ada, &bo], &match_id).await;
    let game = read(&app, on_move, &match_id).await;
    assert_eq!(game["over"], false, "{game}");
    let (status, body) = send(
        &app,
        on_move,
        &match_id,
        game["seq"].as_i64().expect("a seq"),
        "concede",
        json!({ "type": "concede" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["over"], true, "{body}");
    assert_eq!(read(&app, other, &match_id).await["over"], true);
}

// R3934000: a finished game whose moves the retention purge took

/// Dates the end of `match_id` past the retention window and runs the purge: how many log rows it took.
/// `finished_at` moves, not the clock, because the purge reads the wall clock, which the daily offset
/// leaves alone.
async fn purge_its_log(app: &Arc<App>, match_id: &str) -> i64 {
    {
        let mut data = fake_lock(app).await;
        let row = data
            .tables
            .matches
            .iter_mut()
            .find(|row| row.id == match_id)
            .expect("the match row");
        row.finished_at = Some(now_ms() - (MATCH_ACTION_RETENTION_DAYS + 1) * MS_PER_DAY);
    }
    purge_expired(app).await.expect("the purge runs").match_actions
}

/// How many log rows `match_id` has in the store.
async fn log_rows(app: &Arc<App>, match_id: &str) -> usize {
    fake_lock(app)
        .await
        .tables
        .match_actions
        .iter()
        .filter(|row| row.match_id == match_id)
        .count()
}

#[tokio::test]
async fn r3934000_a_finished_game_whose_moves_were_purged_opens_as_its_result_and_refuses_every_move() {
    let (app, ada, bo) = two_friends().await;
    let cy = person(&app, 3, "Cy").await;
    let match_id = daily_game(&app, &ada, &bo, 3).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let seq = read(&app, &ada, &match_id).await["seq"].as_i64().expect("a seq");
    let (status, body) = send(
        &app,
        &ada,
        &match_id,
        seq,
        "concede",
        json!({ "type": "concede" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["over"], true, "{body}");
    assert_eq!(match_status(&app, &match_id).await, MatchStatus::Finished);

    // Inside the window the game reads and plays from its log, as before.
    let inside = read(&app, &bo, &match_id).await;
    assert_eq!(inside["over"], true, "{inside}");
    assert!(inside["view"].is_object(), "{inside}");
    assert!(inside.get("expired").is_none(), "{inside}");
    let (_, list) = api(&app, "GET", "/api/daily", &bo.token, Value::Null).await;
    let row = list["games"][0].clone();
    assert_eq!(row["matchId"], match_id.as_str(), "{list}");

    assert!(purge_its_log(&app, &match_id).await > 0, "the purge took the log");
    assert_eq!(log_rows(&app, &match_id).await, 0);

    // Each player's read is the result: over, expired, no board.
    for (who, outcome) in [(&ada, "loss"), (&bo, "win")] {
        let game = read(&app, who, &match_id).await;
        assert_eq!(game["over"], true, "{game}");
        assert_eq!(game["expired"], true, "{game}");
        assert_eq!(game["result"]["outcome"], outcome, "{game}");
        assert_eq!(game["result"]["reason"], "concede", "{game}");
        for hidden in ["view", "legal", "forecast"] {
            assert!(game.get(hidden).is_none(), "{hidden} in {game}");
        }
    }
    // It is the list's row, from the stored board.
    let game = read(&app, &bo, &match_id).await;
    for key in [
        "matchId",
        "seat",
        "opponent",
        "turn",
        "life",
        "result",
        "endedAt",
        "mode",
        "perMoveDays",
    ] {
        assert_eq!(game[key], row[key], "{key}: {game} against {row}");
    }

    // A move, from either seat, is refused as over, never as stale, and writes nothing.
    for (who, nonce, action) in [
        (&ada, "after-purge-a", json!({ "type": "endTurn" })),
        (&bo, "after-purge-b", json!({ "type": "concede" })),
    ] {
        let (status, body) = send(&app, who, &match_id, seq + 1, nonce, action).await;
        assert_eq!((status, code_of(&body)), (409, "over"), "{body}");
        assert_eq!(body["game"]["expired"], true, "{body}");
        assert_eq!(body["game"]["over"], true, "{body}");
    }
    assert_eq!(log_rows(&app, &match_id).await, 0);

    // It says nothing to an account that does not play it.
    let (status, body) = api(
        &app,
        "GET",
        &format!("/api/daily/games/{match_id}"),
        &cy.token,
        Value::Null,
    )
    .await;
    assert_eq!(status, 404, "{body}");

    // The list still shows it with its result.
    let (status, list) = api(&app, "GET", "/api/daily", &ada.token, Value::Null).await;
    assert_eq!(status, 200, "{list}");
    assert_eq!(list["games"][0]["over"], true, "{list}");
    assert_eq!(list["games"][0]["result"]["outcome"], "loss", "{list}");
}

#[tokio::test]
async fn r3934000_a_game_lost_on_time_whose_moves_were_purged_opens_as_won_on_time_and_answers_time_up() {
    let (app, ada, bo) = two_friends().await;
    let match_id = daily_game(&app, &ada, &bo, 1).await;
    past_the_mulligan(&app, [&ada, &bo], &match_id).await;
    let (late, waiting) = holder(&app, [&ada, &bo], &match_id).await;
    app.daily.set_offset(MS_PER_DAY);
    let settled = read(&app, waiting, &match_id).await;
    assert_eq!(settled["lostOnTime"], true, "{settled}");
    let seq = settled["seq"].as_i64().expect("a seq");

    assert!(purge_its_log(&app, &match_id).await > 0, "the purge took the log");

    let game = read(&app, waiting, &match_id).await;
    assert_eq!(game["expired"], true, "{game}");
    assert_eq!(
        game["result"],
        json!({ "outcome": "win", "reason": "disconnect" }),
        "{game}"
    );
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
    assert_eq!(body["game"]["expired"], true, "{body}");
    assert_eq!(log_rows(&app, &match_id).await, 0);
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

/// R3727004, #3842: a rival send taking the account's last place between the early count and the
/// write (here, written as the write's transaction takes the account's daily lock) leaves the
/// challenge, the accept, the waiting seek and the pairing seek refused `daily_full`, the account at
/// the cap. An accept refused so leaves its challenge open, and a pairing seek refused so puts the
/// seek it claimed back.
#[tokio::test]
async fn r3727004_a_challenge_an_accept_or_a_seek_whose_rival_took_the_last_place_is_refused() {
    use jackioh_server::db::fake::{CallOutcome, FakeTables};
    use jackioh_server::db::store::{DailyInviteKind, DailyInviteRow, DailyInviteState, QueueMode};
    use std::sync::atomic::AtomicBool;

    let seek = json!({ "mode": "random", "perMoveDays": 1 });
    for case in ["challenge", "accept", "seek", "pair"] {
        let (app, ada, bo) = two_friends().await;
        let cy = person(&app, 3, "Cy").await;
        for _ in 1..DAILY_MAX_ACTIVE_GAMES {
            daily_game(&app, &ada, &bo, 14).await;
        }
        let mut challenge_id = String::new();
        if case == "accept" {
            let (status, body) = api(
                &app,
                "POST",
                "/api/daily/challenges",
                &bo.token,
                random_challenge(&ada, 14),
            )
            .await;
            assert_eq!(status, 201, "{case}: {body}");
            challenge_id = body["id"].as_str().expect("a challenge id").to_string();
        }
        if case == "pair" {
            let (status, body) = api(&app, "POST", "/api/daily/seeks", &cy.token, seek.clone()).await;
            assert_eq!(status, 201, "{case}: {body}");
        }

        // Ada's rival send, committed just before her request's write takes the lock.
        let rival = DailyInviteRow {
            id: "00000000-0000-4000-8000-000000003842".into(),
            kind: DailyInviteKind::Seek,
            from_profile: ada.id.clone(),
            to_profile: None,
            mode: QueueMode::Bo1,
            per_move_days: 7,
            choice: json!({ "mode": "random" }),
            state: DailyInviteState::Open,
            match_id: None,
            created_at: 0,
            expires_at: app.daily.now() + MS_PER_DAY,
        };
        let fired = Arc::new(AtomicBool::new(false));
        let once = Arc::clone(&fired);
        fake(&app).lock().await.before_call = Some(Arc::new(move |method: &str, tables: &mut FakeTables| {
            if method == "daily.lock" && !once.swap(true, Ordering::SeqCst) {
                tables.daily.invites.push(rival.clone());
            }
            CallOutcome::Proceed
        }));
        let (status, body) = match case {
            "challenge" => {
                api(
                    &app,
                    "POST",
                    "/api/daily/challenges",
                    &ada.token,
                    random_challenge(&bo, 14),
                )
                .await
            }
            "accept" => {
                api(
                    &app,
                    "POST",
                    &format!("/api/daily/challenges/{challenge_id}/accept"),
                    &ada.token,
                    json!({ "mode": "random" }),
                )
                .await
            }
            _ => api(&app, "POST", "/api/daily/seeks", &ada.token, seek.clone()).await,
        };
        fake(&app).lock().await.before_call = None;
        assert_eq!((status, code_of(&body)), (409, "daily_full"), "{case}: {body}");
        assert!(fired.load(Ordering::SeqCst), "{case}: the write took the lock");

        let mut tx = app.db.begin(None).await.expect("a transaction");
        let held = tx
            .daily_active_count(ada.id.as_str(), app.daily.now())
            .await
            .expect("a count");
        tx.commit().await.expect("commit");
        assert_eq!(
            held,
            i64::from(DAILY_MAX_ACTIVE_GAMES),
            "{case}: Ada holds the cap, no more"
        );
        if case == "accept" {
            let (_, list) = api(&app, "GET", "/api/daily", &ada.token, Value::Null).await;
            assert_eq!(
                list["challenges"]["incoming"].as_array().map(Vec::len),
                Some(1),
                "{case}: the challenge is still open: {list}"
            );
        }
        if case == "pair" {
            let (_, list) = api(&app, "GET", "/api/daily", &cy.token, Value::Null).await;
            assert_eq!(
                list["seeks"].as_array().map(Vec::len),
                Some(1),
                "{case}: Cy's seek is back: {list}"
            );
            assert_eq!(list["games"], json!([]), "{case}: {list}");
        }
    }
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

// R4019000: a banned challenger's open challenge cannot be accepted and leaves the inbox;
// unbanned, it accepts again.

#[tokio::test]
async fn r4019000_a_banned_challengers_challenge_cannot_be_accepted() {
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

    {
        let mut tx = app.db.begin(Some(&ada.id)).await.expect("a transaction");
        tx.profiles_set_status(&ada.id, ProfileStatus::Banned)
            .await
            .expect("the ban");
        tx.commit().await.expect("commit");
    }

    // The accept answers 404 as to a non-friend and writes no daily game ...
    let (status, body) = api(
        &app,
        "POST",
        &format!("/api/daily/challenges/{id}/accept"),
        &bo.token,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!((status, code_of(&body)), (404, "not_found"), "{body}");
    let (_, list) = api(&app, "GET", "/api/daily", &bo.token, Value::Null).await;
    assert_eq!(list["games"], json!([]), "{list}");
    assert_eq!(
        list["challenges"],
        json!({ "incoming": [], "outgoing": [] }),
        "{list}"
    );

    // ... and the challenge accepts again once the ban is lifted.
    {
        let mut tx = app.db.begin(Some(&ada.id)).await.expect("a transaction");
        tx.profiles_set_status(&ada.id, ProfileStatus::Active)
            .await
            .expect("the unban");
        tx.commit().await.expect("commit");
    }
    let (status, body) = api(
        &app,
        "POST",
        &format!("/api/daily/challenges/{id}/accept"),
        &bo.token,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    assert!(body["matchId"].is_string(), "{body}");
}

// R4019000: a seek posted while a banned account's matching seek waits never pairs with it;
// the banned seek pairs again once the ban is lifted.

#[tokio::test]
async fn r4019000_a_seek_never_pairs_with_a_banned_seek() {
    let app = app_with(None, TestAppOptions::default()).await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    let cy = person(&app, 3, "Cy").await;
    let seek = json!({ "mode": "random", "perMoveDays": 3 });

    // Ada's seek waits; banning her leaves it open but unpairable.
    let (status, body) = api(&app, "POST", "/api/daily/seeks", &ada.token, seek.clone()).await;
    assert_eq!(status, 201, "{body}");
    assert!(body["seekId"].is_string(), "{body}");
    {
        let mut tx = app.db.begin(Some(&ada.id)).await.expect("a transaction");
        tx.profiles_set_status(&ada.id, ProfileStatus::Banned)
            .await
            .expect("the ban");
        tx.commit().await.expect("commit");
    }

    // Bo's matching seek waits beside it instead of pairing with Ada's.
    let (status, body) = api(&app, "POST", "/api/daily/seeks", &bo.token, seek.clone()).await;
    assert_eq!(status, 201, "{body}");
    assert!(body["seekId"].is_string(), "no banned pairing: {body}");

    // Cy pairs with Bo's seek, never Ada's.
    let (status, body) = api(&app, "POST", "/api/daily/seeks", &cy.token, seek.clone()).await;
    assert_eq!(status, 201, "{body}");
    assert!(body["matchId"].is_string(), "{body}");

    // Unbanned, Ada's still-open seek pairs with the next matching seek.
    {
        let mut tx = app.db.begin(Some(&ada.id)).await.expect("a transaction");
        tx.profiles_set_status(&ada.id, ProfileStatus::Active)
            .await
            .expect("the unban");
        tx.commit().await.expect("commit");
    }
    let dan = person(&app, 4, "Dan").await;
    let (status, body) = api(&app, "POST", "/api/daily/seeks", &dan.token, seek).await;
    assert_eq!(status, 201, "{body}");
    assert!(body["matchId"].is_string(), "the unbanned seek pairs: {body}");
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

// R4621000: a finished daily game stays for the other player when one deletes their account

#[tokio::test]
async fn r4621000_a_finished_daily_game_stays_for_the_other_player_when_one_deletes_their_account() {
    let gone = json!({ "profileId": "", "username": "" });
    for purged in [false, true] {
        let (app, ada, bo) = two_friends().await;
        let match_id = daily_game(&app, &ada, &bo, 3).await;
        past_the_mulligan(&app, [&ada, &bo], &match_id).await;
        let seq = read(&app, &ada, &match_id).await["seq"].as_i64().expect("a seq");
        let (status, body) = send(
            &app,
            &ada,
            &match_id,
            seq,
            "concede",
            json!({ "type": "concede" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(match_status(&app, &match_id).await, MatchStatus::Finished);

        let (status, body) = api(&app, "DELETE", "/api/account", &ada.token, Value::Null).await;
        assert_eq!(status, 204, "purged {purged}: {body}");
        if purged {
            assert!(purge_its_log(&app, &match_id).await > 0, "the purge took the log");
        }

        // Bo's list still holds the game, its opponent named by nobody.
        let (status, list) = api(&app, "GET", "/api/daily", &bo.token, Value::Null).await;
        assert_eq!(status, 200, "purged {purged}: {list}");
        let row = &list["games"][0];
        assert_eq!(row["matchId"], match_id.as_str(), "purged {purged}: {list}");
        assert_eq!(row["over"], true, "purged {purged}: {list}");
        assert_eq!(row["result"]["outcome"], "win", "purged {purged}: {list}");
        assert_eq!(row["opponent"], gone, "purged {purged}: {list}");

        // Its board opens with the result, as before.
        let game = read(&app, &bo, &match_id).await;
        assert_eq!(game["opponent"], gone, "purged {purged}: {game}");
        assert_eq!(game["over"], true, "purged {purged}: {game}");
        if purged {
            assert_eq!(game["expired"], true, "{game}");
            assert_eq!(game["result"]["outcome"], "win", "{game}");
        } else {
            assert!(game["view"].is_object(), "{game}");
            assert!(!game["view"]["result"].is_null(), "{game}");
            assert!(game.get("expired").is_none(), "{game}");
        }

        // A move is refused as over, the game it answers naming nobody either.
        let (status, body) = send(
            &app,
            &bo,
            &match_id,
            seq + 1,
            "after-the-delete",
            json!({ "type": "endTurn" }),
        )
        .await;
        assert_eq!((status, code_of(&body)), (409, "over"), "purged {purged}: {body}");
        assert_eq!(body["game"]["opponent"], gone, "purged {purged}: {body}");

        // R768: its replay still opens from Bo's seat while its log is kept.
        if !purged {
            let (status, replay) = api(
                &app,
                "GET",
                &format!("/api/replays/{match_id}"),
                &bo.token,
                Value::Null,
            )
            .await;
            assert_eq!(status, 200, "{replay}");
        }

        // No rematch against an account that is gone: it is nobody's friend now.
        let (status, body) = api(
            &app,
            "POST",
            "/api/daily/challenges",
            &bo.token,
            random_challenge(&ada, 3),
        )
        .await;
        assert_eq!(status, 404, "purged {purged}: {body}");
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

/// #4016 (item 17 of #3897: template injection): the daily mail sends no HTML part, and the
/// unsubscribe page's body is one of its fixed strings whatever `token` it is sent, `<script>`
/// included. The mail's game lines carry the opponent's username and the match id as plain text
/// (§9.4, R1432), so there is nothing to escape; the unsubscribe handler UUID-parses the token
/// before it reaches the page, so a hostile token never reaches it either.
#[tokio::test]
async fn daily_email_has_no_html_part_and_unsubscribe_page_reflects_no_token() {
    let (quiet, ada, bo) = two_friends().await;
    let match_id = daily_game(&quiet, &ada, &bo, 1).await;

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
    assert!(swept["emailed"].as_u64().unwrap_or_default() >= 1, "{swept}");

    let mails: Vec<Value> = seen
        .lock()
        .expect("the requests")
        .iter()
        .map(|(_, _, body)| serde_json::from_slice(body).expect("a JSON mail"))
        .collect();
    // No HTML part is ever sent: the mail is plain text only. If an `html` part is ever added,
    // this test must grow escaping asserts for the opponent username and the match id instead of
    // dropping this assert.
    for mail in &mails {
        assert!(mail.get("html").is_none(), "{mail}");
    }
    let text = mails
        .iter()
        .filter_map(|mail| mail["text"].as_str())
        .find(|text| text.contains(match_id.as_str()))
        .expect("a mail listing the game");
    assert!(text.contains("- Your game against "), "{text}");

    let page_of = |body: &str| {
        format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>JackiOh</title></head><body>{body}</body></html>"
        )
    };
    let not_ours = page_of(
        "<p>That link is not one of ours any more. You can turn daily game emails off from the Daily screen.</p>",
    );
    // An empty, a garbage and a markup token all get exactly the fixed not-ours page.
    for token in [
        String::new(),
        "not-a-uuid".to_string(),
        "%3Cscript%3Ealert(1)%3C%2Fscript%3E".to_string(),
    ] {
        let (status, _headers, body) = call(
            &app,
            "GET",
            &format!("/api/daily/unsubscribe?token={token}"),
            None,
            Value::Null,
        )
        .await;
        assert_eq!(status, 200, "{body}");
        let body = body.as_str().expect("an HTML page");
        assert_eq!(body, not_ours, "{body}");
        assert!(!body.contains("alert(1)"), "{body}");
        assert!(!body.contains("<script>"), "{body}");
    }

    // A well-formed but unknown token is echoed only inside the form's action URL.
    let token = uuid::Uuid::new_v4().to_string();
    let (status, _headers, body) = call(
        &app,
        "GET",
        &format!("/api/daily/unsubscribe?token={token}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body.as_str().expect("an HTML page"),
        page_of(&format!(
            "<p>Stop the emails about your daily games?</p>\
             <form method=\"post\" action=\"/api/daily/unsubscribe?token={token}\">\
             <input type=\"hidden\" name=\"List-Unsubscribe\" value=\"One-Click\">\
             <button type=\"submit\">Turn off daily game emails</button></form>"
        )),
        "{body}"
    );

    // Posting a hostile token's link unsubscribes nobody: exactly the not-ours page.
    let (status, _headers, body) = call(
        &app,
        "POST",
        "/api/daily/unsubscribe?token=%3Cscript%3Ealert(1)%3C%2Fscript%3E",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body.as_str().expect("an HTML page"), not_ours, "{body}");
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

/// A device whose push service answers `status`, and one that answers 201, are pushed ("your move").
/// Returns the service's base URL and the endpoints left, oldest first.
async fn push_answered(status: StatusCode) -> (String, Vec<String>) {
    let (push_api, seen) = fake_service(status).await;
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

    // The game starts waiting on Ada: both devices are pushed.
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
    let left = push_endpoints(&app, &ada).await;
    (push_api, left)
}

/// §9.13: a subscription its push service answers 410 is gone, and its row is deleted.
#[tokio::test]
async fn a_push_subscription_answered_410_is_deleted() {
    let (push_api, left) = push_answered(StatusCode::GONE).await;
    assert_eq!(left, vec![format!("{push_api}/push/2/live")]);
}

/// §9.13, #3844: a subscription its push service answers 403, as it does after the VAPID keys
/// change, is deleted too, not retried every sweep.
#[tokio::test]
async fn a_push_subscription_answered_403_is_deleted() {
    let (push_api, left) = push_answered(StatusCode::FORBIDDEN).await;
    assert_eq!(left, vec![format!("{push_api}/push/2/live")]);
}

/// §9.13: a subscription its push service answers 404 is gone, and its row is deleted.
#[tokio::test]
async fn a_push_subscription_answered_404_is_deleted() {
    let (push_api, left) = push_answered(StatusCode::NOT_FOUND).await;
    assert_eq!(left, vec![format!("{push_api}/push/2/live")]);
}

/// §9.13, #3844: a throttled push (429) is retried by the next sweep, so no row goes: only 404, 410
/// and 403 delete.
#[tokio::test]
async fn a_push_subscription_answered_429_is_kept() {
    let (push_api, left) = push_answered(StatusCode::TOO_MANY_REQUESTS).await;
    assert_eq!(
        left,
        vec![
            format!("{push_api}/push/1/gone"),
            format!("{push_api}/push/2/live")
        ]
    );
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

/// Takes `n` of the player emails of the UTC day starting at `day`, as that many sends would (#3843).
async fn spend_emails(app: &Arc<App>, day: i64, n: i32) {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    for k in 1..=n {
        assert!(
            tx.daily_email_claim(day, DAILY_PLAYER_EMAILS_PER_DAY)
                .await
                .expect("a claim"),
            "email {k} of {n} was free"
        );
    }
    tx.commit().await.expect("commit");
}

/// How many of `seen`'s requests went to `path`.
fn requests_to(seen: &Requests, path: &str) -> usize {
    seen.lock()
        .expect("the requests")
        .iter()
        .filter(|(at, _, _)| at == path)
        .count()
}

/// §9.13, #3843: once the UTC day's `DAILY_PLAYER_EMAILS_PER_DAY` are spent, a restarted server's
/// sweep sends no email and still pushes, logs the cap once and not once a player, and marks no email
/// sent, so the next day's sweep sends both.
#[tokio::test]
async fn past_the_days_player_email_cap_the_sweep_still_pushes_and_the_next_day_emails() {
    let (quiet, ada, bo) = two_friends().await;
    // Both owe the mulligan, so the game waits on both.
    daily_game(&quiet, &ada, &bo, 3).await;
    notifier_idle(&quiet).await;
    let (service, seen) = fake_service(StatusCode::OK).await;
    subscribe(&quiet, &ada, 1, format!("{service}/push/1/ada")).await;
    subscribe(&quiet, &bo, 2, format!("{service}/push/2/bo")).await;
    let now = quiet.daily.now();
    let today = utc_day_bounds(now).0;
    spend_emails(&quiet, today, DAILY_PLAYER_EMAILS_PER_DAY).await;

    // A restart over the spent count: the cap is the store's, not the process's.
    let app = app_with(
        Some(quiet.db.clone()),
        TestAppOptions {
            mail_api: Some(format!("{service}/emails")),
            push_keys: Some(vapid_pair()),
            ..TestAppOptions::default()
        },
    )
    .await;
    let logs = record_logs();
    let swept = sweep(&app, now).await.expect("a sweep");
    assert_eq!((swept.emailed, swept.pushed), (0, 2));
    assert_eq!(requests_to(&seen, "/emails"), 0);
    assert_eq!(logs.named("daily.email.capped").len(), 1, "{:?}", logs.entries());
    for who in [&ada, &bo] {
        let row = notify_row(&app, who).await;
        assert_eq!((row.last_email_at, row.last_push_at), (None, Some(now)));
    }

    // The next UTC day has its own count: the emails held back go, and the pushes, sent, wait.
    let tomorrow = now + MS_PER_DAY;
    let swept = sweep(&app, tomorrow).await.expect("a sweep");
    assert_eq!((swept.emailed, swept.pushed), (2, 0));
    assert_eq!(requests_to(&seen, "/emails"), 2);
    for who in [&ada, &bo] {
        assert_eq!(notify_row(&app, who).await.last_email_at, Some(tomorrow));
    }
    assert_eq!(logs.named("daily.email.capped").len(), 1);
}

/// §9.13, #3843: the after-move task and the sweep take from one count: with one email of the day
/// left, a start that waits on both players emails one, and the sweep after it emails nobody.
#[tokio::test]
async fn the_after_move_queue_and_the_sweep_share_the_days_player_email_cap() {
    let (service, seen) = fake_service(StatusCode::OK).await;
    let app = app_with(
        None,
        TestAppOptions {
            mail_api: Some(format!("{service}/emails")),
            ..TestAppOptions::default()
        },
    )
    .await;
    // Noon UTC, so the day cannot turn between the start and the sweep.
    let real = app.daily.now();
    app.daily
        .set_offset(utc_day_bounds(real).0 + MS_PER_DAY / 2 - real);
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;
    spend_emails(
        &app,
        utc_day_bounds(app.daily.now()).0,
        DAILY_PLAYER_EMAILS_PER_DAY - 1,
    )
    .await;

    // Both owe the mulligan, so the start queues both, and only one email of the day is left.
    daily_game(&app, &ada, &bo, 3).await;
    notifier_idle(&app).await;
    assert_eq!(seen.lock().expect("the requests").len(), 1);
    let mut emailed = 0;
    for who in [&ada, &bo] {
        emailed += usize::from(notify_row(&app, who).await.last_email_at.is_some());
    }
    assert_eq!(emailed, 1);

    // The one the queue held back is not emailed by the sweep either.
    let swept = sweep(&app, app.daily.now()).await.expect("a sweep");
    assert_eq!(swept.emailed, 0);
    assert_eq!(seen.lock().expect("the requests").len(), 1);
}

/// §9.13, #3843: an email the mail service refused gives its place back, so an outage spends none of
/// the day's `DAILY_PLAYER_EMAILS_PER_DAY`.
#[tokio::test]
async fn an_email_the_mail_service_refused_spends_none_of_the_days_cap() {
    let (quiet, ada, bo) = two_friends().await;
    daily_game(&quiet, &ada, &bo, 3).await;
    notifier_idle(&quiet).await;
    let (service, seen) = fake_service(StatusCode::INTERNAL_SERVER_ERROR).await;
    let app = app_with(
        Some(quiet.db.clone()),
        TestAppOptions {
            mail_api: Some(format!("{service}/emails/gone")),
            ..TestAppOptions::default()
        },
    )
    .await;
    let now = app.daily.now();
    let swept = sweep(&app, now).await.expect("a sweep");
    assert_eq!(swept.emailed, 0);
    assert_eq!(requests_to(&seen, "/emails/gone"), 2);

    // Every one of the day's emails is still free, and none past them.
    let today = utc_day_bounds(now).0;
    spend_emails(&app, today, DAILY_PLAYER_EMAILS_PER_DAY).await;
    let mut tx = app.db.begin(None).await.expect("a transaction");
    assert!(
        !tx.daily_email_claim(today, DAILY_PLAYER_EMAILS_PER_DAY)
            .await
            .expect("a claim")
    );
    tx.commit().await.expect("commit");
}

/// §9.13: the after-move task and the sweep deciding for one profile at once send one email and
/// one push, not one each. The task's send holds the profile's lock across decide, send and
/// mark-sent; the sweep waits on it, then re-reads fresh state and finds nothing due.
#[tokio::test]
async fn the_after_move_task_and_the_sweep_send_one_profile_one_email_and_push() {
    let (mail_api, mail_seen, mail_gate) = held_service(StatusCode::OK).await;
    let (push_api, push_seen) = fake_service(StatusCode::OK).await;
    let app = app_with(
        None,
        TestAppOptions {
            mail_api: Some(format!("{mail_api}/emails")),
            push_keys: Some(vapid_pair()),
            ..TestAppOptions::default()
        },
    )
    .await;
    let ada = person(&app, 1, "Ada").await;
    let bo = person(&app, 2, "Bo").await;
    befriend(&app, &ada, &bo).await;
    subscribe(&app, &ada, 1, format!("{push_api}/push/1/ada")).await;
    subscribe(&app, &bo, 2, format!("{push_api}/push/2/bo")).await;

    // Ada challenges, Bo accepts: both owe the mulligan, so both are due. The accept answers
    // before any send (enqueue-and-return), and the task takes the acceptor first.
    answers_at_once("the accept", daily_game(&app, &ada, &bo, 1)).await;
    eventually("Bo's mail reached the service", || {
        mail_seen.lock().expect("the requests").len() == 1
    })
    .await;

    // The sweep starts while that send is still held: it collides with the task on Bo.
    let sweep = tokio::spawn({
        let app = Arc::clone(&app);
        async move { sweep(&app, app.daily.now()).await }
    });
    tokio::time::sleep(Duration::from_secs(1)).await;
    // The sweep cannot finish while the gate is closed: its sends stall, or it waits on the
    // acceptor's lock without sending. Which holder it reaches first is the seats' coin toss, so
    // the challenger's mail may or may not have gone too.
    assert!(!sweep.is_finished(), "the sweep collides instead of finishing");
    assert!(
        (1..=2).contains(&mail_seen.lock().expect("the requests").len()),
        "one or two mails while gated: {:?}",
        mail_seen.lock().expect("the requests").len()
    );

    mail_gate.add_permits(1);
    let swept = sweep.await.expect("the sweep").expect("a sweep");
    notifier_idle(&app).await;

    // Exactly one email per recipient and one push per device, however the two collided: the
    // sweep sent the challenger's notices, or waited on the queue's locks and sent none.
    assert_eq!(mail_seen.lock().expect("the requests").len(), 2);
    let mut to: Vec<String> = mail_seen
        .lock()
        .expect("the requests")
        .iter()
        .map(|(_, _, body)| {
            serde_json::from_slice::<Value>(body).expect("a JSON mail")["to"][0]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    to.sort();
    assert_eq!(
        to,
        vec![
            format!("{}@example.test", ada.id),
            format!("{}@example.test", bo.id)
        ]
    );
    let mut paths: Vec<String> = push_seen
        .lock()
        .expect("the requests")
        .iter()
        .map(|(path, _, _)| path.clone())
        .collect();
    paths.sort();
    assert_eq!(paths, vec!["/push/1/ada".to_string(), "/push/2/bo".to_string()]);
    for who in [&ada, &bo] {
        let row = notify_row(&app, who).await;
        assert!(row.last_email_at.is_some(), "{}", who.name);
        assert!(row.last_push_at.is_some(), "{}", who.name);
    }
    assert!(
        swept.emailed <= 1,
        "the sweep emailed at most one: {}",
        swept.emailed
    );
    assert_eq!(
        swept.pushed, swept.emailed,
        "the sweep pushed for each profile it emailed"
    );
}

/// §9.13: the exclusion is per profile: a stalled send for one account never delays another's.
/// Ada's push is held; Bo's goes while it is still held, and Ada is pushed exactly once total.
#[tokio::test]
async fn a_stalled_send_for_one_account_never_delays_another() {
    let (held_api, held_seen, held_gate) = held_service(StatusCode::CREATED).await;
    let (quick_api, quick_seen) = fake_service(StatusCode::OK).await;
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
    let cy = person(&app, 3, "Cy").await;
    let dan = person(&app, 5, "Dan").await;
    befriend(&app, &bo, &dan).await;
    befriend(&app, &cy, &ada).await;

    // Bo and Dan's game, created first, is past the mulligan with Bo holding the move. Nothing is
    // sent for any of it: no devices yet, and no mailer at all.
    let first = daily_game(&app, &bo, &dan, 7).await;
    past_the_mulligan(&app, [&bo, &dan], &first).await;
    for n in 0..8 {
        let (on_move, _) = holder(&app, [&bo, &dan], &first).await;
        if on_move.id == bo.id {
            break;
        }
        one_move(&app, [&bo, &dan], &first, n).await;
    }
    let (on_move, _) = holder(&app, [&bo, &dan], &first).await;
    assert_eq!(on_move.id, bo.id, "Bo holds the first game's move");
    notifier_idle(&app).await;

    subscribe(&app, &ada, 1, format!("{held_api}/push/ada")).await;
    subscribe(&app, &bo, 2, format!("{quick_api}/push/bo")).await;

    // Cy challenges, Ada accepts: both owe the mulligan, so Ada is due. The accept answers before
    // any send, and the task takes Ada first and stalls in her push's send.
    answers_at_once("the accept", daily_game(&app, &cy, &ada, 7)).await;
    eventually("Ada's push reached the service", || {
        held_seen.lock().expect("the requests").len() == 1
    })
    .await;

    // The sweep starts while Ada's send is still held. Bo's game was created first, so the sweep
    // reaches him without touching Ada's lock.
    let sweep = tokio::spawn({
        let app = Arc::clone(&app);
        async move { sweep(&app, app.daily.now()).await }
    });
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        quick_seen.lock().expect("the requests").len(),
        1,
        "Bo is pushed while Ada's send is held"
    );
    assert_eq!(
        held_seen.lock().expect("the requests").len(),
        1,
        "Ada is pushed once, not once per caller"
    );
    assert!(!sweep.is_finished(), "the sweep waits on Ada's lock");

    held_gate.add_permits(1);
    let swept = sweep.await.expect("the sweep").expect("a sweep");
    notifier_idle(&app).await;

    // Ada's push went exactly once, from the queue's task; the sweep sent Bo's alone.
    assert_eq!(held_seen.lock().expect("the requests").len(), 1);
    assert_eq!((swept.emailed, swept.pushed), (0, 1));
    assert!(notify_row(&app, &bo).await.last_push_at.is_some());
    assert!(notify_row(&app, &ada).await.last_push_at.is_some());
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

// R4039000: what the daily games write to the notification feed

mod r4039000_daily_feed {
    use super::*;
    use crate::support::store::{feed, feed_kinds};

    /// `who`'s unread feed rows.
    async fn unread(app: &Arc<App>, who: &Account) -> Vec<Value> {
        feed(app, who).await["notifications"]
            .as_array()
            .expect("a page of rows")
            .iter()
            .filter(|row| row["read"] == false)
            .cloned()
            .collect()
    }

    /// Marks every row of `who`'s feed read.
    async fn read_all(app: &Arc<App>, who: &Account) {
        let (status, body) = api(app, "POST", "/api/notifications/read", &who.token, json!({})).await;
        assert_eq!((status, body), (200, json!({ "unread": 0 })));
    }

    /// `who`'s rows of `kind`, newest first.
    async fn of_kind(app: &Arc<App>, who: &Account, kind: &str) -> Vec<Value> {
        feed(app, who).await["notifications"]
            .as_array()
            .expect("a page of rows")
            .iter()
            .filter(|row| row["kind"] == kind)
            .cloned()
            .collect()
    }

    /// The move `who`, holding it, makes in `match_id`: the game as the move left it.
    async fn move_now(app: &Arc<App>, who: &Account, match_id: &str, nonce: &str) -> Value {
        let game = read(app, who, match_id).await;
        assert_eq!(game["hasMove"], true, "{game}");
        let (status, body) = send(
            app,
            who,
            match_id,
            game["seq"].as_i64().expect("a seq"),
            nonce,
            next_move(&game),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        body
    }

    /// A game `ada` challenged `bo` to, past its mulligan, both feeds read: its match id.
    async fn begun(app: &Arc<App>, ada: &Account, bo: &Account) -> String {
        let match_id = daily_game(app, ada, bo, 1).await;
        past_the_mulligan(app, [ada, bo], &match_id).await;
        read_all(app, ada).await;
        read_all(app, bo).await;
        match_id
    }

    /// A hand-over's game is All Random, dealt from a random seed, and a card can end the turn of the seat
    /// the move passes to as that turn starts, before that seat holds the move, so the move comes straight
    /// back: Classic+ #26 Tommy Tempo casts itself when drawn and ends the turn, and Meditative+ #18
    /// Cognitive Offloading has the AI play the rest of it. A test of hand-overs deals again, up to this
    /// many times, until every move it makes hands over. Whether a move passed is read off the
    /// list, which marks nothing (R4233000): opening the recipient's board would deal with the
    /// very row the feed is checked for.
    const DEALS: usize = 8;

    #[tokio::test]
    async fn r4039000_a_handover_tells_the_next_holder_alone() {
        for _ in 0..DEALS {
            let (app, ada, bo) = two_friends().await;
            let match_id = begun(&app, &ada, &bo).await;
            let (on_move, other) = holder(&app, [&ada, &bo], &match_id).await;
            move_now(&app, on_move, &match_id, "hand-over").await;
            // R4233000: whether the move passed is read off the list, which marks nothing, so the
            // recipient's row is still unread when the feed is checked.
            if !list_has_move(&app, other, &match_id).await {
                continue;
            }
            let told = unread(&app, other).await;
            assert_eq!(
                told,
                vec![json!({
                    "id": told[0]["id"],
                    "kind": "dailyMove",
                    "actor": { "profileId": on_move.id, "username": on_move.name },
                    "subject": match_id,
                    "payload": {},
                    "createdAt": told[0]["createdAt"],
                    "read": false,
                })]
            );
            assert_eq!(
                unread(&app, on_move).await,
                Vec::<Value>::new(),
                "the mover is told nothing"
            );
            return;
        }
        panic!("the move passed in none of {DEALS} deals");
    }

    #[tokio::test]
    async fn r4039000_a_second_move_stands_beside_a_read_row() {
        for _ in 0..DEALS {
            let (app, ada, bo) = two_friends().await;
            let match_id = begun(&app, &ada, &bo).await;
            let (first, second) = holder(&app, [&ada, &bo], &match_id).await;
            // The mulligan's own rows stay listed, read: the moves below add to them.
            let base_first = of_kind(&app, first, "dailyMove").await.len();
            let base_second = of_kind(&app, second, "dailyMove").await.len();
            // First hands over, Second hands back, First hands over again. Each move is made
            // from the mover's own board and the pass read off the list, which marks nothing,
            // so only the mover's own rows are marked along the way.
            move_now(&app, first, &match_id, "one").await;
            if !list_has_move(&app, second, &match_id).await {
                continue;
            }
            let told = of_kind(&app, second, "dailyMove").await;
            assert_eq!(told.len(), base_second + 1, "{told:?}");
            assert_eq!(told[0]["read"], json!(false), "{told:?}");
            move_now(&app, second, &match_id, "two").await;
            if !list_has_move(&app, first, &match_id).await {
                continue;
            }
            move_now(&app, first, &match_id, "three").await;
            if !list_has_move(&app, second, &match_id).await {
                continue;
            }
            // R4233000: Second opened its board (and acted) between the two moves to it, marking
            // the first row read, so the newer move stands beside it instead of replacing it. The
            // store still replaces an unread row (the contract proves it), and the read flags tell
            // the two apart.
            let again = of_kind(&app, second, "dailyMove").await;
            assert_eq!(again.len(), base_second + 2, "{again:?}");
            assert_eq!(again[0]["read"], json!(false), "{again:?}");
            assert!(again[1..].iter().all(|row| row["read"] == true), "{again:?}");
            assert!(again[0]["id"].as_i64() > told[0]["id"].as_i64(), "{again:?}");
            // R4233000: First opened its board to send "three", marking its own row read; First
            // was told once more, and none of its rows is unread.
            let firsts = of_kind(&app, first, "dailyMove").await;
            assert_eq!(firsts.len(), base_first + 1, "First was told once: {firsts:?}");
            assert!(firsts.iter().all(|row| row["read"] == true), "{firsts:?}");

            // Once read, the next move is a row of its own.
            read_all(&app, second).await;
            move_now(&app, second, &match_id, "four").await;
            if !list_has_move(&app, first, &match_id).await {
                continue;
            }
            move_now(&app, first, &match_id, "five").await;
            if !list_has_move(&app, second, &match_id).await {
                continue;
            }
            let rows = of_kind(&app, second, "dailyMove").await;
            assert_eq!(rows.len(), base_second + 3, "{rows:?}");
            assert_eq!(rows[0]["read"], json!(false), "{rows:?}");
            assert!(rows[1..].iter().all(|row| row["read"] == true), "{rows:?}");
            return;
        }
        panic!("every move passed in none of {DEALS} deals");
    }

    #[tokio::test]
    async fn r4039000_a_concession_tells_both_their_result() {
        let (app, ada, bo) = two_friends().await;
        let match_id = begun(&app, &ada, &bo).await;
        let game = read(&app, &ada, &match_id).await;
        let (status, body) = send(
            &app,
            &ada,
            &match_id,
            game["seq"].as_i64().expect("a seq"),
            "concede",
            json!({ "type": "concede" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        // R4233000: the conceder's own row is written already read; both rows are still listed,
        // and the other player's stays unread until they open the board.
        for (who, other, outcome, read) in [(&ada, &bo, "loss", true), (&bo, &ada, "win", false)] {
            let rows = of_kind(&app, who, "dailyEnded").await;
            assert_eq!(rows.len(), 1, "{}: {rows:?}", who.name);
            assert_eq!(rows[0]["subject"], json!(match_id));
            assert_eq!(rows[0]["actor"]["profileId"], json!(other.id));
            assert_eq!(
                rows[0]["payload"],
                json!({ "outcome": outcome, "reason": "concede" })
            );
            assert_eq!(rows[0]["read"], json!(read), "{}: {rows:?}", who.name);
        }
        assert_eq!(unread(&app, &ada).await, Vec::<Value>::new());
        assert_eq!(unread(&app, &bo).await.len(), 1);
    }

    #[tokio::test]
    async fn r4039000_time_up_tells_both_on_time() {
        for through_the_sweep in [true, false] {
            let (app, ada, bo) = two_friends().await;
            let match_id = begun(&app, &ada, &bo).await;
            let (late, waiting) = holder(&app, [&ada, &bo], &match_id).await;
            app.daily.set_offset(MS_PER_DAY);
            if through_the_sweep {
                let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
                assert_eq!((status, swept["settled"].clone()), (200, json!(1)), "{swept}");
                // Nobody opened a board: both rows are unread.
                for (who, outcome) in [(late, "loss"), (waiting, "win")] {
                    let rows = unread(&app, who).await;
                    assert_eq!(
                        rows.iter()
                            .map(|row| (row["kind"].clone(), row["payload"].clone()))
                            .collect::<Vec<_>>(),
                        vec![(
                            json!("dailyEnded"),
                            json!({ "outcome": outcome, "reason": "disconnect" })
                        )],
                        "{}: {rows:?}",
                        who.name
                    );
                }
            } else {
                assert_eq!(read(&app, waiting, &match_id).await["lostOnTime"], true);
                // R4233000: a loss on time is the server's act, so both rows are written unread,
                // and the read that settled the game marked the reader's own row read.
                let rows = unread(&app, late).await;
                assert_eq!(
                    rows.iter()
                        .map(|row| (row["kind"].clone(), row["payload"].clone()))
                        .collect::<Vec<_>>(),
                    vec![(
                        json!("dailyEnded"),
                        json!({ "outcome": "loss", "reason": "disconnect" })
                    )],
                    "{}: {rows:?}",
                    late.name
                );
                let rows = of_kind(&app, waiting, "dailyEnded").await;
                assert_eq!(rows.len(), 1, "{rows:?}");
                assert_eq!(
                    rows[0]["payload"],
                    json!({ "outcome": "win", "reason": "disconnect" }),
                    "{rows:?}"
                );
                assert_eq!(rows[0]["read"], json!(true), "{rows:?}");
            }
            // Read again, the game writes nothing more.
            read(&app, late, &match_id).await;
            assert_eq!(of_kind(&app, waiting, "dailyEnded").await.len(), 1);
        }
    }

    #[tokio::test]
    async fn r4039000_a_challenge_its_decline_and_a_cancel() {
        let (app, ada, bo) = two_friends().await;
        read_all(&app, &ada).await;
        read_all(&app, &bo).await;
        let (status, body) = api(
            &app,
            "POST",
            "/api/daily/challenges",
            &ada.token,
            random_challenge(&bo, 3),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let id = body["id"].as_str().expect("a challenge id").to_string();
        let rows = unread(&app, &bo).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(
            (
                rows[0]["kind"].clone(),
                rows[0]["subject"].clone(),
                rows[0]["payload"].clone()
            ),
            (
                json!("dailyChallenge"),
                json!(id),
                json!({ "mode": "random", "perMoveDays": 3 })
            )
        );
        assert_eq!(rows[0]["actor"]["profileId"], json!(ada.id));
        assert_eq!(unread(&app, &ada).await, Vec::<Value>::new());

        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/daily/challenges/{id}/decline"),
            &bo.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");
        let rows = unread(&app, &ada).await;
        assert_eq!(
            rows.iter()
                .map(|row| (
                    row["kind"].clone(),
                    row["subject"].clone(),
                    row["payload"].clone()
                ))
                .collect::<Vec<_>>(),
            vec![(json!("dailyAnswered"), json!(id), json!({ "answer": "declined" }))]
        );
        // R4233000: answering the challenge marked the decliner's row read; it stays listed, and
        // the decliner is told nothing new.
        let rows = of_kind(&app, &bo, "dailyChallenge").await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["subject"], json!(id), "{rows:?}");
        assert_eq!(rows[0]["read"], json!(true), "{rows:?}");
        assert_eq!(unread(&app, &bo).await, Vec::<Value>::new());

        // A cancel tells nobody.
        read_all(&app, &ada).await;
        read_all(&app, &bo).await;
        let (_, body) = api(
            &app,
            "POST",
            "/api/daily/challenges",
            &ada.token,
            random_challenge(&bo, 7),
        )
        .await;
        let id = body["id"].as_str().expect("a challenge id").to_string();
        read_all(&app, &bo).await;
        let (status, body) = api(
            &app,
            "DELETE",
            &format!("/api/daily/challenges/{id}"),
            &ada.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");
        assert_eq!(unread(&app, &ada).await, Vec::<Value>::new());
        assert_eq!(unread(&app, &bo).await, Vec::<Value>::new());

        // An accept tells the challenger, linking the board, and no "your move" beside it.
        let (_, body) = api(
            &app,
            "POST",
            "/api/daily/challenges",
            &ada.token,
            random_challenge(&bo, 1),
        )
        .await;
        let id = body["id"].as_str().expect("a challenge id").to_string();
        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/daily/challenges/{id}/accept"),
            &bo.token,
            json!({ "mode": "random" }),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let rows = unread(&app, &ada).await;
        assert_eq!(
            rows.iter()
                .map(|row| (
                    row["kind"].clone(),
                    row["subject"].clone(),
                    row["payload"].clone()
                ))
                .collect::<Vec<_>>(),
            vec![(
                json!("dailyAnswered"),
                json!(id),
                json!({ "answer": "accepted", "matchId": body["matchId"] })
            )]
        );
    }

    #[tokio::test]
    async fn r4039000_a_seek_pairing_tells_the_waiting_seeker() {
        let app = app_with(None, TestAppOptions::default()).await;
        let ada = person(&app, 1, "Ada").await;
        let bo = person(&app, 2, "Bo").await;
        let seek = json!({ "mode": "random", "perMoveDays": 3 });
        let (status, body) = api(&app, "POST", "/api/daily/seeks", &ada.token, seek.clone()).await;
        assert_eq!(status, 201, "{body}");
        assert_eq!(
            feed_kinds(&feed(&app, &ada).await),
            Vec::<String>::new(),
            "a seek alone tells nobody"
        );
        let (status, body) = api(&app, "POST", "/api/daily/seeks", &bo.token, seek).await;
        assert_eq!(status, 201, "{body}");
        let match_id = body["matchId"].as_str().expect("it paired").to_string();
        let rows = unread(&app, &ada).await;
        assert_eq!(
            rows.iter()
                .map(|row| (
                    row["kind"].clone(),
                    row["subject"].clone(),
                    row["actor"]["profileId"].clone()
                ))
                .collect::<Vec<_>>(),
            vec![(json!("dailyMove"), json!(match_id), json!(bo.id))]
        );
        assert_eq!(
            feed_kinds(&feed(&app, &bo).await),
            Vec::<String>::new(),
            "the claimer is told nothing"
        );
    }

    #[tokio::test]
    async fn r4039000_a_block_writes_nothing() {
        let (app, ada, bo) = two_friends().await;
        let match_id = begun(&app, &ada, &bo).await;
        let (status, body) = api(
            &app,
            "POST",
            "/api/blocks",
            &bo.token,
            json!({ "profileId": ada.id }),
        )
        .await;
        assert!(status == 201 || status == 204, "{status} {body}");
        // R3727007: the game goes on, and neither is told of the other's moves, nor of its end.
        let (on_move, _) = holder(&app, [&ada, &bo], &match_id).await;
        move_now(&app, on_move, &match_id, "after-block").await;
        let game = read(&app, &ada, &match_id).await;
        let (status, body) = send(
            &app,
            &ada,
            &match_id,
            game["seq"].as_i64().expect("a seq"),
            "concede",
            json!({ "type": "concede" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(unread(&app, &ada).await, Vec::<Value>::new());
        assert_eq!(unread(&app, &bo).await, Vec::<Value>::new());
    }

    #[tokio::test]
    async fn r4039000_email_available_follows_the_mailer() {
        let (mail_api, _seen) = fake_service(StatusCode::OK).await;
        for (mail, available) in [(Some(mail_api), true), (None, false)] {
            let app = app_with(
                None,
                TestAppOptions {
                    mail_api: mail,
                    ..TestAppOptions::default()
                },
            )
            .await;
            let ada = person(&app, 1, "Ada").await;
            let (status, settings) = api(&app, "GET", "/api/daily/settings", &ada.token, Value::Null).await;
            assert_eq!(
                (status, settings["emailAvailable"].clone()),
                (200, json!(available)),
                "{settings}"
            );
            let (status, settings) = api(
                &app,
                "PUT",
                "/api/daily/settings",
                &ada.token,
                json!({ "emailOn": false }),
            )
            .await;
            assert_eq!(
                (status, settings["emailAvailable"].clone()),
                (200, json!(available)),
                "{settings}"
            );
        }
    }
}

mod r4233000_read_on_open {
    //! R4233000 (#4233): a notification counts as seen once its owner opens or answers what it is
    //! about. Reading a daily board (or sending an action there) marks that game's rows read, and
    //! the game-ending row told to the player whose own act ended it is written already read.
    use super::*;
    use crate::support::store::{feed, feed_kinds};

    /// `who`'s unread feed rows.
    async fn unread(app: &Arc<App>, who: &Account) -> Vec<Value> {
        feed(app, who).await["notifications"]
            .as_array()
            .expect("a page of rows")
            .iter()
            .filter(|row| row["read"] == false)
            .cloned()
            .collect()
    }

    /// Marks every row of `who`'s feed read.
    async fn read_all(app: &Arc<App>, who: &Account) {
        let (status, body) = api(app, "POST", "/api/notifications/read", &who.token, json!({})).await;
        assert_eq!((status, body), (200, json!({ "unread": 0 })));
    }

    /// `who`'s rows of `kind`, newest first.
    async fn of_kind(app: &Arc<App>, who: &Account, kind: &str) -> Vec<Value> {
        feed(app, who).await["notifications"]
            .as_array()
            .expect("a page of rows")
            .iter()
            .filter(|row| row["kind"] == kind)
            .cloned()
            .collect()
    }

    /// The move `who`, holding it, makes in `match_id`: the game as the move left it.
    async fn move_now(app: &Arc<App>, who: &Account, match_id: &str, nonce: &str) -> Value {
        let game = read(app, who, match_id).await;
        assert_eq!(game["hasMove"], true, "{game}");
        let (status, body) = send(
            app,
            who,
            match_id,
            game["seq"].as_i64().expect("a seq"),
            nonce,
            next_move(&game),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        body
    }

    /// A game `ada` challenged `bo` to, past its mulligan, both feeds read: its match id.
    async fn begun(app: &Arc<App>, ada: &Account, bo: &Account) -> String {
        let match_id = daily_game(app, ada, bo, 1).await;
        past_the_mulligan(app, [ada, bo], &match_id).await;
        read_all(app, ada).await;
        read_all(app, bo).await;
        match_id
    }

    /// `who`'s `notificationsUnread` in the social read.
    async fn social_unread(app: &Arc<App>, who: &Account) -> Value {
        let (status, social) = api(app, "GET", "/api/social", &who.token, Value::Null).await;
        assert_eq!(status, 200, "{social}");
        social["notificationsUnread"].clone()
    }

    /// A hand-over's game deals again until every move it makes hands over (as
    /// `r4039000_daily_feed`'s `DEALS`).
    const DEALS: usize = 8;

    #[tokio::test]
    async fn r4233000_a_board_read_marks_its_move_rows_read() {
        for _ in 0..DEALS {
            let (app, ada, bo) = two_friends().await;
            let match_id = begun(&app, &ada, &bo).await;
            let (on_move, other) = holder(&app, [&ada, &bo], &match_id).await;
            move_now(&app, on_move, &match_id, "hand-over").await;
            if !list_has_move(&app, other, &match_id).await {
                continue;
            }
            // One unread row, and the social read counts it.
            assert_eq!(unread(&app, other).await.len(), 1);
            assert_eq!(social_unread(&app, other).await, json!(1));

            // Opening the board marks it read: the social read drops by one, the row stays
            // listed, and reading again changes nothing.
            read(&app, other, &match_id).await;
            assert_eq!(unread(&app, other).await, Vec::<Value>::new());
            assert_eq!(social_unread(&app, other).await, json!(0));
            let rows = of_kind(&app, other, "dailyMove").await;
            assert_eq!(rows.len(), 1, "{rows:?}");
            assert_eq!(rows[0]["subject"], json!(match_id), "{rows:?}");
            assert_eq!(rows[0]["read"], json!(true), "{rows:?}");
            read(&app, other, &match_id).await;
            assert_eq!(unread(&app, other).await, Vec::<Value>::new());
            assert_eq!(social_unread(&app, other).await, json!(0));
            assert_eq!(of_kind(&app, other, "dailyMove").await.len(), 1);
            return;
        }
        panic!("the move passed in none of {DEALS} deals");
    }

    #[tokio::test]
    async fn r4233000_an_action_marks_the_same_rows() {
        for _ in 0..DEALS {
            let (app, ada, bo) = two_friends().await;
            let match_id = begun(&app, &ada, &bo).await;
            let (on_move, other) = holder(&app, [&ada, &bo], &match_id).await;
            move_now(&app, on_move, &match_id, "hand-over").await;
            if !list_has_move(&app, other, &match_id).await {
                continue;
            }
            assert_eq!(unread(&app, other).await.len(), 1);
            // The Daily-screen turn: open the board, send the move. Nothing waits on the
            // player after it, and the bell clears.
            let answered = move_now(&app, other, &match_id, "answer").await;
            assert_eq!(unread(&app, other).await, Vec::<Value>::new(), "after the action");
            assert_eq!(social_unread(&app, other).await, json!(0));
            let rows = of_kind(&app, other, "dailyMove").await;
            assert_eq!(rows.len(), 1, "{rows:?}");
            assert_eq!(rows[0]["read"], json!(true), "{rows:?}");
            assert!(answered["seq"].as_i64().is_some_and(|seq| seq > 0));
            return;
        }
        panic!("the move passed in none of {DEALS} deals");
    }

    #[tokio::test]
    async fn r4233000_the_senders_own_ended_row_is_written_read() {
        let (app, ada, bo) = two_friends().await;
        let match_id = begun(&app, &ada, &bo).await;
        let game = read(&app, &bo, &match_id).await;
        let (status, body) = send(
            &app,
            &bo,
            &match_id,
            game["seq"].as_i64().expect("a seq"),
            "concede",
            json!({ "type": "concede" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        // The conceder's own row is listed but already read, so their social read counts
        // nothing; the other player's row stays unread until they open the board.
        let rows = of_kind(&app, &bo, "dailyEnded").await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["subject"], json!(match_id), "{rows:?}");
        assert_eq!(
            rows[0]["payload"],
            json!({ "outcome": "loss", "reason": "concede" }),
            "{rows:?}"
        );
        assert_eq!(rows[0]["read"], json!(true), "{rows:?}");
        assert_eq!(social_unread(&app, &bo).await, json!(0));
        let rows = of_kind(&app, &ada, "dailyEnded").await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["read"], json!(false), "{rows:?}");
        assert_eq!(social_unread(&app, &ada).await, json!(1));
        assert!(feed_kinds(&feed(&app, &bo).await).contains(&"dailyEnded".to_string()));
    }

    #[tokio::test]
    async fn r4233000_accepting_or_declining_a_daily_challenge_marks_its_row() {
        let (app, ada, bo) = two_friends().await;
        read_all(&app, &ada).await;
        read_all(&app, &bo).await;
        let (status, body) = api(
            &app,
            "POST",
            "/api/daily/challenges",
            &ada.token,
            random_challenge(&bo, 3),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let first = body["id"].as_str().expect("a challenge id").to_string();
        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/daily/challenges/{first}/decline"),
            &bo.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");
        // Declining marked that challenge's row; nothing else changed.
        let rows = of_kind(&app, &bo, "dailyChallenge").await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["subject"], json!(first), "{rows:?}");
        assert_eq!(rows[0]["read"], json!(true), "{rows:?}");
        assert_eq!(unread(&app, &bo).await, Vec::<Value>::new());

        // A second challenge's row is untouched until it is answered, and answering it leaves
        // the first alone.
        let (status, body) = api(
            &app,
            "POST",
            "/api/daily/challenges",
            &ada.token,
            random_challenge(&bo, 7),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let second = body["id"].as_str().expect("a challenge id").to_string();
        let rows = of_kind(&app, &bo, "dailyChallenge").await;
        assert_eq!(rows.len(), 2, "{rows:?}");
        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/daily/challenges/{second}/accept"),
            &bo.token,
            json!({ "mode": "random" }),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let rows = of_kind(&app, &bo, "dailyChallenge").await;
        assert_eq!(
            rows.iter()
                .map(|row| (row["subject"].clone(), row["read"].clone()))
                .collect::<Vec<_>>(),
            vec![(json!(second), json!(true)), (json!(first), json!(true)),],
            "{rows:?}"
        );
        assert_eq!(unread(&app, &bo).await, Vec::<Value>::new());
    }

    /// #4291 (F12): the accept's `dailyAnswered` row links the board, but reading the board marks
    /// only the game's move and ended rows, so the answer stays unread (unspecified: R4233000 names
    /// no act that answers an answer).
    #[tokio::test]
    async fn r4233000_opening_the_board_an_accept_links_leaves_its_answer_unread() {
        let (app, ada, bo) = two_friends().await;
        read_all(&app, &ada).await;
        read_all(&app, &bo).await;
        let (status, body) = api(
            &app,
            "POST",
            "/api/daily/challenges",
            &ada.token,
            random_challenge(&bo, 3),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let challenge = body["id"].as_str().expect("a challenge id").to_string();
        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/daily/challenges/{challenge}/accept"),
            &bo.token,
            json!({ "mode": "random" }),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let match_id = body["matchId"].as_str().expect("a match id").to_string();
        let answered = of_kind(&app, &ada, "dailyAnswered").await;
        assert_eq!(answered.len(), 1, "{answered:?}");
        assert_eq!(answered[0]["payload"]["matchId"], json!(match_id), "{answered:?}");
        assert_eq!(answered[0]["read"], json!(false), "{answered:?}");

        let game = read(&app, &ada, &match_id).await;
        assert_eq!(game["matchId"], json!(match_id), "{game}");
        let answered = of_kind(&app, &ada, "dailyAnswered").await;
        assert_eq!(answered[0]["read"], json!(false), "{answered:?}");
        assert_eq!(social_unread(&app, &ada).await, json!(1));
    }

    /// #4291 (F13): a challenge its sender cancels, or one that lapses unanswered, is no act of its
    /// friend's, so its row keeps counting (unspecified: R4233000 names only the friend's answers).
    #[tokio::test]
    async fn r4233000_a_challenge_cancelled_or_lapsed_unanswered_keeps_its_row_unread() {
        let (app, ada, bo) = two_friends().await;
        read_all(&app, &ada).await;
        read_all(&app, &bo).await;
        let challenge = |days: i64| {
            let app = app.clone();
            let ada_token = ada.token.clone();
            let body = random_challenge(&bo, days);
            async move {
                let (status, body) = api(&app, "POST", "/api/daily/challenges", &ada_token, body).await;
                assert_eq!(status, 201, "{body}");
                body["id"].as_str().expect("a challenge id").to_string()
            }
        };
        let cancelled = challenge(3).await;
        let (status, body) = api(
            &app,
            "DELETE",
            &format!("/api/daily/challenges/{cancelled}"),
            &ada.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");
        let lapsed = challenge(7).await;
        app.daily
            .set_offset(jackioh_server::config::DAILY_CHALLENGE_TTL_DAYS * MS_PER_DAY + 1);
        let (status, _headers, swept) = call(&app, "POST", "/daily-sweep", None, Value::Null).await;
        assert_eq!(status, 200, "{swept}");
        assert_eq!(swept["expired"], 1, "{swept}");

        let rows = of_kind(&app, &bo, "dailyChallenge").await;
        assert_eq!(
            rows.iter()
                .map(|row| (row["subject"].clone(), row["read"].clone()))
                .collect::<Vec<_>>(),
            vec![(json!(lapsed), json!(false)), (json!(cancelled), json!(false))],
            "{rows:?}"
        );
        assert_eq!(social_unread(&app, &bo).await, json!(2));
    }

    #[tokio::test]
    async fn r4233000_a_mark_failure_leaves_the_move_answered() {
        let logs = record_logs();
        let (app, ada, bo) = two_friends().await;
        let match_id = begun(&app, &ada, &bo).await;
        let (on_move, _) = holder(&app, [&ada, &bo], &match_id).await;
        let before = read(&app, on_move, &match_id).await;
        fail_on(&app, "notifications.markSubject", "the store cannot answer").await;
        // Every mark fails, and the move is still answered: the failure is logged, never the
        // act's.
        let game = read(&app, on_move, &match_id).await;
        let (status, body) = send(
            &app,
            on_move,
            &match_id,
            game["seq"].as_i64().expect("a seq"),
            "marked-fail",
            next_move(&game),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["seq"], before["seq"].as_i64().expect("a seq") + 1, "{body}");
        stop_failing(&app).await;
        assert!(
            !logs.named("notifications.mark_failed").is_empty(),
            "{:?}",
            logs.entries()
        );
    }
}

/// #4510: the list reads every game's portraits in one match read, not one whole match per game.
#[tokio::test]
async fn list_reads_every_games_portraits_in_one_match_read() {
    use jackioh_engine::wire::PortraitId;
    let (app, ada, bo) = two_friends().await;
    let first = daily_game(&app, &ada, &bo, 1).await;
    let second = daily_game(&app, &bo, &ada, 1).await;
    let dealt = [
        (&first, (PortraitId::Gary, PortraitId::Timmy)),
        (&second, (PortraitId::Dfender, PortraitId::Shredder)),
    ];
    {
        let mut data = fake_lock(&app).await;
        for (match_id, pair) in dealt {
            let row = data
                .tables
                .matches
                .iter_mut()
                .find(|row| &row.id == match_id)
                .expect("the game's match");
            row.portraits = Some(pair);
        }
    }

    let calls = record_calls(&app).await;
    let (status, ada_list) = api(&app, "GET", "/api/daily", &ada.token, Value::Null).await;
    assert_eq!(status, 200, "{ada_list}");
    let seen = calls.lock().expect("call log").clone();
    stop_failing(&app).await;
    assert_eq!(
        seen.iter()
            .filter(|call| call.as_str() == "matches.portraits")
            .count(),
        1,
        "{seen:?}"
    );
    assert!(!seen.iter().any(|call| call == "matches.get"), "{seen:?}");

    let (_, bo_list) = api(&app, "GET", "/api/daily", &bo.token, Value::Null).await;
    for (match_id, pair) in dealt {
        let pick = |list: &Value| {
            let game = list["games"]
                .as_array()
                .and_then(|games| games.iter().find(|game| game["matchId"] == json!(match_id)))
                .unwrap_or_else(|| panic!("{match_id} in {list}"))
                .clone();
            (game["portraits"]["me"].clone(), game["portraits"]["them"].clone())
        };
        let (mine, theirs) = pick(&ada_list);
        let (bo_mine, bo_theirs) = pick(&bo_list);
        assert_eq!((&mine, &theirs), (&bo_theirs, &bo_mine), "{match_id}");
        let mut both = vec![
            mine.as_str().expect("a portrait").to_string(),
            theirs.as_str().expect("a portrait").to_string(),
        ];
        both.sort();
        let mut want = vec![pair.0.as_str().to_string(), pair.1.as_str().to_string()];
        want.sort();
        assert_eq!(both, want, "{match_id}");
    }
}
