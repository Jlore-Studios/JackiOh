//! Golden traces (docs/v0.3.0/SURFACE.md §13) and the recorded hotseat game: the oracle the Rust
//! engine is held to (README V4), recorded from the TypeScript engine before any Rust ran (part 23),
//! and the port of `packages/cards/test/hotseat-replay.test.ts`.
//!
//! THE GOLDEN TRACES. `golden/games.jsonl` holds 240 games, one per line, written by
//! `scripts/golden/record.ts` from the TypeScript engine: seeds 1–200 dealt, seeded and played as
//! `packages/cards/test/fuzz.test.ts` plays them, seeds 201–240 as `fuzz-handicap.test.ts` plays its
//! own (one seat on Medium or Hard, R180–R184). For each line this file does what §13.3 says:
//! `create_game(args)`, `begin_game`, check `begin`; then for each step check `l` (the actor's legal
//! actions before the step, as a set) for the actor `a.playerId`, apply `a` with `reduce`, and check
//! `s` (`hash_state`, §5.2), `v` (both seats' `view_for`) and `e` (the step's events), every hash
//! FNV-1a 32 over the UTF-16 code units of the value's canonical JSON. Last, the game's ending and
//! its length must be the line's `end`.
//!
//! On a game's first mismatch the test names the seed, the step, which hash and the action, writes
//! the Rust side's canonical text to `target/golden-diff/<seed>-<step>-<which>.json`, and prints the
//! command that writes TS's side next to it (`pnpm exec tsx scripts/golden/record.ts --seed <k>
//! --dump-step <n>`, `.ts.json`, run in a checkout of 05f5cfd, the last commit that has the
//! TypeScript): diff the two. Every game is replayed, so the report lists every
//! game that diverges with its first divergent step; fix the earliest seed and step first, since one
//! root cause often explains dozens. The games are split over GOLDEN_SHARDS tests so that the test
//! harness replays them on as many threads (the engine's `clippy.toml` bans spawning one here).
//!
//! Never edit a trace by hand. After an intended rules change (there is none in v0.3.0),
//! `cargo jackioh golden bless` rewrites the file from the Rust engine.
//!
//! The engine's `clippy.toml` (SURFACE §3) applies to this test too: data comes in with
//! `include_str!`, and the two places that touch the file system at run time (the diff written on a
//! divergence, and the optional comparison with a local Cypress recording that TS's test made) say
//! why they are allowed to.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::replay::{canonical, fnv1a32_utf16};
use jackioh_engine::{
    Action, CreateGameArgs, FoldArgs, FoldResult, GameEvent, GameOverReason, GameResult, GameState, Phase,
    PlayerId, Winner, begin_game, create_game, fold, hash_state, legal_actions, reduce, view_for,
};

// =============================================================================================
// The golden traces (SURFACE §13)
// =============================================================================================

/// §13.3: the traces, recorded by `scripts/golden/record.ts`.
const GAMES: &str = include_str!("golden/games.jsonl");

/// §13.1: 200 games on this spec's resources and 40 with one seat handicapped.
const GOLDEN_GAMES: usize = 240;

/// §13.3: the file format's version, each line's `v`.
const FORMAT_VERSION: u64 = 1;

/// §13.3: where a divergence leaves the Rust side's canonical text (the workspace's `target/`).
const DIFF_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/golden-diff");

/// A game's first divergence from its trace.
struct Mismatch {
    seed: String,
    /// `begin`, the 0-based step index, or `end`.
    step: String,
    /// `s`, `v-p1`, `v-p2`, `e`, `l`, `refused` (reduce refused the recorded action) or `end`.
    which: &'static str,
    expected: String,
    actual: String,
    /// The recorded action, as JSON, for a step.
    action: Option<String>,
    /// Where the Rust side's canonical text was written, when there is one to write.
    diff: Option<String>,
}

