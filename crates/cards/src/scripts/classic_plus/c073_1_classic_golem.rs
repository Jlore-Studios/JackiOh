//! C+ #73.1 Classic Golem (SPEC §8.7 row 73.1, B7). (4) Unit, Token (printed Legendary). 10/10 → 20/20.
//!   Base:    "Rush, First Strike, Trample / When this attacks a Unit, that Unit transforms into a random
//!            Radiant Classic or Classic+ Unit. If it dies, this may attack again."
//!   Radiant: "Rush, Trample, Divine Shield / When this attacks a Unit, that Unit transforms into a random
//!            Classic or Classic+ Unit. If it dies, this may attack again."
//!
//! R2285002 (the designer's note, back from R424's self-transform): "when this attacks" is its
//! `when_attacks` hook, before any damage of each attack it makes on a Unit, declared or forced (R53,
//! R2285000). The attacked Unit is transformed in place (§6.3) into a random non-token Classic or
//! Classic+ Unit (R380; no rng draw when none can land or the Unit is Immutable, R23, R129), with no
//! Cry (R1) and type kept (R35): on its Radiant face for the base Golem, on its base face for the
//! Radiant one, whose Radiant faces have the higher stats. The attack goes on against the new Unit.
//! "If this destroys it" is its `after_attack` hook, run only for a Golem that survived the combat
//! (R2285001): when the attacked Unit is among the ones it destroyed (R42), it may attack again. R2657004:
//! a Golem that entered this turn stays summoning sick through that second attack, so its Rush keeps it
//! off the hero; through v0.5.9e the refresh lifted the sickness (`script_v0_5_9e`, kept, R767000).

