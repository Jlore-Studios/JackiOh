//! The retention purge (`crates/server/src/api/retention.rs`): which cutoffs it hands the store
//! (docs/v0.3.0/SURFACE.md §11.2). What the store then deletes is asserted in `tests/store/contract.rs`
//! and `tests/sql/07_retention_purge.sql`.

use serde_json::json;

use jackioh_engine::wire::GameRecord;
use jackioh_server::api::retention::purge_expired;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    CODE_ATTEMPT_RETENTION_DAYS, GAME_RECORD_RAW_RETENTION_DAYS, IDEMPOTENCY_KEY_TTL_HOURS,
    MATCH_ACTION_RETENTION_DAYS, PLAY_TELEMETRY_RETENTION_DAYS,
};
use jackioh_server::db::store::{CodeAttempt, MatchActionRow, MatchRow, MatchSignalRow};
use jackioh_server::economy::idempotency::sweep_expired;
use jackioh_server::economy::store::IdempotencyRow;

use crate::support::deps::test_app;
use crate::support::store::{fake_lock, from, to_json};

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
const MINUTE_MS: i64 = 60_000;

// Harness (a private copy per file)

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

/// R1442: one seat's end-of-game signals, the smallest row of a match's play telemetry.
fn signals_row(match_id: &str) -> MatchSignalRow {
    from(json!({
        "matchId": match_id,
        "seat": "p1",
        "drawOffers": 0,
        "drawAccepted": false,
        "rematchOffered": false,
        "rematchAccepted": false,
        "timeouts": 0,
        "pilot": "human",
    }))
}

/// A live game record with nothing in it but its id.
fn game_record(id: &str) -> GameRecord {
    from(json!({
        "id": id,
        "source": "live",
        "mode": "bo1",
        "patch": "v0.1.1",
        "pilots": { "p1": "human", "p2": "human" },
        "game": {
            "first": "p1",
            "winner": "p1",
            "reason": "hero-death",
            "turns": 5,
            "seats": {
                "p1": { "deck": ["a"], "opening": [], "drawn": [], "played": [] },
                "p2": { "deck": ["b"], "opening": [], "drawn": [], "played": [] }
            }
        }
    }))
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
        let play_telemetry_ended_before = now - PLAY_TELEMETRY_RETENTION_DAYS * DAY_MS;

        log_attempt(&app, "at-cutoff", code_attempts_before).await;
        log_attempt(&app, "past-cutoff", code_attempts_before - 1).await;
        {
            let mut data = fake_lock(&app).await;
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
            // R1442: the play telemetry's own cutoff, on matches with no log left.
            data.tables
                .matches
                .push(finished_match("t-at-cutoff", Some(play_telemetry_ended_before)));
            data.tables.matches.push(finished_match(
                "t-past-cutoff",
                Some(play_telemetry_ended_before - 1),
            ));
            for match_id in ["t-at-cutoff", "t-past-cutoff"] {
                data.tables.match_signals.push(signals_row(match_id));
            }
        }
        // R1044001: a game record's detail, on its own cutoff, written then dated by hand.
        let game_records_recorded_before = now - GAME_RECORD_RAW_RETENTION_DAYS * DAY_MS;
        for (id, recorded_at) in [
            ("g-at-cutoff", game_records_recorded_before),
            ("g-past-cutoff", game_records_recorded_before - 1),
        ] {
            let mut t = app.db.begin(None).await.expect("begin");
            assert!(
                t.game_records_insert(&game_record(id))
                    .await
                    .expect("gameRecords.insert")
            );
            t.commit().await.expect("commit");
            fake_lock(&app).await.tables.game_record_ledger[id].recorded_at = recorded_at;
        }

        let purged = purge_expired(&app).await.expect("purgeExpired");
        assert_eq!(
            to_json(&purged),
            json!({ "codeAttempts": 1, "matchActions": 1, "playTelemetry": 1, "gameRecords": 1 })
        );
        let data = fake_lock(&app).await;
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
        let signals: Vec<String> = data
            .tables
            .match_signals
            .iter()
            .map(|row| row.match_id.clone())
            .collect();
        assert_eq!(signals, vec!["t-at-cutoff".to_string()]);
        let records: Vec<String> = data
            .tables
            .game_records
            .iter()
            .map(|row| row.id.clone())
            .collect();
        assert_eq!(records, vec!["g-at-cutoff".to_string()]);
        // The emptied record's row stays, so the same game is never written again.
        assert!(data.tables.game_record_ledger.contains_key("g-past-cutoff"));
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
            let mut data = fake_lock(&app).await;
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
        assert_eq!(
            to_json(&purged),
            json!({ "codeAttempts": 1, "matchActions": 1, "playTelemetry": 0, "gameRecords": 0 })
        );
        let data = fake_lock(&app).await;
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

    #[tokio::test(start_paused = true)]
    async fn r2124000_the_sweep_takes_keys_past_their_ttl_and_keeps_the_rest() {
        let app = test_app().await;
        let cutoff = now_ms() - IDEMPOTENCY_KEY_TTL_HOURS * HOUR_MS;
        let mut t = app.db.begin(None).await.expect("begin");
        for (n, created_at) in [(1, cutoff - 1), (2, cutoff), (3, cutoff + MINUTE_MS)] {
            let row: IdempotencyRow = from(json!({
                "profileId": "p1", "key": format!("00000000-0000-4000-8000-00000212440{n}"),
                "route": "/api/packs/buy", "bodyHash": "ab".repeat(32), "status": 200,
                "body": { "pack": "Core", "jlore": 0, "unopened": n }, "createdAt": created_at,
            }));
            t.economy_idempotency_put(&row)
                .await
                .expect("economy.idempotencyKeys.put");
        }
        t.commit().await.expect("commit");

        assert_eq!(sweep_expired(&app).await.expect("the sweep"), 1);
        let kept: Vec<i64> = fake_lock(&app)
            .await
            .tables
            .economy
            .idempotency_keys
            .iter()
            .map(|row| row.created_at)
            .collect();
        assert_eq!(kept, vec![cutoff, cutoff + MINUTE_MS]);
        assert_eq!(sweep_expired(&app).await.expect("the sweep again"), 0);
    }
}
