//! Player settings on the account (`src/api/settings.rs`, SPEC §9.1, R633, R634).
//!
//! The questions here are the endpoints' own: who may call them (an active account, and only about
//! itself), what a body must look like, what a write does to what is stored (a group replaced only by
//! a strictly later one, the others left alone), and that a time from a clock running ahead is taken
//! as now. The merge itself is the store's and is asserted against both stores in
//! `tests/store/contract.rs`; the database's half — RLS, the grants and the SQL function — is
//! `tests/sql/11_player_settings.sql` and `02_rls_as_client.sql`.
//!
//! Every request goes through the whole app (`support::deps::test_app`, the E2E fixtures): the
//! caller is `e2e-p1`, the other account `e2e-p2`, the pending one `e2e-pending`, and a banned
//! account is `e2e-p2` set to `banned`. A time "now" is read from the server's wall clock.

use std::sync::Arc;
use std::time::Duration;

use jackioh_server::app::App;
use jackioh_server::config::{
    PLAYER_SETTINGS_BYTES_MAX, PLAYER_SETTINGS_GROUPS_MAX, PLAYER_SETTINGS_KEYS_MAX,
    PLAYER_SETTINGS_NAME_MAX_LENGTH, PLAYER_SETTINGS_TEXT_MAX_LENGTH,
};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

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

/// `Number.MAX_SAFE_INTEGER + 2`: past the largest whole number a client can send exactly.
const PAST_SAFE_INTEGER: i64 = 9_007_199_254_740_993;

/// A fresh app, and the profile ids of the three fixture accounts.
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

/// A store value built from JSON.
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

/// An object with computed keys.
fn object(entries: impl IntoIterator<Item = (String, Value)>) -> Value {
    Value::Object(entries.into_iter().collect::<Map<String, Value>>())
}

/// Every key `expected` names is in `actual` and matches, recursively.
fn is_match(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|held| is_match(held, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(held, value)| is_match(held, value))
        }
        _ => actual == expected,
    }
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

/// The profiles, of the three fixture accounts, that hold a settings row.
async fn stored(fixture: &Fixture) -> Vec<String> {
    let mut held = Vec::new();
    for profile_id in [&fixture.profile, &fixture.other, &fixture.pending] {
        let mut tx = fixture.app.db.begin(None).await.expect("a store transaction");
        let row = tx
            .player_settings_get(profile_id)
            .await
            .expect("playerSettings.get");
        tx.commit().await.expect("commit");
        if row.is_some() {
            held.push(profile_id.clone());
        }
    }
    held
}

async fn get(app: &Arc<App>, bearer: &str) -> (u16, Value) {
    let (status, _, body) = call(app, "GET", "/api/settings", Some(bearer), Value::Null).await;
    (status, body)
}

async fn put(app: &Arc<App>, body: Value, bearer: &str) -> (u16, Value) {
    let (status, _, body) = call(app, "PUT", "/api/settings", Some(bearer), body).await;
    (status, body)
}

fn settings_of(response: (u16, Value)) -> Value {
    let (status, body) = response;
    assert_eq!(status, 200, "{body}");
    body["settings"].clone()
}

mod r633_the_routes_an_active_account_and_only_about_itself {
    use super::*;

    #[tokio::test]
    async fn r633_declares_get_and_put_api_settings_both_active() {
        // GET and PUT exist, every other method on the path is the router's 404, and both turn a
        // pending account away, which a `user` route would not.
        let f = setup().await;
        assert_eq!(get(&f.app, TOKEN).await.0, 200);
        assert_eq!(put(&f.app, json!({ "groups": {} }), TOKEN).await.0, 200);
        for method in ["POST", "DELETE", "PATCH"] {
            let (status, _, body) = call(&f.app, method, "/api/settings", Some(TOKEN), json!({})).await;
            assert_eq!(status, 404, "{method} /api/settings");
            assert_eq!(body["error"]["code"], "not_found");
        }
        assert_eq!(get(&f.app, PENDING_TOKEN).await.0, 403);
        assert_eq!(put(&f.app, json!({ "groups": {} }), PENDING_TOKEN).await.0, 403);
    }

