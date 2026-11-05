//! #20 Pointmaster (SPEC §8.1): Unit, Human, 6/1 → 12/2, "Rush, First Strike", radiant "Rush, First
//! Strike, Divine Shield" (7/1 → 14/2 and no Rush until issue #3136). Engine cell: "Keywords only".
//! Printed keywords supplied by §10.4 layer system (§8 Conventions, §4.3 step 1). Empty Script.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-020";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script::default(),
        radiant: Script::default(),
    }
}

// #20 Pointmaster — SPEC §8.1 row 20, BUILD M4-T4 row 20 (must-pass: "Rush, First Strike; radiant Divine Shield").
// The script is empty, so these tests prove the PRINTED keywords reach combat (§4.3 step 1, §4.4 step 1).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    mod n20_pointmaster_base {
        use super::*;

        #[test]
        fn s4_3_step_1_first_strike_kills_the_defender_before_it_can_strike_back() {
            crate::register_all();
            // A 3/4 would kill a 6/1 on the exchange; First Strike is the whole reason it does not.
            let mut s = scenario(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-012"] } }));

            s.attack("core-020", "core-012");

            s.expect_in_zone("core-012", "graveyard");
            s.expect_in_zone("core-020", "field");
            s.expect_stats("core-020", json!({ "health": 1 }));
        }

        #[test]
        fn issue_3136_it_is_a_6_1_with_rush_so_it_attacks_a_unit_the_turn_it_enters_never_the_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-020", "core-005"], "library": ["core-005"] },
                "p2": { "field": ["core-012"], "hand": ["core-005"], "library": ["core-005"] },
            }));

            s.play("core-020", json!({}));
            s.expect_stats("core-020", json!({ "attack": 6, "health": 1, "maxHealth": 1 }));
            assert!(s.stats("core-020").keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));

            // Summoning sick, and it still hits a Unit, but not the hero (Rush, §4.1).
            s.expect_refused(|s| s.attack("core-020", "hero"));
            s.attack("core-020", "core-012");
            s.expect_in_zone("core-012", "graveyard");
            s.expect_stats("core-020", json!({ "health": 1 }));
        }
    }

    mod n20_pointmaster_radiant {
        use super::*;

        #[test]
        fn issue_3136_it_is_a_12_2_with_rush_so_it_attacks_a_unit_the_turn_it_enters_never_the_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": "core-020", "radiant": true }, "core-005"], "library": ["core-005"] },
                "p2": { "field": ["core-012"], "hand": ["core-005"], "library": ["core-005"] },
            }));

            s.play("core-020", json!({}));
            s.expect_stats("core-020", json!({ "attack": 12, "health": 2, "maxHealth": 2 }));
            assert!(s.stats("core-020").keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));

            s.expect_refused(|s| s.attack("core-020", "hero"));
            s.attack("core-020", "core-012");
            s.expect_in_zone("core-012", "graveyard");
        }

        #[test]
        fn s4_4_step_1_divine_shield_negates_the_whole_counter_attack_and_is_then_gone() {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-020", "radiant": true }] },
                // A radiant #19 is an 18/18 with Taunt: it survives the 12 and hits back for 18.
                "p2": { "field": [{ "def": "core-019", "radiant": true }] },
            }));
            let pointmaster = s.card("core-020").clone();

            s.attack("core-020", "core-019");

            s.expect_stats("core-019", json!({ "health": 6, "maxHealth": 18 }));
            s.expect_in_zone(&pointmaster, "field");
            s.expect_stats(&pointmaster, json!({ "health": 2, "maxHealth": 2 }));
            // The shield is spent, so the next hit lands: a shield that never spends is R63's bug.
            assert_eq!(s.card(&pointmaster).divine_shield_spent, Some(true));
        }

        #[test]
        fn first_strike_still_lets_the_radiant_face_kill_outright_without_taking_a_hit() {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-020", "radiant": true }] },
                "p2": { "field": ["core-019"] },
            }));

            // 12 into a base 9/9 kills it in step 1, so the Divine Shield is never even called on.
            s.attack("core-020", "core-019");

            s.expect_in_zone("core-019", "graveyard");
            s.expect_stats("core-020", json!({ "health": 2 }));
            assert_ne!(s.card("core-020").divine_shield_spent, Some(true));
        }
    }
}
