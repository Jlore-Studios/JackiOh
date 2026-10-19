//! R844 (#1224): the AI prices the mana its coming turns will not bring. Holding Meditative #18
//! Expedition12 with one spare crystal and two K-Pop Fanatics (core-050, Radiant 2/2 Divine Shield)
//! that next turn's four crystals would cast, it keeps the Expedition. With Silly Silas (core-052,
//! (4) 4/4, Radiant 8/8) it still casts it, so the price does not stop it casting the card at all.
//!
//! R1012005: both are the hand eval's decisions. With the learned evaluation on, the search takes the
//! network's correction too (R1012001), and they check nothing; `evaluate.rs` holds the price itself
//! whichever evaluation is on.

use jackioh_ai::{AI_BUDGET, AI_EVAL_KIND, AiOptions, EvalKind, decide};
use jackioh_engine::testkit::{ActionBody, GameState, active_units_of, create_rng, find_instance_mut, json};

use super::support::{AI, register_cards, scenario};

const EXPEDITION: &str = "meditative-018";
const FANATIC: &str = "core-050";
const SILAS: &str = "core-052";
const VANILLA: &str = "core-008";
const FILLER: &str = "core-005";

/// The AI's turn (the scenario's turn 9: four crystals a refresh) with one crystal left, `hand` in
/// hand (the Expedition first) and Mr. Vanilla on each side. Its Vanilla has spent both exertions,
/// so casting the Expedition and ending the turn are its only moves; next turn the Vanilla's attack
/// keeps the lost turn from ending by itself (R82), so the reply stops at the AI's next decision.
fn position(hand: &[&str]) -> GameState {
    register_cards();
    let s = scenario(json!({
        "seed": "r844-expedition12",
        "p1": { "hand": hand, "field": [VANILLA], "mana": 1, "library": [FILLER, FILLER, FILLER] },
        "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER, FILLER] },
    }));
    let mut state = s.state().clone();
    let id = active_units_of(&state, AI)[0].id.clone();
    let unit = find_instance_mut(&mut state, &id).expect("the Vanilla stands on the field");
    unit.exertion.attacked = true;
    unit.exertion.switched = true;
    state
}

/// Whether the AI's decision at `state`, on the stream `seed`, casts the Expedition.
fn casts_expedition(state: &GameState, seed: &str) -> bool {
    let expedition = state.players[AI].hand[0].id.clone();
    let mut options = AiOptions::with_budget(create_rng(seed, 0), AI_BUDGET);
    let decision = decide(state, AI, &mut options).expect("the AI decides");
    matches!(&decision.action, ActionBody::Play { instance_id, .. } if *instance_id == expedition)
}

/// R1012005: whether the hand eval decides, the evaluation these two decisions are held to.
fn the_hand_eval_decides() -> bool {
    AI_EVAL_KIND == EvalKind::Hand
}

#[test]
fn r844_r1012005_with_one_spare_crystal_the_ai_keeps_expedition12_rather_than_lose_next_turns_mana() {
    if !the_hand_eval_decides() {
        return;
    }
    let state = position(&[EXPEDITION, FANATIC, FANATIC]);
    for seed in ["r844-keep-a", "r844-keep-b", "r844-keep-c"] {
        assert!(!casts_expedition(&state, seed), "it cast the card on {seed}");
    }
}

#[test]
fn r844_r1012005_when_its_radiant_face_doubles_a_four_drop_the_ai_still_casts_expedition12() {
    if !the_hand_eval_decides() {
        return;
    }
    let state = position(&[EXPEDITION, SILAS]);
    for seed in ["r844-cast-a", "r844-cast-b", "r844-cast-c"] {
        assert!(casts_expedition(&state, seed), "it kept the card on {seed}");
    }
}
