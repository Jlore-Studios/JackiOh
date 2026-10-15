//! B52 (#936, `.fullsend/SPEC.md` S7): sharing a finished replay with a friend. A seat holder shares
//! `POST /api/replays/:matchId/shares {friendId}`; the friend's `GET /api/replays/:matchId` then
//! answers the sharer's seat's steps; unshare, unfriend or a block ends it; anyone else still gets
//! the unknown-id answer; `GET /api/replays/shared` lists what was shared with the caller.
//!
//! A finished match is written straight to the store as the actor and `api/results.rs` leave one
//! (as `tests/api/replays.rs` does): its row, its log, its results row, then the final hash. A, B
//! held the seats; C is A's (and B's) friend; D is nobody's.

use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use jackioh_engine::{Action, ActionBody, GameState, PlayerId, Winner, legal_actions, reduce, seat_to_act};
use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::actor::engine::{hash_state, seat_played_by};
use jackioh_server::app::{App, now_ms};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchActionRow, MatchRow, ResultRow};

use crate::support::deps::{add_user, call, empty_test_app};
use crate::support::engine::{begun_game, decks_that_open_on_the_mulligans, install_test_cards, real_pool};

/// How many log actions a game runs before its seat to act concedes: one page of steps.
const PLAYED_ACTIONS: usize = 8;
/// An hour, in milliseconds: how long before its end a seeded match began.
const HOUR_MS: i64 = 3_600_000;

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

macro_rules! q {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut tx = $app.db.begin(None).await.expect("begin");
        let out = tx.$method($($arg),*).await.expect(stringify!($method));
        tx.commit().await.expect("commit");
        out
    }};
}

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
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

// ---------------------------------------------------------------------------------------------
// A finished match (as `tests/api/replays.rs` writes one)
// ---------------------------------------------------------------------------------------------

struct Played {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    log: Vec<Action>,
    last: GameState,
}

fn act(state: &GameState, log: &mut Vec<Action>, seat: PlayerId, body: ActionBody) -> GameState {
    let action = Action::new(body, seat, format!("logged-action-{}", log.len() + 1));
    let result = reduce(state, &action);
    if let Some(error) = result.error {
        panic!("{action:?} was refused: {error}");
    }
    log.push(action);
    result.state
}

/// The seat to act ends its turn when it may and otherwise takes its first legal action, until the
/// game ends or the log holds `max` actions; then the seat to act concedes.
fn played_game(seed: &str, max: usize) -> Played {
    install_test_cards();
    let decks = decks_that_open_on_the_mulligans(&real_pool(), seed);
    let mut state = begun_game(seed, decks.clone());
    let mut log = Vec::new();
    while state.result.is_none() && log.len() < max {
        let seat = seat_to_act(&state).expect("a seat to act");
        let legal = legal_actions(&state, seat);
        let body = if legal.iter().any(|body| matches!(body, ActionBody::EndTurn)) {
            ActionBody::EndTurn
        } else {
            legal.first().cloned().expect("a legal action")
        };
        state = act(&state, &mut log, seat, body);
    }
    if state.result.is_none() {
        let seat = seat_to_act(&state).expect("a seat to act");
        state = act(&state, &mut log, seat, ActionBody::Concede);
    }
    Played {
        seed: seed.to_string(),
        decks,
        log,
        last: state,
    }
}

