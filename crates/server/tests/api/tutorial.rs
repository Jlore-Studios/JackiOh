//! Tutorial progress on the account (`src/api/tutorial.rs`, SPEC §9.10, R320).
//!
//! The questions here are the endpoints' own: who may call them (an active account, and only about
//! itself), what a body must look like, what a write does to what is stored (a union, and the newer
//! choice), and that a time from a clock running ahead is taken as now. The merge itself is the
//! store's and is asserted against both stores in `tests/store/contract.rs`; the database's half —
//! RLS, the grants and the SQL function — is `tests/sql/05_tutorial_progress.sql` and
//! `02_rls_as_client.sql`.
//!
//! Port of `apps/server/test/api/tutorial.test.ts` (part 18). TS built a router over the tutorial
//! routes alone with seeded profiles and scripted tokens; here every request goes through the whole
//! app (`support::deps::test_app`, the E2E fixtures), so the caller is `e2e-p1`, the other account
//! `e2e-p2` and the pending one `e2e-pending`, and a banned account is `e2e-p2` set to `banned`.
//! TS's manual clock is the server's own wall clock: a time "now" is read from it around the call.

use std::sync::Arc;
use std::time::Duration;

use jackioh_server::app::App;
use jackioh_server::config::{TUTORIAL_LESSON_ID_MAX_LENGTH, TUTORIAL_LESSONS_MAX};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::support::deps::{call, test_app};

/// The caller's bearer token and managed-auth user id (`e2e/support/config.ts`'s `e2e-p1`).
const TOKEN: &str = "e2e-token-p1";
const USER: &str = "e2e-p1";
/// The other active account (`e2e-p2`).
const OTHER_TOKEN: &str = "e2e-token-p2";
const OTHER_USER: &str = "e2e-p2";
/// The pending account (`e2e-pending`).
const PENDING_TOKEN: &str = "e2e-token-pending";
const PENDING_USER: &str = "e2e-pending";

/// JS's `Number.MAX_SAFE_INTEGER + 2`: past the largest whole number a client can send exactly.
const PAST_SAFE_INTEGER: i64 = 9_007_199_254_740_993;

/// TS's `beforeEach`: a fresh app, and the profile ids of the three fixture accounts.
struct Fixture {
    app: Arc<App>,
    profile: String,
    other: String,
    pending: String,
}

async fn setup() -> Fixture {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let other = profile_id_of(&app, OTHER_USER).await;
    let pending = profile_id_of(&app, PENDING_USER).await;
    Fixture {
        app,
        profile,
        other,
        pending,
    }
}

/// A store value built from TS's own object literal.
fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("a store value from its JSON")
}

/// A config count as a `usize`, whichever integer type `config.rs` gives it.
fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
}

/// The clock the server reads (`app::now_ms`), in epoch milliseconds: the wall clock as the server
/// anchored it, moved by tokio's.
fn wall_ms() -> i64 {
    jackioh_server::app::now_ms()
}

async fn profile_id_of(app: &App, user_id: &str) -> String {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let profile = tx
        .profiles_get_by_user_id(user_id)
        .await
        .expect("profiles.getByUserId")
        .unwrap_or_else(|| panic!("the fixture account {user_id} has a profile"));
    tx.commit().await.expect("commit");
    profile.id
}

async fn set_status(app: &App, profile_id: &str, status: &str) {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    tx.profiles_set_status(profile_id, from(json!(status)))
        .await
        .expect("profiles.setStatus");
    tx.commit().await.expect("commit");
}

/// The profiles that hold a tutorial row, of the three fixture accounts (TS read
/// `store.tables.tutorial`, which held no one else's).
async fn stored(fixture: &Fixture) -> Vec<String> {
    let mut held = Vec::new();
    for profile_id in [&fixture.profile, &fixture.other, &fixture.pending] {
        let mut tx = fixture.app.db.begin(None).await.expect("a store transaction");
        let row = tx.tutorial_get(profile_id).await.expect("tutorial.get");
        tx.commit().await.expect("commit");
        if row.is_some() {
            held.push(profile_id.clone());
        }
    }
    held
}