impl Mismatch {
    fn report(&self) -> String {
        let mut text = format!(
            "seed {} step {}: `{}` expected {}, Rust has {}",
            self.seed, self.step, self.which, self.expected, self.actual
        );
        if let Some(action) = &self.action {
            text.push_str(&format!("\n    action: {action}"));
        }
        if let Some(diff) = &self.diff {
            text.push_str(&format!("\n    Rust side: {diff}"));
        }
        if let Some(k) = seed_number(&self.seed) {
            // A refusal or an ending is compared with the state the previous step left.
            let step = match (self.which, self.step.parse::<usize>()) {
                ("refused", Ok(0)) => "begin".to_string(),
                ("refused", Ok(n)) => (n - 1).to_string(),
                _ => self.step.clone(),
            };
            if step != "end" {
                text.push_str(&format!(
                    "\n    TS side:   in a checkout of 05f5cfd, the last commit with the TypeScript \
                     (git worktree add ../jackioh-ts 05f5cfd, then pnpm install there):\
                     \n               pnpm exec tsx scripts/golden/record.ts --seed {k} --dump-step {step}"
                ));
            }
        }
        text
    }
}

/// The seed number `record.ts --seed` takes: the trailing number of `jackioh-fuzz-<k>` or
/// `jackioh-fuzz-handicap-<k>`.
fn seed_number(seed: &str) -> Option<u32> {
    seed.rsplit('-').next().and_then(|tail| tail.parse::<u32>().ok())
}

/// §13.3: write the Rust side's canonical text where `record.ts --dump-step` writes TS's, and say
/// where it went.
#[allow(clippy::disallowed_methods)] // SURFACE §13.3: the diff file is this harness's output; clippy.toml's I/O ban guards the rules, which never reach this.
fn write_diff(seed: &str, step: &str, which: &str, text: &str) -> String {
    let dir = PathBuf::from(DIFF_DIR);
    let path = dir.join(format!("{seed}-{step}-{which}.json"));
    match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, format!("{text}\n"))) {
        Ok(()) => path.display().to_string(),
        Err(error) => format!("(could not write {}: {error})", path.display()),
    }
}

/// §5.2 step 1: the state's canonical text as `hash_state` hashes it, without `applied` and `opening`.
fn hashed_state_text(state: &GameState) -> String {
    let mut value = serde_json::to_value(state).expect("a GameState serialises");
    if let Some(object) = value.as_object_mut() {
        object.remove("applied");
        object.remove("opening");
    }
    canonical(&value)
}

/// §13.2's `l`: the actor's legal actions as a set — each action's canonical text, distinct, sorted
/// by UTF-16 code unit (JS's default `sort`), joined by "\n".
fn legal_text(state: &GameState, actor: PlayerId) -> String {
    let mut lines: Vec<String> = legal_actions(state, actor)
        .iter()
        .map(|action| canonical(&serde_json::to_value(action).expect("an ActionBody serialises")))
        .collect();
    lines.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    lines.dedup();
    lines.join("\n")
}

/// The recorded hash at `key` (a string), or a description of what is there instead.
fn recorded(snapshot: &Value, key: &str) -> String {
    snapshot[key]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("<no `{key}` in the line>"))
}

/// Checks `s`, `v` and `e` of one snapshot (`begin`, or the state a step left with its events).
fn check_snapshot(
    seed: &str,
    step: &str,
    action: Option<&str>,
    snapshot: &Value,
    state: &GameState,
    events: &[GameEvent],
) -> Result<(), Box<Mismatch>> {
    let mismatch = |which: &'static str, expected: String, actual: String, text: &str| {
        Box::new(Mismatch {
            seed: seed.to_string(),
            step: step.to_string(),
            which,
            expected,
            actual,
            action: action.map(str::to_string),
            diff: Some(write_diff(seed, step, which, text)),
        })
    };

    let s = hash_state(state);
    let expected = recorded(snapshot, "s");
    if s != expected {
        return Err(mismatch("s", expected, s, &hashed_state_text(state)));
    }

    for (index, (player, which)) in [(PlayerId::P1, "v-p1"), (PlayerId::P2, "v-p2")]
        .into_iter()
        .enumerate()
    {
        let view = serde_json::to_value(view_for(state, player)).expect("a PlayerView serialises");
        let text = canonical(&view);
        let hash = fnv1a32_utf16(&text);
        let expected = snapshot["v"][index]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("<no `v[{index}]` in the line>"));
        if hash != expected {
            return Err(mismatch(which, expected, hash, &text));
        }
    }

    let text = canonical(&serde_json::to_value(events).expect("GameEvents serialise"));
    let hash = fnv1a32_utf16(&text);
    let expected = recorded(snapshot, "e");
    if hash != expected {
        return Err(mismatch("e", expected, hash, &text));
    }
    Ok(())
}

