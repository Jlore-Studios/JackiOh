//! R3214000: `/ws/social`, one live socket per signed-in tab, over the whole app (`app::router`) served
//! on in-memory pipes (`super::duos::serve`). It signs in as `/ws/match` does, sends its account's own
//! social read first, then a `social`, `chat` or `friends` frame to each account a change concerns and
//! to no other, checks its session again every `SOCIAL_SESSION_CHECK_SECONDS`, and is capped per
//! account and per address apart from the other sockets.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_server::actor::ws_server::{SOCIAL_PATH, WATCH_PATH, WS_CLOSE, WS_SUBPROTOCOL, bearer_protocol};
use jackioh_server::api::http::account_key;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    API_REQUESTS_PER_MINUTE, AUTH_SESSION_LIVE_CACHE_SECONDS, CHALLENGE_TTL_MS, MALFORMED_FRAMES_PER_WINDOW,
    SOCIAL_SESSION_CHECK_SECONDS, SOCIAL_SOCKETS_PER_ACCOUNT, WS_MAX_CONNECTIONS_PER_ADDRESS,
};
use jackioh_server::db::store::ProfileStatus;
use jackioh_server::social::live::TOO_MANY_TABS;

use super::duos::{
    PROMPTLY, Served, WsClient, api, assert_refused_for_its_sign_in, ms, offers_without_a_credential, serve,
    settle,
};
use crate::support::deps::{add_user, empty_test_app, fake_auth};
use crate::support::store::{Account, account, befriend, fake, uuid};

/// RFC 6455's policy violation: a socket past its malformed budget (R835002).
const CLOSE_POLICY_VIOLATED: u16 = 1008;

/// The session check's interval as a duration.
fn check_interval() -> Duration {
    Duration::from_secs(u64::try_from(SOCIAL_SESSION_CHECK_SECONDS).expect("a positive interval"))
}

/// `who`'s social socket, once its first frame, its own social read, has arrived.
async fn connected(served: &Served, who: &Account) -> WsClient {
    let client = served.connect(SOCIAL_PATH, &who.token).await;
    client
        .wait_for(0, PROMPTLY, |frame| frame["type"] == "social")
        .await
        .unwrap_or_else(|| panic!("no first social frame: {:?}", client.frames()));
    client
}

/// The next `social` frame from index `from` on.
async fn next_social(client: &WsClient, from: usize) -> Value {
    client
        .wait_for(from, PROMPTLY, |frame| frame["type"] == "social")
        .await
        .unwrap_or_else(|| panic!("no social frame from {from}: {:?}", client.frames()))
        .2
}

/// Asserts `client` is sent nothing from index `from` on.
async fn assert_told_nothing(client: &WsClient, from: usize, who: &str) {
    let told = client.wait_for(from, PROMPTLY, |_| true).await;
    assert!(told.is_none(), "{who} was told {:?}", client.frames());
}

/// The refusal frame and the close code a refused socket gets.
async fn refusal(client: &WsClient) -> (Value, Option<u16>) {
    let (_, _, frame) = client
        .wait_for(0, PROMPTLY, |frame| {
            frame["type"] == "error" && frame["code"] == "forbidden"
        })
        .await
        .unwrap_or_else(|| panic!("no forbidden: {:?}", client.frames()));
    (frame, client.wait_closed(PROMPTLY).await)
}

fn len(list: &Value) -> usize {
    list.as_array().map_or(0, Vec::len)
}

async fn heartbeat(app: &Arc<App>, who: &Account) {
    let (status, body) = api(app, "POST", "/api/presence", &who.token, Value::Null).await;
    assert_eq!(status, 204, "{body}");
}

