//! S #8 Conjure Rosemary Triscuit (SPEC §8.10 row S 8; §2.4, R80, R311): (2) Spell, Rare.
//!   Base:    "Add a Rosemary Triscuit to your hand. Shuffle a Wardrum into your deck."
//!   Radiant: "Add a Radiant Rosemary Triscuit to your hand. Shuffle a Radiant Wardrum into your deck."
//!
//! Engine: `add_to_hand` S #8.1 Rosemary Triscuit, then `shuffle_into` a new C+ #37 Wardrum at a
//! random position (R311), in that order (C+ #32 Otherworldly Removal's shape). One add never
//! overfills a hand the play just freed (§2.4, R4). A full deck refuses the Wardrum (R80).
//! Radiant: both `radiant: true` (R275: Refresh 4 against 2; a Radiant Wardrum).

use jackioh_engine::effects::{add_to_hand, shuffle_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-008";

/// S #8.1 Rosemary Triscuit.
const TRISCUIT: &str = "stinkyscape-008-1";
/// C+ #37 Wardrum.
const WARDRUM: &str = "classicplus-037";

fn conjure(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let mut hand = json!({ "defId": TRISCUIT });
            let mut deck = json!({ "defId": WARDRUM, "count": 1 });
            if radiant {
                hand["radiant"] = json!(true);
                deck["radiant"] = json!(true);
            }
            vec![add_to_hand(json_as(hand)), shuffle_into(json_as(deck))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: conjure(false),
        radiant: conjure(true),
    }
}

// S #8 Conjure Rosemary Triscuit — SPEC §8.10 row S 8, BUILD M12 row S 8 (R80).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CONJURE: &str = "stinkyscape-008";
    const TRISCUIT: &str = "stinkyscape-008-1";
    const WARDRUM: &str = "classicplus-037";
    const FILLER: &str = "core-005";

    fn conjuring(hand: Value) -> Scenario {
        scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }))
    }

    fn library_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        def_ids(&s.pile(player, "library"))
    }

    mod base {
        use super::*;

        #[test]
        fn adds_the_triscuit_to_your_hand_and_shuffles_one_wardrum_into_your_deck() {
            crate::register_all();
            let mut s = conjuring(json!([CONJURE, FILLER, FILLER]));

            s.play(CONJURE, json!({}));

            assert_eq!(def_ids(&s.hand(P1)), [FILLER, FILLER, TRISCUIT]);
            assert!(s.hand(P1).iter().all(|card| !card.radiant));
            let wardrums =
                library_defs(&s, P1).into_iter().filter(|def| def == WARDRUM).count();
            assert_eq!(wardrums, 1);
            assert!(
                s.pile(P1, "library")
                    .iter()
                    .filter(|card| card.def_id == WARDRUM)
                    .all(|card| !card.radiant)
            );
            s.expect_in_zone(CONJURE, "graveyard");
        }

        #[test]
        fn a_hand_full_before_the_play_still_gains_the_triscuit_one_add_never_burns() {
            crate::register_all();
            let mut hand = vec![json!(CONJURE)];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let mut s = conjuring(json!(hand));

            s.play(CONJURE, json!({}));

            assert_eq!(def_ids(&s.hand(P1)).len(), 10);
            assert_eq!(def_ids(&s.hand(P1))[9], TRISCUIT);
        }

        #[test]
        fn r4_a_full_hand_burns_the_triscuit() {
            crate::register_all();
            let mut hand = vec![json!(CONJURE)];
            hand.extend((0..HAND_CAP).map(|_| json!(FILLER)));
            let mut s = conjuring(json!(hand));

            s.play(CONJURE, json!({}));

            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert!(!def_ids(&s.hand(P1)).contains(&TRISCUIT.to_string()));
            assert!(
                def_ids(&s.pile(P1, "graveyard")).contains(&TRISCUIT.to_string()),
                "it burned"
            );
            assert_eq!(
                library_defs(&s, P1)
                    .into_iter()
                    .filter(|def| def == WARDRUM)
                    .count(),
                1
            );
        }

        #[test]
        fn r80_a_full_deck_refuses_the_wardrum_and_reports_libraryoverflow() {
            crate::register_all();
            let library = vec![json!(FILLER); 60];
            let mut s = scenario(json!({
                "p1": { "hand": [CONJURE], "library": library },
                "p2": { "hand": [FILLER] },
            }));

            s.play(CONJURE, json!({}));

            assert!(!library_defs(&s, P1).contains(&WARDRUM.to_string()));
            assert!(def_ids(&s.hand(P1)).contains(&TRISCUIT.to_string()));
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::LibraryOverflow { .. }))
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_a_radiant_triscuit_and_shuffles_a_radiant_wardrum() {
            crate::register_all();
            let mut s = conjuring(json!([{ "def": CONJURE, "radiant": true }, FILLER]));

            s.play(CONJURE, json!({}));

            let fresh: Vec<CardInstance> =
                s.hand(P1).into_iter().filter(|card| card.def_id == TRISCUIT).collect();
            assert_eq!(fresh.len(), 1);
            assert!(fresh.iter().all(|card| card.radiant));
            let wardrums: Vec<CardInstance> = s
                .pile(P1, "library")
                .into_iter()
                .filter(|card| card.def_id == WARDRUM)
                .collect();
            assert_eq!(wardrums.len(), 1);
            assert!(wardrums.iter().all(|card| card.radiant));
        }
    }
}