/// Replays one line of `games.jsonl` (§13.3). `Ok` carries the number of steps replayed.
fn replay_line(index: usize, line: &str) -> Result<usize, Box<Mismatch>> {
    let game: Value = serde_json::from_str(line)
        .unwrap_or_else(|error| panic!("games.jsonl line {}: not JSON: {error}", index + 1));
    let seed = game["seed"]
        .as_str()
        .unwrap_or_else(|| panic!("games.jsonl line {}: no `seed`", index + 1))
        .to_string();
    assert_eq!(
        game["v"].as_u64(),
        Some(FORMAT_VERSION),
        "games.jsonl line {} ({seed}): format version",
        index + 1
    );
    let args: CreateGameArgs = serde_json::from_value(game["args"].clone())
        .unwrap_or_else(|error| panic!("{seed}: `args` is not createGame's options: {error}"));

    let begun = begin_game(&create_game(&args));
    if let Some(error) = begun.error {
        return Err(Box::new(Mismatch {
            seed,
            step: "begin".to_string(),
            which: "refused",
            expected: "beginGame to set the game up".to_string(),
            actual: format!("an error: {error}"),
            action: None,
            diff: None,
        }));
    }
    let mut state = begun.state;
    check_snapshot(&seed, "begin", None, &game["begin"], &state, &begun.events)?;

    let steps = game["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("{seed}: no `steps`"));
    for (n, step) in steps.iter().enumerate() {
        let label = n.to_string();
        let action_json = step["a"].to_string();
        let action: Action = serde_json::from_value(step["a"].clone())
            .unwrap_or_else(|error| panic!("{seed} step {n}: `a` is not an Action ({error}): {action_json}"));

        // `l` for the actor, on the state the action is chosen in.
        let text = legal_text(&state, action.player_id);
        let hash = fnv1a32_utf16(&text);
        let expected = recorded(step, "l");
        if hash != expected {
            return Err(Box::new(Mismatch {
                seed: seed.clone(),
                step: label.clone(),
                which: "l",
                expected,
                actual: hash,
                action: Some(action_json),
                diff: Some(write_diff(&seed, &label, "l", &text)),
            }));
        }

        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            return Err(Box::new(Mismatch {
                seed: seed.clone(),
                step: label.clone(),
                which: "refused",
                expected: "reduce to apply the recorded action".to_string(),
                actual: format!("a refusal: {error}"),
                action: Some(action_json),
                diff: Some(write_diff(&seed, &label, "refused", &hashed_state_text(&state))),
            }));
        }
        state = result.state;
        check_snapshot(&seed, &label, Some(&action_json), step, &state, &result.events)?;
    }

    let ending = json!({
        "winner": state.result.as_ref().map(|result| result.winner),
        "reason": state.result.as_ref().map(|result| result.reason),
        "steps": steps.len(),
    });
    if ending != game["end"] {
        return Err(Box::new(Mismatch {
            seed,
            step: "end".to_string(),
            which: "end",
            expected: game["end"].to_string(),
            actual: ending.to_string(),
            action: None,
            diff: None,
        }));
    }
    Ok(steps.len())
}

/// The lines of `games.jsonl`, blank ones (a trailing newline) left out.
fn golden_lines() -> Vec<&'static str> {
    GAMES.lines().filter(|line| !line.trim().is_empty()).collect()
}

