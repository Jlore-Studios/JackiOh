//! B43–B47 (#936, `.fullsend/SPEC.md` S7): friend requests by `name#tag`, the mutual add, accept,
//! decline, cancel and unfriend, blocks, and the status a friend shows.
//!
//! Every request goes through the whole router over the fake store (`support::deps`), from accounts
//! seeded active with a tagged username (`Ada#1`). S7 writes a refusal's body as `{code}`; the repo's
//! error envelope is `{error: {code, message, details?}}`, so `code_is` reads the code wherever the
//! body carries it. A live 1v1 match is started on the real registry with the server tests' cards
//! (`support::engine`), and a Duos match through S6's room routes.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_server::app::App;
use jackioh_server::config::{FRIEND_REQUESTS_PER_HOUR, MAX_FRIENDS, PRESENCE_TTL_MS};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, StartMatchInput};

use crate::support::deps::{TEST_CATALOG_VERSION, add_user, call, empty_test_app};
use crate::support::engine::{fake_deck, install_test_cards};

/// Unit conversions, not configuration.
const MINUTE_MS: u64 = 60_000;
const HOUR_MS: u64 = 3_600_000;

/// How many accepts the cap test makes before it lets §9.8's per-account minute pass.
const ACCEPTS_PER_MINUTE: usize = 100;

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// A config count as a `usize`, whichever integer type `config.rs` gives it.
fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
}

/// A config duration as milliseconds, whichever integer type `config.rs` gives it.
fn millis<T>(value: T) -> u64
where
    u64: TryFrom<T>,
    <u64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    u64::try_from(value).expect("a duration fits a u64")
}

async fn advance(ms: u64) {
    tokio::time::advance(Duration::from_millis(ms)).await;
}

/// A uuid-shaped id, so a route that parses its path as a uuid takes it.
fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

/// One signed-in account: its profile id, its username as shown (`name#tag`) and its token.
struct Account {
    id: String,
    name: String,
    token: String,
}

/// An account seeded with `status`, username `base#n`.
async fn seeded(app: &Arc<App>, n: u32, base: &str, status: &str) -> Account {
    let id = uuid(n);
    let user_id = format!("user-{n}");
    let tag = i64::from(n);
    fake(app).lock().await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "status": status,
        "username": base,
        "usernameTag": tag,
        "usernamePrompted": true,
    }));
    let token = add_user(app, &user_id, &format!("{user_id}@example.test"), true);
    Account {
        id,
        name: format!("{base}#{tag}"),
        token,
    }
}

/// An active account, username `base#n`.
async fn account(app: &Arc<App>, n: u32, base: &str) -> Account {
    seeded(app, n, base, "active").await
}

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

/// Whether a refusal's body carries `code`: S7's `{code}`, or the repo's envelope.
fn code_is(body: &Value, code: &str) -> bool {
    [
        &body["code"],
        &body["error"]["code"],
        &body["error"]["details"]["code"],
        &body["error"]["details"]["reason"],
    ]
    .iter()
    .any(|value| value.as_str() == Some(code))
}

async fn request(app: &Arc<App>, from: &Account, username: &str) -> (u16, Value) {
    api(
        app,
        "POST",
        "/api/friends/requests",
        &from.token,
        json!({ "username": username }),
    )
    .await
}

/// `from` asks `to`; answers the request id.
async fn requested(app: &Arc<App>, from: &Account, to: &Account) -> String {
    let (status, body) = request(app, from, &to.name).await;
    assert_eq!(status, 201, "{body}");
    body["requestId"]
        .as_str()
        .expect("a request id")
        .to_string()
}

async fn accept(app: &Arc<App>, who: &Account, request_id: &str) -> (u16, Value) {
    api(
        app,
        "POST",
        &format!("/api/friends/requests/{request_id}/accept"),
        &who.token,
        Value::Null,
    )
    .await
}

/// `a` asks `b`, `b` accepts.
async fn befriend(app: &Arc<App>, a: &Account, b: &Account) {
    let request_id = requested(app, a, b).await;
    let (status, body) = accept(app, b, &request_id).await;
    assert_eq!(status, 200, "{body}");
}

async fn block(app: &Arc<App>, who: &Account, whom: &Account) {
    let (status, body) = api(
        app,
        "POST",
        "/api/blocks",
        &who.token,
        json!({ "profileId": whom.id }),
    )
    .await;
    assert_eq!(status, 204, "{body}");
}

