//! The pool probe (#3117, R380) over the cards' tests: every random pool rotated one place, as one more
//! card in it would move every roll, so a test that pins where a roll lands fails here before the pull
//! request that next adds a card to its pool fails it. super.yml's daily tests job runs it (#4181);
//! `sh scripts/pool-probe.sh` runs the cards', AI's, server's and tools' tests that way by hand.

use std::io::Write as _;
use std::path::Path;
use std::process::Command;

/// The probe: the cards' tests at `cards lib`'s opt-levels (.github/ci/rust-test.toml), every one run
/// whatever fails, offline (the outer run fetched every crate), uncoloured for `failed` to read.
const PROBE: &[&str] = &[
    "test",
    "-p",
    "jackioh-cards",
    "--features",
    "jackioh-engine/pool-probe",
    "--no-fail-fast",
    "--offline",
    "--color",
    "never",
    "--config",
    "profile.test.package.jackioh-engine.opt-level=1",
    "--config",
    "profile.test.package.jackioh-cards.opt-level=0",
];
/// Its own target directory, so it never waits on the outer run's build lock (git ignores `target/`).
const TARGET_DIR: &str = "target/pool-probe";
/// How much of a failed build's stderr the report keeps: its end, where cargo names the error.
const STDERR_TAIL_LINES: usize = 80;

/// The tests a `cargo test` stdout reports failed, one per `test <name> ... FAILED` line.
fn failed(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("test ")?.strip_suffix(" ... FAILED"))
        .collect()
}

/// What a failed probe says: each test it failed and the two fixes, or, if none failed, the build's end.
fn report(stdout: &str, stderr: &str) -> String {
    let names = failed(stdout);
    if names.is_empty() {
        let lines: Vec<&str> = stderr.lines().collect();
        let tail = lines[lines.len().saturating_sub(STDERR_TAIL_LINES)..].join("\n");
        return format!("the pool probe failed before any test did:\n{tail}");
    }
    let listed: Vec<String> = names.iter().map(|name| format!("  {name}")).collect();
    format!(
        "the pool probe (every random pool rotated one place, #3117) failed {} of the cards' tests: each \
         leans on where a roll lands, so the pull request that next adds a card to its pool would fail it \
         (R380):\n{}\n\
         Convert each as crates/engine/src/testkit/pool_probe.rs's header says: a test of a pool's content \
         or order reads it under `with_pool_shift(0, …)`; a test of a roll searches for it with `seed_where` \
         or `cursor_where` (#848) or builds the position directly. One runs alone with \
         `cargo test -p jackioh-cards --features jackioh-engine/pool-probe <name>`.",
        names.len(),
        listed.join("\n"),
    )
}

#[test]
fn the_report_names_every_failed_test_and_both_fixes() {
    let stdout = "\
running 3 tests
test scripts::a::tests::pins_a_roll ... FAILED
test scripts::b::tests::holds ... ok
test scripts::c::tests::pins_an_order ... FAILED

failures:

---- scripts::a::tests::pins_a_roll stdout ----
assertion failed

failures:
    scripts::a::tests::pins_a_roll
    scripts::c::tests::pins_an_order

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
";
    assert_eq!(
        failed(stdout),
        [
            "scripts::a::tests::pins_a_roll",
            "scripts::c::tests::pins_an_order"
        ]
    );
    let said = report(stdout, "");
    assert!(said.contains("failed 2 of the cards' tests"), "{said}");
    for part in [
        "scripts::a::tests::pins_a_roll",
        "scripts::c::tests::pins_an_order",
        "with_pool_shift(0, …)",
        "seed_where",
        "cursor_where",
    ] {
        assert!(said.contains(part), "{part} missing from: {said}");
    }
    assert!(!said.contains("scripts::b::"), "{said}");
    let broken = report(
        "",
        "error: could not compile `jackioh-cards` (lib test) due to 1 previous error",
    );
    assert!(broken.contains("could not compile"), "{broken}");
}

#[test]
#[ignore = "daily: rebuilds the cards' tests under the pool-probe feature and runs them, about five minutes; the engine's pool_probe canary keeps the rotation itself per pull request"]
fn every_cards_test_passes_with_every_random_pool_rotated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO"))
        .args(PROBE)
        .arg("--target-dir")
        .arg(root.join(TARGET_DIR))
        .current_dir(&root)
        .output()
        .expect("cargo runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // libtest captures `print!` and `eprint!` alone, so each binary's summary and failures (every failed
    // test's panic among them) reach the daily log this way, as tests/perf.rs writes its table.
    {
        let mut log = std::io::stderr().lock();
        let mut in_failures = false;
        for line in stdout.lines() {
            in_failures |= line == "failures:";
            let summary = line.starts_with("test result:");
            if in_failures || summary {
                let _ = writeln!(log, "{line}");
            }
            in_failures &= !summary;
        }
    }
    assert!(output.status.success(), "{}", report(&stdout, &stderr));
}
