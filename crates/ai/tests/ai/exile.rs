//! A hit that exiles the Unit it damages (#4065, R4065001): Meditative #72 The Banisher's damage marks a Unit
//! and the next state check exiles it (R1124), so the AI's count of the damage past its Taunts reads that hit
//! as it reads a Poisonous one: it removes a Taunt it deals 1 or more to, with no Reborn body back, and a First
//! Strike Banisher on guard stops an attacker it damages whole. Each board is then stepped through the engine.
//! The cards: The Banisher, a 2/1 (Radiant 4/2 with Cleave); Core #19 Midrange Menace (Taunt) set to a 0/8
//! wall; Mr. Vanilla; Core #30 Archivist; and Core #67 Zoomerbin Oomen, a 1/2.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const BANISHER: &str = "meditative-072";
const MENACE: &str = "core-019";
const VANILLA: &str = "core-008";
const ARCHIVIST: &str = "core-030";
const OOMEN: &str = "core-067";

/// A library for each side, so an end of turn draws a card and no fatigue moves a number.
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// AI_EVAL with the exile read off: a hit that exiles reads as plain damage, as before #4065.
const OFF: EvalWeights = EvalWeights {
    exile_on_damage_read: false,
    ..AI_EVAL
};

/// One side at 30 life with `field` on it.
fn side(field: Value) -> Value {
    json!({ "health": 30, "field": field, "library": LIBRARY })
}

/// A board of `p1` against `p2`, on the scenario's turn 9 with the AI on turn.
fn game(p1: Value, p2: Value) -> Scenario {
    register_cards();
    scenario(json!({ "seed": "ai-exile", "p1": p1, "p2": p2 }))
}

/// The AI's 0/8 Taunt in `lane`.
fn wall(lane: i32) -> Value {
    json!({ "def": MENACE, "lane": lane, "statsOverride": { "attack": 0, "health": 8 } })
}

/// The human's 8/8 in `lane`.
fn big(lane: i32) -> Value {
    json!({ "def": VANILLA, "lane": lane, "statsOverride": { "attack": 8, "health": 8 } })
}

/// The human's The Banisher in `lane`.
fn banisher(lane: i32) -> Value {
    json!({ "def": BANISHER, "lane": lane })
}

/// The AI's wall, given `keywords`, in lane 3 against the human's The Banisher and 8/8.
fn walled(keywords: &[Keyword]) -> Scenario {
    let mut s = game(side(json!([wall(3)])), side(json!([banisher(2), big(4)])));
    s.card_mut(MENACE)
        .granted_keywords
        .extend(keywords.iter().cloned());
    s
}

/// The Banisher's 2 exiles the wall, so the Oomen's 1 and the 8/8's 8 reach the AI: 9. Read as plain damage,
/// the 8/8 breaks the wall and 1 and 2 go on: 3, and the greedy yardstick's smallest-first walk spends all
/// three on it.
#[test]
fn r4065001_an_enemy_banisher_removes_a_taunt_in_the_walk() {
    let mut s = game(
        side(json!([wall(3)])),
        side(json!([{ "def": OOMEN, "lane": 1 }, banisher(2), big(4)])),
    );
    assert_eq!(face_threat(s.state(), HUMAN), 9);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 3);
    assert_eq!(face_threat_with(s.state(), HUMAN, &GREEDY_EVAL), 0);

    let menace = s.card(MENACE).clone();
    s.end_turn();
    s.attack(BANISHER, &menace).expect_in_zone(&menace, "exile");
    s.attack(OOMEN, "hero")
        .attack(VANILLA, "hero")
        .expect_health(AI, 21);
}

/// R1124: an exiled Reborn wall never comes back, so the 8/8 goes on: 8. Read as plain damage, the 8/8 breaks
/// it and the Reborn body binds the Banisher: 0.
#[test]
fn r4065001_an_exiled_reborn_taunt_never_comes_back() {
    let mut s = walled(&[Keyword::Reborn]);
    assert_eq!(face_threat(s.state(), HUMAN), 8);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 0);

    let menace = s.card(MENACE).clone();
    s.end_turn();
    s.attack(BANISHER, &menace).expect_in_zone(&menace, "exile");
    assert!(s.unit(AI, 3).is_none(), "{:?}", s.unit(AI, 3));
    s.attack(VANILLA, "hero").expect_health(AI, 22);
}

