//! C #41 State of the Game (SPEC §8.6 row 41). Unit 3/3 → 6/6, cost 1, Common.
//!   Base:    "Indestructible"
//!   Radiant: "Indestructible, Lifesteal" — the adopted Radiant face: a keyword-only Unit's Radiant
//!            owes one more keyword or a stronger one (R275), and Taunt is out (R347).
//!   Engine:  "Keywords only; under R347 it never has Taunt."
//!
//! There is nothing to script. The stats and both faces' keywords are printed on the catalog entry
//! (`base.keywords = [Indestructible]`, `radiant.keywords = [Indestructible, Lifesteal]`), and §10.4
//! layer 1 reads them from there; granting them here would only say the same thing twice. Where the
//! behaviour lives instead:
//!   Indestructible — §4.4 step 4 (it takes no damage), §4.5 step 1 and R46 (a destroy mark is
//!                    ignored and knocks it into Attack Position), R69 (it still dies if its max
//!                    health falls to 0), §6.1 (exile, bounce and a Tribute still remove it), and
//!                    R347 (§10.4's keyword set drops Taunt whenever it holds Indestructible, so it has
//!                    no Taunt even in Defense Position).
//!   Lifesteal      — §4.4 step 8 heals its controller's hero the amount it actually deals.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-041";

pub fn script() -> CardScripts {
    let base = Script::default();

    // The same empty script: the Radiant face differs only in what the engine reads off the catalog
    // (its doubled stats and the added Lifesteal).
    CardScripts { radiant: base.clone(), base }
}

