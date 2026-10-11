//! Meditative #41 CN Smuggler (SPEC §8.8 row 41). (2) Unit, CN, Rare, 4/5 → 8/10.
//!
//!   Base:    "Start of turn: Add an Auspicious Rock and a random CN card to your hand."
//!   Radiant: "Start of turn: Add a Radiant Auspicious Rock and a random Radiant CN card to your hand."
//!
//! Engine: `start_of_turn` (its controller's, R62): `add_to_hand` the Rock (M #39.1), then
//! `add_random_from_catalog` through `catalog.pool` (non-token CN cards of every set, R380 and
//! R382, never this card, R387). Both cards are Radiant on the Radiant face. The hand cap burns
//! extras, in that order.
//! Tunes: none (the "an" and "a" are no printed numbers, as on C+ #78).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-041";

/// The Rock this adds first (M #39.1).
pub const ROCK: &str = "meditative-039-1";

/// The CN pool both faces draw from: non-token CN cards of every set, never this card (R387 is
/// the engine's — `add_random_from_catalog` already excludes the running card).
fn cn_pool() -> Value {
    json!({ "tags": ["CN"] })
}

fn smuggler(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![
                add_to_hand(json_as(json!({ "defId": ROCK, "radiant": radiant }))),
                add_random_from_catalog(
                    json_as(json!({ "query": cn_pool(), "radiant": radiant })),
                ),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: smuggler(false),
        radiant: smuggler(true),
    }
}

// Meditative #41 CN Smuggler — SPEC §8.8 row 41, BUILD M10 row M 41: "At your start of turn only,
// adds a base Auspicious Rock, then a random non-token CN card of any set (never itself, R387),
// hidden from the opponent, the hand cap burning in that order; radiant 8/10 and both cards
// Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-008";
    const LIBRARY: [&str; 3] = ["core-025", "core-008", "core-020"];

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).iter().map(|card| card.def_id.clone()).collect()
    }

    fn is_cn_non_token(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        def.tags.contains(&Tag::Cn) && !def.token && def_id != ID
    }

    #[test]
    fn start_of_turn_adds_a_rock_then_a_non_token_cn_card_not_itself() {
        crate::register_all();
        // No Meditative card in a pool that names no set (R1420) unless the set is previewed.
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));

        // Refresh, then the start-of-turn triggers, then the draw (§2.2): the Rock, the CN
        // card, and last the drawn card.
        s.start_turn();

        let hand = hand_defs(&s, P1);
        assert_eq!(hand.len(), 4, "the Rock, one CN card, then the draw");
        assert_eq!(hand[1], ROCK, "the Rock first");
        assert!(is_cn_non_token(&hand[2]), "{} is a non-token CN card, not itself", hand[2]);
        assert!(!hand[2].starts_with("meditative-"), "no Meditative card without a preview");
        assert!(s.hand(P1).iter().all(|card| !card.radiant));
        // Hidden from the opponent.
        let theirs = serde_json::to_string(&s.view(P2)).unwrap();
        assert!(!theirs.contains(ROCK));
        assert!(!theirs.contains(&hand[2]));
    }

    #[test]
    fn previewed_the_cn_pool_reaches_meditative_cards() {
        crate::register_all();
        let _preview = preview_sets(&[SetName::Meditative]);
        let ids: Vec<String> =
            crate::query::pool(ID, &json_as(json!({ "tags": ["CN"] }))).iter().map(|def| def.id.clone()).collect();
        assert!(ids.contains(&"meditative-039".to_string()));
        assert!(ids.contains(&"meditative-043".to_string()));
        assert!(ids.contains(&"meditative-044".to_string()));
        assert!(!ids.contains(&ID.to_string()), "never itself");
        assert!(!ids.iter().any(|id| crate::card_def(id).token), "no tokens");
    }

    #[test]
    fn nothing_at_the_opponents_start() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));

        s.start_turn();

        assert_eq!(hand_defs(&s, P1), vec![FILLER]);
        assert_eq!(hand_defs(&s, P2).len(), 2, "p2 drew, and nothing else");
    }

    #[test]
    fn the_hand_cap_burns_the_cn_card_first() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                "field": [{ "def": ID, "lane": 1 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));

        s.start_turn();

        // Nine in hand: the Rock is the tenth, the CN card burns, and the draw after it burns too.
        let hand = hand_defs(&s, P1);
        assert_eq!(hand.len(), 10);
        assert_eq!(hand[9], ROCK, "the Rock lands first");
        let burned: Vec<String> =
            s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect();
        let burned_cn: Vec<&String> = burned.iter().filter(|def| is_cn_non_token(def)).collect();
        assert_eq!(burned_cn.len(), 1, "the CN card burns first: {burned:?}");
        assert_eq!(hand.iter().filter(|def| *def == ROCK).count(), 1);
    }

    #[test]
    fn radiant_both_radiant() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(4), Some(5), Some(8), Some(10)]
        );
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": true, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));

        s.start_turn();

        let hand = s.hand(P1);
        assert_eq!(hand.len(), 4, "the Rock, the CN card, then the draw");
        assert!(hand[1].radiant && hand[2].radiant, "both cards Radiant");
        assert_eq!(hand[1].def_id, ROCK);
    }
}
