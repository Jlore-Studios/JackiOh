//! `POST /api/account/consent` and what `GET /api/auth/me` says is owed (`src/api/consent.rs`): the
//! age screen (R757000) and the terms a player agrees to (R757001).
//!
//! Every account here is seeded with `"consent": false`, so it starts with neither an age check nor
//! an agreement recorded. What the store keeps is read back through the port (`consent_get`); the
//! store's own rules are `tests/store/contract.rs`'s and `tests/sql/18_account_consent.sql`'s.

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_server::api::consent::utc_year_month;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{BIRTH_YEAR_EARLIEST, MIN_ACCOUNT_AGE_YEARS, TERMS_VERSION};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{ConsentRow, Db, TermsSource};

use crate::support::deps::{add_user, call, test_app};

const CONSENT: &str = "/api/account/consent";

fn store_of(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => data.clone(),
        Db::Pg(_) => panic!("the test app runs on the in-memory store"),
    }
}

/// A verified user with a profile of `status` that has recorded no consent, and its token.
async fn owing(app: &Arc<App>, id: &str, status: &str) -> String {
    let user_id = format!("user-{id}");
    let token = add_user(app, &user_id, &format!("{id}@example.test"), true);
    let _ = store_of(app).lock().await.seed_profile(json!({
        "id": id, "userId": user_id, "status": status, "consent": false,
    }));
    token
}

async fn consent_of(app: &App, id: &str) -> ConsentRow {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let consent = tx.consent_get(id).await.expect("a read");
    tx.commit().await.expect("a commit");
    consent
}

async fn profile_exists(app: &App, id: &str) -> bool {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let profile = tx.profiles_get_by_id(id).await.expect("a read");
    tx.commit().await.expect("a commit");
    profile.is_some()
}

