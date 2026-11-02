//! SPEC §8.11 Training Grounds 4 Boulder Fist Ogre: (2) Unit, Common, 6/7 → 18/21, Rush on both faces,
//! a card of the held Training Grounds set (R1420, R2194000). Keywords only (§6.1, `docs/radiant-audit.md`, S #68).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-004";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Rush and the tripled stats are catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #4 Boulder Fist Ogre — SPEC §8.11 row 4, BUILD M13 row TG 4.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4

    fn played(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": super::ID, "radiant": radiant }, FILLER, FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER, FILLER], "library": [FILLER, FILLER], "field": [{ "def": BODY, "lane": 5 }] },
        }));
        s.play(super::ID, json!({}));
        s
    }

    fn has_rush(s: &Scenario) -> bool {
        s.stats(super::ID).keywords.iter().any(|keyword| matches!(keyword, Keyword::Rush))
    }

    mod base {
        use super::*;

        #[test]
        fn s6_1_a_6_7_with_rush() {
            let mut s = played(false);
            s.expect_stats(super::super::ID, json!({ "attack": 6, "health": 7 }));
            assert!(has_rush(&s));
        }

        #[test]
        fn s6_1_rush_it_attacks_an_enemy_unit_the_turn_it_is_played() {
            let mut s = played(false);
            let body = s.unit_at(PlayerId::P2, 5);
            s.attack(super::super::ID, &body);
            s.expect_in_zone(&body, "graveyard");
        }

        #[test]
        fn s6_1_rush_it_may_not_attack_the_hero_that_turn() {
            let mut s = played(false);
            s.expect_refused(|s| s.attack(super::super::ID, "hero"));
        }

        #[test]
        fn s6_1_it_attacks_the_hero_from_its_next_turn() {
            let mut s = played(false);
            s.end_turn().end_turn();
            s.attack(super::super::ID, "hero");
            s.expect_health(PlayerId::P2, HERO_HEALTH - 6);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s6_1_an_18_21_with_rush_that_hits_a_unit_at_once_and_the_hero_only_next_turn() {
            let mut s = played(true);
            s.expect_stats(super::super::ID, json!({ "attack": 18, "health": 21 }));
            assert!(has_rush(&s));
            s.expect_refused(|s| s.attack(super::super::ID, "hero"));
            let body = s.unit_at(PlayerId::P2, 5);
            s.attack(super::super::ID, &body);
            s.expect_in_zone(&body, "graveyard");
            s.end_turn().end_turn();
            s.attack(super::super::ID, "hero");
            s.expect_health(PlayerId::P2, HERO_HEALTH - 18);
        }
    }
}
