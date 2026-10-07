//! Port of `packages/engine/test/damage-pipeline.test.ts`.
//!
//! B5 E6's additions to the damage pipeline (SPEC §4.4; docs/classic-sets.md B5 E6; R463): Spell
//! Damage at step 0, the hero's divisors after Armor and the lowest hit cap at step 3, Trample on a
//! Spell — and that every projection of a hit (R44's lethal projection, the Zephyrs scorer) reads the
//! same steps as the hit itself.

use jackioh_engine::effects::damage;
use jackioh_engine::subsystems::lethal::projected_hero_damage;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::damage_combat::{
    anime_armor, argus, bolt, grunt, hidden_argus, hidden_lens, lance, lens, playing, recorder, replays_to,
    solar, sweep, wall,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};
use crate::rules::fixtures::scripts::anti_oneshot;

fn spell_in(state: &mut GameState, def_id: &str) -> CardInstance {
    in_hand(state, def_id, P1, 1).first().cloned().expect("no card")
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn damage_hits(events: &[GameEvent]) -> Vec<(String, i32)> {
    events_of_type(events, GameEventType::Damage)
        .iter()
        .map(|hit| match hit {
            GameEvent::Damage {
                target_id, amount, ..
            } => (target_id.clone(), *amount),
            other => panic!("not a damage event: {other:?}"),
        })
        .collect()
}

fn hero(player: PlayerId) -> DamageTarget {
    DamageTarget::Hero { player }
}

/// `describe("E6 Spell Damage (§4.4 step 0)")`.
mod e6_spell_damage_s4_4_step_0 {
    use super::*;

    #[test]
    fn a_spell_s_hit_is_raised_by_the_sum_of_the_spell_damage_on_its_controller_s_units_once_per_hit() {
        let mut state = playing("dc-spell-damage");
        put(&mut state, &solar.id, slot(P1, Row::Units, 1), json!({}));
        put(
            &mut state,
            &solar.id,
            slot(P1, Row::Units, 2),
            json!({ "radiant": true }),
        );
        // The opponent's Spell Damage is theirs; a Field Spell and a face-down Trap printing it are no units.
        put(&mut state, &solar.id, slot(P2, Row::Units, 1), json!({}));
        put(&mut state, &lens.id, slot(P1, Row::Backrow, 1), json!({}));
        put(&mut state, &hidden_lens.id, slot(P1, Row::Backrow, 2), json!({}));
        assert_eq!(spell_damage_of(&state, P1), 2 + 5);
        let target = put(&mut state, &wall.id, slot(P2, Row::Units, 2), json!({}));
        find_instance_mut(&mut state, &target.id)
            .expect("on the field")
            .buffs = AttackHealth {
            attack: 0,
            health: 20,
        };
        let spell = spell_in(&mut state, &bolt.id);
        let mut game = recorder(&state);

        let result = game.play(json!({
            "type": "play",
            "instanceId": spell.id,
            "targets": [{ "pick": "instance", "instanceId": target.id }],
            "playerId": "p1",
        }));
        assert_eq!(
            to_json(&events_of_type(&result.events, GameEventType::Damage)),
            json!([{ "type": "damage", "sourceId": spell.id, "targetId": target.id, "amount": 3 + 7, "combat": false }])
        );
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn not_a_unit_s_a_field_spell_s_or_a_trap_s_hit_and_a_hit_of_0_is_nothing_to_raise() {
        let mut state = playing("dc-spell-damage-other");
        let mage = put(&mut state, &solar.id, slot(P1, Row::Units, 1), json!({}));
        let field = put(&mut state, &sweep.id, slot(P1, Row::Backrow, 1), json!({}));
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        apply_effects(
            &[damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
            ))],
            &mut make_context(&mut sink, Some(&mage), HookOptions::default()),
        );
        apply_effects(
            &[damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
            ))],
            &mut make_context(&mut sink, Some(&field), HookOptions::default()),
        );
        assert_eq!(sink.state.players[P2].hero.health, 26);
        let spell = spell_in(sink.state, &bolt.id);
        assert_eq!(
            deal_damage(
                &mut sink,
                DamageArgs {
                    source: Some(spell.clone()),
                    target: hero(P2),
                    amount: 0,
                    flags: None,
                },
            ),
            0
        );
        assert_eq!(sink.state.players[P2].hero.health, 26);
        // A Spell that is resolving, or in a hand, is the Spell all the same.
        assert_eq!(
            deal_damage(
                &mut sink,
                DamageArgs {
                    source: Some(spell.clone()),
                    target: hero(P2),
                    amount: 1,
                    flags: None,
                },
            ),
            3
        );
    }
}