async fn heartbeat(app: &Arc<App>, who: &Account) {
    let (status, body) = api(app, "POST", "/api/presence", &who.token, Value::Null).await;
    assert_eq!(status, 204, "{body}");
}

/// `GET /api/friends` for `who`.
async fn lists(app: &Arc<App>, who: &Account) -> Value {
    let (status, body) = api(app, "GET", "/api/friends", &who.token, Value::Null).await;
    assert_eq!(status, 200, "{body}");
    body
}

fn len(list: &Value) -> usize {
    list.as_array().map_or(0, Vec::len)
}

/// The `profileId`s of a `friends` or `blocked` list.
fn profile_ids(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["profileId"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The entry of `friends` for `profile_id`.
fn friend_entry(lists: &Value, profile_id: &str) -> Value {
    lists["friends"]
        .as_array()
        .and_then(|friends| {
            friends
                .iter()
                .find(|friend| friend["profileId"] == profile_id)
                .cloned()
        })
        .unwrap_or_else(|| panic!("{profile_id} is not among the friends: {lists}"))
}

fn assert_player_ref(value: &Value, account: &Account) {
    assert_eq!(value["profileId"], json!(account.id), "{value}");
    assert_eq!(value["username"], json!(account.name), "{value}");
}

/// A live, unranked or ranked, 1v1 match between `p1` and `p2` on the real registry, with both
/// profiles marked in it.
async fn live_match(app: &Arc<App>, match_id: &str, p1: &Account, p2: &Account, ranked: bool) {
    install_test_cards();
    let input: StartMatchInput = serde_json::from_value(json!({
        "matchId": match_id,
        "seed": format!("seed-{match_id}"),
        "catalogVersion": TEST_CATALOG_VERSION,
        "ranked": ranked,
        "seats": [
            { "profileId": p1.id, "player": "p1", "deck": fake_deck(&[]) },
            { "profileId": p2.id, "player": "p2", "deck": fake_deck(&[]) },
        ],
    }))
    .expect("a StartMatchInput");
    app.matches
        .start(app, input)
        .await
        .expect("the registry starts the match");
    for profile_id in [&p1.id, &p2.id] {
        let mut tx = app.db.begin(None).await.expect("a store transaction");
        tx.profiles_set_in_match(profile_id, Some(match_id))
            .await
            .expect("profiles.setInMatch");
        tx.commit().await.expect("commit");
    }
}

/// S6: a Duos room hosted by `host` and joined by `guest`; answers the match id.
async fn duos_match(app: &Arc<App>, host: &Account, guest: &Account) -> String {
    jackioh_cards::register_all();
    let (status, created) = api(
        app,
        "POST",
        "/api/duos/rooms",
        &host.token,
        json!({ "difficulty": "easy", "random": true }),
    )
    .await;
    assert_eq!(status, 201, "{created}");
    let code = created["code"].as_str().expect("a room code").to_string();
    let (status, joined) = api(
        app,
        "POST",
        &format!("/api/duos/rooms/{code}/join"),
        &guest.token,
        json!({ "random": true }),
    )
    .await;
    assert_eq!(status, 200, "{joined}");
    joined["matchId"].as_str().expect("a match id").to_string()
}

// ---------------------------------------------------------------------------------------------
// B43 — a request by exact `name#tag`
// ---------------------------------------------------------------------------------------------

mod b43_a_request_by_name_and_tag {
    use super::*;

    #[tokio::test]
    async fn b43_a_request_by_exact_name_and_tag_is_pending_on_both_sides() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;

        let request_id = requested(&app, &ada, &bea).await;

        let mine = lists(&app, &ada).await;
        assert_eq!(len(&mine["outgoing"]), 1, "{mine}");
        let outgoing = &mine["outgoing"][0];
        assert_eq!(outgoing["requestId"], json!(request_id));
        assert_player_ref(&outgoing["from"], &ada);
        assert_player_ref(&outgoing["to"], &bea);
        assert!(outgoing["createdAt"].is_string(), "{outgoing}");
        assert_eq!(len(&mine["incoming"]), 0, "{mine}");
        assert_eq!(len(&mine["friends"]), 0, "{mine}");

        let theirs = lists(&app, &bea).await;
        assert_eq!(len(&theirs["incoming"]), 1, "{theirs}");
        assert_eq!(theirs["incoming"][0]["requestId"], json!(request_id));
        assert_player_ref(&theirs["incoming"][0]["from"], &ada);
        assert_player_ref(&theirs["incoming"][0]["to"], &bea);
        assert_eq!(len(&theirs["outgoing"]), 0, "{theirs}");
        assert_eq!(len(&theirs["friends"]), 0, "{theirs}");
    }

    #[tokio::test]
    async fn b43_an_unknown_name_a_wrong_tag_or_a_missing_tag_answers_404_not_found_and_creates_nothing() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;

        for typed in ["Nobody#1".to_string(), "Bea#3".to_string(), "Bea".to_string()] {
            let (status, body) = request(&app, &ada, &typed).await;
            assert_eq!(status, 404, "{typed}: {body}");
            assert!(code_is(&body, "not_found"), "{typed}: {body}");
        }
        assert_eq!(len(&lists(&app, &ada).await["outgoing"]), 0);
        assert_eq!(len(&lists(&app, &bea).await["incoming"]), 0);
    }

    #[tokio::test]
    async fn b43_b46_a_blocked_pair_answers_the_same_404_as_an_unknown_name_in_either_direction() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let (unknown_status, unknown) = request(&app, &ada, "Nobody#9").await;
        assert_eq!(unknown_status, 404, "{unknown}");

        block(&app, &ada, &bea).await;

        for (from, to) in [(&ada, &bea), (&bea, &ada)] {
            let (status, body) = request(&app, from, &to.name).await;
            assert_eq!(status, unknown_status, "{body}");
            assert!(code_is(&body, "not_found"), "{body}");
            assert!(
                !body.to_string().to_lowercase().contains("block"),
                "a refusal never says a block is why: {body}"
            );
        }
        assert_eq!(len(&lists(&app, &ada).await["incoming"]), 0);
        assert_eq!(len(&lists(&app, &bea).await["incoming"]), 0);
    }

    #[tokio::test]
    async fn b43_a_request_to_oneself_is_409_self() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;

        let (status, body) = request(&app, &ada, &ada.name).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "self"), "{body}");
        assert_eq!(len(&lists(&app, &ada).await["outgoing"]), 0);
    }

    #[tokio::test]
    async fn b43_a_request_to_a_friend_is_409_already_friends() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;

        for (from, to) in [(&ada, &bea), (&bea, &ada)] {
            let (status, body) = request(&app, from, &to.name).await;
            assert_eq!(status, 409, "{body}");
            assert!(code_is(&body, "already_friends"), "{body}");
        }
        assert_eq!(len(&lists(&app, &ada).await["outgoing"]), 0);
        assert_eq!(len(&lists(&app, &bea).await["outgoing"]), 0);
    }

    #[tokio::test]
    async fn b43_a_second_request_to_the_same_player_is_409_already_requested() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        requested(&app, &ada, &bea).await;

        let (status, body) = request(&app, &ada, &bea.name).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "already_requested"), "{body}");
        assert_eq!(len(&lists(&app, &bea).await["incoming"]), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn b43_a_request_from_a_player_holding_max_friends_is_409_friend_cap() {
        let app = empty_test_app().await;
        let hub = account(&app, 1, "Hub").await;
        let cap = count(MAX_FRIENDS);
        for k in 0..cap {
            let n = 1_000 + u32::try_from(k).expect("a small index");
            let pal = account(&app, n, "Pal").await;
            let request_id = requested(&app, &pal, &hub).await;
            let (status, body) = accept(&app, &hub, &request_id).await;
            assert_eq!(status, 200, "friend {k}: {body}");
            if (k + 1) % ACCEPTS_PER_MINUTE == 0 {
                advance(MINUTE_MS + 1).await;
            }
        }
        assert_eq!(len(&lists(&app, &hub).await["friends"]), cap);
        let late = account(&app, 9_000, "Late").await;

        let (status, body) = request(&app, &hub, &late.name).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "friend_cap"), "{body}");
        assert_eq!(len(&lists(&app, &late).await["incoming"]), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn b43_past_friend_requests_per_hour_a_request_answers_429_until_the_hour_has_passed() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let limit = count(FRIEND_REQUESTS_PER_HOUR);
        let mut targets = Vec::new();
        for k in 0..limit + 2 {
            targets.push(account(&app, 100 + u32::try_from(k).expect("a small index"), "Pal").await);
        }
        for target in targets.iter().take(limit) {
            requested(&app, &ada, target).await;
        }

        let (status, body) = request(&app, &ada, &targets[limit].name).await;
        assert_eq!(status, 429, "{body}");
        assert_eq!(len(&lists(&app, &targets[limit]).await["incoming"]), 0);

        advance(HOUR_MS + 1).await;
        let (status, body) = request(&app, &ada, &targets[limit + 1].name).await;
        assert_eq!(status, 201, "{body}");
    }

    #[tokio::test]
    async fn b43_the_friend_routes_turn_away_a_pending_account_and_a_caller_with_no_token() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let pending = seeded(&app, 2, "Pen", "pending").await;

        let (status, _) = api(&app, "GET", "/api/friends", &pending.token, Value::Null).await;
        assert_eq!(status, 403);
        let (status, _) = request(&app, &pending, &ada.name).await;
        assert_eq!(status, 403);
        let (status, _, _) = call(&app, "GET", "/api/friends", None, Value::Null).await;
        assert_eq!(status, 401);
        assert_eq!(len(&lists(&app, &ada).await["incoming"]), 0);
    }
}

