//! The AI's static reads of a hit's riders through §4.4 step 7 (#5229, R5229000): a hit that leaves the
//! hero life adds the Burn its source states (S+ #114 Reign's "Damage also Burns 4", S+ #151 Volcanic
//! Rounds' "Damage dealt also Burns", R3540002, R4783001), and the Taunt walk counts that Burn as one more
//! unpierced hit, its tick at the turn's end (R4154000). Each board is also played through the engine,
//! which deals what the static read says.

use jackioh_ai::*;
use jackioh_engine::hero_burn_of;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const REIGN: &str = "stinkyscapeplus-114";
const VOLCANIC: &str = "stinkyscapeplus-151";
const VANILLA: &str = "core-008";

/// AI_EVAL with the damage riders read off: a hit adds no counter, as before #5229.
const OFF: EvalWeights = EvalWeights {
    damage_rider_read: false,
    ..AI_EVAL
};

/// p1 (the AI, on turn in its main phase) and p2 as given, each with plain cards to draw and no turn that
/// ends by itself, so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_cards();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        side["library"] = json!([VANILLA; 6].to_vec());
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [AI, HUMAN] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// The AI's Reign, a 12/8 Windfury Pierce, against the enemy as given: the scenario and the Reign's id.
fn reign_into(seed: &str, enemy: Value) -> (Scenario, String) {
    let s = board(seed, json!({ "field": [REIGN] }), enemy);
    let reign = s.unit(AI, 1).expect("the Reign stands in lane 1").id;
    (s, reign)
}

/// R5229000: Reign's two swings at a hero on 30 deal 24 and leave Burn 8, whose tick at the turn's end
/// takes the last 8: `face_threat` reads 32, where with the riders unread it reads 24, and `ready_gap` reads
/// the lethal. The engine plays it so.
#[test]
fn r5229000_reign_swinging_twice_at_a_hero_on_30_reads_32() {
    let (mut s, reign) = reign_into("r5229000-reign", json!({ "health": 30 }));
    assert_eq!(face_threat(s.state(), AI), 32);
    assert_eq!(face_threat_with(s.state(), AI, &OFF), 24);
    assert_eq!(face_threat_with(s.state(), AI, &GREEDY_EVAL), 24);
    assert!(ready_gap(s.state(), AI) <= 0.0, "{}", ready_gap(s.state(), AI));

    s.attack(reign.as_str(), "hero");
    s.attack(reign.as_str(), "hero");
    s.expect_health(HUMAN, 6);
    assert_eq!(hero_burn_of(s.state(), HUMAN), 8);
    assert!(s.state().result.is_none());
    s.end_turn();
    let result = s.state().result.expect("the Burn's tick ends the game");
    assert_eq!(result.winner, Winner::P1);
}

/// R5229000: the tick is unpierced, so a hero's Armor 3 takes 3 of the Burn 8 (29 read, the hero left on 1),
/// and a hit that leaves the hero no life adds no Burn: on a hero at 12 the first swing is lethal alone,
/// and the read is the 24 the swings deal.
#[test]
fn r5229000_the_tick_meets_armor_and_a_lethal_hit_leaves_no_burn() {
    let (mut s, reign) = reign_into("r5229000-armor", json!({ "health": 30, "armor": 3 }));
    assert_eq!(face_threat(s.state(), AI), 29);
    assert_eq!(face_threat_with(s.state(), AI, &OFF), 24);
    s.attack(reign.as_str(), "hero");
    s.attack(reign.as_str(), "hero");
    s.end_turn();
    s.expect_health(HUMAN, 1);
    assert!(s.state().result.is_none());

    let (s, _) = reign_into("r5229000-lethal", json!({ "health": 12 }));
    assert_eq!(face_threat(s.state(), AI), 24);
}

/// R5229000, R4783001: a 6/2 Volcanic Rounds' hit on a hero Burns the 6 it dealt, so its threat reads 12,
/// where with the riders unread it reads 6; the engine leaves the hero at 30 less 12.
#[test]
fn r5229000_a_damage_dealt_also_burns_hit_counts_what_it_dealt() {
    let mut s = board(
        "r5229000-volcanic",
        json!({ "field": [{ "def": VOLCANIC, "statsOverride": { "attack": 6, "health": 2 } }] }),
        json!({ "health": 30 }),
    );
    let rounds = s.unit(AI, 1).expect("the Rounds stands in lane 1").id;
    assert_eq!(face_threat(s.state(), AI), 12);
    assert_eq!(face_threat_with(s.state(), AI, &OFF), 6);
    assert_eq!(face_threat_with(s.state(), AI, &GREEDY_EVAL), 6);
    s.attack(rounds.as_str(), "hero");
    s.expect_health(HUMAN, 24);
    s.end_turn();
    s.expect_health(HUMAN, 18);
}
