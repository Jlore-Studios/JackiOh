//! #19 Midrange Menace (SPEC §8.1): 9/9 → 18/18, "Taunt; End of turn: heal to full", radiant
//! "Taunt, Immutable; same". Engine cell: "Heal removes all damage".
//!
//! The radiant cell lists keywords without "Plus", so `[Taunt, Immutable]` is the radiant form's
//! complete keyword list, and "same" restates the end-of-turn clause (§8 Conventions): both faces carry
//! the identical hook. Neither keyword is granted here: both are PRINTED in the catalog and applied by
//! §10.4's layer system (R23's scope is the effects library's), so a re-grant would double it.
//!
//! "End of turn" is the controller's own (§6.2), never the opponent's, as in #13. "Heal to full" is
//! §6.3's Heal with `toFull`: it takes ALL damage off and raises nothing. §4 says damage stays between
//! turns, which is why the card needs this at all.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-019";

/// The one clause both faces share; a fresh Script per face so neither can be mutated into the other.
fn menace() -> Script {
    Script {
        end_of_turn: Some(hook(|_ctx| {
            vec![heal(json_as(json!({ "target": { "of": "self" }, "toFull": true })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: menace(),
        radiant: menace(),
    }
}

// #19 Midrange Menace — SPEC §8.1 row 19, BUILD M4-T4 row 19.
//
// Must-pass (M4-T4): "Taunt enforced; heals to full at own end of turn; radiant Immutable refuses
// Sheepish". §4: damage stays on a unit until a heal (§6.3) or leaving the field (R78) takes it off.
//
// R23's "refuses Sheepish": the transform half belongs to #41 Sheepish's file. This file owns the half
// that makes R23 apply: the radiant face carries Immutable, printed and visible in the view, and the
// base face does not. Both faces are asserted for the keyword list and the end-of-turn heal.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// §10.8: the keywords the view publishes for a unit, which is what §10.4's layers computed.
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

    mod n19_midrange_menace_base {
        use super::*;

        #[test]
        fn is_a_printed_9_9_with_taunt_and_without_immutable() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": ["core-019"] }, "p2": { "field": ["core-012"] } }));

            s.expect_stats("core-019", json!({ "attack": 9, "health": 9, "maxHealth": 9 }));
            assert!(keywords_of(&s, P1, 1).contains(&"Taunt".to_string()));
            assert!(!keywords_of(&s, P1, 1).contains(&"Immutable".to_string()));
        }

        #[test]
        fn s4_2_step_3_taunt_is_enforced_the_enemy_hero_cannot_be_attacked_while_it_stands() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": ["core-011"] }, "p2": { "field": ["core-019"] } }));

            s.expect_refused_with(|s| s.attack("core-011", "hero"), "Taunt");
            s.expect_health(P2, 30);

            // The refusal left the state untouched (§9.3), so the attacker still has its exertion and may
            // take the Taunt unit — which is the only legal target.
            s.attack("core-011", "core-019");
            s.expect_stats("core-019", json!({ "health": 6 }));
        }

        #[test]
        fn heals_to_full_at_its_own_end_of_turn_removing_all_damage() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-019", "damage": 5 }], "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] },
            }));
            s.expect_stats("core-019", json!({ "health": 4, "maxHealth": 9 }));

            s.end_turn();

            s.expect_stats("core-019", json!({ "health": 9, "maxHealth": 9 }));
            s.expect_events(json!(["healed"]));
        }

        #[test]
        fn s6_2_does_not_heal_at_the_opponent_s_end_of_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": "core-019", "damage": 5 }], "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] },
            }));

            s.end_turn(); // p2's end of turn: an end-of-turn hook fires for its own controller only

            s.expect_stats("core-019", json!({ "health": 4 }));
        }

        #[test]
        fn heals_nothing_when_it_is_undamaged_and_never_raises_max_health() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": ["core-019"], "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] },
            }));

            s.end_turn();

            s.expect_stats("core-019", json!({ "health": 9, "maxHealth": 9 }));
        }
    }

    mod n19_midrange_menace_radiant {
        use super::*;

        #[test]
        fn r23_is_a_printed_18_18_with_taunt_and_immutable_the_keyword_that_refuses_sheepish() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-019", "radiant": true }] },
                "p2": { "field": ["core-012"] },
            }));

            s.expect_stats("core-019", json!({ "attack": 18, "health": 18, "maxHealth": 18 }));
            let keywords = keywords_of(&s, P1, 1);
            for wanted in ["Taunt", "Immutable"] {
                assert!(keywords.contains(&wanted.to_string()), "{wanted} missing from {keywords:?}");
            }
        }

        #[test]
        fn s4_2_step_3_the_radiant_face_still_walls_the_hero_off() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": ["core-011"] },
                "p2": { "field": [{ "def": "core-019", "radiant": true }] },
            }));

            s.expect_refused_with(|s| s.attack("core-011", "hero"), "Taunt");
            s.expect_health(P2, 30);
        }

        #[test]
        fn same_the_radiant_face_heals_to_full_at_its_own_end_of_turn_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": "core-019", "radiant": true, "damage": 10 }], "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] },
            }));
            s.expect_stats("core-019", json!({ "health": 8, "maxHealth": 18 }));

            s.end_turn();

            s.expect_stats("core-019", json!({ "health": 18, "maxHealth": 18 }));
        }

        #[test]
        fn s6_2_the_radiant_face_does_not_heal_at_the_opponent_s_end_of_turn_either() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": "core-019", "radiant": true, "damage": 10 }], "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] },
            }));

            s.end_turn();

            s.expect_stats("core-019", json!({ "health": 8 }));
        }
    }
}
