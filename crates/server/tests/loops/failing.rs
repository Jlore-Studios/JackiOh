//! `loops::FailingStreak` and the sweeps that feed it (#3770): a step that fails on every pass for
//! `LOOP_FAILING_ALERT_MS` and `LOOP_FAILING_ALERT_PASSES` passes in a row raises one `<step>.failing`
//! alert, logs `<step>.recovered` on its next success, and alerts again on a new streak. The store's
//! failure switches (`fail_on`, `stop_failing`) and tokio's paused clock drive each loop; every
//! reading falls half an interval after a pass, so it never races the next one.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;
use tokio::task::JoinHandle;

use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::api::{queue, results, retention, series};
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    CHALLENGE_SWEEP_INTERVAL_SECONDS, LOOP_FAILING_ALERT_MS, LOOP_FAILING_ALERT_PASSES,
    MATCH_CEILING_MINUTES, MATCH_REAPER_INTERVAL_SECONDS, MATCHMAKER_SWEEP_INTERVAL_SECONDS, MS_PER_SECOND,
    RETENTION_PURGE_INTERVAL_SECONDS, SERIES_SWEEP_INTERVAL_SECONDS,
};
use jackioh_server::db::store::{MatchRow, StoreError};
use jackioh_server::loops::FailingStreak;

use crate::support::deps::{RecordingLogger, empty_test_app, record_logs};
use crate::support::engine::{fake_deck, install_test_cards};
use crate::support::store::{
    advance, fail_on, fake_lock, fake_read, from, sleep_settled, stop_failing, to_json,
};

/// One minute, in the milliseconds the clock counts.
const MINUTE_MS: i64 = 60 * MS_PER_SECOND;

/// The pass (counting from 1, one every `interval_ms`) a streak that began on pass 1 alerts on:
/// the first with `LOOP_FAILING_ALERT_PASSES` passes behind it and `LOOP_FAILING_ALERT_MS` elapsed
/// since pass 1.
fn alerting_pass(interval_ms: i64) -> i64 {
    let mut pass = i64::from(LOOP_FAILING_ALERT_PASSES);
    while (pass - 1) * interval_ms < LOOP_FAILING_ALERT_MS {
        pass += 1;
    }
    pass
}

/// How many alerts a loop whose streak alerts on pass `alerting` has raised after each of `passes`
/// passes, counted from the streak's start: none before, one from then on.
fn expected_alerts(alerting: i64, passes: i64) -> Vec<usize> {
    (1..=passes).map(|pass| usize::from(pass >= alerting)).collect()
}

/// The `event` lines logged after each of `passes` passes of a loop that began just now, counted
/// from the lines there already were. The first reading is `first_wait_ms` on, the rest one
/// `interval_ms` apart.
async fn alerts_after_each_pass(
    logs: &RecordingLogger,
    event: &str,
    first_wait_ms: i64,
    interval_ms: i64,
    passes: i64,
) -> Vec<usize> {
    let before = logs.named(event).len();
    let mut counts = Vec::new();
    sleep_settled(first_wait_ms).await;
    counts.push(logs.named(event).len() - before);
    for _ in 1..passes {
        sleep_settled(interval_ms).await;
        counts.push(logs.named(event).len() - before);
    }
    counts
}

/// The names of the `.failing` events logged as alerts, in order.
fn failing_alerts(logs: &RecordingLogger) -> Vec<String> {
    logs.entries()
        .into_iter()
        .filter(|entry| entry.event.ends_with(".failing") && entry.data["alert"] == json!(true))
        .map(|entry| entry.event)
        .collect()
}

/// A loop's cadence, from a config constant in seconds, in milliseconds.
fn ms(seconds: i64) -> i64 {
    seconds * MS_PER_SECOND
}

#[tokio::test(start_paused = true)]
async fn a_streak_alerts_once_after_enough_passes_and_enough_time() {
    let logs = record_logs();
    let mut streak = FailingStreak::new("test.step");

    // Enough passes, not enough time: nothing, to the last millisecond.
    for _ in 0..LOOP_FAILING_ALERT_PASSES + 2 {
        streak.failed("down");
    }
    advance(LOOP_FAILING_ALERT_MS - 1).await;
    streak.failed("down");
    assert!(logs.named("test.step.failing").is_empty());
    advance(1).await;
    streak.failed("down");
    let alerts = logs.named("test.step.failing");
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].level, "alert");
    assert_eq!(alerts[0].data["alert"], json!(true));
    assert_eq!(alerts[0].data["step"], json!("test.step"));
    assert_eq!(alerts[0].data["message"], json!("down"));
    for _ in 0..5 {
        streak.failed("down");
    }
    assert_eq!(logs.named("test.step.failing").len(), 1);

    // The next success closes the streak, once.
    streak.ok();
    let recovered = logs.named("test.step.recovered");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].level, "info");
    streak.ok();
    assert_eq!(logs.named("test.step.recovered").len(), 1);

    // Enough time, not enough passes: nothing; one pass more alerts again.
    streak.failed("down again");
    advance(LOOP_FAILING_ALERT_MS).await;
    for _ in 1..LOOP_FAILING_ALERT_PASSES - 1 {
        streak.failed("down again");
    }
    assert_eq!(logs.named("test.step.failing").len(), 1);
    streak.failed("down again");
    let alerts = logs.named("test.step.failing");
    assert_eq!(alerts.len(), 2);
    assert_eq!(alerts[1].data["message"], json!("down again"));
    streak.ok();
    assert_eq!(logs.named("test.step.recovered").len(), 2);

    // A blip adds neither an alert nor a recovery.
    streak.failed("blip");
    streak.ok();
    assert_eq!(logs.named("test.step.failing").len(), 2);
    assert_eq!(logs.named("test.step.recovered").len(), 2);
}

