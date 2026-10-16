//! The opponent's reply (SPEC §9.9, docs/polish/3-ai.md "The opponent's reply"). A line that passes
//! the turn is worth what is left of it after the opponent answers, so `decide` scores its best lines
//! one turn deeper: on the determinization, the opponent plays its turn by a fixed rule and ends it,
//! and the line is scored at the seat's next decision.
//!
//! The rule, one step at a time:
//!   1. A card the line itself put in the opponent's hand (a Pocket Chaos it was handed, the units a
//!      Flood bounced) is no guess: the seat watched it arrive. The opponent plays the one whose
//!      result its own evaluation likes best, if that beats standing still.
//!   2. An Activate of a card the seat can see face-up on the field (The Power to Punish's damage, a
//!      Heroic Power's rolled Ping) is no guess either. The opponent tries up to
//!      AI_REPLY.visible_activations, those aimed at the seat's hero first, and uses the one its own
//!      evaluation likes best if that beats standing still.
//!   3. Otherwise it is a static trader: of every attack it may declare, it takes the one with the
//!      best value read off the layers (lethal, a kill it survives, a trade up, the face). First, a
//!      unit of its with Deft in Defense Position, its switch and its attack unspent, is switched to
//!      Attack on trial (R49: it can still attack): the switch is kept when the best attack after it
//!      is one of the unit's own, and otherwise the unit stays in Defense.
//!   4. When none is worth doing it ends its turn.
//!
//! It plays none of the cards it held unseen, because those are samples: a guessed hand would add
//! noise, not information; nor does it use a face-down card's Reveal, whose card is a sample too.
//! Every step is a real `reduce` (one node each), so the engine decides what actually happens: traps,
//! First Strike, Divine Shield, Taunt, Lifesteal, end-of-turn damage and the seat's own start of turn.
//! Nothing here recurses.
//!
//! Port of `packages/ai/src/reply.ts`. TS's `try { reduce } catch` is `catch_unwind` around
//! `reduce`, whose refusals come back as `error` and whose impossible states panic (SURFACE §4.4.9).

use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexSet;
use jackioh_engine::{
    Action, ActionBody, CardInstance, GameState, KeywordKind, PlayerId, Selection, UnitView, effective_life,
    find_instance, has_keyword, legal_actions, pierces, reduce, subsystems, takes_strike_back, unit_view,
};

use crate::candidates::candidate_actions_in;
use crate::config::{AI_EVAL, AI_REPLY, AI_SEARCH};
use crate::evaluate::{NextSwing, evaluate, unit_worth};
use crate::lethal::deft_ready_switch;
use crate::simulate::{LineStatus, best_auto_answer, line_status, simulate, static_score, surcharge_nodes};
use crate::types::NodeCounter;

struct Side {
    view: UnitView,
    shield: bool,
    indestructible: bool,
    first_strike: bool,
    poisonous: bool,
    /// R346: its hits skip §4.4 step 2, the Armor.
    pierce: bool,
}

fn side_of(state: &GameState, unit: &CardInstance) -> Side {
    let view = unit_view(state, unit);
    Side {
        shield: has_keyword(&view.keywords, KeywordKind::DivineShield),
        indestructible: has_keyword(&view.keywords, KeywordKind::Indestructible),
        first_strike: has_keyword(&view.keywords, KeywordKind::FirstStrike),
        poisonous: has_keyword(&view.keywords, KeywordKind::Poisonous),
        pierce: pierces(state, Some(unit), None),
        view,
    }
}

/// What one blow does (`blow`'s answer).
struct Blow {
    dealt: i32,
    kills: bool,
    pops_shield: bool,
}

/// §4.4 steps 1, 2, 4 and 7 for one blow: what `from` deals to `to`, and whether it kills. Step 2's
/// Armor is skipped when `from` pierces (R346).
fn blow(from: &Side, to: &Side) -> Blow {
    let amount = from.view.attack;
    if amount <= 0 {
        return Blow {
            dealt: 0,
            kills: false,
            pops_shield: false,
        };
    }
    if to.shield {
        return Blow {
            dealt: 0,
            kills: false,
            pops_shield: true,
        };
    }
    if to.indestructible {
        return Blow {
            dealt: 0,
            kills: false,
            pops_shield: false,
        };
    }
    let dealt = if from.pierce {
        amount
    } else {
        (amount - to.view.armor).max(0)
    };
    let kills = dealt >= to.view.health || (from.poisonous && dealt > 0);
    Blow {
        dealt,
        kills,
        pops_shield: false,
    }
}