// ---------------------------------------------------------------------------------------------
// B44 — the mutual add
// ---------------------------------------------------------------------------------------------

mod b44_the_mutual_add {
    use super::*;

    #[tokio::test]
    async fn b44_a_request_the_other_side_already_sent_makes_them_friends_at_once() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        requested(&app, &ada, &bea).await;

        let (status, body) = request(&app, &bea, &ada.name).await;

        assert!((200..300).contains(&status), "{status}: {body}");
        for (who, other) in [(&ada, &bea), (&bea, &ada)] {
            let held = lists(&app, who).await;
            assert_eq!(profile_ids(&held["friends"]), vec![other.id.clone()], "{held}");
            assert_eq!(len(&held["incoming"]), 0, "{held}");
            assert_eq!(len(&held["outgoing"]), 0, "{held}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B45 — accept, decline, cancel, unfriend
// ---------------------------------------------------------------------------------------------

mod b45_accept_decline_cancel_and_unfriend {
    use super::*;

    #[tokio::test]
    async fn b45_accept_makes_a_friendship_answers_the_friend_and_deletes_the_request() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let request_id = requested(&app, &ada, &bea).await;

        let (status, body) = accept(&app, &bea, &request_id).await;

        assert_eq!(status, 200, "{body}");
        let friend = &body["friend"];
        assert_player_ref(friend, &ada);
        assert!(
            ["offline", "online", "inMatch"].contains(&friend["status"].as_str().unwrap_or("")),
            "{friend}"
        );
        assert!(friend["matchId"].is_null(), "{friend}");
        assert!(friend["watchable"].is_boolean(), "{friend}");
        assert!(friend["since"].is_string(), "{friend}");

        for (who, other) in [(&ada, &bea), (&bea, &ada)] {
            let held = lists(&app, who).await;
            assert_eq!(profile_ids(&held["friends"]), vec![other.id.clone()], "{held}");
            assert_player_ref(&friend_entry(&held, &other.id), other);
            assert_eq!(len(&held["incoming"]), 0, "{held}");
            assert_eq!(len(&held["outgoing"]), 0, "{held}");
        }
        // The request is gone: accepting it again finds nothing.
        assert_eq!(accept(&app, &bea, &request_id).await.0, 404);
    }

    #[tokio::test]
    async fn b45_accepting_an_unknown_request_is_404() {
        let app = empty_test_app().await;
        let bea = account(&app, 2, "Bea").await;

        let (status, body) = accept(&app, &bea, &uuid(777)).await;

        assert_eq!(status, 404, "{body}");
        assert_eq!(len(&lists(&app, &bea).await["friends"]), 0);
    }

    #[tokio::test]
    async fn b45_the_sender_cannot_accept_their_own_request() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let request_id = requested(&app, &ada, &bea).await;

        let (status, body) = accept(&app, &ada, &request_id).await;

        assert_ne!(status, 200, "{body}");
        assert_eq!(len(&lists(&app, &ada).await["friends"]), 0);
        assert_eq!(len(&lists(&app, &bea).await["incoming"]), 1);
    }

    #[tokio::test]
    async fn b45_decline_deletes_the_request_and_makes_no_friendship() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let request_id = requested(&app, &ada, &bea).await;

        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/friends/requests/{request_id}/decline"),
            &bea.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        for who in [&ada, &bea] {
            let held = lists(&app, who).await;
            assert_eq!(len(&held["friends"]), 0, "{held}");
            assert_eq!(len(&held["incoming"]), 0, "{held}");
            assert_eq!(len(&held["outgoing"]), 0, "{held}");
        }
        // Declined, not remembered: Ada may ask again.
        assert_eq!(request(&app, &ada, &bea.name).await.0, 201);
    }