#[tokio::test(start_paused = true)]
async fn the_reaper_alerts_once_while_the_store_keeps_failing_and_again_after_a_recovery() {
    let logs = record_logs();
    let app = empty_test_app().await;
    let interval = ms(MATCH_REAPER_INTERVAL_SECONDS);
    let alerting = alerting_pass(interval);
    fail_on(&app, "matches.live", "the live matches cannot be read").await;
    let reaper = tokio::spawn(results::run_reaper(app.clone()));

    let passes = alerting + 5;
    assert_eq!(
        alerts_after_each_pass(
            &logs,
            "matches.reaper.failing",
            interval + interval / 2,
            interval,
            passes
        )
        .await,
        expected_alerts(alerting, passes)
    );
    let alerts = logs.named("matches.reaper.failing");
    assert!(
        alerts[0].data["message"]
            .as_str()
            .is_some_and(|message| message.contains("the live matches cannot be read")),
        "{:?}",
        alerts[0]
    );
    assert_eq!(
        logs.named("matches.reaper_failed").len(),
        usize::try_from(passes).expect("a count"),
        "every failed pass keeps its warning"
    );

    // The first good pass says so, once.
    stop_failing(&app).await;
    sleep_settled(interval).await;
    let recovered = logs.named("matches.reaper.recovered");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].level, "info");
    sleep_settled(3 * interval).await;
    assert_eq!(logs.named("matches.reaper.recovered").len(), 1);

    // A new streak alerts again.
    fail_on(&app, "matches.live", "the live matches cannot be read").await;
    let passes = alerting + 2;
    assert_eq!(
        alerts_after_each_pass(&logs, "matches.reaper.failing", interval, interval, passes).await,
        expected_alerts(alerting, passes)
    );
    assert_eq!(logs.named("matches.reaper.failing").len(), 2);
    reaper.abort();
}

#[tokio::test(start_paused = true)]
async fn the_matchmaker_alerts_when_the_queue_cannot_be_read() {
    let logs = record_logs();
    let app = empty_test_app().await;
    let interval = ms(MATCHMAKER_SWEEP_INTERVAL_SECONDS);
    let alerting = alerting_pass(interval);
    fail_on(&app, "tickets.listOpen", "the queue cannot be read").await;
    let matchmaker = tokio::spawn(queue::run_matchmaker(app.clone()));

    let passes = alerting + 3;
    assert_eq!(
        alerts_after_each_pass(
            &logs,
            "matchmaker.failing",
            interval + interval / 2,
            interval,
            passes
        )
        .await,
        expected_alerts(alerting, passes)
    );
    assert_eq!(
        logs.named("matchmaker.sweep_failed").len(),
        usize::try_from(passes).expect("a count"),
        "every failed pass keeps its warning"
    );
    matchmaker.abort();
}

/// A live match of `players`, started past the ceiling and not answering, as a crashed actor leaves
/// it, with both players' profiles in it.
async fn stuck_match(app: &Arc<App>, id: &str, players: (&str, &str)) {
    for player in [players.0, players.1] {
        fake_lock(app)
            .await
            .seed_profile(json!({ "id": player, "inMatchId": id }));
    }
    let started_at = now_ms() - (MATCH_CEILING_MINUTES + 1) * MINUTE_MS;
    let row: MatchRow = from(json!({
        "id": id,
        "seed": "seed-1",
        "players": [players.0, players.1],
        "decks": [fake_deck(&[]), fake_deck(&[])],
        "catalogVersion": jackioh_cards::catalog_version(),
        "ranked": true,
        "status": "live",
        "createdAt": started_at,
        "finishedAt": null,
        "clocks": to_json(&initial_clocks(started_at)),
    }));
    let mut tx = app.db.begin(None).await.expect("a store transaction begins");
    tx.matches_create(&row).await.expect("the match row is written");
    tx.commit().await.expect("the store commits");
}

/// Makes the first result write after each read of the live matches fail, so the first match of
/// every reaper pass will not reap and the rest do.
async fn wedge_first_match(app: &Arc<App>) {
    let writes = Arc::new(AtomicUsize::new(0));
    fake_lock(app).await.on_call = Some(Arc::new(move |called: &str| {
        match called {
            "matches.live" => writes.store(0, Ordering::SeqCst),
            "results.insert" if writes.fetch_add(1, Ordering::SeqCst) == 0 => {
                return Err(StoreError::Other("the wedged match will not write".to_string()));
            }
            _ => {}
        }
        Ok(())
    }));
}

