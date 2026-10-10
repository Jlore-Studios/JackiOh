//! C+ #32.3 Blade Storm (SPEC §8.7 row 32.3; §4.5, R59, R283, R386, R652). (1) Spell token, printed Epic.
//!   Base:    "Cast Whirlwind until a Unit dies." (the round cap stays a declared number, not shown)
//!   Radiant: "Deal 1 damage to all enemy Units. Repeat until a Unit dies, up to {rounds|time|times}."
//!
//! Each round is one effect list followed by its own state check (one of the two lists R59 lets check
//! inside themselves, beside R283's) so killed Units die and Death hooks resolve before the next round. The storm stops after a round in which any Unit died
//! (Reborn counts), after its round cap, or when no Unit is left. Cap is declared `rounds` (R386),
//! read through `param`: `BLADE_STORM_ROUNDS` (30) on both faces. Base face casts Whirlwind (C+ #21)
//! as a real Spell cast (R70, §4.4), so Divine Shield, Armor (Pierce goes through it, R652) and Spell
//! Damage apply round by round.
//! Radiant face hits enemy Units only; a death on either side still stops it (refs Whirlwind, R279).

use jackioh_engine::effects::{cast_rounds_until_death, damage_rounds_until_death};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-032-3";

/// Whirlwind (C+ #21), which the base face casts round after round.
const WHIRLWIND: &str = "classicplus-021";