    #[tokio::test]
    async fn r633_refuses_a_caller_with_no_token_401_and_a_pending_or_banned_account_403_writing_nothing() {
        let f = setup().await;
        set_status(&f.app, &f.other, "banned").await;
        let (pending, banned) = (PENDING_TOKEN, OTHER_TOKEN);
        let body = json!({ "groups": { "audio": { "at": 1, "values": { "master": 0.5 } } } });

        let (no_token, _, _) = call(&f.app, "PUT", "/api/settings", None, body.clone()).await;
        assert_eq!(no_token, 401);
        for bearer in [pending, banned] {
            assert_eq!(get(&f.app, bearer).await.0, 403);
            assert_eq!(put(&f.app, body.clone(), bearer).await.0, 403);
        }
        assert!(stored(&f).await.is_empty());
    }

    #[tokio::test]
    async fn r633_reads_an_account_with_no_settings_yet_as_none_and_writes_land_on_the_callers_own_row_only()
    {
        let f = setup().await;
        assert_eq!(settings_of(get(&f.app, TOKEN).await), json!({ "groups": {} }));

        // A body naming another profile changes nothing about whose row is written: the profile is the
        // verified token's, never a field of the request.
        settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": 5, "values": { "master": 0.5 } } }, "profileId": f.other }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            settings_of(get(&f.app, TOKEN).await),
            json!({ "groups": { "audio": { "at": 5, "values": { "master": 0.5 } } } })
        );
        assert_eq!(
            settings_of(get(&f.app, OTHER_TOKEN).await),
            json!({ "groups": {} })
        );
        assert_eq!(stored(&f).await, std::slice::from_ref(&f.profile));
    }
}

mod r634_a_write_replaces_a_group_only_with_a_strictly_later_one {
    use super::*;

