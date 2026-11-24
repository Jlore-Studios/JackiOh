//! SPEC §8.13 IA #190 Total Recall ((4) Unit, Legendary, 6/8 → 12/16), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Add copies of the last five cards your opponent played to your hand."
//!   Radiant: "Cry: Add copies of the last five cards your opponent played to your hand. They cost ({discount}) less."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::effects::{AddPlayCopiesArgs, add_play_copies};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-190";

fn total_recall(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let cost_mod = if radiant {
                Some(-param(&*ctx, "discount"))
            } else {
                None
            };
            vec![add_play_copies(AddPlayCopiesArgs {
                of: PlayerSpec::Enemy,
                last: Some(5),
                last_turn: None,
                copies: None,
                cost_mod,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: total_recall(false),
        radiant: total_recall(true),
    }
}

// IA #190 Total Recall — SPEC §8.13 row 190.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const SPELL: &str = "core-016";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "hand": [SPELL, SPELL], "mana": 10 },
            "active": "p2",
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn adds_copies_of_last_plays() {
            let mut s = board(false);
            s.play(SPELL, json!({}));
            s.play(SPELL, json!({}));
            s.end_turn();
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 6, "health": 8 }));
            let hand = s.hand(P1);
            assert_eq!(hand.iter().filter(|c| c.def_id == SPELL).count(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_12_16_and_copies_carry_discount() {
            let mut s = board(true);
            s.play(SPELL, json!({}));
            s.end_turn();
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 12, "health": 16 }));
            let copy = s.hand(P1).into_iter().find(|c| c.def_id == SPELL).unwrap();
            assert_eq!(copy.cost_mod, -1);
        }
    }
}
