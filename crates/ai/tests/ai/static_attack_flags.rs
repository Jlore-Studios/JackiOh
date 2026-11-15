//! Scorch's swings and Toxopid's Ignores Taunt in the AI's threat (#3909). A unit that can only attack the
//! Unit directly across from it (S+ #37 Scorch, `attacksOnlyAcross`) may attack no hero (R3550001,
//! `attack_restriction`), so `face_threat` counts none of its swings (one a turn since issue #3951, ten
//! before) and `wall_switches` offers no wall against them. An attacker that Ignores Taunt (S+ #121 Toxopid, `ignoresTaunt`) skips §4.2 step 3
//! (R3560007), so no Taunt binds it, Defense Position's included. The stand-ins: Mr. Vanilla, a 4/4; Core
//! #25, a 7/7 with Armor 7; and Midrange Menace, a 9/9 with Taunt.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, close, register_cards, unit};

const SCORCH: &str = "stinkyscapeplus-037";
const TOXOPID: &str = "stinkyscapeplus-121";
const VANILLA: &str = "core-008";
const BIG: &str = "core-025";
const MENACE: &str = "core-019";

/// AI_EVAL with the two flags read off: every swing may hit the hero and every Taunt binds, as before #3909.
const OFF: EvalWeights = EvalWeights {
    static_attack_flags_read: false,
    ..AI_EVAL
};

/// The AI's setup `ai` against the enemy's `enemy`, each a scenario side such as
/// `{ "health": 10, "field": [{ "def": BIG, "lane": 3 }] }`.
fn duel(ai: Value, enemy: Value) -> GameState {
    register_cards();
    scenario(json!({ "seed": "ai-static-flags", "p1": ai, "p2": enemy }))
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

/// A lone Scorch, with nothing across from it, reaches no hero on either side: its swing of 1 (Radiant 2)
/// counted none, where the old reading and the greedy yardstick count it.
#[test]
fn scorch_swings_at_no_hero_so_it_adds_no_face_threat() {
    for (radiant, before) in [(false, 1), (true, 2)] {
        let scorch = json!({ "field": [{ "def": SCORCH, "lane": 3, "radiant": radiant }] });

        let s = duel(json!({}), scorch.clone());
        assert_eq!(face_threat(&s, HUMAN), 0, "radiant {radiant}");
        assert_eq!(face_threat_with(&s, HUMAN, &OFF), before, "radiant {radiant}");
        assert_eq!(
            face_threat_with(&s, HUMAN, &GREEDY_EVAL),
            before,
            "radiant {radiant}"
        );

        let s = duel(scorch, json!({}));
        assert_eq!(face_threat(&s, AI), 0, "radiant {radiant}");
        assert_eq!(face_threat_with(&s, AI, &OFF), before, "radiant {radiant}");
    }
}

/// Leaving Scorch out takes only its own swings: a 4/4 beside it still counts.
#[test]
fn scorch_costs_the_rest_of_the_board_nothing() {
    let s = duel(
        json!({}),
        json!({ "field": [{ "def": SCORCH, "lane": 3 }, { "def": VANILLA, "lane": 1 }] }),
    );
    assert_eq!(face_threat(&s, HUMAN), 4);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 5);
    assert_eq!(face_threat_with(&s, HUMAN, &GREEDY_EVAL), 5);
}

/// At 1 life a lone Scorch read as a face hit looked lethal, and the 7/7's Taunt across from it would have
/// walled it; read as the engine applies it, nothing threatens and nothing calls for a wall.
#[test]
fn a_lone_scorch_calls_for_no_wall() {
    let s = duel(
        json!({ "health": 1, "field": [{ "def": BIG, "lane": 3 }] }),
        json!({ "field": [{ "def": SCORCH, "lane": 3 }] }),
    );
    assert_eq!(face_threat(&s, HUMAN), 0);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 1);
    assert!(walls(&s).is_empty(), "{:?}", walls(&s));
    // Under the old reading the switch soaked the swing (the 7/7's health), which is why it was offered.
    assert_eq!(face_threat_with(&act(&s, AI, wall(&s)), HUMAN, &OFF), 0);

    // Control: a 4/4 in the 7/7's lane is lethal at 4 life, and the switch walls it.
    let c = duel(
        json!({ "health": 4, "field": [{ "def": BIG, "lane": 1 }] }),
        json!({ "field": [{ "def": VANILLA, "lane": 1 }, { "def": SCORCH, "lane": 3 }] }),
    );
    assert_eq!(face_threat(&c, HUMAN), 4);
    assert_eq!(walls(&c), vec![wall(&c)]);
}

