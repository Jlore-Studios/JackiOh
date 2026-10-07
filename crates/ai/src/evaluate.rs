//! The AI's static evaluation (SPEC §9.9): hero health (concave, so the last points weigh most),
//! hero armor, board stats and keywords through §10.4's layers (a Defense-Position unit's attack at
//! a discount), cards in hand, library, the enemy's face damage next turn against our health and the
//! reverse, and, late in the game, the turn cap. Patch v0.2.0's mechanics through the engine's own
//! readers: a unit the engine bars from attacking (E35) counts no attack, Spell Damage (E6) and Immune
//! to Spells are worth something, a Brittle count (B3.3) discounts its card, an Animated backrow card
//! (B3.1) is partly a unit already, and an own hand card's `costMod` (B3.4, R65) moves its worth. It
//! reads only what the seat may know or what a determinization sampled, and nothing here reads a
//! handicap or a difficulty (R180).
//!
//! Port of `packages/ai/src/evaluate.ts`. TS's defaulted arguments (`w = AI_EVAL`, `next = "enemy"`)
//! are passed explicitly.

use jackioh_engine::config::{HERO_HEALTH, TURN_CAP_PLAYER_TURNS};
use jackioh_engine::{
    CardInstance, CardType, GameState, Keyword, KeywordKind, PlayerId, Position, UnitView,
    active_brittle_count, active_units_of, animated_kind_of, cannot_attack, find_def, has_keyword,
    hero_armor_of, own_cost, query_cost, subsystems, unit_view,
};
use serde::{Deserialize, Serialize};

use crate::config::EvalWeights;

/// A hero's health, concave: heroHealth × sqrt(h × HERO_HEALTH), so 30 health is worth 30.
fn hero_value(health: i32, w: &EvalWeights) -> f64 {
    if health <= 0 {
        return -w.win;
    }
    w.hero_health * (f64::from(health) * f64::from(HERO_HEALTH)).sqrt()
}

/// §10.8, R33: whether `seat` may read this backrow card. A placeholder never is.
fn readable_by(state: &GameState, card: &CardInstance, seat: PlayerId) -> bool {
    let Some(def) = find_def(Some(state), &card.def_id) else {
        return false;
    };
    if def.type_ != CardType::Trap && def.type_ != CardType::FieldTrap {
        return true;
    }
    if card.face_up == Some(true) {
        return true;
    }
    card.controller == seat
}

/// B3.3: the share of itself a card keeps under its Brittle count (`active_brittle_count`), 1 with none.
fn brittle_share(card: &CardInstance, w: &EvalWeights) -> f64 {
    match active_brittle_count(card) {
        None => 1.0,
        Some(count) => 1.0 - w.brittle_discount / f64::from(count.max(1)),
    }
}

/// Whether the unit may attack at all: no "Can't attack" keyword, and no E35 status bars it.
fn can_swing(state: &GameState, unit: &CardInstance, view: &UnitView) -> bool {
    !has_keyword(&view.keywords, KeywordKind::CantAttack) && !cannot_attack(state, unit)
}

/// One unit's worth on the board: its stats through the layers and its keywords (AI_EVAL), Spell
/// Damage per point, all discounted by its Brittle count.
pub fn unit_worth(state: &GameState, unit: &CardInstance, w: &EvalWeights) -> f64 {
    let view = unit_view(state, unit);
    let weights = &w.keyword;
    let mut value = w.health * f64::from(view.health) + w.armor_point * f64::from(view.armor);
    if can_swing(state, unit, &view) {
        // §4.1: a Defense-Position unit cannot attack until it spends a turn's exertion switching back,
        // though it still strikes back in full, so only part of its attack counts.
        let share = if view.position == Position::Def {
            w.defense_attack_share
        } else {
            1.0
        };
        value += w.attack * f64::from(view.attack) * share;
    }
    for keyword in &view.keywords {
        if let Some(weight) = weights.of(keyword.kind()) {
            value += weight;
        }
        if let Keyword::SpellDamage { n } = keyword {
            value += w.spell_damage * f64::from(*n);
        }
    }
    // §4.1: Defense Position's Taunt and Armor +1 are the position's, not the unit's. Their worth is
    // the damage they soak, which `face_threat` and the reply already count, so here they are worth
    // only `positionGrants` of a printed Taunt and a point of printed Armor.
    if view.position == Position::Def && has_keyword(&view.keywords, KeywordKind::Taunt) {
        value -= (1.0 - w.position_grants) * (weights.taunt + w.armor_point);
    }
    value * brittle_share(unit, w)
}

