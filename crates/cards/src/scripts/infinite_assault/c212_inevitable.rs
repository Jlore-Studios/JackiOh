//! IA #212 Inevitable (SPEC §8.13 row 212), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "End of turn: Give your Units +{buff}/+{buff} for each turn this has been on the
//!            field."
//!   Radiant: "End of turn: Give your Units +{buff}/+{buff} for each turn this has been on the
//!            field."
//!
//! Engine: a Field Spell's `end_of_turn` hook (its controller's ends): its memory `turns` rises
//! by one — this end included — and a permanent `buff_all_units` of the declared `buff` times
//! that count lands on its controller's Units (R4593008; R78 resets the count when it leaves the
//! field). Built by C9 (#4593).

use jackioh_engine::effects::{buff_all_units, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-212";

/// The memory counting its controller's ends of turn while this stands.
const TURNS: &str = "turns";

fn inevitable() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            let turns =
                recalled(ctx, TURNS).and_then(|stored| stored.as_i64()).unwrap_or(0) as i32 + 1;
            let gain = param(&*ctx, "buff") * turns;
            vec![
                remember(json_as(json!({ "key": TURNS, "value": turns }))),
                buff_all_units(json_as(json!({ "attack": gain, "health": gain }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's +2/+2 is its declared `buff`.
    let radiant = inevitable();
    CardScripts {
        base: inevitable(),
        radiant,
    }
}

// IA #212 Inevitable — SPEC §8.13 row 212 (R4593008).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "field": [VANILLA],
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [FILLER],
                "library": crate::filler(6),
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
        fn r4593008_gives_1_then_2() {
            let mut s = standing(false, "inevitable-base");
            s.end_turn();
            assert_eq!(stats(&s), (5, 5));
            s.end_turn();
            assert_eq!(stats(&s), (5, 5), "the opponent's end adds nothing");
            s.end_turn();
            assert_eq!(stats(&s), (7, 7));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593008_gives_2_then_4() {
            let mut s = standing(true, "inevitable-radiant");
            s.end_turn();
            assert_eq!(stats(&s), (6, 6));
            s.end_turn();
            s.end_turn();
            assert_eq!(stats(&s), (10, 10));
        }
    }
}
