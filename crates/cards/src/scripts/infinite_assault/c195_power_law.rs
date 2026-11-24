//! IA #195 Power Law (SPEC §8.13 row 195), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cry: Gain +{buff}/+{buff} for each Infinite Assault card you've played this game."
//!   Radiant: "Cry: Gain +{buff}/+{buff} for each Infinite Assault card you've played this game."
//!
//! Engine: a Cry (R1): a permanent `buff` on itself of the declared `buff` times the plays
//! `played_this_game_from_set` counts for the Infinite Assault set (R4593000). Built by C9
//! (#4593).

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-195";

fn power_law() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let count = played_this_game_from_set(ctx.state, ctx.controller, SetName::InfiniteAssault);
            let gain = param(&*ctx, "buff") * count;
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
    let radiant = power_law();
    CardScripts {
        base: power_law(),
        radiant,
    }
}

// IA #195 Power Law — SPEC §8.13 row 195 (R4593000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const SEEDLING: &str = "infiniteassault-193"; // Seedling Mind: another Infinite Assault card.
    const FILLER: &str = "core-005"; // Stockpile: a Core card.

    fn holding(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [SEEDLING, FILLER, { "def": ID, "radiant": radiant }],
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
        fn r4593000_counts_itself_and_an_earlier_infinite_assault_play() {
            let mut s = holding("power-law-base", false);
            s.play(SEEDLING, json!({ "zone": 2 }));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(stats(&s), (4, 4));
        }

        #[test]
        fn r4593000_a_core_play_adds_nothing_and_writes_no_set_count() {
            let mut s = holding("power-law-core", false);
            s.play(FILLER, json!({}));
            let log = s.state().players[P1]
                .game_log
                .clone()
                .expect("a play writes a game log");
            assert!(log.played_by_set.is_none(), "shipped plays log no set");
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(stats(&s), (3, 3), "only Power Law itself counts");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593000_counts_itself_and_an_earlier_infinite_assault_play() {
            let mut s = holding("power-law-radiant", true);
            s.play(SEEDLING, json!({ "zone": 2 }));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(stats(&s), (8, 8));
        }
    }
}
