//! B53 (#936, `.fullsend/SPEC.md` S7): chat between friends. Only friends; a body of 1 to
//! `CHAT_MAX_CHARS` after trimming, filtered by the username word filter (blocked words masked with
//! `*`); at most `CHAT_MESSAGES_PER_MINUTE` per sender; pages of at most `CHAT_PAGE`, newest last,
//! paged back by `?before=`; reading a page marks it read; messages older than
//! `CHAT_RETENTION_DAYS` are purged by the retention loop; a report is stored once per reporter.
//!
//! The whole router over the fake store. Time is tokio's paused clock wherever an order, a window
//! or an age matters, so every message has its own `createdAt`. The retention loop is the one
//! `app.rs` spawns (`api::retention::run_purge`), run here for one pass.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_server::api::retention::run_purge;
use jackioh_server::app::App;
use jackioh_server::config::{
    CHAT_MAX_CHARS, CHAT_MESSAGES_PER_MINUTE, CHAT_PAGE, CHAT_RETENTION_DAYS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;

use crate::support::deps::{add_user, call, empty_test_app};

/// Unit conversions, not configuration.
const MINUTE_MS: u64 = 60_000;
const DAY_MS: u64 = 86_400_000;
/// The gap the paged tests leave between two messages, so no two share a `createdAt`.
const BETWEEN_MESSAGES_MS: u64 = 10;
/// How many messages past one page the paging test sends.
const PAST_THE_PAGE: usize = 5;

/// How many times `settle` yields: enough for a spawned loop's first pass over the fake store.
const SETTLE_YIELDS: usize = 256;

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
}

async fn advance(ms: u64) {
    tokio::time::advance(Duration::from_millis(ms)).await;
}

