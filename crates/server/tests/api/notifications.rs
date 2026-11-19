//! R4039000 (#4039): the in-game notification feed's routes. `GET /api/notifications` pages the
//! caller's own rows newest first, `POST /api/notifications/read` marks the caller's own rows read up
//! to an id or all of them, each account keeps its newest `NOTIFICATIONS_KEEP` rows, the daily sweep
//! deletes rows past `NOTIFICATIONS_KEEP_DAYS`, and the social read counts the unread ones. Which event
//! writes which row is each event's own file's (`daily.rs`, `friends.rs`, `shares.rs`, `lobbies.rs`).

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{MS_PER_DAY, NOTIFICATIONS_KEEP, NOTIFICATIONS_KEEP_DAYS, NOTIFICATIONS_PAGE};
use jackioh_server::daily::notify::sweep;
use jackioh_server::notifications::store::{NewNotification, NotificationKind, NotificationRow};

use crate::support::deps::{call, empty_test_app};
use crate::support::store::{Account, account, feed, feed_kinds};

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

/// Writes a `replayShared` row for `to` by `actor` about `subject`, written at `at`, as `notify` does.
async fn write(app: &App, to: &Account, actor: &Account, subject: &str, at: i64) -> NotificationRow {
    let mut tx = app.db.begin(None).await.expect("a store transaction begins");
    let row = tx
        .notifications_add(
            &NewNotification {
                profile_id: to.id.clone(),
                kind: NotificationKind::ReplayShared,
                actor_id: Some(actor.id.clone()),
                subject: Some(subject.to_string()),
                payload: json!({}),
                created_at: at,
                read: false,
            },
            NOTIFICATIONS_KEEP,
        )
        .await
        .expect("the row is written");
    tx.commit().await.expect("the write commits");
    row
}

