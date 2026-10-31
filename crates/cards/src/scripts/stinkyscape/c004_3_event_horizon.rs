//! S #4.3 Event Horizon (SPEC §8.10 row S 4.3, SPEC §7; §6.2 Echo, §6.3 Extra turn, R845–R847,
//! R1857002, R1870003): (4) Spell, Terminus, Token (printed Mythic), one of Olivian Expanse's picks.
//!   Base:    "Take an extra turn after this one."
//!   Radiant: "Echo 1" plus the same: it resolves twice, and two turns are owed.
//!
//! Engine: `take_extra_turn` owes its caster one more turn, taken as this one ends, a whole turn
//! (R845, R846). No once-a-game flag: M #19.1 Temporal Rift's is that card's alone (R847), so every
//! cast owes one (R1870003). The Radiant face's Echo is the `echo` flag (C+ #36.1 Bone Storm's).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004-3";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![take_extra_turn(json_as(json!({})))])),
        ..Script::default()
    };
    let radiant = Script {
        static_flags: Some(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };
    CardScripts { base, radiant }
}

// S #4.3 Event Horizon — SPEC §8.10 row S 4.3, BUILD M12 row S 4.3 (R1870003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HORIZON: &str = "stinkyscape-004-3";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.

    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "event-horizon",
            "turn": 1,
            "p1": {
                "hand": hand,
                "field": [VANILLA],
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn next_turns(s: &mut Scenario, turns: usize) -> Vec<PlayerId> {
        (0..turns)
            .map(|_| {
                s.end_turn();
                s.state().active
            })
            .collect()
    }

    mod r1870003_event_horizon {
        use super::*;

        #[test]
        fn r1870003_caster_takes_the_next_turn_whole() {
            let mut s = board(json!([{ "def": HORIZON, "radiant": false }]));
            let library = s.state().players.p1.library.len();
            s.play(HORIZON, json!({}));
            assert_eq!(s.state().players.p1.extra_turns, Some(1));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().turn, 2, "the turn cap counts it");
            assert_eq!(s.state().players.p1.turns_started, 2);
            assert_eq!(s.state().players.p1.mana.max, 2, "max mana grows");
            assert_eq!(s.state().players.p1.library.len(), library - 1);
            let unit = s.unit(P1, 1).expect("the unit stands");
            assert!(!is_sick(s.state(), &unit));
            s.end_turn();
            assert_eq!(s.state().active, P2);
        }

        #[test]
        fn r1870003_cast_twice_in_a_game_two_extra_turns() {
            let mut s = board(json!([HORIZON, HORIZON, FILLER]));
            s.play(HORIZON, json!({}));
            assert_eq!(next_turns(&mut s, 1), vec![P1], "the first extra turn");
            // Cast again on that turn: no once-a-game flag holds it back (R847 is Temporal Rift's).
            s.state_mut().players.p1.mana.current = 10;
            s.play(HORIZON, json!({}));
            assert_eq!(s.state().players.p1.extra_turns, Some(1), "the second cast owes one");
            assert_eq!(next_turns(&mut s, 2), vec![P1, P2]);
        }

        #[test]
        fn r1870003_radiant_owes_two() {
            let mut s = board(json!([{ "def": HORIZON, "radiant": true }]));
            s.play(HORIZON, json!({}));
            assert_eq!(s.state().players.p1.extra_turns, Some(2), "Echo 1: it resolves twice");
            assert_eq!(next_turns(&mut s, 3), vec![P1, P1, P2], "two extra turns in a row");
        }
    }
}