/// B3.4, R65: the crystals the card's own `costMod` moves its cost by (`own_cost`, floored at 0 as a
/// price is): +1 for a Degrade's dearer card, −1 for an Upgrade's cheaper one, 0 for an X-cost card.
fn cost_shift(state: &GameState, card: &CardInstance) -> i32 {
    if card.cost_mod == 0 {
        return 0;
    }
    match own_cost(state, card) {
        None => 0,
        Some(own) => own.max(0) - (own - card.cost_mod).max(0),
    }
}

/// An own hand card. ponytail: a Degrade or Upgrade of a hand card's stats is not read here.
fn hand_card_value(state: &GameState, card: &CardInstance, w: &EvalWeights) -> f64 {
    let def = find_def(Some(state), &card.def_id);
    let cost = match def {
        None => w.opponent_hand_cost,
        Some(def) => f64::from(query_cost(def)),
    };
    let radiant = if card.radiant { w.radiant_in_hand } else { 0.0 };
    let shift = if def.is_none() {
        0.0
    } else {
        w.hand_cost_delta * f64::from(cost_shift(state, card))
    };
    (w.hand_card + w.hand_per_cost * cost.min(w.hand_cost_cap) + radiant - shift) * brittle_share(card, w)
}

/// A hero's side of the ledger: its concave health and its armor.
fn hero_side(state: &GameState, player: PlayerId, w: &EvalWeights) -> f64 {
    hero_value(state.players[player].hero.health, w) + w.hero_armor * f64::from(hero_armor_of(state, player))
}

/// Everything on one side but the hero: the board, the backrow, the hand and the library.
fn material(state: &GameState, player: PlayerId, seat: PlayerId, w: &EvalWeights) -> f64 {
    let side = &state.players[player];
    let mut value = 0.0;
    for unit in active_units_of(state, player) {
        value += unit_worth(state, unit, w);
    }

    for card in side.backrow.iter().flatten() {
        let def = find_def(Some(state), &card.def_id);
        match def {
            Some(def) if readable_by(state, card, seat) => {
                value += brittle_share(card, w)
                    * (w.backrow_base + w.backrow_per_cost * f64::from(query_cost(def)));
                // B3.1: a Unit in waiting (`unit_worth` brings its own Brittle share).
                if w.animated_share != 0.0 && animated_kind_of(state, card).is_some() {
                    value += w.animated_share * unit_worth(state, card, w);
                }
            }
            _ => value += w.enemy_face_down,
        }
    }

    if player == seat {
        for card in &side.hand {
            value += hand_card_value(state, card, w);
        }
    } else {
        value += side.hand.len() as f64 * (w.hand_card + w.hand_per_cost * w.opponent_hand_cost);
    }

    value += w.library_card * (side.library.len() as f64).min(w.library_comfort);
    value
}

/// The face damage `attacker`'s board could deal the other hero next turn: units with attack > 0,
/// position ATK, no "Can't attack" and no E35 status barring attacks, sorted ascending by attack.
/// Taunt units on the defending side soak up the smallest attackers first; each Taunt costs health +
/// armor, plus one extra attacker if it has Divine Shield. Each remaining attacker's attack goes
/// through `subsystems::projected_hero_damage` (armor and the Anti-oneshot cap, per hit), and the
/// results are summed.
pub fn face_threat(state: &GameState, attacker: PlayerId) -> i32 {
    let mut attacks: Vec<i32> = Vec::new();
    for unit in active_units_of(state, attacker) {
        let view = unit_view(state, unit);
        if view.attack <= 0 || view.position != Position::Atk {
            continue;
        }
        if !can_swing(state, unit, &view) {
            continue;
        }
        attacks.push(view.attack);
    }
    damage_past_taunts(state, attacker.opponent(), &attacks)
}

