//! R4008000, R4008001 (#4008, part of #3897): every door that answers right or wrong to a guessable value
//! holds a limit that counts guesses, per account and per address, and carries one guess a request or
//! frame. `crates/server/README.md`, Guessable values, lists the doors; a new door adds its test here.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use serde_json::{Value, json};

use jackioh_server::actor::protocol::{ClientMessage, parse_client_message};
use jackioh_server::app::App;
use jackioh_server::config::{
    API_REQUESTS_PER_MINUTE, CODE_ALPHABET, CODE_ATTEMPTS_PER_IP_PER_HOUR,
    CODE_ATTEMPTS_PER_PROFILE_PER_HOUR, CODE_TRIES_PER_ACCOUNT_PER_HOUR, CODE_TRIES_PER_ADDRESS_PER_HOUR,
    CODE_TRY_WINDOW_MS, FRIEND_REQUESTS_PER_ADDRESS_PER_HOUR, FRIEND_REQUESTS_PER_HOUR, ROOM_CODE_LENGTH,
};

use crate::support::deps::{add_user, empty_test_app, json_request, send, test_app};
use crate::support::store::{Account, account, advance, fake, fake_ref, uuid};

/// One address the tests use for the second address; the first is the harness's default.
const OTHER_IP: &str = "198.51.100.9";
const IP: &str = "198.51.100.7";

/// A well-formed invite code that is never minted.
const UNMINTED_INVITE: &str = "ABCD-EFGH-JKMN-PQRT";

/// A request from `ip`: its status and its JSON body.
async fn from_ip(
    app: &Arc<App>,
    method: &str,
    path: &str,
    token: &str,
    ip: &str,
    body: Value,
) -> (u16, Value) {
    let (status, _headers, body) = send(app, json_request(method, path, &body, Some(token), Some(ip))).await;
    (status, body)
}

/// The `n`th well-formed room code, so each `n` is a distinct code nobody holds.
fn unheld_code(n: usize) -> String {
    let alphabet: Vec<char> = CODE_ALPHABET.chars().collect();
    let base = alphabet.len();
    let mut rest = n;
    let mut digits = Vec::new();
    for _ in 0..ROOM_CODE_LENGTH {
        digits.push(alphabet[rest % base]);
        rest /= base;
    }
    digits.into_iter().rev().collect()
}

fn error_code(body: &Value) -> &str {
    body["error"]["code"].as_str().unwrap_or_default()
}

/// A pending account with a verified email, the caller §9.4's redemption is for.
async fn pending_account(app: &Arc<App>, n: usize) -> String {
    let id = format!("pending-{n}");
    let user_id = format!("user-{id}");
    let token = add_user(app, &user_id, &format!("{id}@example.test"), true);
    fake(app)
        .lock()
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": "pending" }));
    token
}

/// `GET /api/duos/rooms/:code`, the cheapest code door.
async fn duos_get(app: &Arc<App>, who: &Account, ip: &str, code: &str) -> (u16, Value) {
    from_ip(
        app,
        "GET",
        &format!("/api/duos/rooms/{code}"),
        &who.token,
        ip,
        Value::Null,
    )
    .await
}

/// The seven code doors, each asked for `code`.
async fn door(app: &Arc<App>, which: usize, who: &Account, ip: &str, code: &str) -> (u16, Value) {
    let (method, path, body) = match which {
        0 => (
            "POST",
            format!("/api/rooms/{code}/join"),
            json!({ "mode": "random" }),
        ),
        1 => ("DELETE", format!("/api/rooms/{code}"), Value::Null),
        2 => ("GET", format!("/api/duos/rooms/{code}"), Value::Null),
        3 => (
            "POST",
            format!("/api/duos/rooms/{code}/join"),
            json!({ "random": true }),
        ),
        4 => ("DELETE", format!("/api/duos/rooms/{code}"), Value::Null),
        5 => ("GET", format!("/api/lobbies/{code}"), Value::Null),
        _ => ("POST", format!("/api/lobbies/{code}/enter"), Value::Null),
    };
    from_ip(app, method, &path, &who.token, ip, body).await
}

const DOORS: usize = 7;