/// The Radiant face's hit of each round, "Deal 1 damage", is the declared number `damage` (R386);
/// the base face's rounds cast Whirlwind, whose own 1 is Whirlwind's number.
pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![cast_rounds_until_death(json_as(json!({
                "def": WHIRLWIND,
                "rounds": param(&*ctx, "rounds"),
            })))]
        })),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|ctx| {
            vec![damage_rounds_until_death(json_as(json!({
                "amount": param(&*ctx, "damage"),
                "rounds": param(&*ctx, "rounds"),
                "side": "enemy",
            })))]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #32.3 Blade Storm (SPEC §8.7, R59, R70, R283, R652): base face casts Whirlwind round after
// round (C+ #21, Pierce, so Armor does not stop it) until a Unit dies, no Unit is left, or round cap is reached. Radiant hits enemy Units only.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const STORM: &str = "classicplus-032-3";
    const WHIRLWIND: &str = "classicplus-021";
    const MENACE: &str = "core-019"; // 9/9 Taunt
    const TIMMY: &str = "core-011"; // 3/3
    const JILLIAX: &str = "core-056"; // 3/2, Divine Shield
    const DEFENDER: &str = "core-003"; // 1/1, Divine Shield, Reborn
    const SEVEN: &str = "core-025"; // 7/7, Armor 7
    const UNBREAKABLE: &str = "classic-041"; // 3/3, Indestructible
    const SOLARIUS: &str = "classicplus-038"; // 3/2, Spell Damage +2
    const HOGAR: &str = "classicplus-028"; // 3/4 Taunt, Reborn; Death: heal your hero 3
    const FILLER: &str = "core-005";

    /// A 3/50 Tempo Timmy: it outlives any storm, so its hits count the rounds.
    fn counter() -> Value {
        json!({ "def": TIMMY, "statsOverride": { "attack": 3, "health": 50 } })
    }

    fn storm(p1: Value, p2: Value, radiant: bool, tune: Option<i32>) -> Scenario {
        let mut own = p1;
        own["hand"] = json!([{ "def": STORM, "radiant": radiant }, FILLER]);
        let mut theirs = json!({ "hand": [FILLER] });
        if let Value::Object(fields) = p2 {
            for (key, value) in fields {
                theirs[key.as_str()] = value;
            }
        }
        let mut s = scenario(json!({ "p1": own, "p2": theirs }));
        if let Some(tune) = tune {
            let id = s.card(STORM).id.clone();
            let live = find_instance_mut(s.state_mut(), &id).expect("the storm is in the state");
            step_param(live, "rounds", tune);
        }
        s.play(STORM, json!({}));
        s
    }

    fn hits(s: &Scenario, card: Option<CardInstance>) -> Vec<i32> {
        let card = card.expect("no such unit");
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if *target_id == card.id => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// How many Whirlwinds the storm cast: one real Spell cast per round (R70, R652).
    fn whirlwinds(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == WHIRLWIND))
            .count()
    }

    fn deaths(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn damage_amounts(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// The unit's id, or the fallback reference.
    fn unit_or(s: &Scenario, seat: PlayerId, lane: i32, fallback: &str) -> String {
        s.unit(seat, lane).map(|card| card.id).unwrap_or_else(|| fallback.to_string())
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    use crate::js;

    mod c_n32_3_blade_storm {
        use super::*;

        #[test]
        fn declares_its_round_cap_blade_storm_rounds_on_both_faces_step_8_r386() {
            crate::register_all();
            assert_eq!(ID, STORM);
            let def = crate::card_def(ID);
            assert_eq!(def.id, STORM);
            let scripts = script();
            // The two faces are two scripts.
            assert!(!std::sync::Arc::ptr_eq(
                scripts.base.cry.as_ref().unwrap(),
                scripts.radiant.cry.as_ref().unwrap()
            ));
            assert_eq!(BLADE_STORM_ROUNDS, 30);
            assert_eq!(
                js(&def.params),
                json!([
                    { "key": "rounds", "base": BLADE_STORM_ROUNDS, "radiant": BLADE_STORM_ROUNDS, "better": "up", "step": 8, "min": 1 },
                    { "key": "damage", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 }
                ])
            );
            assert_eq!(def.base.text, "Cast Whirlwind until a Unit dies.");
        }

        mod base {
            use super::*;

            #[test]
            fn r652_casts_whirlwind_round_after_round_until_one_dies_the_3_3_dies_in_round_3_and_the_storm_stops() {
                crate::register_all();
                let mut s = storm(json!({ "field": [TIMMY] }), json!({ "field": [MENACE] }), false, None);
                let menace = s.unit(P2, 1);
                assert_eq!(hits(&s, menace), [1, 1, 1]);
                assert_eq!(whirlwinds(&s), 3);
                assert_eq!(deaths(&s), [TIMMY]);
                s.expect_stats(MENACE, json!({ "health": 6 }));
                s.expect_in_zone(STORM, "graveyard");
            }

            #[test]
            fn r59_divine_shields_pop_in_the_first_round() {
                crate::register_all();
                let s = storm(json!({ "field": [JILLIAX] }), json!({ "field": [MENACE] }), false, None);
                let index_of = |want: GameEventType| -> i64 {
                    s.events()
                        .iter()
                        .position(|event| event.event_type() == want)
                        .map(|at| at as i64)
                        .unwrap_or(-1)
                };
                assert!(index_of(GameEventType::DivineShieldLost) < index_of(GameEventType::Destroyed));
                assert_eq!(hits(&s, s.unit(P2, 1)), [1, 1, 1]);
                assert_eq!(deaths(&s), [JILLIAX]);
            }

            #[test]
            fn r59_a_reborn_death_counts_the_storm_stops_and_the_unit_comes_back() {
                crate::register_all();
                let mut s = storm(json!({ "field": [DEFENDER] }), json!({ "field": [MENACE] }), false, None);
                assert_eq!(deaths(&s), [DEFENDER]);
                assert_eq!(hits(&s, s.unit(P2, 1)), [1, 1]);
                s.expect_in_zone(DEFENDER, "field");
                s.expect_stats(DEFENDER, json!({ "health": 1 }));
            }

            #[test]
            fn r59_its_death_hooks_resolve_in_the_round_that_killed_it_and_no_round_follows() {
                crate::register_all();
                let mut s = storm(json!({ "field": [MENACE] }), json!({ "field": [HOGAR], "health": 20 }), false, None);
                // Round 4 kills the 3/4: its Death heals 3 and Reborn brings it back at 1, which nothing hits again.
                s.expect_health(P2, 23);
                s.expect_stats(HOGAR, json!({ "health": 1 }));
                assert_eq!(hits(&s, s.unit(P1, 1)), [1, 1, 1, 1]);
            }

            #[test]
            fn r59_every_hit_of_a_round_lands_before_its_state_check_spell_damage_2_kills_solarius_and_the_3_3_together() {
                crate::register_all();
                let s = storm(json!({ "field": [SOLARIUS] }), json!({ "field": [TIMMY, MENACE] }), false, None);
                assert_eq!(sorted(deaths(&s)), sorted(vec![SOLARIUS.to_string(), TIMMY.to_string()]));
                assert_eq!(hits(&s, s.unit(P2, 2)), [3]);
            }

            #[test]
            fn r652_pierce_goes_through_armor_the_7_7_with_armor_7_takes_1_a_round_and_dies_in_round_7() {
                crate::register_all();
                let mut s = storm(json!({ "field": [SEVEN] }), json!({ "field": [counter()] }), false, None);
                // Seven hits of 1 past Armor 7, one per round; the storm stops in round 7.
                assert_eq!(whirlwinds(&s), 7);
                assert_eq!(deaths(&s), [SEVEN]);
                assert_eq!(hits(&s, s.unit(P2, 1)), [1, 1, 1, 1, 1, 1, 1]);
                let counter_unit = unit_or(&s, P2, 1, TIMMY);
                s.expect_stats(&counter_unit, json!({ "health": 50 - 7 }));
            }

            #[test]
            fn r59_a_board_nothing_kills_runs_exactly_30_rounds_blade_storm_rounds_and_stops() {
                crate::register_all();
                let mut s = storm(json!({ "field": [counter()] }), json!({ "field": [UNBREAKABLE] }), false, None);
                assert_eq!(hits(&s, s.unit(P1, 1)).len(), BLADE_STORM_ROUNDS as usize);
                assert_eq!(whirlwinds(&s), BLADE_STORM_ROUNDS as usize);
                assert!(deaths(&s).is_empty());
                let counter_unit = unit_or(&s, P1, 1, TIMMY);
                s.expect_stats(&counter_unit, json!({ "health": 50 - BLADE_STORM_ROUNDS }));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(STORM, "graveyard");
            }

            #[test]
            fn s9_3_the_storm_replays_from_a_json_copy_to_the_same_hash_and_events() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [STORM, FILLER], "field": [TIMMY, counter()] },
                    "p2": { "hand": [FILLER], "field": [HOGAR] },
                }));
                let action: Action = json_as(json!({
                    "type": "play",
                    "instanceId": s.card(STORM).id,
                    "playerId": "p1",
                    "nonce": "storm-replay",
                }));
                let thawed: GameState = serde_json::from_str(&serde_json::to_string(s.state()).unwrap()).unwrap();
                let live = reduce(s.state(), &action);
                let again = reduce(&thawed, &action);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&again.state), hash_state(&live.state));
                assert_eq!(again.events, live.events);
            }

            #[test]
            fn s8_7_with_no_unit_left_it_casts_nothing() {
                crate::register_all();
                let mut s = storm(json!({}), json!({}), false, None);
                assert!(damage_amounts(&s).is_empty());
                assert_eq!(whirlwinds(&s), 0);
                s.expect_in_zone(STORM, "graveyard");
            }

            #[test]
            fn r386_a_degrade_moves_the_cap_by_its_step_of_8_22_rounds() {
                crate::register_all();
                let s = storm(json!({}), json!({ "field": [UNBREAKABLE, counter()] }), false, Some(-1));
                assert_eq!(hits(&s, s.unit(P2, 2)).len(), (BLADE_STORM_ROUNDS - 8) as usize);
            }

            #[test]
            fn r386_an_upgrade_moves_it_the_other_way_38_rounds() {
                crate::register_all();
                let s = storm(json!({}), json!({ "field": [UNBREAKABLE, counter()] }), false, Some(1));
                assert_eq!(hits(&s, s.unit(P2, 2)).len(), (BLADE_STORM_ROUNDS + 8) as usize);
                assert_eq!(param_decl_of(s.state(), STORM, "rounds").and_then(|decl| decl.step), Some(8));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s8_7_hits_enemy_units_only_your_own_are_never_touched() {
                crate::register_all();
                let s = storm(json!({ "field": [TIMMY] }), json!({ "field": [TIMMY, MENACE] }), true, None);
                assert!(hits(&s, s.unit(P1, 1)).is_empty());
                assert_eq!(hits(&s, s.unit(P2, 2)), [1, 1, 1]);
                assert_eq!(deaths(&s), [TIMMY]);
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id).as_deref(), Some(TIMMY));
            }

            #[test]
            fn r59_spell_damage_raises_every_round_s_hit_on_the_enemy() {
                crate::register_all();
                let mut s = storm(json!({ "field": [SOLARIUS] }), json!({ "field": [MENACE] }), true, None);
                assert_eq!(deaths(&s), [MENACE]);
                assert_eq!(damage_amounts(&s), [3, 3, 3]);
                s.expect_in_zone(SOLARIUS, "field");
            }

            #[test]
            fn r59_an_enemy_board_nothing_kills_runs_the_30_rounds_too_your_own_units_untouched() {
                crate::register_all();
                let s = storm(json!({ "field": [TIMMY] }), json!({ "field": [UNBREAKABLE, counter()] }), true, None);
                assert_eq!(hits(&s, s.unit(P2, 2)).len(), BLADE_STORM_ROUNDS as usize);
                assert!(hits(&s, s.unit(P1, 1)).is_empty());
            }
        }

            #[test]
            fn r386_an_upgrade_hits_for_2_each_round_and_a_degrade_finds_the_hit_at_its_floor_of_1() {
                crate::register_all();
                let mut own = json!({ "field": [TIMMY] });
                own["hand"] = json!([{ "def": STORM, "radiant": true }, FILLER]);
                let mut s = scenario(json!({ "p1": own, "p2": { "hand": [FILLER], "field": [MENACE] } }));
                assert!(!crate::can_degrade_number(&s, STORM, "damage"));
                assert_eq!(crate::upgrade_number(&mut s, STORM, "damage"), 2);
                s.play(STORM, json!({}));
                // 2 a round into a 9/9: it dies in the fifth.
                assert_eq!(hits(&s, s.unit(P2, 1).or_else(|| Some(s.card(MENACE).clone()))), [2, 2, 2, 2, 2]);
            }
    }
}
