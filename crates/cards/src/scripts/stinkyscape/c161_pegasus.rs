//! S #161 Faction: Pegasus (SPEC §8.10 row S 161; §2.4, §5.2, §6.3): (2) Spell, Faction, Rare.
//!   Base:    "Add A Very Dandy Lad, Tmack Trav, Gokugains and Arctic Citadel to your hand."
//!   Radiant: "Add a Radiant A Very Dandy Lad, Tmack Trav, Gokugains and Arctic Citadel to your hand."
//!
//! Engine: one `add_to_hand` per card named, S #113 A Very Dandy Lad, S #94 Tmack Trav, S #123 Gokugains, S
//! #114 Arctic Citadel, in the order printed (S #10 Otherworldly Endurance's pattern), each a new card on its
//! own face at its own price, a full hand burning what does not fit, in that order (§2.4). Radiant: each
//! `radiant: true` (§5.2, R1880000). The cards are named, so they reach the hand whether or not the set ships
//! (R1420). Declares no numbers.
//!
//! #2521: renamed from Pegasus, tagged Faction and (2) where it was (3). It and the four cards it adds are
//! the Pegasus faction that S #163 Swear Loyalities keeps or exiles (R2521004).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-161";

/// The four cards it adds, in the order printed.
const ADDS: [&str; 4] = [
    "stinkyscape-113", // A Very Dandy Lad (S #113)
    "stinkyscape-094", // Tmack Trav (S #94)
    "stinkyscape-123", // Gokugains (S #123)
    "stinkyscape-114", // Arctic Citadel (S #114)
];

fn pegasus(radiant: bool) -> Script {
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
        base: pegasus(false),
        radiant: pegasus(true),
    }
}

// S #161 Faction: Pegasus — SPEC §8.10 row S 161, BUILD M12 row S 161.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const PEGASUS: &str = "stinkyscape-161";
    const FILLER: &str = "core-005";
    const ADDS: [&str; 4] = [
        "stinkyscape-113",
        "stinkyscape-094",
        "stinkyscape-123",
        "stinkyscape-114",
    ];

    fn board(radiant: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": PEGASUS, "radiant": radiant })];
        held.extend(hand.as_array().cloned().unwrap_or_default());
        scenario(json!({ "p1": { "hand": held }, "p2": { "hand": [FILLER] } }))
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1880000_adds_the_four_named_cards_in_order_on_their_base_faces_at_their_printed_costs() {
            let mut s = board(false, json!([FILLER]));

            s.play(PEGASUS, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| !card.radiant));
            assert!(made.iter().all(|card| card.cost_override.is_none() && card.cost_mod == 0));
            assert!(made.iter().all(|card| card.owner == P1));
        }

        #[test]
        fn r4_a_hand_with_room_for_one_keeps_the_a_very_dandy_lad_and_burns_the_rest() {
            let mut s = board(false, filler(HAND_CAP as usize - 1));

            s.play(PEGASUS, json!({}));

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
        fn r1880000_adds_all_four_radiant() {
            let mut s = board(true, json!([FILLER]));

            s.play(PEGASUS, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| card.radiant));
        }
    }
}