    #[tokio::test]
    async fn r634_a_later_group_replaces_the_stored_one_whole_an_older_or_tied_one_changes_nothing_the_answer_is_what_is_stored()
     {
        let f = setup().await;
        let t = wall_ms() - 60_000;
        settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": t, "values": { "master": 0.5, "muted": false } } } }),
                TOKEN,
            )
            .await,
        );

        let later = settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": t + 1000, "values": { "master": 0.9 } } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            later["groups"]["audio"],
            json!({ "at": t + 1000, "values": { "master": 0.9 } })
        );

        // A device that last changed the audio before that, and one that did so at the same moment.
        let older = settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": t + 500, "values": { "master": 0.1 } } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(older, later);
        let tied = settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": t + 1000, "values": { "master": 0.2 } } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(tied, later);
        assert_eq!(settings_of(get(&f.app, TOKEN).await), later);
    }

    #[tokio::test]
    async fn r634_a_group_a_write_does_not_name_stays_and_one_write_can_win_one_group_and_lose_another() {
        let f = setup().await;
        let t = wall_ms() - 60_000;
        settings_of(
            put(
                &f.app,
                json!({
                    "groups": {
                        "gameplay": { "at": t + 100, "values": { "dragToPlay": false } },
                        "audio": { "at": t, "values": { "master": 0.4 } }
                    }
                }),
                TOKEN,
            )
            .await,
        );

        let merged = settings_of(
            put(
                &f.app,
                json!({
                    "groups": {
                        "audio": { "at": t + 200, "values": { "master": 0.7 } },
                        "gameplay": { "at": t, "values": { "dragToPlay": true } }
                    }
                }),
                TOKEN,
            )
            .await,
        );

        assert_eq!(
            merged["groups"]["audio"],
            json!({ "at": t + 200, "values": { "master": 0.7 } })
        );
        assert_eq!(
            merged["groups"]["gameplay"],
            json!({ "at": t + 100, "values": { "dragToPlay": false } })
        );
        // And a write that names neither leaves both.
        let with_fx = settings_of(
            put(
                &f.app,
                json!({ "groups": { "fx": { "at": t, "values": { "speed": 2 } } } }),
                TOKEN,
            )
            .await,
        );
        let expected = json!({ "groups": { "audio": { "at": t + 200 }, "gameplay": { "at": t + 100 }, "fx": { "at": t } } });
        assert!(is_match(&with_fx, &expected), "{with_fx}");
        let unchanged = settings_of(put(&f.app, json!({ "groups": {} }), TOKEN).await);
        assert!(
            is_match(
                &unchanged,
                &json!({ "groups": { "audio": {}, "gameplay": {}, "fx": {} } })
            ),
            "{unchanged}"
        );
    }

    #[tokio::test]
    async fn r634_takes_a_group_timed_after_the_servers_clock_as_made_now_so_a_clock_running_ahead_cannot_pin_it()
     {
        let f = setup().await;
        let before = wall_ms();
        let ahead = settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": before + 86_400_000, "values": { "master": 0.1 } } } }),
                TOKEN,
            )
            .await,
        );
        let after = wall_ms();
        let now = ahead["groups"]["audio"]["at"].as_i64().expect("a stored time");
        assert!(
            (before..=after).contains(&now),
            "{now} is the server's now, between {before} and {after}"
        );
        assert_eq!(
            ahead["groups"]["audio"],
            json!({ "at": now, "values": { "master": 0.1 } })
        );

        // A moment later, a change from a device whose clock is right wins over it.
        while wall_ms() <= now {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let device = wall_ms();
        let later = settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": device, "values": { "master": 0.8 } } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            later["groups"]["audio"],
            json!({ "at": device, "values": { "master": 0.8 } })
        );
    }

    #[tokio::test]
    async fn r634_keeps_a_group_the_client_no_longer_has_and_hands_back_every_value_it_was_given() {
        let f = setup().await;
        let values = json!({ "on": true, "level": 0.25, "station": "lofi", "count": 3, "negative": -2 });
        let answer = settings_of(
            put(
                &f.app,
                json!({ "groups": { "old-store": { "at": 1, "values": values } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            answer["groups"]["old-store"],
            json!({ "at": 1, "values": values })
        );
    }
}

mod r633_the_body_is_checked_before_anything_is_stored {
    use super::*;

    async fn refused(app: &Arc<App>, body: Value) -> Value {
        let (status, answer) = put(app, body, TOKEN).await;
        assert_eq!(status, 400, "{answer}");
        answer
    }

    #[tokio::test]
    async fn r633_refuses_a_body_whose_groups_are_not_an_object() {
        let f = setup().await;
        for body in [
            json!({}),
            json!({ "groups": null }),
            json!({ "groups": [] }),
            json!({ "groups": "audio" }),
            json!({ "groups": 1 }),
        ] {
            assert_eq!(refused(&f.app, body).await["error"]["code"], "bad_request");
        }
        assert!(stored(&f).await.is_empty());
    }

    #[tokio::test]
    async fn r633_refuses_a_group_whose_id_is_not_a_lower_case_slug_of_a_bounded_length() {
        let f = setup().await;
        let group = json!({ "at": 1, "values": {} });
        let too_long = "a".repeat(count(PLAYER_SETTINGS_NAME_MAX_LENGTH) + 1);
        for id in ["Audio", "a b", "1a", "-a", "a--b", "a-", "", too_long.as_str()] {
            refused(
                &f.app,
                json!({ "groups": object([(id.to_string(), group.clone())]) }),
            )
            .await;
        }
        let longest = "a".repeat(count(PLAYER_SETTINGS_NAME_MAX_LENGTH));
        let answer = settings_of(put(&f.app, json!({ "groups": object([(longest, group)]) }), TOKEN).await);
        assert!(answer["groups"].is_object(), "{answer}");
    }

    #[tokio::test]
    async fn r633_refuses_a_group_that_is_not_at_values_with_a_whole_millisecond_time() {
        let f = setup().await;
        for group in [
            json!(null),
            json!(1),
            json!([]),
            json!("x"),
            json!({}),
            json!({ "at": 1 }),
            json!({ "values": {} }),
            json!({ "at": "1", "values": {} }),
            json!({ "at": -1, "values": {} }),
            json!({ "at": 1.5, "values": {} }),
            json!({ "at": PAST_SAFE_INTEGER, "values": {} }),
            json!({ "at": 1, "values": [] }),
            json!({ "at": 1, "values": null }),
        ] {
            assert_eq!(
                refused(&f.app, json!({ "groups": { "audio": group } })).await["error"]["code"],
                "bad_request"
            );
        }
        assert!(stored(&f).await.is_empty());
    }

    #[tokio::test]
    async fn r633_refuses_values_that_are_not_flat_booleans_finite_numbers_and_short_texts_and_names_that_are_not_words()
     {
        let f = setup().await;
        let too_long = "x".repeat(count(PLAYER_SETTINGS_TEXT_MAX_LENGTH) + 1);
        for value in [
            json!(null),
            json!([]),
            json!({}),
            json!([1]),
            json!({ "a": 1 }),
            json!(too_long),
        ] {
            refused(
                &f.app,
                json!({ "groups": { "audio": { "at": 1, "values": { "master": value } } } }),
            )
            .await;
        }
        let long_name = "x".repeat(count(PLAYER_SETTINGS_NAME_MAX_LENGTH) + 1);
        for name in ["", "1a", "a b", "a-b", "a.b", "__proto__x!", long_name.as_str()] {
            let values = object([(name.to_string(), json!(1))]);
            refused(
                &f.app,
                json!({ "groups": { "audio": { "at": 1, "values": values } } }),
            )
            .await;
        }
        // JSON cannot carry NaN or Infinity, so a number the parser keeps is finite; the longest text is fine.
        let longest = "x".repeat(count(PLAYER_SETTINGS_TEXT_MAX_LENGTH));
        let answer = settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": 1, "values": { "station": longest } } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(answer["groups"]["audio"]["values"], json!({ "station": longest }));
    }

    #[tokio::test]
    async fn r633_refuses_more_groups_and_more_settings_in_a_group_than_an_account_holds() {
        let f = setup().await;
        let many = object(
            (0..count(PLAYER_SETTINGS_GROUPS_MAX) + 1)
                .map(|at| (format!("group-{at}"), json!({ "at": 1, "values": {} }))),
        );
        let message = refused(&f.app, json!({ "groups": many })).await["error"]["message"].clone();
        assert!(
            message
                .as_str()
                .unwrap_or_default()
                .contains(&PLAYER_SETTINGS_GROUPS_MAX.to_string()),
            "{message}"
        );
        let values =
            object((0..count(PLAYER_SETTINGS_KEYS_MAX) + 1).map(|at| (format!("k{at}"), json!(true))));
        let message = refused(
            &f.app,
            json!({ "groups": { "audio": { "at": 1, "values": values } } }),
        )
        .await["error"]["message"]
            .clone();
        assert!(
            message
                .as_str()
                .unwrap_or_default()
                .contains(&PLAYER_SETTINGS_KEYS_MAX.to_string()),
            "{message}"
        );
        let fits = object((0..count(PLAYER_SETTINGS_KEYS_MAX)).map(|at| (format!("k{at}"), json!(true))));
        let answer = settings_of(
            put(
                &f.app,
                json!({ "groups": { "audio": { "at": 1, "values": fits } } }),
                TOKEN,
            )
            .await,
        );
        let held = answer["groups"]["audio"]["values"]
            .as_object()
            .map_or(0, Map::len);
        assert_eq!(held, count(PLAYER_SETTINGS_KEYS_MAX));
    }

    #[tokio::test]
    async fn r633_answers_a_result_past_the_accounts_caps_with_409_and_writes_nothing() {
        let f = setup().await;
        // Groups: fill the account to its group cap, then name one more.
        let filled = object((0..count(PLAYER_SETTINGS_GROUPS_MAX)).map(|at| {
            (
                format!("group-{at}"),
                json!({ "at": 1, "values": { "on": true } }),
            )
        }));
        settings_of(put(&f.app, json!({ "groups": filled }), TOKEN).await);
        let (status, body) = put(
            &f.app,
            json!({ "groups": { "one-more": { "at": 1, "values": {} } } }),
            TOKEN,
        )
        .await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(
            body["error"]["details"],
            json!({ "groups": PLAYER_SETTINGS_GROUPS_MAX, "bytes": PLAYER_SETTINGS_BYTES_MAX })
        );
        let held = settings_of(get(&f.app, TOKEN).await);
        assert!(held["groups"].get("one-more").is_none(), "{held}");
    }

    #[tokio::test]
    async fn r633_answers_a_result_past_the_byte_cap_with_409_and_a_group_already_held_can_still_be_replaced()
    {
        let f = setup().await;
        let text = "x".repeat(count(PLAYER_SETTINGS_TEXT_MAX_LENGTH));
        let wide = |prefix: &str| {
            object((0..count(PLAYER_SETTINGS_KEYS_MAX)).map(|at| (format!("{prefix}{at}"), json!(text))))
        };
        // Each of these groups is about 1.7 kB of text, so the third pushes the account past 4 kB.
        settings_of(
            put(
                &f.app,
                json!({ "groups": { "first": { "at": 1, "values": wide("a") } } }),
                TOKEN,
            )
            .await,
        );
        settings_of(
            put(
                &f.app,
                json!({ "groups": { "second": { "at": 1, "values": wide("b") } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            put(
                &f.app,
                json!({ "groups": { "third": { "at": 1, "values": wide("c") } } }),
                TOKEN
            )
            .await
            .0,
            409
        );
        // Replacing a held group with a smaller one is fine.
        let shrunk = settings_of(
            put(
                &f.app,
                json!({ "groups": { "first": { "at": 2, "values": { "a0": true } } } }),
                TOKEN,
            )
            .await,
        );
        assert_eq!(
            shrunk["groups"]["first"],
            json!({ "at": 2, "values": { "a0": true } })
        );
    }
}
