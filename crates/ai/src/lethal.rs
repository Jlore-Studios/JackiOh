//! The lethal solver (SPEC §9.9): bounded search for a line winning this turn across all determinizations.
//!
//! Stages:
//! 1. Depth-first walk in move order for `AI_SEARCH.lethalQuickNodes`.
//! 2. Best-first walk ordered by `ready_gap` (enemy hero life including Duos pool R964000 less ready attacks).
//!
//! Proved lines are verified across all determinizations and trimmed (`trim_lethal`, R1965000).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use indexmap::IndexSet;
use jackioh_engine::{
    ActionBody, AttackTarget, CardInstance, ExertionKind, GameState, KeywordKind, Phase, PlayerId, Rng,
    active_units_of, attack_restriction, attacks_per_turn, can_attack, can_lose, effective_life,
    find_instance, has_exertion, is_sick, legal_actions, unit_has, unit_view,
};

use crate::candidates::candidate_actions;
use crate::config::{AI_SEARCH, LETHAL_HINT_DETERMINIZATIONS, LETHAL_HINT_NODES, LETHAL_HINT_SEED};
use crate::determinize::{DeterminizeOptions, det_ground, determinize_on};
use crate::evaluate::{Prices, damage_past_taunts, deft_in_defense};
use crate::observe::redact;
use crate::simulate::{
    LineStatus, create_node_counter, create_sub_counter, line_status, search_signature, simulate,
};
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

/// The actions the solver tries from a position: never ending the turn, and no position switch but
/// a Deft unit's to Attack while its attack is unspent (`deft_ready_switch`).
fn lethal_moves(state: &GameState, seat: PlayerId, prices: &Prices) -> Vec<ActionBody> {
    candidate_actions(state, seat, prices)
        .into_iter()
        .filter(|action| match action {
            ActionBody::EndTurn => false,
            ActionBody::SwitchPosition { instance_id } => deft_ready_switch(state, instance_id),
            _ => true,
        })
        .collect()
}

/// R49: whether `instance_id` is a Deft unit in Defense Position whose attack is unspent. Its switch
/// to Attack spends only the switch's exertion, so it can still attack this turn.
pub(crate) fn deft_ready_switch(state: &GameState, instance_id: &str) -> bool {
    find_instance(state, instance_id).is_some_and(|unit| {
        deft_in_defense(&unit_view(state, unit)) && has_exertion(state, unit, ExertionKind::Attack)
    })
}

fn push_moves(
    stack: &mut Vec<Frame>,
    state: Rc<GameState>,
    line: Rc<Vec<ActionBody>>,
    seat: PlayerId,
    prices: &Prices,
) {
    let moves = lethal_moves(&state, seat, prices);
    // Reversed, so the first move in move order is the first one popped.
    for action in moves.into_iter().rev() {
        stack.push(Frame {
            state: Rc::clone(&state),
            line: Rc::clone(&line),
            action,
        });
    }
}

