//! S #121 System Schematic (SPEC §8.10 row S 121; §2.4, R81): (1) Spell, Epic.
//!   Base:    "Add your choice of a Core System, a Secure System, an Unsecure System or a Wild System
//!            to your hand."
//!   Radiant: "Add your choice of a Radiant Core System, Secure System, Unsecure System or Wild System
//!            to your hand."
//!
//! Engine: four modes, one per System (S #117 to #120), chosen with the play (R81), its player's
//! choice and not a random pick (ST-S14-10); the Cry is one `add_to_hand` of the chosen System, a
//! full hand burning it (§2.4). Radiant: the one added is Radiant. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-121";

/// Each mode's word and the System it adds.
const SYSTEMS: [(&str, &str); 4] = [
    ("Core System", "stinkyscape-117"),
    ("Secure System", "stinkyscape-118"),
    ("Unsecure System", "stinkyscape-119"),
    ("Wild System", "stinkyscape-120"),
];

fn schematic(radiant: bool) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: SYSTEMS.iter().map(|(mode, _)| mode.to_string()).collect(),
        }],
        cry: Some(hook(move |ctx| {
            let chosen = chosen_options(ctx);
            SYSTEMS
                .iter()
                .filter(|(mode, _)| chosen.first().is_some_and(|option| option == mode))
                .map(|(_, def_id)| add_to_hand(json_as(json!({ "defId": def_id, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: schematic(false),
        radiant: schematic(true),
    }
}

// S #121 System Schematic — SPEC §8.10 row S 121, BUILD M12 row S 121.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const SCHEMATIC: &str = "stinkyscape-121";
    const FILLER: &str = "core-005";

    const SYSTEMS: [(&str, &str); 4] = [
        ("Core System", "stinkyscape-117"),
        ("Secure System", "stinkyscape-118"),
        ("Unsecure System", "stinkyscape-119"),
        ("Wild System", "stinkyscape-120"),
    ];

    fn schematic_board(radiant: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": SCHEMATIC, "radiant": radiant })];
        held.extend(hand.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": held },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn systems_in_hand(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| SYSTEMS.iter().any(|(_, id)| card.def_id == *id))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn each_of_the_four_modes_adds_its_system() {
            let s = schematic_board(false, json!([FILLER]));
            let held = s.card(SCHEMATIC).id.clone();
            let offered: Vec<Vec<String>> = legal_actions(s.state(), P1)
                .iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, modes, .. } if *instance_id == held => modes.clone(),
                    _ => None,
                })
                .collect();
            let words: Vec<Vec<String>> = SYSTEMS.iter().map(|(mode, _)| vec![mode.to_string()]).collect();
            assert_eq!(offered, words);

            for (mode, id) in SYSTEMS {
                let mut s = schematic_board(false, json!([FILLER]));
                s.play(SCHEMATIC, json!({ "modes": [mode] }));
                let made = systems_in_hand(&s);
                assert_eq!(made.len(), 1, "{mode}");
                assert_eq!(made[0].def_id, id);
                assert!(!made[0].radiant);
            }
        }

        #[test]
        fn a_full_hand_burns_it() {
            let mut s = schematic_board(false, filler(HAND_CAP as usize));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize + 1);

            s.play(SCHEMATIC, json!({ "modes": ["Wild System"] }));

            assert!(systems_in_hand(&s).is_empty());
            assert!(
                s.state().players[P1]
                    .graveyard
                    .iter()
                    .any(|card| card.def_id == "stinkyscape-120")
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_chosen_one_is_radiant() {
            let mut s = schematic_board(true, json!([FILLER]));

            s.play(SCHEMATIC, json!({ "modes": ["Secure System"] }));

            let made = systems_in_hand(&s);
            assert_eq!(made.len(), 1);
            assert_eq!(made[0].def_id, "stinkyscape-118");
            assert!(made[0].radiant);
        }
    }
}
