//! B33, B34, B41, B42 (#936, `.fullsend/SPEC.md` S6): the Duos co-op rooms over HTTP. The host makes
//! a room (`201 {code}`); a second active account joins and starts the match (host A1, guest A2, the
//! AI on B1 and B2) and both learn its `matchId`; a third gets `409 full`, the host `409 own_room`; an
//! unjoined room expires `DUOS_ROOM_TTL_MS` after it was made and the host may delete it first;
//! `GET /api/duos/live` names the caller's live co-op match; the per-match replay answers each human
//! their own seat and anyone else 404.
//!
//! The whole router over the fake store, from accounts seeded active with a tagged username. The
//! seats choose `random: true` (the server deals), except where a saved deck is the point. A
//! refusal's code is read wherever the body carries it (`code_is`).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_server::app::App;
use jackioh_server::config::{CODE_ALPHABET, DUOS_ROOM_TTL_MS, MAX_SAVED_DECKS, ROOM_CODE_LENGTH};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CollectionEntry, Db};

use crate::support::deps::{add_user, call, empty_test_app};

/// The epoch-ms stamp a saved deck carries; nothing reads it back.
const AT: i64 = 1_700_000_000_000;

/// S6's room code length, stated by the spec ("6 chars").
const DUOS_CODE_CHARS: usize = 6;

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

macro_rules! store {
    ($app:expr, $t:ident => $call:expr) => {{
        let mut $t = $app
            .db
            .begin(None)
            .await
            .expect("the fake store opens a transaction");
        let out = $call;
        $t.commit().await.expect("the fake store commits");
        out
    }};
}

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

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

fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

struct Account {
    id: String,
    name: String,
    token: String,
}

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

