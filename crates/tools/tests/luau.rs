//! `golden record` and `luau diff` on the built `jackioh` (#559): the forced cards are in both decks of
//! every recorded game and the file replays clean; a build is its own parent without a mismatch; and
//! a doctored hash is named by seed, step and hash, with exit 1. The timings of `luau bench` are not
//! tested (its comparison is unit-tested in `src/luau.rs`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_jackioh");

/// Two Core cards, forced into every deck.
const FORCED: [&str; 2] = ["core-055", "core-024"];

/// A directory for one test, empty, under the temporary directory.
fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jackioh-luau-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn jackioh(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().expect("jackioh runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn record(seeds: &str, force: &str, out: &Path) {
    let output = jackioh(&[
        "golden",
        "record",
        "--seeds",
        seeds,
        "--force",
        force,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{}{}", stdout(&output), stderr(&output));
}

#[test]
fn golden_record_forces_the_cards_into_both_decks_of_every_game_and_replays_clean() {
    let dir = scratch("record");
    let file = dir.join("forced.jsonl");
    // One plain seed and two handicapped ones, whose decks differ in size.
    record("200..202", &FORCED.join(","), &file);

    let text = fs::read_to_string(&file).unwrap();
    let games: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(games.len(), 3);
    for game in &games {
        for seat in 0..2 {
            let deck: Vec<&str> = game["args"]["decks"][seat]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap())
                .collect();
            for id in FORCED {
                assert!(
                    deck.contains(&id),
                    "{} seat {seat}: no {id} in {deck:?}",
                    game["seed"]
                );
            }
            let mut sorted = deck.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(
                sorted.len(),
                deck.len(),
                "{} seat {seat}: a card twice",
                game["seed"]
            );
        }
    }

    let checked = jackioh(&["golden", "check", "--file", file.to_str().unwrap()]);
    assert!(
        checked.status.success(),
        "{}{}",
        stdout(&checked),
        stderr(&checked)
    );
    assert!(
        stdout(&checked).contains("3 of 3 games replayed identically"),
        "{}",
        stdout(&checked)
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn luau_diff_with_this_binary_as_its_own_parent_reports_0_mismatches() {
    let output = jackioh(&[
        "luau",
        "diff",
        "--parent-bin",
        BIN,
        "--cards",
        &FORCED.join(","),
        "--games",
        "3",
        "--seed",
        "1",
    ]);
    assert!(output.status.success(), "{}{}", stdout(&output), stderr(&output));
    assert!(stdout(&output).contains("0 mismatches"), "{}", stdout(&output));
}

#[test]
fn a_doctored_hash_makes_luau_diff_name_the_seed_the_step_and_the_hash_and_exit_1() {
    let dir = scratch("doctored");
    let file = dir.join("forced.jsonl");
    record("1..2", FORCED[0], &file);

    // Change the first game's first state hash; the second game stays as recorded.
    let text = fs::read_to_string(&file).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut first: Value = serde_json::from_str(&lines[0]).unwrap();
    let doctored = if first["steps"][0]["s"] == "00000000" {
        "ffffffff"
    } else {
        "00000000"
    };
    first["steps"][0]["s"] = Value::from(doctored);
    lines[0] = serde_json::to_string(&first).unwrap();
    fs::write(&file, lines.join("\n") + "\n").unwrap();

    let output = jackioh(&[
        "luau",
        "diff",
        "--parent-bin",
        BIN,
        "--traces",
        file.to_str().unwrap(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains(&format!("seed jackioh-fuzz-1 step 0: `s` expected {doctored}")),
        "{out}"
    );
    assert!(out.contains("1 diverged"), "{out}");
    assert!(stderr(&output).contains("1 mismatch"), "{}", stderr(&output));
    assert!(file.exists(), "a failed diff keeps its traces");
    fs::remove_dir_all(&dir).unwrap();
}