/// Replays every game of one shard and fails with every divergent game's first mismatch.
fn replay_shard(shard: usize) {
    register_all();
    let mut failures: Vec<Box<Mismatch>> = Vec::new();
    let mut games = 0usize;
    let mut steps = 0usize;
    for (index, line) in golden_lines().into_iter().enumerate() {
        if index % GOLDEN_SHARDS != shard {
            continue;
        }
        games += 1;
        match replay_line(index, line) {
            Ok(n) => steps += n,
            Err(mismatch) => failures.push(mismatch),
        }
    }
    assert!(
        failures.is_empty(),
        "golden shard {shard}: {} of {games} games diverged from their TypeScript trace \
         ({steps} steps of the others replayed identically):\n{}",
        failures.len(),
        failures
            .iter()
            .map(|mismatch| mismatch.report())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

mod golden_traces {
    use super::*;

    #[test]
    fn the_file_holds_every_recorded_game_once() {
        let lines = golden_lines();
        assert_eq!(
            lines.len(),
            GOLDEN_GAMES,
            "SURFACE §13.1: 200 games and 40 handicapped"
        );
        let mut seen = BTreeSet::new();
        for (index, line) in lines.iter().enumerate() {
            let game: Value = serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("games.jsonl line {}: not JSON: {error}", index + 1));
            let seed = game["seed"].as_str().unwrap_or_default().to_string();
            assert_eq!(
                game["args"]["seed"].as_str(),
                Some(seed.as_str()),
                "line {}: args.seed",
                index + 1
            );
            assert!(seen.insert(seed.clone()), "{seed} is recorded twice");
        }
    }
}

/// One test per shard of the games, numbered 0.. in order; `GOLDEN_SHARDS` is how many there are.
macro_rules! golden_shards {
    ($($name:ident = $shard:expr),* $(,)?) => {
        /// How many tests the games are split over: game `i` is replayed by shard `i % GOLDEN_SHARDS`,
        /// and the test harness runs the shards on as many threads.
        const GOLDEN_SHARDS: usize = [$($shard),*].len();

        mod replays_the_golden_traces {
            use super::*;
            $(
                #[test]
                fn $name() {
                    replay_shard($shard);
                }
            )*
        }
    };
}

golden_shards!(
    shard_00 = 0,
    shard_01 = 1,
    shard_02 = 2,
    shard_03 = 3,
    shard_04 = 4,
    shard_05 = 5,
    shard_06 = 6,
    shard_07 = 7,
    shard_08 = 8,
    shard_09 = 9,
    shard_10 = 10,
    shard_11 = 11,
);

// =============================================================================================
// The recorded hotseat game (port of packages/cards/test/hotseat-replay.test.ts)
// =============================================================================================
//
// The vitest half of BUILD M5-T3's acceptance: "the same seed and actions reproduce the same final
// state hash in the browser and in vitest".
//
// The browser half already exists. `e2e/cypress/e2e/01-hotseat-full-game.cy.ts` plays a whole game
// through `/dev/hotseat`, then `cy.task("replayHash")` folds the recorded `(seed, decks, log)` in a
// Node child process and asserts the browser's own `hashState` equals that fold. What did not exist
// was a fold inside `vitest` — `apps/web/src/game/hotseat.test.ts` runs two sessions against a
// SCRIPTED engine whose `reduce` increments a turn counter, so it would pass against an engine with
// no determinism at all. This file is the missing fold: the real `reduce`, the real catalog, the
// real card scripts, and a hash written down.
//
// WHY IT LIVES IN the engine's golden tests. The log names real cards (`core-003`, `core-045`, …) and
// folding it without their scripts registered would fizzle every Cry (`script_of` falls back to the
// empty script) and reach a different state. Only `jackioh-cards` owns the catalog and the scripts,
// and `jackioh-engine` must not depend on it — the dependency runs the other way — so the engine
// takes it as a dev-dependency (SURFACE §2), which integration tests may, next to the golden traces,
// which fold the same way.
//
// WHY THE LOG IS COMMITTED HERE. `e2e/artifacts/` is gitignored (`e2e/.gitignore`), and CI's
// test job runs on a fresh checkout where no browser has ever run — so a test that read the artifact
// directly would find nothing there. Skipping when the file is absent would make this another check
// that measures nothing, and failing when it is absent would make `cargo test` depend on having run
// Cypress first. The recording is therefore checked in, byte-identical, as
// `tests/golden/01-hotseat-full-game.json` (copied from `packages/cards/test/fixtures/`), and:
//
//   - the fold below ALWAYS runs, against the committed copy, and never skips;
//   - the last test compares the committed copy with `e2e/artifacts/…` WHEN a local Cypress run has
//     left one there, so a recording that drifts is reported instead of silently diverging. Its
//     absence is the normal state (gitignored, never in CI) and is not evidence of drift, so it is
//     the one thing here that is conditional — and it is conditional on a file that only exists
//     when there is something to compare.
//
// WHY A LITERAL HASH. Comparing a fold with a fold in the same process proves nothing: both would
// move together. `EXPECTED_HASH` is written down, so a change in shuffling, in turn order, in any
// card's script, or a lost `register_all()` moves it and fails here. It is not a magic number: spec
// 01 asserts the browser's own hash equals a fold of this same log, so this value is the browser's
// hash for as long as that spec is green.

/// The recording, as `e2e/support/tasks/replay.ts` writes it: `{ seed, decks, log }`.
#[derive(Deserialize)]
struct Recording {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    log: Vec<Action>,
}

/// The committed recording (TS's `COMMITTED`), compiled in (SURFACE §3: data reaches a pure crate's
/// tests with `include_str!`).
const COMMITTED: &str = include_str!("golden/01-hotseat-full-game.json");
const COMMITTED_PATH: &str = "crates/engine/tests/golden/01-hotseat-full-game.json";

/// Where Cypress leaves its copy. Gitignored, so it is present only after a local `pnpm test:e2e`.
const RECORDED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../e2e/artifacts/01-hotseat-full-game.json"
);

