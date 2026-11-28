//! FvH #57 Otherworldly Hunting (SPEC §8.14 row 57; §2.4, R1837000): (1) Spell, Otherworldly, Epic.
//!   Base:    "Add a Kill Command, a Tackling and a Felinor Companion to your hand."
//!   Radiant: "Add a Radiant Kill Command, a Radiant Tackling and a Radiant Felinor Companion to your
//!            hand."
//!
//! Engine: three `add_to_hand`, FvH #57.1 Kill Command, #57.2 Tackling and #57.3 Felinor Companion,
//! in that order (S #10 Otherworldly Endurance's shape), a full hand burning what does not fit (§2.4,
//! R4). Radiant: each `radiant: true`, the designer's "Radiant {all}" reaching the whole list (R1867001's
//! reading). Declares no numbers. Built by S05 (#5255).

use jackioh_engine::prelude::*;

use crate::scripts::felinors_vs_humans::{c057_1_kill_command, c057_2_tackling, c057_3_felinor_companion};

pub const ID: &str = "felinorsvshumans-057";

/// FvH #57.1 Kill Command, #57.2 Tackling, #57.3 Felinor Companion — the order they are added in.
const TOKENS: [&str; 3] = [c057_1_kill_command::ID, c057_2_tackling::ID, c057_3_felinor_companion::ID];

fn hunting(radiant: bool) -> Script {
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
        base: hunting(false),
        radiant: hunting(true),
    }
}

// FvH #57 Otherworldly Hunting — SPEC §8.14 row 57 (R4, R1837000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const KILL_COMMAND: &str = "felinorsvshumans-057-1";
    const TACKLING: &str = "felinorsvshumans-057-2";
    const COMPANION: &str = "felinorsvshumans-057-3";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// The cards the play added: p1's hand but the filler.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn adds_a_kill_command_a_tackling_and_a_felinor_companion_in_that_order() {
            let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));

            s.play(ID, json!({}));

            let fresh = added(&s);
            assert_eq!(def_ids(&fresh), [KILL_COMMAND, TACKLING, COMPANION]);
            for card in nonempty(fresh, "the added cards") {
                assert!(!card.radiant, "{} is not Radiant", card.def_id);
            }
        }

        #[test]
        fn r4_a_hand_with_room_for_one_keeps_the_kill_command_and_burns_the_others() {
            let mut hand = vec![json!(ID)];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));

            s.play(ID, json!({}));

            assert_eq!(def_ids(&added(&s)), [KILL_COMMAND]);
            let burned: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(burned, [TACKLING, COMPANION]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_a_radiant_kill_command_tackling_and_felinor_companion() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(ID, json!({}));

            let fresh = added(&s);
            assert_eq!(def_ids(&fresh), [KILL_COMMAND, TACKLING, COMPANION]);
            for card in nonempty(fresh, "the added cards") {
                assert!(card.radiant, "{} is Radiant", card.def_id);
            }
        }
    }
}
