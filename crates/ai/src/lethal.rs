//! The lethal solver (SPEC §9.9): bounded search for a line winning this turn across all determinizations.
//!
//! Stages:
//! 1. Depth-first walk in move order for `AI_SEARCH.lethalQuickNodes`. A step that slice cannot pay the
//!    surcharge of (`UNPAID_SURCHARGE`), walked or replayed on another determinization, leaves the walk
//!    cut, not exhausted, so stage 2 still runs (R4807000).
//! 2. Best-first walk ordered by `ready_gap` (enemy hero life including Duos pool R964000 less ready attacks).
//!    R5119000: at a gap above 0 it keeps its attackers ready (`Gaps::taunt_free`), and a child whose dive
//!    at gaps no closer than its parent's has tried more than `AI_SEARCH.lethal_dive_moves` moves is set
//!    aside, so the positions below it come after the others of their gap.
//!
//! Each walk's visited set reads `position_key`, `search_signature` with the units' lanes (R4806000), so a
//! play tried in two lanes reaches two positions.
//!
//! Proved lines are verified across all determinizations and trimmed (`trim_lethal`, R1965000).
//!
//! R4788000: a decision whose allowance was scaled down hands the walk a reach past it
//! (`find_lethal_reaching`), spent only from a position that already reads as lethal and on a won line's proof.

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
    LineStatus, UNPAID_SURCHARGE, create_node_counter, create_sub_counter, line_status, position_key,
    simulate,
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
    /// R5119000: the gap were no Taunt standing (`Gaps::taunt_free`).
    taunt_free: f64,
    /// R5119000: the position's `Branch` in the walk.
    branch: usize,
    order: u32,
}

/// R5119000: a position the best-first walk opened, as the moves tried below it count against it.
struct Branch {
    /// The branch of the position it was reached from; the root's is its own.
    parent: usize,
    gap: f64,
    /// The moves the walk has tried at it and below it from positions no closer than its parent.
    dive: usize,
    /// Set once `dive` passes `AI_SEARCH.lethal_dive_moves`: the positions below it come after the
    /// others of their gap.
    aside: bool,
}

/// R5119000: whether `branch` or a branch above it is set aside.
fn below_aside(branches: &[Branch], mut branch: usize) -> bool {
    while let Some(at) = branches.get(branch) {
        if at.aside {
            return true;
        }
        if at.parent == branch {
            return false;
        }
        branch = at.parent;
    }
    false
}

/// R5119000: counts an expansion's `tried` moves, made at `gap`, against `branch` and each branch above it
/// whose parent stands no closer than `gap`, and sets aside each one whose count passes `dive_moves`.
fn charge_dive(branches: &mut [Branch], mut branch: usize, gap: f64, tried: usize, dive_moves: usize) {
    while let Some(parent) = branches.get(branch).map(|at| at.parent) {
        if parent == branch {
            return;
        }
        let flat = branches.get(parent).is_some_and(|up| gap >= up.gap);
        if flat && let Some(at) = branches.get_mut(branch) {
            at.dive += tried;
            at.aside |= at.dive > dive_moves;
        }
        branch = parent;
    }
}