#[tokio::test(start_paused = true)]
async fn r4008000_a_wrong_room_code_past_the_accounts_allowance_is_refused_for_every_code() {
    let app = empty_test_app().await;
    let host = account(&app, 1, "Host").await;
    let guest = account(&app, 2, "Guest").await;
    let (status, room) = from_ip(
        &app,
        "POST",
        "/api/rooms",
        &host.token,
        IP,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 200, "{room}");
    let real = room["code"].as_str().expect("a room code").to_string();

    for n in 0..CODE_TRIES_PER_ACCOUNT_PER_HOUR {
        let path = format!("/api/rooms/{}/join", unheld_code(n));
        // The extra codes of the first body are read as nothing: they count for nothing.
        let body = if n == 0 {
            json!({ "mode": "random", "codes": ["AAAAAA", "BBBBBB"] })
        } else {
            json!({ "mode": "random" })
        };
        let (status, body) = from_ip(&app, "POST", &path, &guest.token, IP, body).await;
        assert_eq!(status, 404, "{body}");
    }
    let path = format!("/api/rooms/{}/join", unheld_code(CODE_TRIES_PER_ACCOUNT_PER_HOUR));
    let (status, body) = from_ip(&app, "POST", &path, &guest.token, IP, json!({ "mode": "random" })).await;
    assert_eq!((status, error_code(&body)), (429, "rate_limited"), "{body}");
    assert!(
        body["error"]["details"]["retryAfterMs"].as_i64().unwrap_or(0) > 0,
        "{body}"
    );
    // The real code is refused too: counting without refusing would hold nothing.
    let path = format!("/api/rooms/{real}/join");
    let (status, body) = from_ip(&app, "POST", &path, &guest.token, IP, json!({ "mode": "random" })).await;
    assert_eq!(status, 429, "{body}");

    // The window passes (and the first room lapses with it): the host's new room joins.
    advance(CODE_TRY_WINDOW_MS + 1).await;
    let (status, room) = from_ip(
        &app,
        "POST",
        "/api/rooms",
        &host.token,
        IP,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 200, "{room}");
    let path = format!("/api/rooms/{}/join", room["code"].as_str().expect("a room code"));
    let (status, body) = from_ip(&app, "POST", &path, &guest.token, IP, json!({ "mode": "random" })).await;
    assert_eq!(status, 200, "{body}");
}

#[tokio::test(start_paused = true)]
async fn r4008000_a_wrong_code_sent_again_counts_once_and_a_malformed_one_never() {
    let app = empty_test_app().await;
    let who = account(&app, 1, "Pal").await;
    let again = unheld_code(1);
    for _ in 0..CODE_TRIES_PER_ACCOUNT_PER_HOUR + 5 {
        assert_eq!(duos_get(&app, &who, IP, &again).await.0, 404);
    }
    for _ in 0..CODE_TRIES_PER_ACCOUNT_PER_HOUR + 5 {
        assert_eq!(duos_get(&app, &who, IP, "ZZZ").await.0, 404);
    }
    assert_eq!(duos_get(&app, &who, IP, &unheld_code(2)).await.0, 404);
}

/// Every future started before any is awaited; the answers in order. `release` runs once each has been
/// polled once, so a future held up by what `release` frees is held up with the others, not run alone.
async fn join_all<F: Future>(futures: Vec<F>, release: impl FnOnce()) -> Vec<F::Output> {
    let mut release = Some(release);
    let mut futures: Vec<Pin<Box<F>>> = futures.into_iter().map(Box::pin).collect();
    let mut outputs: Vec<Option<F::Output>> = futures.iter().map(|_| None).collect();
    std::future::poll_fn(|cx| {
        let mut pending = false;
        for (future, output) in futures.iter_mut().zip(outputs.iter_mut()) {
            if output.is_none() {
                match future.as_mut().poll(cx) {
                    Poll::Ready(value) => *output = Some(value),
                    Poll::Pending => pending = true,
                }
            }
        }
        if let Some(release) = release.take() {
            release();
        }
        if pending { Poll::Pending } else { Poll::Ready(()) }
    })
    .await;
    outputs
        .into_iter()
        .map(|output| output.expect("every future finished"))
        .collect()
}

