//! B33, B34, B41, B42 (#936, `.fullsend/SPEC.md` S6): the Duos co-op rooms over HTTP. The host makes
//! a room (`201 {code}`); a second active account joins and starts the match (host A1, guest A2, the
//! AI on B1 and B2) and both learn its `matchId`; a third gets `409 full`, the host `409 own_room`; an
//! unjoined room expires `DUOS_ROOM_TTL_MS` after it was made and the host may delete it first;
//! `GET /api/duos/live` names the caller's live co-op match; the per-match replay answers each human
//! their own seat of a finished game (409 while it is live, 410 once the retention purge has taken
//! its log, 422 when this build cannot fold it as it was played) and anyone else 404; a blocked pair
//! neither reads nor joins the other's room, and a joined room answers its two people alone.
//!
//! The whole router over the fake store, from accounts seeded active with a tagged username. The
//! seats choose `random: true` (the server deals), except where a saved deck is the point. A
//! refusal's code is read wherever the body carries it (`code_is`).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_engine::wire::{DuosActionBody, DuosSeat};
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    CODE_ALPHABET, DUOS_ROOM_TTL_MS, MATCH_ACTION_RETENTION_DAYS, MATCH_CEILING_MS, MAX_OPEN_ROOMS_PER_HOST,
    MAX_SAVED_DECKS, REPLAY_REQUESTS_PER_MINUTE, ROOM_CODE_LENGTH,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CollectionEntry, Db};
use jackioh_server::duos::registry;
use jackioh_server::duos::store::{DuosActionRow, DuosMatch, DuosRoom};

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
    with_saved_cards(app, profile_id, deck_id, &deck()).await;
}

/// Every playable card in the profile's collection, and one saved deck `deck_id` of `cards`.
async fn with_saved_cards(app: &Arc<App>, profile_id: &str, deck_id: &str, cards: &[String]) {
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
                "cards": cards,
                "portrait": null,
                "catalogVersion": jackioh_cards::catalog_version(),
                "createdAt": AT,
                "updatedAt": AT,
            })),
            MAX_SAVED_DECKS as _,
        )
        .await
        .expect("decks.upsert"));
    assert_eq!(serde_json::to_value(outcome).expect("an outcome"), "created");
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
    api(
        app,
        "GET",
        &format!("/api/duos/rooms/{code}"),
        &who.token,
        Value::Null,
    )
    .await
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

    /// A started match both people then concede (B11), so it is finished.
    async fn finished(&self) -> String {
        let (_, match_id) = self.started().await;
        finish(&self.app, &match_id).await;
        match_id
    }
}

/// Both people concede through the live actor, which ends the match (B11) and writes its result.
async fn finish(app: &Arc<App>, match_id: &str) {
    let actor = registry(app).get(match_id).expect("the live actor");
    for (seat, nonce) in [(DuosSeat::A1, "a1-concede"), (DuosSeat::A2, "a2-concede")] {
        let answer = actor
            .submit(seat, nonce.to_string(), DuosActionBody::Concede)
            .await;
        let answer = serde_json::to_value(&answer).expect("an answer");
        assert_eq!(answer["type"], "ack", "{answer}");
    }
    assert!(actor.is_finished(), "the result is written");
}

/// The match row and its log, as the store holds them.
async fn stored(app: &Arc<App>, match_id: &str) -> (DuosMatch, Vec<DuosActionRow>) {
    store!(app, t => {
        let row = t.duos_matches_get(match_id).await.expect("duos.matches.get").expect("the match");
        let log = t.duos_actions(match_id).await.expect("duos.actions");
        (row, log)
    })
}