async fn account(app: &Arc<App>, n: u32, base: &str) -> Account {
    seeded(app, n, base, "active").await
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

/// Every playable catalog id, in `catalog.json` order.
fn playable() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| {
            !def.token
                && !serde_json::to_value(&def.tags)
                    .expect("tags")
                    .as_array()
                    .is_some_and(|tags| tags.contains(&json!("Token")))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// The first `DECK_SIZE` playable ids: a legal deck.
fn deck() -> Vec<String> {
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    playable()[..size].to_vec()
}

/// Every playable card in the profile's collection, and one saved deck `deck_id`.
async fn with_saved_deck(app: &Arc<App>, profile_id: &str, deck_id: &str) {
    let rows: Vec<Value> = playable()
        .iter()
        .map(|id| json!({ "cardId": id, "quantity": 1 }))
        .collect();
    let entries: Vec<CollectionEntry> = from(Value::Array(rows));
    store!(app, t => t.collection_upsert_quantities(profile_id, &entries).await.expect("collection.upsertQuantities"));
    let outcome = store!(app, t => t
        .decks_upsert(
            &from(json!({
                "id": deck_id,
                "profileId": profile_id,
                "name": "co-op deck",
                "cards": deck(),
                "portrait": null,
                "catalogVersion": jackioh_cards::catalog_version(),
                "createdAt": AT,
                "updatedAt": AT,
            })),
            MAX_SAVED_DECKS as _,
        )
        .await
        .expect("decks.upsert"));
    assert_eq!(serde_json::to_value(&outcome).expect("an outcome"), "created");
}

async fn create(app: &Arc<App>, host: &Account, body: Value) -> (u16, Value) {
    api(app, "POST", "/api/duos/rooms", &host.token, body).await
}

/// An Easy room, the host dealt at random; answers its code.
async fn room(app: &Arc<App>, host: &Account) -> String {
    let (status, body) = create(app, host, json!({ "difficulty": "easy", "random": true })).await;
    assert_eq!(status, 201, "{body}");
    body["code"].as_str().expect("a room code").to_string()
}

async fn join(app: &Arc<App>, who: &Account, code: &str) -> (u16, Value) {
    api(
        app,
        "POST",
        &format!("/api/duos/rooms/{code}/join"),
        &who.token,
        json!({ "random": true }),
    )
    .await
}

async fn get_room(app: &Arc<App>, who: &Account, code: &str) -> (u16, Value) {
    api(app, "GET", &format!("/api/duos/rooms/{code}"), &who.token, Value::Null).await
}

async fn live(app: &Arc<App>, who: &Account) -> Value {
    let (status, body) = api(app, "GET", "/api/duos/live", &who.token, Value::Null).await;
    assert_eq!(status, 200, "{body}");
    body["matchId"].clone()
}

async fn replay(app: &Arc<App>, who: &Account, match_id: &str) -> (u16, Value) {
    api(
        app,
        "GET",
        &format!("/api/duos/matches/{match_id}/replay"),
        &who.token,
        Value::Null,
    )
    .await
}

/// Host, guest and a third account, all active.
struct Three {
    app: Arc<App>,
    host: Account,
    guest: Account,
    third: Account,
}

async fn three() -> Three {
    jackioh_cards::register_all();
    let app = empty_test_app().await;
    let host = account(&app, 1, "Hana").await;
    let guest = account(&app, 2, "Gus").await;
    let third = account(&app, 3, "Tess").await;
    Three {
        app,
        host,
        guest,
        third,
    }
}

impl Three {
    /// A room joined by the guest: `(code, matchId)`.
    async fn started(&self) -> (String, String) {
        let code = room(&self.app, &self.host).await;
        let (status, body) = join(&self.app, &self.guest, &code).await;
        assert_eq!(status, 200, "{body}");
        let match_id = body["matchId"].as_str().expect("a match id").to_string();
        (code, match_id)
    }
}

// ---------------------------------------------------------------------------------------------
// B33 — a room the host holds, a join that starts the match
// ---------------------------------------------------------------------------------------------

mod b33_rooms {
    use super::*;

    #[tokio::test]
    async fn b33_a_room_answers_201_with_a_six_character_code_from_the_room_code_alphabet() {
        let t = three().await;

        let (status, body) = create(&t.app, &t.host, json!({ "difficulty": "easy", "random": true })).await;

        assert_eq!(status, 201, "{body}");
        let code = body["code"].as_str().expect("a room code");
        assert_eq!(code.chars().count(), DUOS_CODE_CHARS, "{code}");
        assert_eq!(code.chars().count(), ROOM_CODE_LENGTH, "{code}");
        assert!(code.chars().all(|ch| CODE_ALPHABET.contains(ch)), "{code}");
    }

    #[tokio::test]
    async fn b33_the_room_names_its_host_and_difficulty_and_no_guest_or_match_until_a_join() {
        let t = three().await;
        let (status, body) = create(&t.app, &t.host, json!({ "difficulty": "hard", "random": true })).await;
        assert_eq!(status, 201, "{body}");
        let code = body["code"].as_str().expect("a room code").to_string();

        let (status, shown) = get_room(&t.app, &t.third, &code).await;

        assert_eq!(status, 200, "{shown}");
        assert_eq!(shown["code"], json!(code));
        assert_eq!(shown["difficulty"], "hard");
        assert_eq!(shown["host"]["profileId"], json!(t.host.id));
        assert_eq!(shown["host"]["username"], json!(t.host.name));
        assert!(shown["guest"].is_null(), "{shown}");
        assert!(shown["matchId"].is_null(), "{shown}");
        assert!(live(&t.app, &t.host).await.is_null());
    }

    #[tokio::test]
    async fn b33_b41_b42_a_join_starts_the_match_with_the_host_as_a1_and_the_guest_as_a2_and_both_learn_its_id() {
        let t = three().await;
        let code = room(&t.app, &t.host).await;

        let (status, body) = join(&t.app, &t.guest, &code).await;

        assert_eq!(status, 200, "{body}");
        let match_id = body["matchId"].as_str().expect("a match id").to_string();
        let (status, shown) = get_room(&t.app, &t.host, &code).await;
        assert_eq!(status, 200, "{shown}");
        assert_eq!(shown["guest"]["profileId"], json!(t.guest.id));
        assert_eq!(shown["guest"]["username"], json!(t.guest.name));
        assert_eq!(shown["matchId"], json!(match_id));
        assert_eq!(live(&t.app, &t.host).await, json!(match_id));
        assert_eq!(live(&t.app, &t.guest).await, json!(match_id));
        for (who, seat) in [(&t.host, "a1"), (&t.guest, "a2")] {
            let (status, body) = replay(&t.app, who, &match_id).await;
            assert_eq!(status, 200, "{body}");
            assert_eq!(body["seat"], seat, "{body}");
            for step in body["steps"].as_array().expect("steps") {
                assert_eq!(step["seat"], seat, "{step}");
            }
        }
    }

    #[tokio::test]
    async fn b33_a_third_account_joining_a_full_room_is_409_full() {
        let t = three().await;
        let (code, match_id) = t.started().await;

        let (status, body) = join(&t.app, &t.third, &code).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "full"), "{body}");
        assert!(live(&t.app, &t.third).await.is_null());
        assert_eq!(get_room(&t.app, &t.host, &code).await.1["guest"]["profileId"], json!(t.guest.id));
        assert_eq!(live(&t.app, &t.guest).await, json!(match_id));
    }

    #[tokio::test]
    async fn b33_the_host_joining_their_own_room_is_409_own_room() {
        let t = three().await;
        let code = room(&t.app, &t.host).await;

        let (status, body) = join(&t.app, &t.host, &code).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "own_room"), "{body}");
        let (status, shown) = get_room(&t.app, &t.host, &code).await;
        assert_eq!(status, 200, "{shown}");
        assert!(shown["guest"].is_null(), "{shown}");
        assert!(live(&t.app, &t.host).await.is_null());
    }

    #[tokio::test]
    async fn b33_an_unknown_code_is_404_to_look_up_and_to_join() {
        let t = three().await;

        assert_eq!(get_room(&t.app, &t.third, "ZZZZZZ").await.0, 404);
        assert_eq!(join(&t.app, &t.third, "ZZZZZZ").await.0, 404);
        assert!(live(&t.app, &t.third).await.is_null());
    }

    #[tokio::test]
    async fn b33_a_room_needs_exactly_one_of_a_deck_and_random_and_a_known_difficulty() {
        let t = three().await;
        with_saved_deck(&t.app, &t.host.id, &uuid(800)).await;

        for body in [
            json!({ "difficulty": "easy" }),
            json!({ "difficulty": "easy", "deckId": uuid(800), "random": true }),
            json!({ "difficulty": "nightmare", "random": true }),
            json!({ "random": true }),
        ] {
            let (status, answer) = create(&t.app, &t.host, body.clone()).await;
            assert!((400..500).contains(&status), "{body} answered {status}: {answer}");
        }
    }

    #[tokio::test]
    async fn b33_a_saved_legal_deck_hosts_a_room() {
        let t = three().await;
        with_saved_deck(&t.app, &t.host.id, &uuid(800)).await;

        let (status, body) = create(&t.app, &t.host, json!({ "difficulty": "medium", "deckId": uuid(800) })).await;

        assert_eq!(status, 201, "{body}");
        let code = body["code"].as_str().expect("a room code").to_string();
        let (status, shown) = get_room(&t.app, &t.host, &code).await;
        assert_eq!(status, 200, "{shown}");
        assert_eq!(shown["difficulty"], "medium");
    }

    #[tokio::test]
    async fn b33_the_duos_routes_turn_away_a_pending_account_and_a_caller_with_no_token() {
        let t = three().await;
        let pending = seeded(&t.app, 4, "Pen", "pending").await;

        let (status, _) = create(&t.app, &pending, json!({ "difficulty": "easy", "random": true })).await;
        assert_eq!(status, 403);
        let (status, _) = api(&t.app, "GET", "/api/duos/live", &pending.token, Value::Null).await;
        assert_eq!(status, 403);
        let (status, _, _) = call(
            &t.app,
            "POST",
            "/api/duos/rooms",
            None,
            json!({ "difficulty": "easy", "random": true }),
        )
        .await;
        assert_eq!(status, 401);
    }
}

