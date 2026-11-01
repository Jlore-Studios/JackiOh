//! S #47 Circadian Fullcycler (SPEC §8.10 row 47): (3) Field Spell ((2) with Brittle 5 until #2657), Epic, a card of the Stinkyscape set.
//!   Base:    "Brittle 3. Aura: The AI plays each of your turns. End of turn: Summon a Drone Corvette."
//!   Radiant: "Brittle 3. Aura: The AI plays each of your turns. End of turn: Summon a Radiant Drone
//!             Corvette."
//! Engine: Brittle 3 is the catalog's numbered keyword, its count started as the card enters (R385). The
//!   autopilot as an aura (R1844000), `pilotsTurn`: each of its controller's turns while it stands is
//!   R1183002's autopilot's from its start, its first prompts too, within R1183003's bound; a whole turn
//!   beats any count, and one that crumbles as a turn begins leaves that turn to its player. One that
//!   arrives mid-turn waits for its controller's next turn. End of turn (R62), its controller's: a summon
//!   of the Drone Corvette into their leftmost open unit zone (R64); with no open zone nothing. Radiant: a
//!   Radiant Corvette.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-047";

/// The Drone Corvette token (§8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            pilots_turn: Some(true),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": CORVETTE, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S #47 Circadian Fullcycler — SPEC §8.10 row 47, BUILD M12 row S 47: "each of your turns while it stands
// is the autopilot's, within its bound (R1844000, R1183003); End of turn: a Drone Corvette summoned;
// Brittle 3 crumbles it (R385); radiant: a Radiant Corvette".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FULLCYCLER: &str = "stinkyscape-047";
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    /// p2 to act, p1 with the Fullcycler face-up in the backrow; p2 with cards to play, so their turn
    /// does not end by itself.
    fn fielded(radiant: bool, hand: Value) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "hand": hand,
                "backrow": [{ "def": FULLCYCLER, "radiant": radiant, "faceUp": true }],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER, FILLER], "library": filler(6) },
        }))
    }

    fn engaged(events: &[GameEvent], player: PlayerId) -> bool {
        events
            .iter()
            .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: by, .. } if *by == player))
    }

    fn plays_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { player: by, .. } if *by == player))
            .count()
    }

    /// Whether each Drone Corvette `player` summoned among `events` is Radiant.
    fn corvettes(events: &[GameEvent], s: &Scenario, player: PlayerId) -> Vec<bool> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    player: by,
                    instance_id,
                    def_id,
                    ..
                } if *by == player && def_id == CORVETTE => Some(s.card(instance_id.as_str()).radiant),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1844000_your_turn_is_the_autopilots_whole_and_its_end_summons_a_drone_corvette() {
        let mut s = fielded(false, json!([VANILLA, VANILLA]));
        s.end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1), "p1's turn began piloted");
        assert!(plays_by(events, P1) >= 2, "the autopilot played p1's hand");
        assert_eq!(s.state().active, P2, "and ended p1's turn itself");
        assert_eq!(s.state().players[P1].autopilot, None);
        assert_eq!(corvettes(events, &s, P1), vec![false]);
    }

    #[test]
    fn r1844000_played_mid_turn_it_waits_for_your_next_turn() {
        let mut s = scenario(json!({
            "p1": { "hand": [FULLCYCLER, VANILLA], "library": filler(6) },
            "p2": { "hand": [FILLER, FILLER], "library": filler(6) },
        }));
        s.play(FULLCYCLER, json!({ "zone": 1 }));
        assert!(!engaged(s.last_events(), P1), "the turn it arrives is p1's own");
        assert_eq!(s.state().active, P1);
        let id = s.backrow(P1, 1).expect("the Fullcycler in lane 1").id;
        assert_eq!(active_brittle_count(s.card(id.as_str())), Some(3), "Brittle 3 starts as it enters");
    }

    #[test]
    fn r385_crumbling_as_your_turn_begins_it_leaves_that_turn_to_you() {
        let mut s = fielded(false, json!([VANILLA, VANILLA]));
        let id = s.card(FULLCYCLER).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the Fullcycler is on the field").brittle =
            Some(BrittleCounter {
                count: 1,
                since: 1,
                printed: Some(true),
            });
        s.end_turn();
        let events = s.last_events();
        assert!(events.iter().any(|event| matches!(event, GameEvent::Crumbled { .. })));
        assert_eq!(plays_by(events, P1), 0, "no aura acts on the turn it crumbled");
        assert_eq!(s.state().active, P1);
        assert_eq!(s.state().players[P1].autopilot, None);
        s.expect_in_zone(id.as_str(), "graveyard");
    }

    #[test]
    fn radiant_a_radiant_drone_corvette() {
        let mut s = fielded(true, json!([VANILLA]));
        s.end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1));
        assert_eq!(s.state().active, P2);
        assert_eq!(corvettes(events, &s, P1), vec![true]);
    }
}