/// A copy of the finished match `from` under a new id, its row and its log's actions as `edit`
/// leaves them: a match the server never played, to fold.
async fn copied(
    app: &Arc<App>,
    from_id: &str,
    n: u32,
    edit: impl FnOnce(&mut Value, &mut Vec<Value>),
) -> String {
    let (row, log) = stored(app, from_id).await;
    let mut row = serde_json::to_value(&row).expect("a row");
    let mut actions: Vec<Value> = log.iter().map(|entry| entry.action.clone()).collect();
    edit(&mut row, &mut actions);
    let id = uuid(n);
    row["id"] = json!(id);
    let copy: DuosMatch = from(row);
    let (result, ended_at) = (
        copy.result.clone().expect("a finished match's result"),
        copy.ended_at.expect("a finished match's end"),
    );
    store!(app, t => {
        t.duos_matches_create(&copy).await.expect("duos.matches.create");
        t.duos_matches_finish(&id, &result, ended_at).await.expect("duos.matches.finish");
        for (at, action) in actions.iter().enumerate() {
            let seq = i64::try_from(at).expect("a seq") + 1;
            let row = DuosActionRow {
                match_id: id.clone(),
                seq,
                nonce: action["nonce"].as_str().expect("a nonce").to_string(),
                action: action.clone(),
                at: ended_at,
            };
            t.duos_actions_append(&row).await.expect("duos.actions.append");
        }
    });
    id
}

fn refusal_reason(body: &Value) -> &Value {
    &body["error"]["details"]["reason"]
}

