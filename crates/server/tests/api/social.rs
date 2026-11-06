//! R3210000 (#3210): `GET /api/social`, the social poll's one read of the challenges and the unread
//! counts, and presence on every signed-in request that passes auth, the rate limit and the gate.

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_server::api::http::account_key;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    API_REQUESTS_PER_MINUTE, PRESENCE_TOUCH_EVERY_MS, PRESENCE_TTL_MS, TERMS_VERSION,
};

use crate::support::deps::{add_user, call, empty_test_app};
use crate::support::store::{Account, account, advance, befriend, fake, record_calls, stop_failing, uuid};

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
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

/// `who`'s `GET` of `path`, which answers 200.
async fn read(app: &Arc<App>, who: &Account, path: &str) -> Value {
    let (status, body) = api(app, "GET", path, &who.token, Value::Null).await;
    assert_eq!(status, 200, "{path}: {body}");
    body
}

/// `GET /api/challenges` and `GET /api/chat/unread` as one answer, `{incoming, outgoing, unread}`.
fn merged(challenges: &Value, unread: &Value) -> Value {
    json!({
        "incoming": challenges["incoming"],
        "outgoing": challenges["outgoing"],
        "unread": unread["unread"],
    })
}

/// What `who` reads from the two routes the social read stands in for.
async fn two_reads(app: &Arc<App>, who: &Account) -> Value {
    let challenges = read(app, who, "/api/challenges").await;
    let unread = read(app, who, "/api/chat/unread").await;
    merged(&challenges, &unread)
}

/// `friend`'s status as `viewer`'s friends list reads it.
async fn status_of(app: &Arc<App>, viewer: &Account, friend: &Account) -> String {
    let listed = read(app, viewer, "/api/friends").await;
    listed["friends"]
        .as_array()
        .and_then(|friends| friends.iter().find(|row| row["profileId"] == json!(friend.id)))
        .and_then(|row| row["status"].as_str())
        .unwrap_or_else(|| panic!("{} is not in the list: {listed}", friend.id))
        .to_string()
}