/// A finished match between `players` (p1, p2), written as the actor and the results writer leave
/// one.
async fn seed_finished(app: &Arc<App>, id: &str, players: (&str, &str), played: &Played) {
    let ended_at = now_ms();
    let created_at = ended_at - HOUR_MS;
    let row: MatchRow = from(json!({
        "id": id,
        "seed": played.seed,
        "players": [players.0, players.1],
        "decks": [played.decks.0, played.decks.1],
        "catalogVersion": jackioh_cards::catalog_version(),
        "status": "live",
        "createdAt": created_at,
        "finishedAt": null,
        "clocks": to_json(&initial_clocks(created_at)),
    }));
    q!(app, matches_create(&row));
    let rows: Vec<MatchActionRow> = played
        .log
        .iter()
        .enumerate()
        .map(|(n, action)| MatchActionRow {
            match_id: id.to_string(),
            seq: i64::try_from(n + 1).expect("a small seq"),
            action: action.clone(),
            at: created_at,
        })
        .collect();
    q!(app, matches_append_actions(&rows));
    // The results row names the seats as the game ended (a Glitch's swap crosses them, R677).
    let (first, second) = if seat_played_by(&played.last, PlayerId::P1) == PlayerId::P1 {
        players
    } else {
        (players.1, players.0)
    };
    let ended = played.last.result.as_ref().expect("a finished game");
    let winner = match ended.winner {
        Winner::P1 => json!(first),
        Winner::P2 => json!(second),
        Winner::Draw => Value::Null,
    };
    let result: ResultRow = from(json!({
        "matchId": id,
        "players": [first, second],
        "winnerProfileId": winner,
        "reason": to_json(&ended.reason),
        "turns": played.last.turn,
        "endedAt": ended_at,
        "ratingBefore": [1000, 1000],
        "ratingAfter": [1000, 1000],
    }));
    q!(app, results_insert(&result));
    q!(app, matches_finish(id, ended_at));
    q!(app, matches_record_final_hash(id, &hash_state(&played.last)));
}

/// A, B seated in a finished match; C friends with both; D a stranger.
struct World {
    app: Arc<App>,
    match_id: String,
    a: Account,
    b: Account,
    c: Account,
    d: Account,
}

async fn world(seed: &str) -> World {
    let app = empty_test_app().await;
    let a = account(&app, 1, "Ada").await;
    let b = account(&app, 2, "Bea").await;
    let c = account(&app, 3, "Cy").await;
    let d = account(&app, 4, "Dee").await;
    befriend(&app, &a, &c).await;
    befriend(&app, &b, &c).await;
    let played = played_game(seed, PLAYED_ACTIONS);
    let match_id = uuid(700);
    seed_finished(&app, &match_id, (a.id.as_str(), b.id.as_str()), &played).await;
    World {
        app,
        match_id,
        a,
        b,
        c,
        d,
    }
}

impl World {
    async fn share(&self, sharer: &Account, friend: &Account) -> (u16, Value) {
        self.share_match(&self.match_id, sharer, friend).await
    }

    async fn share_match(&self, match_id: &str, sharer: &Account, friend: &Account) -> (u16, Value) {
        api(
            &self.app,
            "POST",
            &format!("/api/replays/{match_id}/shares"),
            &sharer.token,
            json!({ "friendId": friend.id }),
        )
        .await
    }

    /// The first page of `who`'s `GET /api/replays/:matchId`.
    async fn replay(&self, who: &Account) -> (u16, Value) {
        api(
            &self.app,
            "GET",
            &format!("/api/replays/{}", self.match_id),
            &who.token,
            Value::Null,
        )
        .await
    }

    /// What `who` gets for a match id nobody ever played.
    async fn unknown(&self, who: &Account) -> (u16, Value) {
        api(
            &self.app,
            "GET",
            &format!("/api/replays/{}", uuid(799)),
            &who.token,
            Value::Null,
        )
        .await
    }

    async fn shared_with(&self, who: &Account) -> Vec<Value> {
        let (status, body) = api(&self.app, "GET", "/api/replays/shared", &who.token, Value::Null).await;
        assert_eq!(status, 200, "{body}");
        body["shared"].as_array().expect("a shared list").clone()
    }

    /// `who` gets exactly what an unknown id gets.
    async fn assert_unknown_to(&self, who: &Account) {
        let held = self.replay(who).await;
        let unknown = self.unknown(who).await;
        assert_eq!(unknown.0, 404, "{}", unknown.1);
        assert_eq!(held, unknown);
    }
}

// ---------------------------------------------------------------------------------------------
// B52
// ---------------------------------------------------------------------------------------------

mod b52_sharing_a_replay {
    use super::*;

