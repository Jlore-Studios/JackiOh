//! Turn-level beam search (SPEC §9.9): sequences of `legal_actions` played through `reduce` on one
//! determinization, the best `beamWidth` open lines kept per depth, every line closed at the end of
//! the seat's turn (or where it yields or the game ends) and scored there. `decide` scores the best
//! of them again after the opponent's reply (reply.rs), and re-scores its finalists on the other
//! determinizations with `replay_line`.

use std::cmp::Ordering;

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::{ActionBody, GameState, Phase, PlayerId};

use crate::candidates::{
    candidate_actions, candidate_set, root_candidate_actions, root_candidate_set, stands_down,
    wall_switches_priced,
};
use crate::config::{AI_EVAL, AI_SEARCH};
use crate::evaluate::{NextSwing, Prices, evaluate_priced};
use crate::reply::{hidden_card_ids, reply_score};
use crate::simulate::{
    AnswersTried, LineStatus, best_auto_answer, close_line, create_sub_counter, lane_free_signature,
    line_status, own_prompt_score, search_signature, simulate, static_score, unit_lanes,
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
/// stand-downs (`stands_down`) the cut left out (#1759, R1759000), then, while the opponent's board
/// threatens lethal, the first `AI_SEARCH.position_switches` switches to Defense that lower that threat
/// that both left out (`wall_switches_priced`, #2976, R2976000), then endTurn when it is legal. The
/// candidates keep `candidate_actions`' order (R4159000).
pub fn expansion(state: &GameState, seat: PlayerId, width: usize, prices: &Prices) -> Vec<ActionBody> {
    expand(candidate_actions(state, seat, prices), state, seat, width, prices)
}

/// `expansion` at the search's root (#1635): of `root_candidate_actions`, which keeps every distinct
/// placement of a play.
pub fn root_expansion(state: &GameState, seat: PlayerId, width: usize, prices: &Prices) -> Vec<ActionBody> {
    expand(
        root_candidate_actions(state, seat, prices),
        state,
        seat,
        width,
        prices,
    )
}

/// `expansion` of the given candidates.
fn expand(
    candidates: Vec<ActionBody>,
    state: &GameState,
    seat: PlayerId,
    width: usize,
    prices: &Prices,
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
    let walls = wall_switches_priced(state, seat, &candidates, prices);
    let mut moves: Vec<ActionBody> = candidates
        .into_iter()
        .filter(|action| !matches!(action, ActionBody::EndTurn))
        .collect();
    moves.truncate(width);
    let left_out: Vec<ActionBody> = stand_downs
        .into_iter()
        .filter(|stand_down| !moves.contains(stand_down))
        .take(AI_SEARCH.position_switches.max(0) as usize)
        .collect();
    moves.extend(left_out);
    let walls_left_out: Vec<ActionBody> = walls
        .into_iter()
        .filter(|wall| !moves.contains(wall))
        .take(AI_SEARCH.position_switches.max(0) as usize)
        .collect();
    moves.extend(walls_left_out);
    if let Some(end) = end {
        moves.push(end);
    }
    moves
}

/// Beam search on one determinization expanding `root_expansion` (#1635) then `expansion`.
/// Scores intermediate states (`own_prompt_score` #1112), drops transposition duplicates, tracks
/// lane variants (#1797, R4132000), and returns complete lines best first.
pub fn beam_search(
    det: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
    budget: SearchBudget,
    prices: &Prices,
) -> Vec<Line> {
    let root_turn = det.turn;
    let mut complete: Vec<Line> = Vec::new();
    let mut frontier: Vec<OpenLine> = vec![OpenLine {
        state: det.clone(),
        actions: Vec::new(),
        score: evaluate_priced(det, seat, NextSwing::Enemy, &AI_EVAL, prices),
    }];
    let mut left_open: Vec<OpenLine> = Vec::new();
    // #1797: the lane variants of the children the beam kept, closed after it (each a state and its line).
    let mut lane_variants: Vec<(GameState, Vec<ActionBody>)> = Vec::new();
    let mut stopped = false;

    let mut depth = 0;
    while depth < budget.max_depth && !frontier.is_empty() {
        let mut children: Vec<OpenLine> = Vec::new();
        // #1797: each open child's position with its units' lanes; how many children each position
        // without where its cards stand (`lane_free_signature`, R4132000) has had; and its lane variants,
        // by that signature.
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
                root_expansion(&line.state, seat, width, prices)
            } else {
                expansion(&line.state, seat, width, prices)
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
                if !seen.insert((signature, unit_lanes(&next))) {
                    continue;
                }
                // #1797: a later child that differs from its position's first only in where its cards
                // stand (its units' lanes, which side of a standing unit a new one goes, a backrow card's
                // lane: R4132000) is a lane variant. The evaluation reads no lane, so it would score alike
                // and fill the beam; it is played no further but closed beside the first, so the reply can
                // tell the lanes apart. At most AI_SEARCH.zone_variants of a position, the first included.
                let lane_free = lane_free_signature(&next, seat);
                let placed = placements.entry(lane_free.clone()).or_insert(0);
                *placed += 1;
                if *placed > 1 {
                    if *placed <= AI_SEARCH.zone_variants.max(1) as usize {
                        variants.push((lane_free, next, actions));
                    }
                    continue;
                }
                // #1112: a child waiting on the seat's own prompt is ranked where its answers lead.
                let score = own_prompt_score(&next, seat, counter, prices)
                    .unwrap_or_else(|| evaluate_priced(&next, seat, NextSwing::Enemy, &AI_EVAL, prices));
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
            .map(|line| lane_free_signature(&line.state, seat))
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
    // lane. R3498000: a wrap-up that refused the beam's exploring lets these endTurns through, so that the
    // lines it cut short end their turn and can be replied to; a counter without one ignores this.
    counter.end_exploring();
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
            evaluate_priced(&end, seat, NextSwing::Enemy, &AI_EVAL, prices)
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

/// R4447000: how a finalist's replay on another determinization plays on past the first step that world
/// does not offer: at most `steps` greedy steps (`fill_turn`), each trying `width` candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayFill {
    pub steps: usize,
    pub width: usize,
}

impl ReplayFill {
    /// No fill: the replay ends its turn at the cut, as before R4447000.
    pub const OFF: ReplayFill = ReplayFill { steps: 0, width: 0 };

    /// `AI_SEARCH.replay_fill_steps` steps of `width` candidates (the decision's `branching`).
    pub fn configured(width: usize) -> ReplayFill {
        ReplayFill {
            steps: AI_SEARCH.replay_fill_steps.max(0) as usize,
            width,
        }
    }

    /// The most nodes one replay's fill may take, `steps × (width + 1)`; 0 when off.
    pub fn ceiling(self) -> usize {
        self.steps * (self.width + 1)
    }
}

/// R4447000: a replayed line's score, whether a step its world does not offer cut it, and the state it
/// was scored at (after its closing endTurn, before the opponent's reply).
#[derive(Clone, Debug, PartialEq)]
pub struct LineReplay {
    pub score: f64,
    pub cut: bool,
    pub closed: GameState,
}

/// Whether a line stands open in the seat's main phase with nothing pending.
fn open_in_main(state: &GameState, seat: PlayerId, root_turn: i32) -> bool {
    line_status(state, seat, root_turn) == LineStatus::Open
        && state.pending.is_none()
        && state.active == seat
        && state.phase == Phase::Main
}

/// R4447000: a cut replay's turn played on greedily: at most `fill.steps` steps, each the best by
/// `static_score` of the first `fill.width` candidates that are not endTurn (`candidate_actions`' move
/// order), taken only when it scores above standing still. A step leaving the seat's own prompt open is
/// not tried. Every node comes from a slice of `fill.ceiling()`; a step the slice or the counter cannot
/// finish is not taken, and the replay's closing endTurn then meets the counter as it stands.
fn fill_turn(
    mut state: GameState,
    seat: PlayerId,
    root_turn: i32,
    counter: &dyn NodeCounter,
    fill: ReplayFill,
    prices: &Prices,
) -> GameState {
    let slice = create_sub_counter(counter, fill.ceiling());
    for _ in 0..fill.steps {
        if !open_in_main(&state, seat, root_turn) {
            break;
        }
        let mut best_score = static_score(&state, seat, root_turn, prices);
        let mut best: Option<GameState> = None;
        let moves = candidate_actions(&state, seat, prices)
            .into_iter()
            .filter(|action| !matches!(action, ActionBody::EndTurn))
            .take(fill.width);
        for action in moves {
            let next = match simulate(&state, seat, &action, &slice, prices) {
                None => return state,
                Some(Err(_)) => continue,
                Some(Ok(next)) => next,
            };
            if next
                .pending
                .as_ref()
                .is_some_and(|pending| pending.player_id == seat)
            {
                continue;
            }
            let score = static_score(&next, seat, root_turn, prices);
            // Strictly above: a tie keeps standing still, or the earlier candidate.
            if score > best_score {
                best_score = score;
                best = Some(next);
            }
        }
        match best {
            Some(next) => state = next,
            None => break,
        }
    }
    state
}

/// Replay `actions` on another determinization (`root_candidate_set`, #1635). If the line diverges
/// at an own-prompt, auto-answers up to `max_auto_answers` (#1636). Closes turn and scores either
/// with `reply_score` or statically. R4447000: reports whether a step its world does not offer cut the
/// line, and with `fill` on plays that world's turn on (`fill_turn`); `reply` is `Some(hidden)` to score
/// after the opponent's reply.
pub fn replay_line(
    det: &GameState,
    seat: PlayerId,
    actions: &[ActionBody],
    counter: &dyn NodeCounter,
    reply: Option<&IndexSet<String>>,
    prices: &Prices,
    fill: ReplayFill,
) -> Option<LineReplay> {
    let root_turn = det.turn;
    let mut state = det.clone();
    let mut answered: usize = 0;
    let mut at = 0;
    let mut cut = false;

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
                cut = true;
                break;
            }
            match best_auto_answer(&state, seat, AnswersTried::Drawn, counter, "line", prices)? {
                Err(_) => {
                    cut = true;
                    break;
                }
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

    // R4447000: a cut line plays its world's turn on, when the fill is on.
    if cut && fill.steps > 0 {
        state = fill_turn(state, seat, root_turn, counter, fill, prices);
    }

    // A line still open at the seat's main phase ends its turn here, so every finalist is scored at
    // the same point; running out of nodes for that step is running out of nodes.
    if open_in_main(&state, seat, root_turn) {
        match simulate(&state, seat, &ActionBody::EndTurn, counter, prices) {
            None => return None,
            // A finalist whose endTurn is refused keeps its static score, the hand eval's.
            Some(Err(_)) => {
                let score = evaluate_priced(&state, seat, NextSwing::Enemy, &AI_EVAL, prices);
                return Some(LineReplay {
                    score,
                    cut,
                    closed: state,
                });
            }
            Some(Ok(next)) => state = next,
        }
    }

    let score = match reply {
        Some(hidden) => reply_score(&state, seat, root_turn, counter, hidden, prices)?,
        None => static_score(&state, seat, root_turn, prices),
    };
    Some(LineReplay {
        score,
        cut,
        closed: state,
    })
}

/// `replay_line` with the fill off (R4447000): cut at the first step its world does not offer, whatever
/// `AI_SEARCH.replay_fill_steps` is. `hidden` defaults to `det`'s hidden cards when `reply` is set.
pub fn score_line(
    det: &GameState,
    seat: PlayerId,
    actions: &[ActionBody],
    counter: &dyn NodeCounter,
    reply: bool,
    hidden: Option<&IndexSet<String>>,
    prices: &Prices,
) -> Option<f64> {
    let default_hidden;
    let reply = match (reply, hidden) {
        (false, _) => None,
        (true, Some(hidden)) => Some(hidden),
        (true, None) => {
            default_hidden = hidden_card_ids(det, seat);
            Some(&default_hidden)
        }
    };
    replay_line(det, seat, actions, counter, reply, prices, ReplayFill::OFF).map(|replay| replay.score)
}