/// The final state hash of the recorded game.
///
/// Refresh it ONLY together with the recording, and only when the rules deliberately changed: run
/// `pnpm test:e2e` (spec 01 re-records `e2e/artifacts/01-hotseat-full-game.json` and checks the
/// browser against a fold of it), copy that file over the fixture here, and paste the hash this test
/// reports. Editing the number on its own turns the check into a rubber stamp.
///
/// The one exception is a change to what the state RECORDS about the same game, which leaves the log
/// as it is. The polish-4 hunt made three: the other player's turn log is emptied at each turn start
/// (§6.2's "this turn"), and `lastDamagedBy` names only the hit that took a unit to 0 (R42). A fold of
/// this log before and after the second differs in that field on three instances and nowhere else.
/// The third: the turn log records what each play paid (`costsPaid`, R213), and a hand card's queued
/// trigger no longer takes a number from `nextSeq` (R177). A fold before and after it differs in p1's
/// `costsPaid` and in `nextSeq` and the two frontier ids it numbers, 4 lower, and nowhere else.
///
/// Round 6 of the hunt moved it twice more, and moved the log's ids with it. `createGame` now numbers
/// each deck's cards in an order of the seed's own (R223), so the log's 40 deck-card ids were relabeled
/// through that mapping and nothing else in the log changed; and the state counts the field's
/// departures (`fieldExits`, R174). A fold of the old log under the old numbering and a fold of the
/// relabeled log, relabeled back, differ in `fieldExits` alone — the same game, action for action.
///
/// The Coin (R244) re-recorded it, by the procedure above: p2 is dealt The Coin after the mulligan,
/// so spec 01, which plays whatever the client offers, plays it on p2's first turn (nonce n6), and
/// every instance created after setup takes an id one higher. A new game, not a relabeled one: 51
/// actions where there were 50, still won by p1 by hero death.
///
/// The concurrent mulligan (R265) moved it once more, under the exception above: the log is the same
/// 51 actions and folds without a refusal. Both mulligan prompts now open in `beginGame`, which does
/// not run the resolution loop, where p2's used to open inside p1's answer and be dispatched there, so
/// one event fewer takes a number: a fold before and after differs in `nextSeq` and the two frontier
/// ids it numbers, 1 lower, and nowhere else.
///
/// The Radiant pass (R275, R276) re-recorded it the same way: its decks hold cards whose Radiant faces
/// were raised (#8, #25, #73, #81, the Rush and Felinor Tokens), and a Radiant Saintess's Death now
/// reaches the hand, so the same seed plays a different game. 38 actions, still won by p1 by hero death.
/// Spec 01 records the same log under the concurrent mulligan, which folds to the hash below.
///
/// R311 moved the hash and not the game: every library card now records what its owner was shown of
/// it going in (`knownAs`). The same fold with that field stripped from every instance hashes to
/// "cc583237", the value before it.
///
/// Patch v0.1.1 (R360–R366) re-recorded it by the procedure above: its decks hold #1, #8, #20, #25,
/// #56, #77, #81 and #92, whose stats, keywords or rules the patch changed, so the same seed plays a
/// different game. 37 actions, still won by p1 by hero death.
///
/// Patch v0.2.0 moved it three times more and not the game. Its draw count (B5 E4, R457): each player's
/// state counts the draws they made on the turn running (`draws`). Its announce (R448) and play records
/// (R451), under the exception above: every play now emits `cardAnnounced` before it moves, which the
/// frontier numbers from `nextSeq`, so every id numbered after a play is one higher per play; and the
/// state records each player's plays by type this turn (`turnLog.playedByType`), by tag this game and
/// their last face-up play (`gameLog`), and the last Spell played (`lastSpell`). Its Core patch to #32
/// Prem Panther (R426) made the Panther's text its attack's own hook rather than a trigger on every
/// death, so the deaths it watched queue no entries numbered from `nextSeq`. The same 37 actions fold
/// with no refusal to the same end, won by p1 by hero death.
///
/// C+ #35 Rollback's history (R419) moved it once more and not the game: every turn's start now records
/// the field in `state.boardHistory`. The same fold with that field deleted hashes to "2d6aab2a", the
/// value before it.
///
/// v0.3.0 (SURFACE §5.2): the Rust engine must fold this same log to this same hash, which practice
/// saves on players' devices and e2e specs 01, 13 and 22 rely on. Nothing about the game changed.
const EXPECTED_HASH: &str = "a798906b";

