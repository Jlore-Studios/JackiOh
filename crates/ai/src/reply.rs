//! The opponent's reply (SPEC §9.9, docs/polish/3-ai.md "The opponent's reply"). A line that passes
//! the turn is worth what is left of it after the opponent answers, so `decide` scores its best lines
//! one turn deeper: on the determinization, the opponent plays its turn by a fixed rule and ends it,
//! and the line is scored at the seat's next decision.
//!
//! The rule, one step at a time:
//!   1. A card the line itself put in the opponent's hand (a Pocket Chaos it was handed, the units a
//!      Flood bounced) is no guess: the seat watched it arrive. The opponent plays the one whose
//!      result its own evaluation likes best, if that beats standing still.
//!   2. Otherwise it is a static trader: of every attack it may declare, it takes the one with the
//!      best value read off the layers (lethal, a kill it survives, a trade up, the face).
//!   3. When neither is worth doing it ends its turn.
//! It plays none of the cards it held unseen, because those are samples: a guessed hand would add
//! noise, not information. Every step is a real `reduce` (one node each), so the engine decides what
//! actually happens: traps, First Strike, Divine Shield, Taunt, Lifesteal, end-of-turn damage and the
//! seat's own start of turn. Nothing here recurses.
//!
//! Port of `packages/ai/src/reply.ts`. TS's `try { reduce } catch` is `catch_unwind` around
//! `reduce`, whose refusals come back as `error` and whose impossible states panic (SURFACE §4.4.9).

use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexSet;
use jackioh_engine::{
    Action, ActionBody, CardInstance, GameState, KeywordKind, PlayerId, UnitView, find_instance, has_keyword,
    legal_actions, reduce, subsystems, unit_view,
};

use crate::candidates::candidate_actions;
use crate::config::{AI_EVAL, AI_REPLY};
use crate::evaluate::{NextSwing, evaluate, unit_worth};
use crate::simulate::{LineStatus, line_status, simulate, static_score};
use crate::types::NodeCounter;

struct Side {
    view: UnitView,
    shield: bool,
    indestructible: bool,
    first_strike: bool,
    poisonous: bool,
}

fn side_of(view: UnitView) -> Side {
    Side {
        shield: has_keyword(&view.keywords, KeywordKind::DivineShield),
        indestructible: has_keyword(&view.keywords, KeywordKind::Indestructible),
        first_strike: has_keyword(&view.keywords, KeywordKind::FirstStrike),
        poisonous: has_keyword(&view.keywords, KeywordKind::Poisonous),
        view,
    }
}

/// What one blow does (`blow`'s answer).
struct Blow {
    dealt: i32,
    kills: bool,
    pops_shield: bool,
}

/// §4.4 steps 1, 2, 4 and 7 for one blow: what `from` deals to `to`, and whether it kills.
fn blow(from: &Side, to: &Side) -> Blow {
    let amount = from.view.attack;
    if amount <= 0 {
        return Blow { dealt: 0, kills: false, pops_shield: false };
    }
    if to.shield {
        return Blow { dealt: 0, kills: false, pops_shield: true };
    }
    if to.indestructible {
        return Blow { dealt: 0, kills: false, pops_shield: false };
    }
    let dealt = (amount - to.view.armor).max(0);
    let kills = dealt >= to.view.health || (from.poisonous && dealt > 0);
    Blow { dealt, kills, pops_shield: false }
}

/// The opponent's static value of `attacker` hitting `target` (§4.3): the unit it kills, less its own
/// body when that dies too, with First Strike deciding who strikes first. A hit that kills nothing is
/// worth a little per point dealt, so a second attacker can finish what the first one started.
fn trade_value(state: &GameState, attacker: &CardInstance, target_id: &str) -> f64 {
    let Some(target) = find_instance(state, target_id) else {
        return f64::NEG_INFINITY;
    };
    let a = side_of(unit_view(state, attacker));
    let t = side_of(unit_view(state, target));
    let out = blow(&a, &t);
    let back = blow(&t, &a);
    let mut kills = out.kills;
    let mut dies = back.kills;
    if a.first_strike && !t.first_strike && kills {
        dies = false;
    }
    if t.first_strike && !a.first_strike && dies {
        kills = false;
    }

    let mut value = if kills {
        unit_worth(state, target, &AI_EVAL)
    } else {
        AI_REPLY.chip_per_damage * f64::from(out.dealt.min(t.view.health))
    };
    if out.pops_shield {
        value += AI_EVAL.keyword.divine_shield;
    }
    if dies {
        value -= unit_worth(state, attacker, &AI_EVAL);
    }
    value
}

/// The opponent's attacks worth making, best static value first (ties keep legal_actions order).
fn ranked_attacks(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    let opp = seat.opponent();
    let hero = format!("hero-{}", seat.as_str());
    let health = state.players[seat].hero.health;
    let mut ranked: Vec<(ActionBody, f64)> = Vec::new();
    for action in legal_actions(state, opp) {
        let ActionBody::Attack { attacker_id, target_id } = &action else {
            continue;
        };
        let Some(attacker) = find_instance(state, attacker_id) else {
            continue;
        };
        let value = if *target_id == hero {
            let damage = subsystems::projected_hero_damage(state, seat, unit_view(state, attacker).attack, false);
            if damage >= health { AI_EVAL.win } else { AI_REPLY.face_per_damage * f64::from(damage) }
        } else {
            trade_value(state, attacker, target_id)
        };
        if value > 0.0 {
            ranked.push((action, value));
        }
    }
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    ranked.into_iter().map(|(action, _)| action).collect()
}