async fn get(app: &Arc<App>, bearer: &str) -> (u16, Value) {
    let (status, _, body) = call(app, "GET", "/api/tutorial", Some(bearer), Value::Null).await;
    (status, body)
}

async fn put(app: &Arc<App>, body: Value, bearer: &str) -> (u16, Value) {
    let (status, _, body) = call(app, "PUT", "/api/tutorial", Some(bearer), body).await;
    (status, body)
}

fn progress_of(response: (u16, Value)) -> Value {
    let (status, body) = response;
    assert_eq!(status, 200, "{body}");
    body["progress"].clone()
}

/// The `completed` list of a progress answer.
fn completed_of(progress: &Value) -> Vec<String> {
    progress["completed"]
        .as_array()
        .expect("completed is a list")
        .iter()
        .map(|lesson| lesson.as_str().expect("a lesson id").to_string())
        .collect()
}

mod r320_the_routes_an_active_account_and_only_about_itself {
    use super::*;

    #[tokio::test]
    async fn r320_declares_get_and_put_api_tutorial_both_active() {
        // TS read the declared route list; here the app answers for it. GET and PUT exist, every
        // other method on the path is the router's 404, and both turn a pending account away, which
        // a `user` route would not.
        let f = setup().await;
        assert_eq!(get(&f.app, TOKEN).await.0, 200);
        assert_eq!(put(&f.app, json!({ "completed": [] }), TOKEN).await.0, 200);
        for method in ["POST", "DELETE", "PATCH"] {
            let (status, _, body) = call(&f.app, method, "/api/tutorial", Some(TOKEN), json!({})).await;
            assert_eq!(status, 404, "{method} /api/tutorial");
            assert_eq!(body["error"]["code"], "not_found");
        }
        assert_eq!(get(&f.app, PENDING_TOKEN).await.0, 403);
        assert_eq!(
            put(&f.app, json!({ "completed": [] }), PENDING_TOKEN).await.0,
            403
        );
    }

    #[tokio::test]
    async fn r320_refuses_a_caller_with_no_token_401_and_a_pending_or_banned_account_403_writing_nothing() {
        let f = setup().await;
        set_status(&f.app, &f.other, "banned").await;
        let (pending, banned) = (PENDING_TOKEN, OTHER_TOKEN);
        let body = json!({ "completed": ["basics"] });

        let (no_token, _, _) = call(&f.app, "PUT", "/api/tutorial", None, body.clone()).await;
        assert_eq!(no_token, 401);
        for bearer in [pending, banned] {
            assert_eq!(get(&f.app, bearer).await.0, 403);
            assert_eq!(put(&f.app, body.clone(), bearer).await.0, 403);
        }
        assert!(stored(&f).await.is_empty());
    }

