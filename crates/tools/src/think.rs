//! `cargo jackioh think [--file F] [--at K] [--seat p1|p2] [--budget gate|full] [--json]` (issue #834):
//! one saved game position through the AI's eyes.
//!
//! Reads `{seed, decks, log, handicaps?, …}` (`ReplayInput`: what e2e spec 13's `PracticeDebug` dumps) from
//! `--file` or stdin, folds `log[..K]` (`--at`, default the whole log; raw entries, rejects included, so
//! step 0 is the opening after `begin_game`), and prints what `seat` (default `seat_to_act`) sees there:
//! both sides of the board and its own hand as `redact` leaves them (R185), the legal-action count, the
//! `evaluate` score and `decide`'s answer: its reason, the planned line in words and the search's stats.
//! Nothing the seat cannot see is printed, so the output can be pasted into an issue.
//!
//! The decision is the AI's at the quality gates' budget, from the stream a gate match starts the seat's
//! AI on (`{seed}:ctl:{seat}` at cursor 0): the same position gives the same answer on every run. A
//! match's own cursor at that decision is not in a log, so a decision inside a game may differ from
//! this one in its determinizations. There is no `--tier`: a tier is the handicaps the input carries
//! (R180), applied when the game is created, and the AI's budget is the same at every tier (§9.9).

use std::io::Read as _;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use jackioh_ai::{
    AI_BUDGET, AI_EVAL, AI_GATE_BUDGET, AiOptions, DecisionReason, NextSwing, SearchStats, decide, evaluate,
    redact,
};
use jackioh_engine::replay::{ReplayInput, fold, hash_state};
use jackioh_engine::{
    ActionBody, GameResult, GameState, PLAYER_IDS, PlayerId, Rng, legal_actions, seat_to_act,
};

use crate::board::{EVAL_DECIMALS, board, describe, hand_names};
use crate::sweep::to_fixed;

/// `cargo jackioh think …`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The position's file (`{seed, decks, log, …}`); default stdin.
    #[arg(long, value_name = "FILE")]
    pub file: Option<PathBuf>,
    /// Fold this many log entries; default the whole log.
    #[arg(long, value_name = "K")]
    pub at: Option<usize>,
    /// The seat whose eyes to use, p1 or p2; default the seat the game waits on.
    #[arg(long, value_name = "SEAT", value_parser = ["p1", "p2"])]
    pub seat: Option<String>,
    /// `full` decides at AI_BUDGET, `gate` (the default) at AI_GATE_BUDGET.
    #[arg(long, value_parser = ["gate", "full"], default_value = "gate")]
    pub budget: String,
    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// One position and the AI's answer to it, everything from `seat`'s view. `reason`, `action`, `line`
/// and `stats` are `Decision`'s; they are absent when the seat owes no action.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Inspection {
    seed: String,
    /// The log entries folded, and how many the log holds.
    at: usize,
    log_len: usize,
    turn: i32,
    active: PlayerId,
    /// `hash_state` of the folded state, the number `replay` prints for the same log.
    hash: String,
    result: Option<GameResult>,
    seat: PlayerId,
    seat_to_act: Option<PlayerId>,
    legal_count: usize,
    /// `evaluate` of the seat's redacted state, for the seat, with the opponent to swing next.
    eval: f64,
    /// Each side of the board, p1's then p2's.
    board: [String; 2],
    /// The seat's own hand, by name.
    hand: Vec<String>,
    reason: Option<DecisionReason>,
    action: Option<ActionBody>,
    action_described: Option<String>,
    line: Vec<ActionBody>,
    line_described: Vec<String>,
    stats: Option<SearchStats>,
    /// The nonces of the folded entries the reducer refused (their messages are not shown).
    fold_errors: Vec<String>,
}

/// An action in words, without the trailing space `describe` leaves a play that has no targets.
fn in_words(state: &GameState, seat: PlayerId, action: &ActionBody) -> String {
    describe(state, seat, action).trim_end().to_string()
}

/// The stream a gate match starts `seat`'s AI on (`play_match`).
fn decision_rng(seed: &str, seat: PlayerId) -> Rng {
    Rng::new(&format!("{seed}:ctl:{seat}"), 0)
}