/// One opponent action through the reducer; `None` when it panics or is refused.
fn reduce_for(state: &GameState, actor: PlayerId, action: &ActionBody, counter: &dyn NodeCounter) -> Option<GameState> {
    let action = Action::new(action.clone(), actor, format!("reply:{}", counter.used()));
    match catch_unwind(AssertUnwindSafe(|| reduce(state, &action))) {
        Ok(next) if next.error.is_none() => Some(next.state),
        _ => None,
    }
}

/// `known_play`'s answer: TS's `GameState | null | undefined`.
enum Known {
    /// The counter refused a node (TS `null`).
    Cut,
    /// No such play is worth making (TS `undefined`).
    Nothing,
    Played(GameState),
}

/// Step 1: the best play of a card the opponent holds that is not in `hidden`, by the opponent's own
/// evaluation, when it beats standing still.
fn known_play(state: &GameState, seat: PlayerId, hidden: &IndexSet<String>, counter: &mut dyn NodeCounter) -> Known {
    let opp = seat.opponent();
    let plays: Vec<ActionBody> = candidate_actions(state, opp)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if !hidden.contains(instance_id)))
        .take(AI_REPLY.known_plays.max(0) as usize)
        .collect();
    if plays.is_empty() {
        return Known::Nothing;
    }
    let mut best: Option<GameState> = None;
    let mut best_value = evaluate(state, opp, NextSwing::Enemy, &AI_EVAL);
    for play in &plays {
        let next = match simulate(state, opp, play, &mut *counter) {
            None => return Known::Cut,
            Some(Err(_)) => continue,
            Some(Ok(next)) => next,
        };
        if next.result.is_some_and(|result| result.winner.player() == Some(opp)) {
            return Known::Played(next);
        }
        let value = evaluate(&next, opp, NextSwing::Enemy, &AI_EVAL);
        // Strictly greater, so ties keep move order.
        if value > best_value {
            best_value = value;
            best = Some(next);
        }
    }
    match best {
        Some(state) => Known::Played(state),
        None => Known::Nothing,
    }
}

/// The ids of the cards the opponent holds unseen: its hand and its library, as sampled.
pub fn hidden_card_ids(state: &GameState, seat: PlayerId) -> IndexSet<String> {
    let side = &state.players[seat.opponent()];
    side.hand.iter().chain(side.library.iter()).map(|card| card.id.clone()).collect()
}

/// The opponent's reply to a line that handed it the turn: from `state` (the opponent's main phase,
/// or a prompt on the way there) it plays by the rule in this file's header and ends its turn.
/// Prompts on either side are answered with their first legal answer. `hidden` holds the ids of the
/// cards the opponent held unseen when the decision began (`hidden_card_ids` of the decision's
/// determinization, TS's default); any other card in its hand is one it may play. Returns the state
/// at the seat's next main phase or prompt, or where the game ended; `None` when the counter refused
/// a node.
pub fn simulate_reply(
    state: &GameState,
    seat: PlayerId,
    counter: &mut dyn NodeCounter,
    hidden: &IndexSet<String>,
) -> Option<GameState> {
    let opp = seat.opponent();
    let mut current = state.clone();
    for _step in 0..AI_REPLY.max_steps {
        if current.result.is_some() {
            return Some(current);
        }
        if let Some(pending) = &current.pending {
            // The seat's own prompt at its next turn start is where its next decision begins.
            if pending.player_id == seat && current.active == seat {
                return Some(current);
            }
            let answerer = pending.player_id;
            let Some(answer) = legal_actions(&current, answerer).into_iter().next() else {
                return Some(current);
            };
            if !counter.take() {
                return None;
            }
            let Some(next) = reduce_for(&current, answerer, &answer, &*counter) else {
                return Some(current);
            };
            current = next;
            continue;
        }
        if current.active != opp {
            return Some(current);
        }

        match known_play(&current, seat, hidden, &mut *counter) {
            Known::Cut => return None,
            Known::Played(played) => {
                current = played;
                continue;
            }
            Known::Nothing => {}
        }

        let attack = ranked_attacks(&current, seat).into_iter().next();
        if !counter.take() {
            return None;
        }
        let step = attack.clone().unwrap_or(ActionBody::EndTurn);
        if let Some(next) = reduce_for(&current, opp, &step, &*counter) {
            current = next;
            continue;
        }
        // A refused attack ends the turn instead; a refused endTurn ends the reply where it stands.
        if attack.is_none() || !counter.take() {
            return if attack.is_none() { Some(current) } else { None };
        }
        let Some(ended) = reduce_for(&current, opp, &ActionBody::EndTurn, &*counter) else {
            return Some(current);
        };
        current = ended;
    }
    Some(current)
}

/// A closed line's value after the opponent's reply: `end` is where the line stopped. A line that
/// handed the opponent the turn is scored at the seat's next decision, where the seat swings first,
/// less the crystals it left unspent; a line that ended the game, or stopped on the seat's own
/// prompt, keeps its static score. `hidden` is as for `simulate_reply` (TS's default:
/// `hidden_card_ids(end, seat)`). `None` when the counter ran out.
pub fn reply_score(
    end: &GameState,
    seat: PlayerId,
    root_turn: i32,
    counter: &mut dyn NodeCounter,
    hidden: &IndexSet<String>,
) -> Option<f64> {
    let status = line_status(end, seat, root_turn);
    if status == LineStatus::Over || status == LineStatus::Open {
        return Some(static_score(end, seat, root_turn));
    }
    let unspent = if status == LineStatus::Passed { end.players[seat].turn_log.unspent_at_end.unwrap_or(0) } else { 0 };
    let after = simulate_reply(end, seat, counter, hidden)?;
    let next = if after.active == seat { NextSwing::Seat } else { NextSwing::Enemy };
    Some(evaluate(&after, seat, next, &AI_EVAL) - AI_EVAL.unspent_mana * f64::from(unspent))
}