/// Replays a lethal line on each of `worlds`: each action must be legal there (equality with
/// legal_actions, ActionBody being plain data) and the line must end with `seat` the winner. `None`
/// when the counter ran out. The walks prove a line on the determinizations but the first, where
/// they found it.
fn wins_on(
    worlds: &[GameState],
    seat: PlayerId,
    line: &[ActionBody],
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Option<bool> {
    for det in worlds {
        let mut state = det.clone();
        for action in line {
            if state.result.is_some() {
                break;
            }
            if !legal_actions(&state, seat).iter().any(|legal| legal == action) {
                return Some(false);
            }
            match simulate(&state, seat, action, counter, prices) {
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

/// Whether `legal_actions` lists an attack for `unit` (`combat::can_attack`).
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

/// Whether `unit` may hit enemy hero once Taunts clear (§4.2, Gauntlet R1849005, Charge/Rush §6.1).
fn may_hit_hero(state: &GameState, unit: &CardInstance) -> bool {
    let hero = AttackTarget::Hero {
        player: unit.controller.opponent(),
    };
    attack_restriction(state, unit, &hero).is_ok()
        && (!is_sick(state, unit) || unit_has(state, unit, KeywordKind::Charge))
}

/// R636: the attacks `unit` may still declare this turn, `attacks_per_turn` less those it has declared
/// (`exertion.attacks`, or 1 when it has only `attacked`): two for a fresh Windfury unit, one after
/// its first swing.
fn attacks_left(state: &GameState, unit: &CardInstance) -> i32 {
    let spent = unit
        .exertion
        .attacks
        .unwrap_or(if unit.exertion.attacked { 1 } else { 0 });
    attacks_per_turn(state, unit) - spent
}

/// Gap to lethal: enemy hero life (including Duos pool R964000) less damage past Taunts (#1823).
pub fn ready_gap(state: &GameState, seat: PlayerId) -> f64 {
    if let Some(result) = &state.result {
        return if result.winner.player() == Some(seat) {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let opp = seat.opponent();
    // R2201000: a seat that cannot lose stands infinitely far from lethal.
    if !can_lose(state, opp) {
        return f64::INFINITY;
    }
    let life = effective_life(state, opp);
    // legal_actions lists no attack while a prompt is open or outside the seat's own main phase.
    if state.pending.is_some() || state.active != seat || state.phase != Phase::Main {
        return f64::from(life);
    }
    // Fused scripts are built from state on lookup (SURFACE §6.6).
    let mut attacks: Vec<(&CardInstance, i32, bool)> = Vec::new();
    for unit in active_units_of(state, seat) {
        let attack = unit_view(state, unit).attack;
        if attack > 0 && has_attack(state, unit) {
            let hits_hero = may_hit_hero(state, unit);
            for _ in 0..attacks_left(state, unit) {
                attacks.push((unit, attack, hits_hero));
            }
        }
    }
    f64::from(life - damage_past_taunts(state, opp, &attacks))
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
    prices: &Prices,
) -> Option<Verdict> {
    if let Some(result) = &next.result {
        if result.winner.player() != Some(seat) {
            return Some(Verdict::Dead);
        }
        let verdict = wins_on(dets.get(1..).unwrap_or_default(), seat, line, counter, prices)?;
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
fn depth_first(dets: &[GameState], seat: PlayerId, counter: &dyn NodeCounter, prices: &Prices) -> Walk {
    let Some(root) = dets.first() else {
        return Walk::Exhausted;
    };
    let mut visited: IndexSet<String> = IndexSet::new();
    visited.insert(search_signature(root, seat));
    let mut stack: Vec<Frame> = Vec::new();
    push_moves(
        &mut stack,
        Rc::new(root.clone()),
        Rc::new(Vec::new()),
        seat,
        prices,
    );

    while let Some(frame) = stack.pop() {
        let next = match simulate(&frame.state, seat, &frame.action, counter, prices) {
            None => return Walk::Cut,
            Some(Err(_)) => continue,
            Some(Ok(next)) => next,
        };
        let mut line: Vec<ActionBody> = frame.line.as_ref().clone();
        line.push(frame.action.clone());
        let Some(verdict) = judge(dets, seat, &line, &next, root.turn, &mut visited, counter, prices) else {
            return Walk::Cut;
        };
        match verdict {
            Verdict::Lethal => return Walk::Line(line),
            Verdict::Deeper => push_moves(&mut stack, Rc::new(next), Rc::new(line), seat, prices),
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
fn best_first(dets: &[GameState], seat: PlayerId, counter: &dyn NodeCounter, prices: &Prices) -> Walk {
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
        for action in lethal_moves(&node.state, seat, prices).into_iter().take(width) {
            let next = match simulate(&node.state, seat, &action, counter, prices) {
                None => return Walk::Cut,
                Some(Err(_)) => continue,
                Some(Ok(next)) => next,
            };
            let mut line = node.line.clone();
            line.push(action);
            let Some(verdict) = judge(dets, seat, &line, &next, root.turn, &mut visited, counter, prices)
            else {
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

/// Runs depth-first then best-first lethal search on dets[0], requiring candidate lines
/// to win on all determinizations. Spends at most `limit` nodes of `counter`.
pub fn find_lethal(
    dets: &[GameState],
    seat: PlayerId,
    counter: &dyn NodeCounter,
    limit: usize,
    prices: &Prices,
) -> Option<Vec<ActionBody>> {
    find_lethal_with_quick_nodes(
        dets,
        seat,
        counter,
        limit,
        AI_SEARCH.lethal_quick_nodes as usize,
        prices,
    )
}

/// `find_lethal` with a configurable depth-first quick-node budget.
pub fn find_lethal_with_quick_nodes(
    dets: &[GameState],
    seat: PlayerId,
    counter: &dyn NodeCounter,
    limit: usize,
    quick_nodes: usize,
    prices: &Prices,
) -> Option<Vec<ActionBody>> {
    let root = dets.first()?;
    // R2201000: there is no lethal on a seat that cannot lose, so nothing is searched for one.
    if limit == 0 || !can_lose(root, seat.opponent()) {
        return None;
    }
    if line_status(root, seat, root.turn) != LineStatus::Open {
        return None;
    }

    let start = counter.used();
    let first = {
        let quick = create_sub_counter(counter, limit.min(quick_nodes));
        depth_first(dets, seat, &quick, prices)
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
        best_first(dets, seat, &rest, prices)
    };
    match second {
        Walk::Line(line) => Some(line),
        Walk::Exhausted | Walk::Cut => None,
    }
}

/// R1965000: trim `line` to the actions it needs so unnecessary free plays (e.g. C #90 In Too Deep,
/// R454) do not stall `decide`. Finds the shortest winning suffix across all worlds, then greedily
/// drops unneeded prefix actions.
pub fn trim_lethal(
    dets: &[GameState],
    seat: PlayerId,
    mut line: Vec<ActionBody>,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Option<Vec<ActionBody>> {
    for start in (1..line.len()).rev() {
        if wins_on(dets, seat, &line[start..], counter, prices)? {
            line.drain(..start);
            break;
        }
    }
    // The line less its first action does not win: the suffixes above included it.
    let mut at = 1;
    while at < line.len() {
        let mut without = line.clone();
        without.remove(at);
        match wins_on(dets, seat, &without, counter, prices) {
            None => break,
            Some(true) => {
                line = without;
                at = 0;
            }
            Some(false) => at += 1,
        }
    }
    Some(line)
}

/// R2102000: practice AI lethal hint (SPEC §9.9). Like `decide` (R185), samples determinizations
/// from `redact(state, seat)` and checks if `find_lethal` finds a win across all worlds within budget.
pub fn lethal_hint(state: &GameState, seat: PlayerId, should_stop: Option<&dyn Fn() -> bool>) -> bool {
    let own_main = state.active == seat && state.phase == Phase::Main;
    if state.result.is_some() || state.pending.is_some() || !own_main {
        return false;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let public = redact(state, seat);
        let ground = det_ground(&public, seat);
        let mut rng = Rng::new(LETHAL_HINT_SEED, 0);
        let worlds: Vec<GameState> = (0..LETHAL_HINT_DETERMINIZATIONS)
            .map(|_| determinize_on(&public, seat, &mut rng, DeterminizeOptions::default(), &ground))
            .collect();
        let counter = create_node_counter(LETHAL_HINT_NODES, should_stop);
        find_lethal(&worlds, seat, &counter, LETHAL_HINT_NODES, Prices::NONE).is_some()
    }))
    .unwrap_or(false)
}
