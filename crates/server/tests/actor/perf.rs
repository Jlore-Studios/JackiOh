//! The match actor's speed (#3429): matches played at once through the actor on the in-memory store,
//! both seats on the random policy (§10.7), on a runtime of `SERVER_PERF_WORKER_THREADS` workers.
//!
//! The latency is `submit`'s round trip: the actor answers an action only after it has carried out
//! every output of the change, both seats' views among them (`jackioh_shared::host::duel`'s
//! `after_change`), so the answer marks the broadcast. The daily test holds its median to
//! `SERVER_PERF_BROADCAST_MS_BUDGET` and the matches finished per second to
//! `SERVER_PERF_MATCHES_PER_SECOND_FLOOR`; the smoke test plays `SERVER_PERF_SMOKE_MATCHES` matches
//! per pull request with no budget, so the harness keeps working.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Instant;

use serde_json::json;

use jackioh_engine::rng::Rng;
use jackioh_engine::subsystems::choose_action;
use jackioh_engine::{PlayerId, seat_to_act};
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::actor::match_actor::MatchActor;
use jackioh_server::config::{
    MS_PER_SECOND, RATING_START, SERVER_PERF_ACTION_CAP, SERVER_PERF_BROADCAST_MS_BUDGET,
    SERVER_PERF_MATCHES, SERVER_PERF_MATCHES_PER_SECOND_FLOOR, SERVER_PERF_SMOKE_MATCHES,
    SERVER_PERF_WORKER_THREADS,
};

use crate::support::deps::test_app;
use crate::support::socket::{FakeSocket, create_fake_socket};
use crate::support::store::{fake, from, to_json};

const MEDIAN: f64 = 0.5;
const P95: f64 = 0.95;

/// What a run of matches measured.
struct Measured {
    /// Each action's `submit` round trip, in milliseconds.
    latencies_ms: Vec<f64>,
    matches: usize,
    /// From the first action of any match to the last match's end.
    wall_seconds: f64,
}

/// Plays one match to its end, each seat to act taking the random policy's action from the match's
/// own stream: each action's round trip, in milliseconds.
async fn drive_match(seed: String, actor: MatchActor, one: FakeSocket, two: FakeSocket) -> Vec<f64> {
    let mut policy = Rng::new(&format!("{seed}:policy"), 0);
    let mut latencies = Vec::new();
    let mut frames = (0, 0);
    loop {
        let state = actor.engine_state();
        if state.result.is_some() {
            break;
        }
        let n = latencies.len();
        assert!(
            n < SERVER_PERF_ACTION_CAP,
            "{seed}: no end after {SERVER_PERF_ACTION_CAP} actions"
        );
        let seat: PlayerId = seat_to_act(&state).unwrap_or(state.active);
        let body = choose_action(&state, seat, &mut policy)
            .unwrap_or_else(|| panic!("{seed} step {n}: no legal action for {seat} while the game is live"));
        let started = Instant::now();
        let reply = actor.submit(seat, format!("perf-{n}"), body).await;
        latencies.push(started.elapsed().as_secs_f64() * MS_PER_SECOND as f64);
        let reply = to_json(&reply);
        assert_eq!(reply["type"], "ack", "{seed} step {n} by {seat}: {reply}");
        frames.0 += one.sent().len();
        frames.1 += two.sent().len();
        one.clear();
        two.clear();
    }
    actor.idle().await;
    let actions = latencies.len();
    assert!(
        frames.0 >= actions && frames.1 >= actions,
        "{seed}: {actions} actions pushed {} frames to p1 and {} to p2",
        frames.0,
        frames.1
    );
    latencies
}

