//! A unit its own Brittle tick destroys before it can attack (#3793, R3793000): the tick runs at the start
//! of its controller's turn, before that turn's attacks, so `face_threat` counts none of its attacks, and
//! neither `wall_switches` nor `stands_down` offers a switch on its account. The cards: Mr. Vanilla, a 4/4;
//! Core #25, a 4-mana 7/7 with Armor 7; Core #66 The Rock, a 10/10 with Indestructible; and Core #89 Corpse
//! Eater (Rush, Brittle 1).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, close, in_graveyard, on_field, register_cards, unit};

const VANILLA: &str = "core-008";
const BIG: &str = "core-025";
const ROCK: &str = "core-066";
const EATER: &str = "core-089";

/// A library for each side, so an end of turn draws a card and no fatigue moves a number.
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// AI_EVAL with the crumble read off: every unit that may swing counts, as before #3793.
const OFF: EvalWeights = EvalWeights {
    brittle_crumbles_before_swing: false,
    ..AI_EVAL
};

/// One side at `health` with `field` on it.
fn side(health: i32, field: Value) -> Value {
    json!({ "health": health, "field": field, "library": LIBRARY })
}

/// A board on the scenario's turn 9, the AI on turn.
fn board(p1: Value, p2: Value) -> GameState {
    register_cards();
    scenario(json!({ "seed": "ai-brittle", "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// `def` in lane 3, in Attack Position.
fn lane(def: &str) -> Value {
    json!({ "def": def, "lane": 3 })
}

/// Give each of `player`'s units a Brittle count of `count`, started on turn `since`.
fn crumbling(state: &mut GameState, player: PlayerId, count: i32, since: i32) {
    let ids: Vec<String> = active_units_of(state, player)
        .into_iter()
        .map(|unit| unit.id.clone())
        .collect();
    for id in ids {
        find_instance_mut(state, &id).expect("the unit").brittle = Some(BrittleCounter {
            count,
            since,
            printed: None,
        });
    }
}

/// The AI at 7 life with the 7/7 in lane 3, against a 4/4 in each of lanes 2 to 4: a threat of 12, lethal,
/// which the 7/7's Taunt in Defense Position walls (as `gauntlet.rs`'s board without The Gauntlet).
fn lethal() -> GameState {
    board(
        side(7, json!([lane(BIG)])),
        side(
            30,
            json!([{ "def": VANILLA, "lane": 2 }, lane(VANILLA), { "def": VANILLA, "lane": 4 }]),
        ),
    )
}

/// The AI's 4/4 against `theirs`, both in lane 3.
fn duel(theirs: &str) -> GameState {
    board(side(30, json!([lane(VANILLA)])), side(30, json!([lane(theirs)])))
}

/// The AI's 4/4's switch to Defense Position.
fn stand_down(state: &GameState) -> ActionBody {
    ActionBody::SwitchPosition {
        instance_id: unit(state, AI, VANILLA).id.clone(),
    }
}

/// The AI's 4/4 against an empty board and a hero at 4 life: lethal pressure.
fn own() -> GameState {
    board(side(30, json!([lane(VANILLA)])), side(4, json!([])))
}

/// The 7/7's switch to Defense Position.
fn wall(state: &GameState) -> ActionBody {
    ActionBody::SwitchPosition {
        instance_id: unit(state, AI, BIG).id.clone(),
    }
}

/// The walls the AI is offered on `state`.
fn walls(state: &GameState) -> Vec<ActionBody> {
    wall_switches(state, AI, &candidate_actions(state, AI, Prices::NONE))
}

/// Started on turn 8, a Brittle 1 count first ticks at the human's start of turn 10, before the human's
/// attacks: the three 4/4s threaten nothing, so there is no lethal to wall.
#[test]
fn r3793000_a_unit_whose_brittle_tick_falls_before_its_next_swing_adds_nothing_to_the_face_threat() {
    let mut s = lethal();
    assert_eq!(face_threat(&s, HUMAN), 12);
    assert_eq!(walls(&s), vec![wall(&s)]);

    crumbling(&mut s, HUMAN, 1, 8);
    assert_eq!(face_threat(&s, HUMAN), 0);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 12);
    assert!(walls(&s).is_empty(), "{:?}", walls(&s));
    close(
        evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(&s, AI, NextSwing::Enemy, &OFF),
        AI_EVAL.lethal_threat,
    );
}

/// The 7/7 would kill the AI's 4/4 and survive, so the 4/4 stands down, until the 7/7 crumbles first.
#[test]
fn r3793000_a_crumbling_enemy_unit_stands_nothing_down() {
    let mut s = duel(BIG);
    assert!(stands_down(&s, &stand_down(&s)));

    crumbling(&mut s, HUMAN, 1, 8);
    assert!(!stands_down(&s, &stand_down(&s)));
}

/// A count started on turn 9 first ticks at turn 11, after the human's turn 10; a count of 2 only drops to 1.
#[test]
fn r3793000_a_unit_whose_tick_is_not_yet_due_or_leaves_it_standing_still_counts() {
    let mut s = lethal();
    crumbling(&mut s, HUMAN, 1, 9);
    assert_eq!(face_threat(&s, HUMAN), 12);
    assert_eq!(walls(&s), vec![wall(&s)]);

    let mut s = lethal();
    crumbling(&mut s, HUMAN, 2, 8);
    assert_eq!(face_threat(&s, HUMAN), 12);

    let mut s = duel(BIG);
    crumbling(&mut s, HUMAN, 1, 9);
    assert!(stands_down(&s, &stand_down(&s)));
}

/// R46: Indestructible ignores the crumble's destroy, so The Rock still swings.
#[test]
fn r3793000_an_indestructible_unit_still_counts() {
    let mut s = duel(ROCK);
    crumbling(&mut s, HUMAN, 1, 8);
    assert_eq!(face_threat(&s, HUMAN), 10);
    assert!(stands_down(&s, &stand_down(&s)));
}

/// The AI's own 4/4, its count started on turn 9, ticks at the AI's start of turn 11 and crumbles before
/// it can swing at the enemy's 4 life.
#[test]
fn r3793000_the_ais_own_crumbling_unit_adds_no_lethal_pressure() {
    let mut s = own();
    crumbling(&mut s, AI, 1, 9);
    assert_eq!(face_threat(&s, AI), 0);
    assert_eq!(face_threat_with(&s, AI, &OFF), 4);
    close(
        evaluate(&s, AI, NextSwing::Enemy, &OFF) - evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.lethal_pressure,
    );
}

/// Read as the start of a side's turn (`NextSwing::Seat` with the seat on turn, or the enemy on turn), the
/// side on turn has its tick behind it and swings this turn; the other side's next start is still to come.
#[test]
fn r3793000_a_side_that_swings_first_on_its_own_turn_swings_before_its_next_tick() {
    // (a) The AI on turn swings first: its own 4/4 swings before the tick that would crumble it.
    let mut s = own();
    crumbling(&mut s, AI, 1, 9);
    close(
        evaluate(&s, AI, NextSwing::Seat, &AI_EVAL),
        evaluate(&s, AI, NextSwing::Seat, &OFF),
    );

    // (b) The enemy's 4/4s still crumble at its start of turn 10, so the answerable threat goes too.
    let mut s = lethal();
    crumbling(&mut s, HUMAN, 1, 8);
    close(
        evaluate(&s, AI, NextSwing::Seat, &AI_EVAL) - evaluate(&s, AI, NextSwing::Seat, &OFF),
        AI_EVAL.answerable_threat * AI_EVAL.lethal_threat,
    );

    // (c) The human on turn 10, its count started on turn 9: its tick is due at its start of turn 12. Read
    // at the end of its turn its 4/4s crumble first; read with the enemy swinging next, they swing now.
    let mut s = lethal();
    s.turn = 10;
    s.active = HUMAN;
    crumbling(&mut s, HUMAN, 1, 9);
    assert_eq!(face_threat(&s, HUMAN), 0);
    close(
        evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&s, AI, NextSwing::Enemy, &OFF),
    );
}

/// Core #89 grown to 8/8 and set on the human's field on turn 9 starts its printed Brittle 1 then: it first
/// ticks at turn 11, so it swings on turn 10 at the AI's 8 life. Started on turn 8, it crumbles at the human's
/// start of turn 10 and is no lethal.
#[test]
fn r3793000_a_grown_corpse_eater_no_longer_reads_as_lethal() {
    let mut s = board(
        side(8, json!([])),
        side(
            30,
            json!([{ "def": EATER, "lane": 3, "statsOverride": { "attack": 8, "health": 8 } }]),
        ),
    );
    assert_eq!(active_brittle_count(unit(&s, HUMAN, EATER)), Some(1));
    assert_eq!(face_threat(&s, HUMAN), 8);

    let id = unit(&s, HUMAN, EATER).id.clone();
    let eater = find_instance_mut(&mut s, &id).expect("the Corpse Eater");
    eater.brittle = eater
        .brittle
        .map(|brittle| BrittleCounter { since: 8, ..brittle });
    assert_eq!(face_threat(&s, HUMAN), 0);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 8);
    close(
        evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(&s, AI, NextSwing::Enemy, &OFF),
        AI_EVAL.lethal_threat,
    );
}

/// The greedy yardstick stays frozen: GREEDY_EVAL leaves the flag off and still counts a crumbling unit, and
/// its score is the same as on the board without the counts (its `brittle_discount` is 0).
#[test]
fn r3793000_greedy_eval_keeps_counting_a_crumbling_unit() {
    const { assert!(AI_EVAL.brittle_crumbles_before_swing) };
    const { assert!(!GREEDY_EVAL.brittle_crumbles_before_swing) };
    let plain = lethal();
    let mut crumbled = lethal();
    crumbling(&mut crumbled, HUMAN, 1, 8);
    assert_eq!(face_threat_with(&crumbled, HUMAN, &GREEDY_EVAL), 12);
    close(
        evaluate(&plain, AI, NextSwing::Enemy, &GREEDY_EVAL),
        evaluate(&crumbled, AI, NextSwing::Enemy, &GREEDY_EVAL),
    );
}

/// R846: an extra turn owed to the AI starts as its turn ends, as turn 10, so its count started on turn 9
/// is not due yet and its 4/4 swings; one started on turn 8 is due then and crumbles it.
#[test]
fn r3793000_an_owed_extra_turn_is_the_controllers_next_start() {
    let mut s = own();
    crumbling(&mut s, AI, 1, 9);
    s.players[AI].extra_turns = Some(1);
    assert_eq!(face_threat(&s, AI), 4);
    let next = act(&s, AI, &ActionBody::EndTurn);
    assert_eq!((next.turn, next.active), (10, AI));
    assert!(on_field(&next, AI, VANILLA));

    let mut s = own();
    crumbling(&mut s, AI, 1, 8);
    s.players[AI].extra_turns = Some(1);
    assert_eq!(face_threat(&s, AI), 0);
    let next = act(&s, AI, &ActionBody::EndTurn);
    assert_eq!((next.turn, next.active), (10, AI));
    assert!(in_graveyard(&next, AI, VANILLA));
}

/// The engine's own tick crumbles exactly the units the read leaves out, on either side.
#[test]
fn r3793000_the_engines_own_tick_agrees_with_the_read() {
    // The AI's count started on turn 9 ticks at its start of turn 11, after the human's turn 10.
    let mut s = own();
    crumbling(&mut s, AI, 1, 9);
    assert_eq!(face_threat(&s, AI), 0);
    let human_turn = act(&s, AI, &ActionBody::EndTurn);
    assert!(on_field(&human_turn, AI, VANILLA));
    let next = act(&human_turn, HUMAN, &ActionBody::EndTurn);
    assert_eq!((next.turn, next.active), (11, AI));
    assert!(in_graveyard(&next, AI, VANILLA));

    // The human's: started on turn 9 it lives through its start of turn 10, started on turn 8 it does not.
    let quiet = || board(side(30, json!([])), side(30, json!([lane(VANILLA)])));
    let mut s = quiet();
    crumbling(&mut s, HUMAN, 1, 9);
    assert_eq!(face_threat(&s, HUMAN), 4);
    assert!(on_field(&act(&s, AI, &ActionBody::EndTurn), HUMAN, VANILLA));

    let mut s = quiet();
    crumbling(&mut s, HUMAN, 1, 8);
    assert_eq!(face_threat(&s, HUMAN), 0);
    assert!(in_graveyard(&act(&s, AI, &ActionBody::EndTurn), HUMAN, VANILLA));
}
