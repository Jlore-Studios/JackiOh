//! `perf` on the built `jackioh` (#3429): once per pull request, so the harness keeps working and
//! the work budget, which needs no clock, is held (#4196), and daily with every median held to
//! the test profile's budgets.

use std::io::Write as _;
use std::process::{Command, Output};

use serde_json::Value;

use jackioh_engine::testkit::golden::HOTSEAT;

const BIN: &str = env!("CARGO_BIN_EXE_jackioh");

/// The report's measures, in its order.
const MEASURES: [&str; 12] = [
    "reduce",
    "view_for",
    "combat_forecast",
    "legal_actions",
    "hash_state",
    "fold",
    "duos_reduce",
    "duos_view_for",
    "duos_combat_forecast",
    "duos_legal_actions",
    "duos_fold",
    "view_for bytes",
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
    const MIN_FORECAST_ATTACKS: u64 = 10;

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
    assert!(corpus["plain"].as_u64() >= Some(1), "{report:#}");
    assert!(corpus["handicapped"].as_u64() >= Some(1), "{report:#}");
    assert!(corpus["runStart"].as_u64() >= Some(1), "{report:#}");
    assert!(corpus["coop"].as_u64() >= Some(1), "{report:#}");
    assert!(corpus["forecastAttacks"].as_u64().is_some(), "{report:#}");
    assert!(corpus["duosForecastAttacks"].as_u64().is_some(), "{report:#}");
    assert!(
        corpus["maxForecastAttacks"].as_u64() >= Some(MIN_FORECAST_ATTACKS),
        "{report:#}"
    );
    assert!(report["fold"]["actions"].as_u64() >= Some(1), "{report:#}");
    assert!(report["duosFold"]["actions"].as_u64() >= Some(1), "{report:#}");

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

    // The work budget, the one `--smoke` judges (#4196), read by its row's name.
    let row = rows
        .iter()
        .find(|row| row["measure"] == "view_for bytes")
        .unwrap_or_else(|| panic!("no view_for bytes row: {report:#}"));
    let number = |key: &str| row[key].as_f64().unwrap_or(f64::NAN);
    assert!(
        number("median") <= number("budget"),
        "view_for bytes is over its budget: {row}"
    );

    // The fold is timed per folded action, so a longer game does not use up its time budget.
    let row = rows
        .iter()
        .find(|row| row["measure"] == "fold")
        .unwrap_or_else(|| panic!("no fold row: {report:#}"));
    assert_eq!(row["unit"], "us/action", "{row}");

    let row = rows
        .iter()
        .find(|row| row["measure"] == "duos_fold")
        .unwrap_or_else(|| panic!("no duos_fold row: {report:#}"));
    assert_eq!(row["unit"], "us/action", "{row}");
}

#[test]
#[ignore = "daily: ten timed passes over the corpus held to the test profile's budgets; perf_smoke_times_the_corpus_once holds the work budget on every pull request"]
fn perf_holds_every_budget() {
    let output = jackioh(&["perf"]);
    // libtest captures `print!` and `eprint!` alone, so the table reaches the daily log this way.
    let _ = writeln!(std::io::stderr().lock(), "{}", stdout(&output));
    assert!(output.status.success(), "{}{}", stdout(&output), stderr(&output));
}