/// §4.4 steps 1 and 2: a Divine Shield takes the Banisher's whole hit and Armor 2 its 2, so it deals nothing
/// and exiles nothing; the 8/8 alone does not break the wall.
#[test]
fn r4065001_a_hit_a_shield_or_armor_takes_exiles_nothing() {
    for keyword in [Keyword::DivineShield, Keyword::Armor { n: 2 }] {
        let mut s = walled(std::slice::from_ref(&keyword));
        assert_eq!(face_threat(s.state(), HUMAN), 0, "{keyword:?}");
        assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 0, "{keyword:?}");

        let menace = s.card(MENACE).clone();
        s.end_turn();
        s.attack(BANISHER, &menace).expect_in_zone(&menace, "field");
    }
}

/// §4.3 step 1: the AI's First Strike Banisher, on guard in Defense Position, strikes each attacker first, and
/// its 2 exiles it before it strikes, so neither the 5/5 nor the 8/8 gets past: 0. Read as plain damage, the
/// 5/5 survives the 2 and breaks the 2/1, and the 8/8 goes on: 8.
#[test]
fn r4065001_a_first_strike_banisher_on_guard_stops_an_attacker_whole() {
    let mut s = game(
        side(json!([{ "def": BANISHER, "lane": 3, "position": "DEF" }])),
        side(json!([
            { "def": VANILLA, "lane": 2, "statsOverride": { "attack": 5, "health": 5 } },
            { "def": ARCHIVIST, "lane": 4, "statsOverride": { "attack": 8, "health": 8 } },
        ])),
    );
    s.card_mut(BANISHER).granted_keywords.push(Keyword::FirstStrike);
    assert_eq!(face_threat(s.state(), HUMAN), 0);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 8);

    let (banisher, vanilla) = (s.card(BANISHER).clone(), s.unit_at(HUMAN, 2));
    s.end_turn();
    s.attack(VANILLA, &banisher)
        .expect_in_zone(&vanilla, "exile")
        .expect_in_zone(&banisher, "field");
}

/// R63: a Radiant Banisher's Cleave splash is a hit of its own, so it exiles the wall beside its target too,
/// and the 8/8 goes on: 8. Read as plain damage the splash leaves that wall at 4, which binds the 8/8: 0.
#[test]
fn r4065001_a_cleaving_banisher_exiles_the_taunt_beside_its_target() {
    let mut s = game(
        side(json!([wall(2), wall(3)])),
        side(json!([{ "def": BANISHER, "lane": 1, "radiant": true }, big(4)])),
    );
    assert_eq!(face_threat(s.state(), HUMAN), 8);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 0);

    let (left, right) = (s.unit_at(AI, 2), s.unit_at(AI, 3));
    s.end_turn();
    s.attack(BANISHER, &left)
        .expect_in_zone(&left, "exile")
        .expect_in_zone(&right, "exile");
    s.attack(VANILLA, "hero").expect_health(AI, 22);
}

/// The greedy yardstick keeps the old reads: GREEDY_EVAL leaves the flag off. With a plain 2/1 in the
/// Banisher's place, the board reads as with the flag off.
#[test]
fn r4065001_greedy_eval_and_boards_without_an_exiler_read_as_before() {
    const { assert!(AI_EVAL.exile_on_damage_read) };
    const { assert!(!GREEDY_EVAL.exile_on_damage_read) };
    let s = game(
        side(json!([wall(3)])),
        side(json!([
            { "def": OOMEN, "lane": 1 },
            { "def": OOMEN, "lane": 2, "statsOverride": { "attack": 2, "health": 1 } },
            big(4),
        ])),
    );
    assert_eq!(face_threat(s.state(), HUMAN), 3);
    assert_eq!(face_threat_with(s.state(), HUMAN, &OFF), 3);
}