    #[tokio::test]
    async fn b45_declining_an_unknown_request_is_404() {
        let app = empty_test_app().await;
        let bea = account(&app, 2, "Bea").await;

        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/friends/requests/{}/decline", uuid(777)),
            &bea.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 404, "{body}");
    }

    #[tokio::test]
    async fn b45_the_sender_cancels_a_request_and_it_is_gone_for_both() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let request_id = requested(&app, &ada, &bea).await;

        let (status, body) = api(
            &app,
            "DELETE",
            &format!("/api/friends/requests/{request_id}"),
            &ada.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        assert_eq!(len(&lists(&app, &ada).await["outgoing"]), 0);
        assert_eq!(len(&lists(&app, &bea).await["incoming"]), 0);
        assert_eq!(accept(&app, &bea, &request_id).await.0, 404);
        assert_eq!(len(&lists(&app, &bea).await["friends"]), 0);
    }

    #[tokio::test]
    async fn b45_removing_a_friend_ends_the_friendship_on_both_sides() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;

        let (status, body) = api(
            &app,
            "DELETE",
            &format!("/api/friends/{}", bea.id),
            &ada.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        assert_eq!(len(&lists(&app, &ada).await["friends"]), 0);
        assert_eq!(len(&lists(&app, &bea).await["friends"]), 0);
        // Not friends any more, so a new request is a request again.
        assert_eq!(request(&app, &bea, &ada.name).await.0, 201);
    }
}