    #[tokio::test]
    async fn r320_reads_an_account_with_no_progress_yet_as_none_and_writes_land_on_the_callers_own_row_only()
    {
        let f = setup().await;
        assert_eq!(
            progress_of(get(&f.app, TOKEN).await),
            json!({ "completed": [], "hiddenChoice": null })
        );

        // A body naming another profile changes nothing about whose row is written: the profile is the
        // verified token's, never a field of the request.
        progress_of(
            put(
                &f.app,
                json!({ "completed": ["basics"], "profileId": f.other }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            progress_of(get(&f.app, TOKEN).await),
            json!({ "completed": ["basics"], "hiddenChoice": null })
        );
        assert_eq!(
            progress_of(get(&f.app, OTHER_TOKEN).await),
            json!({ "completed": [], "hiddenChoice": null })
        );
        assert_eq!(stored(&f).await, std::slice::from_ref(&f.profile));
    }
}

mod r320_a_write_merges_into_the_account_and_never_takes_anything_away {
    use super::*;

    #[tokio::test]
    async fn r320_unions_the_lessons_a_stale_devices_write_removes_none_and_the_answer_is_the_merged_progress()
     {
        let f = setup().await;
        let put_completed = |completed: Value| put(&f.app, json!({ "completed": completed }), TOKEN);
        assert_eq!(
            completed_of(&progress_of(put_completed(json!(["spells", "basics"])).await)),
            ["basics", "spells"]
        );
        // A device that has won only lesson 1, or nothing at all.
        assert_eq!(
            completed_of(&progress_of(put_completed(json!(["basics"])).await)),
            ["basics", "spells"]
        );
        assert_eq!(
            completed_of(&progress_of(put_completed(json!([])).await)),
            ["basics", "spells"]
        );
        // Another device's lesson joins them; the same write again is the same answer.
        let merged = progress_of(put_completed(json!(["traps", "traps"])).await);
        assert_eq!(completed_of(&merged), ["basics", "spells", "traps"]);
        assert_eq!(progress_of(put_completed(json!(["traps"])).await), merged);
        assert_eq!(progress_of(get(&f.app, TOKEN).await), merged);
    }

    #[tokio::test]
    async fn r320_keeps_the_newest_hide_show_choice_show_made_later_is_not_undone_by_an_older_hide() {
        let f = setup().await;
        let t0 = wall_ms() - 60_000;
        let hide = progress_of(
            put(
                &f.app,
                json!({ "completed": [], "hiddenChoice": { "hidden": true, "at": t0 } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(hide["hiddenChoice"], json!({ "hidden": true, "at": t0 }));

        let show = progress_of(
            put(
                &f.app,
                json!({ "completed": [], "hiddenChoice": { "hidden": false, "at": t0 + 30_000 } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            show["hiddenChoice"],
            json!({ "hidden": false, "at": t0 + 30_000 })
        );

        // The first device, still holding its older Hide, writes again.
        let stale = progress_of(
            put(
                &f.app,
                json!({ "completed": ["basics"], "hiddenChoice": { "hidden": true, "at": t0 } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            stale,
            json!({ "completed": ["basics"], "hiddenChoice": { "hidden": false, "at": t0 + 30_000 } })
        );

        // A write with no choice (absent or null) leaves the stored one alone.
        assert_eq!(
            progress_of(put(&f.app, json!({ "completed": [] }), TOKEN).await)["hiddenChoice"],
            json!({ "hidden": false, "at": t0 + 30_000 })
        );
        assert_eq!(
            progress_of(put(&f.app, json!({ "completed": [], "hiddenChoice": null }), TOKEN).await)["hiddenChoice"],
            json!({ "hidden": false, "at": t0 + 30_000 })
        );
    }

    #[tokio::test]
    async fn r320_takes_a_choice_timed_after_the_servers_clock_as_made_now_so_a_clock_running_ahead_cannot_pin_it()
     {
        let f = setup().await;
        let before = wall_ms();
        let ahead = progress_of(
            put(
                &f.app,
                json!({ "completed": [], "hiddenChoice": { "hidden": true, "at": before + 86_400_000 } }),
                TOKEN,
            )
            .await,
        );
        let after = wall_ms();
        let now = ahead["hiddenChoice"]["at"].as_i64().expect("a stored time");
        assert!(
            (before..=after).contains(&now),
            "{now} is the server's now, between {before} and {after}"
        );
        assert_eq!(ahead["hiddenChoice"], json!({ "hidden": true, "at": now }));

        // A moment later, a choice from a device whose clock is right wins over it. (TS moved its
        // manual clock a minute; the wall clock is waited on until it has moved past `now`.)
        while wall_ms() <= now {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let device = wall_ms();
        let later = progress_of(
            put(
                &f.app,
                json!({ "completed": [], "hiddenChoice": { "hidden": false, "at": device } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(later["hiddenChoice"], json!({ "hidden": false, "at": device }));
    }
}

mod r320_the_body_is_checked_before_anything_is_stored {
    use super::*;

    async fn refused(app: &Arc<App>, body: Value) -> Value {
        let (status, answer) = put(app, body, TOKEN).await;
        assert_eq!(status, 400, "{answer}");
        answer
    }

    #[tokio::test]
    async fn r320_refuses_a_body_whose_lessons_are_not_a_list_of_lower_case_slugs() {
        let f = setup().await;
        let too_long = "a".repeat(count(TUTORIAL_LESSON_ID_MAX_LENGTH) + 1);
        for completed in [
            None,
            Some(json!(null)),
            Some(json!("basics")),
            Some(json!({ "basics": true })),
            Some(json!([1])),
            Some(json!([null])),
            Some(json!([""])),
            Some(json!(["Basics"])),
            Some(json!(["a b"])),
            Some(json!(["-a"])),
            Some(json!(["a--b"])),
            Some(json!(["a-"])),
            Some(json!([too_long])),
        ] {
            let body = match completed {
                None => json!({}),
                Some(completed) => json!({ "completed": completed }),
            };
            assert_eq!(refused(&f.app, body).await["error"]["code"], "bad_request");
        }
        // The longest id allowed is fine.
        let longest = "a".repeat(count(TUTORIAL_LESSON_ID_MAX_LENGTH));
        assert_eq!(
            completed_of(&progress_of(
                put(&f.app, json!({ "completed": [longest] }), TOKEN).await
            ))
            .len(),
            1
        );
    }

    #[tokio::test]
    async fn r320_refuses_more_distinct_lessons_than_an_account_holds_counting_a_repeat_once() {
        let f = setup().await;
        let many: Vec<String> = (0..count(TUTORIAL_LESSONS_MAX) + 1)
            .map(|at| format!("lesson-{at}"))
            .collect();
        let message = refused(&f.app, json!({ "completed": many })).await["error"]["message"].clone();
        assert!(
            message
                .as_str()
                .unwrap_or_default()
                .contains(&TUTORIAL_LESSONS_MAX.to_string()),
            "{message}"
        );
        let first = &many[..count(TUTORIAL_LESSONS_MAX)];
        let repeated: Vec<&String> = first.iter().chain(first).collect();
        assert_eq!(
            completed_of(&progress_of(
                put(&f.app, json!({ "completed": repeated }), TOKEN).await
            ))
            .len(),
            count(TUTORIAL_LESSONS_MAX)
        );
        assert_eq!(stored(&f).await.len(), 1);
    }

    #[tokio::test]
    async fn r320_answers_a_union_past_the_cap_with_409_and_writes_nothing() {
        let f = setup().await;
        let first: Vec<String> = (0..count(TUTORIAL_LESSONS_MAX))
            .map(|at| format!("lesson-{at}"))
            .collect();
        progress_of(put(&f.app, json!({ "completed": first }), TOKEN).await);
        let (status, body) = put(&f.app, json!({ "completed": ["one-more"] }), TOKEN).await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"]["details"], json!({ "limit": TUTORIAL_LESSONS_MAX }));
        assert!(!completed_of(&progress_of(get(&f.app, TOKEN).await)).contains(&"one-more".to_string()));
    }

    #[tokio::test]
    async fn r320_refuses_a_malformed_choice() {
        let f = setup().await;
        for hidden_choice in [
            json!(true),
            json!("hidden"),
            json!([]),
            json!({ "hidden": "yes", "at": 1 }),
            json!({ "hidden": true }),
            json!({ "hidden": true, "at": -1 }),
            json!({ "hidden": true, "at": 1.5 }),
            json!({ "hidden": true, "at": PAST_SAFE_INTEGER }),
        ] {
            let body = json!({ "completed": [], "hiddenChoice": hidden_choice });
            assert_eq!(refused(&f.app, body).await["error"]["code"], "bad_request");
        }
        assert!(stored(&f).await.is_empty());
    }
}