use jackioh_engine::effects::{may_attack_again, transform_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-073-1";

/// §8.7: "a random Classic or Classic+ Unit" — non-token Units of the two sets the text names (R380).
fn classic_units() -> Value {
    json!({ "type": "Unit", "set": ["Classic", "Classic+"] })
}

/// A hero target is `hero-<player>` in an attack's facts.
const HERO_TARGET: &str = "hero-";

/// "When this attacks a Unit, transform that Unit": `radiant` is the new Unit's face.
fn when_it_attacks(radiant: bool) -> Hook {
    hook(move |ctx| {
        let Some(attack) = when_attacks_of(ctx) else {
            return Vec::new();
        };
        if attack.target_id.starts_with(HERO_TARGET) {
            return Vec::new();
        }
        let args = json!({ "instanceId": attack.target_id, "query": classic_units(), "radiant": radiant });
        vec![transform_random(json_as(args))]
    })
}

/// "If it dies, this may attack again." `lift_sick` is v0.5.9e's refresh, which lifted summoning
/// sickness for the turn (R2657004).
fn after_it_attacks(lift_sick: bool) -> Hook {
    hook(move |ctx| {
        let Some(combat) = after_attack_of(ctx) else {
            return Vec::new();
        };
        if combat.target_id.starts_with(HERO_TARGET) || !combat.destroyed_ids.contains(&combat.target_id) {
            return Vec::new();
        }
        let mut args = json!({ "target": { "of": "self" } });
        if lift_sick {
            args["liftSick"] = json!(true);
        }
        vec![may_attack_again(json_as(args))]
    })
}

fn scripts(lift_sick: bool) -> CardScripts {
    let base = Script {
        when_attacks: Some(when_it_attacks(true)),
        after_attack: Some(after_it_attacks(lift_sick)),
        ..Script::default()
    };

    // The Radiant face transforms into a base-face Unit; its 20/20 and keywords are its catalog face.
    let radiant = Script {
        when_attacks: Some(when_it_attacks(false)),
        after_attack: Some(after_it_attacks(lift_sick)),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    scripts(false)
}

/// R767000: through v0.5.9e its second attack lifted its summoning sickness, so a Golem played that
/// turn could hit the hero after a kill, before R2657004; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9e() -> CardScripts {
    scripts(true)
}

// BUILD M9 Classic+ row C+ 73.1: when it attacks a Unit, declared or forced, before any damage
// (R2285000), that Unit is transformed (§6.3) into a random non-token Classic or Classic+ Unit (a Unit,
// R35) with no Cry (R1), Radiant for the base Golem and base-face for the Radiant one (R2285002); the
// Golem fights the new Unit, and destroying it lets a surviving Golem attack again (R2285001); an
// attack on the hero transforms nothing; radiant 20/20 with Divine Shield for First Strike.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GOLEM: &str = "classicplus-073-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4

    use crate::js;

    fn golem(defender: Option<Value>, radiant_face: bool, seed: &str) -> Scenario {
        let field: Vec<Value> = defender.into_iter().collect();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [VANILLA], "field": [{ "def": GOLEM, "radiant": radiant_face }], "library": [VANILLA] },
            "p2": { "hand": [VANILLA], "field": field, "library": [VANILLA] }
        }))
    }

    struct Transformed {
        instance_id: String,
        from_def_id: String,
        to_def_id: String,
        new_instance_id: String,
    }

    fn transformed(s: &Scenario) -> Vec<Transformed> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Transformed { instance_id, from_def_id, to_def_id, new_instance_id, .. } => {
                    Some(Transformed {
                        instance_id: instance_id.clone(),
                        from_def_id: from_def_id.clone(),
                        to_def_id: to_def_id.clone(),
                        new_instance_id: new_instance_id.clone(),
                    })
                }
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

    fn keyword_kinds(keywords: &[Keyword]) -> Vec<Value> {
        keywords.iter().map(|keyword| js(keyword)["kind"].clone()).collect()
    }

    /// The first seed whose Golem (`radiant_face` its face), attacking a Vanilla, survives the combat
    /// and destroys the new Unit (R42) or fights it and leaves it on the field.
    fn seed_where_the_golem_survives_and(kills: bool, radiant_face: bool) -> String {
        seed_where("golem", |seed| {
            let mut s = golem(Some(json!(VANILLA)), radiant_face, seed);
            let it = s.card(GOLEM).id.clone();
            attack_with(&mut s);
            let Some(made) = transformed(&s).first().map(|event| event.new_instance_id.clone()) else {
                return false;
            };
            let killed = s.events().iter().any(|event| {
                matches!(event, GameEvent::Destroyed { instance_id, killer_id: Some(killer), .. }
                    if *instance_id == made && *killer == it)
            });
            let fought = s.events().iter().any(|event| {
                matches!(event, GameEvent::Damage { target_id, .. } if *target_id == made)
            });
            let stands = s.unit(P2, 1).is_some_and(|unit| unit.id == made);
            let alive = s.unit(P1, 1).is_some_and(|unit| unit.id == it);
            alive && if kills { killed } else { fought && stands && !killed }
        })
    }

    fn may_attack(s: &Scenario, id: &str) -> bool {
        legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .any(|action| action["type"] == json!("attack") && action["attackerId"] == json!(id))
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
        assert!(base.when_attacks.is_some() && base.after_attack.is_some());
        assert!(radiant.when_attacks.is_some() && radiant.after_attack.is_some());
        assert!(!Arc::ptr_eq(base.when_attacks.as_ref().unwrap(), radiant.when_attacks.as_ref().unwrap()));
    }

    mod base {
        use super::*;

        #[test]
        fn r2285002_r380_r35_attacking_a_unit_transforms_that_unit_before_any_damage_into_a_random_radiant_classic_or_classic_unit() {
            crate::register_all();
            for seed in 0..12 {
                let name = format!("golem-{seed}");
                let mut s = golem(Some(json!(VANILLA)), false, &name);
                let defender = s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default();
                let it = s.card(GOLEM).id.clone();
                attack_with(&mut s);
                let events = transformed(&s);
                let event = events.first().expect("the attacked Unit is transformed");
                assert_eq!((event.instance_id.as_str(), event.from_def_id.as_str()), (defender.as_str(), VANILLA));
                assert!(events.iter().all(|event| event.instance_id != it), "{seed}: the Golem stays itself");
                let made = crate::card_def(&event.to_def_id);
                assert_eq!(made.type_, CardType::Unit);
                assert!(!made.token);
                assert!(js(&made.set) == json!("Classic") || js(&made.set) == json!("Classic+"), "{seed}");
                // The transform comes before the combat's first hit, and the Golem stays itself.
                let order = types_of(&s);
                let at = order.iter().position(|kind| *kind == "transformed");
                let hit = order.iter().position(|kind| *kind == "damage" || *kind == "divineShieldLost");
                assert!(hit.is_none_or(|hit| at.is_some_and(|at| at < hit)), "{seed}: {order:?}");
                // The new Unit is p2's, on its Radiant face, while it stands.
                if let Some(unit) = s.unit(P2, 1).filter(|unit| unit.id == event.new_instance_id) {
                    assert!(unit.radiant, "{seed}");
                }
            }
        }

        #[test]
        fn r2285002_the_attack_goes_on_against_the_new_unit_which_the_golem_hits() {
            crate::register_all();
            let seed = seed_where_the_golem_survives_and(false, false);
            let mut s = golem(Some(json!(VANILLA)), false, &seed);
            attack_with(&mut s);
            let made = transformed(&s)[0].new_instance_id.clone();
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Damage { target_id, .. } if *target_id == made
            )));
        }

        #[test]
        fn r1_the_new_unit_fires_no_cry_it_is_transformed_never_summoned_or_played() {
            crate::register_all();
            let mut s = golem(Some(json!(VANILLA)), false, "golem-cry");
            attack_with(&mut s);
            let order = types_of(&s);
            let from = order.iter().position(|kind| *kind == "transformed").expect("a transform");
            assert!(!order[from..].iter().any(|kind| *kind == "summoned" || *kind == "cardPlayed"));
        }

        #[test]
        fn r2285001_r42_destroying_the_new_unit_lets_the_surviving_golem_attack_again() {
            crate::register_all();
            let seed = seed_where_the_golem_survives_and(true, false);
            let mut s = golem(Some(json!(VANILLA)), false, &seed);
            attack_with(&mut s);
            let it = s.card(GOLEM).id.clone();
            assert_eq!(js(&s.card(GOLEM).exertion), json!({ "attacked": false, "switched": false }));
            assert!(may_attack(&s, &it));
            s.attack(GOLEM, "hero");
            assert_eq!(types_of(&s).iter().filter(|kind| **kind == "attackDeclared").count(), 2);
        }

        #[test]
        fn r2657004_played_this_turn_its_second_attack_after_a_kill_may_hit_a_unit_never_the_hero() {
            crate::register_all();
            let played = |seed: &str| {
                let mut s = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [GOLEM, VANILLA], "library": [VANILLA], "mana": 4 },
                    "p2": { "hand": [VANILLA], "field": [VANILLA, VANILLA], "library": [VANILLA] }
                }));
                s.play(GOLEM, json!({ "zone": 1 }));
                attack_with(&mut s);
                s
            };
            let seed = seed_where("golem-played", |seed| {
                let s = played(seed);
                s.unit(P1, 1).is_some_and(|unit| unit.def_id == GOLEM)
                    && s.unit(P2, 1).is_none()
                    && s.unit(P2, 2).is_some()
            });
            let mut s = played(&seed);
            let it = s.card(GOLEM).id.clone();
            assert_eq!(s.card(GOLEM).summoned_turn, Some(s.state().turn), "still summoning sick");
            let second = s.unit(P2, 2).expect("the other Vanilla").id;
            let offered = |s: &Scenario, target: &str| {
                legal_actions(s.state(), P1).iter().map(js).any(|action| {
                    action["type"] == json!("attack")
                        && action["attackerId"] == json!(it)
                        && action["targetId"] == json!(target)
                })
            };
            assert!(offered(&s, &second), "Rush lets it attack a Unit");
            assert!(!offered(&s, "hero-p2"), "never the hero on the turn it is played");
            s.expect_refused(|s| s.attack(GOLEM, "hero"));
        }

        #[test]
        fn r2285001_a_new_unit_that_survives_leaves_the_golem_spent() {
            crate::register_all();
            let seed = seed_where_the_golem_survives_and(false, false);
            let mut s = golem(Some(json!(VANILLA)), false, &seed);
            attack_with(&mut s);
            let it = s.card(GOLEM).id.clone();
            assert!(!may_attack(&s, &it));
        }

        #[test]
        fn r1869002_the_fuzz_monitor_agrees_the_golem_s_second_attack_is_a_fresh_exertion() {
            crate::register_all();
            let seed = seed_where_the_golem_survives_and(true, false);
            let mut s = golem(Some(json!(VANILLA)), false, &seed);
            let mut monitor = create_invariant_monitor(s.state());
            let from = s.events().len();
            attack_with(&mut s);
            assert_eq!(monitor.after(&s.events()[from..], s.state()), Vec::<String>::new());
            let it = s.card(GOLEM).id.clone();
            let action: ActionBody = json_as(json!({ "type": "attack", "attackerId": it, "targetId": "hero-p2" }));
            let again = monitor.before(s.state(), P1, &action);
            // I3 names the cards the scenario placed without an entry event; nothing else may be found.
            let others: Vec<String> = again.into_iter().filter(|finding| !finding.starts_with("I3")).collect();
            assert_eq!(others, Vec::<String>::new());
        }

        #[test]
        fn r2285002_attacking_the_hero_transforms_nothing() {
            crate::register_all();
            let mut s = golem(None, false, "golem-hero");
            attack_with(&mut s);
            s.expect_health(P2, 20);
            assert!(transformed(&s).is_empty());
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
        }

        #[test]
        fn r53_r2285000_a_forced_attack_is_an_attack_too_the_unit_it_is_forced_to_attack_is_transformed() {
            crate::register_all();
            // Core #9 Moths to the Flame (1/14): "Start of turn: Every enemy Unit attacks this."
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA], "field": [GOLEM], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "field": ["core-009"], "library": [VANILLA] }
            }));
            s.end_turn();
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. })));
            let events = transformed(&s);
            assert_eq!(events.first().map(|event| event.from_def_id.as_str()), Some("core-009"));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
        }

        #[test]
        fn r23_r129_an_immutable_target_is_not_transformed_and_takes_no_draw() {
            crate::register_all();
            let mut s = golem(Some(json!(VANILLA)), false, "golem-immutable");
            let defender = s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default();
            find_instance_mut(s.state_mut(), &defender)
                .expect("the Vanilla is on the field")
                .granted_keywords
                .push(json_as(json!({ "kind": "Immutable" })));
            let cursor = s.state().rng_cursor;
            attack_with(&mut s);
            assert!(transformed(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
            // The Golem's First Strike kills the 4/4 it attacked, so it may attack again.
            s.expect_in_zone(defender.as_str(), "graveyard");
            let it = s.card(GOLEM).id.clone();
            assert!(may_attack(&s, &it));
        }

        #[test]
        fn r129_a_fixed_seed_makes_the_same_unit_the_draw_is_one_rng_pick() {
            crate::register_all();
            let mut one = golem(Some(json!(VANILLA)), false, "golem-fixed");
            attack_with(&mut one);
            let mut two = golem(Some(json!(VANILLA)), false, "golem-fixed");
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
        fn r2285002_the_radiant_golem_transforms_the_unit_it_attacks_into_a_base_face_classic_or_classic_unit() {
            crate::register_all();
            for seed in 0..12 {
                let name = format!("golem-radiant-{seed}");
                let mut s = golem(Some(json!(VANILLA)), true, &name);
                attack_with(&mut s);
                let events = transformed(&s);
                let event = events.first().expect("the attacked Unit is transformed");
                let made = crate::card_def(&event.to_def_id);
                assert!(js(&made.set) == json!("Classic") || js(&made.set) == json!("Classic+"), "{seed}");
                if let Some(unit) = s.unit(P2, 1).filter(|unit| unit.id == event.new_instance_id) {
                    assert!(!unit.radiant, "{seed}");
                }
            }
        }

        #[test]
        fn r2285001_destroying_the_new_unit_lets_it_attack_again() {
            crate::register_all();
            let seed = seed_where_the_golem_survives_and(true, true);
            let mut s = golem(Some(json!(VANILLA)), true, &seed);
            assert!(keyword_kinds(&s.stats(GOLEM).keywords).contains(&json!("Divine Shield")));
            attack_with(&mut s);
            let it = s.card(GOLEM).id.clone();
            assert!(may_attack(&s, &it));
        }

        #[test]
        fn r2285002_attacking_the_hero_transforms_nothing() {
            crate::register_all();
            let mut s = golem(None, true, "golem-radiant-hero");
            attack_with(&mut s);
            s.expect_health(P2, 10);
            assert!(transformed(&s).is_empty());
        }
    }
}
