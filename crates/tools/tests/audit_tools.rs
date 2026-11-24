//! Audit of the project's own gates (`crates/tools`): each test asserts what a README or a ruling
//! promises and fails on the current tree. They drive the built `jackioh`, as `acceptance.rs` does.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use jackioh_engine::{Action, CreateGameArgs, GameEvent, begin_game, create_game, every_instance, reduce};

const BIN: &str = env!("CARGO_BIN_EXE_jackioh");

/// A fresh directory outside the repository (`JACKIOH_AUDIT_TMP` moves it off the system temp dir).
fn scratch(name: &str) -> PathBuf {
    let base = std::env::var_os("JACKIOH_AUDIT_TMP").map_or_else(std::env::temp_dir, PathBuf::from);
    let dir = base.join(format!("jackioh-audit-tools-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
    fs::write(path, text).expect("a fixture file");
}

/// `jackioh spec --root <root> check`: whether it passed, and what it printed.
fn spec_check(root: &Path) -> (bool, String) {
    let output = Command::new(BIN)
        .args(["spec", "--root"])
        .arg(root)
        .arg("check")
        .output()
        .expect("jackioh runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

/// A spec tree of one ruling, R1, whose only proof is the file `proof_path` holding `proof`.
fn spec_fixture(name: &str, proof_path: &str, proof: &str) -> PathBuf {
    let root = scratch(name);
    write(
        &root,
        "spec/rulings/R0001.md",
        &format!(
            "---\nid: R1\ntitle: A fixture ruling\ncards: \"issue #1\"\nproven_in:\n  - {proof_path}\n---\nThe ruling.\n"
        ),
    );
    write(&root, "crates/cards/catalog.json", "{}\n");
    write(&root, proof_path, proof);
    root
}

/// Asserts `spec check` passes the control and refuses each proof that does not run.
fn refuses_each(proof_path: &str, control: &str, dead: &[(&str, &str)]) {
    let live = spec_fixture("control", proof_path, control);
    let (ok, text) = spec_check(&live);
    assert!(ok, "the control, a test that runs, passes: {text}");
    let _ = fs::remove_dir_all(&live);
    let mut passed = Vec::new();
    for (name, proof) in dead {
        let root = spec_fixture(name, proof_path, proof);
        let (ok, text) = spec_check(&root);
        if ok {
            passed.push(format!("{name}:\n{proof}-> {}", text.trim()));
        }
        let _ = fs::remove_dir_all(&root);
    }
    assert!(
        passed.is_empty(),
        "R1's only proof is no test that runs, yet `spec check` passed:\n{}",
        passed.join("\n")
    );
}

/// spec/README.md, Note format: "`proven_in` lists the files whose tests prove it, and only a test
/// that runs counts (#1718)"; a TypeScript test proves a ruling "as long as no `.skip`, `.todo`,
/// `.fixme` or `.only` turns it off". A test that is commented out, or a title that is only text in
/// a string, runs nothing, so `spec check` must refuse it as it refuses a Rust
/// `// fn r999_commented() {}` (`rust_code_only`).
#[test]
fn spec_check_refuses_a_typescript_proof_that_is_commented_out() {
    refuses_each(
        "apps/web/src/r1.test.ts",
        "import { expect, it } from \"vitest\";\nit(\"R1 the fixture ruling holds\", () => { expect(1).toBe(1); });\n",
        &[
            (
                "line-comment",
                "import { expect, it } from \"vitest\";\n// it(\"R1 the fixture ruling holds\", () => { expect(1).toBe(1); });\n",
            ),
            (
                "block-comment",
                "import { expect, it } from \"vitest\";\n/*\nit(\"R1 the fixture ruling holds\", () => { expect(1).toBe(1); });\n*/\n",
            ),
            (
                "string",
                "export const todo = 'write it(\"R1 the fixture ruling holds\") one day';\n",
            ),
        ],
    );
}

/// The same for an SQL evidence script (spec/README.md: "An SQL evidence script proves it with a
/// `\echo '### R195: … ###'` heading"): a heading on a `--` comment line announces no check.
#[test]
fn spec_check_refuses_an_sql_heading_that_is_commented_out() {
    refuses_each(
        "crates/server/tests/sql/checks.sql",
        "\\echo '### R1: the fixture ruling ###'\nselect 1;\n",
        &[("sql-comment", "-- \\echo '### R1: the fixture ruling ###'\n-- select 1;\n")],
    );
}

/// The games `classify`'s stage 2 records to confirm a script-only claim on `id`
/// (`classify.rs`, `forced_games_diverge`: `record_seeds(1..=FORCED_GAMES, [id], [])`, which is
/// `golden record --seeds 1..40 --force id`), folded again here: how many there are, in how many the
/// card resolved, in how many it resolved on its Radiant face, and in how many any instance of it
/// was Radiant in any zone.
fn forced_games(id: &str) -> (usize, usize, usize, usize) {
    let dir = scratch(&format!("forced-{id}"));
    let out = dir.join("forced.jsonl");
    let output = Command::new(BIN)
        .args(["golden", "record", "--seeds", "1..40", "--force", id, "--out"])
        .arg(&out)
        .output()
        .expect("jackioh runs");
    assert!(
        output.status.success(),
        "golden record failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    jackioh_cards::register_all();
    let text = fs::read_to_string(&out).expect("the recorded games");
    let (mut games, mut resolved, mut resolved_radiant, mut held_radiant) = (0, 0, 0, 0);
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let game: serde_json::Value = serde_json::from_str(line).expect("a recorded game");
        let args: CreateGameArgs = serde_json::from_value(game["args"].clone()).expect("its options");
        let begun = begin_game(&create_game(&args));
        assert!(begun.error.is_none(), "the game begins");
        let mut state = begun.state;
        let mut events: Vec<GameEvent> = begun.events;
        let (mut any, mut any_radiant, mut any_held) = (false, false, false);
        let mut steps = game["steps"].as_array().expect("its steps").iter();
        loop {
            for event in &events {
                if let GameEvent::CardResolved { def_id, radiant, .. } = event
                    && def_id == id
                {
                    any = true;
                    any_radiant |= *radiant == Some(true);
                }
            }
            any_held |= every_instance(&state).any(|card| card.def_id == id && card.radiant);
            let Some(step) = steps.next() else { break };
            let action: Action = serde_json::from_value(step["a"].clone()).expect("a recorded action");
            let result = reduce(&state, &action);
            assert!(result.error.is_none(), "the recorded action replays");
            state = result.state;
            events = result.events;
        }
        games += 1;
        resolved += usize::from(any);
        resolved_radiant += usize::from(any_radiant);
        held_radiant += usize::from(any_held);
    }
    let _ = fs::remove_dir_all(&dir);
    (games, resolved, resolved_radiant, held_radiant)
}

/// R1365000: "A card plays differently when … its script's code changes", and a script-only claim is
/// kept "only when, in `FORCED_GAMES` (40) games with it forced into both decks, a divergence on the
/// base build touches the card"; R1370000 then refuses a `plays` claim those games do not show.
/// Every card has a Radiant face with its own half of the script (§5.2): Core #2 Speciesist's
/// Radiant Cry is `destroy_all`, its base Cry `destroy`. So the games that confirm a change to
/// Speciesist's script must play its Radiant Cry at least once, or a change to that Cry can never
/// diverge. The fuzz gate met the same gap and pins the changed cards Radiant (#4161,
/// `play_radiant_rotation`: "the plain wave's copies are Radiant only by an in-game upgrade").
#[test]
fn classify_forced_games_play_the_forced_cards_radiant_face() {
    let (games, resolved, resolved_radiant, held_radiant) = forced_games("core-002");
    println!(
        "core-002: {games} forced games, resolved in {resolved}, resolved Radiant in {resolved_radiant}, a Radiant copy held in {held_radiant}"
    );
    assert_eq!(games, 40);
    assert!(resolved > 0, "the control: the forced games play the card");
    assert!(
        resolved_radiant > 0,
        "none of classify's {games} forced games of core-002 resolves it Radiant (it resolves on its base face in {resolved}, and a Radiant copy exists in {held_radiant}): a change to its Radiant Cry cannot diverge, so `classify --claims` reads it as playing as before"
    );
}
