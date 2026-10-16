//! B50, B51 (#936, `.fullsend/SPEC.md` S7): watching a friend's live 1v1 game over `GET /ws/watch`.
//! A friend's hello for a live unranked match that allows spectators and has room gets the friend's
//! seat's views, each `SPECTATE_DELAY_MS` after the state it shows, with `legal: []`; ranked,
//! opted-out, full, non-friend and blocked are `forbidden` then closed; a watcher's frames never
//! reach the reducer; a watcher leaving changes nothing in the match; the players' own match sockets
//! hear `watchers {count}` when the count changes. A watcher already in is let go the same way
//! (`forbidden`, then closed) once the friend turns watching off, or the two unfriend or either
//! blocks the other.
//!
//! The match is a real one on the registry, played with the server tests' cards
//! (`support::engine`); its players are attached with in-memory sockets (`support::socket`), as the
//! actor tests attach them. The watcher is a real WebSocket over the app served on in-memory pipes
//! (`actor/duos.rs`'s harness), on tokio's paused clock.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::time::Instant;

use jackioh_server::app::App;
use jackioh_server::config::{SPECTATE_DELAY_MS, SPECTATORS_PER_MATCH, WS_IDLE_TIMEOUT_SECONDS};
use jackioh_server::db::store::StartMatchInput;

use super::duos::{Account, PROMPTLY, Served, WsClient, account, api, ms, serve, settle, uuid};
use crate::support::deps::{TEST_CATALOG_VERSION, empty_test_app};
use crate::support::engine::{fake_deck, install_test_cards};
use crate::support::socket::{FakeSocket, create_fake_socket};

/// S7's watch socket.
const WATCH_PATH: &str = "/ws/watch";
/// How long after the watcher's hello the players act, so the delay is measured from their action.
const BEFORE_THE_PLAYERS_ACT: Duration = Duration::from_secs(5);
/// How late past `SPECTATE_DELAY_MS` a delayed view may still arrive.
const DELAY_SLACK: Duration = Duration::from_secs(1);
/// How far either side of `WS_IDLE_TIMEOUT_SECONDS` the hello gate's deadline is looked for.
const IDLE_SLACK: Duration = Duration::from_secs(1);

fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
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

