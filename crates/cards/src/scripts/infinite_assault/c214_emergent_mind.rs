//! IA #214 Emergent Mind (SPEC §8.13 row 214), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Start of turn: Gain +{buff}/+{buff} for each card you played last turn."
//!   Radiant: "Start of turn: Gain +{buff}/+{buff} for each card you played last turn."
//!
//! Engine: at each of its controller's ends of turn its memory `played` remembers their
//! `cards_played_this_turn` — itself included on the turn it is played — and at their next
//! start of turn a permanent `buff` of the declared `buff` times that count lands on itself
//! (R4593009). A Mind that arrived on the opponent's turn remembered nothing, so it gains
//! nothing at its first start. Built by C9 (#4593).

use jackioh_engine::effects::{buff, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-214";

/// The memory holding last turn's plays.
const PLAYED: &str = "played";

fn mind() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            vec![remember(json_as(json!({
                "key": PLAYED,
                "value": cards_played_this_turn(ctx.state, ctx.controller),
            })))]
        })),
        start_of_turn: Some(hook(|ctx| {
            let last =
                recalled(ctx, PLAYED).and_then(|stored| stored.as_i64()).unwrap_or(0) as i32;
            let gain = param(&*ctx, "buff") * last;
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
    // The same script: the Radiant face's +2/+2 is its declared `buff`.
    let radiant = mind();
    CardScripts {
        base: mind(),
        radiant,
    }
}

// IA #214 Emergent Mind — SPEC §8.13 row 214 (R4593009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [FILLER, FILLER],
                "library": crate::filler(8),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn stats(s: &Scenario) -> (i32, i32) {
        let unit = s.unit_at(P1, 1);
        let view = unit_view(s.state(), &unit);
        (view.attack, view.health)
    }

    mod base {
        use super::*;

        #[test]
        fn r4593009_gains_for_last_turns_plays_and_nothing_for_none() {
            let mut s = standing(false, "mind-base");
            s.end_turn();
            s.end_turn();
            s.end_turn();
            assert_eq!(stats(&s), (4, 4), "no plays remembered, no gain");
            s.end_turn();
            s.play(FILLER, json!({}));
            s.play(FILLER, json!({}));
            s.end_turn();
            s.end_turn();
            s.end_turn();
            assert_eq!(stats(&s), (6, 6));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593009_gains_twice_per_play() {
            let mut s = standing(true, "mind-radiant");
            s.play(FILLER, json!({}));
            s.play(FILLER, json!({}));
            s.end_turn();
            s.end_turn();
            s.end_turn();
            assert_eq!(stats(&s), (12, 12));
        }
    }
}
