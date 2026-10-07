//! What the AI may know of patch v0.2.0's instance data (SPEC §9.9, R185; R385, R386, B5 E39): a
//! hidden card's Brittle count, its tuning and its enchantments are the card's, so `redact` drops them
//! with its face, and two true states that differ only in them redact to the same hash.
//!
//! Port of `packages/ai/test/observe-instance-data.test.ts`.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, card_by_id, clone};

fn p1_side() -> Value {
    json!({ "hand": ["core-008"], "field": ["core-011"], "library": ["core-020", "core-053"] })
}

fn p2_side() -> Value {
    json!({ "hand": ["core-002", "core-011"], "field": ["core-019"], "library": ["core-005", "core-016"] })
}

/// A value as its JSON, for comparisons that pin the wire shape rather than a Rust type's name.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

mod r185_the_ai_never_reads_a_hidden_cards_instance_data {
    use super::*;

    /// R185 a hidden card's Brittle count, tuning and enchantments go with its face
    #[test]
    fn r185_a_hidden_cards_brittle_count_tuning_and_enchantments_go_with_its_face() {
        jackioh_cards::register_all();
        let base = scenario(json!({ "seed": "observe-instance", "p1": p1_side(), "p2": p2_side() }))
            .state()
            .clone();
        let mut changed = clone(&base);
        let side = &mut changed.players[PlayerId::P2];
        for card in side.hand.iter_mut().chain(side.library.iter_mut()) {
            card.brittle = Some(BrittleCounter { count: 2, since: 1, printed: None });
            card.tuning = Some(json_as(json!({ "attack": 3, "numbers": { "damage": 1 } })));
            card.enchantments = Some(vec![json_as(json!({ "kind": "castOnDraw" }))]);
        }
        let hidden = hidden_instance_ids(&changed, AI);
        let public = redact(&changed, AI);
        for id in &hidden {
            let card = card_by_id(&public, id);
            assert!(card.as_ref().map_or(true, |card| card.tuning.is_none()), "{id}");
            assert!(card.as_ref().map_or(true, |card| card.brittle.is_none()), "{id}");
            assert!(card.as_ref().map_or(true, |card| card.enchantments.is_none()), "{id}");
        }
        assert_eq!(hash_state(&redact(&changed, AI)), hash_state(&redact(&base, AI)));
    }

    /// R185 a public card's instance data stays, since the seat reads it
    #[test]
    fn r185_a_public_cards_instance_data_stays_since_the_seat_reads_it() {
        jackioh_cards::register_all();
        let mut state = scenario(json!({ "seed": "observe-instance-public", "p1": p1_side(), "p2": p2_side() }))
            .state()
            .clone();
        let unit_id = state.players[PlayerId::P2]
            .units
            .iter()
            .flatten()
            .flatten()
            .next()
            .unwrap_or_else(|| panic!("expected a unit"))
            .id
            .clone();
        {
            let unit = find_instance_mut(&mut state, &unit_id).expect("the unit is on the field");
            unit.tuning = Some(json_as(json!({ "attack": 1 })));
            unit.brittle = Some(BrittleCounter { count: 1, since: 1, printed: None });
        }
        let public = redact(&state, AI);
        assert_eq!(
            js(card_by_id(&public, &unit_id).and_then(|card| card.tuning.clone())),
            json!({ "attack": 1 })
        );
        assert_eq!(
            js(card_by_id(&public, &unit_id).and_then(|card| card.brittle.clone())),
            json!({ "count": 1, "since": 1 })
        );
    }
}