/// A Toxopid swings past the Taunt Defense Position grants, and past a printed Taunt.
#[test]
fn toxopid_walks_past_a_defense_position_unit() {
    for (radiant, attack) in [(false, 10), (true, 20)] {
        let s = duel(
            json!({ "field": [{ "def": VANILLA, "lane": 3, "position": "DEF" }] }),
            json!({ "field": [{ "def": TOXOPID, "lane": 3, "radiant": radiant }] }),
        );
        assert_eq!(face_threat(&s, HUMAN), attack, "radiant {radiant}");
        assert_eq!(face_threat_with(&s, HUMAN, &OFF), 0, "radiant {radiant}");
        assert_eq!(face_threat_with(&s, HUMAN, &GREEDY_EVAL), 0, "radiant {radiant}");
    }

    let s = duel(
        json!({ "field": [{ "def": MENACE, "lane": 1 }] }),
        json!({ "field": [{ "def": TOXOPID, "lane": 3 }] }),
    );
    assert_eq!(face_threat(&s, HUMAN), 10);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 0);
}

/// Only the Taunt-ignorer walks past: the Taunt still binds every other attacker.
#[test]
fn a_taunt_still_walls_every_other_attacker() {
    let s = duel(
        json!({ "field": [{ "def": MENACE, "lane": 1 }] }),
        json!({ "field": [{ "def": TOXOPID, "lane": 3 }, { "def": VANILLA, "lane": 1 }] }),
    );
    assert_eq!(face_threat(&s, HUMAN), 10);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 0);

    let s = duel(
        json!({ "field": [{ "def": MENACE, "lane": 1 }] }),
        json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
    );
    assert_eq!(face_threat(&s, HUMAN), 0);
    assert_eq!(face_threat_with(&s, HUMAN, &OFF), 0);
}

/// At 10 life against a Toxopid, the 7/7's switch to Defense Position used to look like it walled the
/// lethal swing; it walls nothing, so it is not offered.
#[test]
fn a_toxopid_makes_a_defense_switch_pointless() {
    let s = duel(
        json!({ "health": 10, "field": [{ "def": BIG, "lane": 1 }] }),
        json!({ "field": [{ "def": TOXOPID, "lane": 3 }] }),
    );
    assert_eq!(face_threat(&s, HUMAN), 10);
    assert!(walls(&s).is_empty(), "{:?}", walls(&s));

    let walled = act(&s, AI, wall(&s));
    assert_eq!(face_threat(&walled, HUMAN), 10);
    assert_eq!(face_threat_with(&walled, HUMAN, &OFF), 0);
}

/// The evaluation drops the lethal threat an enemy Scorch would pose the AI, and the lethal pressure the
/// AI's own Scorch would put on the enemy.
#[test]
fn the_evaluation_drops_the_lethal_threat_and_pressure_these_flags_wall_off() {
    let s = duel(
        json!({ "health": 1 }),
        json!({ "field": [{ "def": SCORCH, "lane": 3 }] }),
    );
    close(
        evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(&s, AI, NextSwing::Enemy, &OFF),
        AI_EVAL.lethal_threat,
    );

    let s = duel(
        json!({ "field": [{ "def": SCORCH, "lane": 3 }] }),
        json!({ "health": 1 }),
    );
    close(
        evaluate(&s, AI, NextSwing::Enemy, &OFF) - evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.lethal_pressure,
    );
}

/// The lethal solver's gap reads Ignores Taunt as well: the AI's Toxopid lands its 10 past the enemy's
/// Menace, leaving 5 of the enemy's 15.
#[test]
fn the_lethal_gap_reads_ignores_taunt_too() {
    let s = duel(
        json!({ "field": [{ "def": TOXOPID, "lane": 3 }] }),
        json!({ "health": 15, "field": [{ "def": MENACE, "lane": 1 }] }),
    );
    let toxopid = unit(&s, AI, TOXOPID);
    let attacks = [(toxopid, 10, true)];
    assert_eq!(damage_past_taunts_with(&s, HUMAN, &attacks, &AI_EVAL), 10);
    assert_eq!(damage_past_taunts_with(&s, HUMAN, &attacks, &OFF), 0);
    assert_eq!(damage_past_taunts(&s, HUMAN, &attacks), 10);
    close(ready_gap(&s, AI), 5.0);
}

/// With neither card on the field, the flag changes no number, on either side.
#[test]
fn without_these_cards_the_threat_is_unchanged() {
    for (ai, enemy) in [
        (
            json!({ "field": [{ "def": BIG, "lane": 3 }] }),
            json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }] }),
        ),
        (
            json!({ "field": [{ "def": BIG, "lane": 1, "position": "DEF" }] }),
            json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
        ),
        (
            json!({ "field": [{ "def": MENACE, "lane": 1 }] }),
            json!({ "field": [{ "def": BIG, "lane": 5 }, { "def": VANILLA, "lane": 4 }] }),
        ),
    ] {
        let s = duel(ai.clone(), enemy.clone());
        for seat in [AI, HUMAN] {
            assert_eq!(
                face_threat(&s, seat),
                face_threat_with(&s, seat, &OFF),
                "{seat:?}: {ai} vs {enemy}"
            );
        }
    }
}

/// The greedy yardstick stays frozen: GREEDY_EVAL leaves the flag off.
#[test]
fn greedy_eval_keeps_the_old_reading() {
    const { assert!(AI_EVAL.static_attack_flags_read) };
    const { assert!(!GREEDY_EVAL.static_attack_flags_read) };
}
