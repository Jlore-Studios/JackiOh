//! Turn-level beam search (SPEC §9.9): sequences of `legal_actions` played through `reduce` on one
//! determinization, the best `beamWidth` open lines kept per depth, every line closed at the end of
//! the seat's turn (or where it yields or the game ends) and scored there. `decide` scores the best
//! of them again after the opponent's reply (reply.rs), and re-scores its finalists on the other
//! determinizations with `score_line`.
//!
//! Port of `packages/ai/src/search.ts`.

use std::cmp::Ordering;

use indexmap::IndexSet;
use jackioh_engine::{ActionBody, GameState, Phase, PlayerId};

use crate::candidates::{action_key, candidate_actions};
use crate::config::AI_EVAL;
use crate::evaluate::{NextSwing, evaluate};
use crate::reply::{hidden_card_ids, reply_score};
use crate::simulate::{LineStatus, close_line, line_status, search_signature, simulate, static_score};
use crate::types::{NodeCounter, SearchBudget};

/// A complete line. `end` is the state it was scored at (after its closing endTurn), which `decide`
/// scores again after the opponent's reply without replaying the line.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub actions: Vec<ActionBody>,
    pub score: f64,
    pub status: LineStatus,
    pub end: GameState,
}

struct OpenLine {
    state: GameState,
    actions: Vec<ActionBody>,
    score: f64,
}

/// `b.score - a.score` as a comparator: best first; a NaN difference reads as a tie, as `sort` read it.
fn by_score_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

/// Best first; equal scores keep their order (`sort_by` is stable, as Array.prototype.sort is).
fn best_first_open(mut lines: Vec<OpenLine>) -> Vec<OpenLine> {
    lines.sort_by(|a, b| by_score_desc(a.score, b.score));
    lines
}

/// Best first; equal scores keep their order (`sort_by` is stable, as Array.prototype.sort is).
fn best_first_lines(mut lines: Vec<Line>) -> Vec<Line> {
    lines.sort_by(|a, b| by_score_desc(a.score, b.score));
    lines
}

/// The first `width` candidates that are not endTurn, plus endTurn when it is legal.
fn expansion(state: &GameState, seat: PlayerId, width: usize) -> Vec<ActionBody> {
    let candidates = candidate_actions(state, seat);
    let end = candidates
        .iter()
        .find(|action| matches!(action, ActionBody::EndTurn))
        .cloned();
    let mut moves: Vec<ActionBody> = candidates
        .into_iter()
        .filter(|action| !matches!(action, ActionBody::EndTurn))
        .take(width)
        .collect();
    if let Some(end) = end {
        moves.push(end);
    }
    moves
}