#[tokio::test(start_paused = true)]
async fn r4008000_requests_sent_at_once_cannot_try_more_codes_than_the_allowance() {
    let app = empty_test_app().await;
    let who = account(&app, 1, "Pal").await;
    // The store is held while every request starts, so each stops inside its lookup; once all wait there
    // the store is freed and they finish one after another, none having counted before the others began.
    let store = fake_ref(&app).lock().await;
    let burst = CODE_TRIES_PER_ACCOUNT_PER_HOUR * 4;
    let codes: Vec<String> = (0..burst).map(unheld_code).collect();
    let answers = join_all(
        codes.iter().map(|code| duos_get(&app, &who, IP, code)).collect(),
        move || drop(store),
    )
    .await;
    let answered = answers.iter().filter(|(status, _)| *status == 404).count();
    let refused = answers.iter().filter(|(status, _)| *status == 429).count();
    assert_eq!(
        (answered, refused),
        (
            CODE_TRIES_PER_ACCOUNT_PER_HOUR,
            burst - CODE_TRIES_PER_ACCOUNT_PER_HOUR
        ),
        "{answers:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn r4008000_a_right_code_counts_as_a_guess_as_a_wrong_one_does() {
    let app = empty_test_app().await;
    let guesser = account(&app, 1, "Pal").await;
    // Co-op rooms nobody has joined are readable by a stranger; each is a code the guesser got right.
    for n in 0..CODE_TRIES_PER_ACCOUNT_PER_HOUR {
        let host = account(&app, 100 + n as u32, "Host").await;
        let (status, room) = from_ip(
            &app,
            "POST",
            "/api/duos/rooms",
            &host.token,
            OTHER_IP,
            json!({ "difficulty": "easy", "random": true }),
        )
        .await;
        assert_eq!(status, 201, "{room}");
        let code = room["code"].as_str().expect("a code").to_string();
        assert_eq!(duos_get(&app, &guesser, IP, &code).await.0, 200);
    }
    let (status, body) = duos_get(&app, &guesser, IP, &unheld_code(1)).await;
    assert_eq!((status, error_code(&body)), (429, "rate_limited"), "{body}");
}

#[tokio::test(start_paused = true)]
async fn r4008000_every_door_of_a_code_shares_one_allowance() {
    let app = empty_test_app().await;
    let who = account(&app, 1, "Pal").await;
    for n in 0..CODE_TRIES_PER_ACCOUNT_PER_HOUR {
        let which = n % DOORS;
        let (status, body) = door(&app, which, &who, IP, &unheld_code(n)).await;
        assert_ne!(status, 429, "door {which}: {body}");
        if which == 1 {
            assert_eq!((status, &body["cancelled"]), (200, &json!(false)), "{body}");
        }
    }
    for which in 0..DOORS {
        let (status, body) = door(&app, which, &who, IP, &unheld_code(100 + which)).await;
        assert_eq!(
            (status, error_code(&body)),
            (429, "rate_limited"),
            "door {which}: {body}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn r4008000_the_address_holds_its_own_allowance_across_accounts() {
    let app = empty_test_app().await;
    let mut spent = 0;
    let mut k = 0;
    while spent < CODE_TRIES_PER_ADDRESS_PER_HOUR {
        let pal = account(&app, 10 + k, "Pal").await;
        k += 1;
        for _ in 0..CODE_TRIES_PER_ACCOUNT_PER_HOUR.min(CODE_TRIES_PER_ADDRESS_PER_HOUR - spent) {
            let (status, body) = duos_get(&app, &pal, IP, &unheld_code(spent)).await;
            assert_eq!(status, 404, "{body}");
            spent += 1;
        }
    }
    let fresh = account(&app, 99, "Fresh").await;
    let (status, body) = duos_get(&app, &fresh, IP, &unheld_code(1000)).await;
    assert_eq!((status, error_code(&body)), (429, "rate_limited"), "{body}");
    assert_eq!(duos_get(&app, &fresh, OTHER_IP, &unheld_code(1000)).await.0, 404);
}

#[tokio::test(start_paused = true)]
async fn r4008000_the_people_of_a_lobby_and_a_room_use_their_own_with_the_allowance_spent() {
    let app = empty_test_app().await;
    let host = account(&app, 1, "Host").await;
    let (status, lobby) = from_ip(
        &app,
        "POST",
        "/api/lobbies",
        &host.token,
        IP,
        json!({ "format": "duel", "mode": "random" }),
    )
    .await;
    assert_eq!(status, 201, "{lobby}");
    let (status, room) = from_ip(
        &app,
        "POST",
        "/api/rooms",
        &host.token,
        IP,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 200, "{room}");
    let (status, duos) = from_ip(
        &app,
        "POST",
        "/api/duos/rooms",
        &host.token,
        IP,
        json!({ "difficulty": "easy", "random": true }),
    )
    .await;
    assert_eq!(status, 201, "{duos}");

    for n in 0..CODE_TRIES_PER_ACCOUNT_PER_HOUR {
        assert_eq!(duos_get(&app, &host, IP, &unheld_code(n)).await.0, 404);
    }
    let own = |body: &Value| body["code"].as_str().expect("a code").to_string();
    let (status, body) = from_ip(
        &app,
        "GET",
        &format!("/api/lobbies/{}", own(&lobby)),
        &host.token,
        IP,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = duos_get(&app, &host, IP, &own(&duos)).await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = from_ip(
        &app,
        "DELETE",
        &format!("/api/rooms/{}", own(&room)),
        &host.token,
        IP,
        Value::Null,
    )
    .await;
    assert_eq!((status, &body["cancelled"]), (200, &json!(true)), "{body}");
    assert_eq!(duos_get(&app, &host, IP, &unheld_code(500)).await.0, 429);
}

/// A friend request for the unknown name `Ghost#n`.
async fn ghost_request(app: &Arc<App>, who: &Account, ip: &str, n: usize) -> (u16, Value) {
    from_ip(
        app,
        "POST",
        "/api/friends/requests",
        &who.token,
        ip,
        json!({ "username": format!("Ghost#{n}") }),
    )
    .await
}

#[tokio::test(start_paused = true)]
async fn r4008001_friend_requests_past_the_addresss_hourly_allowance_are_refused_for_every_account() {
    let app = empty_test_app().await;
    let mut sent = 0;
    let mut k = 0;
    while sent < FRIEND_REQUESTS_PER_ADDRESS_PER_HOUR {
        let pal = account(&app, 10 + k, "Pal").await;
        k += 1;
        let turn = FRIEND_REQUESTS_PER_HOUR.min(FRIEND_REQUESTS_PER_ADDRESS_PER_HOUR - sent);
        for _ in 0..turn {
            let (status, body) = ghost_request(&app, &pal, IP, sent).await;
            assert_eq!(status, 404, "{body}");
            sent += 1;
        }
    }
    let fresh = account(&app, 99, "Fresh").await;
    let (status, body) = ghost_request(&app, &fresh, IP, 9999).await;
    assert_eq!((status, error_code(&body)), (429, "rate_limited"), "{body}");
    assert_eq!(ghost_request(&app, &fresh, OTHER_IP, 9999).await.0, 404);
}

#[tokio::test(start_paused = true)]
async fn r4008001_a_friend_request_carries_one_name() {
    let app = empty_test_app().await;
    let who = account(&app, 1, "Pal").await;
    let (status, body) = from_ip(
        &app,
        "POST",
        "/api/friends/requests",
        &who.token,
        IP,
        json!({ "username": ["A#1", "B#2"] }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
}

async fn redeem(app: &Arc<App>, token: &str, ip: &str, body: Value) -> (u16, Value) {
    from_ip(app, "POST", "/api/codes/redeem", token, ip, body).await
}

async fn attempts_remaining(app: &Arc<App>, token: &str, ip: &str) -> i64 {
    let (status, body) = from_ip(app, "GET", "/api/codes/status", token, ip, Value::Null).await;
    assert_eq!(status, 200, "{body}");
    body["attemptsRemaining"].as_i64().expect("attemptsRemaining")
}

#[tokio::test(start_paused = true)]
async fn r4008000_invite_code_guesses_past_the_profiles_allowance_are_refused() {
    let app = empty_test_app().await;
    let token = pending_account(&app, 1).await;
    for _ in 0..=CODE_ATTEMPTS_PER_PROFILE_PER_HOUR {
        let (status, body) = redeem(&app, &token, IP, json!({ "code": UNMINTED_INVITE })).await;
        assert_eq!((status, error_code(&body)), (400, "invalid_code"), "{body}");
    }
    let (status, body) = redeem(&app, &token, IP, json!({ "code": UNMINTED_INVITE })).await;
    assert_eq!((status, error_code(&body)), (429, "rate_limited"), "{body}");
}

#[tokio::test(start_paused = true)]
async fn r4008000_invite_code_guesses_past_the_addresss_allowance_are_refused_across_accounts() {
    let app = empty_test_app().await;
    let per_ip = usize::try_from(CODE_ATTEMPTS_PER_IP_PER_HOUR).expect("a count");
    let per_profile = usize::try_from(CODE_ATTEMPTS_PER_PROFILE_PER_HOUR).expect("a count");
    let mut made = 0;
    let mut k = 0;
    while made <= per_ip {
        let token = pending_account(&app, k).await;
        k += 1;
        for _ in 0..(per_profile + 1).min(per_ip + 1 - made) {
            let (status, body) = redeem(&app, &token, IP, json!({ "code": UNMINTED_INVITE })).await;
            assert_eq!((status, error_code(&body)), (400, "invalid_code"), "{body}");
            made += 1;
        }
    }
    let fresh = pending_account(&app, 999).await;
    let (status, body) = redeem(&app, &fresh, IP, json!({ "code": UNMINTED_INVITE })).await;
    assert_eq!((status, error_code(&body)), (429, "rate_limited"), "{body}");
    let (status, body) = redeem(&app, &fresh, OTHER_IP, json!({ "code": UNMINTED_INVITE })).await;
    assert_eq!((status, error_code(&body)), (400, "invalid_code"), "{body}");
}

#[tokio::test(start_paused = true)]
async fn r4008000_a_redeem_request_carries_one_code() {
    let app = empty_test_app().await;
    let token = pending_account(&app, 1).await;
    let before = attempts_remaining(&app, &token, IP).await;
    let (status, body) = redeem(
        &app,
        &token,
        IP,
        json!({ "code": [UNMINTED_INVITE, "WXYZ-2345-6789-ABCD"] }),
    )
    .await;
    assert_eq!((status, error_code(&body)), (400, "bad_request"), "{body}");
    assert_eq!(attempts_remaining(&app, &token, IP).await, before);

    let (status, body) = redeem(
        &app,
        &token,
        IP,
        json!({ "code": UNMINTED_INVITE, "codes": ["WXYZ-2345-6789-ABCD", "WXYZ-2345-6789-ABCE"] }),
    )
    .await;
    assert_eq!((status, error_code(&body)), (400, "invalid_code"), "{body}");
    assert_eq!(attempts_remaining(&app, &token, IP).await, before - 1);
}

#[tokio::test(start_paused = true)]
async fn r4008000_username_sign_in_reads_one_guess_a_request() {
    let app = test_app().await;
    for body in [
        json!({ "username": "e2e_p1", "password": ["a", "b"] }),
        json!({ "username": ["e2e_p1", "e2e_p2"], "password": "x" }),
    ] {
        let (status, _headers, reply) = send(
            &app,
            json_request("POST", "/api/auth/username-signin", &body, None, Some(IP)),
        )
        .await;
        assert_eq!(status, 400, "{reply}");
    }
}

#[tokio::test(start_paused = true)]
async fn r4008000_the_unsubscribe_link_is_held_to_the_addresss_request_allowance() {
    let app = empty_test_app().await;
    let get = |ip: &'static str, n: usize| {
        let app = app.clone();
        async move {
            let path = format!("/api/daily/unsubscribe?token={}", uuid(n));
            let (status, _headers, body) =
                send(&app, json_request("GET", &path, &Value::Null, None, Some(ip))).await;
            (status, body)
        }
    };
    for n in 0..API_REQUESTS_PER_MINUTE {
        assert_eq!(get(IP, n).await.0, 200);
    }
    let (status, body) = get(IP, API_REQUESTS_PER_MINUTE).await;
    assert_eq!((status, error_code(&body)), (429, "rate_limited"), "{body}");
    assert_eq!(get(OTHER_IP, 0).await.0, 200);
}

#[tokio::test(start_paused = true)]
async fn r4008000_an_unsubscribe_request_reads_one_token() {
    let app = empty_test_app().await;
    let ada = account(&app, 1, "Ada").await;
    let (status, settings) = from_ip(&app, "GET", "/api/daily/settings", &ada.token, IP, Value::Null).await;
    assert_eq!(status, 200, "{settings}");
    let row = {
        let mut tx = app.db.begin(Some(&ada.id)).await.expect("a transaction");
        let row = tx.daily_notify_get(&ada.id).await.expect("Ada's daily row");
        tx.commit().await.expect("a commit");
        row
    };

    let path = format!(
        "/api/daily/unsubscribe?token={}&token={}",
        uuid(777),
        row.unsubscribe_token
    );
    let (status, _headers, page) =
        send(&app, json_request("POST", &path, &Value::Null, None, Some(IP))).await;
    assert_eq!(status, 200, "{page}");
    assert!(
        page.as_str().is_some_and(|page| page.contains("not one of ours")),
        "{page}"
    );
    let (_, settings) = from_ip(&app, "GET", "/api/daily/settings", &ada.token, IP, Value::Null).await;
    assert_eq!(settings["emailOn"], true, "{settings}");
}

#[test]
fn r4008000_no_socket_frame_carries_a_code_or_checks_one() {
    assert!(parse_client_message(r#"{"type":"joinRoom","roomCode":"ABCDEF"}"#).is_err());
    assert!(parse_client_message(r#"{"type":"hello","matchId":["a","b"]}"#).is_err());
    let hello = parse_client_message(r#"{"type":"hello","token":"t","matchId":"m","roomCode":"ABCDEF"}"#);
    assert!(matches!(hello, Ok(ClientMessage::Hello(_))));
}
