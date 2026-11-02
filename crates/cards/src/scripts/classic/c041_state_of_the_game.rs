//! C #41 State of the Game (SPEC §8.6 row 41). Unit 3/3 → 6/6, cost 1, Common.
//!   Base:    "Indestructible, Immutable"
//!   Radiant: "Indestructible, Immutable, Lifesteal" — the adopted Radiant face: a keyword-only Unit's
//!            Radiant owes one more keyword or a stronger one (R275), and Taunt is out (R347).
//!   Engine:  "Keywords only; under R347 it never has Taunt, and under R2179000 it is never in Defense."
//!
//! Nothing to script: both faces' keywords are printed on the catalog entry and §10.4 layer 1 reads
//! them from there (Immutable since issue #2179, catalog data, so no script is kept). Where the
//! behavior lives instead:
//!   Indestructible — §4.4 step 4 (it takes no damage), §4.5 step 1 and R46 (a destroy mark is
//!                    ignored), R69 (it still dies if its max health falls to 0), §6.1 (exile, bounce
//!                    and a Tribute still remove it), R347 (§10.4 drops Taunt whenever it holds
//!                    Indestructible) and R2179000 (it can't be in Defense Position).
//!   Immutable      — R23, R386 and R781001: no Transform, Vanilla, Fuse-onto, Nerf or Buff, and while
//!                    it is in play no Spell, Field Spell or Field Trap targets it; Units, attacks,
//!                    random picks and "all" effects still reach it.
//!   Lifesteal      — §4.4 step 8 heals its controller's hero the amount it actually deals.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-041";

pub fn script() -> CardScripts {
    let base = Script::default();

    CardScripts { radiant: base.clone(), base }
}

