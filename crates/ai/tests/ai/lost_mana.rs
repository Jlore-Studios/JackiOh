//! R844 (#1224): the AI prices the mana its coming turns will not bring (`evaluate.rs` holds the price
//! itself). Holding Meditative #18 Expedition12 with one spare crystal, it casts the Expedition beside two
//! Fanatics, (3) 1/1s with a Radiant 2/2 Divine Shield face, that next turn's four crystals would cast, and
//! beside Silly Silas, a (4) 4/4, Radiant 8/8: the price does not stop it casting the card. The Fanatic
//! was K-Pop Fanatic (Core #50) until issue #4292 took its Radiant Divine Shield off; since then it is a
//! stand-in with those numbers and that Divine Shield, and no Cry.
//!
//! R1012005, R4159000: both are the hand eval's decisions, held again now that no network corrects it.
//! With the Fanatics the hand eval once kept the card; casting it is the line R1012005 measured to win
//! sooner there (turn 18.0 against 20.0).

use jackioh_ai::{AI_BUDGET, AiOptions, decide};
use jackioh_engine::testkit::{ActionBody, GameState, active_units_of, create_rng, find_instance_mut, json};

use super::support::{AI, register_cards, scenario};

const EXPEDITION: &str = "meditative-018";
/// K-Pop Fanatic's Radiant face until issue #4292 (Core #50, whose Cry it leaves out), as a stand-in.
const FANATIC: &str = "stand-in (3) 1/1 R2/2 Divine Shield";
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

#[test]
fn r844_r1012005_r4159000_with_two_fanatics_and_one_spare_crystal_the_ai_casts_expedition12() {
    let state = position(&[EXPEDITION, FANATIC, FANATIC]);
    for seed in ["r844-keep-a", "r844-keep-b", "r844-keep-c"] {
        assert!(casts_expedition(&state, seed), "it kept the card on {seed}");
    }
}

#[test]
fn r844_r1012005_when_its_radiant_face_doubles_a_four_drop_the_ai_still_casts_expedition12() {
    let state = position(&[EXPEDITION, SILAS]);
    for seed in ["r844-cast-a", "r844-cast-b", "r844-cast-c"] {
        assert!(casts_expedition(&state, seed), "it kept the card on {seed}");
    }
}
