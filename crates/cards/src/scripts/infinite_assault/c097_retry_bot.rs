//! SPEC §8.13 IA #97 Retry Bot ((1) Unit, Common, 2/1 → 4/2), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reborn"
//!   Radiant: "Reborn, Rush"
//!
//! Reborn and Rush are the catalog's: the script is empty, as C+ #6 Wrong-House Attacker's is, and
//! the Radiant face only adds Rush. Reborn's return is the engine's (§6.1); the log the Recursion
//! cards read (`recursion_log.rs`, R4589000) needs nothing of this card beyond its set.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-097";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Rush and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #97 Retry Bot — SPEC §8.13 row 97.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB],
                "field": [],
                "mana": 10,
            },
            "p2": { "hand": [], "library": crate::filler(2) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn reborn_returns_it_at_1_health() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            let bot = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&bot).keywords, KeywordKind::Reborn));
            s.play(HIT_JOB, pick::targets(pick::instance(&bot)));
            let back = s.unit_at(P1, 1);
            assert_eq!(back.def_id, ID, "Reborn returns it");
            s.expect_stats(&back, json!({ "attack": 2, "health": 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn reborn_and_rush() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            let bot = s.unit_at(P1, 1);
            let keywords = &s.stats(&bot).keywords;
            assert!(has_keyword(keywords, KeywordKind::Reborn));
            assert!(has_keyword(keywords, KeywordKind::Rush));
            s.expect_stats(&bot, json!({ "attack": 4, "health": 2 }));
        }
    }
}
