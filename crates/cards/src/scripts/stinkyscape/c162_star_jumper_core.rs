//! S #162 Faction: Star-Jumper Core (SPEC §8.10 row S 162; §2.4, §5.2, §6.3): (2) Spell, Faction, Rare.
//!   Base:    "Add a Badger, Honey Badger, Ozark, Orca and Galaxy to your hand."
//!   Radiant: "Add a Radiant Badger, Honey Badger, Ozark, Orca and Galaxy to your hand."
//!
//! Engine: one `add_to_hand` per card named, S #75 Badger, S #76 Honey Badger, S #78 Ozark, S #79 Orca, S #80
//! Galaxy, in the order printed (S #10 Otherworldly Endurance's pattern), each a new card on its own face at
//! its own price, a full hand burning what does not fit, in that order (§2.4). Radiant: each `radiant: true`
//! (§5.2, R1880000). ST8: the Radiant face adds the five Radiant, where the designer's line repeated the
//! base. The cards are named, so they reach the hand whether or not the set ships (R1420). Declares no
//! numbers.
//!
//! #2521: renamed from Star-jumper Core, tagged Faction and (2) where it was (3). It and the five cards it
//! adds are the Star-Jumper Core faction that S #163 Swear Loyalities keeps or exiles (R2521004).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-162";

/// The five cards it adds, in the order printed.
const ADDS: [&str; 5] = [
    "stinkyscape-075", // Badger (S #75)
    "stinkyscape-076", // Honey Badger (S #76)
    "stinkyscape-078", // Ozark (S #78)
    "stinkyscape-079", // Orca (S #79)
    "stinkyscape-080", // Galaxy (S #80)
];

fn star_jumper_core(radiant: bool) -> Script {
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
        base: star_jumper_core(false),
        radiant: star_jumper_core(true),
    }
}

// S #162 Faction: Star-Jumper Core — SPEC §8.10 row S 162, BUILD M12 row S 162: "adds the 5 named cards in
// order, each on its base face at its printed cost; a hand with room for fewer keeps the first and burns the
// rest; radiant all of them Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const STAR_JUMPER_CORE: &str = "stinkyscape-162";
    const FILLER: &str = "core-005";
    const ADDS: [&str; 5] = [
        "stinkyscape-075",
        "stinkyscape-076",
        "stinkyscape-078",
        "stinkyscape-079",
        "stinkyscape-080",
    ];

    fn board(radiant: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": STAR_JUMPER_CORE, "radiant": radiant })];
        held.extend(hand.as_array().cloned().unwrap_or_default());
        scenario(json!({ "p1": { "hand": held }, "p2": { "hand": [FILLER] } }))
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1880000_adds_the_five_named_cards_in_order_on_their_base_faces_at_their_printed_costs() {
            let mut s = board(false, json!([FILLER]));

            s.play(STAR_JUMPER_CORE, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| !card.radiant));
            assert!(made.iter().all(|card| card.cost_override.is_none() && card.cost_mod == 0));
            assert!(made.iter().all(|card| card.owner == P1));
        }

        #[test]
        fn r4_a_hand_with_room_for_one_keeps_the_badger_and_burns_the_rest() {
            let mut s = board(false, filler(HAND_CAP as usize - 1));

            s.play(STAR_JUMPER_CORE, json!({}));

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
        fn r1880000_adds_all_five_radiant() {
            let mut s = board(true, json!([FILLER]));

            s.play(STAR_JUMPER_CORE, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| card.radiant));
        }
    }
}