// ---------------------------------------------------------------------------------------------
// B46 — blocks
// ---------------------------------------------------------------------------------------------

mod b46_blocks {
    use super::*;

    #[tokio::test]
    async fn b46_a_block_removes_the_friendship_and_lists_the_blocked_player_for_the_blocker_only() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;

        block(&app, &ada, &bea).await;

        let blocker = lists(&app, &ada).await;
        assert_eq!(len(&blocker["friends"]), 0, "{blocker}");
        assert_eq!(profile_ids(&blocker["blocked"]), vec![bea.id.clone()], "{blocker}");
        assert_player_ref(&blocker["blocked"][0], &bea);
        let blocked = lists(&app, &bea).await;
        assert_eq!(len(&blocked["friends"]), 0, "{blocked}");
        assert_eq!(len(&blocked["blocked"]), 0, "{blocked}");
    }

    #[tokio::test]
    async fn b46_a_block_removes_pending_requests_in_both_directions() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let cy = account(&app, 3, "Cy").await;
        // Ada asked Bea; Cy asked Ada.
        requested(&app, &ada, &bea).await;
        requested(&app, &cy, &ada).await;

        block(&app, &ada, &bea).await;
        block(&app, &ada, &cy).await;

        let held = lists(&app, &ada).await;
        assert_eq!(len(&held["incoming"]), 0, "{held}");
        assert_eq!(len(&held["outgoing"]), 0, "{held}");
        assert_eq!(len(&lists(&app, &bea).await["incoming"]), 0);
        assert_eq!(len(&lists(&app, &cy).await["outgoing"]), 0);
    }

    #[tokio::test]
    async fn b46_a_block_stops_new_requests_both_ways_until_the_blocker_unblocks() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        block(&app, &bea, &ada).await;
        assert_eq!(request(&app, &ada, &bea.name).await.0, 404);
        assert_eq!(request(&app, &bea, &ada.name).await.0, 404);

        let (status, body) = api(
            &app,
            "DELETE",
            &format!("/api/blocks/{}", ada.id),
            &bea.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        assert_eq!(len(&lists(&app, &bea).await["blocked"]), 0);
        // Unblocked is not befriended: the friendship the block ended stays ended.
        assert_eq!(len(&lists(&app, &bea).await["friends"]), 0);
        assert_eq!(request(&app, &ada, &bea.name).await.0, 201);
    }
}

// ---------------------------------------------------------------------------------------------
// B47 — status
// ---------------------------------------------------------------------------------------------