/// Beam search on one determinization. The frontier starts at `det`. Each open line expands its first
/// rootBranching (depth 0) or branching (deeper) candidates, plus endTurn. Every child is simulated
/// and scored with `evaluate`; children whose status is not "open" become complete lines scored by
/// terminalScore; the best beamWidth open children (stable) form the next frontier. The loop stops at
/// maxDepth (open lines are closed by terminalScore) or when the counter refuses. Returns complete
/// lines, best first.
pub fn beam_search(
    det: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
    budget: SearchBudget,
) -> Vec<Line> {
    let root_turn = det.turn;
    let mut complete: Vec<Line> = Vec::new();
    let mut frontier: Vec<OpenLine> = vec![OpenLine {
        state: det.clone(),
        actions: Vec::new(),
        score: evaluate(det, seat, NextSwing::Enemy, &AI_EVAL),
    }];
    let mut left_open: Vec<OpenLine> = Vec::new();
    let mut stopped = false;

    let mut depth = 0;
    while depth < budget.max_depth && !frontier.is_empty() {
        let mut children: Vec<OpenLine> = Vec::new();
        let mut seen: IndexSet<String> = IndexSet::new();
        let width = if depth == 0 {
            budget.root_branching
        } else {
            budget.branching
        };

        for line in &frontier {
            if stopped {
                break;
            }
            for action in expansion(&line.state, seat, width) {
                let next = match simulate(&line.state, seat, &action, counter) {
                    None => {
                        stopped = true;
                        break;
                    }
                    Some(Err(_)) => continue,
                    Some(Ok(next)) => next,
                };

                let mut actions = line.actions.clone();
                actions.push(action);
                let status = line_status(&next, seat, root_turn);
                if status != LineStatus::Open {
                    let score = static_score(&next, seat, root_turn);
                    complete.push(Line {
                        actions,
                        score,
                        status,
                        end: next,
                    });
                    continue;
                }
                // Two orders of the same moves reach the same position: keep the first, which ranks higher.
                let signature = search_signature(&next, seat);
                if seen.contains(&signature) {
                    continue;
                }
                seen.insert(signature);
                let score = evaluate(&next, seat, NextSwing::Enemy, &AI_EVAL);
                children.push(OpenLine {
                    state: next,
                    actions,
                    score,
                });
            }
        }

        if stopped {
            left_open = std::mem::take(&mut frontier);
            left_open.extend(children);
            break;
        }
        let mut next_frontier = best_first_open(children);
        next_frontier.truncate(budget.beam_width.max(1));
        frontier = next_frontier;
        depth += 1;
    }

    if !stopped {
        left_open = frontier;
    }

    // Open lines at maxDepth, or where the counter refused, are closed where they stand.
    for line in left_open {
        if line.actions.is_empty() {
            continue;
        }
        let end = close_line(&line.state, seat, root_turn, counter);
        // TS asked whether `closeLine` handed back the very state it was given (no step was taken):
        // a step always moves the state on, so equality answers the same question.
        let score = if end == line.state {
            evaluate(&end, seat, NextSwing::Enemy, &AI_EVAL)
        } else {
            static_score(&end, seat, root_turn)
        };
        let status = line_status(&end, seat, root_turn);
        complete.push(Line {
            actions: line.actions,
            score,
            status,
            end,
        });
    }

    best_first_lines(complete)
}

/// Replay `actions` on another determinization. At the first action whose action_key is not in that
/// state's candidate_actions the line is truncated. What is left ends its turn and is scored: after
/// the opponent's reply when `reply` is set (`reply_score`, with `hidden` as there, by default the
/// opponent's unseen cards in `det`), statically otherwise. `None` if the counter ran out.
pub fn score_line(
    det: &GameState,
    seat: PlayerId,
    actions: &[ActionBody],
    counter: &dyn NodeCounter,
    reply: bool,
    hidden: Option<&IndexSet<String>>,
) -> Option<f64> {
    let root_turn = det.turn;
    let mut state = det.clone();

    for action in actions {
        if line_status(&state, seat, root_turn) != LineStatus::Open {
            break;
        }
        let key = action_key(action);
        if !candidate_actions(&state, seat)
            .iter()
            .any(|candidate| action_key(candidate) == key)
        {
            break;
        }
        match simulate(&state, seat, action, counter) {
            None => return None,
            Some(Err(_)) => break,
            Some(Ok(next)) => state = next,
        }
    }

    // A line still open at the seat's main phase ends its turn here, so every finalist is scored at
    // the same point; running out of nodes for that step is running out of nodes.
    if line_status(&state, seat, root_turn) == LineStatus::Open
        && state.pending.is_none()
        && state.active == seat
        && state.phase == Phase::Main
    {
        match simulate(&state, seat, &ActionBody::EndTurn, counter) {
            None => return None,
            Some(Err(_)) => return Some(evaluate(&state, seat, NextSwing::Enemy, &AI_EVAL)),
            Some(Ok(next)) => state = next,
        }
    }

    if reply {
        let default_hidden;
        let hidden = match hidden {
            Some(hidden) => hidden,
            None => {
                default_hidden = hidden_card_ids(det, seat);
                &default_hidden
            }
        };
        reply_score(&state, seat, root_turn, counter, hidden)
    } else {
        Some(static_score(&state, seat, root_turn))
    }
}