/// A live 1v1 match between `p1` and `p2` on the real registry, both marked in it.
async fn start_match(app: &Arc<App>, match_id: &str, p1: &Account, p2: &Account, ranked: bool) {
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

/// The friend (p1) and a rival (p2) in a live match with their sockets attached; a watcher who is
/// the friend's friend; the app served.
struct World {
    app: Arc<App>,
    served: Served,
    match_id: String,
    friend: Account,
    watcher: Account,
    p1: FakeSocket,
    p2: FakeSocket,
}

async fn world(ranked: bool) -> World {
    install_test_cards();
    let app = empty_test_app().await;
    let friend = account(&app, 11, "Fern").await;
    let rival = account(&app, 12, "Rex").await;
    let watcher = account(&app, 13, "Wes").await;
    befriend(&app, &watcher, &friend).await;
    let match_id = uuid(400);
    start_match(&app, &match_id, &friend, &rival, ranked).await;
    let p1 = create_fake_socket();
    let p2 = create_fake_socket();
    app.matches
        .attach(&app, &match_id, &friend.id, p1.socket())
        .await
        .expect("the friend attaches");
    app.matches
        .attach(&app, &match_id, &rival.id, p2.socket())
        .await
        .expect("the rival attaches");
    settle().await;
    let served = serve(&app);
    World {
        app,
        served,
        match_id,
        friend,
        watcher,
        p1,
        p2,
    }
}

impl World {
    /// `who` says hello to watch the friend in this match.
    async fn watch(&self, who: &Account) -> WsClient {
        self.watch_match(&self.match_id, who).await
    }

    async fn watch_match(&self, match_id: &str, who: &Account) -> WsClient {
        let client = self.served.connect(WATCH_PATH, &who.token).await;
        client
            .send_json(json!({ "type": "hello", "matchId": match_id, "friendId": self.friend.id }))
            .await;
        settle().await;
        client
    }

    /// Both players keep their whole opening hand: the friend's turn 1 begins.
    async fn keep_both(&self) {
        for (socket, nonce) in [(&self.p1, "keep-p1"), (&self.p2, "keep-p2")] {
            let view = last_view_of(socket);
            let keep: Vec<Value> = view["you"]["hand"]
                .as_array()
                .expect("a hand")
                .iter()
                .map(|card| card["instanceId"].clone())
                .collect();
            socket.receive_json(json!({
                "type": "action",
                "action": { "type": "mulligan", "keep": keep, "nonce": nonce },
            }));
            settle().await;
        }
    }
}

/// A player's latest `PlayerView`.
fn last_view_of(socket: &FakeSocket) -> Value {
    socket
        .of_type("view")
        .last()
        .map(|frame| frame["view"].clone())
        .expect("the player has a view")
}

fn hand_ids(view: &Value) -> Vec<Value> {
    view["you"]["hand"]
        .as_array()
        .map(|hand| hand.iter().map(|card| card["instanceId"].clone()).collect())
        .unwrap_or_default()
}

/// The counts a player's match socket was told, in order.
fn watcher_counts(socket: &FakeSocket) -> Vec<Value> {
    socket
        .of_type("watchers")
        .iter()
        .map(|frame| frame["count"].clone())
        .collect()
}

/// `forbidden`, then the socket closes, and no view ever arrived.
async fn assert_forbidden(client: &WsClient) {
    let refusal = client
        .wait_for(0, PROMPTLY, |frame| {
            frame["type"] == "error" && frame["code"] == "forbidden"
        })
        .await;
    assert!(refusal.is_some(), "no forbidden: {:?}", client.frames());
    assert!(
        client.wait_closed(PROMPTLY).await.is_some(),
        "the socket stays open after forbidden"
    );
    assert!(client.of_type("view").is_empty(), "{:?}", client.frames());
}

// ---------------------------------------------------------------------------------------------
// B50 — who may watch, and what they see
// ---------------------------------------------------------------------------------------------

mod b50_watching_a_friend {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b50_a_friend_watches_the_friends_seat_spectate_delay_ms_late_with_no_legal_actions() {
        let w = world(false).await;
        let watcher = w.watch(&w.watcher).await;
        assert!(watcher.of_type("error").is_empty(), "{:?}", watcher.frames());
        tokio::time::sleep(BEFORE_THE_PLAYERS_ACT).await;
        let acted = Instant::now();
        w.keep_both().await;
        let friends_view = last_view_of(&w.p1);
        assert_eq!(friends_view["phase"], "main", "{friends_view}");
        let delay = ms(SPECTATE_DELAY_MS);

        let (_, at, frame) = watcher
            .wait_for(0, delay + DELAY_SLACK, |frame| {
                frame["type"] == "view" && frame["view"]["phase"] == "main"
            })
            .await
            .unwrap_or_else(|| panic!("the watcher never saw the turn begin: {:?}", watcher.frames()));

        assert!(
            at >= acted + delay,
            "shown {:?} after the state, before the delay",
            at - acted
        );
        assert_eq!(frame["legal"], json!([]), "{frame}");
        assert_eq!(frame["view"]["viewer"], "p1", "the friend's seat: {frame}");
        assert_eq!(
            hand_ids(&frame["view"]),
            hand_ids(&friends_view),
            "the friend's own hand"
        );
        for view in watcher.of_type("view") {
            assert_eq!(view["legal"], json!([]), "{view}");
            assert_eq!(view["view"]["viewer"], "p1", "{view}");
        }
        assert!(watcher.of_type("error").is_empty(), "{:?}", watcher.frames());
    }

    #[tokio::test(start_paused = true)]
    async fn b50_a_ranked_match_is_forbidden() {
        let w = world(true).await;

        let watcher = w.watch(&w.watcher).await;

        assert_forbidden(&watcher).await;
    }

    #[tokio::test(start_paused = true)]
    async fn b50_a_friend_who_turned_spectators_off_is_forbidden() {
        let w = world(false).await;
        let (status, body) = api(
            &w.app,
            "PUT",
            "/api/settings",
            &w.friend.token,
            json!({ "groups": {}, "allowSpectators": false }),
        )
        .await;
        assert_eq!(status, 200, "{body}");

        let watcher = w.watch(&w.watcher).await;

        assert_forbidden(&watcher).await;
    }

    #[tokio::test(start_paused = true)]
    async fn b50_b51_a_full_match_is_forbidden_to_one_watcher_more() {
        let w = world(false).await;
        let cap = count(SPECTATORS_PER_MATCH);
        let mut admitted = vec![w.watch(&w.watcher).await];
        for k in 1..cap {
            let fan = account(&w.app, 20 + u32::try_from(k).expect("a small index"), "Fan").await;
            befriend(&w.app, &fan, &w.friend).await;
            admitted.push(w.watch(&fan).await);
        }
        settle().await;
        for client in &admitted {
            assert!(client.of_type("error").is_empty(), "{:?}", client.frames());
            assert!(client.close_code().is_none(), "an admitted watcher stays");
        }
        let late = account(&w.app, 90, "Late").await;
        befriend(&w.app, &late, &w.friend).await;

        let refused = w.watch(&late).await;

        assert_forbidden(&refused).await;
        assert_eq!(watcher_counts(&w.p1).last(), Some(&json!(cap)));
        assert_eq!(watcher_counts(&w.p2).last(), Some(&json!(cap)));
    }

    #[tokio::test(start_paused = true)]
    async fn b50_a_watcher_who_is_not_the_players_friend_is_forbidden() {
        let w = world(false).await;
        let stranger = account(&w.app, 30, "Stan").await;

        let watcher = w.watch(&stranger).await;

        assert_forbidden(&watcher).await;
        assert!(
            watcher_counts(&w.p1).iter().all(|count| *count == json!(0)),
            "nobody joined: {:?}",
            watcher_counts(&w.p1)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn b50_b46_a_block_either_way_is_forbidden() {
        let w = world(false).await;
        let other = account(&w.app, 31, "Ola").await;
        befriend(&w.app, &other, &w.friend).await;
        // The friend blocks the first watcher; the second blocks the friend.
        block(&w.app, &w.friend, &w.watcher).await;
        block(&w.app, &other, &w.friend).await;

        for who in [&w.watcher, &other] {
            let watcher = w.watch(who).await;
            assert_forbidden(&watcher).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b50_a_match_the_friend_does_not_play_is_forbidden() {
        let w = world(false).await;
        let left = account(&w.app, 32, "Lee").await;
        let right = account(&w.app, 33, "Rae").await;
        let elsewhere = uuid(401);
        start_match(&w.app, &elsewhere, &left, &right, false).await;

        for match_id in [elsewhere, uuid(402)] {
            let watcher = w.watch_match(&match_id, &w.watcher).await;
            assert_forbidden(&watcher).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b50_a_watchers_frames_after_hello_never_reach_the_reducer() {
        let w = world(false).await;
        w.keep_both().await;
        let watcher = w.watch(&w.watcher).await;
        settle().await;
        w.p1.clear();
        w.p2.clear();

        watcher
            .send_json(json!({ "type": "action", "action": { "type": "endTurn", "nonce": "w-end" } }))
            .await;
        watcher
            .send_json(json!({
                "type": "action",
                "nonce": "w-duos",
                "body": { "kind": "duel", "body": { "type": "endTurn" } },
            }))
            .await;
        watcher
            .send_json(json!({ "type": "hello", "matchId": w.match_id, "friendId": w.friend.id }))
            .await;
        watcher.send_text("not json").await;
        settle().await;

        for socket in [&w.p1, &w.p2] {
            assert!(socket.of_type("view").is_empty(), "{:?}", socket.messages());
            assert!(socket.of_type("ack").is_empty(), "{:?}", socket.messages());
        }
        assert!(watcher.of_type("ack").is_empty(), "{:?}", watcher.frames());
        assert!(
            watcher.of_type("error").is_empty(),
            "ignored, not answered: {:?}",
            watcher.frames()
        );
        // The friend's own end of turn still lands: the reducer was there all along.
        w.p1.receive_json(json!({ "type": "action", "action": { "type": "endTurn", "nonce": "f-end" } }));
        settle().await;
        assert_eq!(last_view_of(&w.p2)["active"], "p2");
    }

    #[tokio::test(start_paused = true)]
    async fn b50_b51_a_watcher_leaving_changes_nothing_in_the_match_but_the_count() {
        let w = world(false).await;
        w.keep_both().await;
        let watcher = w.watch(&w.watcher).await;
        settle().await;
        assert_eq!(watcher_counts(&w.p1).last(), Some(&json!(1)));
        w.p1.clear();
        w.p2.clear();

        watcher.hang_up().await;
        settle().await;

        for socket in [&w.p1, &w.p2] {
            assert!(socket.of_type("view").is_empty(), "{:?}", socket.messages());
            assert_eq!(watcher_counts(socket), vec![json!(0)]);
        }
        let presence = w.app.matches.presence_of(&w.match_id).expect("the match is live");
        assert!(presence.p1 && presence.p2, "both players are still there");
    }
}

// ---------------------------------------------------------------------------------------------
// B51 — the count on the players' sockets
// ---------------------------------------------------------------------------------------------

mod b51_the_watcher_count {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b51_the_players_match_sockets_hear_the_watcher_count_each_time_it_changes() {
        let w = world(false).await;
        let second = account(&w.app, 14, "Wil").await;
        befriend(&w.app, &second, &w.friend).await;
        w.p1.clear();
        w.p2.clear();

        let first = w.watch(&w.watcher).await;
        let _second = w.watch(&second).await;
        first.hang_up().await;
        settle().await;

        for socket in [&w.p1, &w.p2] {
            assert_eq!(watcher_counts(socket), vec![json!(1), json!(2), json!(1)]);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B50, B46 — a watch the friend withdraws, or that an unfriend or a block ends, ends at once
// ---------------------------------------------------------------------------------------------

mod b50_b46_a_watch_ends_when_its_leave_does {
    use super::*;

    /// `forbidden`, as a refused hello gets, then the socket closes with R148's 4403.
    async fn assert_let_go(client: &WsClient) {
        let refusal = client
            .wait_for(0, PROMPTLY, |frame| {
                frame["type"] == "error" && frame["code"] == "forbidden"
            })
            .await;
        assert!(refusal.is_some(), "no forbidden: {:?}", client.frames());
        assert_eq!(
            client.wait_closed(PROMPTLY).await,
            Some(4403),
            "{:?}",
            client.frames()
        );
    }

    /// The world with a second friend of the friend's watching too, both admitted; the players' counts
    /// cleared.
    async fn two_watching() -> (World, Account, WsClient, WsClient) {
        let w = world(false).await;
        let other = account(&w.app, 15, "Oda").await;
        befriend(&w.app, &other, &w.friend).await;
        let first = w.watch(&w.watcher).await;
        let second = w.watch(&other).await;
        settle().await;
        for client in [&first, &second] {
            assert!(client.of_type("error").is_empty(), "{:?}", client.frames());
            assert!(client.close_code().is_none(), "admitted");
        }
        assert_eq!(watcher_counts(&w.p1).last(), Some(&json!(2)));
        w.p1.clear();
        w.p2.clear();
        (w, other, first, second)
    }

    #[tokio::test(start_paused = true)]
    async fn b50_turning_watching_off_lets_every_watcher_go_and_the_players_hear_the_count() {
        let (w, _other, first, second) = two_watching().await;

        let (status, body) = api(
            &w.app,
            "PUT",
            "/api/settings",
            &w.friend.token,
            json!({ "allowSpectators": false }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        settle().await;

        assert_let_go(&first).await;
        assert_let_go(&second).await;
        for socket in [&w.p1, &w.p2] {
            assert_eq!(
                watcher_counts(socket).last(),
                Some(&json!(0)),
                "{:?}",
                socket.messages()
            );
        }
        assert_eq!(w.app.matches.watcher_count(&w.match_id), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn b50_turning_watching_off_with_the_settings_groups_lets_the_watcher_go_too() {
        let w = world(false).await;
        let watcher = w.watch(&w.watcher).await;
        assert_eq!(watcher_counts(&w.p1).last(), Some(&json!(1)));

        let (status, body) = api(
            &w.app,
            "PUT",
            "/api/settings",
            &w.friend.token,
            json!({ "groups": {}, "allowSpectators": false }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        settle().await;

        assert_let_go(&watcher).await;
        assert_eq!(watcher_counts(&w.p1).last(), Some(&json!(0)));
    }

    #[tokio::test(start_paused = true)]
    async fn b50_b45_an_unfriend_either_way_lets_that_watcher_go_and_no_other() {
        for friend_removes in [true, false] {
            let (w, _other, first, second) = two_watching().await;
            let (who, whom) = if friend_removes {
                (&w.friend, &w.watcher)
            } else {
                (&w.watcher, &w.friend)
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
            settle().await;

            assert_let_go(&first).await;
            assert!(second.close_code().is_none(), "the other friend still watches");
            assert!(second.of_type("error").is_empty(), "{:?}", second.frames());
            for socket in [&w.p1, &w.p2] {
                assert_eq!(watcher_counts(socket), vec![json!(1)], "{:?}", socket.messages());
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b50_b46_a_block_either_way_lets_that_watcher_go_and_no_other() {
        for friend_blocks in [true, false] {
            let (w, _other, first, second) = two_watching().await;
            let (who, whom) = if friend_blocks {
                (&w.friend, &w.watcher)
            } else {
                (&w.watcher, &w.friend)
            };

            block(&w.app, who, whom).await;
            settle().await;

            assert_let_go(&first).await;
            assert!(second.close_code().is_none(), "the other friend still watches");
            for socket in [&w.p1, &w.p2] {
                assert_eq!(watcher_counts(socket), vec![json!(1)], "{:?}", socket.messages());
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// R835002 — the gate before the hello
// ---------------------------------------------------------------------------------------------

mod r835002_the_hello_gate {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r835002_a_watch_socket_that_sends_no_hello_is_refused_after_the_idle_timeout() {
        let w = world(false).await;
        let idle = Duration::from_secs(u64::try_from(WS_IDLE_TIMEOUT_SECONDS).expect("a span"));

        // Signed in, and then silent: the pings it answers keep it alive, the deadline does not.
        let client = w.served.connect(WATCH_PATH, &w.watcher.token).await;
        assert_eq!(client.wait_closed(idle - IDLE_SLACK).await, None);
        assert_eq!(client.wait_closed(IDLE_SLACK * 2).await, Some(4403));

        assert!(
            client
                .of_type("error")
                .iter()
                .any(|frame| frame["code"] == "forbidden"),
            "{:?}",
            client.frames()
        );
        assert!(client.of_type("view").is_empty(), "{:?}", client.frames());
        assert_eq!(w.app.matches.watcher_count(&w.match_id), 0);
    }
}