async fn challenge(app: &Arc<App>, from: &Account, to: &Account) {
    let (status, body) = api(
        app,
        "POST",
        "/api/challenges",
        &from.token,
        json!({ "mode": "random", "friendId": to.id }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
}

async fn chat(app: &Arc<App>, from: &Account, to: &Account) {
    let (status, body) = api(
        app,
        "POST",
        &format!("/api/chat/{}", to.id),
        &from.token,
        json!({ "body": "hi" }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
}

fn len(list: &Value) -> usize {
    list.as_array().map_or(0, Vec::len)
}

/// A config duration as milliseconds, whichever integer type `config.rs` gives it.
fn millis<T>(value: T) -> u64
where
    u64: TryFrom<T>,
    <u64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    u64::try_from(value).expect("a duration fits a u64")
}

/// How many of the store calls logged were `method`.
fn calls_to(calls: &std::sync::Mutex<Vec<String>>, method: &str) -> usize {
    calls
        .lock()
        .expect("the call log")
        .iter()
        .filter(|called| *called == method)
        .count()
}

// The one social read

mod r3210000_the_social_read {
    use super::*;

    #[tokio::test]
    async fn r3210000_the_social_read_answers_the_two_reads_for_the_caller_alone() {
        jackioh_cards::register_all();
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        let cyd = account(&app, 3, "Cyd").await;
        befriend(&app, &ada, &bea).await;
        befriend(&app, &ada, &cyd).await;
        befriend(&app, &bea, &cyd).await;
        challenge(&app, &bea, &ada).await;
        challenge(&app, &ada, &cyd).await;
        chat(&app, &bea, &ada).await;
        chat(&app, &bea, &ada).await;
        chat(&app, &cyd, &ada).await;
        // Between two others: no read of Ada's names it.
        chat(&app, &bea, &cyd).await;

        let adas = read(&app, &ada, "/api/social").await;
        assert_eq!(adas, two_reads(&app, &ada).await);
        assert_eq!(len(&adas["incoming"]), 1, "{adas}");
        assert_eq!(adas["incoming"][0]["from"]["profileId"], json!(bea.id));
        assert_eq!(len(&adas["outgoing"]), 1, "{adas}");
        assert_eq!(adas["outgoing"][0]["to"]["profileId"], json!(cyd.id));
        assert_eq!(adas["unread"], json!({ bea.id.clone(): 2, cyd.id.clone(): 1 }));

        let beas = read(&app, &bea, "/api/social").await;
        assert_eq!(beas, two_reads(&app, &bea).await);
        assert_eq!(len(&beas["incoming"]), 0, "{beas}");
        assert_eq!(len(&beas["outgoing"]), 1, "{beas}");
        assert_eq!(beas["unread"], json!({}));

        let cyds = read(&app, &cyd, "/api/social").await;
        assert_eq!(cyds, two_reads(&app, &cyd).await);
        assert_eq!(cyds["unread"], json!({ bea.id.clone(): 1 }));
    }

    #[tokio::test]
    async fn r3210000_a_blocked_pair_gets_from_the_social_read_what_the_two_reads_give_it() {
        jackioh_cards::register_all();
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;
        challenge(&app, &bea, &ada).await;
        chat(&app, &bea, &ada).await;
        let (status, body) = api(
            &app,
            "POST",
            "/api/blocks",
            &ada.token,
            json!({ "profileId": bea.id }),
        )
        .await;
        assert_eq!(status, 204, "{body}");

        for who in [&ada, &bea] {
            assert_eq!(
                read(&app, who, "/api/social").await,
                two_reads(&app, who).await,
                "{}",
                who.name
            );
        }
        let adas = read(&app, &ada, "/api/social").await;
        assert_eq!(len(&adas["incoming"]), 0, "{adas}");
        assert_eq!(adas["unread"], json!({}));
    }

    #[tokio::test]
    async fn r3210000_a_pending_or_banned_account_or_no_token_gets_what_the_two_reads_answer() {
        let app = empty_test_app().await;
        let pending = seeded(&app, 1, "Pen", "pending").await;
        let banned = seeded(&app, 2, "Ban", "banned").await;

        for who in [&pending, &banned] {
            let social = api(&app, "GET", "/api/social", &who.token, Value::Null).await;
            assert_eq!(social.0, 403, "{}: {}", who.name, social.1);
            for path in ["/api/challenges", "/api/chat/unread"] {
                assert_eq!(
                    api(&app, "GET", path, &who.token, Value::Null).await,
                    social,
                    "{}: {path}",
                    who.name
                );
            }
        }
        for path in ["/api/social", "/api/challenges", "/api/chat/unread"] {
            let (status, _, body) = call(&app, "GET", path, None, Value::Null).await;
            assert_eq!(status, 401, "{path}: {body}");
        }
    }
}

// Presence on every signed-in request

mod r3210000_presence_on_every_request {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r3210000_any_signed_in_request_shows_the_caller_online_until_presence_ttl() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;
        advance(millis(PRESENCE_TTL_MS) + 1).await;
        assert_eq!(status_of(&app, &ada, &bea).await, "offline");

        read(&app, &bea, "/api/auth/me").await;
        assert_eq!(status_of(&app, &ada, &bea).await, "online");

        advance(millis(PRESENCE_TTL_MS) + 1).await;
        assert_eq!(status_of(&app, &ada, &bea).await, "offline");

        read(&app, &bea, "/api/social").await;
        assert_eq!(status_of(&app, &ada, &bea).await, "online");
    }

    #[tokio::test(start_paused = true)]
    async fn r3210000_presence_is_written_at_most_once_a_presence_touch_every_ms() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        read(&app, &ada, "/api/social").await;
        let calls = record_calls(&app).await;

        for _ in 0..3 {
            read(&app, &ada, "/api/social").await;
        }
        advance(millis(PRESENCE_TOUCH_EVERY_MS) - 1).await;
        read(&app, &ada, "/api/social").await;
        assert_eq!(calls_to(&calls, "presence.touch"), 0);
        assert_eq!(calls_to(&calls, "consent.get"), 0);

        advance(1).await;
        read(&app, &ada, "/api/social").await;
        assert_eq!(calls_to(&calls, "presence.touch"), 1);
        assert_eq!(calls_to(&calls, "consent.get"), 1);
        stop_failing(&app).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r3210000_a_refused_request_refreshes_no_presence() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        befriend(&app, &ada, &bea).await;
        advance(millis(PRESENCE_TTL_MS) + 1).await;

        // §9.8: Bea's minute is spent, so her next request is refused before anything is read.
        for _ in 0..API_REQUESTS_PER_MINUTE {
            assert!(app.limiter.allow(&account_key(&bea.id), now_ms()));
        }
        let (status, body) = api(&app, "GET", "/api/social", &bea.token, Value::Null).await;
        assert_eq!(status, 429, "{body}");
        assert_eq!(status_of(&app, &ada, &bea).await, "offline");

        let (status, _, body) = call(&app, "GET", "/api/social", Some("not-a-token"), Value::Null).await;
        assert_eq!(status, 401, "{body}");
        assert_eq!(status_of(&app, &ada, &bea).await, "offline");
    }

    #[tokio::test]
    async fn r3210000_a_pending_or_banned_account_is_never_shown_online() {
        let app = empty_test_app().await;
        let pending = seeded(&app, 1, "Pen", "pending").await;
        let banned = seeded(&app, 2, "Ban", "banned").await;

        for who in [&pending, &banned] {
            // A route open to every signed-in account, and one the gate refuses it.
            api(&app, "GET", "/api/auth/me", &who.token, Value::Null).await;
            let (status, body) = api(&app, "GET", "/api/social", &who.token, Value::Null).await;
            assert_eq!(status, 403, "{body}");
        }
        let presence = fake(&app).lock().await.tables.social.presence.clone();
        assert!(!presence.contains_key(&pending.id), "{presence:?}");
        assert!(!presence.contains_key(&banned.id), "{presence:?}");
    }

    #[tokio::test]
    async fn r3210000_no_request_counts_while_the_terms_are_owed_and_the_next_after_agreeing_does() {
        let app = empty_test_app().await;
        let ada = account(&app, 1, "Ada").await;
        let bea = account(&app, 2, "Bea").await;
        // The terms changed since Bea agreed: she holds no agreement to the version in force.
        fake(&app)
            .lock()
            .await
            .tables
            .terms_acceptances
            .retain(|row| row.profile_id != bea.id);
        befriend(&app, &ada, &bea).await;

        read(&app, &bea, "/api/social").await;
        assert_eq!(status_of(&app, &ada, &bea).await, "offline");
        assert!(
            !fake(&app)
                .lock()
                .await
                .tables
                .social
                .presence
                .contains_key(&bea.id)
        );

        let (status, body) = api(
            &app,
            "POST",
            "/api/account/consent",
            &bea.token,
            json!({ "termsVersion": TERMS_VERSION }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        // Agreeing cleared the touch's mark: the next request counts at once, not a period later.
        read(&app, &bea, "/api/social").await;
        assert_eq!(status_of(&app, &ada, &bea).await, "online");
    }
}
