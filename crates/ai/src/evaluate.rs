//! The AI's static evaluation (SPEC §9.9): hero health (concave, so the last points weigh most),
//! hero armor, board stats and keywords through §10.4's layers (a Defense-Position unit's attack at
//! a discount), cards in hand, library, the enemy's face damage next turn against our life and the
//! reverse (in a Duos duel own HP and the team's pool, R964000), and, late in the game, the turn cap.
//! Patch v0.2.0's mechanics through the engine's own readers: a unit the engine bars from attacking
//! (E35) counts no attack, Spell Damage (E6) and Immune to Spells are worth something, a Brittle count
//! (B3.3) discounts its card, an Animated backrow card (B3.1) is partly a unit already, and an own hand
//! card's `costMod` (B3.4, R65) moves its worth. It reads only what the seat may know or what a
//! determinization sampled, and nothing here reads a handicap or a difficulty (R180).
//!
//! Port of `packages/ai/src/evaluate.ts`. TS's defaulted arguments (`w = AI_EVAL`, `next = "enemy"`)
//! are passed explicitly.

use jackioh_engine::config::{DUOS_OWN_HEALTH, FIRST_GRADE, HERO_HEALTH, TURN_CAP_PLAYER_TURNS};
use jackioh_engine::{
    AttackTarget, CardInstance, GameState, Keyword, KeywordKind, PlayerId, Position, UnitView,
    active_brittle_count, active_units_of, animated_kind_of, attack_restriction, attacks_per_turn,
    backrow_is_public, cannot_attack, card_keywords, credit_lapsing, effective_life, find_def, has_keyword,
    hero_armor_of, owed_mana_of, own_cost, pierces, query_cost, reset_instance, subsystems,
    takes_strike_back, unit_view,
};
use serde::{Deserialize, Serialize};

use crate::config::EvalWeights;

/// The life `player` started the game with, where the hero value and `closing` are anchored (R964000):
/// HERO_HEALTH in a 1v1 game; in a Duos duel the seat's own HP and its team's pool at its cap (§13.4;
/// every team's own HP is DUOS_OWN_HEALTH, §13.7). Both teams' life is public (§13.6).
pub fn starting_life(state: &GameState, player: PlayerId) -> i32 {
    match state.players[player].duos {
        Some(life) => DUOS_OWN_HEALTH + life.pool_cap,
        None => HERO_HEALTH,
    }
}

/// A hero's life (`effective_life`, R964000), concave: heroHealth × sqrt(h × start), `start` its
/// `starting_life`, so a full hero is worth its life: 30 in a 1v1 game.
fn hero_value(health: i32, start: i32, w: &EvalWeights) -> f64 {
    if health <= 0 {
        return -w.win;
    }
    w.hero_health * (f64::from(health) * f64::from(start)).sqrt()
}

