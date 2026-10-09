//! Face-down plays through the AI's eyes (ME-ALTPLAY, R1046).
//!
//! A set card redacts to a placeholder with no `setAs`, and `determinize` samples Units for a
//! hidden backrow card only while a Unit permission acts on that side.

use jackioh_ai::{DeterminizeOptions, determinize, redact};
use jackioh_engine::testkit::{
    ActionBody, CardType, GameState, Row, ZoneChoice, create_rng, find_def, json, legal_actions,
};

use super::support::{AI, HUMAN, register_cards, scenario};

const BREAKER: &str = "meditative-045";
const VANILLA: &str = "core-008";
const SHEEPISH: &str = "core-041";

/// p1 with a face-up Breaker acting and a set vanilla in the backrow, seen by p2.
fn set_state(seed: &str) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "p1": {
            "mana": 8,
            "hand": [VANILLA],
            "field": [{ "def": BREAKER, "lane": 1 }],
        },
        "p2": { "hand": [VANILLA] },
    }))
    .state()
    .clone();
    let unit = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == VANILLA)
        .cloned()
        .expect("the vanilla");
    state = super::support::act(
        &state,
        AI,
        ActionBody::Play {
            instance_id: unit.id,
            zone: Some(ZoneChoice {
                row: Row::Backrow,
                lane: 1,
            }),
            x: None,
            embiggen: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: Some(jackioh_engine::RevealAt::StartOfNextTurn),
        },
    );
    state
}

fn backrow_def(state: &GameState) -> String {
    state.players[AI].backrow[0]
        .as_ref()
        .expect("a set card")
        .def_id
        .clone()
}

#[test]
fn r1046_redact_drops_set_as_and_determinize_samples_units_only_under_a_permission() {
    // The set card really is set.
    let state = set_state("alt-play-redact");
    let set = state.players[AI].backrow[0].as_ref().expect("a set card");
    assert!(set.set_as.is_some());

    // Redacted for the opponent, it is a placeholder with no face-down form.
    let public = redact(&state, HUMAN);
    let hidden = public.players[AI].backrow[0].as_ref().expect("the placeholder");
    assert_eq!(hidden.set_as, None);
    let js = serde_json::to_value(hidden).expect("a placeholder serialises");
    assert!(js.get("setAs").is_none());

    // Determinized worlds may hold a Unit while the Breaker acts...
    let options = DeterminizeOptions {
        match_shown_cost: Some(false),
    };
    let mut units = 0;
    for n in 0..50 {
        let mut rng = create_rng(&format!("alt-play-det-{n}"), 0);
        let world = determinize(&public, HUMAN, &mut rng, options);
        let def = find_def(Some(&world), &backrow_def(&world)).expect("a sampled def");
        if def.type_ == CardType::Unit {
            units += 1;
        } else {
            assert!(
                def.type_ == CardType::Trap || def.type_ == CardType::FieldTrap,
                "a Trap without a permission, got {:?}",
                def.type_
            );
        }
    }
    assert!(units > 0, "no Unit sampled in 50 worlds under a permission");

    // ...and never without one.
    register_cards();
    let plain = scenario(json!({
        "seed": "alt-play-no-grant",
        "active": "p1",
        "p1": { "backrow": [{ "def": SHEEPISH, "faceUp": false }] },
        "p2": { "hand": [VANILLA] },
    }))
    .state()
    .clone();
    let public = redact(&plain, HUMAN);
    for n in 0..50 {
        let mut rng = create_rng(&format!("alt-play-plain-{n}"), 0);
        let world = determinize(&public, HUMAN, &mut rng, options);
        let sampled = backrow_def(&world);
        let def = find_def(Some(&world), &sampled).expect("a sampled def");
        assert!(
            def.type_ == CardType::Trap || def.type_ == CardType::FieldTrap,
            "a Unit sampled with no permission: {sampled}",
        );
    }

    // The permission is live: the owner's Units are offered face-down.
    let offered: Vec<_> = legal_actions(&state, AI)
        .into_iter()
        .filter(|action| {
            matches!(
                action,
                ActionBody::Play {
                    face_down: Some(_),
                    ..
                }
            )
        })
        .collect();
    assert!(!offered.is_empty());
}