// C #41 State of the Game — SPEC §8.6 row 41, BUILD M9 Classic row C 41 (R347, R2179000, R781001). Both
// scripts are empty: these fixtures prove the catalog keywords do the work (§4.1, §4.2, §4.4, §4.5). Removal
// sources, none of them aimed at it: Whirlwind (classicplus-021), Twisting Nether (core-088), Lag in
// the System (classic-025), Flood (core-017) and Carnivorous Cube (core-022, whose Cry Tributes, R428).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const STATE: &str = "classic-041";
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const WHIRLWIND: &str = "classicplus-021"; // (0) Spell: Pierce. Deal 1 damage to all Units.
    const NETHER: &str = "core-088"; // (4) Spell: Destroy all permanents.
    const LAG: &str = "classic-025"; // (0) Spell: Exile every (1) Cost or less card on the field.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const CUBE: &str = "core-022"; // (3) Unit: Cry: Tribute one of your other Units and remember it.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no text.
    const FILLER: &str = "core-005"; // (1) Spell, a card to keep a hand from auto-ending the turn (§2.5).
    const FLAME: &str = "classic-016"; // (1) Spell, Book: Deal 4 damage.

    use crate::js;

    fn keyword_kinds(s: &Scenario, player: PlayerId, lane: i32) -> Vec<String> {
        let unit = s.unit(player, lane).unwrap_or_else(|| panic!("no unit in {} lane {}", js(&player), lane));
        s.stats(&unit.id).keywords.iter().map(|keyword| js(keyword)["kind"].as_str().unwrap_or("").to_string()).collect()
    }

    fn switch_offered(s: &Scenario, id: &str) -> bool {
        legal_actions(s.state(), PlayerId::P1)
            .iter()
            .any(|action| matches!(action, ActionBody::SwitchPosition { instance_id } if instance_id == id))
    }

    mod c_41_state_of_the_game {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn s4_4_step_4_combat_damage_never_removes_it_it_attacks_a_9_9_and_takes_nothing_back() {
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
            fn s4_4_step_4_a_damage_effect_never_removes_it_whirlwind_deals_it_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [WHIRLWIND, FILLER] },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(WHIRLWIND, json!({}));

                s.expect_in_zone(&state, "field");
                s.expect_stats(&state, json!({ "health": 3, "maxHealth": 3 }));
                assert_eq!(
                    s.events().iter().map(js).filter(|event| event["type"] == "damage" && event["targetId"] == state).count(),
                    0,
                );
            }

            #[test]
            fn r781001_immutable_no_spell_may_target_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [FLAME, HIT_JOB, FILLER], "mana": 6 },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();
                for spell in [FLAME, HIT_JOB] {
                    s.expect_refused(|s| s.play(spell, json!({ "targets": [{ "pick": "instance", "instanceId": state }] })));
                }
                s.expect_in_zone(&state, "field");
                assert!(keyword_kinds(&s, PlayerId::P1, 1).contains(&"Immutable".to_string()));
            }

            #[test]
            fn r46_a_destroy_effect_leaves_it_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [NETHER, FILLER], "mana": 4 },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(NETHER, json!({}));

                s.expect_in_zone(&state, "field");
                assert_eq!(js(&s.stats(&state).position), "ATK");
                assert_eq!(s.pile(PlayerId::P1, "graveyard").len(), 0);
            }

            #[test]
            fn r347_r2179000_it_never_has_taunt_and_never_stands_in_defense() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE, { "def": VANILLA, "lane": 2 }] },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                let state = s.card(STATE).id.clone();

                assert_eq!(keyword_kinds(&s, PlayerId::P1, 1), vec!["Indestructible", "Immutable"]);
                // R2179000: no switch to Defense is offered or accepted.
                assert!(!switch_offered(&s, &state));
                s.expect_refused_with(|s| s.switch_position(state.as_str()), "Defense Position");
                assert_eq!(js(&s.stats(&state).position), "ATK");

                // §4.2 step 3 finds no Taunt on p1's side, so the attacker may pass it by for the Vanilla.
                s.end_turn();
                let attacker = s.unit(PlayerId::P2, 1).map(|unit| unit.id.clone());
                let other = s.unit(PlayerId::P1, 2).map(|unit| unit.id.clone());
                let (Some(attacker), Some(other)) = (attacker, other) else {
                    panic!("both Vanillas should be on the board");
                };
                s.attack(&attacker, &other);
                s.expect_events(json!(["attackDeclared"]));
            }

            #[test]
            fn s6_1_an_exile_removes_it_no_destroy_is_involved_so_indestructible_does_not_help() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [STATE] },
                    "p2": { "hand": [LAG, FILLER], "library": [FILLER, FILLER] },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(LAG, json!({}));

                s.expect_in_zone(&state, "exile");
                assert!(s.unit(PlayerId::P1, 1).is_none());
            }

            #[test]
            fn s6_1_a_bounce_removes_it_to_its_controllers_hand() {
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
            fn s6_3_a_tribute_removes_it_sacrifice_bypasses_indestructible_and_counts_as_a_death() {
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
            fn r275_the_radiant_face_is_a_6_6_with_indestructible_immutable_and_lifesteal() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));

                s.expect_stats(STATE, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
                assert_eq!(keyword_kinds(&s, PlayerId::P1, 1), vec!["Indestructible", "Immutable", "Lifesteal"]);
            }

            #[test]
            fn s4_4_step_8_its_damage_to_the_enemy_hero_heals_your_hero_the_amount_dealt() {
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
            fn s4_4_step_8_its_damage_to_a_unit_heals_you_too_and_the_strike_back_still_deals_it_nothing() {
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
            fn r2179000_set_up_in_defense_it_stands_in_attack_and_has_no_taunt() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true }] },
                    "p2": { "hand": [FILLER, WHIRLWIND] },
                }));
                let state = s.card(STATE).id.clone();
                s.card_mut(&state).position = Some(Position::Def);
                s.end_turn();
                s.play(WHIRLWIND, json!({}));
                assert_eq!(js(&s.stats(&state).position), "ATK");
                assert_eq!(keyword_kinds(&s, PlayerId::P1, 1), vec!["Indestructible", "Immutable", "Lifesteal"]);
            }

            #[test]
            fn r46_a_destroy_effect_leaves_the_radiant_face_on_the_field_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": STATE, "radiant": true }] },
                    "p2": { "hand": [NETHER, FILLER], "mana": 4 },
                    "active": "p2",
                }));
                let state = s.card(STATE).id.clone();

                s.play(NETHER, json!({}));

                s.expect_in_zone(&state, "field");
                s.expect_stats(&state, json!({ "health": 6 }));
            }
        }
    }
}
