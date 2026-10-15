//! #19 Midrange Menace (SPEC §8.1): 9/9 → 18/18, "Taunt; End of turn: heal to full", radiant
//! "Taunt, Immutable; same". Engine cell: "Heal removes all damage".
//!
//! The radiant cell lists keywords without "Plus", so `[Taunt, Immutable]` is the radiant form's
//! complete keyword list, and "same" restates the end-of-turn clause (§8 Conventions): both faces carry
//! the identical hook. Neither keyword is granted here: both are PRINTED in the catalog and applied by
//! §10.4's layer system (combat enforces the Taunt; R23's Immutable scope is the effects library's), so
//! a re-grant would double it.
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

    // R781001: the Radiant face prints Immutable, so while it is in play no Spell, Field Spell or Field
    // Trap can target it, its controller's own included; a Unit's Cry can, and an effect on every Unit or
    // a random pick still reaches it.
    mod n19_midrange_menace_radiant_immutable {
        use super::*;

        const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
        const POSTDOC: &str = "core-061"; // (2) Unit; Radiant Cry: summon a Vanilla copy of any unit.
        const WHIRLWIND: &str = "classicplus-021"; // (0) Spell: Pierce, deal 1 damage to all Units.
        const SOUL_SHOT: &str = "classicplus-025"; // (2) Spell: Destroy a random enemy Unit.
        const PUNISH: &str = "classic-020"; // Field Spell: Activate, choose one, one mode destroys a Unit.
        const DOOM: &str = "destroy a Unit at the start of your next turn";
        const FILLER: &str = "core-010";

        fn at(id: &str) -> Value {
            json!([{ "pick": "instance", "instanceId": id }])
        }

        fn menace_board(p1: Value) -> Scenario {
            let mut p1 = p1;
            p1["library"] = json!([FILLER, FILLER]);
            p1["mana"] = json!(10);
            scenario(json!({
                "p1": p1,
                "p2": { "field": [{ "def": "core-019", "radiant": true }, "core-012"], "library": [FILLER, FILLER] },
            }))
        }

        #[test]
        fn r781001_a_spell_cannot_target_it_and_a_units_cry_can() {
            crate::register_all();
            let mut s = menace_board(json!({ "hand": [HIT_JOB, { "def": POSTDOC, "radiant": true }, FILLER] }));
            let menace = s.card("core-019").id.clone();

            s.expect_refused(|s| s.play(HIT_JOB, json!({ "targets": at(&menace) })));
            s.play(POSTDOC, json!({ "targets": at(&menace) }));

            let copies = (1..=5)
                .filter_map(|lane| s.unit(P1, lane))
                .filter(|unit| unit.def_id == "core-019")
                .count();
            assert_eq!(copies, 1, "the Postdoc's Cry named it and copied it");
            s.expect_in_zone(&menace, "field");
        }

        #[test]
        fn r781001_its_own_controllers_spell_cannot_target_it_either() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, FILLER], "field": [{ "def": "core-019", "radiant": true }], "library": [FILLER], "mana": 10 },
                "p2": { "library": [FILLER] },
            }));
            let menace = s.card("core-019").id.clone();
            s.expect_refused(|s| s.play(HIT_JOB, json!({ "targets": at(&menace) })));
        }

        #[test]
        fn r781001_an_effect_on_every_unit_still_reaches_it() {
            crate::register_all();
            let mut s = menace_board(json!({ "hand": [WHIRLWIND, FILLER] }));

            s.play(WHIRLWIND, json!({}));

            s.expect_stats("core-019", json!({ "health": 17, "maxHealth": 18 }));
        }

        #[test]
        fn r781001_a_random_pick_still_reaches_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SOUL_SHOT, FILLER], "library": [FILLER, FILLER], "mana": 10 },
                "p2": { "field": [{ "def": "core-019", "radiant": true }], "library": [FILLER, FILLER] },
            }));
            let menace = s.card("core-019").id.clone();

            s.play(SOUL_SHOT, json!({}));

            s.expect_in_zone(&menace, "graveyard");
        }

        #[test]
        fn r781001_a_field_spells_activate_cannot_target_it() {
            crate::register_all();
            let mut s = menace_board(json!({ "hand": [FILLER, FILLER], "backrow": [PUNISH] }));
            let menace = s.card("core-019").id.clone();
            let other = s.card("core-012").id.clone();

            s.expect_refused(|s| s.activate(PUNISH, json!({ "modes": [DOOM], "targets": at(&menace) })));
            s.activate(PUNISH, json!({ "modes": [DOOM], "targets": at(&other) }));
        }
    }
}
