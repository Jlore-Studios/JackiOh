//! B48–B49 (#936, `.fullsend/SPEC.md` S7): challenging a friend to a 1v1. A challenge makes a room
//! exactly as `POST /api/rooms` would (same body, same mode rules) and an open challenge; the friend
//! accepts by joining the room with its code on the existing join route; decline, cancel and expiry
//! free the room.
//!
//! The whole router over the fake store, from accounts seeded active with a tagged username; the
//! rooms are All Random (`{"mode":"random"}`), which needs no saved deck. A refusal's code is read
//! wherever the body carries it (`code_is`): S7's `{code}` or the repo's `{error: {code, …}}`.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_server::app::App;
use jackioh_server::config::{CHALLENGE_TTL_MS, CHALLENGES_PER_HOUR, ROOM_CODE_LENGTH};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, StartMatchInput};

use jackioh_server::social::challenges::sweep;

use crate::support::deps::{TEST_CATALOG_VERSION, add_user, call, empty_test_app};
use crate::support::engine::{fake_deck, install_test_cards};

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// A config duration as milliseconds, whichever integer type `config.rs` gives it.
fn millis<T>(value: T) -> u64
where
    u64: TryFrom<T>,
    <u64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    u64::try_from(value).expect("a duration fits a u64")
}

/// An hour, in milliseconds: the challenge allowance's window.
const HOUR_MS: u64 = 3_600_000;

async fn advance(ms: u64) {
    tokio::time::advance(Duration::from_millis(ms)).await;
}

fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

struct Account {
    id: String,
    name: String,
    token: String,
}

