//! SPEC §8.13 IA #121 Packet Sniffer ((1) Unit, Common, 2/2 → 4/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Look at your opponent's hand."
//!   Radiant: "Cry: Look at your opponent's hand. Then Nerf a random card in it."
//!
//! The Cry opens the opponent's hand as a prompt with nothing to pick — the look itself (R4590006):
//! an empty hand asks nothing. The Radiant face Nerfs one random card of it after the answer (§6.3).

use jackioh_engine::effects::{choose_from_hand, degrade};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-121";

/// The resume step answering the look.
const LOOK: &str = "look";

fn sniffer(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![choose_from_hand(json_as(json!({
                "of": "enemy",
                "min": 0,
                "max": 0,
                "step": LOOK,
                "prompt": "Look at your opponent's hand",
            })))]
        })),
        resume: IndexMap::from([(
            LOOK,
            hook(move |_ctx| {
                if radiant {
                    vec![degrade(json_as(json!({
                        "scope": { "side": "enemy", "zones": ["hand"] },
                        "random": 1,
                    })))]
                } else {
                    vec![]
                }
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: sniffer(false),
        radiant: sniffer(true),
    }
}

// IA #121 Packet Sniffer — SPEC §8.13 row 121 (R4590006).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool, hand: Value) -> Scenario {
        scenario(json!({
            "seed": "packet-sniffer",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(3) },
            "p2": { "hand": hand, "library": crate::filler(3) },
        }))
    }

    /// The instance ids the look offers, in hand order.
    fn offered(s: &Scenario) -> Vec<String> {
        s.state().pending.clone().expect("the look's prompt").options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn degraded(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Degraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4590006_a_look_that_picks_nothing_is_a_hand_prompt_with_no_pick() {
            let mut s = board(false, json!([VANILLA, FILLER]));
            s.play(ID, json!({ "zone": 1 }));
            let pending = s.state().pending.clone().expect("the look opens");
            assert_eq!(pending.kind, PromptKind::Hand);
            assert_eq!(pending.min, 0);
            assert_eq!(pending.max, 0);
            assert_eq!(offered(&s).len(), 2, "the opponent's hand, to look at");
            s.answer(json!([]));
            assert!(s.state().pending.is_none());
            assert!(degraded(&s).is_empty(), "the base face Nerfs nothing");
        }

        #[test]
        fn r4590006_an_empty_hand_asks_nothing() {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590006_the_radiant_face_nerfs_one_random_card_after_the_answer() {
            let mut s = board(true, json!([VANILLA, FILLER]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_some(), "the look still opens");
            s.answer(json!([]));
            assert!(s.state().pending.is_none());
            assert_eq!(degraded(&s).len(), 1, "one random card Nerfed");
        }

        #[test]
        fn r4590006_an_empty_hand_asks_nothing_and_nerfs_nothing() {
            let mut s = board(true, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none());
            assert!(degraded(&s).is_empty());
        }
    }
}
