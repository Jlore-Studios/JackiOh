//! The lethal solver (SPEC §9.9): a bounded search for a line that wins this turn on every
//! determinization, through attack orderings, removing Taunt first, buffs before attacks, a spell to
//! the face, and prompts answered on the way. Nothing in this crate recurses, so both walks keep
//! their own explicit frontier. Two stages share one node allowance:
//!   1. Depth-first in move order (`candidate_actions` puts face attacks and winning trades first) for
//!      AI_SEARCH.lethalQuickNodes nodes. If it runs out of moves before nodes, the whole tree has
//!      been searched and there is no lethal.
//!   2. Otherwise best-first with what is left: it expands the position closest to lethal (`ready_gap`),
//!      trying its first AI_SEARCH.lethalWidth moves at once, so a lethal that starts with a card late
//!      in move order is found on a wide board, where depth-first spends everything below its first
//!      move. A move past the first lethalWidth of its position is never tried by either walk.
//! A line counts only when it wins on every determinization.

use std::rc::Rc;

use indexmap::IndexSet;
use jackioh_engine::{
    ActionBody, AttackTarget, CardInstance, ExertionKind, GameState, Phase, PlayerId, active_units_of,
    can_attack, has_exertion, legal_actions, unit_view,
};

use crate::candidates::{action_key, candidate_actions};
use crate::config::AI_SEARCH;
use crate::evaluate::damage_past_taunts;
use crate::simulate::{LineStatus, create_sub_counter, line_status, search_signature, simulate};
use crate::types::{NodeCounter, StoppedBy};

struct Frame {
    state: Rc<GameState>,
    line: Rc<Vec<ActionBody>>,
    action: ActionBody,
}

/// A position the best-first walk may expand: the line that reached it and how far it is from lethal.
struct Open {
    state: Rc<GameState>,
    line: Vec<ActionBody>,
    gap: f64,
    order: u32,
}

/// What a walk ended with: a lethal line, a tree searched to the end, or a walk the counter cut short.
enum Walk {
    Line(Vec<ActionBody>),
    Exhausted,
    Cut,
}

/// What a new position means for the walk (`judge`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Lethal,
    Deeper,
    Dead,
}

/// The actions the solver tries from a position: no position switches, and never ending the turn.
fn lethal_moves(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    candidate_actions(state, seat)
        .into_iter()
        .filter(|action| !matches!(action, ActionBody::SwitchPosition { .. } | ActionBody::EndTurn))
        .collect()
}

fn push_moves(stack: &mut Vec<Frame>, state: Rc<GameState>, line: Rc<Vec<ActionBody>>, seat: PlayerId) {
    let moves = lethal_moves(&state, seat);
    // Reversed, so the first move in move order is the first one popped.
    for action in moves.into_iter().rev() {
        stack.push(Frame {
            state: Rc::clone(&state),
            line: Rc::clone(&line),
            action,
        });
    }
}

/// Replays a lethal line on every determinization but the first: each action must be legal there
/// (action_key equality with legal_actions) and the line must end with `seat` the winner. `None` when
/// the counter ran out.
fn holds_everywhere(
    dets: &[GameState],
    seat: PlayerId,
    line: &[ActionBody],
    counter: &dyn NodeCounter,
) -> Option<bool> {
    for det in dets.iter().skip(1) {
        let mut state = det.clone();
        for action in line {
            if state.result.is_some() {
                break;
            }
            let key = action_key(action);
            if !legal_actions(&state, seat)
                .iter()
                .any(|legal| action_key(legal) == key)
            {
                return Some(false);
            }
            match simulate(&state, seat, action, counter) {
                None => return None,
                Some(Err(_)) => return Some(false),
                Some(Ok(next)) => state = next,
            }
        }
        if state
            .result
            .is_none_or(|result| result.winner.player() != Some(seat))
        {
            return Some(false);
        }
    }
    Some(true)
}

