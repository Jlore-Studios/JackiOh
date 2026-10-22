//! `golden check` on the built `jackioh` (#1641): a state hash doctored in a copy of `games.jsonl`'s first
//! game is named by seed, step, hash and action, in the words the engine's `tests/golden.rs` holds its own
//! replay of the same doctored game to (`a_doctored_trace`): both replay with
//! `jackioh_engine::testkit::golden`. The command exits 1.

use std::fs;
use std::process::Command;

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_jackioh");

/// The recorded games, as `golden check` reads them by default.
const GAMES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/golden/games.jsonl");

#[test]
fn a_doctored_state_hash_is_named_by_seed_step_hash_and_action_and_exits_1() {
    let dir = std::env::temp_dir().join(format!("jackioh-golden-doctored-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let games = fs::read_to_string(GAMES_PATH).unwrap();
    let mut game: Value = serde_json::from_str(games.lines().next().unwrap()).unwrap();
    let recorded = game["steps"][0]["s"].as_str().unwrap().to_string();
    let doctored = if recorded == "00000000" {
        "ffffffff"
    } else {
        "00000000"
    };
    game["steps"][0]["s"] = Value::from(doctored);
    let action = game["steps"][0]["a"].to_string();
    let file = dir.join("doctored.jsonl");
    fs::write(&file, serde_json::to_string(&game).unwrap() + "\n").unwrap();

    let output = Command::new(BIN)
        .args(["golden", "check", "--file", file.to_str().unwrap()])
        .output()
        .expect("jackioh runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let named = format!(
        "seed jackioh-fuzz-1 step 0: `s` expected {doctored}, Rust has {recorded}\n    action: {action}\n"
    );
    assert!(stdout.contains(&named), "{stdout}");
    assert!(stdout.contains("0 of 1 games replayed identically"), "{stdout}");
    assert!(stdout.contains("1 diverged"), "{stdout}");

    let _ = fs::remove_dir_all(&dir);
}
