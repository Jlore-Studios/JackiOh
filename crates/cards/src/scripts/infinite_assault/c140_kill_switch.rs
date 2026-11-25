//! SPEC §8.13 IA #140 Kill Switch ((3) Trap, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent plays their third card in a turn: End their turn."
//!   Radiant: "Reveals when your opponent plays their third card in a turn: End their turn. They discard {discard|random card|random cards}."
//!
//! IA #14 Overflow Error's shape on the third play (R99, R456: it fires as the third play is
//! played, before that play resolves): the turn ends as soon as what resolves now has resolved
//! (R456), so the third card resolves before the turn ends. On the Radiant face they also discard
//! the card's declared `discard` at random (§6.3 Discard). No `preview`: nothing for the other seat
//! to count down to.

use jackioh_engine::effects::{discard_random, end_turn};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-140";

/// "Their third card in a turn".
const NTH_PLAY: i32 = 3;

/// Whether the face also discards: the Radiant face alone, the card's declared `discard` at random.
fn kill_switch(radiant: bool) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("kill-switch", &[GameEventType::CardPlayed], move |ctx, _event| {
                let mut effects = vec![end_turn(json_as(json!({ "player": "enemy" })))];
                if radiant {
                    effects.push(discard_random(json_as(json!({
                        "count": param(&*ctx, "discard"),
                        "player": "enemy",
                    }))));
                }
                effects
            })
            .with_when(|ctx, event| match event {
                GameEvent::CardPlayed { player, .. } => {
                    *player != ctx.controller
                        && ctx.state.active == *player
                        && cards_played_this_turn(&*ctx.state, *player) == NTH_PLAY
                }
                _ => false,
            }),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: kill_switch(false),
        radiant: kill_switch(true),
    }
}

// IA #140 Kill Switch — SPEC §8.13 row 140.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";

    /// p2's turn, p1's Kill Switch face-down, p2 holding six textless Spells.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "kill-switch",
            "active": "p2",
            "p1": {
                "hand": ["core-005"],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 2 }],
                "library": crate::filler(2),
            },
            "p2": { "hand": [SPELL, SPELL, SPELL, SPELL, SPELL, SPELL], "library": crate::filler(2) },
        }))
    }

    fn play_spells(s: &mut Scenario, count: usize) {
        for _ in 0..count {
            s.play(SPELL, json!({}));
        }
    }

    mod base {
        use super::*;

        #[test]
        fn their_first_two_plays_leave_it_armed() {
            let mut s = board(false);
            play_spells(&mut s, 2);
            assert_eq!(s.card(ID).face_up, Some(false));
            assert_eq!(s.state().active, P2, "their turn runs on");
        }

        #[test]
        fn their_third_play_resolves_before_their_turn_ends() {
            let mut s = board(false);
            play_spells(&mut s, 3);
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(s.hand(P2).len(), 3, "six, less three plays");
            assert_eq!(s.state().active, P1, "their turn ended");
        }

        #[test]
        fn the_third_card_resolves_before_the_turn_ends() {
            let mut s = scenario(json!({
                "seed": "kill-switch-resolves",
                "active": "p2",
                "p1": {
                    "hand": ["core-005"],
                    "backrow": [{ "def": ID, "faceUp": false, "lane": 2 }],
                    "library": crate::filler(2),
                },
                "p2": {
                    "hand": [SPELL, SPELL, "core-005"],
                    "library": crate::filler(3),
                    "mana": 10,
                },
            }));
            play_spells(&mut s, 2);
            s.play("core-005", json!({}));
            assert_eq!(s.hand(P2).len(), 2, "the third card drew before the turn ended");
            assert_eq!(s.state().active, P1, "their turn ended");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn their_third_play_ends_their_turn_and_discards_a_random_card() {
            let mut s = board(true);
            play_spells(&mut s, 3);
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(s.hand(P2).len(), 2, "six, less three plays and one discard");
            assert_eq!(s.state().active, P1, "their turn ended");
        }
    }
}