async fn account(app: &Arc<App>, n: u32, base: &str) -> Account {
    let id = uuid(n);
    let user_id = format!("user-{n}");
    let tag = i64::from(n);
    fake(app).lock().await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "status": "active",
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

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

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

async fn befriend(app: &Arc<App>, a: &Account, b: &Account) {
    let (status, body) = api(
        app,
        "POST",
        "/api/friends/requests",
        &a.token,
        json!({ "username": b.name }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let request_id = body["requestId"].as_str().expect("a request id").to_string();
    let (status, body) = api(
        app,
        "POST",
        &format!("/api/friends/requests/{request_id}/accept"),
        &b.token,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

async fn heartbeat(app: &Arc<App>, who: &Account) {
    let (status, body) = api(app, "POST", "/api/presence", &who.token, Value::Null).await;
    assert_eq!(status, 204, "{body}");
}

/// The All Random room body `POST /api/rooms` takes.
fn random_room() -> Value {
    json!({ "mode": "random" })
}

/// `body` plus `friendId`.
fn with_friend(body: Value, friend: &Account) -> Value {
    let mut body = body;
    body["friendId"] = json!(friend.id);
    body
}

async fn challenge(app: &Arc<App>, from: &Account, body: Value) -> (u16, Value) {
    api(app, "POST", "/api/challenges", &from.token, body).await
}

/// `GET /api/challenges` for `who`.
async fn listed(app: &Arc<App>, who: &Account) -> Value {
    let (status, body) = api(app, "GET", "/api/challenges", &who.token, Value::Null).await;
    assert_eq!(status, 200, "{body}");
    body
}

fn len(list: &Value) -> usize {
    list.as_array().map_or(0, Vec::len)
}

async fn join_room(app: &Arc<App>, who: &Account, code: &str, body: Value) -> (u16, Value) {
    api(app, "POST", &format!("/api/rooms/{code}/join"), &who.token, body).await
}

/// Challenger and friend, friends, the friend online and idle.
struct Pair {
    app: Arc<App>,
    challenger: Account,
    friend: Account,
}

async fn pair() -> Pair {
    jackioh_cards::register_all();
    let app = empty_test_app().await;
    let challenger = account(&app, 1, "Cass").await;
    let friend = account(&app, 2, "Finn").await;
    befriend(&app, &challenger, &friend).await;
    heartbeat(&app, &friend).await;
    heartbeat(&app, &challenger).await;
    Pair {
        app,
        challenger,
        friend,
    }
}

impl Pair {
    /// A challenge to the friend for an All Random room: `(challengeId, roomCode)`.
    async fn challenged(&self) -> (String, String) {
        let (status, body) = challenge(
            &self.app,
            &self.challenger,
            with_friend(random_room(), &self.friend),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        (
            body["challengeId"].as_str().expect("a challenge id").to_string(),
            body["roomCode"].as_str().expect("a room code").to_string(),
        )
    }

    /// Whether neither side lists any challenge.
    async fn none_listed(&self) -> bool {
        let mine = listed(&self.app, &self.challenger).await;
        let theirs = listed(&self.app, &self.friend).await;
        len(&mine["incoming"]) + len(&mine["outgoing"]) + len(&theirs["incoming"]) + len(&theirs["outgoing"])
            == 0
    }
}

/// A live 1v1 match holding `p1` and `p2`, on the real registry.
async fn live_match(app: &Arc<App>, match_id: &str, p1: &Account, p2: &Account) {
    install_test_cards();
    let input: StartMatchInput = serde_json::from_value(json!({
        "matchId": match_id,
        "seed": format!("seed-{match_id}"),
        "catalogVersion": TEST_CATALOG_VERSION,
        "ranked": false,
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

// ---------------------------------------------------------------------------------------------
// B48 — a challenge makes a room for an online, idle friend
// ---------------------------------------------------------------------------------------------

mod b48_a_challenge_to_an_online_idle_friend {
    use super::*;

    #[tokio::test]
    async fn b48_a_challenge_answers_201_with_a_room_code_and_is_listed_incoming_and_outgoing() {
        let p = pair().await;

        let (challenge_id, room_code) = p.challenged().await;

        assert_eq!(room_code.chars().count(), ROOM_CODE_LENGTH);
        let theirs = listed(&p.app, &p.friend).await;
        assert_eq!(len(&theirs["incoming"]), 1, "{theirs}");
        assert_eq!(len(&theirs["outgoing"]), 0, "{theirs}");
        let incoming = &theirs["incoming"][0];
        assert_eq!(incoming["challengeId"], json!(challenge_id));
        assert_eq!(incoming["roomCode"], json!(room_code));
        assert_eq!(incoming["mode"], "random");
        assert_eq!(incoming["from"]["profileId"], json!(p.challenger.id));
        assert_eq!(incoming["from"]["username"], json!(p.challenger.name));
        assert_eq!(incoming["to"]["profileId"], json!(p.friend.id));
        assert_eq!(incoming["to"]["username"], json!(p.friend.name));
        assert!(!incoming["expiresAt"].is_null(), "{incoming}");

        let mine = listed(&p.app, &p.challenger).await;
        assert_eq!(len(&mine["outgoing"]), 1, "{mine}");
        assert_eq!(len(&mine["incoming"]), 0, "{mine}");
        assert_eq!(mine["outgoing"][0]["challengeId"], json!(challenge_id));
    }

    #[tokio::test]
    async fn b48_the_room_keeps_post_api_rooms_mode_rules_so_a_join_in_another_mode_is_refused() {
        let p = pair().await;
        let (_, room_code) = p.challenged().await;

        let (status, body) = join_room(
            &p.app,
            &p.friend,
            &room_code,
            json!({ "mode": "bo1", "deckId": uuid(900) }),
        )
        .await;

        // R264: a room made in one mode refuses a join in another, naming its own.
        assert_eq!(status, 409, "{body}");
        assert_eq!(len(&listed(&p.app, &p.friend).await["incoming"]), 1);
    }

    #[tokio::test]
    async fn b48_a_body_post_api_rooms_refuses_is_refused_alike_and_opens_no_challenge() {
        let p = pair().await;
        let outsider = account(&p.app, 3, "Otto").await;
        for room in [
            json!({ "mode": "bo1", "deckId": uuid(900) }),
            json!({}),
            json!({ "mode": "nonsense" }),
        ] {
            let (room_status, room_body) =
                api(&p.app, "POST", "/api/rooms", &outsider.token, room.clone()).await;
            assert!(room_status >= 400, "POST /api/rooms took {room}: {room_body}");

            let (status, body) = challenge(&p.app, &p.challenger, with_friend(room.clone(), &p.friend)).await;

            assert_eq!(status, room_status, "{room}: {body}");
        }
        assert!(p.none_listed().await);
    }

    #[tokio::test]
    async fn b48_a_challenge_to_a_player_who_is_not_a_friend_is_404() {
        let p = pair().await;
        let stranger = account(&p.app, 3, "Stan").await;
        heartbeat(&p.app, &stranger).await;

        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &stranger)).await;

        assert_eq!(status, 404, "{body}");
        assert_eq!(len(&listed(&p.app, &stranger).await["incoming"]), 0);
        assert_eq!(len(&listed(&p.app, &p.challenger).await["outgoing"]), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn b48_a_challenge_to_an_offline_friend_is_409_offline() {
        let p = pair().await;
        // The heartbeat lapses (B47): the friend is offline.
        advance(millis(jackioh_server::config::PRESENCE_TTL_MS) + 1).await;
        heartbeat(&p.app, &p.challenger).await;

        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &p.friend)).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "offline"), "{body}");
        assert!(p.none_listed().await);
    }

    #[tokio::test]
    async fn b48_a_challenge_to_a_friend_in_a_match_is_409_busy() {
        let p = pair().await;
        let rival = account(&p.app, 3, "Rex").await;
        live_match(&p.app, &uuid(600), &p.friend, &rival).await;
        heartbeat(&p.app, &p.friend).await;

        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &p.friend)).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "busy"), "{body}");
        assert!(p.none_listed().await);
    }

    #[tokio::test]
    async fn b48_a_second_open_challenge_to_the_same_friend_is_409_already_challenged() {
        let p = pair().await;
        let (challenge_id, _) = p.challenged().await;

        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &p.friend)).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "already_challenged"), "{body}");
        let theirs = listed(&p.app, &p.friend).await;
        assert_eq!(len(&theirs["incoming"]), 1, "{theirs}");
        assert_eq!(theirs["incoming"][0]["challengeId"], json!(challenge_id));
    }

    #[tokio::test(start_paused = true)]
    async fn b48_past_challenges_per_hour_a_challenge_answers_429_until_the_hour_has_passed() {
        let p = pair().await;
        for _ in 0..CHALLENGES_PER_HOUR {
            let (challenge_id, _) = p.challenged().await;
            let (status, body) = api(
                &p.app,
                "DELETE",
                &format!("/api/challenges/{challenge_id}"),
                &p.challenger.token,
                Value::Null,
            )
            .await;
            assert_eq!(status, 204, "{body}");
        }

        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &p.friend)).await;

        assert_eq!(status, 429, "{body}");
        assert!(p.none_listed().await, "the refused challenge opened nothing");
        // Another account's allowance is its own.
        let (status, body) = challenge(&p.app, &p.friend, with_friend(random_room(), &p.challenger)).await;
        assert_eq!(status, 201, "{body}");

        advance(HOUR_MS).await;
        heartbeat(&p.app, &p.friend).await;
        heartbeat(&p.app, &p.challenger).await;
        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &p.friend)).await;
        assert_eq!(status, 201, "{body}");
    }

    #[tokio::test]
    async fn b48_b46_a_block_stops_challenges_in_both_directions() {
        let p = pair().await;
        let (status, body) = api(
            &p.app,
            "POST",
            "/api/blocks",
            &p.friend.token,
            json!({ "profileId": p.challenger.id }),
        )
        .await;
        assert_eq!(status, 204, "{body}");

        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &p.friend)).await;
        assert_eq!(status, 404, "{body}");
        let (status, body) = challenge(&p.app, &p.friend, with_friend(random_room(), &p.challenger)).await;
        assert_eq!(status, 404, "{body}");
        assert!(p.none_listed().await);
    }
}

