//! The lethal solver (SPEC §9.9): a bounded search for a line that wins this turn on every
//! determinization, through attack orderings, removing Taunt first, buffs before attacks, a spell to
//! the face, and prompts answered on the way. Nothing in this crate recurses, so both of its walks
//! keep their own explicit frontier.
//!
//! It runs in two stages on one node allowance:
//!   1. A depth-first walk in move order (`candidate_actions` puts face attacks and winning trades
//!      first) for AI_SEARCH.lethalQuickNodes nodes. The usual lethal, a few swings at the face, is
//!      the first path it tries. When the walk runs out of moves before it runs out of nodes, the
//!      whole tree has been searched and there is no lethal to find.
//!   2. Otherwise a best-first walk with what is left: it expands the position closest to lethal
//!      (`ready_gap`: the enemy hero's life, in a Duos duel its own HP and its team's pool (R964000),
//!      less what the attacks still to come would deal past its Taunts), trying its first
//!      AI_SEARCH.lethalWidth moves at once. A depth-first walk spends everything below its first
//!      move, so a lethal that starts with a card late in move order (a spell that kills one's own
//!      unit for its Death, a tribute of the enemy's Taunts) is out of its reach on a wide board. The
//!      best-first walk tries each of the first lethalWidth moves and ranks what they leave by
//!      `ready_gap`, so such a card is found when it is among those moves. A move past the first
//!      lethalWidth of its position is never tried by either walk.
//!
//! A line counts only when it wins on every determinization. `decide` plays a proved line trimmed to
//! the actions it needs (`trim_lethal`, R1965000), since a walk may prove a lethal behind steps it does
//! not need, and only a line's first action is played.
//!
//! Port of `packages/ai/src/lethal.ts`. TS's frames shared one state object between the moves pushed
//! from it; here they share it through an `Rc`.

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

/// Whether `legal_actions` would list an attack for `unit`, which `seat` controls, in a position
/// where `seat` may act in its main phase: some target passes the engine's own test
/// (`combat::can_attack`, the filter `attack_targets` applies). Asked unit by unit, with a unit whose
/// attack is spent ruled out first, because enumerating every legal action (every play, target and
/// mode of a wide hand) cost the best-first walk more than simulating its moves did.
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

/// Whether `unit`, which `has_attack`, may hit the enemy hero once no Taunt walls it: what
/// `combat::why_cannot_attack` asks of a hero target before the Taunt wall (§4.2 step 3), which it
/// tests last. Not `can_attack` on the hero, which refuses every attacker a Taunt wall binds. The unit
/// restrictions refuse the hero inside the Gauntlet (R1849005), and §6.1 lets a unit on its summon
/// turn hit the hero only with Charge, Rush lifting summoning sickness for unit targets alone.
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

/// How far the seat stands from lethal this turn: the enemy hero's life (`effective_life`: in a Duos
/// duel its own HP and its team's pool, R964000) less what the units that may still attack (those
/// `legal_actions` lists an attack for) would deal it past the enemy's Taunts, counted as
/// `face_threat` counts it. A unit that may attack units only (`may_hit_hero`: a Rush unit on its
/// summon turn, or one in the Gauntlet) still binds a Taunt but deals the hero nothing past them
/// (#1823). Zero or less means the attacks left could end the game. It only orders the search; a
/// lethal is always proved by playing it through `reduce`.
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
    // TS first ran `subsystems.syncFusedScripts(state)`, as legalActions did, so a fused card's scripts
    // were this state's own before its layers were read. Rust builds fused scripts from the state on
    // every lookup (SURFACE §6.6), so there is nothing to sync.
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

/// The lethal solver on dets[0]: `lethal_moves` (candidate_actions minus endTurn and every
/// switchPosition but a Deft unit's to Attack), lines at most AI_SEARCH.lethalMaxDepth long, a
/// visited set keyed on `search_signature`. A depth-first walk in move order gets
/// AI_SEARCH.lethalQuickNodes; if it neither found a lethal nor searched the whole tree, a best-first
/// walk by `ready_gap` gets the rest (the header says why). A line is lethal when simulate leaves
/// state.result.winner === seat, and it is returned only if replaying it (equality with legal_actions
/// at each step) also wins on every other determinization; otherwise the search continues. Spends at
/// most `limit` nodes of `counter`.
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

/// `find_lethal` with the depth-first walk's share given: TS's test set `AI_SEARCH.lethalQuickNodes`
/// at run time, which a Rust `const` cannot be (`tests/ai/lethal.rs`).
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

/// R1965000: `line`, a lethal `find_lethal` proved on `dets`, cut down to the actions it needs, for
/// `decide`, which plays only a line's first action and then plans again. A walk may prove a lethal
/// behind steps it does not need: the best-first walk takes the longer line on a tied `ready_gap`, and
/// a free play that leaves the gap where it was (a (0) Spell that draws nothing, or The Coin, played
/// again from the graveyard under C #90 In Too Deep's reward L, R454) can stand in front of the same
/// lethal at every decision, so playing the untrimmed line's first action could repeat it without
/// end. First the shortest suffix that still wins on every world of `dets` (`wins_on`), so the action
/// played is one the rest of the line cannot win without; then, one at a time from the front, each
/// action the line still wins without, starting over after each, until it needs every action left.
/// Each try is `wins_on`'s replay through `simulate`, charged as the walks' own proof is: at most
/// `lethal_max_depth` tries a pass and one pass for each action it drops. `None` when the counter
/// stops it before it has found that suffix: the line it holds then may open with such a free play,
/// and a clock that stops each decision at the same point would have it played every time. When the
/// counter stops it after, the line is kept as far as it was cut.
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

/// R2102000: whether `seat` has a sure win this turn by what it can see, for the practice AI's
/// lethal hint (SPEC §9.9). Like `decide` (R185) it reads only `redact(state, seat)`: it fills that in
/// as LETHAL_HINT_DETERMINIZATIONS worlds drawn from LETHAL_HINT_SEED's stream, its own, and asks
/// `find_lethal` for a line that wins on every one of them within LETHAL_HINT_NODES nodes. So two
/// states that differ only in what `seat` cannot see give one answer, and a face-down card that could
/// be a trap which stops the line keeps it dark. False outside `seat`'s own main phase with no prompt
/// open, once the game is over, when the search panics, and when `should_stop` (the browser's clock,
/// as `AiOptions::should_stop` is the AI's) stops it before it proves a line. Only the page asks it;
/// `decide` never does, so it changes no game.
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