/// Whether `legal_actions` would list an attack for `unit`, which `seat` controls, in a position
/// where `seat` may act in its main phase: some target passes `combat::can_attack`, the engine's own
/// filter. Asked unit by unit, a spent attack ruled out first, because enumerating every legal action
/// cost the best-first walk more than simulating its moves did.
fn has_attack(state: &GameState, unit: &CardInstance) -> bool {
    if !has_exertion(state, unit, ExertionKind::Attack) {
        return false;
    }
    let enemy = unit.controller.opponent();
    if can_attack(state, unit, &AttackTarget::Hero { player: enemy }) {
        return true;
    }
    active_units_of(state, enemy).into_iter().any(|instance| {
        can_attack(
            state,
            unit,
            &AttackTarget::Unit {
                instance: instance.clone(),
            },
        )
    })
}

/// How far the seat stands from lethal this turn: the enemy hero's health less what the units that
/// may still attack (those `legal_actions` lists an attack for) would deal it past the enemy's Taunts,
/// counted as `face_threat` counts it. Zero or less means the attacks left could end the game. It
/// only orders the search; a lethal is always proved by playing it through `reduce`.
pub fn ready_gap(state: &GameState, seat: PlayerId) -> f64 {
    if let Some(result) = &state.result {
        return if result.winner.player() == Some(seat) {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let opp = seat.opponent();
    let health = state.players[opp].hero.health;
    // legal_actions lists no attack while a prompt is open or outside the seat's own main phase.
    if state.pending.is_some() || state.active != seat || state.phase != Phase::Main {
        return f64::from(health);
    }
    // Fused scripts are built from the state on every lookup (§6.6), so no sync is needed here.
    let mut attacks: Vec<i32> = Vec::new();
    for unit in active_units_of(state, seat) {
        let attack = unit_view(state, unit).attack;
        if attack > 0 && has_attack(state, unit) {
            attacks.push(attack);
        }
    }
    f64::from(health - damage_past_taunts(state, opp, &attacks))
}

/// What a new position means for the walk: a proved lethal, a position to search on from, or neither.
/// `None` when the counter ran out while proving a win on the other determinizations.
#[allow(clippy::too_many_arguments)]
fn judge(
    dets: &[GameState],
    seat: PlayerId,
    line: &[ActionBody],
    next: &GameState,
    root_turn: i32,
    visited: &mut IndexSet<String>,
    counter: &dyn NodeCounter,
) -> Option<Verdict> {
    if let Some(result) = &next.result {
        if result.winner.player() != Some(seat) {
            return Some(Verdict::Dead);
        }
        let verdict = holds_everywhere(dets, seat, line, counter)?;
        return Some(if verdict { Verdict::Lethal } else { Verdict::Dead });
    }
    if line.len() as i32 >= AI_SEARCH.lethal_max_depth {
        return Some(Verdict::Dead);
    }
    if line_status(next, seat, root_turn) != LineStatus::Open {
        return Some(Verdict::Dead);
    }
    let signature = search_signature(next, seat);
    if visited.contains(&signature) {
        return Some(Verdict::Dead);
    }
    visited.insert(signature);
    Some(Verdict::Deeper)
}

/// Stage 1: depth-first in move order, with a visited set on `search_signature`.
fn depth_first(dets: &[GameState], seat: PlayerId, counter: &dyn NodeCounter) -> Walk {
    let Some(root) = dets.first() else {
        return Walk::Exhausted;
    };
    let mut visited: IndexSet<String> = IndexSet::new();
    visited.insert(search_signature(root, seat));
    let mut stack: Vec<Frame> = Vec::new();
    push_moves(&mut stack, Rc::new(root.clone()), Rc::new(Vec::new()), seat);

    while let Some(frame) = stack.pop() {
        let next = match simulate(&frame.state, seat, &frame.action, counter) {
            None => return Walk::Cut,
            Some(Err(_)) => continue,
            Some(Ok(next)) => next,
        };
        let mut line: Vec<ActionBody> = frame.line.as_ref().clone();
        line.push(frame.action.clone());
        let Some(verdict) = judge(dets, seat, &line, &next, root.turn, &mut visited, counter) else {
            return Walk::Cut;
        };
        match verdict {
            Verdict::Lethal => return Walk::Line(line),
            Verdict::Deeper => push_moves(&mut stack, Rc::new(next), Rc::new(line), seat),
            Verdict::Dead => {}
        }
    }
    Walk::Exhausted
}

/// Index of the open position to expand next: the smallest gap, then the longest line, then the oldest.
fn next_open(open: &[Open]) -> usize {
    let mut best = 0usize;
    for i in 1..open.len() {
        let (Some(a), Some(b)) = (open.get(i), open.get(best)) else {
            continue;
        };
        let closer = a.gap < b.gap;
        let deeper = a.gap == b.gap && a.line.len() > b.line.len();
        let older = a.gap == b.gap && a.line.len() == b.line.len() && a.order < b.order;
        if closer || deeper || older {
            best = i;
        }
    }
    best
}

/// Stage 2: best-first by `ready_gap`, each expansion simulating its first AI_SEARCH.lethalWidth moves.
fn best_first(dets: &[GameState], seat: PlayerId, counter: &dyn NodeCounter) -> Walk {
    let Some(root) = dets.first() else {
        return Walk::Exhausted;
    };
    let mut visited: IndexSet<String> = IndexSet::new();
    visited.insert(search_signature(root, seat));
    let mut open: Vec<Open> = vec![Open {
        state: Rc::new(root.clone()),
        line: Vec::new(),
        gap: ready_gap(root, seat),
        order: 0,
    }];
    let mut order: u32 = 1;

    while !open.is_empty() {
        let node = open.remove(next_open(&open));
        let width = AI_SEARCH.lethal_width.max(0) as usize;
        for action in lethal_moves(&node.state, seat).into_iter().take(width) {
            let next = match simulate(&node.state, seat, &action, counter) {
                None => return Walk::Cut,
                Some(Err(_)) => continue,
                Some(Ok(next)) => next,
            };
            let mut line = node.line.clone();
            line.push(action);
            let Some(verdict) = judge(dets, seat, &line, &next, root.turn, &mut visited, counter) else {
                return Walk::Cut;
            };
            match verdict {
                Verdict::Lethal => return Walk::Line(line),
                Verdict::Deeper => {
                    let gap = ready_gap(&next, seat);
                    open.push(Open {
                        state: Rc::new(next),
                        line,
                        gap,
                        order,
                    });
                    order += 1;
                }
                Verdict::Dead => {}
            }
        }
    }
    Walk::Exhausted
}

/// The lethal solver on dets[0]: `lethal_moves`, lines at most AI_SEARCH.lethalMaxDepth long, a visited
/// set keyed on `search_signature`. Depth-first gets AI_SEARCH.lethalQuickNodes; if it neither found a
/// lethal nor searched the whole tree, best-first by `ready_gap` gets the rest. A line is returned only
/// if it also wins on every other determinization; spends at most `limit` nodes of `counter`.
pub fn find_lethal(
    dets: &[GameState],
    seat: PlayerId,
    counter: &dyn NodeCounter,
    limit: usize,
) -> Option<Vec<ActionBody>> {
    find_lethal_with_quick_nodes(dets, seat, counter, limit, AI_SEARCH.lethal_quick_nodes as usize)
}

/// `find_lethal` with the depth-first walk's share given, so a test can set it at run time
/// (`tests/ai/lethal.rs`).
pub fn find_lethal_with_quick_nodes(
    dets: &[GameState],
    seat: PlayerId,
    counter: &dyn NodeCounter,
    limit: usize,
    quick_nodes: usize,
) -> Option<Vec<ActionBody>> {
    let root = dets.first()?;
    if limit == 0 {
        return None;
    }
    if line_status(root, seat, root.turn) != LineStatus::Open {
        return None;
    }

    let start = counter.used();
    let first = {
        let quick = create_sub_counter(counter, limit.min(quick_nodes));
        depth_first(dets, seat, &quick)
    };
    match first {
        Walk::Line(line) => return Some(line),
        Walk::Exhausted => return None,
        Walk::Cut => {}
    }
    if counter.stopped_by() != StoppedBy::Exhausted {
        return None;
    }

    let left = limit.saturating_sub(counter.used() - start);
    if left == 0 {
        return None;
    }
    let second = {
        let rest = create_sub_counter(counter, left);
        best_first(dets, seat, &rest)
    };
    match second {
        Walk::Line(line) => Some(line),
        Walk::Exhausted | Walk::Cut => None,
    }
}