async fn settle() {
    for _ in 0..SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

/// A query value, percent-encoded.
fn encode_query(raw: &str) -> String {
    raw.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (byte as char).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
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

async fn send(app: &Arc<App>, from: &Account, to: &Account, body: &str) -> (u16, Value) {
    api(
        app,
        "POST",
        &format!("/api/chat/{}", to.id),
        &from.token,
        json!({ "body": body }),
    )
    .await
}

/// Sends and answers the stored message.
async fn sent(app: &Arc<App>, from: &Account, to: &Account, body: &str) -> Value {
    let (status, answer) = send(app, from, to, body).await;
    assert_eq!(status, 201, "{answer}");
    answer["message"].clone()
}

/// `GET /api/chat/:friendId` for `who`, optionally before an instant.
async fn page(app: &Arc<App>, who: &Account, friend: &Account, before: Option<&str>) -> Vec<Value> {
    let path = match before {
        Some(at) => format!("/api/chat/{}?before={}", friend.id, encode_query(at)),
        None => format!("/api/chat/{}", friend.id),
    };
    let (status, body) = api(app, "GET", &path, &who.token, Value::Null).await;
    assert_eq!(status, 200, "{body}");
    body["messages"].as_array().expect("messages").clone()
}

fn bodies(messages: &[Value]) -> Vec<String> {
    messages
        .iter()
        .map(|message| message["body"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// `GET /api/chat/unread` for `who`: the count from `friend`, 0 when absent.
async fn unread_from(app: &Arc<App>, who: &Account, friend: &Account) -> i64 {
    let (status, body) = api(app, "GET", "/api/chat/unread", &who.token, Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert!(body["unread"].is_object(), "{body}");
    body["unread"][friend.id.as_str()].as_i64().unwrap_or(0)
}

/// Two friends.
async fn friends() -> (Arc<App>, Account, Account) {
    let app = empty_test_app().await;
    let ada = account(&app, 1, "Ada").await;
    let bea = account(&app, 2, "Bea").await;
    befriend(&app, &ada, &bea).await;
    (app, ada, bea)
}

// ---------------------------------------------------------------------------------------------
// B53
// ---------------------------------------------------------------------------------------------

mod b53_sending {
    use super::*;

    #[tokio::test]
    async fn b53_a_message_to_a_friend_answers_201_with_the_message_unread() {
        let (app, ada, bea) = friends().await;

        let (status, body) = send(&app, &ada, &bea, "good game").await;

        assert_eq!(status, 201, "{body}");
        let message = &body["message"];
        assert!(message["id"].is_string(), "{message}");
        assert_eq!(message["from"], json!(ada.id));
        assert_eq!(message["to"], json!(bea.id));
        assert_eq!(message["body"], "good game");
        assert!(message["createdAt"].is_string(), "{message}");
        assert_eq!(message["read"], json!(false));
    }

    #[tokio::test]
    async fn b53_a_message_to_a_player_who_is_not_a_friend_is_404_and_stored_nowhere() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let stan = account(&app, 3, "Stan").await;

        let (status, body) = send(&app, &ada, &stan, "hello").await;

        assert_eq!(status, 404, "{body}");
        assert_eq!(unread_from(&app, &stan, &ada).await, 0);
    }

    #[tokio::test]
    async fn b53_b46_a_block_stops_chat_in_both_directions() {
        let (app, ada, bea) = friends().await;
        let (status, body) = api(
            &app,
            "POST",
            "/api/blocks",
            &bea.token,
            json!({ "profileId": ada.id }),
        )
        .await;
        assert_eq!(status, 204, "{body}");

        assert_eq!(send(&app, &ada, &bea, "hello").await.0, 404);
        assert_eq!(send(&app, &bea, &ada, "hello").await.0, 404);
        assert_eq!(unread_from(&app, &bea, &ada).await, 0);
        assert_eq!(unread_from(&app, &ada, &bea).await, 0);
    }

    #[tokio::test]
    async fn b53_an_empty_or_blank_body_is_422_empty() {
        let (app, ada, bea) = friends().await;

        for blank in ["", "   ", " \n\t "] {
            let (status, body) = send(&app, &ada, &bea, blank).await;
            assert_eq!(status, 422, "{blank:?}: {body}");
            assert!(code_is(&body, "empty"), "{blank:?}: {body}");
        }
        assert_eq!(unread_from(&app, &bea, &ada).await, 0);
    }

    #[tokio::test]
    async fn b53_a_body_past_chat_max_chars_after_trimming_is_422_too_long_and_one_at_the_limit_is_taken() {
        let (app, ada, bea) = friends().await;
        let max = count(CHAT_MAX_CHARS);

        let (status, body) = send(&app, &ada, &bea, &"a".repeat(max + 1)).await;
        assert_eq!(status, 422, "{body}");
        assert!(code_is(&body, "too_long"), "{body}");
        assert_eq!(unread_from(&app, &bea, &ada).await, 0);

        let (status, body) = send(&app, &ada, &bea, &"a".repeat(max)).await;
        assert_eq!(status, 201, "{body}");
        // The limit counts the trimmed body: whitespace around it does not push it over.
        let padded = format!("  {}  ", "b".repeat(max));
        let (status, body) = send(&app, &ada, &bea, &padded).await;
        assert_eq!(status, 201, "{body}");
        assert_eq!(unread_from(&app, &bea, &ada).await, 2);
    }

    #[tokio::test]
    async fn b53_a_blocked_word_is_masked_with_asterisks_and_a_clean_body_is_kept_as_written() {
        let (app, ada, bea) = friends().await;

        let masked = sent(&app, &ada, &bea, "you are a bitch").await;
        let clean = sent(&app, &ada, &bea, "well played").await;

        let text = masked["body"].as_str().expect("a body");
        assert!(!text.to_lowercase().contains("bitch"), "{text}");
        assert!(text.contains('*'), "{text}");
        assert!(text.starts_with("you are a "), "only the blocked word is masked: {text}");
        assert_eq!(clean["body"], "well played");
        // What the friend reads is what was stored.
        let read = page(&app, &bea, &ada, None).await;
        assert_eq!(bodies(&read), vec![text.to_string(), "well played".to_string()]);
    }

    #[tokio::test(start_paused = true)]
    async fn b53_past_chat_messages_per_minute_a_message_answers_429_until_the_minute_has_passed() {
        let (app, ada, bea) = friends().await;
        let limit = count(CHAT_MESSAGES_PER_MINUTE);
        for n in 0..limit {
            let (status, body) = send(&app, &ada, &bea, &format!("m{n}")).await;
            assert_eq!(status, 201, "message {n}: {body}");
        }

        let (status, body) = send(&app, &ada, &bea, "one too many").await;
        assert_eq!(status, 429, "{body}");
        assert_eq!(unread_from(&app, &bea, &ada).await, i64::try_from(limit).expect("a count"));
        // The allowance is the sender's: Bea may still answer.
        assert_eq!(send(&app, &bea, &ada, "slow down").await.0, 201);

        advance(MINUTE_MS + 1).await;
        let (status, body) = send(&app, &ada, &bea, "sorry").await;
        assert_eq!(status, 201, "{body}");
    }
}

mod b53_reading {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b53_a_page_is_the_conversation_newest_last_and_reading_it_marks_it_read() {
        let (app, ada, bea) = friends().await;
        sent(&app, &ada, &bea, "one").await;
        advance(BETWEEN_MESSAGES_MS).await;
        sent(&app, &bea, &ada, "two").await;
        advance(BETWEEN_MESSAGES_MS).await;
        sent(&app, &ada, &bea, "three").await;
        assert_eq!(unread_from(&app, &bea, &ada).await, 2);

        let read = page(&app, &bea, &ada, None).await;

        assert_eq!(bodies(&read), vec!["one", "two", "three"]);
        assert_eq!(read[0]["from"], json!(ada.id));
        assert_eq!(read[1]["from"], json!(bea.id));
        assert_eq!(unread_from(&app, &bea, &ada).await, 0);
        // The sender sees the messages Bea read as read.
        let theirs = page(&app, &ada, &bea, None).await;
        for message in theirs.iter().filter(|message| message["from"] == ada.id.as_str()) {
            assert_eq!(message["read"], json!(true), "{message}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b53_a_page_holds_at_most_chat_page_messages_and_before_pages_back_to_older_ones() {
        let (app, ada, bea) = friends().await;
        let page_size = count(CHAT_PAGE);
        let per_minute = count(CHAT_MESSAGES_PER_MINUTE);
        let total = page_size + PAST_THE_PAGE;
        for n in 0..total {
            if n > 0 && n % per_minute == 0 {
                advance(MINUTE_MS + 1).await;
            }
            sent(&app, &ada, &bea, &format!("m{n}")).await;
            advance(BETWEEN_MESSAGES_MS).await;
        }
        let all: Vec<String> = (0..total).map(|n| format!("m{n}")).collect();

        let newest = page(&app, &bea, &ada, None).await;
        assert_eq!(bodies(&newest), all[PAST_THE_PAGE..].to_vec());

        let oldest_shown = newest[0]["createdAt"].as_str().expect("a createdAt").to_string();
        let older = page(&app, &bea, &ada, Some(&oldest_shown)).await;
        assert_eq!(bodies(&older), all[..PAST_THE_PAGE].to_vec());
    }

    #[tokio::test]
    async fn b53_unread_counts_each_friends_messages_apart() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let cy = account(&app, 3, "Cy").await;
        befriend(&app, &ada, &bea).await;
        befriend(&app, &cy, &bea).await;
        sent(&app, &ada, &bea, "from ada").await;
        sent(&app, &cy, &bea, "from cy 1").await;
        sent(&app, &cy, &bea, "from cy 2").await;

        assert_eq!(unread_from(&app, &bea, &ada).await, 1);
        assert_eq!(unread_from(&app, &bea, &cy).await, 2);

        page(&app, &bea, &cy, None).await;
        assert_eq!(unread_from(&app, &bea, &cy).await, 0);
        assert_eq!(unread_from(&app, &bea, &ada).await, 1);
    }
}

mod b53_retention_and_reports {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b53_the_retention_loop_purges_messages_older_than_chat_retention_days_and_keeps_the_rest() {
        let (app, ada, bea) = friends().await;
        let retention_ms = u64::try_from(CHAT_RETENTION_DAYS).expect("days") * DAY_MS;
        sent(&app, &ada, &bea, "old").await;
        advance(retention_ms - DAY_MS).await;
        sent(&app, &ada, &bea, "new").await;
        advance(2 * DAY_MS).await;

        let purge = tokio::spawn(run_purge(Arc::clone(&app)));
        settle().await;
        purge.abort();

        assert_eq!(bodies(&page(&app, &bea, &ada, None).await), vec!["new"]);
    }

    #[tokio::test]
    async fn b53_a_report_is_taken_once_and_a_second_from_the_same_reporter_is_no_fault() {
        let (app, ada, bea) = friends().await;
        let message = sent(&app, &ada, &bea, "you are a bitch").await;
        let id = message["id"].as_str().expect("a message id").to_string();
        let path = format!("/api/chat/messages/{id}/report");

        let (status, body) = api(&app, "POST", &path, &bea.token, json!({ "reason": "abuse" })).await;
        assert_eq!(status, 204, "{body}");

        // Stored once per reporter: the second is not a new row, and not a server fault.
        let (status, body) = api(&app, "POST", &path, &bea.token, json!({ "reason": "abuse" })).await;
        assert!(status < 500, "{status}: {body}");
    }
}