// C #41 State of the Game — SPEC §8.6 row 41, BUILD M9 Classic row C 41: "3/3 Indestructible: damage
// and destroy effects don't remove it (a destroy knocks it into Attack Position, R46), while exile,
// bounce and a Tribute do; never has Taunt, even in Defense Position (R347); radiant 6/6
// Indestructible, Lifesteal (the adopted Radiant face): its damage heals your hero".
//
// The Engine cell is "Keywords only", so both scripts are empty and these fixtures prove that the
// keywords printed on the catalog faces do the work through §4.1, §4.2, §4.4 and §4.5. The removals
// need a source, each a Core card with its own tests: Hit Job (core-016, "Destroy target Unit"),
// Collateral Damage (core-034, exile a target permanent), Flood (core-017, bounce all Units) and
// Carnivorous Cube (core-022, whose Cry Tributes one of your other Units, R428).
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const STATE: &str = "classic-041";
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const COLLATERAL: &str = "core-034"; // (4) Spell: Exile target permanent and a random card from their deck.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const CUBE: &str = "core-022"; // (3) Unit: Cry: Tribute one of your other Units and remember it.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no text.
    const FILLER: &str = "core-005"; // (1) Spell, a card to keep a hand from auto-ending the turn (§2.5).
    const FLAME: &str = "classic-016"; // (1) Spell, Book: Deal 4 damage.

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    fn keyword_kinds(s: &Scenario, player: PlayerId, lane: i32) -> Vec<String> {
        let unit = s.unit(player, lane).unwrap_or_else(|| panic!("no unit in {} lane {}", js(&player), lane));
        s.stats(&unit.id).keywords.iter().map(|keyword| js(keyword)["kind"].as_str().unwrap_or("").to_string()).collect()
    }

    /// TS `expect(script).toEqual({})`: every field of the TS `Script` type is absent (the Rust
    /// `Script` minus `activate`, which SURFACE §7.2 does not port).
    fn is_empty_script(script: &Script) -> bool {
        script.cost.is_none()
            && script.cry.is_none()
            && script.death.is_none()
            && script.start_of_game.is_none()
            && script.resume.is_empty()
            && script.delayed.is_none()
            && script.set_stat.is_none()
            && script.start_of_turn.is_none()
            && script.end_of_turn.is_none()
            && script.aura.is_none()
            && script.triggers.is_empty()
            && script.on_play_hook.is_none()
            && script.hand_triggers.is_empty()
            && script.static_flags.is_none()
            && script.targets.is_empty()
            && script.modes.is_empty()
            && script.condition_met.is_none()
            && script.preview.is_none()
            && script.activations.is_empty()
            && script.target_checks.is_empty()
            && script.cost_aura.is_none()
            && script.graveyard_play.is_none()
            && script.targeting_discards.is_none()
            && script.records_play_as.is_none()
            && script.draw_limit.is_none()
            && script.replacements.is_empty()
            && script.hero_guard.is_none()
            && script.conditional_keywords.is_none()
            && script.after_attack.is_none()
            && script.plague_multiplier.is_none()
            && script.deck_triggers.is_empty()
            && script.graveyard_triggers.is_empty()
            && script.quests.is_none()
            && script.tribute_when.is_none()
            && script.would_counter.is_none()
            && script.start_of_opponent_turn.is_none()
    }

    mod c_41_state_of_the_game {
        use super::*;

        #[test]
        fn is_keywords_only_both_faces_are_printed_on_the_catalog_so_neither_script_adds_anything() {
            crate::register_all();
            assert_eq!(ID, STATE);
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["base"]["keywords"], json!([{ "kind": "Indestructible" }]));
            assert_eq!(def["radiant"]["keywords"], json!([{ "kind": "Indestructible" }, { "kind": "Lifesteal" }]));
            let scripts = script();
            assert!(is_empty_script(&scripts.base));
            assert!(is_empty_script(&scripts.radiant));
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_3_3_with_indestructible_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FILLER], "field": [STATE] }, "p2": { "hand": [FILLER] } }));

                s.expect_stats(STATE, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
                assert_eq!(keyword_kinds(&s, PlayerId::P1, 1), vec!["Indestructible"]);
            }

            #[test]
            fn c4_4_step_4_combat_damage_never_removes_it_it_attacks_a_9_9_and_takes_nothing_back() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                let menace = s.unit(PlayerId::P2, 1).expect("the 9/9 should be on the board").id.clone();

                s.attack(STATE, &menace);

                s.expect_in_zone(STATE, "field");
                s.expect_stats(STATE, json!({ "health": 3, "maxHealth": 3 }));
                s.expect_stats(&menace, json!({ "health": 6 }));
                // R63: the strike back is reduced to nothing, so no damage event names it.
                let state = s.card(STATE).id.clone();
                assert_eq!(
                    s.events().iter().map(js).filter(|event| event["type"] == "damage" && event["targetId"] == state).count(),
                    0,
                );
            }

            #[test]
            fn c4_4_step_4_a_damage_effect_never_removes_it_book_of_flames_4_deals_it_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [FLAME, FILLER] },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(FLAME, json!({ "targets": [{ "pick": "instance", "instanceId": state }] }));

                s.expect_in_zone(&state, "field");
                s.expect_stats(&state, json!({ "health": 3, "maxHealth": 3 }));
                assert_eq!(
                    s.events().iter().map(js).filter(|event| event["type"] == "damage" && event["targetId"] == state).count(),
                    0,
                );
            }

            #[test]
            fn r46_a_destroy_effect_leaves_it_on_the_field_and_knocks_it_into_attack_position() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "position": "DEF" }] },
                    "p2": { "hand": [HIT_JOB, FILLER] },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": state }] }));

                s.expect_in_zone(&state, "field");
                assert_eq!(js(&s.stats(&state).position), "ATK");
                assert_eq!(s.pile(PlayerId::P1, "graveyard").len(), 0);
                s.expect_events(json!(["positionSwitched"]));
            }

            #[test]
            fn r347_it_never_has_taunt_even_in_defense_position_where_it_keeps_only_the_armor_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "position": "DEF" }, { "def": VANILLA, "lane": 2 }] },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                    "active": "p2",
                }));

                assert_eq!(keyword_kinds(&s, PlayerId::P1, 1), vec!["Indestructible", "Armor"]);
                assert_eq!(s.stats(STATE).armor, 1);

                // §4.2 step 3 finds no Taunt on p1's side, so the attacker may pass it by for the Vanilla.
                let attacker = s.unit(PlayerId::P2, 1).map(|unit| unit.id.clone());
                let other = s.unit(PlayerId::P1, 2).map(|unit| unit.id.clone());
                let (Some(attacker), Some(other)) = (attacker, other) else {
                    panic!("both Vanillas should be on the board");
                };
                s.attack(&attacker, &other);
                s.expect_events(json!(["attackDeclared"]));
            }

            #[test]
            fn c6_1_an_exile_removes_it_no_destroy_is_involved_so_indestructible_does_not_help() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [COLLATERAL, FILLER], "library": [FILLER, FILLER] },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": state }] }));

                s.expect_in_zone(&state, "exile");
                assert!(s.unit(PlayerId::P1, 1).is_none());
            }

            #[test]
            fn c6_1_a_bounce_removes_it_to_its_controllers_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [FLOOD, FILLER] },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(FLOOD, json!({}));

                s.expect_in_zone(&state, "hand");
                assert!(s.hand(PlayerId::P1).iter().any(|card| card.id == state));
            }

            #[test]
            fn c6_3_a_tribute_removes_it_sacrifice_bypasses_indestructible_and_counts_as_a_death() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CUBE, FILLER], "field": [STATE] }, "p2": { "hand": [FILLER] } }));
                let state = s.card(STATE).id.clone();

                s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": state }] }));

                s.expect_in_zone(&state, "graveyard");
                s.expect_events(json!(["destroyed"]));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_the_radiant_face_is_a_6_6_with_indestructible_and_lifesteal() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));

                s.expect_stats(STATE, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
                assert_eq!(keyword_kinds(&s, PlayerId::P1, 1), vec!["Indestructible", "Lifesteal"]);
            }

            #[test]
            fn c4_4_step_8_its_damage_to_the_enemy_hero_heals_your_hero_the_amount_dealt() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true }], "health": 20 },
                    "p2": { "hand": [FILLER] },
                }));

                s.attack(STATE, "hero");

                s.expect_health(PlayerId::P2, 24);
                s.expect_health(PlayerId::P1, 26);
            }

            #[test]
            fn c4_4_step_8_its_damage_to_a_unit_heals_you_too_and_the_strike_back_still_deals_it_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true }], "health": 10 },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                let menace = s.unit(PlayerId::P2, 1).expect("the 9/9 should be on the board").id.clone();

                s.attack(STATE, &menace);

                s.expect_stats(&menace, json!({ "health": 3 }));
                s.expect_stats(STATE, json!({ "health": 6, "maxHealth": 6 }));
                s.expect_health(PlayerId::P1, 16);
            }

            #[test]
            fn r347_still_no_taunt_in_defense_position() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true, "position": "DEF" }] },
                    "p2": { "hand": [FILLER] },
                }));

                assert_eq!(keyword_kinds(&s, PlayerId::P1, 1), vec!["Indestructible", "Lifesteal", "Armor"]);
            }

            #[test]
            fn r46_a_destroy_effect_leaves_the_radiant_face_on_the_field_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true }] },
                    "p2": { "hand": [HIT_JOB, FILLER] },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": state }] }));

                s.expect_in_zone(&state, "field");
                s.expect_stats(&state, json!({ "health": 6 }));
            }
        }
    }
}