    #[tokio::test]
    async fn b52_a_seat_holder_shares_and_the_friend_reads_the_sharers_seat() {
        let w = world("share-1").await;
        w.assert_unknown_to(&w.c).await;

        let (status, body) = w.share(&w.a, &w.c).await;

        assert_eq!(status, 204, "{body}");
        let sharers = w.replay(&w.a).await;
        assert_eq!(sharers.0, 200, "{}", sharers.1);
        let friends = w.replay(&w.c).await;
        assert_eq!(friends, sharers, "the friend reads exactly the sharer's seat");
        assert_eq!(friends.1["steps"][0]["view"]["viewer"], "p1");

        let shared = w.shared_with(&w.c).await;
        assert_eq!(shared.len(), 1, "{shared:?}");
        assert_eq!(shared[0]["matchId"], json!(w.match_id));
        assert_eq!(shared[0]["sharer"]["profileId"], json!(w.a.id));
        assert_eq!(shared[0]["sharer"]["username"], json!(w.a.name));
        assert!(shared[0]["createdAt"].is_string(), "{}", shared[0]);
        assert!(
            shared[0]["endedAt"].is_string(),
            "a finished match has ended: {}",
            shared[0]
        );
        assert!(
            w.shared_with(&w.a).await.is_empty(),
            "the sharer is not shared with"
        );
    }

    #[tokio::test]
    async fn b52_the_other_seat_shares_and_the_friend_reads_that_seat() {
        let w = world("share-2").await;

        let (status, body) = w.share(&w.b, &w.c).await;

        assert_eq!(status, 204, "{body}");
        let sharers = w.replay(&w.b).await;
        assert_eq!(sharers.0, 200, "{}", sharers.1);
        assert_eq!(w.replay(&w.c).await, sharers);
        assert_ne!(w.replay(&w.c).await, w.replay(&w.a).await);
    }

    #[tokio::test]
    async fn b52_anyone_the_replay_was_not_shared_with_still_gets_the_unknown_id_answer() {
        let w = world("share-3").await;
        assert_eq!(w.share(&w.a, &w.c).await.0, 204);

        w.assert_unknown_to(&w.d).await;
        assert!(w.shared_with(&w.d).await.is_empty());
    }