// ---------------------------------------------------------------------------------------------
// B34 — expiry and delete
// ---------------------------------------------------------------------------------------------

mod b34_expiry_and_delete {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b34_an_unjoined_room_expires_duos_room_ttl_after_it_was_made() {
        let t = three().await;
        let code = room(&t.app, &t.host).await;

        advance(millis(DUOS_ROOM_TTL_MS) - 1).await;
        assert_eq!(get_room(&t.app, &t.guest, &code).await.0, 200);

        advance(2).await;
        assert_eq!(get_room(&t.app, &t.guest, &code).await.0, 404);
        let (status, body) = join(&t.app, &t.guest, &code).await;
        assert_eq!(status, 404, "{body}");
        assert!(live(&t.app, &t.guest).await.is_null());
    }

    #[tokio::test]
    async fn b34_the_host_deletes_an_unjoined_room_and_frees_it() {
        let t = three().await;
        let code = room(&t.app, &t.host).await;

        let (status, body) = api(&t.app, "DELETE", &format!("/api/duos/rooms/{code}"), &t.host.token, Value::Null).await;

        assert_eq!(status, 204, "{body}");
        assert_eq!(get_room(&t.app, &t.guest, &code).await.0, 404);
        assert_eq!(join(&t.app, &t.guest, &code).await.0, 404);
        assert!(live(&t.app, &t.guest).await.is_null());
    }