/// The position after `at` log entries: `fold`'s answer for the log cut short.
fn fold_at(input: &ReplayInput, at: usize) -> Result<(GameState, Vec<String>)> {
    if at > input.log.len() {
        bail!(
            "--at {at} is past the end of the log ({} entries)",
            input.log.len()
        );
    }
    let mut cut = input.clone();
    cut.log.truncate(at);
    let folded = fold(&cut);
    let nonces = folded.errors.into_iter().map(|error| error.nonce).collect();
    Ok((folded.state, nonces))
}

/// The position `input` reaches after `at` entries (all of them when `None`), and the AI's answer to
/// it for `seat` (default `seat_to_act`) at `budget`.
fn inspect(
    input: &ReplayInput,
    at: Option<usize>,
    seat: Option<PlayerId>,
    full_budget: bool,
) -> Result<Inspection> {
    let at = at.unwrap_or(input.log.len());
    let (state, fold_errors) = fold_at(input, at)?;
    let Some(seat) = seat.or_else(|| seat_to_act(&state)) else {
        bail!("no seat owes an action at entry {at}; name one with --seat");
    };
    let budget = if full_budget { AI_BUDGET } else { AI_GATE_BUDGET };
    let view = redact(&state, seat);
    let eval = evaluate(&view, seat, NextSwing::Enemy, &AI_EVAL);
    let mut options = AiOptions {
        rng: decision_rng(&input.seed, seat),
        budget,
        should_stop: None,
    };
    let decision = decide(&state, seat, &mut options);
    let (reason, action, line, stats) = match decision {
        Some(decision) => (
            Some(decision.reason),
            Some(decision.action),
            decision.line,
            Some(decision.stats),
        ),
        None => (None, None, Vec::new(), None),
    };
    Ok(Inspection {
        seed: input.seed.clone(),
        at,
        log_len: input.log.len(),
        turn: state.turn,
        active: state.active,
        hash: hash_state(&state),
        result: state.result,
        seat,
        seat_to_act: seat_to_act(&state),
        legal_count: legal_actions(&state, seat).len(),
        eval,
        board: PLAYER_IDS.map(|p| board(&view, p)),
        hand: hand_names(&view, seat),
        reason,
        action_described: action.as_ref().map(|action| in_words(&view, seat, action)),
        action,
        line_described: line.iter().map(|action| in_words(&view, seat, action)).collect(),
        line,
        stats,
        fold_errors,
    })
}

/// The inspection as text, one block.
fn render(found: &Inspection) -> String {
    let mut out = format!(
        "seed {} at {}/{} turn {} active {} seat {} hash {}\n",
        found.seed, found.at, found.log_len, found.turn, found.active, found.seat, found.hash
    );
    for side in &found.board {
        out.push_str(&format!("   {side}\n"));
    }
    out.push_str(&format!("   hand({}): {}\n", found.seat, found.hand.join(", ")));
    if let Some(result) = &found.result {
        out.push_str(&format!(
            "result {}\n",
            serde_json::to_string(result).unwrap_or_default()
        ));
    }
    let to_act = found
        .seat_to_act
        .map_or_else(|| "none".to_string(), |p| p.to_string());
    out.push_str(&format!(
        "seatToAct {to_act} legal {} eval {}\n",
        found.legal_count,
        to_fixed(found.eval, EVAL_DECIMALS)
    ));
    match (&found.reason, &found.action_described, &found.stats) {
        (Some(reason), Some(action), Some(stats)) => {
            out.push_str(&format!("decision {}: {action}\n", reason_text(*reason)));
            out.push_str(&format!("   line: {}\n", found.line_described.join(" | ")));
            out.push_str(&format!(
                "   stats nodes {} determinizations {} lines {} simErrors {} stoppedBy {} score {}\n",
                stats.nodes,
                stats.determinizations,
                stats.lines,
                stats.sim_errors,
                stats.stopped_by.as_str(),
                to_fixed(stats.score, EVAL_DECIMALS)
            ));
        }
        _ => out.push_str(&format!("decision none: {} owes no action\n", found.seat)),
    }
    if !found.fold_errors.is_empty() {
        out.push_str(&format!(
            "foldErrors {}: {}\n",
            found.fold_errors.len(),
            found.fold_errors.join(", ")
        ));
    }
    out
}