/// What the engine's ceiling ends a co-op game as, and so what the sweep writes for a match it ends
/// (R1047000, R1047001).
fn ceiling_draw() -> Value {
    json!({ "outcome": "draw", "winner": null, "reason": "ceiling" })
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
    async fn b33_b41_b42_a_join_starts_the_match_with_the_host_as_a1_and_the_guest_as_a2_and_both_learn_its_id()
     {
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
        // §13.9: a live co-op game has no replay yet, for either of its people.
        for who in [&t.host, &t.guest] {
            let (status, body) = replay(&t.app, who, &match_id).await;
            assert_eq!(status, 409, "{body}");
            assert!(body["steps"].is_null(), "{body}");
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
        assert_eq!(
            get_room(&t.app, &t.host, &code).await.1["guest"]["profileId"],
            json!(t.guest.id)
        );
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

        let (status, body) = create(
            &t.app,
            &t.host,
            json!({ "difficulty": "medium", "deckId": uuid(800) }),
        )
        .await;

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

        let (status, body) = api(
            &t.app,
            "DELETE",
            &format!("/api/duos/rooms/{code}"),
            &t.host.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        assert_eq!(get_room(&t.app, &t.guest, &code).await.0, 404);
        assert_eq!(join(&t.app, &t.guest, &code).await.0, 404);
        assert!(live(&t.app, &t.guest).await.is_null());
    }

    #[tokio::test]
    async fn b34_only_the_host_frees_a_room() {
        let t = three().await;
        let code = room(&t.app, &t.host).await;

        let (status, _) = api(
            &t.app,
            "DELETE",
            &format!("/api/duos/rooms/{code}"),
            &t.third.token,
            Value::Null,
        )
        .await;

        assert_ne!(status, 204);
        let (status, shown) = get_room(&t.app, &t.guest, &code).await;
        assert_eq!(status, 200, "{shown}");
        assert_eq!(join(&t.app, &t.guest, &code).await.0, 200);
    }

    #[tokio::test]
    async fn b34_deleting_a_joined_room_leaves_its_match_alone() {
        let t = three().await;
        let (code, match_id) = t.started().await;

        let (status, _) = api(
            &t.app,
            "DELETE",
            &format!("/api/duos/rooms/{code}"),
            &t.host.token,
            Value::Null,
        )
        .await;

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

// ---------------------------------------------------------------------------------------------
// B41, §13.9, R767000 — the replay of a finished game, under the catalog it was played on
// ---------------------------------------------------------------------------------------------

mod b41_the_finished_replay {
    use super::*;

    #[tokio::test]
    async fn b41_a_finished_match_answers_each_person_their_own_seats_steps_and_a_live_one_is_409() {
        let t = three().await;
        let (_, match_id) = t.started().await;
        let (status, body) = replay(&t.app, &t.host, &match_id).await;
        assert_eq!(status, 409, "{body}");

        finish(&t.app, &match_id).await;

        let (_, log) = stored(&t.app, &match_id).await;
        for (who, seat) in [(&t.host, "a1"), (&t.guest, "a2")] {
            let (status, body) = replay(&t.app, who, &match_id).await;
            assert_eq!(status, 200, "{body}");
            assert_eq!(body["seat"], seat, "{body}");
            let steps = body["steps"].as_array().expect("steps");
            assert_eq!(steps.len(), log.len() + 1, "as it began, then a step per action");
            for step in steps {
                assert_eq!(step["seat"], seat, "{step}");
            }
            let last = steps.last().expect("the end");
            assert_eq!(
                last["result"],
                json!({ "outcome": "win", "winner": "b", "reason": "concede" })
            );
        }
        let (status, _) = replay(&t.app, &t.third, &match_id).await;
        assert_eq!(status, 404);
    }

    #[tokio::test]
    async fn b41_a_finished_replay_is_folded_once_and_kept_for_the_next_request() {
        let t = three().await;
        let match_id = t.finished().await;
        let held = registry(&t.app);
        assert_eq!(held.replays.held(now_ms()), 0);

        let (status, first) = replay(&t.app, &t.host, &match_id).await;
        assert_eq!(status, 200, "{first}");
        assert_eq!(held.replays.held(now_ms()), 1, "the fold is kept");
        let (status, again) = replay(&t.app, &t.host, &match_id).await;
        assert_eq!(status, 200, "{again}");
        assert_eq!(again, first);
        let (status, theirs) = replay(&t.app, &t.guest, &match_id).await;
        assert_eq!(status, 200, "{theirs}");
        assert_eq!(theirs["seat"], "a2");
        assert_eq!(held.replays.held(now_ms()), 1, "one fold serves both people");
    }

    #[tokio::test]
    async fn r767000_a_join_pins_the_match_to_the_catalog_it_is_dealt_from() {
        let t = three().await;
        let (_, match_id) = t.started().await;

        let (row, _) = stored(&t.app, &match_id).await;

        assert_eq!(row.options.catalog_version, t.app.catalog.version);
        assert_eq!(
            serde_json::to_value(&row.options).expect("options")["catalogVersion"],
            json!(t.app.catalog.version)
        );
    }

    #[tokio::test]
    async fn r767000_a_match_played_on_a_catalog_this_build_does_not_carry_is_422_earlier_patch() {
        let t = three().await;
        let match_id = t.finished().await;
        let gone = copied(&t.app, &match_id, 901, |row, _| {
            row["options"]["catalogVersion"] = json!("v-gone");
        })
        .await;

        let (status, body) = replay(&t.app, &t.host, &gone).await;

        assert_eq!(status, 422, "{body}");
        assert_eq!(*refusal_reason(&body), json!("earlier_patch"), "{body}");
        assert!(body["steps"].is_null(), "{body}");
        // The match it was copied from, on this build's own catalog, still shows.
        assert_eq!(replay(&t.app, &t.host, &match_id).await.0, 200);
    }

    #[tokio::test]
    async fn b24_b41_a_log_that_no_longer_folds_is_refused_rules_changed_never_cut_short() {
        let t = three().await;
        let match_id = t.finished().await;
        // B1 ends a turn during the mulligans: the engine refuses it, so this log does not fold.
        let broken = copied(&t.app, &match_id, 902, |_, actions| {
            actions.insert(
                0,
                json!({ "seat": "b1", "nonce": "refused-1", "body": { "kind": "duel", "body": { "type": "endTurn" } } }),
            );
        })
        .await;

        let (status, body) = replay(&t.app, &t.host, &broken).await;

        assert_eq!(status, 422, "{body}");
        assert_eq!(*refusal_reason(&body), json!("rules_changed"), "{body}");
        assert!(
            body["steps"].is_null(),
            "no step of a fold that went wrong: {body}"
        );
    }

    #[tokio::test]
    async fn b41_a_log_that_folds_to_another_end_than_the_recorded_result_is_refused_rules_changed() {
        let t = three().await;
        let match_id = t.finished().await;
        let misrecorded = copied(&t.app, &match_id, 903, |row, _| {
            row["result"] = json!({ "outcome": "win", "winner": "a", "reason": "concede" });
        })
        .await;

        let (status, body) = replay(&t.app, &t.guest, &misrecorded).await;

        assert_eq!(status, 422, "{body}");
        assert_eq!(*refusal_reason(&body), json!("rules_changed"), "{body}");
    }

    #[tokio::test]
    async fn r768_the_co_op_replay_is_held_to_the_replay_allowance() {
        let t = three().await;
        let match_id = t.finished().await;
        for _ in 0..REPLAY_REQUESTS_PER_MINUTE {
            let (status, body) = replay(&t.app, &t.host, &match_id).await;
            assert_eq!(status, 200, "{body}");
        }

        let (status, body) = replay(&t.app, &t.host, &match_id).await;

        assert_eq!(status, 429, "{body}");
        // The allowance is the account's: the other person still reads their own.
        assert_eq!(replay(&t.app, &t.guest, &match_id).await.0, 200);
    }
}

// ---------------------------------------------------------------------------------------------
// §13.9 — the retention purge takes a finished co-op game's log
// ---------------------------------------------------------------------------------------------

mod retention {
    use super::*;

    /// Unit conversion, not configuration.
    const MS_PER_DAY: u64 = 86_400_000;

    #[tokio::test(start_paused = true)]
    async fn a_finished_co_op_matchs_log_goes_match_action_retention_days_after_it_ended_and_its_replay_is_410()
     {
        let t = three().await;
        let old = t.finished().await;
        assert_eq!(replay(&t.app, &t.host, &old).await.0, 200, "kept, and opened");
        advance(millis(MATCH_ACTION_RETENTION_DAYS) * MS_PER_DAY + 1).await;
        let recent = t.finished().await;
        let (_, live_id) = t.started().await;
        let (_, live_log) = stored(&t.app, &live_id).await;

        let purged = jackioh_server::duos::purge_expired_logs(&t.app)
            .await
            .expect("the purge");

        assert!(purged > 0);
        assert!(
            stored(&t.app, &old).await.1.is_empty(),
            "the old game's log is gone"
        );
        let (row, _) = stored(&t.app, &old).await;
        assert!(row.result.is_some(), "its row and result stay");
        assert!(!stored(&t.app, &recent).await.1.is_empty());
        assert_eq!(stored(&t.app, &live_id).await.1.len(), live_log.len());
        let (status, body) = replay(&t.app, &t.host, &old).await;
        assert_eq!(status, 410, "{body}");
        assert!(code_is(&body, "gone"), "{body}");
        assert_eq!(replay(&t.app, &t.guest, &recent).await.0, 200);
    }
}

// ---------------------------------------------------------------------------------------------
// §13.9 — blocks, and a joined room is its two people's
// ---------------------------------------------------------------------------------------------

mod room_privacy {
    use super::*;

    async fn block(app: &Arc<App>, blocker: &Account, blocked: &Account) {
        store!(app, t => t.blocks_add(&blocker.id, &blocked.id, now_ms()).await.expect("blocks.add"));
    }

    async fn unblock(app: &Arc<App>, blocker: &Account, blocked: &Account) {
        store!(app, t => t.blocks_remove(&blocker.id, &blocked.id).await.expect("blocks.remove"));
    }

    #[tokio::test]
    async fn a_blocked_pair_neither_reads_nor_joins_the_others_room_and_hears_what_an_unknown_code_answers() {
        let t = three().await;
        let code = room(&t.app, &t.host).await;
        let (_, unknown_get) = get_room(&t.app, &t.guest, "ZZZZZZ").await;
        let (_, unknown_join) = join(&t.app, &t.guest, "ZZZZZZ").await;

        for (blocker, blocked) in [(&t.host, &t.guest), (&t.guest, &t.host)] {
            block(&t.app, blocker, blocked).await;
            let (status, body) = get_room(&t.app, &t.guest, &code).await;
            assert_eq!(status, 404, "{body}");
            assert_eq!(body, unknown_get);
            let (status, body) = join(&t.app, &t.guest, &code).await;
            assert_eq!(status, 404, "{body}");
            assert_eq!(body, unknown_join);
            assert!(live(&t.app, &t.guest).await.is_null());
            unblock(&t.app, blocker, blocked).await;
        }

        // Anyone else, and the pair once unblocked, still may.
        assert_eq!(get_room(&t.app, &t.third, &code).await.0, 200);
        let (status, body) = join(&t.app, &t.guest, &code).await;
        assert_eq!(status, 200, "{body}");
    }

    #[tokio::test]
    async fn a_joined_room_answers_its_host_and_guest_alone() {
        let t = three().await;
        let (code, match_id) = t.started().await;

        for who in [&t.host, &t.guest] {
            let (status, shown) = get_room(&t.app, who, &code).await;
            assert_eq!(status, 200, "{shown}");
            assert_eq!(shown["matchId"], json!(match_id));
        }
        let (status, body) = get_room(&t.app, &t.third, &code).await;
        assert_eq!(status, 404, "{body}");
        assert!(body["matchId"].is_null(), "{body}");
        // S6: a third account's join is still told the room is full.
        let (status, body) = join(&t.app, &t.third, &code).await;
        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "full"), "{body}");
    }
}

// ---------------------------------------------------------------------------------------------
// §9.4, R767000 — a host's deck frozen under another catalog
// ---------------------------------------------------------------------------------------------

mod stale_catalog {
    use super::*;

    #[tokio::test]
    async fn a_room_whose_frozen_deck_was_checked_under_another_catalog_refuses_the_join_409_update_required()
    {
        let t = three().await;
        let now = now_ms();
        let stale: DuosRoom = from(json!({
            "code": "STALEA",
            "host": t.host.id,
            "guest": null,
            "difficulty": "easy",
            "hostChoice": {
                "kind": "deck",
                "deck": { "name": "co-op deck", "cards": deck() },
                "catalogVersion": "v-before-the-deploy",
            },
            "guestChoice": null,
            "matchId": null,
            "createdAt": now,
            "expiresAt": now + DUOS_ROOM_TTL_MS,
        }));
        assert!(store!(t.app, tx => tx.duos_rooms_create(&stale).await.expect("duos.rooms.create")));

        let (status, body) = join(&t.app, &t.guest, "STALEA").await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "update_required"), "{body}");
        assert!(live(&t.app, &t.guest).await.is_null(), "no match was started");
        let (status, shown) = get_room(&t.app, &t.host, "STALEA").await;
        assert_eq!(status, 200, "{shown}");
        assert!(shown["guest"].is_null(), "the room is left unclaimed: {shown}");
    }

    #[tokio::test]
    async fn a_room_whose_frozen_deck_was_checked_under_the_running_catalog_starts_its_match() {
        let t = three().await;
        with_saved_deck(&t.app, &t.host.id, &uuid(800)).await;
        let (status, body) = create(
            &t.app,
            &t.host,
            json!({ "difficulty": "easy", "deckId": uuid(800) }),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        let code = body["code"].as_str().expect("a room code").to_string();

        let (status, body) = join(&t.app, &t.guest, &code).await;

        assert_eq!(status, 200, "{body}");
        let match_id = body["matchId"].as_str().expect("a match id");
        let (row, _) = stored(&t.app, match_id).await;
        assert_eq!(row.options.decks.a1, deck(), "the host plays the deck they froze");
    }
}

// ---------------------------------------------------------------------------------------------
// R915005 — a deck with a custom card plays only in practice
// ---------------------------------------------------------------------------------------------

mod a_deck_with_a_custom_card_r915005 {
    //! R915005: a co-op room refuses a deck that names a custom card at both doors, as the 1v1 queue
    //! and rooms do, with the validator's L7 (`loadout_invalid`, the L7 issue as `details`), before
    //! anything is claimed.
    use super::*;

    /// The legal deck with its first card swapped for a custom card: legal by every other rule.
    fn custom_deck() -> Vec<String> {
        let mut cards = deck();
        cards[0] = "custom-0123456789abcdef0123456789abcdef".to_string();
        cards
    }

    fn assert_l7(status: u16, refused: &Value) {
        assert_eq!(status, 422, "{refused}");
        assert_eq!(refused["error"]["code"], "loadout_invalid");
        let details = refused["error"]["details"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(details.len(), 1, "{refused}");
        assert_eq!(details[0]["rule"], "L7");
        assert_eq!(details[0]["cardId"], json!(custom_deck()[0]));
        assert_eq!(refused["error"]["message"], details[0]["message"]);
    }

    #[tokio::test]
    async fn r915005_refuses_a_co_op_room_create_and_a_join_with_a_custom_card_deck_l7_and_claims_nothing() {
        let t = three().await;
        with_saved_cards(&t.app, &t.host.id, &uuid(900), &custom_deck()).await;
        with_saved_cards(&t.app, &t.guest.id, &uuid(901), &custom_deck()).await;

        // The host's door: no room is made.
        let (status, refused) = create(
            &t.app,
            &t.host,
            json!({ "difficulty": "easy", "deckId": uuid(900) }),
        )
        .await;
        assert_l7(status, &refused);
        assert!(fake(&t.app).lock().await.tables.duos.rooms.is_empty());

        // The guest's door: the room stays open to a deck that may play, and no match starts.
        let code = room(&t.app, &t.host).await;
        let (status, refused) = api(
            &t.app,
            "POST",
            &format!("/api/duos/rooms/{code}/join"),
            &t.guest.token,
            json!({ "deckId": uuid(901) }),
        )
        .await;
        assert_l7(status, &refused);
        let (status, shown) = get_room(&t.app, &t.host, &code).await;
        assert_eq!(status, 200, "{shown}");
        assert_eq!(shown["guest"], Value::Null, "{shown}");
        assert_eq!(shown["matchId"], Value::Null, "{shown}");
        assert!(live(&t.app, &t.guest).await.is_null());
        assert!(fake(&t.app).lock().await.tables.duos.matches.is_empty());

        // Control: the guest dealt at random joins the same room.
        let (status, joined) = join(&t.app, &t.guest, &code).await;
        assert_eq!(status, 200, "{joined}");
    }
}

// ---------------------------------------------------------------------------------------------
// R1047000, R1047001 — the ceiling and the sweep's end of a match no actor can be folded back for
// ---------------------------------------------------------------------------------------------

mod r1047000_r1047001_the_sweep {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r1047000_a_match_rebuilt_past_its_ceiling_ends_as_a_ceiling_draw_as_it_arms() {
        let t = three().await;
        let (_, match_id) = t.started().await;
        registry(&t.app).stop(&match_id).await;
        advance(millis(MATCH_CEILING_MS)).await;

        jackioh_server::duos::sweep(&t.app).await.expect("the sweep");
        let actor = registry(&t.app).get(&match_id).expect("the rebuilt actor");
        actor.idle().await;

        assert!(actor.is_finished(), "ended as it armed");
        let (row, log) = stored(&t.app, &match_id).await;
        assert_eq!(row.result, Some(ceiling_draw()));
        assert!(row.ended_at.is_some(), "{row:?}");
        let last = log.last().expect("the ceiling is logged");
        assert_eq!(last.action["seat"], "a1", "{:?}", last.action);
        assert_eq!(
            last.action["body"],
            json!({ "kind": "duel", "body": { "type": "ceilingReached" } }),
            "{:?}",
            last.action
        );
        assert_eq!(last.nonce, format!("srv-ceiling-{}", last.seq));
        assert!(live(&t.app, &t.host).await.is_null());
        assert!(live(&t.app, &t.guest).await.is_null());
        assert_eq!(replay(&t.app, &t.host, &match_id).await.0, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn r1047001_a_match_that_cannot_be_folded_back_is_retried_until_its_ceiling_then_ended_and_both_people_freed()
     {
        let t = three().await;
        let (_, match_id) = t.started().await;
        registry(&t.app).stop(&match_id).await;
        // A row the fold cannot read: the match can no longer be folded back.
        let (_, log) = stored(&t.app, &match_id).await;
        let unreadable = DuosActionRow {
            match_id: match_id.clone(),
            seq: i64::try_from(log.len()).expect("a seq") + 1,
            nonce: "unreadable".into(),
            action: json!({ "seat": "a1" }),
            at: now_ms(),
        };
        store!(&t.app, tx => tx.duos_actions_append(&unreadable).await.expect("duos.actions.append"));

        jackioh_server::duos::sweep(&t.app).await.expect("the sweep");

        assert_eq!(
            live(&t.app, &t.host).await,
            json!(match_id),
            "before its ceiling it is left alone"
        );
        assert!(
            registry(&t.app).get(&match_id).is_none(),
            "no actor could be built"
        );
        assert!(stored(&t.app, &match_id).await.0.ended_at.is_none());

        advance(millis(MATCH_CEILING_MS)).await;
        jackioh_server::duos::sweep(&t.app).await.expect("the sweep");

        let (row, _) = stored(&t.app, &match_id).await;
        assert_eq!(row.result, Some(ceiling_draw()));
        assert!(row.ended_at.is_some(), "{row:?}");
        assert!(live(&t.app, &t.host).await.is_null());
        assert!(live(&t.app, &t.guest).await.is_null());
        for who in [&t.host, &t.guest] {
            let (status, body) = create(&t.app, who, json!({ "difficulty": "easy", "random": true })).await;
            assert_eq!(status, 201, "{body}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// R1047002 — a host's open co-op rooms are bounded
// ---------------------------------------------------------------------------------------------

mod r1047002_the_room_cap {
    use super::*;

    #[tokio::test]
    async fn r1047002_a_hosts_fourth_open_co_op_room_is_409_conflict_and_a_deleted_one_frees_its_place() {
        let t = three().await;
        let mut codes = Vec::new();
        for _ in 0..MAX_OPEN_ROOMS_PER_HOST {
            codes.push(room(&t.app, &t.host).await);
        }

        let (status, body) = create(&t.app, &t.host, json!({ "difficulty": "easy", "random": true })).await;

        assert_eq!(status, 409, "{body}");
        assert!(code_is(&body, "conflict"), "{body}");
        assert_eq!(
            fake(&t.app).lock().await.tables.duos.rooms.len(),
            codes.len(),
            "no room was made"
        );
        let (status, body) = create(&t.app, &t.guest, json!({ "difficulty": "easy", "random": true })).await;
        assert_eq!(status, 201, "the cap is each host's own: {body}");

        let (status, body) = api(
            &t.app,
            "DELETE",
            &format!("/api/duos/rooms/{}", codes[0]),
            &t.host.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");
        let (status, body) = create(&t.app, &t.host, json!({ "difficulty": "easy", "random": true })).await;
        assert_eq!(status, 201, "{body}");
    }
}
