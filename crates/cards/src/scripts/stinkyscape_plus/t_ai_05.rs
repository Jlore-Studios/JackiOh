//! SPEC §8.12 S+ #T-AI-5 Lantern Distiller, a card of the Stinkyscape+ set (R1420, R3535000). Built by S25 (#3571).
//!   Base:    "Evolves at the start of its owner's turn.
//!            Evolve: Gain +2/+2."
//!   Radiant: "Evolves at the start of its owner's turn.
//!            Evolve: Gain +4/+4."
//! Engine: a (1) Unit 1/2 → 2/4, CN, AI, Token (no printed rarity, R482). "Evolves when" is the card's own
//!   trigger evolving itself (R3536001): a `start_of_turn` hook, which runs on its controller's turn alone
//!   ("owner's" read as Burn reads it, R3540001), and the `evolve` hook behind it runs the first time only
//!   (R3536000): `buff` +2/+2, Radiant +4/+4. Declares no numbers (R482: its numbers are the script's).

use jackioh_engine::effects::{buff, evolve};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-ai-05";

/// "Gain +2/+2".
const GAIN: i32 = 2;

/// "Gain +4/+4".
const RADIANT_GAIN: i32 = 4;

fn face(gain: i32) -> Script {
    Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
        })),
        evolve: Some(hook(move |_ctx| {
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": gain,
                "health": gain,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(GAIN),
        radiant: face(RADIANT_GAIN),
    }
}

// S+ #T-AI-5 Lantern Distiller — SPEC §8.12 row T-AI-5 (R3536000, R3536001).
#[cfg(test)]
mod tests {
    use super::{GAIN, ID, RADIANT_GAIN};
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    #[test]
    fn evolves_at_its_controllers_next_turn_start_once() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s-t-ai-05",
                json!({ "field": [{ "def": ID, "radiant": radiant }] }),
                json!({}),
            );
            let (attack, health, gain) = if radiant {
                (2, 4, RADIANT_GAIN)
            } else {
                (1, 2, GAIN)
            };
            s.expect_stats(ID, json!({ "attack": attack, "health": health }));
            // The opponent's turn start is not its owner's.
            s.end_turn();
            assert_ne!(s.card(ID).evolved, Some(true), "radiant {radiant}");
            s.expect_stats(ID, json!({ "attack": attack, "health": health }));
            // Its owner's next turn start evolves it.
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.card(ID).evolved, Some(true), "radiant {radiant}");
            s.expect_stats(ID, json!({ "attack": attack + gain, "health": health + gain }));
            // Once: the turn after changes nothing and emits no second `evolved`.
            next_turn_of(&mut s, P1);
            s.expect_stats(ID, json!({ "attack": attack + gain, "health": health + gain }));
            let evolved = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Evolved { .. }))
                .count();
            assert_eq!(evolved, 1, "radiant {radiant}");
        }
    }
}