#[tokio::test(start_paused = true)]
async fn the_reaper_alerts_for_one_wedged_match_among_healthy_ones() {
    let logs = record_logs();
    install_test_cards();
    let app = empty_test_app().await;
    // The wedged match is the first the pass reaps: the fake lists live matches in the order made.
    stuck_match(&app, "match-wedged", ("profile-a", "profile-b")).await;
    stuck_match(&app, "match-healthy", ("profile-c", "profile-d")).await;
    wedge_first_match(&app).await;
    let interval = ms(MATCH_REAPER_INTERVAL_SECONDS);
    let alerting = alerting_pass(interval);
    let reaper = tokio::spawn(results::run_reaper(app.clone()));

    let passes = alerting + 3;
    assert_eq!(
        alerts_after_each_pass(
            &logs,
            "matches.reaper.failing",
            interval + interval / 2,
            interval,
            passes
        )
        .await,
        expected_alerts(alerting, passes)
    );
    let results = fake_read(&app, |data| to_json(&data.tables.results)).await;
    assert_eq!(results.as_array().map(Vec::len), Some(1), "{results}");
    assert_eq!(results[0]["matchId"], json!("match-healthy"));
    assert_eq!(
        logs.named("reaper.failed").len(),
        usize::try_from(passes).expect("a count"),
        "the wedged match fails on every pass"
    );
    assert!(logs.named("matches.reaper_failed").is_empty());
    reaper.abort();
}

#[tokio::test(start_paused = true)]
async fn the_hourly_purge_alerts_once_its_failed_passes_reach_the_threshold() {
    let logs = record_logs();
    let app = empty_test_app().await;
    let interval = ms(RETENTION_PURGE_INTERVAL_SECONDS);
    let alerting = alerting_pass(interval);
    fail_on(&app, "purgeExpired", "the purge cannot run").await;
    // The purge runs first, then waits: readings fall half an hour after each pass.
    let purge = tokio::spawn(retention::run_purge(app.clone()));

    let passes = alerting + 2;
    assert_eq!(
        alerts_after_each_pass(&logs, "retention.purge.failing", interval / 2, interval, passes).await,
        expected_alerts(alerting, passes)
    );
    assert_eq!(failing_alerts(&logs), ["retention.purge.failing"]);
    purge.abort();
}

/// One sweep loop: the alert its store failure raises, the store method to fail, when its first pass
/// falls after it starts, how far apart its passes are, and how it is started.
struct Sweep {
    event: &'static str,
    method: &'static str,
    first_pass_ms: i64,
    interval_ms: i64,
    spawn: fn(Arc<App>) -> JoinHandle<()>,
}

#[tokio::test(start_paused = true)]
async fn every_other_sweep_alerts_when_its_store_keeps_failing() {
    let logs = record_logs();
    let sweeps = [
        Sweep {
            event: "series.sweeper.failing",
            method: "series.active",
            first_pass_ms: ms(SERIES_SWEEP_INTERVAL_SECONDS),
            interval_ms: ms(SERIES_SWEEP_INTERVAL_SECONDS),
            spawn: |app| tokio::spawn(series::run_sweeper(app)),
        },
        Sweep {
            event: "duos.sweeper.failing",
            method: "duos.rooms.reap",
            first_pass_ms: ms(MATCH_REAPER_INTERVAL_SECONDS),
            interval_ms: ms(MATCH_REAPER_INTERVAL_SECONDS),
            spawn: |app| tokio::spawn(jackioh_server::duos::run_sweeper(app)),
        },
        Sweep {
            event: "challenges.sweeper.failing",
            method: "challenges.purge",
            first_pass_ms: 0,
            interval_ms: ms(CHALLENGE_SWEEP_INTERVAL_SECONDS),
            spawn: |app| tokio::spawn(jackioh_server::social::challenges::run_sweeper(app)),
        },
        Sweep {
            event: "seats.reconcile.failing",
            method: "profiles.seated",
            first_pass_ms: ms(MATCH_REAPER_INTERVAL_SECONDS),
            interval_ms: ms(MATCH_REAPER_INTERVAL_SECONDS),
            spawn: |app| tokio::spawn(results::run_reaper(app)),
        },
    ];
    for sweep in sweeps {
        let app = empty_test_app().await;
        let alerting = alerting_pass(sweep.interval_ms);
        fail_on(&app, sweep.method, "the store cannot answer").await;
        let handle = (sweep.spawn)(app.clone());
        let passes = alerting + 3;
        assert_eq!(
            alerts_after_each_pass(
                &logs,
                sweep.event,
                sweep.first_pass_ms + sweep.interval_ms / 2,
                sweep.interval_ms,
                passes
            )
            .await,
            expected_alerts(alerting, passes),
            "{}",
            sweep.event
        );
        handle.abort();
    }
}