/// A reason as the JSON spells it.
fn reason_text(reason: DecisionReason) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// `cargo jackioh think`.
pub fn run(args: Args) -> Result<()> {
    jackioh_cards::register_all();
    let text = match &args.file {
        Some(path) => std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
        None => {
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .context("reading stdin")?;
            text
        }
    };
    let input: ReplayInput = serde_json::from_str(&text).context("the input is not {seed, decks, log, …}")?;
    let seat = match args.seat.as_deref() {
        Some(seat) => Some(
            serde_json::from_value::<PlayerId>(serde_json::Value::String(seat.to_string()))
                .with_context(|| format!("--seat {seat:?} is not p1 or p2"))?,
        ),
        None => None,
    };
    let found = inspect(&input, args.at, seat, args.budget == "full")?;
    if args.json {
        println!("{}", serde_json::to_string(&found)?);
    } else {
        print!("{}", render(&found));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_ai::{HIDDEN_DEF_ID, StoppedBy};
    use jackioh_engine::zones::active_units_of;
    use serde_json::{Value, json};

    /// The hotseat game e2e spec 01 recorded (SURFACE §13): 37 log entries, 9 turns.
    const HOTSEAT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../engine/tests/golden/01-hotseat-full-game.json"
    );
    /// A mid-game position in it: turn 4, p2 to act with four cards in hand, a search of 327 nodes.
    const MID_GAME: usize = 13;

    fn hotseat() -> ReplayInput {
        jackioh_cards::register_all();
        serde_json::from_str(&std::fs::read_to_string(HOTSEAT).unwrap()).unwrap()
    }

    /// Whether `text` holds `word` on its own, not inside a longer name.
    fn mentions(text: &str, word: &str) -> bool {
        text.match_indices(word).any(|(start, _)| {
            let before = text[..start].chars().next_back();
            let after = text[start + word.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
        })
    }

    #[test]
    fn think_is_deterministic_mid_game() {
        let input = hotseat();
        let first = inspect(&input, Some(MID_GAME), None, false).unwrap();
        let second = inspect(&input, Some(MID_GAME), None, false).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert_eq!(first.at, MID_GAME);
        assert!(first.turn >= 1, "turn {}", first.turn);
        assert_eq!(first.seat, PlayerId::P2);
        assert_eq!(first.reason, Some(DecisionReason::Search));
        let stats = first.stats.expect("a search has stats");
        assert!(stats.nodes > 0);
        assert!(matches!(
            stats.stopped_by,
            StoppedBy::Exhausted | StoppedBy::Budget | StoppedBy::Clock
        ));
        assert!(!first.line.is_empty());
        assert_eq!(first.line.len(), first.line_described.len());
        assert_eq!(first.action.as_ref(), first.line.first());
        assert!(first.legal_count > 0);
        assert!(first.fold_errors.is_empty());
    }

    #[test]
    fn think_hides_what_the_seat_cannot_see() {
        let input = hotseat();
        // (def id, name) of the cards of `p` that are in its hand or backrow or on its field, and of its
        // library too when asked: the think never prints a library.
        let names = |state: &GameState, p: PlayerId, library: bool| -> Vec<(String, String)> {
            let side = &state.players[p];
            let library = if library { side.library.as_slice() } else { &[] };
            let cards = side
                .hand
                .iter()
                .chain(library)
                .chain(side.backrow.iter().flatten())
                .chain(active_units_of(state, p));
            cards
                .map(|card| (card.def_id.clone(), crate::board::name_of(state, &card.def_id)))
                .collect()
        };
        for (at, seat) in [
            (MID_GAME, PlayerId::P1),
            (MID_GAME, PlayerId::P2),
            (24, PlayerId::P1),
        ] {
            let (state, _) = fold_at(&input, at).unwrap();
            let view = redact(&state, seat);
            // What the seat may read: every card the redacted state still names, on either side.
            let known: Vec<String> = PLAYER_IDS
                .iter()
                .flat_map(|p| names(&view, *p, false))
                .filter(|(def_id, _)| def_id != HIDDEN_DEF_ID)
                .map(|(_, name)| name)
                .collect();
            let hidden: Vec<(String, String)> = names(&state, seat.opponent(), true)
                .into_iter()
                .filter(|(_, name)| !known.contains(name))
                .collect();
            assert!(!hidden.is_empty(), "nothing hidden at {at} for {seat}");

            let found = inspect(&input, Some(at), Some(seat), false).unwrap();
            let outputs = [render(&found), serde_json::to_string(&found).unwrap()];
            for output in &outputs {
                for (def_id, name) in &hidden {
                    assert!(
                        !mentions(output, name),
                        "{name} shown to {seat} at {at}:\n{output}"
                    );
                    assert!(
                        !mentions(output, def_id),
                        "{def_id} shown to {seat} at {at}:\n{output}"
                    );
                }
                // Not vacuous: the seat's own hand is named.
                assert!(
                    found.hand.iter().all(|name| mentions(output, name)),
                    "{:?} missing from:\n{output}",
                    found.hand
                );
            }
            assert!(!found.hand.is_empty());
        }
    }

    #[test]
    fn think_at_zero_is_the_opening_and_past_the_end_is_an_error() {
        let input = hotseat();
        let opening = inspect(&input, Some(0), None, false).unwrap();
        assert_eq!(opening.at, 0);
        assert_eq!(opening.log_len, input.log.len());
        assert_eq!(opening.turn, 0);
        assert_eq!(opening.reason, Some(DecisionReason::Mulligan));

        let whole = inspect(&input, None, None, false).unwrap();
        assert_eq!(whole.at, input.log.len());
        assert_eq!(whole.hash, "5c3d578c");
        assert!(whole.result.is_some());
        assert_eq!(whole.reason, None);
        assert!(render(&whole).contains("decision none"));

        let error = inspect(&input, Some(input.log.len() + 1), None, false).unwrap_err();
        assert!(format!("{error}").contains("past the end"), "{error}");
    }

    #[test]
    fn think_text_names_the_position_the_legal_count_the_score_and_the_decision() {
        let found = inspect(&hotseat(), Some(MID_GAME), None, false).unwrap();
        let text = render(&found);
        for needle in [
            "seed 01-hotseat at 13/37 turn 4 active p2 seat p2",
            "hand(p2): ",
            "seatToAct p2 legal ",
            " eval ",
            "decision search: ",
            "   line: ",
            "stats nodes ",
            "stoppedBy ",
        ] {
            assert!(text.contains(needle), "{needle:?} missing from:\n{text}");
        }
        assert!(!text.contains("foldErrors"));
        assert_eq!(text.lines().filter(|line| line.starts_with("   p")).count(), 2);
    }

    #[test]
    fn think_lists_a_refused_entry_by_its_nonce_alone() {
        jackioh_cards::register_all();
        let mut value: Value = serde_json::from_str(&std::fs::read_to_string(HOTSEAT).unwrap()).unwrap();
        value["log"].as_array_mut().unwrap().push(json!({
            "type": "attack", "attackerId": "no-such-card", "targetId": "hero-p2", "playerId": "p1", "nonce": "refused-1"
        }));
        let input: ReplayInput = serde_json::from_value(value).unwrap();
        let found = inspect(&input, None, None, false).unwrap();
        assert_eq!(found.fold_errors, vec!["refused-1".to_string()]);
        assert_eq!(found.hash, "5c3d578c");
        assert!(render(&found).contains("foldErrors 1: refused-1"));
    }

    #[test]
    fn think_runs_on_a_file_and_refuses_a_seat_that_is_not_one() {
        let args = |seat: Option<&str>| Args {
            file: Some(PathBuf::from(HOTSEAT)),
            at: Some(MID_GAME),
            seat: seat.map(str::to_string),
            budget: "gate".to_string(),
            json: true,
        };
        run(args(Some("p1"))).unwrap();
        assert!(run(args(Some("p3"))).is_err());
    }
}
