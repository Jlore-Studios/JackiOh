//! `golden record` then `golden check` on the built `jackioh` (#1641, #2578): five recorded games
//! replay clean, and a state hash doctored in the first is named by seed, step, hash and action, with
//! exit 1; `--force` puts the cards named in both decks of every game recorded (#559); `golden check`
//! with no file folds the hotseat fixture alone; and `golden bless`, retired with the 240 committed
//! games, fails pointing to `golden record`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

use jackioh_engine::testkit::golden::HOTSEAT_HASH;

const BIN: &str = env!("CARGO_BIN_EXE_jackioh");

/// Two Core cards, forced into every deck.
const FORCED: [&str; 2] = ["core-055", "core-024"];

/// A directory for one test, empty, under the temporary directory.
fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jackioh-golden-{test}-{}", std::process::id()));
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

#[test]
fn golden_record_then_check_file_replays_clean_and_a_doctored_state_hash_is_named_and_exits_1() {
    let dir = scratch("doctored");
    let file = dir.join("recorded.jsonl");
    let path = file.to_str().unwrap();

    let recorded = jackioh(&["golden", "record", "--seeds", "1..5", "--out", path]);
    assert!(
        recorded.status.success(),
        "{}{}",
        stdout(&recorded),
        stderr(&recorded)
    );
    let clean = jackioh(&["golden", "check", "--file", path]);
    assert!(clean.status.success(), "{}{}", stdout(&clean), stderr(&clean));
    assert!(
        stdout(&clean).contains("5 of 5 games replayed identically"),
        "{}",
        stdout(&clean)
    );

    let text = fs::read_to_string(&file).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    assert_eq!(lines.len(), 5, "{text}");
    let mut game: Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(game["seed"].as_str(), Some("jackioh-fuzz-1"));
    let hash = game["steps"][0]["s"].as_str().unwrap().to_string();
    let doctored = if hash == "00000000" {
        "ffffffff"
    } else {
        "00000000"
    };
    game["steps"][0]["s"] = Value::from(doctored);
    let action = game["steps"][0]["a"].to_string();
    lines[0] = serde_json::to_string(&game).unwrap();
    fs::write(&file, lines.join("\n") + "\n").unwrap();

    let output = jackioh(&["golden", "check", "--file", path]);
    let out = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{out}{}", stderr(&output));
    let named = format!(
        "seed jackioh-fuzz-1 step 0: `s` expected {doctored}, Rust has {hash}\n    action: {action}\n"
    );
    assert!(out.contains(&named), "{out}");
    assert!(out.contains("jackioh-fuzz-1-0-s.json"), "{out}");
    assert!(out.contains("4 of 5 games replayed identically"), "{out}");
    assert!(out.contains("1 diverged"), "{out}");

    let _ = fs::remove_dir_all(&dir);
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
fn golden_check_with_no_file_folds_the_hotseat_fixture_alone_and_exits_0() {
    let output = jackioh(&["golden", "check"]);
    let out = stdout(&output);
    assert!(output.status.success(), "{out}{}", stderr(&output));
    assert!(
        out.contains(&format!("hotseat fixture: folds to {HOTSEAT_HASH}")),
        "{out}"
    );
    assert!(!out.contains("games replayed"), "{out}");
}

#[test]
fn golden_bless_is_retired_and_points_to_golden_record() {
    let output = jackioh(&["golden", "bless", "--seeds", "200"]);
    let err = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{}{err}", stdout(&output));
    assert!(err.contains("golden record"), "{err}");
    assert!(err.contains("#2578"), "{err}");
}
