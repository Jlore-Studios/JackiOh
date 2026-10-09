//! The retention purge (`crates/server/src/api/retention.rs`): which cutoffs it hands the store.
//! What the store then deletes is asserted against both stores in `tests/store/contract.rs`, and in
//! Postgres by `tests/sql/07_retention_purge.sql`.
//!
//! Runs on `support::deps::test_app()` (the fake store), with `tokio::time::pause()` standing in
//! for the manual clock.
//! Surface contract: docs/v0.3.0/SURFACE.md §11.2.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use jackioh_server::api::retention::purge_expired;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{CODE_ATTEMPT_RETENTION_DAYS, MATCH_ACTION_RETENTION_DAYS};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CodeAttempt, Db, MatchActionRow, MatchRow};

use crate::support::deps::test_app;

const DAY_MS: i64 = 86_400_000;

// Harness (a private copy per file)

/// Builds a value from a JSON literal, so the test depends on the JSON shape only.
fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// The fake store's tables behind the test app.
async fn fake(app: &App) -> tokio::sync::MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

fn finished_match(id: &str, finished_at: Option<i64>) -> MatchRow {
    from(json!({
        "id": id,
        "seed": "seed",
        "players": ["p1", "p2"],
        "decks": [[], []],
        "catalogVersion": "test-1",
        "ranked": false,
        "status": if finished_at.is_none() { "live" } else { "finished" },
        "createdAt": 0,
        "finishedAt": finished_at,
        "clocks": { "turnDeadline": null, "promptDeadline": null, "graceDeadline": { "p1": null, "p2": null }, "ceilingAt": 0 },
    }))
}

fn action_row(match_id: &str) -> MatchActionRow {
    from(
        json!({ "matchId": match_id, "seq": 1, "action": { "type": "endTurn", "playerId": "p1", "nonce": "n" }, "at": 0 }),
    )
}

/// Logs one code attempt in a transaction of its own.
async fn log_attempt(app: &App, ip_hash: &str, at: i64) {
    let attempt: CodeAttempt = from(
        json!({ "profileId": null, "ipHash": ip_hash, "result": "rejected", "reason": "missing", "at": at }),
    );
    let mut t = app.db.begin(None).await.expect("begin");
    t.codes_log_attempt(&attempt).await.expect("codes.logAttempt");
    t.commit().await.expect("commit");
}

// The retention purge

mod the_retention_purge {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn asks_the_store_for_the_cutoffs_config_ts_names_counted_back_from_now() {
        // The store is an enum with no spy seam, so the cutoffs are pinned from outside: a row exactly
        // at each is kept and one a millisecond older is purged.
        let app = test_app().await;
        let now = now_ms();
        let code_attempts_before = now - CODE_ATTEMPT_RETENTION_DAYS * DAY_MS;
        let match_actions_ended_before = now - MATCH_ACTION_RETENTION_DAYS * DAY_MS;

        log_attempt(&app, "at-cutoff", code_attempts_before).await;
        log_attempt(&app, "past-cutoff", code_attempts_before - 1).await;
        {
            let mut data = fake(&app).await;
            data.tables
                .matches
                .push(finished_match("m-at-cutoff", Some(match_actions_ended_before)));
            data.tables.matches.push(finished_match(
                "m-past-cutoff",
                Some(match_actions_ended_before - 1),
            ));
            for match_id in ["m-at-cutoff", "m-past-cutoff"] {
                data.tables.match_actions.push(action_row(match_id));
            }
        }

        let purged = purge_expired(&app).await.expect("purgeExpired");
        assert_eq!(json_of(&purged), json!({ "codeAttempts": 1, "matchActions": 1 }));
        let data = fake(&app).await;
        let attempts: Vec<String> = data
            .tables
            .attempts
            .iter()
            .map(|row| row.ip_hash.clone())
            .collect();
        assert_eq!(attempts, vec!["at-cutoff".to_string()]);
        let actions: Vec<String> = data
            .tables
            .match_actions
            .iter()
            .map(|row| row.match_id.clone())
            .collect();
        assert_eq!(actions, vec!["m-at-cutoff".to_string()]);
    }

    #[tokio::test(start_paused = true)]
    async fn deletes_old_attempts_and_the_logs_of_long_finished_matches_and_keeps_the_rest() {
        let app = test_app().await;
        let now = now_ms();
        let at = |days: i64| now - days * DAY_MS;
        let code_days = CODE_ATTEMPT_RETENTION_DAYS;
        let match_days = MATCH_ACTION_RETENTION_DAYS;
        for (ip_hash, days) in [("old", code_days + 1), ("recent", code_days - 1)] {
            log_attempt(&app, ip_hash, at(days)).await;
        }
        {
            let mut data = fake(&app).await;
            data.tables
                .matches
                .push(finished_match("m-old", Some(at(match_days + 1))));
            data.tables
                .matches
                .push(finished_match("m-recent", Some(at(match_days - 1))));
            data.tables.matches.push(finished_match("m-live", None));
            for match_id in ["m-old", "m-recent", "m-live"] {
                data.tables.match_actions.push(action_row(match_id));
            }
        }

        let purged = purge_expired(&app).await.expect("purgeExpired");
        assert_eq!(json_of(&purged), json!({ "codeAttempts": 1, "matchActions": 1 }));
        let data = fake(&app).await;
        let attempts: Vec<String> = data
            .tables
            .attempts
            .iter()
            .map(|row| row.ip_hash.clone())
            .collect();
        assert_eq!(attempts, vec!["recent".to_string()]);
        let actions: Vec<String> = data
            .tables
            .match_actions
            .iter()
            .map(|row| row.match_id.clone())
            .collect();
        assert_eq!(actions, vec!["m-recent".to_string(), "m-live".to_string()]);
    }
}