    #[tokio::test]
    async fn b34_only_the_host_frees_a_room() {
        let t = three().await;
        let code = room(&t.app, &t.host).await;

        let (status, _) = api(&t.app, "DELETE", &format!("/api/duos/rooms/{code}"), &t.third.token, Value::Null).await;

        assert_ne!(status, 204);
        let (status, shown) = get_room(&t.app, &t.guest, &code).await;
        assert_eq!(status, 200, "{shown}");
        assert_eq!(join(&t.app, &t.guest, &code).await.0, 200);
    }

    #[tokio::test]
    async fn b34_deleting_a_joined_room_leaves_its_match_alone() {
        let t = three().await;
        let (code, match_id) = t.started().await;

        let (status, _) = api(&t.app, "DELETE", &format!("/api/duos/rooms/{code}"), &t.host.token, Value::Null).await;

        assert_ne!(status, 500);
        assert_eq!(live(&t.app, &t.host).await, json!(match_id));
        assert_eq!(live(&t.app, &t.guest).await, json!(match_id));
    }
}

// ---------------------------------------------------------------------------------------------
// B41, B42 — the replay route and the live route
// ---------------------------------------------------------------------------------------------

mod b41_b42_replay_and_live {
    use super::*;

    #[tokio::test]
    async fn b41_the_replay_is_404_to_an_account_that_holds_no_seat_and_for_an_unknown_match() {
        let t = three().await;
        let (_, match_id) = t.started().await;

        let (status, body) = replay(&t.app, &t.third, &match_id).await;
        assert_eq!(status, 404, "{body}");
        assert!(body["steps"].is_null(), "no step reaches a non-player: {body}");
        let (status, body) = replay(&t.app, &t.host, &uuid(999)).await;
        assert_eq!(status, 404, "{body}");
    }

    #[tokio::test]
    async fn b42_live_is_null_for_an_account_in_no_co_op_match() {
        let t = three().await;
        assert!(live(&t.app, &t.third).await.is_null());
        let (_, match_id) = t.started().await;

        assert!(live(&t.app, &t.third).await.is_null());
        assert_eq!(live(&t.app, &t.host).await, json!(match_id));
    }
}
