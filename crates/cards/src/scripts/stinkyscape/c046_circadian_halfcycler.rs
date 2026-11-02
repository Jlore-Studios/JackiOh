//! S #46 Circadian Halfcycler (SPEC §8.10 row 46): (2) Field Spell, Epic, a card of the Stinkyscape set.
//!   Base:    "Aura: The AI plays the first 3 actions of each of your turns.
//!             End of turn: Summon a Drone Interceptor."
//!   Radiant: "Aura: The AI plays the first 3 actions of each of your turns.
//!             End of turn: Summon a Radiant Drone Interceptor."
//! Engine: the autopilot as an aura (R1844000), `pilotsActions: 3`: each of its controller's turns while
//!   it stands engages R1183002's autopilot as the turn begins, which plays its first 3 actions (R456's
//!   plays and attacks, never an answer) and then hands the turn back for good; one that arrives mid-turn
//!   waits for its controller's next turn. End of turn (R62), its controller's: a summon of the Drone
//!   Interceptor into their leftmost open unit zone (R64); with no open zone nothing. Radiant: a Radiant
//!   Interceptor.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-046";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

/// R1844000: "the first 3 actions of each of your turns", the printed count the aura plays.
const PILOTED_ACTIONS: i32 = 3;

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            pilots_actions: Some(PILOTED_ACTIONS),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))]
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

// S #46 Circadian Halfcycler — SPEC §8.10 row 46, BUILD M12 row S 46 (R1844000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HALFCYCLER: &str = "stinkyscape-046";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn fielded(radiant: bool, active: &str, hand: Value, field: Value) -> Scenario {
        scenario(json!({
            "active": active,
            "p1": {
                "hand": hand,
                "backrow": [{ "def": HALFCYCLER, "radiant": radiant, "faceUp": true }],
                "field": field,
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn engaged(events: &[GameEvent], player: PlayerId) -> bool {
        events
            .iter()
            .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: by, .. } if *by == player))
    }

    fn actions(events: &[GameEvent]) -> usize {
        events
            .iter()
            .filter(|event| {
                matches!(event, GameEvent::CardPlayed { player: P1, .. } | GameEvent::AttackDeclared { .. })
            })
            .count()
    }

    fn units(s: &Scenario, player: PlayerId) -> Vec<(String, bool)> {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    #[test]
    fn r1844000_the_autopilot_plays_your_first_3_actions_then_hands_the_turn_back() {
        let mut s = fielded(false, "p2", json!(vec![VANILLA; 4]), json!([]));
        s.end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1), "p1's turn began piloted");
        assert_eq!(actions(events), 3, "the aura's count, and no more");
        assert_eq!(s.state().active, P1, "then the turn is p1's again");
        assert_eq!(s.state().players[P1].autopilot, None, "handed back for good");
        assert!(
            legal_actions(s.state(), P1)
                .iter()
                .any(|action| action.action_type() == ActionType::Play),
            "with plays left to make"
        );
        assert!(
            !units(&s, P1).iter().any(|(def_id, _)| def_id == INTERCEPTOR),
            "no Interceptor until p1's own end of turn"
        );
    }

    #[test]
    fn r1844000_played_mid_turn_it_waits_for_your_next_turn() {
        let mut hand = vec![HALFCYCLER];
        hand.extend([VANILLA; 4]);
        let mut s = scenario(json!({
            "p1": { "hand": hand, "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.play(HALFCYCLER, json!({ "zone": 1 }));
        assert!(!engaged(s.last_events(), P1), "the turn it arrives is p1's own");
        assert_eq!(s.state().players[P1].autopilot, None);
        s.end_turn().end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1), "p1's next turn began piloted");
        assert_eq!(actions(events), 3);
    }

    #[test]
    fn r62_at_your_end_of_turn_it_summons_a_drone_interceptor() {
        let mut s = fielded(false, "p1", json!([FILLER]), json!([]));
        s.end_turn();
        assert_eq!(units(&s, P1), vec![(INTERCEPTOR.to_string(), false)]);
        assert!(units(&s, P2).is_empty());
    }

    #[test]
    fn r64_a_full_unit_row_summons_nothing() {
        let mut s = fielded(false, "p1", json!([FILLER]), json!(vec![VANILLA; 5]));
        s.end_turn();
        assert_eq!(units(&s, P1), vec![(VANILLA.to_string(), false); 5]);
    }

    #[test]
    fn radiant_a_radiant_interceptor_and_the_same_3_actions() {
        let mut s = fielded(true, "p1", json!([FILLER]), json!([]));
        s.end_turn();
        assert_eq!(units(&s, P1), vec![(INTERCEPTOR.to_string(), true)]);

        let mut s = fielded(true, "p2", json!(vec![VANILLA; 4]), json!([]));
        s.end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1));
        assert_eq!(actions(events), 3);
        assert_eq!(s.state().players[P1].autopilot, None);
    }
}
