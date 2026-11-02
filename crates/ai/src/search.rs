//! Turn-level beam search (SPEC §9.9): sequences of `legal_actions` played through `reduce` on one
//! determinization, the best `beamWidth` open lines kept per depth, every line closed at the end of
//! the seat's turn (or where it yields or the game ends) and scored there. `decide` scores the best
//! of them again after the opponent's reply (reply.rs), and re-scores its finalists on the other
//! determinizations with `score_line`.

use std::cmp::Ordering;

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::{ActionBody, GameState, Phase, PlayerId};

use crate::candidates::{
    candidate_actions, candidate_set, root_candidate_actions, root_candidate_set, stands_down,
};
use crate::config::{AI_MOVE_ORDER, AI_SEARCH, MoveOrder};
use crate::evaluate::{NextSwing, Prices};
use crate::learned::{finalist_score, line_score, open_score};
use crate::policy::{POLICY_PRIOR, PolicyPrior, prior_order};
use crate::reply::{hidden_card_ids, reply_score};
use crate::simulate::{
    AnswersTried, LineStatus, best_auto_answer, close_line, line_status, own_prompt_score, search_signature,
    simulate, static_score, unit_lanes,
};
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

/// `b.score - a.score` as a comparator: best first; a NaN difference reads as a tie.
fn by_score_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

/// Best first; equal scores keep their order (`sort_by` is stable).
fn best_first_open(mut lines: Vec<OpenLine>) -> Vec<OpenLine> {
    lines.sort_by(|a, b| by_score_desc(a.score, b.score));
    lines
}

/// Best first; equal scores keep their order (`sort_by` is stable).
fn best_first_lines(mut lines: Vec<Line>) -> Vec<Line> {
    lines.sort_by(|a, b| by_score_desc(a.score, b.score));
    lines
}

/// The first `width` candidates that are not endTurn, then the first `AI_SEARCH.position_switches`
/// stand-downs (`stands_down`) the cut left out (#1759, R1759000), then endTurn when it is legal. With
/// `MoveOrder::Learned` the candidates are sorted by `prior` before the first `width` are taken
/// (`prior_order`, R1171001); with `MoveOrder::Heuristic` they keep `candidate_actions`' order.
pub fn expansion(
    state: &GameState,
    seat: PlayerId,
    width: usize,
    order: MoveOrder,
    prior: &PolicyPrior<'_>,
    prices: &Prices,
) -> Vec<ActionBody> {
    expand(
        candidate_actions(state, seat, prices),
        state,
        seat,
        width,
        order,
        prior,
    )
}

/// `expansion` at the search's root (#1635): of `root_candidate_actions`, which keeps every distinct
/// placement of a play.
pub fn root_expansion(
    state: &GameState,
    seat: PlayerId,
    width: usize,
    order: MoveOrder,
    prior: &PolicyPrior<'_>,
    prices: &Prices,
) -> Vec<ActionBody> {
    expand(
        root_candidate_actions(state, seat, prices),
        state,
        seat,
        width,
        order,
        prior,
    )
}

/// `expansion` of the given candidates.
fn expand(
    candidates: Vec<ActionBody>,
    state: &GameState,
    seat: PlayerId,
    width: usize,
    order: MoveOrder,
    prior: &PolicyPrior<'_>,
) -> Vec<ActionBody> {
    let end = candidates
        .iter()
        .find(|action| matches!(action, ActionBody::EndTurn))
        .cloned();
    let stand_downs: Vec<ActionBody> = candidates
        .iter()
        .filter(|action| stands_down(state, action))
        .cloned()
        .collect();
    let mut moves: Vec<ActionBody> = candidates
        .into_iter()
        .filter(|action| !matches!(action, ActionBody::EndTurn))
        .collect();
    if order == MoveOrder::Learned {
        moves = prior_order(state, seat, moves, prior);
    }
    moves.truncate(width);
    let left_out: Vec<ActionBody> = stand_downs
        .into_iter()
        .filter(|stand_down| !moves.contains(stand_down))
        .take(AI_SEARCH.position_switches.max(0) as usize)
        .collect();
    moves.extend(left_out);
    if let Some(end) = end {
        moves.push(end);
    }
    moves
}

