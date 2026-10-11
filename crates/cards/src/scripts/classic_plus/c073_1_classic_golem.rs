//! C+ #73.1 Classic Golem (SPEC §8.7 row 73.1, B7). (4) Unit, Token (printed Legendary). 10/10 → 20/20.
//!   Base:    "Rush, First Strike, Trample / After this attacks a Unit, it transforms into a random
//!            Classic or Classic+ Unit. If this destroyed that Unit, the new Unit may attack again
//!            this turn."
//!   Radiant: "Rush, Trample, Divine Shield / After this attacks a Unit, it transforms into a random
//!            Radiant Classic or Classic+ Unit. If this destroyed that Unit, the new Unit may attack
//!            again this turn."
//!
//! R424: the transform follows the combat of an attack the Golem declared on a Unit (not a forced one,
//! R53), so its own stats fight (§4.4); a Golem that left the field transforms into nothing. The new
//! Unit is a random non-token Classic or Classic+ one (R380; no rng draw when none can land, R129),
//! transformed in place (§6.3) with no Cry (R1), type kept (R35); killing the defender (R42) readies it.

use jackioh_engine::effects::transform_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-073-1";

/// §8.7: "a random Classic or Classic+ Unit" — non-token Units of the two sets the text names (R380).
fn classic_units() -> Value {
    json!({ "type": "Unit", "set": ["Classic", "Classic+"] })
}

/// A hero target is `hero-<player>` in the combat's facts.
const HERO_TARGET: &str = "hero-";

