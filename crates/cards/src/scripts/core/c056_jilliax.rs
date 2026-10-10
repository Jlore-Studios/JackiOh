//! #56 Jilliax (SPEC §8.3): 3/2 → 6/4, "Rush, Taunt, Lifesteal, Divine Shield" / radiant "Rush,
//! Taunt, Lifesteal, Divine Shield, Reborn". Engine cell: "Keywords only".
//!
//! Both faces are empty scripts on purpose. §10.4's keyword layer reads the printed keywords off the
//! running face of the def (`face_of` in engine/src/layers.rs), and each keyword is a rule the engine
//! owns (§6.1): Rush (`why_cannot_declare`, §4.1, §4.2 step 1), Charge, Taunt (`taunt_wall`, §4.2
//! step 3), Lifesteal (§4.4 step 8), Divine Shield (step 1), Indestructible (step 4, §4.5 step 1,
//! R46, R69). Re-stating any would be a second implementation (CLAUDE.md rule 5, BUILD M4-T4).
//!
//! §8 Conventions: a keyword cell with no "Plus" "gives the radiant form's complete keyword list", so it
//! REPLACES the base list; `tests/cross/catalog.rs` (M4-T1) pins `catalog.json`'s data.
//! R46/R69: a marked Indestructible unit goes to Attack Position and loses Taunt for the turn instead
//! of dying, and is still collected at 0 max health; Sacrifice and Exile go around it (§6.3).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-056";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script::default(),
        radiant: Script::default(),
    }
}

