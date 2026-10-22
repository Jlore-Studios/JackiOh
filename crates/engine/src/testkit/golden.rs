//! The golden replay (SURFACE §13.3), its pure half: the canonical texts, one recorded game's replay
//! to its first `Mismatch` and the hotseat fixture's fold. The engine's `tests/golden.rs` and
//! `cargo jackioh golden check` both replay with it, so the two can never disagree about a game (#1641).
//!
//! A mismatch carries the Rust side's canonical text (`text`) and never writes it: the engine's
//! `clippy.toml` bans file I/O here, so each caller writes it to `target/golden-diff/` under
//! `Mismatch::diff_file` and sets `diff` to where it went, which `report` prints.

use serde_json::{Value, json};

use crate::replay::{canonical, fnv1a32_utf16};
use crate::{
    Action, CreateGameArgs, FoldArgs, GameEvent, GameState, PlayerId, begin_game, create_game, fold,
    hash_state, legal_actions, reduce, view_for,
};

/// §13.3: the file format's version, each line's `v`.
pub const FORMAT_VERSION: u32 = 1;

/// The recorded hotseat game, as `e2e/support/tasks/replay.ts` writes it: `{ seed, decks, log }`.
pub const HOTSEAT: &str = include_str!("../../tests/golden/01-hotseat-full-game.json");

/// The final state hash of the recorded hotseat game (`hotseat-replay.test.ts`'s `EXPECTED_HASH`, SURFACE
/// §5.2).
///
/// Refresh it ONLY together with the recording, and only when the rules deliberately changed: run
/// `pnpm test:e2e` (spec 01 re-records `e2e/artifacts/01-hotseat-full-game.json` and checks the
/// browser against a fold of it), copy that file over `crates/engine/tests/golden/`'s fixture, and paste
/// the hash the engine's `tests/golden.rs` reports. Editing the number on its own turns the check into a
/// rubber stamp.
///
/// The exception is a change to what the state RECORDS about the same game, which leaves the log as it
/// is and moves the hash alone: R42, R174, R177, R213, R223, R265, R311, R419, R426, R448, R451 and
/// R457 (§6.2; B5 E4), and R943's Created mark and R982's last element, which the Meditative set's
/// release (R1420, v0.3.5) began to record: the same log, both stripped, still folds to the old
/// `a798906b`. A change to the game itself re-records the log (R244, R275, R276, R360–R366).
/// Practice saves on players' devices and e2e specs 01, 13 and 22 rely on the hash holding (§5.2).
pub const HOTSEAT_HASH: &str = "8784adae";

/// A game's first divergence from its trace.
pub struct Mismatch {
    pub seed: String,
    /// `begin`, the 0-based step index, or `end`.
    pub step: String,
    /// `s`, `v-p1`, `v-p2`, `e`, `l`, `refused` (reduce refused the recorded action), `end`, or
    /// `line` (the line itself is not a trace).
    pub which: &'static str,
    pub expected: String,
    pub actual: String,
    /// The recorded action, as JSON, for a step.
    pub action: Option<String>,
    /// The Rust side's canonical text, when there is one to write.
    pub text: Option<String>,
    /// Where the caller wrote `text`.
    pub diff: Option<String>,
}

impl Mismatch {
    /// The file `text` goes in under `target/golden-diff/`, where `record.ts --dump-step` writes TS's.
    pub fn diff_file(&self) -> String {
        format!("{}-{}-{}.json", self.seed, self.step, self.which)
    }

    pub fn report(&self) -> String {
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
            // A refusal is compared with the state the previous step left.
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
pub fn seed_number(seed: &str) -> Option<u32> {
    seed.rsplit('-').next().and_then(|tail| tail.parse::<u32>().ok())
}

/// §5.2 step 1: the state's canonical text as `hash_state` hashes it, without `applied` and `opening`.
pub fn hashed_state_text(state: &GameState) -> String {
    let mut value = serde_json::to_value(state).expect("a GameState serialises");
    if let Some(object) = value.as_object_mut() {
        object.remove("applied");
        object.remove("opening");
    }
    canonical(&value)
}

/// The canonical text of both seats' views, p1 first.
pub fn view_texts(state: &GameState) -> [String; 2] {
    [PlayerId::P1, PlayerId::P2].map(|player| {
        canonical(&serde_json::to_value(view_for(state, player)).expect("a PlayerView serialises"))
    })
}

/// The canonical text of a step's events.
pub fn events_text(events: &[GameEvent]) -> String {
    canonical(&serde_json::to_value(events).expect("GameEvents serialise"))
}

/// §13.2's `l`: the actor's legal actions as a set — each action's canonical text, distinct, sorted
/// by UTF-16 code unit (JS's default `sort`), joined by "\n".
pub fn legal_text(state: &GameState, actor: PlayerId) -> String {
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
    let mismatch = |which: &'static str, expected: String, actual: String, text: String| {
        Box::new(Mismatch {
            seed: seed.to_string(),
            step: step.to_string(),
            which,
            expected,
            actual,
            action: action.map(str::to_string),
            text: Some(text),
            diff: None,
        })
    };

    let s = hash_state(state);
    let expected = recorded(snapshot, "s");
    if s != expected {
        return Err(mismatch("s", expected, s, hashed_state_text(state)));
    }

    for (index, (text, which)) in view_texts(state).into_iter().zip(["v-p1", "v-p2"]).enumerate() {
        let hash = fnv1a32_utf16(&text);
        let expected = snapshot["v"][index]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("<no `v[{index}]` in the line>"));
        if hash != expected {
            return Err(mismatch(which, expected, hash, text));
        }
    }