/// What the recorded game ends in — a second anchor, so the hash is not the only witness.
const EXPECTED_RESULT: GameResult = GameResult {
    winner: Winner::P1,
    reason: GameOverReason::HeroDeath,
};
const EXPECTED_ACTIONS: usize = 37;

fn read(path: &str, text: &str) -> Recording {
    let parsed: Value =
        serde_json::from_str(text).unwrap_or_else(|error| panic!("{path} is not JSON: {error}"));
    if !parsed["seed"].is_string() || !parsed["decks"].is_array() || !parsed["log"].is_array() {
        panic!("{path} is not a recorded hotseat game: expected {{ seed, decks, log }}");
    }
    serde_json::from_value(parsed)
        .unwrap_or_else(|error| panic!("{path} is not a recorded hotseat game: {error}"))
}

fn recording() -> Recording {
    read(COMMITTED_PATH, COMMITTED)
}

/// `fold({ seed, decks, log })`, the three fields the recording carries and nothing else.
fn fold_recording(seed: &str, decks: &(Vec<String>, Vec<String>), log: &[Action]) -> FoldResult {
    let input: FoldArgs = serde_json::from_value(json!({ "seed": seed, "decks": decks, "log": log }))
        .expect("{ seed, decks, log } is a fold's input");
    fold(&input)
}

mod the_recorded_hotseat_game_replays_in_vitest_build_m5_t3 {
    use super::*;

    #[test]
    fn folds_the_browsers_own_seed_decks_log_to_the_recorded_final_state_hash() {
        // The catalog and the card scripts, exactly as the web client registers them before the
        // browser's first `reduce`. Without them the Crys fizzle and the hash moves.
        register_all();
        let recording = recording();

        let replayed = fold_recording(&recording.seed, &recording.decks, &recording.log);

        // Every recorded action is legal against a fold from scratch. A rejection would mean the log
        // and the engine have parted company, and the hash below would be a hash of a shorter game.
        let refusals: Vec<String> = replayed
            .errors
            .iter()
            .map(|error| format!("{}: {}", error.nonce, error.error))
            .collect();
        assert_eq!(refusals, Vec::<String>::new());
        assert_eq!(hash_state(&replayed.state), EXPECTED_HASH);

        // Anchors that say what that hash IS, so a future diff reads as a rules change and not as an
        // unexplained number: the game is over, p1 won by hero death, and p2's hero is dead.
        assert_eq!(replayed.state.result, Some(EXPECTED_RESULT));
        assert_eq!(replayed.state.phase, Phase::Over);
        assert!(replayed.state.players.p2.hero.health <= 0);
        assert!(replayed.state.players.p1.hero.health > 0);
    }

