//! S #95 Faction: Vanguard Coalition (SPEC §8.10 row S 95; §2.4, §5.2, §6.3): (2) Spell, Faction, Rare.
//!   Base:    "Add a Scornboi, Ocean Tsunami, Tylannt, Vxyl, Siva 1 and Siva 2 to your hand."
//!   Radiant: "Add a Radiant Scornboi, Ocean Tsunami, Tylannt, Vxyl, Siva 1 and Siva 2 to your hand."
//!
//! Engine: one `add_to_hand` per card named, S #115 Scornboi, S #96 Ocean Tsunami, S #6 Tylannt, S #103 Vxyl,
//! S #87 Siva 1, S #88 Siva 2, in the order printed (S #10 Otherworldly Endurance's pattern), each a new card
//! on its own face at its own price, a full hand burning what does not fit, in that order (§2.4). Radiant:
//! each `radiant: true` (§5.2, R1880000). ST8: the Radiant face keeps Vxyl, which the designer's line
//! dropped. The cards are named, so they reach the hand whether or not the set ships (R1420). Declares no
//! numbers.
//!
//! #2521: renamed from Vanguard Coalition, tagged Faction and (2) where it was (3). It and the six cards it
//! adds are the Vanguard Coalition faction that S #163 Swear Loyalities keeps or exiles (R2521004). Tylannt
//! is ETHICS' too.

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-095";

/// The six cards it adds, in the order printed.
const ADDS: [&str; 6] = [
    "stinkyscape-115", // Scornboi (S #115)
    "stinkyscape-096", // Ocean Tsunami (S #96)
    "stinkyscape-006", // Tylannt (S #6)
    "stinkyscape-103", // Vxyl (S #103)
    "stinkyscape-087", // Siva 1 (S #87)
    "stinkyscape-088", // Siva 2 (S #88)
];

fn vanguard_coalition(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            ADDS.iter()
                .map(|def_id| add_to_hand(json_as(json!({ "defId": def_id, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: vanguard_coalition(false),
        radiant: vanguard_coalition(true),
    }
}

// S #95 Faction: Vanguard Coalition — SPEC §8.10 row S 95, BUILD M12 row S 95.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const VANGUARD_COALITION: &str = "stinkyscape-095";
    const FILLER: &str = "core-005";
    const ADDS: [&str; 6] = [
        "stinkyscape-115",
        "stinkyscape-096",
        "stinkyscape-006",
        "stinkyscape-103",
        "stinkyscape-087",
        "stinkyscape-088",
    ];

    fn board(radiant: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": VANGUARD_COALITION, "radiant": radiant })];
        held.extend(hand.as_array().cloned().unwrap_or_default());
        scenario(json!({ "p1": { "hand": held }, "p2": { "hand": [FILLER] } }))
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1880000_adds_the_six_named_cards_in_order_on_their_base_faces_at_their_printed_costs() {
            let mut s = board(false, json!([FILLER]));

            s.play(VANGUARD_COALITION, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| !card.radiant));
            assert!(made.iter().all(|card| card.cost_override.is_none() && card.cost_mod == 0));
            assert!(made.iter().all(|card| card.owner == P1));
        }

        #[test]
        fn r4_a_hand_with_room_for_one_keeps_the_scornboi_and_burns_the_rest() {
            let mut s = board(false, filler(HAND_CAP as usize - 1));

            s.play(VANGUARD_COALITION, json!({}));

            let held = def_ids(&s.hand(P1));
            assert_eq!(held.len(), HAND_CAP as usize);
            assert_eq!(held[HAND_CAP as usize - 1], ADDS[0]);
            let burned: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(burned, ADDS[1..]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1880000_adds_all_six_radiant() {
            let mut s = board(true, json!([FILLER]));

            s.play(VANGUARD_COALITION, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| card.radiant));
        }
    }
}
