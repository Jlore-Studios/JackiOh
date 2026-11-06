//! The Gauntlet (S #157) in the AI's threat (#3287): while it acts, a Unit in its lanes may attack no hero
//! (R1849005, `attack_restriction`), so `face_threat` counts none of its attacks and `wall_switches` offers
//! no wall against them. The cards: Core #25, a 7/7 with Armor 7, and Mr. Vanilla, a 4/4.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, register_cards, unit};

const GAUNTLET: &str = "stinkyscape-157";
const BIG: &str = "core-025";
const VANILLA: &str = "core-008";

/// AI_EVAL with the Gauntlet read off: every unit that may swing counts, as before #3287.
const OFF: EvalWeights = EvalWeights {
    gauntlet_bars_face_attacks: false,
    ..AI_EVAL
};

/// The AI at `health` with the 7/7 in Attack Position in `big_lane`, against a 4/4 in each of `lanes`, and
/// The Gauntlet in `gauntlet`'s backrow (none with `None`).
fn board(gauntlet: Option<PlayerId>, health: i32, big_lane: i32, lanes: &[i32]) -> GameState {
    register_cards();
    let backrow = |seat: PlayerId| {
        if gauntlet == Some(seat) {
            json!([GAUNTLET])
        } else {
            json!([])
        }
    };
    let enemies: Vec<Value> = lanes
        .iter()
        .map(|lane| json!({ "def": VANILLA, "lane": lane }))
        .collect();
    scenario(json!({
        "seed": "ai-gauntlet",
        "p1": { "health": health, "field": [{ "def": BIG, "lane": big_lane }], "backrow": backrow(AI) },
        "p2": { "field": enemies, "backrow": backrow(HUMAN) },
    }))
    .state()
    .clone()
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

/// Three 4/4s inside The Gauntlet, whichever seat holds it, reach no hero: no face threat, so no wall at 7
/// life, where the same board without The Gauntlet is lethal and the 7/7's switch walls all three.
#[test]
fn the_gauntlet_leaves_the_middle_lanes_no_face_threat_and_no_wall() {
    for seat in [AI, HUMAN] {
        let s = board(Some(seat), 7, 3, &[2, 3, 4]);
        assert!(gauntlet_acts(&s), "{seat:?}");
        assert_eq!(face_threat(&s, HUMAN), 0, "{seat:?}");
        assert_eq!(face_threat_with(&s, HUMAN, &OFF), 12, "{seat:?}");
        // Off, the 7/7 inside would bind the three, which land nothing anyway.
        assert!(walls(&s).is_empty(), "{seat:?}: {:?}", walls(&s));
    }

    let c = board(None, 7, 3, &[2, 3, 4]);
    assert!(!gauntlet_acts(&c));
    assert_eq!(face_threat(&c, HUMAN), 12);
    assert_eq!(walls(&c), vec![wall(&c)]);
}

/// A 4/4 in lane 1 or 5 stands outside The Gauntlet: its attack still counts, and a Taunt in its lane walls
/// it, where a Taunt inside The Gauntlet binds no attacker outside it.
#[test]
fn a_unit_outside_the_gauntlet_still_threatens_the_face() {
    for seat in [AI, HUMAN] {
        // Only lane 1's 4/4 threatens, below 5 life, so nothing calls for a wall.
        let s = board(Some(seat), 5, 1, &[1, 2, 3]);
        assert_eq!(face_threat(&s, HUMAN), 4, "{seat:?}");
        assert!(walls(&s).is_empty(), "{seat:?}: {:?}", walls(&s));

        // At 4 life it is lethal, and the 7/7's Taunt in lane 1 binds it.
        let s = board(Some(seat), 4, 1, &[1, 2, 3]);
        assert_eq!(face_threat(&s, HUMAN), 4, "{seat:?}");
        assert_eq!(walls(&s), vec![wall(&s)], "{seat:?}");

        // The 7/7 inside The Gauntlet: its Taunt binds no attacker outside, so its switch lowers nothing.
        let s = board(Some(seat), 4, 3, &[1, 2, 3]);
        assert_eq!(face_threat(&s, HUMAN), 4, "{seat:?}");
        assert!(walls(&s).is_empty(), "{seat:?}: {:?}", walls(&s));

        let s = board(Some(seat), 7, 3, &[1, 5]);
        assert_eq!(face_threat(&s, HUMAN), 8, "{seat:?}");
    }
}

/// With no Gauntlet on the field, the flag changes no number, on either side.
#[test]
fn without_the_gauntlet_the_threat_is_unchanged() {
    for (big_lane, lanes, threat) in [
        (3, &[2, 3, 4][..], 12),
        (1, &[1, 2, 3][..], 12),
        (3, &[1, 5][..], 8),
    ] {
        let s = board(None, 7, big_lane, lanes);
        assert_eq!(face_threat(&s, HUMAN), threat, "{big_lane} {lanes:?}");
        assert_eq!(face_threat_with(&s, HUMAN, &OFF), threat, "{big_lane} {lanes:?}");
        assert_eq!(
            face_threat(&s, AI),
            face_threat_with(&s, AI, &OFF),
            "{big_lane} {lanes:?}"
        );
    }
}

/// The greedy yardstick stays frozen: GREEDY_EVAL leaves the flag off and still counts the Gauntlet's
/// Units at the face.
#[test]
fn greedy_eval_keeps_counting_the_gauntlet_s_units() {
    const { assert!(AI_EVAL.gauntlet_bars_face_attacks) };
    const { assert!(!GREEDY_EVAL.gauntlet_bars_face_attacks) };
    let s = board(Some(HUMAN), 7, 3, &[2, 3, 4]);
    assert_eq!(face_threat_with(&s, HUMAN, &GREEDY_EVAL), 12);
    assert_eq!(face_threat(&s, HUMAN), 0);
}

/// The evaluation drops the lethal threat the Gauntlet's Units would pose the AI, and the lethal pressure
/// the AI's own Units inside it would put on the enemy.
#[test]
fn the_evaluation_drops_the_lethal_threat_and_pressure_the_gauntlet_walls_off() {
    // The enemy's 12 would reach the AI's 7 life. The 7/7 in lane 1 threatens 7 either way, far below the
    // enemy's life, so the pressure term is the same.
    let s = board(Some(AI), 7, 1, &[2, 3, 4]);
    assert_eq!(face_threat(&s, AI), face_threat_with(&s, AI, &OFF));
    close(
        evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(&s, AI, NextSwing::Enemy, &OFF),
        AI_EVAL.lethal_threat,
    );

    // The AI's own three 4/4s inside its Gauntlet would reach the enemy's 7 life.
    register_cards();
    let s = scenario(json!({
        "seed": "ai-gauntlet-own",
        "p1": {
            "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }, { "def": VANILLA, "lane": 4 }],
            "backrow": [GAUNTLET],
        },
        "p2": { "health": 7, "field": [{ "def": BIG, "lane": 1 }] },
    }))
    .state()
    .clone();
    assert_eq!(face_threat(&s, AI), 0);
    assert_eq!(face_threat_with(&s, AI, &OFF), 12);
    close(
        evaluate(&s, AI, NextSwing::Enemy, &OFF) - evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.lethal_pressure,
    );
}