/// Starts `count` matches on the test app, both accounts attached, then plays them all at once.
async fn play_matches(count: usize) -> Measured {
    jackioh_cards::register_all();
    let app = test_app().await;
    let mut started = Vec::new();
    for k in 0..count {
        let (p1, p2) = (format!("perf-p1-{k}"), format!("perf-p2-{k}"));
        {
            let data = fake(&app);
            let mut data = data.lock().await;
            data.seed_profile(json!({ "id": p1, "rating": RATING_START }));
            data.seed_profile(json!({ "id": p2, "rating": RATING_START }));
        }
        let (match_id, seed) = (format!("perf-match-{k}"), format!("perf-server-{k}"));
        app.matches
            .start(
                &app,
                from(json!({
                    "matchId": match_id,
                    "seed": seed,
                    "catalogVersion": jackioh_cards::catalog_version(),
                    "ranked": false,
                    "seats": [
                        { "profileId": p1, "player": "p1", "deck": deal_random_deck(&format!("{seed}:p1"), None) },
                        { "profileId": p2, "player": "p2", "deck": deal_random_deck(&format!("{seed}:p2"), None) },
                    ],
                })),
            )
            .await
            .unwrap_or_else(|error| panic!("{match_id} does not start: {error:?}"));
        let (one, two) = (create_fake_socket(), create_fake_socket());
        app.matches
            .attach(&app, &match_id, &p1, one.socket())
            .await
            .unwrap_or_else(|error| panic!("p1 does not attach to {match_id}: {error:?}"));
        app.matches
            .attach(&app, &match_id, &p2, two.socket())
            .await
            .unwrap_or_else(|error| panic!("p2 does not attach to {match_id}: {error:?}"));
        let actor = app
            .matches
            .actor_for(&app, &match_id)
            .await
            .unwrap_or_else(|error| panic!("{match_id} has no actor: {error:?}"));
        started.push((seed, actor, one, two));
    }

    let clock = Instant::now();
    let handles: Vec<_> = started
        .into_iter()
        .map(|(seed, actor, one, two)| tokio::spawn(drive_match(seed, actor, one, two)))
        .collect();
    let mut latencies_ms = Vec::new();
    for handle in handles {
        latencies_ms.extend(handle.await.expect("a match's driver panicked"));
    }
    Measured {
        latencies_ms,
        matches: count,
        wall_seconds: clock.elapsed().as_secs_f64(),
    }
}

/// `play_matches` on a runtime of `SERVER_PERF_WORKER_THREADS` workers.
fn play_on_workers(count: usize) -> Measured {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(SERVER_PERF_WORKER_THREADS)
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(play_matches(count))
}

/// The value at nearest rank `p` (0 < p <= 1) of a sorted, non-empty list.
fn nearest_rank(sorted: &[f64], p: f64) -> f64 {
    let len = sorted.len();
    sorted[((p * len as f64).ceil() as usize).clamp(1, len) - 1]
}