fn after_it_attacks(radiant: bool) -> Hook {
    hook(move |ctx| {
        let Some(combat) = after_attack_of(ctx) else {
            return Vec::new();
        };
        if combat.forced || !combat.survived || combat.target_id.starts_with(HERO_TARGET) {
            return Vec::new();
        }
        let destroyed = combat.destroyed_ids.contains(&combat.target_id);
        let mut args = json!({ "target": { "of": "self" }, "query": classic_units() });
        if radiant {
            args["radiant"] = json!(true);
        }
        args["readyToAttack"] = json!(destroyed);
        vec![transform_random(json_as(args))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        after_attack: Some(after_it_attacks(false)),
        ..Script::default()
    };

    // The Radiant face transforms into a Radiant Unit; its 20/20 and keywords are its catalog face.
    let radiant = Script {
        after_attack: Some(after_it_attacks(true)),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// BUILD M9 Classic+ row C+ 73.1: after the combat of an attack it declared on a Unit (R424), if it
// survived, it is transformed (§6.3) into a random non-token Classic or Classic+ Unit (a Unit, R35)
// with no Cry (R1); if it destroyed the defender (R42) the new Unit may attack again; a Golem that dies,
// or attacks the hero, transforms nothing; radiant 20/20 with Divine Shield for First Strike.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GOLEM: &str = "classicplus-073-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4

    /// 2/12: survives a 10-attack First Strike.
    fn fauci() -> Value {
        json!({ "def": "core-091", "radiant": true })
    }

    /// 18/18 Taunt: kills a 10/10 back.
    fn big_menace() -> Value {
        json!({ "def": "core-019", "radiant": true })
    }

    use crate::js;

    fn golem(defender: Option<Value>, radiant_face: bool, seed: Option<&str>) -> Scenario {
        let field: Vec<Value> = defender.into_iter().collect();
        let mut options = json!({
            "p1": { "hand": [VANILLA], "field": [{ "def": GOLEM, "radiant": radiant_face }], "library": [VANILLA] },
            "p2": { "hand": [VANILLA], "field": field, "library": [VANILLA] }
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    struct Transformed {
        from_def_id: String,
        to_def_id: String,
        new_instance_id: String,
    }

    fn transformed(s: &Scenario) -> Vec<Transformed> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Transformed { from_def_id, to_def_id, new_instance_id, .. } => Some(Transformed {
                    from_def_id: from_def_id.clone(),
                    to_def_id: to_def_id.clone(),
                    new_instance_id: new_instance_id.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// The Golem attacks p2's lane-1 Unit, or the hero when there is none.
    fn attack_with(s: &mut Scenario) {
        match s.unit(P2, 1) {
            Some(target) => s.attack(GOLEM, &target.id),
            None => s.attack(GOLEM, "hero"),
        };
    }

    fn types_of(s: &Scenario) -> Vec<&'static str> {
        s.events().iter().map(|event| event.event_type().as_str()).collect()
    }

    /// −1 when absent.
    fn index_of(order: &[&str], kind: &str) -> i64 {
        order.iter().position(|entry| *entry == kind).map_or(-1, |at| at as i64)
    }

    fn last_index_of(order: &[&str], kind: &str) -> i64 {
        order.iter().rposition(|entry| *entry == kind).map_or(-1, |at| at as i64)
    }

    fn keyword_kinds(keywords: &[Keyword]) -> Vec<Value> {
        keywords.iter().map(|keyword| js(keyword)["kind"].clone()).collect()
    }

    #[test]
    fn is_a_4_unit_token_printed_legendary_10_10_rush_first_strike_trample_radiant_20_20_trades_first_strike_for_divine_shield() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert!(def.token);
        assert_eq!(js(&def.printed_rarity), json!("Legendary"));
        assert_eq!(js(&def.cost), json!(4));
        assert_eq!([def.base.attack, def.base.health], [Some(10), Some(10)]);
        assert_eq!(keyword_kinds(&def.base.keywords), vec![json!("Rush"), json!("First Strike"), json!("Trample")]);
        assert_eq!([def.radiant.attack, def.radiant.health], [Some(20), Some(20)]);
        assert_eq!(keyword_kinds(&def.radiant.keywords), vec![json!("Rush"), json!("Trample"), json!("Divine Shield")]);
        let CardScripts { base, radiant } = script();
        assert!(base.after_attack.is_some());
        assert!(radiant.after_attack.is_some());
        assert!(!Arc::ptr_eq(base.after_attack.as_ref().unwrap(), radiant.after_attack.as_ref().unwrap()));
    }

    mod base {
        use super::*;

        #[test]
        fn r424_its_own_stats_fight_first_first_strike_kills_the_defender_unhurt_and_trample_s_excess_hits_the_hero() {
            crate::register_all();
            let mut s = golem(Some(json!(VANILLA)), false, None);
            let defender = s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default();
            attack_with(&mut s);
            s.expect_in_zone(defender.as_str(), "graveyard");
            s.expect_health(P2, 24);
            // The fight ended before the transform: the defender died to the Golem's hit.
            let order = types_of(&s);
            assert!(last_index_of(&order, "destroyed") < index_of(&order, "transformed"));
        }

        #[test]
        fn r424_r380_r35_after_attacking_a_unit_it_becomes_a_random_non_token_classic_or_classic_unit_on_its_base_face() {
            crate::register_all();
            for seed in 0..12 {
                let name = format!("golem-{seed}");
                let mut s = golem(Some(json!(VANILLA)), false, Some(&name));
                attack_with(&mut s);
                let events = transformed(&s);
                let event = events.first();
                assert_eq!(event.map(|event| event.from_def_id.as_str()), Some(GOLEM));
                let made = crate::card_def(event.map(|event| event.to_def_id.as_str()).unwrap_or(""));
                assert_eq!(made.type_, CardType::Unit);
                assert!(!made.token);
                assert!(js(&made.set) == json!("Classic") || js(&made.set) == json!("Classic+"));
                let unit = s.unit(P1, 1);
                // B2.7: an X-stats Unit (C+ #69 Buff Billy) made outside a play has no X, so it is 0/0 and dies.
                if made.base.x_stats.is_some() {
                    assert!(unit.is_none());
                    continue;
                }
                assert_eq!(
                    unit.as_ref().map(|unit| unit.id.clone()),
                    event.map(|event| event.new_instance_id.clone()),
                    "{seed}: {}",
                    event.map(|event| event.to_def_id.as_str()).unwrap_or("")
                );
                assert_eq!(unit.as_ref().map(|unit| unit.radiant), Some(false));
                assert_eq!(unit.as_ref().map(|unit| js(&unit.position)), Some(json!("ATK")));
            }
        }

        #[test]
        fn r1_the_new_unit_fires_no_cry_it_is_transformed_never_summoned_or_played() {
            crate::register_all();
            let mut s = golem(Some(json!(VANILLA)), false, None);
            attack_with(&mut s);
            let order = types_of(&s);
            // A missing transform slices from the last event.
            let from = match order.iter().position(|kind| *kind == "transformed") {
                Some(at) => at,
                None => order.len().saturating_sub(1),
            };
            assert!(!order[from..].iter().any(|kind| *kind == "summoned" || *kind == "cardPlayed"));
        }

        #[test]
        fn r42_it_destroyed_the_defender_the_new_unit_has_a_fresh_exertion_and_no_sickness_so_it_may_attack_again() {
            crate::register_all();
            let mut s = golem(Some(json!(VANILLA)), false, None);
            attack_with(&mut s);
            let made = s.unit(P1, 1);
            assert_ne!(made.as_ref().map(|unit| unit.def_id.as_str()), Some(GOLEM));
            assert_eq!(made.as_ref().and_then(|unit| unit.summoned_turn), None);
            assert_eq!(made.as_ref().map(|unit| js(&unit.exertion)), Some(json!({ "attacked": false, "switched": false })));
            assert!(made.is_some());
            let made = made.map(|unit| unit.id).unwrap_or_default();
            assert!(s.stats(made.as_str()).attack > 0);
            let attacks = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| action["type"] == json!("attack") && action["attackerId"] == json!(made))
                .count();
            assert!(attacks > 0);
            s.attack(made.as_str(), "hero");
            assert_eq!(types_of(&s).iter().filter(|kind| **kind == "attackDeclared").count(), 2);
        }

        #[test]
        fn r424_the_fuzz_monitor_agrees_the_readied_new_unit_is_no_sick_attack_i1_and_its_missing_summonedturn_no_i4_mismatch() {
            crate::register_all();
            // The monitor must not read R424's lifted sickness as a lost entry.
            let mut s = golem(Some(json!(VANILLA)), false, None);
            let mut monitor = create_invariant_monitor(s.state());
            let from = s.events().len();
            attack_with(&mut s);
            assert_eq!(monitor.after(&s.events()[from..], s.state()), Vec::<String>::new());
            let made = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
            let action: ActionBody = json_as(json!({ "type": "attack", "attackerId": made, "targetId": "hero-p2" }));
            let again = monitor.before(s.state(), P1, &action);
            // I3 names the cards the scenario placed without an entry event; nothing else may be found.
            let others: Vec<String> = again.into_iter().filter(|finding| !finding.starts_with("I3")).collect();
            assert_eq!(others, Vec::<String>::new());
        }

        #[test]
        fn r424_a_defender_that_survives_it_still_transforms_but_the_new_unit_is_summoning_sick() {
            crate::register_all();
            let mut s = golem(Some(fauci()), false, None);
            attack_with(&mut s);
            assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some("core-091".to_string()));
            assert_eq!(transformed(&s).len(), 1);
            let made = s.unit(P1, 1);
            assert_eq!(made.as_ref().and_then(|unit| unit.summoned_turn), Some(s.state().turn));
            let made_id = made.map(|unit| unit.id);
            assert!(
                !legal_actions(s.state(), P1)
                    .iter()
                    .map(js)
                    .any(|action| action["type"] == json!("attack") && action["attackerId"] == json!(made_id))
            );
        }

        #[test]
        fn r424_a_golem_that_dies_in_the_combat_transforms_into_nothing() {
            crate::register_all();
            let mut s = golem(Some(big_menace()), false, None);
            let it = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
            attack_with(&mut s);
            s.expect_in_zone(it.as_str(), "gone");
            assert!(transformed(&s).is_empty());
            assert!(s.unit(P1, 1).is_none());
        }

        #[test]
        fn r424_a_golem_that_attacks_the_hero_transforms_nothing() {
            crate::register_all();
            let mut s = golem(None, false, None);
            attack_with(&mut s);
            s.expect_health(P2, 20);
            assert!(transformed(&s).is_empty());
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
        }

        #[test]
        fn r53_r424_a_forced_attack_is_no_attack_it_declared_it_transforms_nothing() {
            crate::register_all();
            // Core #9 Moths to the Flame (1/14): "Start of turn: Every enemy Unit attacks this." Damaged to 9,
            // so the Golem's 10 kills it and the forced attack is one that destroyed its target.
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA], "field": [GOLEM], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "field": [{ "def": "core-009", "damage": 5 }], "library": [VANILLA] }
            }));
            s.end_turn();
            assert!(
                s.events()
                    .iter()
                    .any(|event| event.event_type().as_str() == "attackDeclared" && js(event)["forced"] == json!(true))
            );
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == "core-009"));
            assert!(transformed(&s).is_empty());
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
        }

        #[test]
        fn r23_r424_an_immutable_golem_is_not_transformed_and_takes_no_draw_r129() {
            crate::register_all();
            let mut s = golem(Some(json!(VANILLA)), false, None);
            let id = s.card(GOLEM).id.clone();
            find_instance_mut(s.state_mut(), &id)
                .expect("the Golem is on the field")
                .granted_keywords
                .push(json_as(json!({ "kind": "Immutable" })));
            let defender = s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default();
            let cursor = s.state().rng_cursor;
            attack_with(&mut s);
            s.expect_in_zone(defender.as_str(), "graveyard");
            assert!(transformed(&s).is_empty());
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn r129_a_fixed_seed_makes_the_same_unit_the_draw_is_one_rng_pick() {
            crate::register_all();
            let mut one = golem(Some(json!(VANILLA)), false, Some("golem-fixed"));
            attack_with(&mut one);
            let mut two = golem(Some(json!(VANILLA)), false, Some("golem-fixed"));
            attack_with(&mut two);
            assert_eq!(
                transformed(&one).first().map(|event| event.to_def_id.clone()),
                transformed(&two).first().map(|event| event.to_def_id.clone())
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn t_20_20_kills_a_2_12_its_divine_shield_taking_the_hit_and_transforms_into_a_radiant_unit_ready_to_attack_again() {
            crate::register_all();
            let mut s = golem(Some(fauci()), true, None);
            attack_with(&mut s);
            s.expect_in_zone("core-091", "graveyard");
            // No First Strike on the Radiant face: the defender hits back, and the Shield absorbs it.
            assert!(s.events().iter().any(|event| event.event_type().as_str() == "divineShieldLost"));
            let events = transformed(&s);
            let made = crate::card_def(events.first().map(|event| event.to_def_id.as_str()).unwrap_or(""));
            assert_ne!(js(&made.set), json!("Core"));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.radiant), Some(true));
            assert_eq!(s.unit(P1, 1).and_then(|unit| unit.summoned_turn), None);
        }

        #[test]
        fn divine_shield_absorbs_the_hit_of_a_defender_that_survives_the_golem_s_20_it_still_transforms_summoning_sick() {
            crate::register_all();
            let mut s = golem(Some(json!({ "def": VANILLA, "statsOverride": { "attack": 5, "health": 30 } })), true, None);
            assert!(keyword_kinds(&s.stats(GOLEM).keywords).contains(&json!("Divine Shield")));
            attack_with(&mut s);
            assert_eq!(s.unit(P2, 1).map(|unit| unit.damage), Some(20));
            assert!(s.events().iter().any(|event| event.event_type().as_str() == "divineShieldLost"));
            assert_eq!(transformed(&s).len(), 1);
            assert_eq!(s.unit(P1, 1).and_then(|unit| unit.summoned_turn), Some(s.state().turn));
        }

        #[test]
        fn r424_attacking_the_hero_transforms_nothing() {
            crate::register_all();
            let mut s = golem(None, true, None);
            attack_with(&mut s);
            s.expect_health(P2, 10);
            assert!(transformed(&s).is_empty());
        }
    }
}