/// `describe("E6 Trample on a Spell")`.
mod e6_trample_on_a_spell {
    use super::*;

    #[test]
    fn the_excess_over_the_target_unit_s_health_hits_its_controller_s_hero_as_a_new_instance_raised_once() {
        let mut state = playing("dc-lance");
        put(&mut state, &solar.id, slot(P1, Row::Units, 1), json!({}));
        let target = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), json!({}));
        state.players[P2].hero.armor = 3;
        let spell = spell_in(&mut state, &lance.id);
        let mut game = recorder(&state);
        let result = game.play(json!({
            "type": "play",
            "instanceId": spell.id,
            "targets": [{ "pick": "instance", "instanceId": target.id }],
            "playerId": "p1",
        }));
        // 11 + 2 Spell Damage = 13: 2 on the 2/2, 11 on to the hero, less its Armor 3.
        assert_eq!(
            damage_hits(&result.events),
            vec![(target.id.clone(), 2), ("hero-p2".to_string(), 8)]
        );
        assert_eq!(game.state().players[P2].hero.health, 22);
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn a_printed_trample_is_read_off_the_spell_and_an_effect_may_state_it_where_no_source_is_left() {
        let mut state = playing("dc-lance-stated");
        let target = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), json!({}));
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let source = spell_in(sink.state, &lance.id);
        deal_damage(
            &mut sink,
            DamageArgs {
                source: Some(source),
                target: DamageTarget::Unit { instance: target },
                amount: 5,
                flags: None,
            },
        );
        assert_eq!(sink.state.players[P2].hero.health, 27);
        let other = put(sink.state, &grunt.id, slot(P2, Row::Units, 2), json!({}));
        deal_damage(
            &mut sink,
            DamageArgs {
                source: None,
                target: DamageTarget::Unit { instance: other },
                amount: 6,
                flags: Some(json_as(json!({ "trample": true }))),
            },
        );
        assert_eq!(sink.state.players[P2].hero.health, 23);
        // Without it, nothing tramples.
        let third = put(sink.state, &grunt.id, slot(P2, Row::Units, 3), json!({}));
        deal_damage(
            &mut sink,
            DamageArgs {
                source: None,
                target: DamageTarget::Unit { instance: third },
                amount: 6,
                flags: None,
            },
        );
        assert_eq!(sink.state.players[P2].hero.health, 23);
    }
}

/// `describe("E6 hero divisors and caps (§4.4 steps 2 and 3)")`.
mod e6_hero_divisors_and_caps_s4_4_steps_2_and_3 {
    use super::*;

