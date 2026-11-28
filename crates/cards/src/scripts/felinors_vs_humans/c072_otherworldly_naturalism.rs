//! FvH #72 Otherworldly Naturalism (SPEC §8.14 row 72; §2.4, R1837000, R5255000): (2) Spell,
//! Otherworldly, Epic.
//!   Base:    "Add a Wild Growth, a Swipe and a Savage Roar to your hand."
//!   Radiant: "Add a Radiant Wild Growth, a Radiant Swipe and a Radiant Savage Roar to your hand."
//!
//! Engine: three `add_to_hand`, FvH #72.1 Wild Growth, #72.2 Swipe and #72.3 Savage Roar, in that
//! order (S #12 Otherworldly Nightmare's shape), a full hand burning what does not fit (§2.4, R4).
//! Radiant: the same three, each `radiant: true`; the designer's Radiant face repeated #57's list,
//! which the owner's call reads as these (R5255000). Declares no numbers. Built by S05 (#5255).

use jackioh_engine::prelude::*;

use crate::scripts::felinors_vs_humans::{c072_1_wild_growth, c072_2_swipe, c072_3_savage_roar};

pub const ID: &str = "felinorsvshumans-072";

/// FvH #72.1 Wild Growth, #72.2 Swipe, #72.3 Savage Roar — the order they are added in, on both faces
/// (R5255000).
const TOKENS: [&str; 3] = [c072_1_wild_growth::ID, c072_2_swipe::ID, c072_3_savage_roar::ID];

fn naturalism(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            TOKENS
                .iter()
                .map(|def_id| add_to_hand(json_as(json!({ "defId": def_id, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: naturalism(false),
        radiant: naturalism(true),
    }
}

// FvH #72 Otherworldly Naturalism — SPEC §8.14 row 72 (R4, R1837000, R5255000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const WILD_GROWTH: &str = "felinorsvshumans-072-1";
    const SWIPE: &str = "felinorsvshumans-072-2";
    const SAVAGE_ROAR: &str = "felinorsvshumans-072-3";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// The cards the play added: p1's hand but the filler.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn adds_a_wild_growth_a_swipe_and_a_savage_roar_in_that_order() {
            let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));

            s.play(ID, json!({}));

            let fresh = added(&s);
            assert_eq!(def_ids(&fresh), [WILD_GROWTH, SWIPE, SAVAGE_ROAR]);
            for card in nonempty(fresh, "the added cards") {
                assert!(!card.radiant, "{} is not Radiant", card.def_id);
            }
        }

        #[test]
        fn r4_a_hand_with_room_for_one_keeps_the_wild_growth_and_burns_the_others() {
            let mut hand = vec![json!(ID)];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));

            s.play(ID, json!({}));

            assert_eq!(def_ids(&added(&s)), [WILD_GROWTH]);
            let burned: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(burned, [SWIPE, SAVAGE_ROAR]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5255000_adds_a_radiant_wild_growth_swipe_and_savage_roar_not_57s_list() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(ID, json!({}));

            let fresh = added(&s);
            assert_eq!(def_ids(&fresh), [WILD_GROWTH, SWIPE, SAVAGE_ROAR]);
            for card in nonempty(fresh, "the added cards") {
                assert!(card.radiant, "{} is Radiant", card.def_id);
            }
        }
    }
}
