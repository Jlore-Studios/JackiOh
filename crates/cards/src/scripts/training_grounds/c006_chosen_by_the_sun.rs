//! SPEC §8.11 Training Grounds 6 Chosen by the Sun: (1) Unit, Common, 3/3 → 9/9, Divine Shield on both
//! faces, a card of the held Training Grounds set (R1420, R2194000). Keywords only (§6.1, §4.4, `docs/radiant-audit.md`). TG #16 summons it.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-006";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Divine Shield and the tripled stats are catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #6 Chosen by the Sun — SPEC §8.11 row 6, BUILD M13 row TG 6.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4

    /// p2 holds a Chosen by the Sun in lane 1; p1 has two Mr. Vanillas to hit it with.
    fn shielded(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [BODY, BODY] },
            "p2": { "hand": [FILLER], "field": [{ "def": super::ID, "radiant": radiant }] },
        }))
    }

    fn has_shield(s: &Scenario) -> bool {
        s.stats(super::ID).keywords.iter().any(|keyword| matches!(keyword, Keyword::DivineShield))
    }

    mod base {
        use super::*;

        #[test]
        fn s6_1_a_3_3_with_divine_shield() {
            let mut s = shielded(false);
            s.expect_stats(super::super::ID, json!({ "attack": 3, "health": 3 }));
            assert!(has_shield(&s));
        }

        #[test]
        fn s4_4_the_first_hit_deals_nothing_and_removes_the_shield() {
            let mut s = shielded(false);
            let first = s.unit_at(PlayerId::P1, 1);
            s.attack(&first, super::super::ID);
            s.expect_stats(super::super::ID, json!({ "health": 3, "damage": 0 }));
            assert!(!has_shield(&s));
        }

        #[test]
        fn s4_4_the_next_hit_damages_it() {
            let mut s = shielded(false);
            let first = s.unit_at(PlayerId::P1, 1);
            let second = s.unit_at(PlayerId::P1, 2);
            s.attack(&first, super::super::ID);
            s.attack(&second, super::super::ID);
            s.expect_in_zone(super::super::ID, "graveyard");
        }

        #[test]
        fn s6_1_it_is_summoning_sick_the_turn_it_is_played() {
            let mut s = scenario(json!({
                "p1": { "hand": [super::super::ID, FILLER] },
                "p2": { "hand": [FILLER], "field": [BODY] },
            }));
            s.play(super::super::ID, json!({}));
            let body = s.unit_at(PlayerId::P2, 1);
            s.expect_refused(|s| s.attack(super::super::ID, &body));
            s.expect_refused(|s| s.attack(super::super::ID, "hero"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s4_4_a_9_9_whose_shield_eats_the_first_hit_and_the_second_lands() {
            let mut s = shielded(true);
            s.expect_stats(super::super::ID, json!({ "attack": 9, "health": 9 }));
            assert!(has_shield(&s));
            let first = s.unit_at(PlayerId::P1, 1);
            let second = s.unit_at(PlayerId::P1, 2);
            s.attack(&first, super::super::ID);
            s.expect_stats(super::super::ID, json!({ "health": 9, "damage": 0 }));
            assert!(!has_shield(&s));
            s.attack(&second, super::super::ID);
            s.expect_stats(super::super::ID, json!({ "health": 5, "damage": 4 }));
        }
    }
}
