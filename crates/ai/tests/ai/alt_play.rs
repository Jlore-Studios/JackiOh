//! Face-down plays through the AI's eyes (R1046, R783009).
//!
//! A set card redacts to a placeholder with no `setAs`, and `determinize` samples for a hidden backrow
//! card a Trap, a card that prints Cloaked whatever acts (R783009), or a card an aura lends Cloaked to
//! in that side's hand now: every Unit while Knowledge Breaker acts there (R1046).

use jackioh_ai::{DeterminizeOptions, determinize, redact};
use jackioh_engine::testkit::{
    ActionBody, CardScripts, CardType, GameState, Keyword, RevealAt, Row, ZoneChoice, create_rng, find_def,
    json, legal_actions, register_catalog, register_scripts, registered_catalog, registered_scripts,
};

use super::support::{AI, HUMAN, register_cards, scenario};

const BREAKER: &str = "meditative-045";
const VANILLA: &str = "core-008";
const SHEEPISH: &str = "core-041";

/// p1 with a face-up Breaker acting and a vanilla set by the Cloaked it lends in the backrow, seen by p2.
fn set_state(seed: &str) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "p1": {
            "mana": 8,
            "hand": [VANILLA, VANILLA],
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
            equip: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: Some(RevealAt::Cloaked),
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
fn r1046_redact_drops_set_as_and_determinize_samples_units_only_under_the_breaker() {
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

    // Determinized worlds may hold a Unit while the Breaker acts, stamped `cloaked`...
    let options = DeterminizeOptions {
        match_shown_cost: Some(false),
        consistent_pools: Some(false),
    };
    let mut units = 0;
    for n in 0..50 {
        let mut rng = create_rng(&format!("alt-play-det-{n}"), 0);
        let world = determinize(&public, HUMAN, &mut rng, options);
        let sampled = world.players[AI].backrow[0].clone().expect("a sampled card");
        let def = find_def(Some(&world), &sampled.def_id).expect("a sampled def");
        if def.type_ == CardType::Unit {
            units += 1;
            assert_eq!(sampled.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        } else {
            assert!(
                def.type_ == CardType::Trap
                    || def.type_ == CardType::FieldTrap
                    || prints_cloaked(&world, &sampled.def_id),
                "a Trap or a card that prints Cloaked, got {:?}",
                def.type_
            );
        }
    }
    assert!(units > 0, "no Unit sampled in 50 worlds under the Breaker");

    // ...and without it, only a Trap or a card that prints Cloaked.
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
            def.type_ == CardType::Trap
                || def.type_ == CardType::FieldTrap
                || prints_cloaked(&world, &sampled),
            "a Unit that prints no Cloaked sampled with no Breaker: {sampled}",
        );
    }

    // The aura is live: the owner's Units are offered face-down, Cloaked.
    let offered: Vec<_> = legal_actions(&state, AI)
        .into_iter()
        .filter(|action| {
            matches!(
                action,
                ActionBody::Play {
                    face_down: Some(RevealAt::Cloaked),
                    ..
                }
            )
        })
        .collect();
    assert!(!offered.is_empty());
}

/// Whether the definition prints Cloaked on a face (Cloaked Toe Cracker does, R783009).
fn prints_cloaked(state: &GameState, def_id: &str) -> bool {
    let def = find_def(Some(state), def_id).expect("a def");
    [&def.base, &def.radiant]
        .into_iter()
        .any(|face| face.keywords.contains(&Keyword::Cloaked))
}

/// R783009: a vanilla that prints Cloaked, registered beside the real cards on this thread only.
const CLOAKED: &str = "ai-cloaked";

fn register_cloaked() {
    register_cards();
    let mut def = find_def(None, VANILLA).expect("the vanilla").clone();
    def.id = CLOAKED.to_string();
    def.index = "783".to_string();
    def.base.keywords = vec![Keyword::Cloaked];
    def.radiant.keywords = vec![Keyword::Cloaked];
    let mut defs = registered_catalog().clone();
    defs.insert(def.id.clone(), def);
    register_catalog(defs);
    let mut scripts = registered_scripts().clone();
    scripts.insert(CLOAKED.to_string(), CardScripts::default());
    register_scripts(scripts);
}

#[test]
fn r783009_determinize_samples_a_printed_cloaked_card_stamped_cloaked() {
    register_cloaked();
    let mut state = scenario(json!({
        "seed": "r783009-cloaked",
        "active": "p1",
        "p1": { "mana": 8, "hand": [CLOAKED, VANILLA] },
        "p2": { "hand": [VANILLA] },
    }))
    .state()
    .clone();
    let card = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == CLOAKED)
        .cloned()
        .expect("the Cloaked card");
    // No permission acts: Cloaked alone offers the face-down play.
    assert!(legal_actions(&state, AI).iter().any(|action| matches!(
        action,
        ActionBody::Play { instance_id, face_down: Some(RevealAt::Cloaked), .. } if *instance_id == card.id
    )));
    state = super::support::act(
        &state,
        AI,
        ActionBody::Play {
            instance_id: card.id,
            zone: Some(ZoneChoice {
                row: Row::Backrow,
                lane: 1,
            }),
            x: None,
            embiggen: None,
            equip: None,
            tributes: None,
            targets: None,
            modes: None,
            plague: None,
            face_down: Some(RevealAt::Cloaked),
        },
    );
    let set = state.players[AI].backrow[0].clone().expect("the set card");
    assert_eq!(set.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
    // Its controller's legal actions hold its Reveal.
    assert!(legal_actions(&state, AI).iter().any(|action| matches!(
        action,
        ActionBody::Activate { instance_id, ability, .. } if *instance_id == set.id && ability.as_deref() == Some("reveal")
    )));

    // The opponent's worlds may hold it, stamped `cloaked`; every other sample is a Trap, unstamped.
    let public = redact(&state, HUMAN);
    let options = DeterminizeOptions {
        match_shown_cost: Some(false),
        consistent_pools: Some(false),
    };
    let mut cloaked = 0;
    for n in 0..300 {
        let mut rng = create_rng(&format!("r783009-det-{n}"), 0);
        let world = determinize(&public, HUMAN, &mut rng, options);
        let sampled = world.players[AI].backrow[0].clone().expect("a sampled card");
        if sampled.def_id == CLOAKED {
            cloaked += 1;
            assert_eq!(sampled.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        } else {
            let def = find_def(Some(&world), &sampled.def_id).expect("a sampled def");
            if prints_cloaked(&world, &sampled.def_id) {
                // Another card that prints Cloaked (Cloaked Toe Cracker), stamped the same way.
                assert_eq!(sampled.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
            } else {
                assert!(
                    def.type_ == CardType::Trap || def.type_ == CardType::FieldTrap,
                    "a non-Trap sampled with no aura: {}",
                    sampled.def_id
                );
                assert_eq!(sampled.set_as, None);
            }
        }
    }
    assert!(cloaked > 0, "the Cloaked card was never sampled in 300 worlds");
}