/// What a walk ended with: a lethal line, a tree searched to the end, or a walk the counter cut short
/// or that skipped a step its slice could not pay for (R4807000).
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
    /// R4807000: the line wins here, but a step of it on another determinization has a surcharge the
    /// counter's slice cannot pay, so it is neither proved nor refuted there.
    Unpaid,
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
/// they found it. `Lethal` when it wins on every world, `Unpaid` when a step's surcharge does not fit
/// `counter`'s slice (`UNPAID_SURCHARGE`, R4807000), else `Dead`.
fn wins_on(
    worlds: &[GameState],
    seat: PlayerId,
    line: &[ActionBody],
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Option<Verdict> {
    for det in worlds {
        let mut state = det.clone();
        for action in line {
            if state.result.is_some() {
                break;
            }
            if !legal_actions(&state, seat).iter().any(|legal| legal == action) {
                return Some(Verdict::Dead);
            }
            match simulate(&state, seat, action, counter, prices) {
                None => return None,
                Some(Err(error)) if error == UNPAID_SURCHARGE => return Some(Verdict::Unpaid),
                Some(Err(_)) => return Some(Verdict::Dead),
                Some(Ok(next)) => state = next,
            }
        }
        if state
            .result
            .is_none_or(|result| result.winner.player() != Some(seat))
        {
            return Some(Verdict::Dead);
        }
    }
    Some(Verdict::Lethal)
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
    gaps(state, seat).ready
}

/// `ready_gap` and, R5119000, the same gap were no Taunt standing.
struct Gaps {
    ready: f64,
    /// The enemy hero's life less every ready attack that may hit it once no Taunt walls it
    /// (`may_hit_hero`): what spending an attacker on a Taunt the gap already counted it against gives
    /// up. It reads the attacks through an open prompt, where `ready` does not: `ready` is what the
    /// seat can deal from this position, and `legal_actions` lists no attack until the prompt is
    /// answered, but a play that opens a prompt is one step split in two and answering spends no attack,
    /// so the attackers the line has not spent are still the ones it counts.
    taunt_free: f64,
}

/// `ready_gap`, with `Gaps::taunt_free` beside it from the same ready attackers.
fn gaps(state: &GameState, seat: PlayerId) -> Gaps {
    let both = |gap: f64| Gaps {
        ready: gap,
        taunt_free: gap,
    };
    if let Some(result) = &state.result {
        return both(if result.winner.player() == Some(seat) {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    let opp = seat.opponent();
    // R2201000: a seat that cannot lose stands infinitely far from lethal.
    if !can_lose(state, opp) {
        return both(f64::INFINITY);
    }
    let life = effective_life(state, opp);
    // legal_actions lists no attack outside the seat's own main phase.
    if state.active != seat || state.phase != Phase::Main {
        return both(f64::from(life));
    }
    // Fused scripts are built from state on lookup (SURFACE §6.6).
    let mut attacks: Vec<(&CardInstance, i32, bool)> = Vec::new();
    let mut free = 0;
    // R4792000: while a Mayor acts each attack's target is drawn (R1200), so one counts only when the
    // hero is the only target it may draw.
    let drawn =
        crate::config::AI_REPLY.drawn_attacks && jackioh_engine::random_targets::targets_random(state);
    for unit in active_units_of(state, seat) {
        let attack = unit_view(state, unit).attack;
        if attack > 0 && has_attack(state, unit) {
            let hits_hero = may_hit_hero(state, unit);
            if hits_hero {
                free += attack * attacks_left(state, unit).max(0);
            }
            if !drawn || crate::candidates::draws_only_hero(state, unit) {
                for _ in 0..attacks_left(state, unit) {
                    attacks.push((unit, attack, hits_hero));
                }
            }
        }
    }
    // Nor while a prompt is open.
    let ready = if state.pending.is_some() {
        life
    } else {
        life - damage_past_taunts(state, opp, &attacks)
    };
    Gaps {
        ready: f64::from(ready),
        taunt_free: f64::from(life - free),
    }
}

/// What a new position means for the walk: a proved lethal, a position to search on from, a lethal the
/// slice cannot pay to prove (`Unpaid`), or neither. `None` when the counter ran out while proving a win
/// on the other determinizations.
#[expect(
    clippy::too_many_arguments,
    reason = "the walk's position and its counters, each borrowed apart"
)]
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
        return wins_on(dets.get(1..).unwrap_or_default(), seat, line, counter, prices);
    }
    if line.len() as i32 >= AI_SEARCH.lethal_max_depth {
        return Some(Verdict::Dead);
    }
    if line_status(next, seat, root_turn) != LineStatus::Open {
        return Some(Verdict::Dead);
    }
    if !visited.insert(position_key(next, seat)) {
        return Some(Verdict::Dead);
    }
    Some(Verdict::Deeper)
}