// #56 Jilliax (SPEC §8.3, BUILD M4-T4 row 56: "All four keywords; radiant all four plus Reborn").
// A keywords-only card, so every test asserts either the computed keyword set (§10.4's keyword
// layer, read through `view_for`) or the rule each keyword names in §6.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// A turn anchor (§2.5, R82): #10 Rapid Replenish is a 0-cost Spell, so holding one keeps at least
    /// one legal action other than ending the turn, conceding and offering a draw on a side whose units
    /// have all spent their exertion. It is never played; it only stops `reduce` from auto-ending the
    /// turn underneath an assertion. It is a Spell rather than a unit so it never joins the board a test
    /// is counting, and so a def-id reference to the attacker stays unambiguous.
    const ANCHOR: &str = "core-010";

    /// The keyword kinds §10.4 computes for the unit in `lane`, sorted so the set is order-free.
    fn keyword_kinds(g: &Scenario, player: PlayerId, lane: usize) -> Vec<String> {
        let view = g.view(player);
        let unit = view.you.units.get(lane - 1).cloned().flatten();
        let Some(unit) = unit else {
            panic!("no unit in {player} lane {lane}");
        };
        let mut kinds: Vec<String> = unit.keywords.iter().map(|keyword| keyword.kind().as_str().to_string()).collect();
        kinds.sort();
        kinds
    }

    mod jilliax_base {
        use super::*;

        #[test]
        fn s6_1_prints_rush_taunt_lifesteal_and_divine_shield_and_nothing_else() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "field": [{ "def": "core-056", "lane": 1 }] } }));

            assert_eq!(keyword_kinds(&g, PlayerId::P1, 1), ["Divine Shield", "Lifesteal", "Rush", "Taunt"]);
            g.expect_stats("core-056", json!({ "attack": 3, "maxHealth": 2, "health": 2 }));
        }

        #[test]
        fn s4_1_rush_lets_it_attack_a_unit_on_its_summon_turn_but_not_the_hero() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": ["core-056", "core-005"] },
                "p2": { "field": [{ "def": "core-008", "lane": 1, "damage": 1 }] },
            }));

            g.play("core-056", json!({ "zone": 1 }));

            // §4.2 step 2: "Rush cannot hit the hero on its summon turn" — the sickness lift is for units.
            g.expect_refused_with(|g| g.attack("core-056", "hero"), "Rush cannot hit the hero");
            g.expect_health(PlayerId::P2, 30);

            // The same sick unit may attack a unit: 3 into a Mr. Vanilla at 3 health, which kills it.
            //
            // R78 resets an instance's damage as it leaves the field, so the kill cannot be read as
            // `health: 0` on the card. R89: the `destroyed` event carries what the card was, its attack and
            // max health as the layers computed them at the moment it died.
            g.attack("core-056", "core-008");
            g.expect_in_zone("core-008", "graveyard");

            let killed = g.last_events().iter().find(|event| event.event_type().as_str() == "destroyed").cloned();
            match killed {
                Some(GameEvent::Destroyed { def_id, attack, max_health, .. }) => {
                    assert_eq!(def_id, "core-008");
                    assert_eq!(attack, 4);
                    assert_eq!(max_health, 4);
                }
                other => panic!("expected a destroyed event for core-008, got {other:?}"),
            }
        }

        #[test]
        fn s4_2_step_3_taunt_forces_the_attacker_onto_it_while_any_other_enemy_unit_stands() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "field": [{ "def": "core-025", "lane": 1 }] },
                "p2": {
                    "field": [
                        { "def": "core-008", "lane": 1 },
                        { "def": "core-056", "lane": 2 },
                    ],
                },
            }));

            g.expect_refused_with(|g| g.attack("core-025", "core-008"), "Taunt unit must be attacked first");
            g.expect_refused_with(|g| g.attack("core-025", "hero"), "Taunt unit must be attacked first");

            g.attack("core-025", "core-056");
            g.expect_events(json!(["attackDeclared"]));
        }

        #[test]
        fn s4_4_step_1_divine_shield_eats_the_whole_strike_back_and_is_then_gone() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "field": [{ "def": "core-056", "lane": 1 }] },
                "p2": { "field": [{ "def": "core-019", "lane": 1 }] },
            }));

            // Midrange Menace is 9/9 with Taunt, so it is both a legal target and lethal on the strike-back.
            g.attack("core-056", "core-019");

            g.expect_events(json!(["divineShieldLost"]));
            g.expect_in_zone("core-056", "field");
            g.expect_stats("core-056", json!({ "health": 2, "maxHealth": 2 }));
            assert_eq!(keyword_kinds(&g, PlayerId::P1, 1), ["Lifesteal", "Rush", "Taunt"]);
        }

        #[test]
        fn s4_4_step_8_lifesteal_heals_its_controller_s_hero_by_the_amount_dealt_r63() {
            crate::register_all();
            let mut g = scenario(json!({
                // R82/§2.5 TURN ANCHOR: the attack spends Jilliax's only exertion and kills the only enemy unit,
                // so without a card in hand `reduce` would auto-end the turn under the assertion (the tell is
                // `turnAutoEnded` then `damage amount: 1, sourceId: null`). #10 Rapid Replenish is a 0-cost
                // Spell, always affordable, and adds no second body: the attacker is named by def id.
                "p1": { "field": [{ "def": "core-056", "lane": 1 }], "hand": [ANCHOR], "health": 20 },
                "p2": { "field": [{ "def": "core-008", "lane": 1 }] },
            }));

            g.attack("core-056", "core-008");

            // 3 dealt to Mr. Vanilla, so 3 back to the hero — the amount DEALT, not the printed attack.
            g.expect_health(PlayerId::P1, 23);
            g.expect_events(json!(["damage", "healed"]));
        }

        #[test]
        fn s4_4_step_8_lifesteal_heals_nothing_when_the_hit_dealt_nothing() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "field": [{ "def": "core-056", "lane": 1 }], "health": 20 },
                // The Rock is Indestructible, so pipeline step 4 stops the hit and 0 is dealt (R63's zero rule).
                "p2": { "field": [{ "def": "core-066", "lane": 1 }] },
            }));

            g.attack("core-056", "core-066");
            g.expect_health(PlayerId::P1, 20);
        }
    }

    mod jilliax_radiant {
        use super::*;

        #[test]
        fn s8_patch_v0_1_1_the_radiant_face_keeps_all_four_keywords_and_adds_reborn() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "field": [{ "def": "core-056", "radiant": true, "lane": 1 }] } }));

            assert_eq!(
                keyword_kinds(&g, PlayerId::P1, 1),
                ["Divine Shield", "Lifesteal", "Reborn", "Rush", "Taunt"]
            );
            g.expect_stats("core-056", json!({ "attack": 6, "maxHealth": 4, "health": 4 }));
        }

        #[test]
        fn s4_1_rush_not_charge_a_played_radiant_jilliax_may_attack_a_unit_but_not_the_hero() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [{ "def": "core-056", "radiant": true }, "core-005"] },
                "p2": { "field": [{ "def": "core-008", "lane": 1 }] },
            }));

            g.play("core-056", json!({ "zone": 1 }));
            g.expect_refused_with(|g| g.attack("core-056", "hero"), "Rush cannot hit the hero");
            g.attack("core-056", "core-008");
            g.expect_in_zone("core-008", "graveyard");
        }

        #[test]
        fn s4_4_step_1_its_divine_shield_eats_the_first_strike_back_and_s4_5_step_4_reborn_catches_the_second_death() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "field": [{ "def": "core-056", "radiant": true, "lane": 1 }], "hand": [ANCHOR], "health": 20 },
                "p2": { "field": [{ "def": "core-019", "lane": 1 }], "hand": [ANCHOR] },
            }));

            // Midrange Menace's 9 back is negated whole by the shield; Jilliax's 6 still heals 6.
            g.attack("core-056", "core-019");
            g.expect_events(json!(["divineShieldLost"]));
            g.expect_in_zone("core-056", "field");
            g.expect_stats("core-056", json!({ "health": 4, "maxHealth": 4 }));
            g.expect_health(PlayerId::P1, 26);

            // p2's turn: the 9/9 kills the unshielded Jilliax, and Reborn brings it back at 1 health
            // without Reborn — a reset instance (R78), so its printed Divine Shield is whole again.
            g.end_turn();
            g.attack("core-019", "core-056");
            g.expect_in_zone("core-056", "field");
            g.expect_stats("core-056", json!({ "health": 1, "maxHealth": 4 }));
            assert_eq!(keyword_kinds(&g, PlayerId::P1, 1), ["Divine Shield", "Lifesteal", "Rush", "Taunt"]);
        }

        #[test]
        fn s4_2_step_3_taunt_is_still_on_the_radiant_face() {
            crate::register_all();
            // R347 leaves the radiant face's printed Taunt standing.
            let mut g = scenario(json!({
                "p1": { "field": [{ "def": "core-025", "lane": 1 }] },
                "p2": {
                    "field": [
                        { "def": "core-008", "lane": 1 },
                        { "def": "core-056", "radiant": true, "lane": 2 },
                    ],
                },
            }));

            g.expect_refused_with(|g| g.attack("core-025", "core-008"), "Taunt unit must be attacked first");
        }
    }
}
