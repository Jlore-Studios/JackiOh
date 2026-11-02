//! #11 Tempo Timmy (SPEC §8.1): 3/3 → 6/6 Unit, Human, cost 1. Base "Rush, First Strike", radiant
//! "Charge, First Strike". §8's Engine cell is "Keywords only", so there is nothing to script.
//!
//! The radiant cell lists keywords without "Plus", so it is the radiant form's COMPLETE keyword list
//! (§8 Conventions, "Reading a Radiant cell"): Charge replaces Rush (§4.1). Both faces' lists are
//! printed in `catalog.json` and §10.4 layer 1 reads them off the running face, so re-granting them
//! here would double them up.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-011";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = Script::default();
    CardScripts { base, radiant }
}

// #11 Tempo Timmy (SPEC §8.1, BUILD M4-T4 row 11).
//
// The card scripts nothing, so every clause is a keyword: §4.1's Rush and Charge, §4.3's First Strike.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// Makes the hand copy radiant, then plays it.
    fn play_radiant<'a>(s: &'a mut Scenario, card: &str) -> &'a mut Scenario {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
        s.play(card, json!({}))
    }

    mod n11_tempo_timmy {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn attacks_a_unit_on_its_summon_turn_rush_s4_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));
                let vanilla = s.card("core-008").clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn kills_a_3_health_unit_unharmed_first_strike_s4_3_step_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    // Mr. Vanilla, a 4/4 at 3 health with no First Strike, falls to Timmy's 3 first and
                    // deals nothing back (§4.3 step 1).
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));
                let vanilla = s.card("core-008").clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_stats("core-011", json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
            }

            #[test]
            fn cannot_hit_the_hero_on_its_summon_turn_rush_not_charge_s4_2_step_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));

                s.expect_refused_with(
                    |s| s.attack("core-011", "hero"),
                    "Rush cannot hit the hero on its summon turn",
                );
                s.expect_health(PlayerId::P2, 30);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn may_hit_the_hero_on_its_summon_turn_charge_s4_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                play_radiant(&mut s, "core-011");

                s.expect_stats("core-011", json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
                s.attack("core-011", "hero");

                s.expect_health(PlayerId::P2, 24);
            }

            #[test]
            fn still_kills_a_3_health_unit_unharmed_first_strike_is_kept_s8_conventions() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                play_radiant(&mut s, "core-011");
                let vanilla = s.card("core-008").clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_stats("core-011", json!({ "health": 6 }));
            }
        }
    }
}
