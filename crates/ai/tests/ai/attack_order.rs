//! #2825: the move order ranks hero attacks, winning trades and even trades together by the opponent
//! model's values (`reply.rs`): a kill by `trade_value`, a hit on the hero by `reply_face_value`.
//!
//! The cards: Mr. Vanilla 4/4; Trenful Trickster 1/4, whose end of turn summons Traps (the engine: its
//! kill, 4 × `AI_EVAL.health` + 1 × `AI_EVAL.attack` = 5.2, beats a Vanilla's 4 on the face at
//! `AI_REPLY.face_per_damage` 1.0); and Zoomerbin Oomen 1/2 (its kill, 3.2, does not).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const VANILLA: &str = "core-008";
const TRICKSTER: &str = "meditative-016";
const OOMEN: &str = "core-067";
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// p1's five Mr. Vanillas, one in each lane, facing p2's `field` at `health`.
fn crowded(seed: &str, health: i32, field: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": {
            "field": [
                { "def": VANILLA, "lane": 1 },
                { "def": VANILLA, "lane": 2 },
                { "def": VANILLA, "lane": 3 },
                { "def": VANILLA, "lane": 4 },
                { "def": VANILLA, "lane": 5 },
            ],
            "library": LIBRARY,
        },
        "p2": { "health": health, "field": field, "library": LIBRARY },
    }))
    .state()
    .clone()
}

/// The engine in lane 2 beside the Oomen in lane 1.
fn engine_and_oomen() -> Value {
    json!([{ "def": OOMEN, "lane": 1 }, { "def": TRICKSTER, "lane": 2 }])
}

/// The id of `player`'s unit of `def_id` on the field.
fn unit_id(state: &GameState, player: PlayerId, def_id: &str) -> String {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .map(|unit| unit.id.clone())
        .expect("the unit stands on the field")
}

/// The places in `actions` of the attacks on `target_id`.
fn places_of(actions: &[ActionBody], target_id: &str) -> Vec<usize> {
    let places: Vec<usize> = actions
        .iter()
        .enumerate()
        .filter(|(_, action)| matches!(action, ActionBody::Attack { target_id: at, .. } if at == target_id))
        .map(|(place, _)| place)
        .collect();
    assert!(!places.is_empty(), "an attack on {target_id} is a candidate");
    places
}

fn keys(actions: &[ActionBody]) -> Vec<String> {
    actions.iter().map(action_key).collect()
}

const ENEMY_HERO: &str = "hero-p2";

#[test]
fn the_engines_kill_goes_before_every_hit_on_the_hero_and_the_oomens_after() {
    let state = crowded("ai-attack-order-crowded", 30, engine_and_oomen());
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let engine = places_of(&actions, &unit_id(&state, HUMAN, TRICKSTER));
    let face = places_of(&actions, ENEMY_HERO);
    let oomen = places_of(&actions, &unit_id(&state, HUMAN, OOMEN));
    assert!(
        engine.iter().max() < face.iter().min(),
        "every kill of the engine before every hit on the hero: {:?}",
        keys(&actions)
    );
    assert!(
        face.iter().max() < oomen.iter().min(),
        "every hit on the hero before every kill of the Oomen: {:?}",
        keys(&actions)
    );
}

#[test]
fn the_engines_kill_is_within_branching_on_a_crowded_board() {
    // Five hits on the hero and the first winning trade, the Oomen's, filled the six before #2825.
    let state = crowded("ai-attack-order-branching", 30, engine_and_oomen());
    let expanded = expansion(
        &state,
        AI,
        AI_BUDGET.branching,
        MoveOrder::Heuristic,
        &POLICY_PRIOR,
        Prices::NONE,
    );
    let engine = unit_id(&state, HUMAN, TRICKSTER);
    assert!(
        expanded
            .iter()
            .any(|action| matches!(action, ActionBody::Attack { target_id, .. } if *target_id == engine)),
        "a kill of the engine is within {}: {:?}",
        AI_BUDGET.branching,
        keys(&expanded)
    );
}

#[test]
fn a_kill_worth_less_than_the_chip_stays_behind_every_hit_on_the_hero() {
    let state = crowded("ai-attack-order-chip", 30, json!([{ "def": OOMEN, "lane": 1 }]));
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let face = places_of(&actions, ENEMY_HERO);
    let oomen = places_of(&actions, &unit_id(&state, HUMAN, OOMEN));
    assert!(
        face.iter().max() < oomen.iter().min(),
        "every hit on the hero before every kill of the Oomen: {:?}",
        keys(&actions)
    );
}

#[test]
fn every_hit_on_the_hero_goes_first_while_the_ready_attacks_reach_the_face_together() {
    // Five Vanillas deal 20 to a hero at 12: no hit is lethal alone, but together they are.
    let state = crowded("ai-attack-order-lethal", 12, engine_and_oomen());
    assert!(ready_gap(&state, AI) <= 0.0);
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let face = places_of(&actions, ENEMY_HERO);
    let engine = places_of(&actions, &unit_id(&state, HUMAN, TRICKSTER));
    assert!(
        face.iter().max() < engine.iter().min(),
        "every hit on the hero before every kill of the engine: {:?}",
        keys(&actions)
    );
}