/// The opponent's static value of `attacker` hitting `target` (§4.3): the unit it kills, less its own
/// body when that dies too, with First Strike deciding who strikes first and no strike back at all on
/// a Ranged attacker unless the target is Ranged too (R783000). A hit that kills nothing is worth a
/// little per point dealt, so a second attacker can finish what the first one started.
fn trade_value(state: &GameState, attacker: &CardInstance, target_id: &str) -> f64 {
    let Some(target) = find_instance(state, target_id) else {
        return f64::NEG_INFINITY;
    };
    let a = side_of(state, attacker);
    let t = side_of(state, target);
    let out = blow(&a, &t);
    let mut kills = out.kills;
    let mut dies = takes_strike_back(state, attacker, target) && blow(&t, &a).kills;
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

/// The static trader's value of a face hit of `damage` on `seat`'s hero: AI_EVAL.win when it would end
/// the game, judged against all of `seat`'s life (`effective_life`: in a Duos duel its own HP and its
/// team's pool while unspent, R964000), else AI_REPLY.face_per_damage a point.
pub fn reply_face_value(state: &GameState, seat: PlayerId, damage: i32) -> f64 {
    if damage >= effective_life(state, seat) {
        AI_EVAL.win
    } else {
        AI_REPLY.face_per_damage * f64::from(damage)
    }
}

/// The opponent's attacks worth making out of `legal` (legal_actions(state, opp)), best static value
/// first (ties keep legal_actions order).
fn ranked_attacks(state: &GameState, seat: PlayerId, legal: &[ActionBody]) -> Vec<ActionBody> {
    let hero = format!("hero-{}", seat.as_str());
    let mut ranked: Vec<(ActionBody, f64)> = Vec::new();
    for action in legal.iter().cloned() {
        let ActionBody::Attack {
            attacker_id,
            target_id,
        } = &action
        else {
            continue;
        };
        let Some(attacker) = find_instance(state, attacker_id) else {
            continue;
        };
        let value = if *target_id == hero {
            let pierce = pierces(state, Some(attacker), None);
            let damage =
                subsystems::projected_hero_damage(state, seat, unit_view(state, attacker).attack, pierce);
            reply_face_value(state, seat, damage)
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

/// One opponent action through the reducer; `None` when it panics or is refused. A step whose
/// reduce runs the expensive work `surcharge_nodes` knows (a whole AI turn inside a trap's answer)
/// pays its nodes the same way `simulate` does.
fn reduce_for(
    state: &GameState,
    actor: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
) -> Option<GameState> {
    // `None` here already reads "skip this step" to the reply walk, so a surcharge the slice
    // cannot pay returns early instead of draining it node by node first.
    let surcharge = surcharge_nodes(state, actor, action);
    if counter.used() + surcharge > counter.limit() {
        return None;
    }
    for _ in 0..surcharge {
        if !counter.take() {
            return None;
        }
    }
    let action = Action::new(action.clone(), actor, format!("reply:{}", counter.used()));
    match catch_unwind(AssertUnwindSafe(|| reduce(state, &action))) {
        Ok(next) if next.error.is_none() => Some(next.state),
        _ => None,
    }
}

/// `known_play`'s, `visible_activation`'s and `deft_switch`'s answer: TS's `GameState | null | undefined`.
enum Known {
    /// The counter refused a node (TS `null`).
    Cut,
    /// No such play is worth making (TS `undefined`).
    Nothing,
    Played(Box<GameState>),
}

/// Step 1: the best play of a card the opponent holds that is not in `hidden`, by the opponent's own
/// evaluation, when it beats standing still. `legal` is legal_actions(state, opp), already in hand.
fn known_play(
    state: &GameState,
    seat: PlayerId,
    legal: &[ActionBody],
    hidden: &IndexSet<String>,
    counter: &dyn NodeCounter,
) -> Known {
    let opp = seat.opponent();
    let plays: Vec<ActionBody> = candidate_actions_in(state, opp, legal.to_vec())
        .into_iter()
        .filter(
            |action| matches!(action, ActionBody::Play { instance_id, .. } if !hidden.contains(instance_id)),
        )
        .take(AI_REPLY.known_plays.max(0) as usize)
        .collect();
    best_try(state, opp, &plays, counter)
}

/// The best of `tries` for `opp` by its own evaluation: one that wins the game at once, else the one
/// whose result beats standing still by the most (ties keep the list's order).
fn best_try(state: &GameState, opp: PlayerId, tries: &[ActionBody], counter: &dyn NodeCounter) -> Known {
    if tries.is_empty() {
        return Known::Nothing;
    }
    let mut best: Option<GameState> = None;
    let mut best_value = evaluate(state, opp, NextSwing::Enemy, &AI_EVAL);
    for action in tries {
        let next = match simulate(state, opp, action, counter) {
            None => return Known::Cut,
            Some(Err(_)) => continue,
            Some(Ok(next)) => next,
        };
        if next
            .result
            .is_some_and(|result| result.winner.player() == Some(opp))
        {
            return Known::Played(Box::new(next));
        }
        let value = evaluate(&next, opp, NextSwing::Enemy, &AI_EVAL);
        // Strictly greater, so ties keep move order.
        if value > best_value {
            best_value = value;
            best = Some(next);
        }
    }
    match best {
        Some(state) => Known::Played(Box::new(state)),
        None => Known::Nothing,
    }
}

/// Where a visible activation is aimed, in the order a reply step tries them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Aim {
    /// At the seat's hero, where a lethal one is.
    SeatHero,
    /// At a card the seat controls: the opponent's activated removal.
    SeatCard,
    /// Anything else, an activation with no target among them.
    Other,
}

/// Where `action` aims, read off the targets it carries.
fn aim_of(state: &GameState, seat: PlayerId, action: &ActionBody) -> Aim {
    let targets: &[Selection] = match action {
        ActionBody::Activate { targets, .. } | ActionBody::ActivatePower { targets, .. } => {
            targets.as_deref().unwrap_or(&[])
        }
        _ => &[],
    };
    if targets
        .iter()
        .any(|target| matches!(target, Selection::Hero { player } if *player == seat))
    {
        return Aim::SeatHero;
    }
    let at_seat_card = targets.iter().any(|target| {
        matches!(target, Selection::Instance { instance_id }
            if find_instance(state, instance_id).is_some_and(|card| card.controller == seat))
    });
    if at_seat_card { Aim::SeatCard } else { Aim::Other }
}

/// Whether the seat can see the source of `action`, an `activate` (or the `activatePower` alias, which
/// `legal_actions` no longer lists, R752): a card acting face-up on the field (`is_acting_on_field`, so
/// never a face-down card's Reveal, R783006, nor a card determinization filled in) that is none of the
/// cards the opponent held unseen.
fn visible_source(state: &GameState, hidden: &IndexSet<String>, action: &ActionBody) -> bool {
    let (ActionBody::Activate { instance_id, .. } | ActionBody::ActivatePower { instance_id, .. }) = action
    else {
        return false;
    };
    !hidden.contains(instance_id)
        && find_instance(state, instance_id).is_some_and(|card| subsystems::is_acting_on_field(state, card))
}

/// Step 2: the best of the opponent's activations whose source the seat can see, by the opponent's
/// own evaluation, when it beats standing still. At most AI_REPLY.visible_activations are tried,
/// ordered by `aim_of` and in legal_actions order within each aim: `legal_actions` lists the
/// activating side's own targets first, so the seat's hero would otherwise come last.
fn visible_activation(
    state: &GameState,
    seat: PlayerId,
    legal: &[ActionBody],
    hidden: &IndexSet<String>,
    counter: &dyn NodeCounter,
) -> Known {
    let mut tries: Vec<ActionBody> = legal
        .iter()
        .filter(|action| visible_source(state, hidden, action))
        .cloned()
        .collect();
    // Stable, so each aim keeps legal_actions order.
    tries.sort_by_key(|action| aim_of(state, seat, action));
    tries.truncate(AI_REPLY.visible_activations.max(0) as usize);
    best_try(state, seat.opponent(), &tries, counter)
}

/// Step 3, R49: the opponent's Deft units in Defense Position may switch to Attack and still attack.
/// Each one whose switch `legal` lists and whose attack is unspent (`deft_ready_switch`), in
/// legal_actions order, is switched on trial, one node; the first whose best-ranked attack after the
/// switch is one of its own keeps the switch. Otherwise every such unit stays in Defense.
fn deft_switch(state: &GameState, seat: PlayerId, legal: &[ActionBody], counter: &dyn NodeCounter) -> Known {
    let opp = seat.opponent();
    for action in legal {
        let ActionBody::SwitchPosition { instance_id } = action else {
            continue;
        };
        if !deft_ready_switch(state, instance_id) {
            continue;
        }
        if !counter.take() {
            return Known::Cut;
        }
        let Some(switched) = reduce_for(state, opp, action, counter) else {
            continue;
        };
        let after = legal_actions(&switched, opp);
        let best = ranked_attacks(&switched, seat, &after).into_iter().next();
        if matches!(&best, Some(ActionBody::Attack { attacker_id, .. }) if attacker_id == instance_id) {
            return Known::Played(Box::new(switched));
        }
    }
    Known::Nothing
}

/// The ids of the cards the opponent holds unseen: its hand and its library, as sampled.
pub fn hidden_card_ids(state: &GameState, seat: PlayerId) -> IndexSet<String> {
    let side = &state.players[seat.opponent()];
    side.hand
        .iter()
        .chain(side.library.iter())
        .map(|card| card.id.clone())
        .collect()
}

/// The opponent's reply to a line that handed it the turn: from `state` (the opponent's main phase,
/// or a prompt on the way there) it plays by the rule in this file's header and ends its turn.
/// Prompts on either side get their answerer's best of its first AI_SEARCH.max_auto_answer_options
/// answers (`best_auto_answer`), the seat's own off-turn prompts too. `hidden` holds the ids of the
/// cards the opponent held unseen when the decision began (`hidden_card_ids` of the decision's
/// determinization, TS's default); any other card in its hand is one it may play. Returns the state
/// at the seat's next main phase or prompt, or where the game ended; `None` when the counter refused
/// a node.
pub fn simulate_reply(
    state: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
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
            let options = AI_SEARCH.max_auto_answer_options.max(1) as usize;
            let Ok(next) = best_auto_answer(&current, answerer, options, counter, "reply")? else {
                return Some(current);
            };
            current = next;
            continue;
        }
        if current.active != opp {
            return Some(current);
        }

        // One legal_actions serves the known-play try (its ordered candidates), the visible
        // activations, the Deft switches and the static trader's attack list.
        let legal = legal_actions(&current, opp);
        match known_play(&current, seat, &legal, hidden, counter) {
            Known::Cut => return None,
            Known::Played(played) => {
                current = *played;
                continue;
            }
            Known::Nothing => {}
        }
        match visible_activation(&current, seat, &legal, hidden, counter) {
            Known::Cut => return None,
            Known::Played(played) => {
                current = *played;
                continue;
            }
            Known::Nothing => {}
        }
        match deft_switch(&current, seat, &legal, counter) {
            Known::Cut => return None,
            Known::Played(switched) => {
                current = *switched;
                continue;
            }
            Known::Nothing => {}
        }

        let attack = ranked_attacks(&current, seat, &legal).into_iter().next();
        if !counter.take() {
            return None;
        }
        let step = attack.clone().unwrap_or(ActionBody::EndTurn);
        if let Some(next) = reduce_for(&current, opp, &step, counter) {
            current = next;
            continue;
        }
        // A refused attack ends the turn instead; a refused endTurn ends the reply where it stands.
        if attack.is_none() || !counter.take() {
            return if attack.is_none() { Some(current) } else { None };
        }
        let Some(ended) = reduce_for(&current, opp, &ActionBody::EndTurn, counter) else {
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
    counter: &dyn NodeCounter,
    hidden: &IndexSet<String>,
) -> Option<f64> {
    let status = line_status(end, seat, root_turn);
    if status == LineStatus::Over || status == LineStatus::Open {
        return Some(static_score(end, seat, root_turn));
    }
    let unspent = if status == LineStatus::Passed {
        end.players[seat].turn_log.unspent_at_end.unwrap_or(0)
    } else {
        0
    };
    let after = simulate_reply(end, seat, counter, hidden)?;
    let next = if after.active == seat {
        NextSwing::Seat
    } else {
        NextSwing::Enemy
    };
    Some(evaluate(&after, seat, next, &AI_EVAL) - AI_EVAL.unspent_mana * f64::from(unspent))
}
