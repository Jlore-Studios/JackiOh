//! SPEC §8.11 Training Grounds 7 Chosen by the Moon: (1) Unit, Common, 3/3 → 6/6, Taunt and Armor 3
//! (Radiant Armor 6), a card of the held Training Grounds set (R1420, R2194000). Keywords only (§6.1):
//! Armor (R482, §4.4 step 2); Pierce skips it (R346). As M #92. TG #16 summons it.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-007";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Taunt, Armor 6 and the doubled stats are catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #7 Chosen by the Moon — SPEC §8.11 row 7, BUILD M13 row TG 7.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const FIVE: &str = "traininggrounds-001"; // Spikey Steven 5/2
    const THREE: &str = "traininggrounds-006"; // Chosen by the Sun 3/3
    const PIERCER: &str = "classicplus-076-1"; // Brother Ping 4/4 Pierce

    /// p2 holds the Moon in lane 1 (and a Mr. Vanilla in lane 2); p1 swings with `attacker`, its lane-1
    /// Unit.
    fn moon(radiant: bool, position: &str, attacker: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [attacker] },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": super::ID, "radiant": radiant, "position": position }, BODY],
            },
        }))
    }

    /// p1's attacker swings at the Moon.
    fn hit(s: &mut Scenario) {
        let attacker = s.unit_at(PlayerId::P1, 1);
        s.attack(&attacker, super::ID);
    }

    mod base {
        use super::*;

        #[test]
        fn s6_1_a_3_3_with_taunt_and_armor_3() {
            let mut s = moon(false, "ATK", json!(BODY));
            s.expect_stats(super::super::ID, json!({ "attack": 3, "health": 3 }));
            let keywords = s.stats(super::super::ID).keywords;
            assert!(keywords.iter().any(|keyword| matches!(keyword, Keyword::Taunt)));
            assert!(keywords.iter().any(|keyword| matches!(keyword, Keyword::Armor { n: 3 })));
        }

        #[test]
        fn s6_1_taunt_enemies_must_attack_it_first() {
            let mut s = moon(false, "ATK", json!(BODY));
            let body = s.unit_at(PlayerId::P2, 2);
            let attacker = s.unit_at(PlayerId::P1, 1);
            s.expect_refused(|s| s.attack(&attacker, &body));
            s.expect_refused(|s| s.attack(&attacker, "hero"));
        }

        #[test]
        fn s4_4_a_5_damage_hit_deals_2() {
            let mut s = moon(false, "ATK", json!(FIVE));
            hit(&mut s);
            s.expect_stats(super::super::ID, json!({ "health": 1, "damage": 2 }));
        }

        #[test]
        fn s4_4_a_3_damage_hit_deals_0_and_is_absorbed() {
            let mut s = moon(false, "ATK", json!(THREE));
            hit(&mut s);
            s.expect_stats(super::super::ID, json!({ "health": 3, "damage": 0 }));
            s.expect_events(json!(["damageAbsorbed"]));
        }

        #[test]
        fn s4_4_in_defense_position_its_armor_is_4() {
            let mut s = moon(false, "DEF", json!(FIVE));
            hit(&mut s);
            s.expect_stats(super::super::ID, json!({ "damage": 1 }));
        }

        #[test]
        fn r346_a_pierce_hit_ignores_the_armor() {
            let mut s = moon(false, "ATK", json!(PIERCER));
            hit(&mut s);
            s.expect_in_zone(super::super::ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s6_1_a_6_6_with_taunt_and_armor_6() {
            let mut s = moon(true, "ATK", json!(BODY));
            s.expect_stats(super::super::ID, json!({ "attack": 6, "health": 6 }));
            let keywords = s.stats(super::super::ID).keywords;
            assert!(keywords.iter().any(|keyword| matches!(keyword, Keyword::Taunt)));
            assert!(keywords.iter().any(|keyword| matches!(keyword, Keyword::Armor { n: 6 })));
        }

        #[test]
        fn s4_4_a_7_damage_hit_deals_1() {
            let mut s = moon(true, "ATK", json!({ "def": BODY, "statsOverride": { "attack": 7, "health": 7 } }));
            hit(&mut s);
            s.expect_stats(super::super::ID, json!({ "damage": 1 }));
        }
    }
}