/// Beam search on one determinization expanding `root_expansion` (#1635) then `expansion` (R1171001).
/// Scores intermediate states (`own_prompt_score` #1112), drops transposition duplicates, tracks
/// lane variants (#1797), and returns complete lines best first.
pub fn beam_search(
    det: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
    budget: SearchBudget,
    prices: &Prices,
) -> Vec<Line> {
    beam_search_with(det, seat, counter, budget, AI_MOVE_ORDER, &POLICY_PRIOR, prices)
}

/// `beam_search` with a given move order and policy prior, for tests and screens (R1171001).
pub fn beam_search_with(
    det: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
    budget: SearchBudget,
    order: MoveOrder,
    prior: &PolicyPrior<'_>,
    prices: &Prices,
) -> Vec<Line> {
    let root_turn = det.turn;
    let mut complete: Vec<Line> = Vec::new();
    let mut frontier: Vec<OpenLine> = vec![OpenLine {
        state: det.clone(),
        actions: Vec::new(),
        score: open_score(det, seat, NextSwing::Enemy, prices),
    }];
    let mut left_open: Vec<OpenLine> = Vec::new();
    // #1797: the lane variants of the children the beam kept, closed after it (each a state and its line).
    let mut lane_variants: Vec<(GameState, Vec<ActionBody>)> = Vec::new();
    let mut stopped = false;

    let mut depth = 0;
    while depth < budget.max_depth && !frontier.is_empty() {
        let mut children: Vec<OpenLine> = Vec::new();
        // #1797: each open child's position with its units' lanes; how many children each position
        // without them (`search_signature`) has had; and its lane variants, by that signature.
        let mut seen: IndexSet<(String, String)> = IndexSet::new();
        let mut placements: IndexMap<String, usize> = IndexMap::new();
        let mut variants: Vec<(String, GameState, Vec<ActionBody>)> = Vec::new();
        let width = if depth == 0 {
            budget.root_branching
        } else {
            budget.branching
        };

        for line in &frontier {
            if stopped {
                break;
            }
            let moves = if depth == 0 {
                root_expansion(&line.state, seat, width, order, prior, prices)
            } else {
                expansion(&line.state, seat, width, order, prior, prices)
            };
            for action in moves {
                let next = match simulate(&line.state, seat, &action, counter, prices) {
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
                    let score = static_score(&next, seat, root_turn, prices);
                    complete.push(Line {
                        actions,
                        score,
                        status,
                        end: next,
                    });
                    continue;
                }
                // Two orders of the same moves into the same lanes reach the same position: keep the
                // first, which ranks higher.
                let signature = search_signature(&next, seat);
                if !seen.insert((signature.clone(), unit_lanes(&next))) {
                    continue;
                }
                // #1797: a later child that differs from its position's first only in where its units
                // stand is a lane variant. The evaluation reads no lane, so it would score alike and
                // fill the beam; it is played no further but closed beside the first, so the reply can
                // tell the lanes apart. At most AI_SEARCH.zone_variants of a position, the first included.
                let placed = placements.entry(signature.clone()).or_insert(0);
                *placed += 1;
                if *placed > 1 {
                    if *placed <= AI_SEARCH.zone_variants.max(1) as usize {
                        variants.push((signature, next, actions));
                    }
                    continue;
                }
                // #1112: a child waiting on the seat's own prompt is ranked where its answers lead.
                let score = own_prompt_score(&next, seat, counter, prices)
                    .unwrap_or_else(|| open_score(&next, seat, NextSwing::Enemy, prices));
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
            lane_variants.extend(variants.into_iter().map(|(_, state, actions)| (state, actions)));
            break;
        }
        let mut next_frontier = best_first_open(children);
        next_frontier.truncate(budget.beam_width.max(1));
        let kept: IndexSet<String> = next_frontier
            .iter()
            .map(|line| search_signature(&line.state, seat))
            .collect();
        lane_variants.extend(
            variants
                .into_iter()
                .filter(|(signature, _, _)| kept.contains(signature))
                .map(|(_, state, actions)| (state, actions)),
        );
        frontier = next_frontier;
        depth += 1;
    }

    if !stopped {
        left_open = frontier;
    }

    // Open lines at maxDepth, or where the counter refused, are closed where they stand; then the lane
    // variants (#1797), after every line the beam would close without them, so a tie goes to the first
    // lane.
    let open_lines = left_open
        .into_iter()
        .map(|line| (line.state, line.actions))
        .chain(lane_variants);
    for (state, actions) in open_lines {
        if actions.is_empty() {
            continue;
        }
        let end = close_line(&state, seat, root_turn, counter, prices);
        // Equality means `close_line` took no step: a step always moves the state on.
        let score = if end == state {
            line_score(&end, seat, NextSwing::Enemy, prices)
        } else {
            static_score(&end, seat, root_turn, prices)
        };
        let status = line_status(&end, seat, root_turn);
        complete.push(Line {
            actions,
            score,
            status,
            end,
        });
    }

    best_first_lines(complete)
}

/// Replay `actions` on another determinization (`root_candidate_set`, #1635). If the line diverges
/// at an own-prompt, auto-answers up to `max_auto_answers` (#1636). Closes turn and scores either
/// with `reply_score` or statically.
pub fn score_line(
    det: &GameState,
    seat: PlayerId,
    actions: &[ActionBody],
    counter: &dyn NodeCounter,
    reply: bool,
    hidden: Option<&IndexSet<String>>,
    prices: &Prices,
) -> Option<f64> {
    let root_turn = det.turn;
    let mut state = det.clone();
    let mut answered: usize = 0;
    let mut at = 0;

    while let Some(action) = actions.get(at) {
        if line_status(&state, seat, root_turn) != LineStatus::Open {
            break;
        }
        // Membership asks for the candidates' set only (candidate_set), not their move order, and an
        // action is its own key, so `==` on the body is the test.
        let candidates = if at == 0 {
            root_candidate_set(&state, seat)
        } else {
            candidate_set(&state, seat)
        };
        if !candidates.iter().any(|candidate| candidate == action) {
            // #1636: the seat's own prompt offers what the line's world did not: answer it here and
            // play on. Anything else ends the line where it stands.
            let own_prompt = state
                .pending
                .as_ref()
                .is_some_and(|pending| pending.player_id == seat);
            if !own_prompt || answered >= AI_SEARCH.max_auto_answers as usize {
                break;
            }
            match best_auto_answer(&state, seat, AnswersTried::Drawn, counter, "line", prices)? {
                Err(_) => break,
                Ok(next) => state = next,
            }
            answered += 1;
            if matches!(action, ActionBody::Answer { .. }) {
                at += 1;
            }
            continue;
        }
        match simulate(&state, seat, action, counter, prices) {
            None => return None,
            Some(Err(_)) => break,
            Some(Ok(next)) => state = next,
        }
        at += 1;
    }

    // A line still open at the seat's main phase ends its turn here, so every finalist is scored at
    // the same point; running out of nodes for that step is running out of nodes.
    if line_status(&state, seat, root_turn) == LineStatus::Open
        && state.pending.is_none()
        && state.active == seat
        && state.phase == Phase::Main
    {
        match simulate(&state, seat, &ActionBody::EndTurn, counter, prices) {
            None => return None,
            // A finalist whose endTurn is refused keeps its static score, from the evaluation that
            // scores the finalists when they are replied to (#1010).
            Some(Err(_)) if reply => return Some(finalist_score(&state, seat, NextSwing::Enemy, prices)),
            Some(Err(_)) => return Some(line_score(&state, seat, NextSwing::Enemy, prices)),
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
        reply_score(&state, seat, root_turn, counter, hidden, prices)
    } else {
        Some(static_score(&state, seat, root_turn, prices))
    }
}