/// §10.8, R33, R686: whether `seat` may read this backrow card, by the rule `view_for` reads
/// (`backrow_is_public`). A placeholder never is, and is asked first: the rule reads the definition.
fn readable_by(state: &GameState, card: &CardInstance, seat: PlayerId) -> bool {
    find_def(Some(state), &card.def_id).is_some() && backrow_is_public(state, card, seat)
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
    // the damage they soak, which `face_threat` and the reply already count, so here they count for
    // only `positionGrants`. With `defenseOwnTauntKept` a unit's own Taunt, printed or granted
    // (`card_keywords` leaves the position's grant out), is kept in full, and the Armor discount
    // applies only when Armor is on the body, so an Indestructible unit, whose Taunt is gone, still
    // discounts its position Armor. Without it (the frozen baseline), every Taunt in Defense Position
    // is discounted with a point of Armor.
    if view.position == Position::Def {
        let taunts = has_keyword(&view.keywords, KeywordKind::Taunt);
        if !w.defense_own_taunt_kept {
            if taunts {
                value -= (1.0 - w.position_grants) * (weights.taunt + w.armor_point);
            }
        } else {
            if taunts && !has_keyword(&card_keywords(state, unit), KeywordKind::Taunt) {
                value -= (1.0 - w.position_grants) * weights.taunt;
            }
            if view.armor > 0 {
                value -= (1.0 - w.position_grants) * w.armor_point;
            }
        }
    }
    // §10.1: the counters on the card are worth their own, outside the Brittle share — Fed Fauci's
    // Plague Counters are mana it already banked, not part of the body that crumbles.
    value * brittle_share(unit, w) + w.plague_counter * f64::from(unit.counters.plague.unwrap_or(0))
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

/// An own hand card. maxi-tail: a Degrade or Upgrade of a hand card's stats is not read here.
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

/// A hero's side of the ledger: its concave life (`effective_life`, R964000) and its armor.
fn hero_side(state: &GameState, player: PlayerId, w: &EvalWeights) -> f64 {
    hero_value(effective_life(state, player), starting_life(state, player), w)
        + w.hero_armor * f64::from(hero_armor_of(state, player))
}

/// Everything on one side but the hero: the board, the backrow, the hand and the library.
fn material(state: &GameState, player: PlayerId, seat: PlayerId, w: &EvalWeights) -> f64 {
    let side = &state.players[player];
    let mut value = 0.0;
    for unit in active_units_of(state, player) {
        value += unit_worth(state, unit, w);
    }

    // R1224: a debt is coming turns' mana spent early, so it counts against the side that owes it.
    value -= w.owed_mana * f64::from(owed_mana_of(state, player));
    // R1225: on its controller's turn, an unused base-face Jlarna is tributed at end of turn, so it
    // is worth nothing.
    let lapsing: Vec<String> = if state.active == player {
        credit_lapsing(state, player)
    } else {
        Vec::new()
    };

    for card in side.backrow.iter().flatten() {
        if lapsing.iter().any(|id| id == &card.id) {
            continue;
        }
        let def = find_def(Some(state), &card.def_id);
        match def {
            Some(def) if readable_by(state, card, seat) => {
                value += brittle_share(card, w)
                    * (w.backrow_base + w.backrow_per_cost * f64::from(query_cost(def)));
                // B3.1: a Unit in waiting (`unit_worth` brings its own Brittle share).
                if w.animated_share != 0.0 && animated_kind_of(state, card).is_some() {
                    value += w.animated_share * unit_worth(state, card, w);
                }
                // §10.1, R27/R62: a grade counter's banked steps are worth their own — a Combo-Index
                // at B is not the fresh one its def prints. On the controller's own turn a rise this
                // turn's plays already arm counts as banked: the beam can prefer the lines that
                // spend enough to set it off, as the glow does (`grade_rises`, which reads the same
                // per-turn log it will fire on).
                if card.counters.grade.is_some() {
                    let grade = subsystems::grade_of(card);
                    if !subsystems::is_terminal_grade(grade) {
                        let armed = card.controller == state.active && subsystems::grade_rises(state, card);
                        value += w.grade_counter * f64::from(grade - FIRST_GRADE + i32::from(armed));
                    }
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
/// Taunt units on the defending side soak up the smallest attackers they bind first, one §4.4 hit at
/// a time (`damage_past_taunts`). Each remaining attacker's attack goes through
/// `subsystems::projected_hero_damage` (armor, which a Pierce hit skips (R346), and the Anti-oneshot
/// cap, per hit), and the results are summed.
pub fn face_threat(state: &GameState, attacker: PlayerId) -> i32 {
    let mut attacks: Vec<(&CardInstance, i32)> = Vec::new();
    for unit in active_units_of(state, attacker) {
        let view = unit_view(state, unit);
        if view.attack <= 0 || view.position != Position::Atk {
            continue;
        }
        if !can_swing(state, unit, &view) {
            continue;
        }
        // R636: the unit's next turn starts with its exertion reset, so a Windfury unit swings twice.
        for _ in 0..attacks_per_turn(state, unit) {
            attacks.push((unit, view.attack));
        }
    }
    damage_past_taunts(state, attacker.opponent(), &attacks)
}

/// §4.5 step 4: a Reborn unit comes back at 1 health.
const REBORN_HEALTH: i32 = 1;

/// What `attacks` (one entry per attack: the attacker and its attack) deal `defender`'s hero once its
/// Taunt units have soaked up the smallest attackers they bind, as `face_threat` counts it. §4.2 step
/// 3: a Taunt binds only the attackers that may attack it, asked of the engine's own
/// `attack_restriction` (what its Taunt wall reads), so a Flying Taunt in Attack Position holds no
/// ground attacker (R783002) and a landed one, in Defense Position, holds every attacker (R783003).
///
/// Each bound attacker, smallest first, is one §4.4 hit on the Taunt (#898): an attacker its First
/// Strike destroys first lands nothing (§4.3 step 1, `falls_to_first_strike`); a Divine Shield takes
/// the first hit whole (step 1); the Taunt's Armor comes off each hit, unless it pierces (step 2,
/// R346), and a hit the Armor takes whole does nothing (R1361); Poisonous destroys with any damage
/// dealt (step 7); and what a Trample hit lands beyond the Taunt's health reaches the hero (step 9,
/// R63). A Taunt its attackers cannot destroy holds them all, and a destroyed Reborn Taunt comes back
/// and binds the attackers left (§4.5 step 4, `reborn_body`). Lane strike (R94), combat-only bonuses
/// (R1120) and Cleave stay out. Every remaining hit goes through `subsystems::projected_hero_damage`
/// with its attacker's Pierce (R346).
pub fn damage_past_taunts(state: &GameState, defender: PlayerId, attacks: &[(&CardInstance, i32)]) -> i32 {
    let mut free: Vec<(&CardInstance, i32)> = attacks.to_vec();
    free.sort_by_key(|(_, attack)| *attack);
    let mut taunts: Vec<(&CardInstance, UnitView)> = active_units_of(state, defender)
        .into_iter()
        .map(|unit| (unit, unit_view(state, unit)))
        .filter(|(_, view)| has_keyword(&view.keywords, KeywordKind::Taunt))
        .collect();
    taunts.sort_by_key(|(_, view)| view.health + view.armor);

    let mut total = 0;
    for (unit, view) in taunts {
        let mut standing = Some((unit.clone(), view));
        while let Some((taunt, view)) = standing.take() {
            let target = AttackTarget::Unit {
                instance: taunt.clone(),
            };
            let (bound, mut unbound): (Vec<_>, Vec<_>) = free
                .into_iter()
                .partition(|(attacker, _)| attack_restriction(state, attacker, &target).is_ok());
            let mut bound = bound.into_iter();
            let mut shield = has_keyword(&view.keywords, KeywordKind::DivineShield);
            let mut health = view.health;
            let mut poisoned = false;
            while health > 0 && !poisoned {
                let Some((attacker, attack)) = bound.next() else {
                    break;
                };
                // §4.3 step 1: an attacker the Taunt's First Strike destroys lands nothing.
                if falls_to_first_strike(state, attacker, &taunt, &view) {
                    continue;
                }
                // §4.4 step 1: the shield takes the whole hit.
                if shield {
                    shield = false;
                    continue;
                }
                // Step 2: the Taunt's Armor comes off each hit, unless it pierces (R346); a hit it takes
                // whole is no damage instance at all (R1361).
                let pierce = pierces(state, Some(attacker), None);
                let dealt = if pierce {
                    attack
                } else {
                    (attack - view.armor).max(0)
                };
                if dealt <= 0 {
                    continue;
                }
                let keywords = unit_view(state, attacker).keywords;
                // Step 9: what Trample lands beyond the Taunt's health is a hit on the hero (R63).
                if has_keyword(&keywords, KeywordKind::Trample) && dealt > health {
                    total += subsystems::projected_hero_damage(state, defender, dealt - health, pierce);
                }
                // Step 7: Poisonous destroys with any damage dealt.
                poisoned = has_keyword(&keywords, KeywordKind::Poisonous);
                health -= dealt;
            }
            unbound.extend(bound);
            unbound.sort_by_key(|(_, attack)| *attack);
            free = unbound;
            // §4.5 step 4: a destroyed Reborn Taunt comes back and binds what is left.
            if (health <= 0 || poisoned) && has_keyword(&view.keywords, KeywordKind::Reborn) {
                standing = reborn_body(state, &taunt);
            }
        }
    }

    for (attacker, hit) in &free {
        let pierce = pierces(state, Some(attacker), None);
        total += subsystems::projected_hero_damage(state, defender, *hit, pierce);
    }
    total
}

/// §4.3 step 1 and R93, as the engine's own projection reads it (`subsystems::lethal`'s
/// `falls_to_first_strike`, private there): a Taunt with First Strike, against an attacker without
/// it, strikes first, and an attacker that falls there never lands its hit. The strike back runs the
/// §4.4 steps that can stop it — Divine Shield, Armor (which a Pierce Taunt skips, R346),
/// Indestructible and the zero rule — and a Poisonous Taunt destroys with any damage at all. A Ranged
/// attacker takes no strike back from a Taunt that is not Ranged, so it never falls there (R783000).
fn falls_to_first_strike(
    state: &GameState,
    attacker: &CardInstance,
    taunt: &CardInstance,
    view: &UnitView,
) -> bool {
    if view.attack <= 0
        || !has_keyword(&view.keywords, KeywordKind::FirstStrike)
        || !takes_strike_back(state, attacker, taunt)
    {
        return false;
    }
    let mine = unit_view(state, attacker);
    if has_keyword(&mine.keywords, KeywordKind::FirstStrike)
        || has_keyword(&mine.keywords, KeywordKind::DivineShield)
        || has_keyword(&mine.keywords, KeywordKind::Indestructible)
    {
        return false;
    }
    let dealt = if pierces(state, Some(taunt), None) {
        view.attack
    } else {
        (view.attack - mine.armor).max(0)
    };
    dealt > 0 && (has_keyword(&view.keywords, KeywordKind::Poisonous) || dealt >= mine.health)
}

/// §4.5 step 4: the body a destroyed Reborn Taunt comes back as, a reset instance (R78) without
/// Reborn at `REBORN_HEALTH`, and its view — or nothing when the body has no Taunt. A printed Divine
/// Shield comes back with it; a granted Taunt, or Defense Position's, is gone.
fn reborn_body(state: &GameState, taunt: &CardInstance) -> Option<(CardInstance, UnitView)> {
    let mut body = taunt.clone();
    reset_instance(&mut body);
    body.reborn_spent = Some(true);
    body.damage = (unit_view(state, &body).max_health - REBORN_HEALTH).max(0);
    let view = unit_view(state, &body);
    has_keyword(&view.keywords, KeywordKind::Taunt).then_some((body, view))
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
    // §13.4, R964000: a hero's life is `effective_life`, own HP and the team's pool in a Duos duel,
    // the hero's health in a 1v1 game.
    let seat_life = effective_life(state, seat);
    let opp_life = effective_life(state, opp);
    let seat_first = next == NextSwing::Seat;
    let threat_share = if seat_first { w.answerable_threat } else { 1.0 };
    let threat = threat_share
        * (if enemy_damage >= seat_life {
            w.lethal_threat
        } else {
            w.threat_per_damage * f64::from(enemy_damage)
        });
    let pressure = if our_damage >= opp_life {
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
    let closing = w.closing_weight * progress * f64::from(starting_life(state, opp) - opp_life);

    let race = w.enemy_health * f64::from(opp_life);

    let board = material(state, seat, seat, w) - material(state, opp, seat, w);

    hero_side(state, seat, w) - hero_side(state, opp, w) + board - threat + pressure + closing - race
}
