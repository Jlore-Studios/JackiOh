//! Meditative #44 CN Jade Well (SPEC §8.8 row 44). (3) Field Spell, CN, Rare.
//!
//!   Base:    "Start of turn: Add an Auspicious Rock to your hand."
//!   Radiant: "Start of turn: Add a Radiant Auspicious Rock to your hand."
//!
//! Engine: a Field Spell's `start_of_turn` (R62), `add_to_hand` the Rock (M #39.1). Core #6 Mana
//! Well's shape: the same cost, the same "Well", the same clock. MD-C15: a start-of-turn text needs
//! a card that stays on the field, as R402 reasoned for C #78.
//! Tunes: none.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-044";

/// The Rock this adds each turn (M #39.1).
pub const ROCK: &str = "meditative-039-1";

fn well(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![add_to_hand(json_as(json!({ "defId": ROCK, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: well(false),
        radiant: well(true),
    }
}

// Meditative #44 CN Jade Well — SPEC §8.8 row 44, BUILD M10 row M 44: "A Field Spell (MD-C15): at
// your start of turn only, adds a base Auspicious Rock to your hand, nothing at the opponent's
// start; destroyed, it stops; the hand cap burns the Rock; radiant the Rock is Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-008";
    const LIBRARY: [&str; 3] = ["core-025", "core-008", "core-020"];

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).iter().map(|card| card.def_id.clone()).collect()
    }

    #[test]
    fn r964_a_field_spell_that_stays_and_adds_a_rock_at_your_start() {
        crate::register_all();
        assert_eq!(crate::card_def(ID).type_, CardType::FieldSpell);
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "backrow": [ID], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));

        // Refresh, then the start-of-turn triggers, then the draw (§2.2).
        s.start_turn();

        // Mana Well's shape and clock: still on the field, and a Rock in hand.
        assert!(s.backrow(P1, 0).is_some_and(|card| card.def_id == ID));
        let hand = hand_defs(&s, P1);
        assert_eq!(hand.len(), 3, "the Rock, then the draw");
        assert_eq!(hand[1], ROCK);
        assert!(!s.hand(P1)[1].radiant);
    }

    #[test]
    fn nothing_at_the_opponents_start() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [ID], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));

        s.start_turn();

        assert_eq!(hand_defs(&s, P1), vec![FILLER]);
    }

    #[test]
    fn destroyed_it_stops() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": ["core-034", FILLER], "backrow": [ID], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));
        let well = s.backrow(P1, 0).expect("the Well").id.clone();

        // #34 Collateral Damage exiles the Well (its deck-milling side effect is harmless here).
        s.play("core-034", json!({ "targets": [{ "pick": "instance", "instanceId": well }] }));
        s.start_turn();

        // Exiled, it adds nothing; the draw still lands.
        let hand = hand_defs(&s, P1);
        assert_eq!(hand.len(), 2);
        assert!(!hand.contains(&ROCK.to_string()));
    }

    #[test]
    fn radiant_rock() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "backrow": [{ "def": ID, "radiant": true }], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));

        s.start_turn();

        let hand = s.hand(P1);
        let rocks: Vec<&CardInstance> =
            hand.iter().filter(|card| card.def_id == ROCK).collect();
        assert_eq!(rocks.len(), 1);
        assert!(rocks[0].radiant);
    }
}
