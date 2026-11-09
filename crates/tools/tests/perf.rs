//! `perf` on the built `jackioh` (#3429): once per pull request with no budget judged, so the harness
//! keeps working, and daily with every median held to the test profile's budgets.

use std::io::Write as _;
use std::process::{Command, Output};

use serde_json::Value;

use jackioh_engine::testkit::golden::HOTSEAT;

const BIN: &str = env!("CARGO_BIN_EXE_jackioh");

/// The report's measures, in its order.
const MEASURES: [&str; 7] = [
    "reduce",
    "view_for",
    "legal_actions",
    "hash_state",
    "fold",
    "view_for bytes",
    "fold reduce calls",
];

fn jackioh(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().expect("jackioh runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn perf_smoke_times_the_corpus_once() {
    let output = jackioh(&["perf", "--smoke", "--json"]);
    assert!(output.status.success(), "{}{}", stdout(&output), stderr(&output));
    let report: Value = serde_json::from_str(&stdout(&output)).expect("--json prints one JSON object");
    assert_eq!(report["smoke"], true, "{report:#}");
    assert_eq!(report["passes"], 1, "{report:#}");
    assert_eq!(report["failures"], Value::Array(Vec::new()), "{report:#}");

    let hotseat: Value = serde_json::from_str(HOTSEAT).expect("the hotseat fixture is JSON");
    let log = hotseat["log"].as_array().expect("the fixture's log").len();
    let corpus = &report["corpus"];
    assert_eq!(corpus["hotseat"].as_u64(), Some(log as u64 + 1), "{report:#}");
    assert_eq!(corpus["wide"], 2, "{report:#}");
    assert!(corpus["heaviest"].as_u64() >= Some(1), "{report:#}");
    assert!(report["fold"]["actions"].as_u64() >= Some(1), "{report:#}");

    let rows = report["measures"].as_array().expect("the measures");
    let names: Vec<&str> = rows.iter().filter_map(|row| row["measure"].as_str()).collect();
    assert_eq!(names, MEASURES, "{report:#}");
    for row in rows {
        let number = |key: &str| row[key].as_f64().unwrap_or(f64::NAN);
        assert!(row["samples"].as_u64() >= Some(1), "{row}");
        assert!(number("p95") >= number("median"), "{row}");
        assert!(number("median") >= 0.0, "{row}");
        assert!(number("budget") > 0.0, "{row}");
    }
}

#[test]
#[ignore = "daily: ten timed passes over the corpus held to the test profile's budgets; perf_smoke_times_the_corpus_once keeps the harness working per pull request"]
fn perf_holds_every_budget() {
    let output = jackioh(&["perf"]);
    // libtest captures `print!` and `eprint!` alone, so the table reaches the daily log this way.
    let _ = writeln!(std::io::stderr().lock(), "{}", stdout(&output));
    assert!(output.status.success(), "{}{}", stdout(&output), stderr(&output));
}