/// `from` challenges `to` to an All Random room: `(challengeId, roomCode)`.
async fn challenge(app: &Arc<App>, from: &Account, to: &Account) -> (String, String) {
    let (status, body) = api(
        app,
        "POST",
        "/api/challenges",
        &from.token,
        json!({ "mode": "random", "friendId": to.id }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    (
        body["challengeId"].as_str().expect("a challenge id").to_string(),
        body["roomCode"].as_str().expect("a room code").to_string(),
    )
}

/// Ana, Bob and Cy, each a friend of the other two and online, so Ana may challenge Bob.
async fn three_friends(app: &Arc<App>) -> (Account, Account, Account) {
    jackioh_cards::register_all();
    let ana = account(app, 1, "Ana").await;
    let bob = account(app, 2, "Bob").await;
    let cy = account(app, 3, "Cy").await;
    befriend(app, &ana, &bob).await;
    befriend(app, &ana, &cy).await;
    befriend(app, &bob, &cy).await;
    for who in [&ana, &bob, &cy] {
        heartbeat(app, who).await;
    }
    (ana, bob, cy)
}

async fn set_status(app: &Arc<App>, profile_id: &str, status: ProfileStatus) {
    let mut tx = app.db.begin(None).await.expect("a store transaction begins");
    tx.profiles_set_status(profile_id, status)
        .await
        .expect("the status is written");
    tx.commit().await.expect("the status commits");
}

mod r3214000_signing_in {
    //! The upgrade signs in as `/ws/match`'s does (R1379000, R2478000, §9.4's gate), and the socket's
    //! first frame is its own account's social read.
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_socket_is_sent_its_own_social_read_and_only_jackioh_v1_is_echoed() {
        let app = empty_test_app().await;
        let ana = account(&app, 1, "Ana").await;
        let served = serve(&app);

        let offered = bearer_protocol(&ana.token);
        let (client, echoed) = served
            .connect_offering(SOCIAL_PATH, &[WS_SUBPROTOCOL, &offered])
            .await;
        assert_eq!(echoed.as_deref(), Some(WS_SUBPROTOCOL));
        let first = next_social(&client, 0).await;
        assert_eq!(
            first,
            json!({ "type": "social", "incoming": [], "outgoing": [], "unread": {} })
        );
        assert!(client.close_code().is_none());
        for frame in client.frames() {
            let text = frame.to_string();
            assert!(!text.contains(&ana.token) && !text.contains(&offered), "{text}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_r1379000_a_socket_with_no_usable_credential_is_refused_with_4401() {
        let app = empty_test_app().await;
        let served = serve(&app);
        for offer in offers_without_a_credential() {
            let protocols: Vec<&str> = offer.iter().map(String::as_str).collect();
            let (client, _) = served.connect_offering(SOCIAL_PATH, &protocols).await;
            assert_refused_for_its_sign_in(&client, "token-expired").await;
            assert!(
                client.of_type("social").is_empty(),
                "{offer:?}: {:?}",
                client.frames()
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_pending_or_banned_account_is_refused_with_4403() {
        let app = empty_test_app().await;
        let served = serve(&app);
        for (n, status, message) in [
            (1u32, "pending", "redeem an invite code to activate this account"),
            (2, "banned", "this account is banned"),
        ] {
            let user_id = format!("user-{n}");
            fake(&app).lock().await.seed_profile(json!({
                "id": uuid(n),
                "userId": user_id,
                "status": status,
            }));
            let token = add_user(&app, &user_id, &format!("{user_id}@example.test"), true);
            let client = served.connect(SOCIAL_PATH, &token).await;
            let (frame, close) = refusal(&client).await;
            assert_eq!(frame["message"], message, "{status}");
            assert_eq!(close, Some(WS_CLOSE.forbidden), "{status}");
            assert!(
                client.of_type("social").is_empty(),
                "{status}: {:?}",
                client.frames()
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_r2478000_an_upgrade_past_the_accounts_budget_is_closed_with_4429() {
        let app = empty_test_app().await;
        let ana = account(&app, 1, "Ana").await;
        for _ in 0..API_REQUESTS_PER_MINUTE {
            assert!(app.limiter.allow(&account_key(&ana.id), now_ms()));
        }
        let served = serve(&app);
        let client = served.connect(SOCIAL_PATH, &ana.token).await;
        let (frame, close) = refusal(&client).await;
        assert_eq!(frame["message"], "too many requests; slow down");
        assert_eq!(close, Some(WS_CLOSE.rate_limited));
        assert!(client.of_type("social").is_empty(), "{:?}", client.frames());
    }
}

mod r3214000_what_is_pushed {
    //! Each change reaches the accounts it concerns, each with its own read, and no other account.
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_challenge_and_its_answer_reach_its_two_accounts_and_no_one_else() {
        let app = empty_test_app().await;
        let (ana, bob, cy) = three_friends(&app).await;
        let served = serve(&app);
        let (ana_ws, bob_ws, cy_ws) = (
            connected(&served, &ana).await,
            connected(&served, &bob).await,
            connected(&served, &cy).await,
        );
        let (from_ana, from_bob, from_cy) = (ana_ws.received(), bob_ws.received(), cy_ws.received());

        let (challenge_id, _) = challenge(&app, &ana, &bob).await;
        let ana_read = next_social(&ana_ws, from_ana).await;
        assert_eq!(len(&ana_read["outgoing"]), 1, "{ana_read}");
        assert_eq!(ana_read["outgoing"][0]["challengeId"], challenge_id.as_str());
        assert_eq!(len(&ana_read["incoming"]), 0, "{ana_read}");
        let bob_read = next_social(&bob_ws, from_bob).await;
        assert_eq!(len(&bob_read["incoming"]), 1, "{bob_read}");
        assert_eq!(bob_read["incoming"][0]["challengeId"], challenge_id.as_str());
        assert_eq!(len(&bob_read["outgoing"]), 0, "{bob_read}");

        let (from_ana, from_bob) = (ana_ws.received(), bob_ws.received());
        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/challenges/{challenge_id}/decline"),
            &bob.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 204, "{body}");
        for (client, from, who) in [(&ana_ws, from_ana, "Ana"), (&bob_ws, from_bob, "Bob")] {
            let read = next_social(client, from).await;
            assert_eq!(
                (len(&read["incoming"]), len(&read["outgoing"])),
                (0, 0),
                "{who}: {read}"
            );
        }
        assert_told_nothing(&cy_ws, from_cy, "Cy").await;
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_lapsed_challenge_leaves_both_sockets() {
        let app = empty_test_app().await;
        let (ana, bob, _cy) = three_friends(&app).await;
        let served = serve(&app);
        let (ana_ws, bob_ws) = (connected(&served, &ana).await, connected(&served, &bob).await);
        challenge(&app, &ana, &bob).await;
        settle().await;
        let (from_ana, from_bob) = (ana_ws.received(), bob_ws.received());

        tokio::time::sleep(ms(CHALLENGE_TTL_MS)).await;
        for (client, from, who) in [(&ana_ws, from_ana, "Ana"), (&bob_ws, from_bob, "Bob")] {
            let read = next_social(client, from).await;
            assert_eq!(
                (len(&read["incoming"]), len(&read["outgoing"])),
                (0, 0),
                "{who}: {read}"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_an_accepted_challenge_reaches_the_challenger_once_its_game_started() {
        let app = empty_test_app().await;
        let (ana, bob, _cy) = three_friends(&app).await;
        let served = serve(&app);
        let ana_ws = connected(&served, &ana).await;
        let from_ana = ana_ws.received();
        let (_, room_code) = challenge(&app, &ana, &bob).await;
        next_social(&ana_ws, from_ana).await;
        let from_ana = ana_ws.received();

        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/rooms/{room_code}/join"),
            &bob.token,
            json!({ "mode": "random" }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        let read = next_social(&ana_ws, from_ana).await;
        assert_eq!(len(&read["outgoing"]), 0, "{read}");
        // The challenger follows its friend into the game it is told of (`ChallengeToast.tsx`).
        let (status, me) = api(&app, "GET", "/api/auth/me", &ana.token, Value::Null).await;
        assert_eq!(status, 200, "{me}");
        assert_eq!(me["currentMatchId"], body["matchId"], "{me}");
        assert!(me["currentMatchId"].is_string(), "{me}");
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_chat_message_reaches_its_two_accounts_and_carries_no_body() {
        let app = empty_test_app().await;
        let (ana, bob, cy) = three_friends(&app).await;
        let served = serve(&app);
        let (ana_ws, bob_ws, cy_ws) = (
            connected(&served, &ana).await,
            connected(&served, &bob).await,
            connected(&served, &cy).await,
        );
        let (from_ana, from_bob, from_cy) = (ana_ws.received(), bob_ws.received(), cy_ws.received());

        let words = "meet me in the arena at nine";
        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/chat/{}", bob.id),
            &ana.token,
            json!({ "body": words }),
        )
        .await;
        assert_eq!(status, 201, "{body}");

        let (_, _, nudge) = bob_ws
            .wait_for(from_bob, PROMPTLY, |frame| frame["type"] == "chat")
            .await
            .unwrap_or_else(|| panic!("Bob: no chat frame: {:?}", bob_ws.frames()));
        assert_eq!(nudge, json!({ "type": "chat", "friendId": ana.id }));
        let read = next_social(&bob_ws, from_bob).await;
        assert_eq!(read["unread"], json!({ ana.id.clone(): 1 }), "{read}");
        let (_, _, nudge) = ana_ws
            .wait_for(from_ana, PROMPTLY, |frame| frame["type"] == "chat")
            .await
            .unwrap_or_else(|| panic!("Ana: no chat frame: {:?}", ana_ws.frames()));
        assert_eq!(nudge, json!({ "type": "chat", "friendId": bob.id }));
        assert_told_nothing(&cy_ws, from_cy, "Cy").await;
        for client in [&ana_ws, &bob_ws, &cy_ws] {
            for frame in client.frames() {
                assert!(!frame.to_string().contains(words), "{frame}");
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_read_reaches_only_its_reader_and_only_when_it_marked_one() {
        let app = empty_test_app().await;
        let (ana, bob, _cy) = three_friends(&app).await;
        let served = serve(&app);
        let (ana_ws, bob_ws) = (connected(&served, &ana).await, connected(&served, &bob).await);
        let (status, body) = api(
            &app,
            "POST",
            &format!("/api/chat/{}", bob.id),
            &ana.token,
            json!({ "body": "hi" }),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        settle().await;
        let (from_ana, from_bob) = (ana_ws.received(), bob_ws.received());

        let conversation = format!("/api/chat/{}", ana.id);
        let (status, body) = api(&app, "GET", &conversation, &bob.token, Value::Null).await;
        assert_eq!(status, 200, "{body}");
        let read = next_social(&bob_ws, from_bob).await;
        assert_eq!(read["unread"], json!({}), "{read}");
        assert_told_nothing(&ana_ws, from_ana, "Ana").await;

        // Nothing left to mark: nothing is pushed.
        let from_bob = bob_ws.received();
        let (status, body) = api(&app, "GET", &conversation, &bob.token, Value::Null).await;
        assert_eq!(status, 200, "{body}");
        assert_told_nothing(&bob_ws, from_bob, "Bob").await;
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_friends_list_changes_reach_both_and_a_block_of_a_stranger_tells_them_nothing() {
        let app = empty_test_app().await;
        let ana = account(&app, 1, "Ana").await;
        let bob = account(&app, 2, "Bob").await;
        let cy = account(&app, 3, "Cy").await;
        let dee = account(&app, 4, "Dee").await;
        befriend(&app, &ana, &bob).await;
        let served = serve(&app);
        let (ana_ws, bob_ws, cy_ws, dee_ws) = (
            connected(&served, &ana).await,
            connected(&served, &bob).await,
            connected(&served, &cy).await,
            connected(&served, &dee).await,
        );
        let told_of_friends = |client: &WsClient, from: usize| client.of_type_since(from, "friends").len();

        // A request: both lists show it.
        let from: Vec<usize> = [&ana_ws, &bob_ws, &cy_ws, &dee_ws]
            .iter()
            .map(|client| client.received())
            .collect();
        let (status, body) = api(
            &app,
            "POST",
            "/api/friends/requests",
            &ana.token,
            json!({ "username": cy.name }),
        )
        .await;
        assert_eq!(status, 201, "{body}");
        settle().await;
        assert_eq!(told_of_friends(&ana_ws, from[0]), 1, "{:?}", ana_ws.frames());
        assert_eq!(told_of_friends(&cy_ws, from[2]), 1, "{:?}", cy_ws.frames());
        assert_told_nothing(&bob_ws, from[1], "Bob").await;
        assert_told_nothing(&dee_ws, from[3], "Dee").await;

        // A block of a stranger: only the blocker's list changed, and the stranger learns nothing.
        let (from_ana, from_dee) = (ana_ws.received(), dee_ws.received());
        let (status, body) = api(
            &app,
            "POST",
            "/api/blocks",
            &ana.token,
            json!({ "profileId": dee.id }),
        )
        .await;
        assert_eq!(status, 204, "{body}");
        settle().await;
        assert_eq!(told_of_friends(&ana_ws, from_ana), 1, "{:?}", ana_ws.frames());
        assert_told_nothing(&dee_ws, from_dee, "Dee").await;

        // A block of a friend: both lists lose the other, and both are read again.
        let (from_ana, from_bob) = (ana_ws.received(), bob_ws.received());
        let (status, body) = api(
            &app,
            "POST",
            "/api/blocks",
            &ana.token,
            json!({ "profileId": bob.id }),
        )
        .await;
        assert_eq!(status, 204, "{body}");
        settle().await;
        assert_eq!(told_of_friends(&ana_ws, from_ana), 1, "{:?}", ana_ws.frames());
        assert_eq!(told_of_friends(&bob_ws, from_bob), 1, "{:?}", bob_ws.frames());
        next_social(&bob_ws, from_bob).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_hello_is_answered_with_a_social_read_and_junk_closes_past_the_budget() {
        let app = empty_test_app().await;
        let ana = account(&app, 1, "Ana").await;
        let served = serve(&app);
        let ana_ws = connected(&served, &ana).await;
        let from = ana_ws.received();

        // Two hellos inside one interval cost one read.
        ana_ws.send_json(json!({ "type": "hello" })).await;
        ana_ws.send_json(json!({ "type": "hello" })).await;
        next_social(&ana_ws, from).await;
        settle().await;
        assert_eq!(
            ana_ws.of_type_since(from, "social").len(),
            1,
            "{:?}",
            ana_ws.frames()
        );

        let from = ana_ws.received();
        for _ in 0..=MALFORMED_FRAMES_PER_WINDOW {
            ana_ws.send_text("not a hello").await;
        }
        assert_eq!(ana_ws.wait_closed(PROMPTLY).await, Some(CLOSE_POLICY_VIOLATED));
        let malformed = ana_ws.of_type_since(from, "error");
        assert!(
            !malformed.is_empty() && malformed.iter().all(|frame| frame["code"] == "malformed"),
            "{malformed:?}"
        );
    }
}

mod r3214000_the_session_while_open {
    //! The session is checked again every `SOCIAL_SESSION_CHECK_SECONDS`, an R194 window at most.
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r3214000_a_revoked_session_is_closed_within_the_r194_window() {
        const { assert!(SOCIAL_SESSION_CHECK_SECONDS <= AUTH_SESSION_LIVE_CACHE_SECONDS) };
        let app = empty_test_app().await;
        let ana = account(&app, 1, "Ana").await;
        let served = serve(&app);
        let ana_ws = connected(&served, &ana).await;

        fake_auth(&app).revoke_token(&ana.token);
        assert_eq!(
            ana_ws.wait_closed(check_interval() + PROMPTLY).await,
            Some(WS_CLOSE.unauthorized)
        );
        let refusal = ana_ws.of_type("error");
        assert_eq!(refusal.len(), 1, "{refusal:?}");
        assert_eq!(refusal[0]["code"], "forbidden");
        assert_eq!(refusal[0]["message"], "sign in first");
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_an_account_banned_while_open_is_closed_at_the_next_check() {
        let app = empty_test_app().await;
        let ana = account(&app, 1, "Ana").await;
        let served = serve(&app);
        let ana_ws = connected(&served, &ana).await;

        // A session that holds keeps the socket open across checks.
        tokio::time::sleep(check_interval() * 2).await;
        assert!(ana_ws.close_code().is_none(), "{:?}", ana_ws.frames());

        set_status(&app, &ana.id, ProfileStatus::Banned).await;
        assert_eq!(
            ana_ws.wait_closed(check_interval() + PROMPTLY).await,
            Some(WS_CLOSE.forbidden)
        );
        let refusal = ana_ws.of_type("error");
        assert_eq!(refusal.len(), 1, "{refusal:?}");
        assert_eq!(refusal[0]["message"], "this account is banned");
    }
}

mod r3214000_the_connection_limits {
    //! `SOCIAL_SOCKETS_PER_ACCOUNT` per account, and a per-address count apart from the other sockets'.
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r3214000_an_account_holds_at_most_social_sockets_per_account() {
        let app = empty_test_app().await;
        let ana = account(&app, 1, "Ana").await;
        let bob = account(&app, 2, "Bob").await;
        let served = serve(&app);
        let mut held = Vec::new();
        for _ in 0..SOCIAL_SOCKETS_PER_ACCOUNT {
            held.push(connected(&served, &ana).await);
        }

        let past = served.connect(SOCIAL_PATH, &ana.token).await;
        let (frame, close) = refusal(&past).await;
        assert_eq!(frame["message"], TOO_MANY_TABS);
        assert_eq!(close, Some(WS_CLOSE.forbidden));
        assert!(past.of_type("social").is_empty(), "{:?}", past.frames());
        // Another account's cap is its own.
        let _bob_ws = connected(&served, &bob).await;

        // A tab that closes frees its place.
        held.remove(0).hang_up().await;
        let _again = connected(&served, &ana).await;
        assert!(held.iter().all(|client| client.close_code().is_none()));
    }

    #[tokio::test(start_paused = true)]
    async fn r3214000_social_sockets_count_apart_per_address_and_leave_other_slots_free() {
        let app = empty_test_app().await;
        let accounts_needed = WS_MAX_CONNECTIONS_PER_ADDRESS.div_ceil(SOCIAL_SOCKETS_PER_ACCOUNT);
        let mut accounts = Vec::new();
        for n in 0..accounts_needed {
            let n = u32::try_from(n).expect("a few accounts") + 1;
            accounts.push(account(&app, n, "Ana").await);
        }
        let served = serve(&app);
        let mut held = Vec::new();
        for k in 0..WS_MAX_CONNECTIONS_PER_ADDRESS {
            held.push(connected(&served, &accounts[k / SOCIAL_SOCKETS_PER_ACCOUNT]).await);
        }

        let protocols = [WS_SUBPROTOCOL, &bearer_protocol(&accounts[0].token)];
        assert_eq!(served.upgrade_status(SOCIAL_PATH, &protocols).await, 429);
        // The address's match, co-op and watch sockets are counted apart, so idle tabs take none of them.
        assert_eq!(served.upgrade_status(WATCH_PATH, &protocols).await, 101);
        assert!(held.iter().all(|client| client.close_code().is_none()));
    }
}
