//! SPEC §8.13 IA #171 Mimic Unit ((2) Unit, Common, 3/3 → 6/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Gain the keywords of a random enemy Unit."
//!   Radiant: "Cry: Gain the keywords of every enemy Unit."
//!
//! Built by C8 (#4592). R4592000, R4592006.

use jackioh_engine::effects::{GainKeywordsOfArgs, gain_keywords_of};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-171";

fn mimic(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![gain_keywords_of(GainKeywordsOfArgs {
                every: if radiant { Some(true) } else { None },
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mimic(false),
        radiant: mimic(true),
    }
}

// IA #171 Mimic Unit — SPEC §8.13 row 171.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const TAUNT_UNIT: &str = "core-019"; // Midrange Menace: Taunt

    fn board(radiant: bool, enemy_field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": [FILLER], "field": enemy_field },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn gains_keywords_of_random_enemy_unit() {
            let mut s = board(false, json!([{ "def": TAUNT_UNIT, "lane": 1 }]));
            s.play(ID, json!({ "zone": 1 }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 3, "health": 3 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Taunt));
        }

        #[test]
        fn with_no_enemy_units_cry_does_nothing() {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 3, "health": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_6_6_and_gains_keywords_of_every_enemy_unit() {
            let mut s = board(true, json!([{ "def": TAUNT_UNIT, "lane": 1 }]));
            s.play(ID, json!({ "zone": 1 }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 6 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Taunt));
        }
    }
}