    #[test]
    fn is_a_fold_of_the_whole_log_one_action_fewer_is_a_different_hash() {
        // The point of this one is that the assertion above cannot pass by accident. If `fold` ignored
        // the log, or the hash ignored the state, these two would agree.
        register_all();
        let recording = recording();
        let short = fold_recording(
            &recording.seed,
            &recording.decks,
            &recording.log[..recording.log.len() - 1],
        );

        assert_ne!(hash_state(&short.state), EXPECTED_HASH);
        assert_eq!(short.state.result, None);
    }

    #[test]
    fn is_a_fold_of_that_seed_another_seed_is_a_different_hash() {
        register_all();
        let recording = recording();
        let other = fold_recording(
            &format!("{}-not", recording.seed),
            &recording.decks,
            &recording.log,
        );
        assert_ne!(hash_state(&other.state), EXPECTED_HASH);
    }

    #[test]
    fn is_deterministic_folding_twice_in_a_row_gives_the_same_hash() {
        register_all();
        let recording = recording();
        let first = fold_recording(&recording.seed, &recording.decks, &recording.log);
        let second = fold_recording(&recording.seed, &recording.decks, &recording.log);
        assert_eq!(hash_state(&second.state), hash_state(&first.state));
    }

    #[test]
    fn carries_a_real_recording_51_stamped_actions_over_two_deck_legal_libraries() {
        let recording = recording();
        assert_eq!(recording.seed, "01-hotseat");
        assert_eq!(recording.log.len(), EXPECTED_ACTIONS);
        // The typed log cannot lack a nonce or a seat, so read the recording's own JSON for them.
        let raw: Value = serde_json::from_str(COMMITTED).expect("the recording is JSON");
        for action in raw["log"].as_array().expect("the recording has a log") {
            assert!(
                action["nonce"].is_string(),
                "every action carries a nonce: {action}"
            );
            assert!(
                ["p1", "p2"].contains(&action["playerId"].as_str().unwrap_or_default()),
                "every action names its seat: {action}"
            );
        }
        for deck in [&recording.decks.0, &recording.decks.1] {
            assert_eq!(deck.len(), 20);
            assert_eq!(deck.iter().collect::<BTreeSet<_>>().len(), 20);
            for id in deck {
                assert!(CATALOG.contains_key(id), "{id} is a catalog card");
            }
        }
    }

    #[test]
    #[allow(clippy::disallowed_methods)] // reads Cypress's local recording when there is one, as TS's test did; never the rules.
    fn matches_the_last_browser_recording_when_a_local_cypress_run_has_left_one() {
        let recorded = match std::fs::read_to_string(RECORDED) {
            Ok(text) => text,
            // `e2e/artifacts/` is gitignored: on CI and on a checkout where Cypress has not run, there
            // is nothing to compare. The fold above has already run against the committed copy, so
            // nothing is being skipped here except a comparison with a file that does not exist.
            Err(_) => return,
        };

        let recorded: Value = serde_json::from_str(&recorded).expect("Cypress's recording is JSON");
        let committed: Value = serde_json::from_str(COMMITTED).expect("the committed recording is JSON");
        assert_eq!(
            recorded, committed,
            "e2e/artifacts/01-hotseat-full-game.json no longer matches \
             crates/engine/tests/golden/01-hotseat-full-game.json. Spec 01 recorded a different game, \
             so the fixture here is stale: copy the artifact over it and update EXPECTED_HASH with the \
             value this file reports."
        );
    }
}
