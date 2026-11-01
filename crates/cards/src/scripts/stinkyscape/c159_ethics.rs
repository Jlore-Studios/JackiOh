//! S #159 Faction: ETHICS (SPEC §8.10 row S 159; §2.4, §5.2, §6.3): (2) Spell, Faction, Rare.
//!   Base:    "Add an iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka and Tylannt to your hand."
//!   Radiant: "Add a Radiant iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka and Tylannt to your hand."
//!
//! Engine: one `add_to_hand` per card named, S #110 iKwzki, S #111 Shockwave447, S #109 xxFreshJack, S #20
//! Tnack Trav, S #37 Kwzka, S #6 Tylannt, in the order printed (S #10 Otherworldly Endurance's pattern), each
//! a new card on its own face at its own price, a full hand burning what does not fit, in that order (§2.4).
//! Radiant: each `radiant: true` (§5.2, R1880000). The cards are named, so they reach the hand whether or
//! not the set ships (R1420). Declares no numbers.
//!
//! #2521: renamed from ETHICS, tagged Faction and (2) where it was (3). It and the six cards it adds are the
//! ETHICS faction that S #163 Swear Loyalities keeps or exiles (R2521004). Tylannt is Vanguard Coalition's
//! too.

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-159";

/// The six cards it adds, in the order printed.
const ADDS: [&str; 6] = [
    "stinkyscape-110", // iKwzki (S #110)
    "stinkyscape-111", // Shockwave447 (S #111)
    "stinkyscape-109", // xxFreshJack (S #109)
    "stinkyscape-020", // Tnack Trav (S #20)
    "stinkyscape-037", // Kwzka (S #37)
    "stinkyscape-006", // Tylannt (S #6)
];

fn ethics(radiant: bool) -> Script {
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
        base: ethics(false),
        radiant: ethics(true),
    }
}

// S #159 Faction: ETHICS — SPEC §8.10 row S 159, BUILD M12 row S 159.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const ETHICS: &str = "stinkyscape-159";
    const FILLER: &str = "core-005";
    const ADDS: [&str; 6] = [
        "stinkyscape-110",
        "stinkyscape-111",
        "stinkyscape-109",
        "stinkyscape-020",
        "stinkyscape-037",
        "stinkyscape-006",
    ];

    fn board(radiant: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": ETHICS, "radiant": radiant })];
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

            s.play(ETHICS, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| !card.radiant));
            assert!(made.iter().all(|card| card.cost_override.is_none() && card.cost_mod == 0));
            assert!(made.iter().all(|card| card.owner == P1));
        }

        #[test]
        fn r4_a_hand_with_room_for_one_keeps_the_ikwzki_and_burns_the_rest() {
            let mut s = board(false, filler(HAND_CAP as usize - 1));

            s.play(ETHICS, json!({}));

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

            s.play(ETHICS, json!({}));

            let made = added(&s);
            assert_eq!(def_ids(&made), ADDS);
            assert!(made.iter().all(|card| card.radiant));
        }
    }
}