async fn me(app: &Arc<App>, token: &str) -> Value {
    let (status, _, body) = call(app, "GET", "/api/auth/me", Some(token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    body
}

async fn post(app: &Arc<App>, token: &str, body: Value) -> (u16, Value) {
    let (status, _, body) = call(app, "POST", CONSENT, Some(token), body).await;
    (status, body)
}

/// A birth month and year `years` whole years before this month: January of the year after
/// `this year - years` is fewer than `years` years ago, January of `this year - years - 1` more.
fn born_over(years: i32) -> Value {
    let (year, _) = utc_year_month(now_ms());
    json!({ "termsVersion": TERMS_VERSION, "birthYear": year - years - 1, "birthMonth": 1 })
}

fn born_under(years: i32) -> Value {
    let (year, _) = utc_year_month(now_ms());
    json!({ "termsVersion": TERMS_VERSION, "birthYear": year - years + 1, "birthMonth": 1 })
}

#[tokio::test]
async fn r757000_pending_owes_both_until_posted() {
    let app = test_app().await;
    let token = owing(&app, "p-new", "pending").await;

    let before = me(&app, &token).await;
    assert_eq!(before["ageCheckOwed"], json!(true), "{before}");
    assert_eq!(before["termsOwed"], json!(true), "{before}");

    let (status, body) = post(&app, &token, born_over(MIN_ACCOUNT_AGE_YEARS)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, json!({ "ageCheckOwed": false, "termsOwed": false }));

    let after = me(&app, &token).await;
    assert_eq!(after["ageCheckOwed"], json!(false), "{after}");
    assert_eq!(after["termsOwed"], json!(false), "{after}");
    assert_eq!(after["needsInviteCode"], json!(true), "still pending: {after}");

    // Only that the check passed, against which minimum, and the version agreed to at sign-up.
    let consent = consent_of(&app, "p-new").await;
    assert_eq!(consent.age_min, Some(MIN_ACCOUNT_AGE_YEARS));
    assert!(consent.age_checked_at.is_some());
    assert_eq!(consent.terms.len(), 1);
    assert_eq!(consent.terms[0].version, TERMS_VERSION);
    assert_eq!(consent.terms[0].source, TermsSource::Signup);
}

#[tokio::test]
async fn r757000_under_minimum_deletes_the_account() {
    let app = test_app().await;
    let token = owing(&app, "p-young", "pending").await;

    let (status, body) = post(&app, &token, born_under(MIN_ACCOUNT_AGE_YEARS)).await;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["error"]["code"], json!("under_age"));
    // The refusal names no age.
    let message = body["error"]["message"].as_str().expect("a message");
    assert!(!message.contains(&MIN_ACCOUNT_AGE_YEARS.to_string()), "{message}");

    // Deleted as "Delete my account" deletes: the profile, its rows and the sign-in.
    assert!(!profile_exists(&app, "p-young").await);
    let consent = consent_of(&app, "p-young").await;
    assert_eq!((consent.age_checked_at, consent.terms.len()), (None, 0));
    assert!(app.auth.verify(&token).await.is_err());
    let (again, _, _) = call(&app, "GET", "/api/auth/me", Some(&token), Value::Null).await;
    assert_eq!(again, 401);
}

#[tokio::test]
async fn r757000_birth_required_and_range_checked() {
    let app = test_app().await;
    let token = owing(&app, "p-typo", "pending").await;
    let (year, month) = utc_year_month(now_ms());
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    for body in [
        json!({ "termsVersion": TERMS_VERSION }),
        json!({ "termsVersion": TERMS_VERSION, "birthYear": 2000 }),
        json!({ "termsVersion": TERMS_VERSION, "birthYear": "2000", "birthMonth": 1 }),
        json!({ "termsVersion": TERMS_VERSION, "birthYear": 2000, "birthMonth": 13 }),
        json!({ "termsVersion": TERMS_VERSION, "birthYear": 2000, "birthMonth": 0 }),
        json!({ "termsVersion": TERMS_VERSION, "birthYear": BIRTH_YEAR_EARLIEST - 1, "birthMonth": 12 }),
        json!({ "termsVersion": TERMS_VERSION, "birthYear": next_year, "birthMonth": next_month }),
    ] {
        let (status, answer) = post(&app, &token, body.clone()).await;
        assert_eq!(status, 400, "{body}: {answer}");
        assert_eq!(answer["error"]["code"], json!("bad_request"), "{body}");
    }

    // Nothing was stored and nothing was deleted.
    let consent = consent_of(&app, "p-typo").await;
    assert_eq!((consent.age_checked_at, consent.terms.len()), (None, 0));
    assert!(profile_exists(&app, "p-typo").await);
}

#[tokio::test]
async fn r757001_outdated_version_is_update_required() {
    let app = test_app().await;
    let token = owing(&app, "p-stale", "active").await;

    let (status, body) = post(&app, &token, json!({ "termsVersion": "2000-01-01/2000-01-01" })).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"]["code"], json!("update_required"));

    let (status, body) = post(&app, &token, json!({ "termsVersion": 1 })).await;
    assert_eq!(status, 400, "{body}");

    assert!(consent_of(&app, "p-stale").await.terms.is_empty());
    assert_eq!(me(&app, &token).await["termsOwed"], json!(true));
}

#[tokio::test]
async fn r757001_active_account_reaccepts_without_birth() {
    let app = test_app().await;
    let token = owing(&app, "p-old", "active").await;

    // An account active before the age screen is not asked its age, only the terms.
    let before = me(&app, &token).await;
    assert_eq!(before["ageCheckOwed"], json!(false), "{before}");
    assert_eq!(before["termsOwed"], json!(true), "{before}");

    let (status, body) = post(&app, &token, json!({ "termsVersion": TERMS_VERSION })).await;
    assert_eq!(status, 200, "{body}");
    // Agreeing twice records the version once.
    let (status, _) = post(&app, &token, json!({ "termsVersion": TERMS_VERSION })).await;
    assert_eq!(status, 200);

    let consent = consent_of(&app, "p-old").await;
    assert_eq!(consent.age_checked_at, None);
    assert_eq!(consent.terms.len(), 1);
    assert_eq!(consent.terms[0].version, TERMS_VERSION);
    assert_eq!(consent.terms[0].source, TermsSource::Reaccept);
    assert_eq!(me(&app, &token).await["termsOwed"], json!(false));
}