/// Stage 1: depth-first in move order, with a visited set on `position_key` (R4806000). A step whose
/// surcharge the slice cannot pay, here or in a replay on another determinization, is skipped like a
/// refused one, but the walk then ends `Cut`: it has not searched what lies past that step, and the
/// best-first walk may pay for it (R4807000).
fn depth_first(dets: &[GameState], seat: PlayerId, counter: &dyn NodeCounter, prices: &Prices) -> Walk {
    let root = &dets[0]; // `find_lethal` answered an empty slice itself.
    let mut visited: IndexSet<String> = IndexSet::new();
    visited.insert(position_key(root, seat));
    let mut stack: Vec<Frame> = Vec::new();
    push_moves(
        &mut stack,
        Rc::new(root.clone()),
        Rc::new(Vec::new()),
        seat,
        prices,
    );

    let mut unpaid = false;
    while let Some(frame) = stack.pop() {
        let next = match simulate(&frame.state, seat, &frame.action, counter, prices) {
            None => return Walk::Cut,
            Some(Err(error)) => {
                unpaid |= error == UNPAID_SURCHARGE;
                continue;
            }
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
            Verdict::Unpaid => unpaid = true,
            Verdict::Dead => {}
        }
    }
    if unpaid { Walk::Cut } else { Walk::Exhausted }
}

/// Index of the open position to expand next: the smallest gap, then (R5119000) one below no set-aside
/// branch, then the longest line, then (R5119000, with `keep_attacks`, while that gap is above 0) the
/// smallest `taunt_free` gap, then the oldest.
fn next_open(open: &[Open], branches: &[Branch], keep_attacks: bool) -> usize {
    let mut best = 0usize;
    let mut best_aside = open
        .first()
        .is_some_and(|first| below_aside(branches, first.branch));
    for i in 1..open.len() {
        let (Some(a), Some(b)) = (open.get(i), open.get(best)) else {
            continue;
        };
        let a_aside = below_aside(branches, a.branch);
        let closer = a.gap < b.gap;
        let kept = a.gap == b.gap && !a_aside && best_aside;
        let tied = a.gap == b.gap && a_aside == best_aside;
        let deeper = tied && a.line.len() > b.line.len();
        let level = tied && a.line.len() == b.line.len();
        let by_free = level && keep_attacks && a.gap > 0.0;
        let freer = by_free && a.taunt_free < b.taunt_free;
        let older = level && (!by_free || a.taunt_free == b.taunt_free) && a.order < b.order;
        if closer || kept || deeper || freer || older {
            best = i;
            best_aside = a_aside;
        }
    }
    best
}

