//! #20 Pointmaster (SPEC §8.1): 7/2 → 14/4 Unit, Human, "First Strike", radiant "First Strike,
//! Divine Shield". Engine cell: "Keywords only".
//!
//! The radiant cell lists keywords without "Plus", so `[First Strike, Divine Shield]` is the radiant
//! form's complete list (§8 Conventions) — which is also what the base list plus Divine Shield comes
//! to, so the two readings agree.
//!
//! Both faces are keywords only and both keyword lists are PRINTED in the catalog
//! (`core-020.base.keywords = [First Strike]`, `core-020.radiant.keywords = [First Strike, Divine
//! Shield]`), so §10.4's layer system already supplies them: `combat.ts`'s `resolveCombat` gives the
//! First Striker step 1 of §4.3 alone and `damage.ts` spends the Divine Shield on the first hit.
//! There is nothing left for a script to do, and an empty Script is the whole of this card — a hook
//! that re-granted a printed keyword would be a second source of it (§10.4 layer 4).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-020";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script::default(),
        radiant: Script::default(),
    }
}

// #20 Pointmaster — SPEC §8.1 row 20, BUILD M4-T4 row 20.
//
// Must-pass (M4-T4): "First Strike; radiant Divine Shield". Engine cell: "Keywords only". Patch
// v0.1.1 cut its health: 7/1 → 14/2.
//
// Both keyword lists are printed in the catalog and applied by §10.4's layers, so the script is
// empty and these tests prove the PRINTED keywords actually reach combat — the only thing a
// keywords-only card can get wrong. §4.3 step 1: one First Striker hits alone and the other side
// answers in step 2 only if it survives; §4.4 step 1: Divine Shield negates the whole hit and is
// gone.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    fn keywords_of(s: &Scenario, player: PlayerId, lane: usize) -> Vec<String> {
        let view = s.view(player);
        let side = if view.you.player == player { &view.you } else { &view.opponent };
        let unit = match side.units.get(lane - 1) {
            Some(Some(unit)) => unit,
            _ => panic!("no unit in {player} unit lane {lane}"),
        };
        unit.keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    mod n20_pointmaster_base {
        use super::*;

        #[test]
        fn is_a_printed_7_1_human_with_first_strike_and_no_divine_shield() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-012"] } }));

            s.expect_stats("core-020", json!({ "attack": 7, "health": 1, "maxHealth": 1 }));
            assert_eq!(keywords_of(&s, P1, 1), vec!["First Strike".to_string()]);
        }

        #[test]
        fn s4_3_step_1_first_strike_kills_the_defender_before_it_can_strike_back() {
            crate::register_all();
            // A 3/4 would kill a 7/1 on the exchange; First Strike is the whole reason it does not.
            let mut s = scenario(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-012"] } }));

            s.attack("core-020", "core-012");

            s.expect_in_zone("core-012", "graveyard");
            s.expect_in_zone("core-020", "field");
            s.expect_stats("core-020", json!({ "health": 1 }));
        }

        #[test]
        fn s4_3_step_2_a_defender_that_survives_the_first_strike_still_answers() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-019"] } }));

            s.attack("core-020", "core-019");

            // 7 into a 9/9 leaves it standing, and its 9 back kills a 7/1: base has no Divine Shield.
            s.expect_stats("core-019", json!({ "health": 2 }));
            s.expect_in_zone("core-020", "graveyard");
        }

        #[test]
        fn s4_3_two_first_strikers_strike_simultaneously_so_neither_is_spared() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-011"] } }));

            // #11 Tempo Timmy is a 3/3 with First Strike: both are in step 1, so the exchange is mutual.
            s.attack("core-020", "core-011");

            s.expect_in_zone("core-011", "graveyard");
            s.expect_in_zone("core-020", "graveyard");
        }
    }

    mod n20_pointmaster_radiant {
        use super::*;

        #[test]
        fn is_a_printed_14_2_with_first_strike_and_divine_shield() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-020", "radiant": true }] },
                "p2": { "field": ["core-012"] },
            }));

            s.expect_stats("core-020", json!({ "attack": 14, "health": 2, "maxHealth": 2 }));
            let keywords = keywords_of(&s, P1, 1);
            for wanted in ["First Strike", "Divine Shield"] {
                assert!(keywords.contains(&wanted.to_string()), "{wanted} missing from {keywords:?}");
            }
        }

        #[test]
        fn s4_4_step_1_divine_shield_negates_the_whole_counter_attack_and_is_then_gone() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-020", "radiant": true }] },
                // A radiant #19 is an 18/18 with Taunt: it survives the 14 and hits back for 18.
                "p2": { "field": [{ "def": "core-019", "radiant": true }] },
            }));
            let pointmaster = s.card("core-020").clone();

            s.attack("core-020", "core-019");

            s.expect_stats("core-019", json!({ "health": 4, "maxHealth": 18 }));
            s.expect_in_zone(&pointmaster, "field");
            s.expect_stats(&pointmaster, json!({ "health": 2, "maxHealth": 2 }));
            // The shield is spent, so the next hit lands: a shield that never spends is R63's bug.
            assert_eq!(s.card(&pointmaster).divine_shield_spent, Some(true));
        }

        #[test]
        fn first_strike_still_lets_the_radiant_face_kill_outright_without_taking_a_hit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-020", "radiant": true }] },
                "p2": { "field": ["core-019"] },
            }));

            // 14 into a base 9/9 kills it in step 1, so the Divine Shield is never even called on.
            s.attack("core-020", "core-019");

            s.expect_in_zone("core-019", "graveyard");
            s.expect_stats("core-020", json!({ "health": 2 }));
            assert_ne!(s.card("core-020").divine_shield_spent, Some(true));
        }
    }
}