// ---------------------------------------------------------------------------------------------
// B49 — accept by joining, decline, cancel, expiry
// ---------------------------------------------------------------------------------------------

mod b49_accept_decline_cancel_and_expiry {
    use super::*;

    #[tokio::test]
    async fn b49_joining_the_room_with_its_code_accepts_the_challenge_and_starts_the_match() {
        let p = pair().await;
        let (_, room_code) = p.challenged().await;

        let (status, body) = join_room(&p.app, &p.friend, &room_code, random_room()).await;

        assert_eq!(status, 200, "{body}");
        // Accepted: no longer open, so neither side lists it.
        assert!(p.none_listed().await);
        // The match the room started holds both (B47).
        let (status, friends) = api(&p.app, "GET", "/api/friends", &p.friend.token, Value::Null).await;
        assert_eq!(status, 200, "{friends}");
        let challenger = friends["friends"]
            .as_array()
            .and_then(|list| {
                list.iter()
                    .find(|entry| entry["profileId"] == p.challenger.id.as_str())
            })
            .cloned()
            .expect("the challenger is a friend");
        assert_eq!(challenger["status"], "inMatch", "{challenger}");
    }

    #[tokio::test]
    async fn b49_declining_deletes_the_challenge_and_frees_the_room() {
        let p = pair().await;
        let (challenge_id, room_code) = p.challenged().await;

        let (status, body) = api(
            &p.app,
            "POST",
            &format!("/api/challenges/{challenge_id}/decline"),
            &p.friend.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        assert!(p.none_listed().await);
        let (status, body) = join_room(&p.app, &p.friend, &room_code, random_room()).await;
        assert_eq!(status, 404, "the declined challenge's room is gone: {body}");
    }

    #[tokio::test]
    async fn b49_the_challenger_cancels_which_deletes_the_challenge_and_frees_the_room() {
        let p = pair().await;
        let (challenge_id, room_code) = p.challenged().await;

        let (status, body) = api(
            &p.app,
            "DELETE",
            &format!("/api/challenges/{challenge_id}"),
            &p.challenger.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        assert!(p.none_listed().await);
        let (status, body) = join_room(&p.app, &p.friend, &room_code, random_room()).await;
        assert_eq!(status, 404, "the cancelled challenge's room is gone: {body}");
    }

    #[tokio::test(start_paused = true)]
    async fn b49_a_challenge_expires_after_challenge_ttl_and_frees_its_room() {
        let p = pair().await;
        let (challenge_id, room_code) = p.challenged().await;

        advance(millis(CHALLENGE_TTL_MS) - 1).await;
        let theirs = listed(&p.app, &p.friend).await;
        assert_eq!(
            len(&theirs["incoming"]),
            1,
            "still open just before its TTL: {theirs}"
        );
        assert_eq!(theirs["incoming"][0]["challengeId"], json!(challenge_id));

        advance(2).await;
        assert!(p.none_listed().await, "expired challenges are not listed");
        let (status, body) = join_room(&p.app, &p.friend, &room_code, random_room()).await;
        assert_eq!(status, 404, "the expired challenge's room is gone: {body}");
    }

    /// The challenge rows the store holds.
    async fn rows(app: &App) -> usize {
        fake(app).lock().await.tables.social.challenges.len()
    }

    #[tokio::test(start_paused = true)]
    async fn b49_the_sweeper_deletes_every_challenge_row_once_its_ttl_has_passed() {
        let p = pair().await;
        let third = account(&p.app, 3, "Tess").await;
        befriend(&p.app, &p.challenger, &third).await;
        heartbeat(&p.app, &third).await;
        // One declined, one cancelled, one never answered, one accepted by joining (last: the match
        // it starts holds the challenger).
        let (declined, _) = p.challenged().await;
        let (status, _) = api(
            &p.app,
            "POST",
            &format!("/api/challenges/{declined}/decline"),
            &p.friend.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204);
        let (cancelled, _) = p.challenged().await;
        let (status, _) = api(
            &p.app,
            "DELETE",
            &format!("/api/challenges/{cancelled}"),
            &p.challenger.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204);
        let (status, body) = challenge(&p.app, &p.challenger, with_friend(random_room(), &third)).await;
        assert_eq!(status, 201, "{body}");
        let (_, joined) = p.challenged().await;
        let (status, body) = join_room(&p.app, &p.friend, &joined, random_room()).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(rows(&p.app).await, 4);

        // Within its TTL, a closed challenge's row is kept (answering it again is still 204).
        sweep(&p.app).await.expect("a sweep");
        assert_eq!(rows(&p.app).await, 4);

        advance(millis(CHALLENGE_TTL_MS)).await;
        assert_eq!(sweep(&p.app).await.expect("a sweep"), 4);

        assert_eq!(rows(&p.app).await, 0, "no challenge row outlives its TTL");
        assert!(p.none_listed().await);
        assert_eq!(len(&listed(&p.app, &third).await["incoming"]), 0);
    }

    #[tokio::test]
    async fn b49_after_a_decline_the_challenger_may_challenge_the_same_friend_again() {
        let p = pair().await;
        let (challenge_id, _) = p.challenged().await;
        let (status, body) = api(
            &p.app,
            "POST",
            &format!("/api/challenges/{challenge_id}/decline"),
            &p.friend.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");

        let (_, again) = p.challenged().await;

        assert_eq!(again.chars().count(), ROOM_CODE_LENGTH);
        assert_eq!(len(&listed(&p.app, &p.friend).await["incoming"]), 1);
    }
}