/// What a row is held to: a ceiling or a floor, with its constant's name.
enum Bound {
    AtMost(f64, &'static str),
    AtLeast(f64, &'static str),
}

impl Bound {
    fn value(&self) -> f64 {
        match self {
            Bound::AtMost(value, _) | Bound::AtLeast(value, _) => *value,
        }
    }
}

struct ServerRow {
    measure: &'static str,
    unit: &'static str,
    median: f64,
    p95: f64,
    bound: Bound,
}

fn server_rows(measured: &Measured) -> Vec<ServerRow> {
    let mut latencies = measured.latencies_ms.clone();
    latencies.sort_by(f64::total_cmp);
    rows_of(
        nearest_rank(&latencies, MEDIAN),
        nearest_rank(&latencies, P95),
        measured.matches as f64 / measured.wall_seconds,
    )
}

/// The two rows: the latency's median and p95, in milliseconds, and the matches finished per second,
/// one figure for both columns.
fn rows_of(median_ms: f64, p95_ms: f64, matches_per_second: f64) -> Vec<ServerRow> {
    vec![
        ServerRow {
            measure: "action to broadcast",
            unit: "ms",
            median: median_ms,
            p95: p95_ms,
            bound: Bound::AtMost(SERVER_PERF_BROADCAST_MS_BUDGET, "SERVER_PERF_BROADCAST_MS_BUDGET"),
        },
        ServerRow {
            measure: "matches per second",
            unit: "1/s",
            median: matches_per_second,
            p95: matches_per_second,
            bound: Bound::AtLeast(
                SERVER_PERF_MATCHES_PER_SECOND_FLOOR,
                "SERVER_PERF_MATCHES_PER_SECOND_FLOOR",
            ),
        },
    ]
}

/// One message per row past its bound, naming the measure, the median, the p95 and the bound.
fn judge_server(rows: &[ServerRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| {
            let ServerRow {
                measure,
                unit,
                median,
                p95,
                bound,
            } = row;
            let (side, value, name) = match *bound {
                Bound::AtMost(value, name) if *median > value => ("over its budget", value, name),
                Bound::AtLeast(value, name) if *median < value => ("under its floor", value, name),
                _ => return None,
            };
            Some(format!(
                "{measure}: median {median:.2} {unit}, p95 {p95:.2} {unit}, {side} of {value:.2} {unit} ({name})"
            ))
        })
        .collect()
}

/// The rows as the tools' `perf` table prints its own (measure, unit, median, p95 and bound), at two
/// decimals: a run finishes about one match a second.
fn server_table(measured: &Measured, rows: &[ServerRow]) -> String {
    let mut out = format!(
        "perf (server): {} matches at once on {SERVER_PERF_WORKER_THREADS} workers, {} actions in {:.2} s\n",
        measured.matches,
        measured.latencies_ms.len(),
        measured.wall_seconds
    );
    let _ = writeln!(
        out,
        "{:<20} {:<5} {:>12} {:>12} {:>12}",
        "measure", "unit", "median", "p95", "budget"
    );
    for row in rows {
        let _ = writeln!(
            out,
            "{:<20} {:<5} {:>12.2} {:>12.2} {:>12.2}",
            row.measure,
            row.unit,
            row.median,
            row.p95,
            row.bound.value()
        );
    }
    out
}

#[test]
fn perf_server_smoke_plays_matches_at_once() {
    let measured = play_on_workers(SERVER_PERF_SMOKE_MATCHES);
    assert_eq!(measured.matches, SERVER_PERF_SMOKE_MATCHES);
    assert!(!measured.latencies_ms.is_empty());
    let rows = server_rows(&measured);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.p95 >= row.median && row.median > 0.0));
}

#[test]
fn perf_server_a_slow_broadcast_and_a_slow_server_each_fail() {
    let slow = 2.0 * SERVER_PERF_BROADCAST_MS_BUDGET;
    let failures = judge_server(&rows_of(slow, slow, SERVER_PERF_MATCHES_PER_SECOND_FLOOR / 2.0));
    assert_eq!(failures.len(), 2, "{failures:#?}");
    for (message, needed) in failures.iter().zip([
        [
            "action to broadcast:",
            "median",
            "p95",
            "over its budget",
            "SERVER_PERF_BROADCAST_MS_BUDGET",
        ],
        [
            "matches per second:",
            "median",
            "p95",
            "under its floor",
            "SERVER_PERF_MATCHES_PER_SECOND_FLOOR",
        ],
    ]) {
        for word in needed {
            assert!(message.contains(word), "{message:?} lacks {word:?}");
        }
    }

    let at_the_bounds = rows_of(
        SERVER_PERF_BROADCAST_MS_BUDGET,
        SERVER_PERF_BROADCAST_MS_BUDGET,
        SERVER_PERF_MATCHES_PER_SECOND_FLOOR,
    );
    assert_eq!(judge_server(&at_the_bounds), Vec::<String>::new());
}

#[test]
#[ignore = "daily: SERVER_PERF_MATCHES matches at once held to their budgets; perf_server_smoke_plays_matches_at_once keeps the harness working per pull request"]
fn perf_server_holds_its_budgets() {
    let measured = play_on_workers(SERVER_PERF_MATCHES);
    let rows = server_rows(&measured);
    let table = server_table(&measured, &rows);
    // libtest captures `print!` and `eprint!` alone, so the table reaches the daily log this way.
    let _ = writeln!(std::io::stderr().lock(), "{table}");
    let failures = judge_server(&rows);
    assert!(failures.is_empty(), "{table}{}", failures.join("\n"));
}
