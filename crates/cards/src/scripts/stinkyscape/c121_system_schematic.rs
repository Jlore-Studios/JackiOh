//! S #121 System Schematic (SPEC §8.10 row S 121; §2.4, R81, R783012, R1838000, R3039001): (1) Spell, Epic,
//! Tradable, Flashback.
//!   Base:    "Tradable, Flashback / Add your choice of a Core System, a Secure System, an Unsecure System or
//!            a Wild System to your hand."
//!   Radiant: "Tradable, Flashback / Add your choice of a Radiant Core System, Secure System, Unsecure System
//!            or Wild System to your hand."
//!
//! Tradable (R1838000) and Flashback (R783012, kept after each play, R3039001) are catalog keywords the engine
//! plays, on both faces since #3142: one Trade a turn from the hand, and from its owner's graveyard it is
//! offered as a play at its price, once for each of its four modes, each play choosing its own.
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
    use jackioh_engine::{is_tradable, trades_left};

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

    /// The graveyard Schematic's plays `legalActions` offers, each with its modes.
    fn offered_from_the_graveyard(s: &Scenario, id: &str) -> Vec<Vec<String>> {
        legal_actions(s.state(), P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, modes, .. } if instance_id == id => modes.clone(),
                _ => None,
            })
            .collect()
    }

    /// A play of the graveyard card choosing `mode`, sent to `reduce` as `legalActions` offers it.
    fn flashed_back(s: &mut Scenario, id: &str, mode: &str) {
        let action = json_as::<Action>(json!({
            "type": "play",
            "playerId": "p1",
            "instanceId": id,
            "modes": [mode],
            "nonce": format!("schematic-flashback-{id}-{mode}"),
        }));
        let result = reduce(s.state(), &action);
        assert_eq!(result.error, None, "the Flashback play was refused");
        *s.state_mut() = result.state;
    }

    /// #3142: both faces print Tradable and Flashback.
    #[test]
    fn it_prints_tradable_and_flashback_on_both_faces() {
        for radiant in [false, true] {
            let s = schematic_board(radiant, json!([FILLER]));
            let keywords = s.stats(s.card(SCHEMATIC).clone()).keywords;
            assert!(has_keyword(&keywords, KeywordKind::Tradable), "radiant {radiant}");
            assert!(has_keyword(&keywords, KeywordKind::Flashback), "radiant {radiant}");
            assert!(is_tradable(s.state(), s.card(SCHEMATIC)), "radiant {radiant}");
        }
    }

    /// #3142, R1838000: it is traded from the hand, once a turn: it goes into the library for a draw.
    #[test]
    fn r1838000_it_is_traded_from_the_hand_for_a_draw_once_a_turn() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SCHEMATIC, "radiant": radiant }, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let card = s.card(SCHEMATIC).clone();
            let (hand, deck) = (s.hand(P1).len(), s.pile(P1, "library").len());
            s.trade(&card.id);
            s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
            assert_eq!(s.hand(P1).len(), hand);
            assert_eq!(s.pile(P1, "library").len(), deck);
            assert_eq!(trades_left(s.state(), P1), 0, "the turn's one Trade is made, radiant {radiant}");
        }
    }

    /// #3142, R783012, R3039001: it is replayed from the graveyard at its price, each play choosing its mode
    /// and adding that System, and it stays offered after each play.
    #[test]
    fn r3039001_it_is_replayed_from_the_graveyard_choosing_its_mode_each_time() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SCHEMATIC, "radiant": radiant }], "mana": 10, "library": filler(4) },
                "p2": { "hand": [FILLER] },
            }));
            let id = s.card(SCHEMATIC).id.clone();
            s.play(SCHEMATIC, json!({ "modes": ["Core System"] }));
            s.expect_in_zone(&id, "graveyard");
            let words: Vec<Vec<String>> = SYSTEMS.iter().map(|(mode, _)| vec![mode.to_string()]).collect();
            assert_eq!(offered_from_the_graveyard(&s, &id), words, "the four modes, asked again");

            let mana = s.state().players[P1].mana.current;
            flashed_back(&mut s, &id, "Wild System");
            assert_eq!(s.state().players[P1].mana.current, mana - 1, "at its price");
            let made: Vec<String> = systems_in_hand(&s).into_iter().map(|card| card.def_id).collect();
            assert_eq!(made, ["stinkyscape-117", "stinkyscape-120"], "radiant {radiant}");
            assert!(systems_in_hand(&s).iter().all(|card| card.radiant == radiant));
            s.expect_in_zone(&id, "graveyard");
            assert!(!offered_from_the_graveyard(&s, &id).is_empty(), "offered again after the play");
        }
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