    #[test]
    fn r463_divides_after_armor_rounded_up_once_several_multiply_then_the_lowest_cap() {
        let mut state = playing("dc-divisors");
        put(&mut state, &argus.id, slot(P2, Row::Backrow, 1), json!({}));
        assert_eq!(hero_damage_divisor(&state, P2), 2);
        assert_eq!(hero_hit_amount(&state, P2, 5, false), 3);
        state.players[P2].hero.armor = 1;
        assert_eq!(hero_hit_amount(&state, P2, 5, false), 2);
        // Pierce skips the Armor and nothing else.
        assert_eq!(hero_hit_amount(&state, P2, 5, true), 3);
        state.players[P2].hero.armor = 0;
        // Radiant (4) beside base (2): 8, and 5 / 8 rounds up to 1 — once, not at each divisor.
        put(
            &mut state,
            &argus.id,
            slot(P2, Row::Backrow, 2),
            json!({ "radiant": true }),
        );
        assert_eq!(hero_damage_divisor(&state, P2), 8);
        assert_eq!(hero_hit_amount(&state, P2, 5, false), 1);
        assert_eq!(hero_hit_amount(&state, P2, 17, false), 3);
        // Caps: Anti-oneshot's 5 and Anime Armor's 1 — the lowest wins.
        let mut capped = playing("dc-caps");
        put(
            &mut capped,
            &anti_oneshot().id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );
        assert_eq!(hero_damage_cap(&capped, P2), Some(5));
        put(&mut capped, &anime_armor.id, slot(P2, Row::Units, 1), json!({}));
        assert_eq!(hero_damage_cap(&capped, P2), Some(1));
        assert_eq!(hero_hit_amount(&capped, P2, 30, false), 1);
        // A divisor comes before the cap: 30 / 2 = 15, capped at 1 either way; with only Anti-oneshot, 5.
        let mut both = playing("dc-caps-order");
        put(&mut both, &argus.id, slot(P2, Row::Backrow, 1), json!({}));
        put(
            &mut both,
            &anti_oneshot().id,
            slot(P2, Row::Backrow, 2),
            json!({}),
        );
        assert_eq!(hero_hit_amount(&both, P2, 8, false), 4);
        assert_eq!(hero_hit_amount(&both, P2, 30, false), 5);
    }

    #[test]
    fn r463_a_face_down_trap_guards_no_hero_until_it_fires() {
        let mut state = playing("dc-hidden-guard");
        let trap = put(&mut state, &hidden_argus.id, slot(P2, Row::Backrow, 1), json!({}));
        assert_eq!(hero_hit_amount(&state, P2, 6, false), 6);
        find_instance_mut(&mut state, &trap.id)
            .expect("in the backrow")
            .face_up = Some(true);
        assert_eq!(hero_hit_amount(&state, P2, 6, false), 3);
    }

    #[test]
    fn applies_to_every_hit_on_the_hero_fatigue_included_and_not_to_a_unit_lose_health_is_not_damage() {
        let mut state = playing("dc-divisor-hits");
        put(&mut state, &argus.id, slot(P1, Row::Backrow, 1), json!({}));
        let body = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
        find_instance_mut(&mut state, &body.id)
            .expect("on the field")
            .buffs = AttackHealth {
            attack: 0,
            health: 10,
        };
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        deal_damage(
            &mut sink,
            DamageArgs {
                source: None,
                target: hero(P1),
                amount: 7,
                flags: None,
            },
        );
        assert_eq!(sink.state.players[P1].hero.health, 26);
        let body_now = find_instance(sink.state, &body.id)
            .cloned()
            .expect("on the field");
        deal_damage(
            &mut sink,
            DamageArgs {
                source: None,
                target: DamageTarget::Unit { instance: body_now },
                amount: 7,
                flags: None,
            },
        );
        assert_eq!(
            find_instance(sink.state, &body.id).expect("on the field").damage,
            7
        );
        sink.state.players[P1].library = vec![];
        sink.state.players[P1].fatigue_count = 2;
        draw_one(&mut sink, P1, None);
        // The 3rd fatigue draw deals 3, halved and rounded up to 2 (R125).
        assert_eq!(sink.state.players[P1].hero.health, 24);
        // R18: losing health is no hit, so nothing divides it.
        lose_health(&mut sink, P1, 5);
        assert_eq!(sink.state.players[P1].hero.health, 19);
    }

    #[test]
    fn r44_the_lethal_projection_reads_the_same_steps_as_the_hit() {
        let mut state = playing("dc-projection");
        put(&mut state, &argus.id, slot(P2, Row::Backrow, 1), json!({}));
        put(&mut state, &anime_armor.id, slot(P2, Row::Units, 1), json!({}));
        for amount in [1, 2, 5, 9] {
            assert_eq!(
                projected_hero_damage(&state, P2, amount, false),
                hero_hit_amount(&state, P2, amount, false)
            );
        }
        let mut unguarded = playing("dc-projection-plain");
        put(&mut unguarded, &argus.id, slot(P2, Row::Backrow, 1), json!({}));
        assert_eq!(projected_hero_damage(&unguarded, P2, 9, false), 5);
    }
}
