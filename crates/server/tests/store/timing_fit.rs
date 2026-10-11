//! `cli/timing_fit.rs` (timing-fit) and the argument check `cli/timing_backfill.rs` shares with it,
//! SPEC §9.11, R1442: the human think times of each bucket fitted with a log-normal, the AI's moves
//! left out, a bucket under the minimum dropped, and the rows printed to paste into config. The
//! store's read is `tests/store/contract.rs`'s; the backfill's fold is `tests/actor/telemetry.rs`'s.

use serde_json::{Value, json};

use jackioh_engine::ActionType;
use jackioh_server::cli::timing_fit::{BucketFit, fit_think_times, render_fits};
use jackioh_server::cli::{timing_backfill, timing_fit};
use jackioh_server::db::store::ActionTimingRow;
use jackioh_server::ranked::ladder::RankTier;

/// How close two fitted numbers must be: well under the printed four decimals.
const EPSILON: f64 = 1e-9;

fn timing(seq: i64, kind: &str, first: bool, rank: Value, think_ms: i64, pilot: &str) -> ActionTimingRow {
    serde_json::from_value(json!({
        "matchId": "m-fit",
        "seat": "p1",
        "seq": seq,
        "actionKind": kind,
        "legalCount": 3,
        "turn": 2,
        "thinkMs": think_ms,
        "firstInTurn": first,
        "rankBucket": rank,
        "pilot": pilot,
    }))
    .expect("an action timing")
}

/// Three human plays in Golden Grape (100 ms, 1 s, 10 s: a geometric mean of 1 s), two human turn
/// ends with no rank (0 ms, which counts as 1 ms, and 1 ms), and AI moves in the first bucket and in
/// a bucket of their own, which would move the first fit and add a bucket if they were counted.
fn rows() -> Vec<ActionTimingRow> {
    vec![
        timing(1, "play", true, json!("golden"), 100, "human"),
        timing(2, "play", true, json!("golden"), 1_000, "human"),
        timing(3, "play", true, json!("golden"), 10_000, "human"),
        timing(4, "play", true, json!("golden"), 900_000, "ai"),
        timing(5, "endTurn", false, Value::Null, 0, "human"),
        timing(6, "endTurn", false, Value::Null, 1, "human"),
        timing(7, "attack", false, json!("mythic"), 5_000, "ai"),
        timing(8, "attack", false, json!("mythic"), 6_000, "ai"),
        timing(9, "attack", false, json!("mythic"), 7_000, "ai"),
    ]
}

#[track_caller]
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < EPSILON, "{actual} is not {expected}");
}

#[test]
fn r1442_timing_fit_fits_each_bucket_and_leaves_out_ai_rows() {
    let fits = fit_think_times(&rows(), 2);
    let keys: Vec<(Option<RankTier>, ActionType, bool, usize)> = fits
        .iter()
        .map(|fit| (fit.rank_bucket, fit.action_kind, fit.first_in_turn, fit.samples))
        .collect();
    // No rank sorts before a tier, and the AI-only Mythic bucket is not there at all.
    assert_eq!(
        keys,
        vec![
            (None, ActionType::EndTurn, false, 2),
            (Some(RankTier::Golden), ActionType::Play, true, 3),
        ]
    );
    // ln(1) twice.
    close(fits[0].mu, 0.0);
    close(fits[0].sigma, 0.0);
    // ln(100), ln(1000), ln(10000): the mean is ln(1000), each end ln(10) from it.
    close(fits[1].mu, 1_000f64.ln());
    close(fits[1].sigma, 10f64.ln() * (2.0f64 / 3.0).sqrt());
}

#[test]
fn r1442_timing_fit_drops_a_bucket_under_the_minimum() {
    let fits = fit_think_times(&rows(), 3);
    assert_eq!(fits.len(), 1);
    assert_eq!(fits[0].rank_bucket, Some(RankTier::Golden));
    assert_eq!(fits[0].samples, 3);
    assert!(fit_think_times(&rows(), 4).is_empty());
}

#[test]
fn r1442_timing_fit_prints_rows_to_paste_into_config() {
    let fits = [
        BucketFit {
            rank_bucket: None,
            action_kind: ActionType::EndTurn,
            first_in_turn: false,
            samples: 40,
            mu: 7.5,
            sigma: 0.25,
        },
        BucketFit {
            rank_bucket: Some(RankTier::Golden),
            action_kind: ActionType::Play,
            first_in_turn: true,
            samples: 31,
            mu: 8.123_41,
            sigma: 0.734_49,
        },
    ];
    assert_eq!(
        render_fits(&fits),
        "// jackioh-server timing-fit: paste into THINK_TIME_FITS, crates/server/src/config.rs\n\
         \x20   ThinkTimeFit { rank_bucket: None, action_kind: \"endTurn\", first_in_turn: false, samples: 40, mu: 7.5000, sigma: 0.2500 },\n\
         \x20   ThinkTimeFit { rank_bucket: Some(\"golden\"), action_kind: \"play\", first_in_turn: true, samples: 31, mu: 8.1234, sigma: 0.7345 },\n"
    );
    assert_eq!(
        render_fits(&[]),
        "// jackioh-server timing-fit: paste into THINK_TIME_FITS, crates/server/src/config.rs\n"
    );
}

#[tokio::test]
async fn timing_commands_refuse_an_unknown_option() {
    for refusal in [
        timing_fit::run(vec!["--min=3".to_string()]).await,
        timing_backfill::run(vec!["--all".to_string()]).await,
    ] {
        let message = refusal.expect_err("an option neither command takes").to_string();
        assert!(message.contains("Unrecognised argument \"--"), "{message}");
        assert!(message.contains("Usage: jackioh-server timing-"), "{message}");
    }
}