/// What `attacks` (one entry per attacker) deal `defender`'s hero once its Taunt units have soaked up
/// the smallest attackers, as `face_threat` counts it: each Taunt costs health + armor, plus one extra
/// attacker if it has Divine Shield, and every remaining hit goes through
/// `subsystems::projected_hero_damage`.
pub fn damage_past_taunts(state: &GameState, defender: PlayerId, attacks: &[i32]) -> i32 {
    let mut sorted: Vec<i32> = attacks.to_vec();
    sorted.sort();
    let mut taunts: Vec<UnitView> = active_units_of(state, defender)
        .into_iter()
        .map(|unit| unit_view(state, unit))
        .filter(|view| has_keyword(&view.keywords, KeywordKind::Taunt))
        .collect();
    taunts.sort_by_key(|a| a.health + a.armor);

    let mut next = 0usize;
    for taunt in &taunts {
        if has_keyword(&taunt.keywords, KeywordKind::DivineShield) {
            if next >= sorted.len() {
                return 0;
            }
            next += 1;
        }
        let cost = taunt.health.max(0) + taunt.armor;
        let mut soaked = 0;
        while soaked < cost {
            if next >= sorted.len() {
                return 0;
            }
            soaked += sorted.get(next).copied().unwrap_or(0);
            next += 1;
        }
    }

    let mut total = 0;
    for hit in sorted.iter().skip(next) {
        total += subsystems::projected_hero_damage(state, defender, *hit, false);
    }
    total
}

/// Who swings next, for the threat and pressure terms: "enemy" (the default) reads a state as the end
/// of `seat`'s turn, where the enemy's board attacks before `seat` can answer; "seat" reads it as the
/// start of `seat`'s turn (the opponent's reply has been played out), where `seat` attacks first and
/// the enemy's board is a threat `seat` still has a whole turn to answer.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[serde(rename_all = "camelCase")]
pub enum NextSwing {
    #[default]
    Enemy,
    Seat,
}

/// A score for `seat` (higher is better). Pure, cheap (O(cards)), reads only public-or-sampled state.
/// `w` is the AI's own AI_EVAL unless a caller brings its own weights (the greedy baseline's
/// GREEDY_EVAL, which tuning the AI never moves).
pub fn evaluate(state: &GameState, seat: PlayerId, next: NextSwing, w: &EvalWeights) -> f64 {
    let opp = seat.opponent();
    if let Some(result) = &state.result {
        let turn = f64::from(state.turn);
        return match result.winner.player() {
            Some(winner) if winner == seat => w.win - turn,
            Some(winner) if winner == opp => -w.win + turn,
            _ => w.drawn,
        };
    }

    let enemy_damage = face_threat(state, opp);
    let our_damage = face_threat(state, seat);
    let seat_first = next == NextSwing::Seat;
    let threat_share = if seat_first { w.answerable_threat } else { 1.0 };
    let threat = threat_share
        * (if enemy_damage >= state.players[seat].hero.health {
            w.lethal_threat
        } else {
            w.threat_per_damage * f64::from(enemy_damage)
        });
    let pressure = if our_damage >= state.players[opp].hero.health {
        if seat_first {
            w.lethal_on_board
        } else {
            w.lethal_pressure
        }
    } else {
        w.pressure_per_damage * f64::from(our_damage)
    };

    // §2.5: the game is a draw at the turn cap, which scores nothing, so late on every point of damage
    // on the enemy hero is worth more than the concave hero value alone says.
    let span = (f64::from(TURN_CAP_PLAYER_TURNS) - w.closing_from).max(1.0);
    let progress = ((f64::from(state.turn) - w.closing_from).max(0.0) / span).min(1.0);
    let closing = w.closing_weight * progress * f64::from(HERO_HEALTH - state.players[opp].hero.health);

    let race = w.enemy_health * f64::from(state.players[opp].hero.health);

    let board = material(state, seat, seat, w) - material(state, opp, seat, w);

    hero_side(state, seat, w) - hero_side(state, opp, w) + board - threat + pressure + closing - race
}