/// Stage 2: best-first by `ready_gap`, each expansion simulating its first AI_SEARCH.lethalWidth moves.
/// R5119000: a child whose dive at gaps no closer than its parent's has tried more than
/// `AI_SEARCH.lethal_dive_moves` moves is set aside, and while the gap is above 0 the walk keeps its
/// attackers ready (`next_open`). R4788000: `near` is the walk's allowance. Once it is spent the walk goes on only from a position whose
/// ready attacks already reach lethal (`ready_gap` at or under 0), on `far`, which also pays for every
/// proof of a line that won on determinization 0 (`find_lethal_reaching`). With `far` no wider than `near`
/// the walk stops where `near` does.
fn best_first(
    dets: &[GameState],
    seat: PlayerId,
    near: &dyn NodeCounter,
    far: &dyn NodeCounter,
    prices: &Prices,
) -> Walk {
    let root = &dets[0]; // `find_lethal` answered an empty slice itself.
    let mut visited: IndexSet<String> = IndexSet::new();
    visited.insert(position_key(root, seat));
    let root_gaps = gaps(root, seat);
    let mut open: Vec<Open> = vec![Open {
        state: Rc::new(root.clone()),
        line: Vec::new(),
        gap: root_gaps.ready,
        taunt_free: root_gaps.taunt_free,
        branch: 0,
        order: 0,
    }];
    let mut branches: Vec<Branch> = vec![Branch {
        parent: 0,
        gap: root_gaps.ready,
        dive: 0,
        aside: false,
    }];
    let mut order: u32 = 1;
    // R5119000: a negative bound sets nothing aside.
    let dive_moves = usize::try_from(AI_SEARCH.lethal_dive_moves).unwrap_or(usize::MAX);

    let mut near_spent = false;
    while !open.is_empty() {
        let node = open.remove(next_open(&open, &branches, AI_SEARCH.lethal_keep_attacks));
        // `next_open` takes the smallest gap first: when this one does not reach, none left does.
        let reaches = node.gap <= 0.0;
        if near_spent && !reaches {
            return Walk::Cut;
        }
        let counter = if reaches { far } else { near };
        let width = AI_SEARCH.lethal_width.max(0) as usize;
        let mut tried = 0usize;
        for action in lethal_moves(&node.state, seat, prices).into_iter().take(width) {
            tried += 1;
            let next = match simulate(&node.state, seat, &action, counter, prices) {
                None if reaches => return Walk::Cut,
                None => {
                    // The allowance is spent: the children this expansion already made may still reach.
                    near_spent = true;
                    break;
                }
                Some(Err(_)) => continue,
                Some(Ok(next)) => next,
            };
            let mut line = node.line.clone();
            line.push(action);
            let Some(verdict) = judge(dets, seat, &line, &next, root.turn, &mut visited, far, prices) else {
                return Walk::Cut;
            };
            match verdict {
                Verdict::Lethal => return Walk::Line(line),
                Verdict::Deeper => {
                    let next_gaps = gaps(&next, seat);
                    branches.push(Branch {
                        parent: node.branch,
                        gap: next_gaps.ready,
                        dive: 0,
                        aside: false,
                    });
                    open.push(Open {
                        state: Rc::new(next),
                        line,
                        gap: next_gaps.ready,
                        taunt_free: next_gaps.taunt_free,
                        branch: branches.len() - 1,
                        order,
                    });
                    order += 1;
                }
                Verdict::Dead | Verdict::Unpaid => {}
            }
        }
        charge_dive(&mut branches, node.branch, node.gap, tried, dive_moves);
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
    walks(dets, seat, counter, limit, counter, limit, quick_nodes, prices)
}

/// R4788000: `find_lethal` for a decision whose allowance was scaled down (decide.rs `scaled_budget`): it spends at
/// most `limit` nodes of `counter`, and past them goes on, up to `reach` nodes in all and on `beyond`'s
/// nodes, only while the best-first walk stands at a position whose ready attacks already reach lethal
/// (`ready_gap` at or under 0) or proves on the other determinizations a line that won on the first. With
/// `reach` at or under `limit` it is `find_lethal` on `counter`, node for node.
pub fn find_lethal_reaching(
    dets: &[GameState],
    seat: PlayerId,
    counter: &dyn NodeCounter,
    limit: usize,
    beyond: &dyn NodeCounter,
    reach: usize,
    prices: &Prices,
) -> Option<Vec<ActionBody>> {
    walks(
        dets,
        seat,
        counter,
        limit,
        beyond,
        reach,
        AI_SEARCH.lethal_quick_nodes as usize,
        prices,
    )
}

/// Both walks: the depth-first one on `quick_nodes` of `limit`, then the best-first one on the rest of
/// `limit`, drawn from `counter`, and past it on what `reach` leaves of `beyond` (`best_first`).
#[expect(
    clippy::too_many_arguments,
    reason = "the two walks' counters and allowances, each borrowed apart"
)]
fn walks(
    dets: &[GameState],
    seat: PlayerId,
    counter: &dyn NodeCounter,
    limit: usize,
    beyond: &dyn NodeCounter,
    reach: usize,
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

    let spent = counter.used() - start;
    let left = limit.saturating_sub(spent);
    if left == 0 {
        return None;
    }
    let second = {
        let rest = create_sub_counter(counter, left);
        if reach > limit {
            let far = create_sub_counter(beyond, reach.saturating_sub(spent));
            best_first(dets, seat, &rest, &far, prices)
        } else {
            best_first(dets, seat, &rest, &rest, prices)
        }
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
        if wins_on(dets, seat, &line[start..], counter, prices)? == Verdict::Lethal {
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
            Some(Verdict::Lethal) => {
                line = without;
                at = 0;
            }
            Some(_) => at += 1,
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

#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::{json, scenario};

    use super::*;
    use crate::config::AI_BUDGET;
    use crate::simulate::surcharge_nodes;

    const P1: PlayerId = PlayerId::P1;
    /// #8 Mr. Vanilla, a 4/4 with no keywords.
    const VANILLA: &str = "core-008";
    /// #96 My Pawn: an attack it would answer costs `AI_SEARCH.ai_turn_trap_nodes` more (R4809001).
    const MY_PAWN: &str = "core-096";
    /// #7 Jewelosco Scarab, a (1) Unit whose Cry opens a Discover.
    const SCARAB: &str = "core-007";

    /// p1 on turn 9 with Mr. Vanilla on the field since before this turn and nothing in hand, against a
    /// hero at `health`; with `pawn`, p2 has My Pawn face-down in its backrow. p1's ids come first, so
    /// the two worlds share them.
    fn vanilla_into(health: i32, pawn: bool) -> GameState {
        jackioh_cards::register_all();
        let backrow = if pawn { json!([MY_PAWN]) } else { json!([]) };
        scenario(json!({
            "seed": "lethal-r4807000", "active": P1, "turn": 9,
            "p1": { "field": [VANILLA], "library": [VANILLA, VANILLA] },
            "p2": { "health": health, "backrow": backrow, "library": [VANILLA, VANILLA] },
        }))
        .state()
        .clone()
    }

    /// The Vanilla's attack on the enemy hero.
    fn swing(state: &GameState) -> ActionBody {
        let attacker_id = active_units_of(state, P1)
            .first()
            .map(|unit| unit.id.clone())
            .unwrap_or_default();
        ActionBody::Attack {
            attacker_id,
            target_id: format!("hero-{}", P1.opponent().as_str()),
        }
    }

    /// The first walk's slice, as `find_lethal` gives it.
    fn quick() -> impl NodeCounter {
        create_node_counter(AI_SEARCH.lethal_quick_nodes as usize, None)
    }

    #[test]
    fn r4807000_an_attack_the_quick_slice_cannot_pay_for_cuts_the_first_walk() {
        let trapped = vanilla_into(3, true);
        assert!(
            1 + surcharge_nodes(&trapped, P1, &swing(&trapped)) > AI_SEARCH.lethal_quick_nodes as usize,
            "the attack's surcharge fits the quick slice"
        );
        assert!(
            matches!(
                depth_first(std::slice::from_ref(&trapped), P1, &quick(), Prices::NONE),
                Walk::Cut
            ),
            "a walk that skipped a step it could not pay for read as a searched tree"
        );
    }

    #[test]
    fn r4807000_a_replay_the_quick_slice_cannot_pay_for_on_another_world_cuts_the_first_walk() {
        let worlds = [vanilla_into(3, false), vanilla_into(3, true)];
        let line = [swing(&worlds[0])];
        assert_eq!(swing(&worlds[0]), swing(&worlds[1]));
        assert_eq!(
            wins_on(&worlds[1..], P1, &line, &quick(), Prices::NONE),
            Some(Verdict::Unpaid)
        );
        let roomy = create_node_counter(AI_BUDGET.lethal_nodes, None);
        assert_eq!(
            wins_on(&worlds[1..], P1, &line, &roomy, Prices::NONE),
            Some(Verdict::Dead),
            "My Pawn cancels the lethal attack"
        );
        assert!(
            matches!(depth_first(&worlds, P1, &quick(), Prices::NONE), Walk::Cut),
            "a lethal the slice could not pay to prove on the other world read as a searched tree"
        );
    }

    #[test]
    fn a_first_walk_that_pays_every_step_still_reads_a_tree_without_a_lethal_as_searched() {
        // The Vanilla's 4 does not reach 5, so My Pawn would not answer and no surcharge applies.
        let worlds = [vanilla_into(5, true), vanilla_into(5, true)];
        assert_eq!(surcharge_nodes(&worlds[0], P1, &swing(&worlds[0])), 0);
        assert!(matches!(
            depth_first(&worlds, P1, &quick(), Prices::NONE),
            Walk::Exhausted
        ));
    }

    /// p1 on turn 9 with Mr. Vanilla ready and Jewelosco Scarab in hand, one crystal to play it, against a
    /// hero at 10.
    fn scarab_in_hand() -> GameState {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": "lethal-r5119000", "active": P1, "turn": 9,
            "p1": { "field": [VANILLA], "hand": [SCARAB], "mana": 1, "library": [VANILLA, VANILLA] },
            "p2": { "health": 10, "library": [VANILLA, VANILLA] },
        }))
        .state()
        .clone()
    }

    #[test]
    fn r5119000_an_open_prompt_spends_no_attack() {
        let state = scarab_in_hand();
        let ready = gaps(&state, P1);
        assert_eq!(
            (ready.ready, ready.taunt_free),
            (6.0, 6.0),
            "the Vanilla's 4 of the hero's 10"
        );
        let scarab = state.players[P1]
            .hand
            .first()
            .map(|card| card.id.clone())
            .unwrap_or_default();
        let play = legal_actions(&state, P1)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == scarab))
            .expect("the Scarab can be played");
        let counter = create_node_counter(AI_BUDGET.lethal_nodes, None);
        let prompted = simulate(&state, P1, &play, &counter, Prices::NONE)
            .and_then(Result::ok)
            .expect("the play runs");
        assert!(prompted.pending.is_some(), "the Cry's Discover is open");
        let open = gaps(&prompted, P1);
        assert_eq!(open.ready, 10.0, "no attack is listed while the prompt is open");
        assert_eq!(
            open.taunt_free, 6.0,
            "answering it spends no attack, so the Vanilla is still ready"
        );
    }

    /// #19 Midrange Menace, a 9/9 Taunt.
    const MENACE: &str = "core-019";

    #[test]
    fn r5119000_a_position_waiting_on_its_prompt_is_ordered_with_the_attackers_kept() {
        jackioh_cards::register_all();
        // p1's ready Vanilla is walled by a Menace it cannot kill; p1 holds a Scarab and a second Vanilla.
        let root = scenario(json!({
            "seed": "lethal-r5119000-prompt", "active": P1, "turn": 9,
            "p1": { "field": [VANILLA], "hand": [SCARAB, VANILLA], "mana": 2, "library": [VANILLA, VANILLA] },
            "p2": { "field": [MENACE], "health": 8, "library": [VANILLA, VANILLA] },
        }))
        .state()
        .clone();
        let hand = |def_id: &str| {
            root.players[P1]
                .hand
                .iter()
                .find(|card| card.def_id == def_id)
                .map(|card| card.id.clone())
                .unwrap_or_default()
        };
        let (scarab, second) = (hand(SCARAB), hand(VANILLA));
        let legal = legal_actions(&root, P1);
        let pick = |wanted: &dyn Fn(&ActionBody) -> bool| {
            legal
                .iter()
                .find(|action| wanted(action))
                .cloned()
                .expect("the move is legal")
        };
        let attack = pick(
            &|action| matches!(action, ActionBody::Attack { target_id, .. } if !target_id.starts_with("hero")),
        );
        let play =
            pick(&|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == second));
        let prompt =
            pick(&|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == scarab));
        let counter = create_node_counter(AI_BUDGET.lethal_nodes, None);
        let reach = |action: &ActionBody| {
            simulate(&root, P1, action, &counter, Prices::NONE)
                .and_then(Result::ok)
                .expect("the move runs")
        };
        let state = Rc::new(root.clone());
        let branches = [
            branch_at(0, 8.0),
            branch_at(0, 8.0),
            branch_at(0, 8.0),
            branch_at(0, 8.0),
        ];
        // The walk opens the root's children in move order: the attack spends the Vanilla on the Menace,
        // the play keeps it ready, and the Scarab's Cry leaves its Discover open.
        let opened: Vec<Open> = [&attack, &play, &prompt]
            .into_iter()
            .zip(1u32..)
            .map(|(action, order)| {
                let next = reach(action);
                let at = gaps(&next, P1);
                assert_eq!(at.ready, 8.0, "no child comes closer than the root");
                open_at(&state, at.ready, at.taunt_free, 1, order as usize, order)
            })
            .collect();
        assert!(reach(&prompt).pending.is_some(), "the Discover is open");
        assert_eq!(
            next_open(&opened, &branches, true),
            1,
            "the play that kept the Vanilla ready"
        );
        // Once the play is expanded, the position waiting on its Discover comes before the spent attack,
        // as the play did: its answer spends no attack.
        let left = [opened.first(), opened.get(2)].map(|open| {
            open.map(|at| open_at(&state, at.gap, at.taunt_free, 1, at.branch, at.order))
                .expect("two opened")
        });
        assert_eq!(
            next_open(&left, &branches, true),
            1,
            "the Discover before the spent attack"
        );
    }

    /// An open position at `gap`, `taunt_free` and `depth`, on `branch`, the `order`-th the walk opened.
    fn open_at(
        state: &Rc<GameState>,
        gap: f64,
        taunt_free: f64,
        depth: usize,
        branch: usize,
        order: u32,
    ) -> Open {
        Open {
            state: Rc::clone(state),
            line: vec![ActionBody::EndTurn; depth],
            gap,
            taunt_free,
            branch,
            order,
        }
    }

    /// A branch reached from `parent` at `gap`, nothing tried below it yet.
    fn branch_at(parent: usize, gap: f64) -> Branch {
        Branch {
            parent,
            gap,
            dive: 0,
            aside: false,
        }
    }

    #[test]
    fn r5119000_at_a_flat_gap_the_walk_keeps_its_attackers_ready() {
        let state = Rc::new(vanilla_into(5, false));
        let branches = [branch_at(0, 4.0), branch_at(0, 4.0), branch_at(0, 4.0)];
        // Two children of the root at its gap: the first spent its attacker on a Taunt the gap already
        // counted it against, the second kept it ready.
        let pair = [
            open_at(&state, 4.0, 4.0, 1, 1, 1),
            open_at(&state, 4.0, 0.0, 1, 2, 2),
        ];
        assert_eq!(
            next_open(&pair, &branches, true),
            1,
            "the attacker kept ready first"
        );
        assert_eq!(next_open(&pair, &branches, false), 0, "without it, by age alone");
        // Where the ready attacks already reach lethal the oldest comes first: there the kills that open
        // the way are the line.
        let lethal = [
            open_at(&state, 0.0, 0.0, 1, 1, 1),
            open_at(&state, 0.0, -4.0, 1, 2, 2),
        ];
        assert_eq!(next_open(&lethal, &branches, true), 0);
    }

    #[test]
    fn r5119000_a_dive_past_its_moves_at_a_flat_gap_is_set_aside() {
        let state = Rc::new(vanilla_into(5, false));
        // The root at 4, two children at 4, and a grandchild below the first.
        let mut branches = vec![
            branch_at(0, 4.0),
            branch_at(0, 4.0),
            branch_at(0, 4.0),
            branch_at(1, 4.0),
        ];
        let sibling_and_deep = || {
            [
                open_at(&state, 4.0, 4.0, 1, 2, 2),
                open_at(&state, 4.0, 4.0, 2, 3, 3),
            ]
        };
        let dive = 10;
        assert_eq!(
            next_open(&sibling_and_deep(), &branches, true),
            1,
            "the deeper first while nothing is set aside"
        );
        // An expansion that came closer than the root counts against neither branch above it.
        charge_dive(&mut branches, 3, 3.0, dive + 1, dive);
        assert!(!below_aside(&branches, 3));
        // Moves tried at the root's gap below the first child, past the bound, set it aside.
        charge_dive(&mut branches, 3, 4.0, dive + 1, dive);
        assert!(branches[1].aside && branches[3].aside && !branches[2].aside);
        assert_eq!(
            next_open(&sibling_and_deep(), &branches, true),
            0,
            "the sibling before the dive set aside"
        );
        let sibling_and_closer = [
            open_at(&state, 4.0, 4.0, 1, 2, 2),
            open_at(&state, 3.0, 3.0, 2, 3, 4),
        ];
        assert_eq!(
            next_open(&sibling_and_closer, &branches, true),
            1,
            "a closer position first, set aside or not"
        );
    }
}