    let text = events_text(events);
    let hash = fnv1a32_utf16(&text);
    let expected = recorded(snapshot, "e");
    if hash != expected {
        return Err(mismatch("e", expected, hash, text));
    }
    Ok(())
}

/// Replays one parsed line of `games.jsonl` (§13.3): `create_game(args)`, `begin_game`, check `begin`; then
/// per step check `l` for the actor, apply `a` with `reduce`, check `s`, `v` and `e`; last, the ending and
/// length must be `end`. `Ok` carries the number of steps replayed.
pub fn replay_game(seed: &str, game: &Value) -> Result<usize, Box<Mismatch>> {
    let fail = |step: &str, which: &'static str, expected: String, actual: String| {
        Box::new(Mismatch {
            seed: seed.to_string(),
            step: step.to_string(),
            which,
            expected,
            actual,
            action: None,
            text: None,
            diff: None,
        })
    };
    if game["v"].as_u64() != Some(u64::from(FORMAT_VERSION)) {
        return Err(fail(
            "begin",
            "line",
            format!("format version {FORMAT_VERSION}"),
            game["v"].to_string(),
        ));
    }
    let args: CreateGameArgs = match serde_json::from_value(game["args"].clone()) {
        Ok(args) => args,
        Err(error) => {
            return Err(fail(
                "begin",
                "line",
                "createGame's options".into(),
                error.to_string(),
            ));
        }
    };

    let begun = begin_game(&create_game(&args));
    if let Some(error) = begun.error {
        return Err(fail(
            "begin",
            "refused",
            "beginGame to set the game up".into(),
            format!("an error: {error}"),
        ));
    }
    let mut state = begun.state;
    check_snapshot(seed, "begin", None, &game["begin"], &state, &begun.events)?;

    let Some(steps) = game["steps"].as_array() else {
        return Err(fail("begin", "line", "a `steps` array".into(), "none".into()));
    };
    for (n, step) in steps.iter().enumerate() {
        let label = n.to_string();
        let action_json = step["a"].to_string();
        let action: Action = match serde_json::from_value(step["a"].clone()) {
            Ok(action) => action,
            Err(error) => {
                return Err(Box::new(Mismatch {
                    action: Some(action_json),
                    ..*fail(&label, "line", "an Action".into(), error.to_string())
                }));
            }
        };

        // `l` for the actor, on the state the action is chosen in.
        let text = legal_text(&state, action.player_id);
        let hash = fnv1a32_utf16(&text);
        let expected = recorded(step, "l");
        if hash != expected {
            return Err(Box::new(Mismatch {
                action: Some(action_json),
                text: Some(text),
                ..*fail(&label, "l", expected, hash)
            }));
        }

        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            return Err(Box::new(Mismatch {
                action: Some(action_json),
                text: Some(hashed_state_text(&state)),
                ..*fail(
                    &label,
                    "refused",
                    "reduce to apply the recorded action".into(),
                    format!("a refusal: {error}"),
                )
            }));
        }
        state = result.state;
        check_snapshot(seed, &label, Some(&action_json), step, &state, &result.events)?;
    }

    let ending = json!({
        "winner": state.result.as_ref().map(|result| result.winner),
        "reason": state.result.as_ref().map(|result| result.reason),
        "steps": steps.len(),
    });
    if ending != game["end"] {
        return Err(fail("end", "end", game["end"].to_string(), ending.to_string()));
    }
    Ok(steps.len())
}

/// `hotseat-replay.test.ts`'s first assertion: `HOTSEAT` folds with no refusal to `HOTSEAT_HASH`. `Ok`
/// carries the hash, `Err` what went wrong. The caller registers the cards first.
pub fn check_hotseat() -> Result<String, String> {
    let input: FoldArgs = serde_json::from_str(HOTSEAT)
        .map_err(|error| format!("the hotseat fixture is not a fold's input: {error}"))?;
    let replayed = fold(&input);
    if let Some(first) = replayed.errors.first() {
        return Err(format!(
            "the hotseat fixture's fold refused {} action(s); first at nonce \"{}\": {}",
            replayed.errors.len(),
            first.nonce,
            first.error
        ));
    }
    let hash = hash_state(&replayed.state);
    if hash != HOTSEAT_HASH {
        return Err(format!(
            "the hotseat fixture folds to {hash}, not {HOTSEAT_HASH} (SURFACE §5.2)"
        ));
    }
    Ok(hash)
}
