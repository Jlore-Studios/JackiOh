//! C #83 Flame Lance (SPEC §8.6 row 83). (3) Spell, Common.
//!   Base:    "Trample\nDeal {damage} damage to a Unit." — damage 10
//!   Radiant: "Trample\nDeal {damage} damage to a Unit." — damage 20
//!   Engine:  "Trample on a Spell (§4.4): the excess over the target Unit's health hits that Unit's
//!            controller's hero as a new instance, as R346 put Pierce on a Spell. Tunes: damage 10 ↑
//!            (step 2)."
//!
//! "a Unit": a declared target (R81), a Unit on either side, never a hero (R90). One §4.4 hit carrying
//! Trample (B5 E6): the Spell prints it, and the effect states it too (`trample`), so the hit tramples
//! however it is read. Step 2's Armor lowers the hit first; then the amount beyond the target's health
//! before the hit goes to its controller's hero as a new instance (§4.4 step 9, R63). A Divine Shield
//! (step 1) or an Indestructible target (step 4) stops the whole hit, and nothing tramples. The amount
//! is the declared `damage` (R386), 10 or 20, read through `param`.

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-083";

/// "a Unit": units only, either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
                "trample": true
            })))]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 22 is its declared `damage`, which `param` reads off the face.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #83 Flame Lance (SPEC §8.6 row 83; BUILD M9 row C 83). (3) Spell, Common: Trample; deal {damage}