    #[tokio::test]
    async fn b52_unsharing_ends_the_friends_access() {
        let w = world("share-4").await;
        assert_eq!(w.share(&w.a, &w.c).await.0, 204);

        let (status, body) = api(
            &w.app,
            "DELETE",
            &format!("/api/replays/{}/shares/{}", w.match_id, w.c.id),
            &w.a.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        w.assert_unknown_to(&w.c).await;
        assert!(w.shared_with(&w.c).await.is_empty());
        // The sharer still reads their own seat.
        assert_eq!(w.replay(&w.a).await.0, 200);
    }

    #[tokio::test]
    async fn b52_unfriending_ends_the_friends_access() {
        let w = world("share-5").await;
        assert_eq!(w.share(&w.a, &w.c).await.0, 204);

        let (status, body) = api(
            &w.app,
            "DELETE",
            &format!("/api/friends/{}", w.c.id),
            &w.a.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 204, "{body}");
        w.assert_unknown_to(&w.c).await;
        assert!(w.shared_with(&w.c).await.is_empty());
    }

    #[tokio::test]
    async fn b52_b46_a_block_ends_the_friends_access_and_stops_new_shares() {
        let w = world("share-6").await;
        assert_eq!(w.share(&w.a, &w.c).await.0, 204);

        let (status, body) = api(
            &w.app,
            "POST",
            "/api/blocks",
            &w.c.token,
            json!({ "profileId": w.a.id }),
        )
        .await;

        assert_eq!(status, 204, "{body}");
        w.assert_unknown_to(&w.c).await;
        assert!(w.shared_with(&w.c).await.is_empty());
        assert_eq!(w.share(&w.a, &w.c).await.0, 404);
        w.assert_unknown_to(&w.c).await;
    }

    /// A second finished match, between C and D, which C shares with A: a share the other way.
    async fn shared_back(w: &World) -> String {
        let played = played_game("share-back", PLAYED_ACTIONS);
        let match_id = uuid(701);
        seed_finished(&w.app, &match_id, (w.c.id.as_str(), w.d.id.as_str()), &played).await;
        let (status, body) = w.share_match(&match_id, &w.c, &w.a).await;
        assert_eq!(status, 204, "{body}");
        match_id
    }

    /// No share left between A and C, either way: their rows are gone, not merely unread.
    async fn assert_no_share_between_a_and_c(w: &World, back: &str) {
        let left: Vec<Value> = fake(&w.app)
            .lock()
            .await
            .tables
            .replay_shares
            .iter()
            .filter(|row| {
                (row.sharer == w.a.id && row.friend == w.c.id)
                    || (row.sharer == w.c.id && row.friend == w.a.id)
            })
            .map(to_json)
            .collect();
        assert!(left.is_empty(), "{left:?}");
        w.assert_unknown_to(&w.c).await;
        assert!(w.shared_with(&w.c).await.is_empty());
        assert!(w.shared_with(&w.a).await.is_empty());
        let (status, body) = api(
            &w.app,
            "GET",
            &format!("/api/replays/{back}"),
            &w.a.token,
            Value::Null,
        )
        .await;
        assert_eq!((status, body), w.unknown(&w.a).await);
    }

    #[tokio::test]
    async fn b52_b45_an_unfriend_either_way_deletes_the_shares_so_friends_again_read_none() {
        for sharer_removes in [true, false] {
            let w = world("share-10").await;
            assert_eq!(w.share(&w.a, &w.c).await.0, 204);
            let back = shared_back(&w).await;
            assert_eq!(w.shared_with(&w.a).await.len(), 1);
            let (who, whom) = if sharer_removes {
                (&w.a, &w.c)
            } else {
                (&w.c, &w.a)
            };

            let (status, body) = api(
                &w.app,
                "DELETE",
                &format!("/api/friends/{}", whom.id),
                &who.token,
                Value::Null,
            )
            .await;
            assert_eq!(status, 204, "{body}");
            befriend(&w.app, &w.c, &w.a).await;

            assert_no_share_between_a_and_c(&w, &back).await;
        }
    }

    #[tokio::test]
    async fn b52_b46_a_block_either_way_deletes_the_shares_so_unblocked_friends_read_none() {
        for sharer_blocks in [true, false] {
            let w = world("share-11").await;
            assert_eq!(w.share(&w.a, &w.c).await.0, 204);
            let back = shared_back(&w).await;
            let (who, whom) = if sharer_blocks { (&w.a, &w.c) } else { (&w.c, &w.a) };

            let (status, body) = api(
                &w.app,
                "POST",
                "/api/blocks",
                &who.token,
                json!({ "profileId": whom.id }),
            )
            .await;
            assert_eq!(status, 204, "{body}");
            let (status, body) = api(
                &w.app,
                "DELETE",
                &format!("/api/blocks/{}", whom.id),
                &who.token,
                Value::Null,
            )
            .await;
            assert_eq!(status, 204, "{body}");
            befriend(&w.app, &w.a, &w.c).await;

            assert_no_share_between_a_and_c(&w, &back).await;
        }
    }

    #[tokio::test]
    async fn b52_an_unfriend_leaves_the_shares_with_other_friends() {
        let w = world("share-12").await;
        assert_eq!(w.share(&w.a, &w.c).await.0, 204);
        befriend(&w.app, &w.a, &w.d).await;
        assert_eq!(w.share(&w.a, &w.d).await.0, 204);

        let (status, body) = api(
            &w.app,
            "DELETE",
            &format!("/api/friends/{}", w.c.id),
            &w.a.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");

        assert_eq!(w.shared_with(&w.d).await.len(), 1);
        assert_eq!(w.replay(&w.d).await, w.replay(&w.a).await);
    }

    #[tokio::test]
    async fn b52_sharing_with_a_player_who_is_not_a_friend_is_404() {
        let w = world("share-7").await;

        let (status, body) = w.share(&w.a, &w.d).await;

        assert_eq!(status, 404, "{body}");
        w.assert_unknown_to(&w.d).await;
        assert!(w.shared_with(&w.d).await.is_empty());
    }

    #[tokio::test]
    async fn b52_sharing_a_match_the_caller_held_no_seat_in_is_404() {
        let w = world("share-8").await;

        // C is A's friend but held no seat.
        let (status, body) = w.share(&w.c, &w.a).await;

        assert_eq!(status, 404, "{body}");
        assert!(w.shared_with(&w.a).await.is_empty());
    }

    #[tokio::test]
    async fn b52_sharing_an_unknown_match_is_404() {
        let w = world("share-9").await;

        let (status, body) = w.share_match(&uuid(798), &w.a, &w.c).await;

        assert_eq!(status, 404, "{body}");
        assert!(w.shared_with(&w.c).await.is_empty());
    }
}