mod b47_status {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b47_a_friend_with_no_heartbeat_within_presence_ttl_is_offline_with_no_match() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;
        // Bea never sends `POST /api/presence`; whatever else her accept counted for is past too.
        advance(millis(PRESENCE_TTL_MS) + 1).await;

        let friend = friend_entry(&lists(&app, &ada).await, &bea.id);

        assert_eq!(friend["status"], "offline", "{friend}");
        assert!(friend["matchId"].is_null(), "{friend}");
        assert_eq!(friend["watchable"], json!(false), "{friend}");
    }

    #[tokio::test(start_paused = true)]
    async fn b47_a_heartbeat_shows_a_friend_online_until_presence_ttl_has_passed() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;

        heartbeat(&app, &bea).await;
        assert_eq!(friend_entry(&lists(&app, &ada).await, &bea.id)["status"], "online");

        advance(millis(PRESENCE_TTL_MS) - 1).await;
        assert_eq!(friend_entry(&lists(&app, &ada).await, &bea.id)["status"], "online");

        advance(2).await;
        let friend = friend_entry(&lists(&app, &ada).await, &bea.id);
        assert_eq!(friend["status"], "offline", "{friend}");

        // Another heartbeat brings them back.
        heartbeat(&app, &bea).await;
        assert_eq!(friend_entry(&lists(&app, &ada).await, &bea.id)["status"], "online");
    }

    #[tokio::test]
    async fn b47_b50_a_friend_in_a_live_unranked_1v1_is_in_match_naming_it_and_watchable() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let rex = account(&app, 3, "Rex").await;
        befriend(&app, &ada, &bea).await;
        let match_id = uuid(500);

        live_match(&app, &match_id, &bea, &rex, false).await;

        let friend = friend_entry(&lists(&app, &ada).await, &bea.id);
        assert_eq!(friend["status"], "inMatch", "{friend}");
        assert_eq!(friend["matchId"], json!(match_id), "{friend}");
        assert_eq!(friend["watchable"], json!(true), "{friend}");
    }

    #[tokio::test]
    async fn b47_b50_a_friend_in_a_ranked_match_is_in_match_and_not_watchable() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let rex = account(&app, 3, "Rex").await;
        befriend(&app, &ada, &bea).await;
        let match_id = uuid(501);

        live_match(&app, &match_id, &bea, &rex, true).await;

        let friend = friend_entry(&lists(&app, &ada).await, &bea.id);
        assert_eq!(friend["status"], "inMatch", "{friend}");
        assert_eq!(friend["matchId"], json!(match_id), "{friend}");
        assert_eq!(friend["watchable"], json!(false), "{friend}");
    }

    #[tokio::test]
    async fn b47_b50_a_friend_who_turned_spectators_off_is_not_watchable() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let rex = account(&app, 3, "Rex").await;
        befriend(&app, &ada, &bea).await;
        let (status, body) = api(
            &app,
            "PUT",
            "/api/settings",
            &bea.token,
            json!({ "groups": {}, "allowSpectators": false }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        let match_id = uuid(502);

        live_match(&app, &match_id, &bea, &rex, false).await;

        let friend = friend_entry(&lists(&app, &ada).await, &bea.id);
        assert_eq!(friend["status"], "inMatch", "{friend}");
        assert_eq!(friend["watchable"], json!(false), "{friend}");
    }

    #[tokio::test]
    async fn b47_a_friend_in_a_duos_match_is_in_match_naming_it_and_not_watchable() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let gus = account(&app, 3, "Gus").await;
        befriend(&app, &ada, &bea).await;

        let match_id = duos_match(&app, &bea, &gus).await;

        let friend = friend_entry(&lists(&app, &ada).await, &bea.id);
        assert_eq!(friend["status"], "inMatch", "{friend}");
        assert_eq!(friend["matchId"], json!(match_id), "{friend}");
        assert_eq!(friend["watchable"], json!(false), "{friend}");
    }

    #[tokio::test]
    async fn b47_a_heartbeat_from_a_pending_account_or_no_token_is_refused() {
        let app = empty_test_app().await;
        let pending = seeded(&app, 2, "Pen", "pending").await;

        let (status, _) = api(&app, "POST", "/api/presence", &pending.token, Value::Null).await;
        assert_eq!(status, 403);
        let (status, _, _) = call(&app, "POST", "/api/presence", None, Value::Null).await;
        assert_eq!(status, 401);
    }
}