// damage to a Unit — damage 10, Radiant 20, step 2.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LANCE: &str = "classic-083";
    const VANILLA: &str = "core-008"; // 4/4
    const JILLIAX: &str = "core-056"; // 3/2 Rush, Taunt, Lifesteal, Divine Shield
    const THE_ROCK: &str = "core-066"; // 10/10 Indestructible
    const STOCKPILE: &str = "core-005";

    use crate::scenario;

    use crate::js;

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": spare_library() })
    }

    fn spare_library() -> Value {
        json!([VANILLA, VANILLA])
    }

    fn with_spare(side: Value) -> Value {
        let mut merged = side;
        if let (Some(into), Value::Object(extra)) = (merged.as_object_mut(), spare()) {
            for (key, value) in extra {
                into.insert(key, value);
            }
        }
        merged
    }

    fn lance_at<'a>(s: &'a mut Scenario, instance_id: &str) -> &'a mut Scenario {
        s.play(LANCE, json!({ "targets": [{ "pick": "instance", "instanceId": instance_id }] }))
    }

    fn hits_of(s: &Scenario) -> Vec<(String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    fn amounts(s: &Scenario) -> Vec<i32> {
        hits_of(s).into_iter().map(|(_, amount)| amount).collect()
    }

    mod base {
        use super::*;

        /// R90 a Unit target only: no hero is offered, and naming one is refused
        #[test]
        fn r90_a_unit_target_only_no_hero_is_offered_and_naming_one_is_refused() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [VANILLA] }))
            }));
            let lance = s.card(LANCE).clone();
            let plays: Vec<Value> = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(lance.id))
                .collect();
            assert!(!plays.is_empty());
            for play in &plays {
                let all_instances = play["targets"]
                    .as_array()
                    .map(|targets| targets.iter().all(|target| target["pick"] == json!("instance")));
                assert_eq!(all_instances, Some(true));
            }
            s.expect_refused(|s| s.play(LANCE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] })));
        }

        /// §4.4 step 9 Trample: 10 into a 4/4, the 6 beyond its health a new instance on its controller's hero
        #[test]
        fn s4_4_step_9_trample_10_into_a_4_4_the_6_beyond_its_health_a_new_instance_on_its_controller_s_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [VANILLA] }))
            }));
            let foe = must(s.unit(P2, 1), "p2's Vanilla");
            lance_at(&mut s, &foe.id);
            assert_eq!(hits_of(&s), vec![(foe.id.clone(), 4), ("hero-p2".to_string(), 6)]);
            s.expect_in_zone(&foe, "graveyard").expect_health(P2, 24);
        }

        /// R63 the target's health before the hit is its current health: 10 into a 4/4 with 3 damage tramples 9
        #[test]
        fn r63_the_target_s_health_before_the_hit_is_its_current_health_10_into_a_4_4_with_3_damage_tramples_9() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [{ "def": VANILLA, "damage": 3 }] }))
            }));
            let foe = must(s.unit(P2, 1), "p2's Vanilla");
            lance_at(&mut s, &foe.id);
            assert_eq!(hits_of(&s), vec![(foe.id.clone(), 1), ("hero-p2".to_string(), 9)]);
        }

        /// R90 with no Unit on the field it is still played, `legalActions` agreeing, and fizzles: no hit, no hero damage
        #[test]
        fn r90_with_no_unit_on_the_field_it_is_still_played_legalactions_agreeing_and_fizzles_no_hit_no_hero_damage() {
            let mut s = scenario(json!({ "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() }, "p2": spare() }));
            let lance = s.card(LANCE).clone();
            assert!(
                legal_actions(s.state(), P1)
                    .iter()
                    .map(js)
                    .any(|action| action["type"] == json!("play") && action["instanceId"] == json!(lance.id))
            );
            s.play(LANCE, json!({}));
            assert!(hits_of(&s).is_empty());
            s.expect_in_zone(&lance, "graveyard").expect_health(P1, 30).expect_health(P2, 30);
        }

        /// §8.6 either side: on your own Unit the excess hits your own hero
        #[test]
        fn s8_6_either_side_on_your_own_unit_the_excess_hits_your_own_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "field": [VANILLA], "library": spare_library() },
                "p2": spare()
            }));
            let own = must(s.unit(P1, 1), "p1's Vanilla");
            lance_at(&mut s, &own.id);
            s.expect_health(P1, 24);
        }

        /// §4.4 step 2 Armor lowers the hit before the excess is taken
        #[test]
        fn s4_4_step_2_armor_lowers_the_hit_before_the_excess_is_taken() {
            // A Unit in Defense Position has Armor 1: 10 − 1 = 9, 4 to it and 5 beyond.
            let mut s = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [{ "def": VANILLA, "position": "DEF" }] }))
            }));
            let foe = must(s.unit(P2, 1), "p2's Vanilla");
            lance_at(&mut s, &foe.id);
            assert_eq!(hits_of(&s), vec![(foe.id.clone(), 4), ("hero-p2".to_string(), 5)]);
        }

        /// §4.4 steps 1 and 4: a Divine Shield or an Indestructible target stops the hit, and nothing tramples
        #[test]
        fn s4_4_steps_1_and_4_a_divine_shield_or_an_indestructible_target_stops_the_hit_and_nothing_tramples() {
            let mut shield = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [JILLIAX] }))
            }));
            let jilliax = must(shield.unit(P2, 1), "p2's Jilliax");
            lance_at(&mut shield, &jilliax.id);
            assert!(hits_of(&shield).is_empty());
            assert!(shield.last_events().iter().any(|event| matches!(event, GameEvent::DivineShieldLost { .. })));
            shield.expect_health(P2, 30);

            let mut rock = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [THE_ROCK] }))
            }));
            let the_rock = must(rock.unit(P2, 1), "p2's Rock");
            lance_at(&mut rock, &the_rock.id);
            assert!(hits_of(&rock).is_empty());
            rock.expect_health(P2, 30);
        }

        /// R386 its tuned number: an Upgrade's step of 2 makes it 12, 8 beyond a 4/4
        #[test]
        fn r386_its_tuned_number_an_upgrade_s_step_of_2_makes_it_12_8_beyond_a_4_4() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANCE, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [VANILLA] }))
            }));
            step_param(s.card_mut(LANCE), "damage", 1);
            let foe = must(s.unit(P2, 1), "p2's Vanilla");
            lance_at(&mut s, &foe.id);
            assert_eq!(amounts(&s), vec![4, 8]);
        }
    }

    mod radiant {
        use super::*;

        /// §8.6 20 damage: 16 beyond a 4/4
        #[test]
        fn s8_6_20_damage_16_beyond_a_4_4() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": LANCE, "radiant": true }, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [VANILLA] }))
            }));
            let foe = must(s.unit(P2, 1), "p2's Vanilla");
            lance_at(&mut s, &foe.id);
            assert_eq!(amounts(&s), vec![4, 16]);
            s.expect_health(P2, 14);
        }

        /// R386 a Degrade's step of 2 makes the Radiant 18
        #[test]
        fn r386_a_degrade_s_step_of_2_makes_the_radiant_18() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": LANCE, "radiant": true }, STOCKPILE], "library": spare_library() },
                "p2": with_spare(json!({ "field": [VANILLA] }))
            }));
            step_param(s.card_mut(LANCE), "damage", -1);
            let foe = must(s.unit(P2, 1), "p2's Vanilla");
            lance_at(&mut s, &foe.id);
            assert_eq!(amounts(&s), vec![4, 14]);
        }
    }
}
