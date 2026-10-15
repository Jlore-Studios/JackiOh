//! `redact` and custom cards (R915007, SPEC §12.8): the AI knows a custom card only once a card it may
//! read names it. A custom card in the human's hand or library is hidden as any card there is, and
//! its definition and its script leave the AI's state with it; one the human has played stays.

use jackioh_ai::*;
use jackioh_engine::testkit::*;
use jackioh_engine::{CustomCard, CustomFrame, PrintedRarity};

use super::support::{AI, HUMAN};

const IN_HAND: &str = "custom-in-hand";
const ON_FIELD: &str = "custom-on-field";

fn custom(id: &str) -> CustomCard {
    CustomCard {
        id: id.to_string(),
        version: 1,
        frame: CustomFrame {
            name: format!("Custom {id}"),
            type_: CardType::Unit,
            cost: 2,
            rarity: PrintedRarity::Common,
            attack: Some(2),
            health: Some(2),
            radiant_attack: None,
            radiant_health: None,
            keywords: Vec::new(),
            text: String::new(),
            params: Vec::new(),
        },
        source: "return Card {}".to_string(),
        builder: None,
    }
}

#[test]
fn r915007_a_custom_card_the_ai_has_not_seen_leaves_its_state_and_one_it_has_stays() {
    jackioh_cards::register_all();
    let s = scenario(json!({
        "custom": [custom(IN_HAND), custom(ON_FIELD)],
        HUMAN.as_str(): { "hand": [IN_HAND, "core-008"], "field": [ON_FIELD] },
        AI.as_str(): { "hand": ["core-005"] },
    }));
    let state = s.state();
    assert_eq!(state.custom_cards.len(), 2);

    let seen = redact(state, AI);
    assert!(
        seen.custom_defs.contains_key(ON_FIELD),
        "the custom card on the field is public"
    );
    assert!(
        seen.custom_cards.contains_key(ON_FIELD),
        "and the AI may simulate its script"
    );
    assert!(
        !seen.custom_defs.contains_key(IN_HAND),
        "the one in the human's hand is not"
    );
    assert!(!seen.custom_cards.contains_key(IN_HAND));
    let text = serde_json::to_string(&seen).expect("a state serialises");
    assert!(
        !text.contains(IN_HAND),
        "the hidden card's id appears nowhere in what the AI reads"
    );

    // The human's own redaction keeps both: its hand is its own.
    let own = redact(state, HUMAN);
    assert!(own.custom_defs.contains_key(IN_HAND));
    assert!(own.custom_defs.contains_key(ON_FIELD));
}