/// The subjects of a feed body's rows, in its order.
fn subjects(body: &Value) -> Vec<String> {
    body["notifications"]
        .as_array()
        .expect("a page of rows")
        .iter()
        .map(|row| row["subject"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// Ana and Bo, on an app of their own.
async fn two_people() -> (Arc<App>, Account, Account) {
    let app = empty_test_app().await;
    let ana = account(&app, 1, "Ana").await;
    let bo = account(&app, 2, "Bo").await;
    (app, ana, bo)
}

#[tokio::test]
async fn r4039000_the_page_is_newest_first_and_pages_by_before() {
    let (app, ana, bo) = two_people().await;
    let now = now_ms();
    let first = write(&app, &ana, &bo, "replay-0", now).await;
    for n in 1..=(NOTIFICATIONS_PAGE + 2) {
        write(&app, &ana, &bo, &format!("replay-{n}"), now).await;
    }
    write(&app, &bo, &ana, "bos-own", now).await;

    // The first page: the newest NOTIFICATIONS_PAGE, whole, and where the next one starts.
    let page = feed(&app, &ana).await;
    let (status, first_page) = api(&app, "GET", "/api/notifications", &ana.token, Value::Null).await;
    assert_eq!(status, 200, "{first_page}");
    let listed = subjects(&first_page);
    assert_eq!(
        listed.len(),
        usize::try_from(NOTIFICATIONS_PAGE).expect("a page"),
        "{first_page}"
    );
    assert_eq!(listed[0], format!("replay-{}", NOTIFICATIONS_PAGE + 2));
    assert_eq!(first_page["unread"], NOTIFICATIONS_PAGE + 3, "{first_page}");
    assert_eq!(
        first_page["notifications"][0],
        json!({
            "id": first_page["notifications"][0]["id"],
            "kind": "replayShared",
            "actor": { "profileId": bo.id, "username": bo.name },
            "subject": format!("replay-{}", NOTIFICATIONS_PAGE + 2),
            "payload": {},
            "createdAt": now,
            "read": false,
        })
    );
    let next = first_page["nextBefore"].as_i64().expect("a next page");
    assert_eq!(
        json!(next),
        first_page["notifications"][NOTIFICATIONS_PAGE as usize - 1]["id"]
    );

    // The page after it, and the end.
    let (_, second_page) = api(
        &app,
        "GET",
        &format!("/api/notifications?before={next}&limit=2"),
        &ana.token,
        Value::Null,
    )
    .await;
    assert_eq!(
        subjects(&second_page),
        vec!["replay-2", "replay-1"],
        "{second_page}"
    );
    let next = second_page["nextBefore"].as_i64().expect("one more page");
    let (_, last_page) = api(
        &app,
        "GET",
        &format!("/api/notifications?before={next}&limit=2"),
        &ana.token,
        Value::Null,
    )
    .await;
    assert_eq!(subjects(&last_page), vec!["replay-0"], "{last_page}");
    assert_eq!(last_page["nextBefore"], Value::Null, "{last_page}");
    assert_eq!(last_page["notifications"][0]["id"], first.id);

    // The caller's own rows only.
    assert!(!subjects(&page).contains(&"bos-own".to_string()), "{page}");
    assert_eq!(subjects(&feed(&app, &bo).await), vec!["bos-own"]);
}

#[tokio::test]
async fn r4039000_a_bad_before_or_limit_is_400() {
    let (app, ana, _bo) = two_people().await;
    for query in [
        "before=0",
        "before=-3",
        "before=abc",
        "limit=0",
        "limit=51",
        "limit=two",
        "before=1.5",
    ] {
        let (status, body) = api(
            &app,
            "GET",
            &format!("/api/notifications?{query}"),
            &ana.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 400, "{query}: {body}");
    }
    for body in [
        json!({ "upTo": 0 }),
        json!({ "upTo": "3" }),
        json!({ "upTo": -1 }),
        json!({ "upTo": 1.5 }),
    ] {
        let (status, answer) = api(&app, "POST", "/api/notifications/read", &ana.token, body.clone()).await;
        assert_eq!(status, 400, "{body}: {answer}");
    }
    // Signed out, neither route answers.
    for (method, path) in [("GET", "/api/notifications"), ("POST", "/api/notifications/read")] {
        let (status, _headers, body) = call(&app, method, path, None, Value::Null).await;
        assert_eq!(status, 401, "{method} {path}: {body}");
    }
}

#[tokio::test]
async fn r4039000_read_marks_only_the_callers_rows() {
    let (app, ana, bo) = two_people().await;
    let now = now_ms();
    let mut ids = Vec::new();
    for n in 0..3 {
        ids.push(write(&app, &ana, &bo, &format!("replay-{n}"), now).await.id);
    }
    let bos = write(&app, &bo, &ana, "bos-own", now).await;

    // Up to an id, inclusive.
    let (status, body) = api(
        &app,
        "POST",
        "/api/notifications/read",
        &ana.token,
        json!({ "upTo": ids[1] }),
    )
    .await;
    assert_eq!((status, body), (200, json!({ "unread": 1 })));
    let anas = feed(&app, &ana).await;
    let read: Vec<Value> = anas["notifications"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["read"].clone())
        .collect();
    assert_eq!(read, vec![json!(false), json!(true), json!(true)], "{anas}");

    // Every one, and Bo's row, past Ana's ids, is still unread.
    let (status, body) = api(&app, "POST", "/api/notifications/read", &ana.token, json!({})).await;
    assert_eq!((status, body), (200, json!({ "unread": 0 })));
    let (status, body) = api(
        &app,
        "POST",
        "/api/notifications/read",
        &ana.token,
        json!({ "upTo": null }),
    )
    .await;
    assert_eq!((status, body), (200, json!({ "unread": 0 })));
    let bos_feed = feed(&app, &bo).await;
    assert_eq!(bos_feed["unread"], 1, "{bos_feed}");
    assert_eq!(bos_feed["notifications"][0]["id"], bos.id);
    assert_eq!(bos_feed["notifications"][0]["read"], false, "{bos_feed}");
}

#[tokio::test]
async fn r4039000_the_cap_keeps_the_newest() {
    let (app, ana, bo) = two_people().await;
    let now = now_ms();
    let first = write(&app, &ana, &bo, "replay-0", now).await;
    for n in 1..=NOTIFICATIONS_KEEP {
        write(&app, &ana, &bo, &format!("replay-{n}"), now).await;
    }
    let bos = write(&app, &bo, &ana, "bos-own", now).await;
    let mut tx = app.db.begin(None).await.expect("a store transaction begins");
    let all = tx
        .notifications_page(&ana.id, None, NOTIFICATIONS_KEEP + 10)
        .await
        .expect("the rows are read");
    let bos_rows = tx
        .notifications_page(&bo.id, None, 10)
        .await
        .expect("the rows are read");
    tx.commit().await.expect("the read commits");
    assert_eq!(all.len(), usize::try_from(NOTIFICATIONS_KEEP).expect("a cap"));
    assert_eq!(
        all[0].subject.as_deref(),
        Some(format!("replay-{NOTIFICATIONS_KEEP}").as_str())
    );
    assert!(!all.iter().any(|row| row.id == first.id), "the oldest went");
    assert_eq!(bos_rows, vec![bos], "another account's rows are its own");
}

#[tokio::test]
async fn r4039000_the_sweep_purges_rows_past_the_keep_days() {
    let (app, ana, bo) = two_people().await;
    let now = now_ms();
    write(
        &app,
        &ana,
        &bo,
        "old",
        now - NOTIFICATIONS_KEEP_DAYS * MS_PER_DAY - 1,
    )
    .await;
    write(
        &app,
        &ana,
        &bo,
        "at-the-edge",
        now - NOTIFICATIONS_KEEP_DAYS * MS_PER_DAY,
    )
    .await;
    write(&app, &bo, &ana, "bos-new", now).await;
    sweep(&app, now).await.expect("the sweep runs");
    assert_eq!(subjects(&feed(&app, &ana).await), vec!["at-the-edge"]);
    assert_eq!(subjects(&feed(&app, &bo).await), vec!["bos-new"]);

    // A sweep a day past the keep takes the rest, read or not.
    api(&app, "POST", "/api/notifications/read", &bo.token, json!({})).await;
    sweep(&app, now + (NOTIFICATIONS_KEEP_DAYS + 1) * MS_PER_DAY)
        .await
        .expect("the sweep runs");
    assert_eq!(feed_kinds(&feed(&app, &ana).await), Vec::<String>::new());
    assert_eq!(feed_kinds(&feed(&app, &bo).await), Vec::<String>::new());
}

#[tokio::test]
async fn r4039000_the_social_read_counts_unread() {
    let (app, ana, bo) = two_people().await;
    let (_, social) = api(&app, "GET", "/api/social", &ana.token, Value::Null).await;
    assert_eq!(social["notificationsUnread"], 0, "{social}");
    let now = now_ms();
    for n in 0..2 {
        write(&app, &ana, &bo, &format!("replay-{n}"), now).await;
    }
    let (_, social) = api(&app, "GET", "/api/social", &ana.token, Value::Null).await;
    assert_eq!(social["notificationsUnread"], 2, "{social}");
    let (_, social) = api(&app, "GET", "/api/social", &bo.token, Value::Null).await;
    assert_eq!(social["notificationsUnread"], 0, "{social}");
    api(&app, "POST", "/api/notifications/read", &ana.token, json!({})).await;
    let (_, social) = api(&app, "GET", "/api/social", &ana.token, Value::Null).await;
    assert_eq!(social["notificationsUnread"], 0, "{social}");
}
