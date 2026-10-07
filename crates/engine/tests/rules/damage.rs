//! Port of `packages/engine/test/damage.test.ts`.
//!
//! One test per step of the damage pipeline (SPEC §4.4), in order, plus §6.3 healing and R18.

use jackioh_engine::effects::plague;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::{
    armoured, big_body, big_dfender, cleaver, indestructible, lifestealer, plain, poisonous, shielded,
    trample_lifesteal, trampler,
};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};
use crate::rules::fixtures::scripts::anti_oneshot;

/// TS `newGame()`: the harness's default seed.
fn game() -> GameState {
    new_game("engine-test", None)
}

/// Where a hit lands: TS's `onUnit(instance)` and `onHero(player)`. A unit is named by id, so the hit
/// is dealt to the card as the state holds it at that moment (TS hands over the live object).
enum Aim<'a> {
    Unit(&'a str),
    Hero(PlayerId),
}

fn on_unit(id: &str) -> Aim<'_> {
    Aim::Unit(id)
}

fn on_hero(player: PlayerId) -> Aim<'static> {
    Aim::Hero(player)
}

/// The instance as the state holds it now (TS reads its live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("{id} is in no zone"))
}

/// One damage instance, returning the amount actually dealt.
fn hit(sink: &mut EngineSink, source: Option<&str>, target: Aim<'_>, amount: i32, flags: Option<Value>) -> i32 {
    let source = source.map(|id| live(sink.state, id));
    let target = match target {
        Aim::Unit(id) => DamageTarget::Unit {
            instance: live(sink.state, id),
        },
        Aim::Hero(player) => DamageTarget::Hero { player },
    };
    deal_damage(
        sink,
        DamageArgs {
            source,
            target,
            amount,
            flags: flags.map(json_as),
        },
    )
}

/// `put(state, defId, ref, { radiant })` (fixtures/harness.ts), face and all: a hand instance made
/// Radiant before it is placed, as the TS helper does.
fn put_face(state: &mut GameState, def_id: &str, at: ZoneSlot, radiant: bool) -> CardInstance {
    let (player, row, lane) = (at.player, at.row, at.lane);
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    if radiant {
        card.radiant = true;
    }
    let id = card.id.clone();
    if !place_on_field(state, card, at, Default::default()) {
        panic!("could not place {def_id} in {row} {lane}");
    }
    find_instance(state, &id).cloned().expect("placed card")
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn amounts(events: &[GameEvent], kind: GameEventType) -> Vec<i32> {
    events_of_type(events, kind)
        .iter()
        .map(|event| match event {
            GameEvent::Damage { amount, .. } | GameEvent::Healed { amount, .. } => *amount,
            other => panic!("no amount on {other:?}"),
        })
        .collect()
}

/// A fresh sink over `state`, its rng at the state's cursor as reduce builds it (TS `sinkFor`).
fn with_sink<R>(state: &mut GameState, run: impl FnOnce(&mut EngineSink) -> R) -> R {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    run(&mut sink)
}

/// `describe("the damage pipeline (§4.4, M2-T3)")`.
mod the_damage_pipeline_s4_4_m2_t3 {
    use super::*;

    #[test]
    fn step_1_divine_shield_negates_a_10_hit_fully_and_is_gone_and_the_next_hit_lands() {
        let mut state = game();
        let target = put(&mut state, &shielded().id, slot(P2, Row::Units, 1));
        with_sink(&mut state, |sink| {
            assert_eq!(hit(sink, None, on_unit(&target.id), 10, None), 0);
            let now = live(sink.state, &target.id);
            assert_eq!(now.damage, 0);
            assert_eq!(now.divine_shield_spent, Some(true));
            assert!(!unit_has(sink.state, &now, KeywordKind::DivineShield));
            let lost: Vec<String> = events_of_type(sink.events, GameEventType::DivineShieldLost)
                .iter()
                .map(|event| match event {
                    GameEvent::DivineShieldLost { instance_id } => instance_id.clone(),
                    other => panic!("not a divineShieldLost event: {other:?}"),
                })
                .collect();
            assert_eq!(lost, vec![target.id.clone()]);
            assert_eq!(events_of_type(sink.events, GameEventType::Damage).len(), 0);

            // The second hit is a normal instance, and step 5 does not cap it at the target's 2 health.
            assert_eq!(hit(sink, None, on_unit(&target.id), 10, None), 10);
            let now = live(sink.state, &target.id);
            assert_eq!(now.damage, 10);
            assert_eq!(unit_view(sink.state, &now).health, -8);
            assert_eq!(amounts(sink.events, GameEventType::Damage), vec![10]);
        });
    }

    #[test]
    fn step_2_armor_7_turns_a_7_into_0_defense_adds_1_big_d_fender_adds_2_more_true_strike_ignores_it() {
        let mut state = game();
        let target = put(&mut state, &armoured().id, slot(P2, Row::Units, 1));
        with_sink(&mut state, |sink| {
            assert_eq!(unit_view(sink.state, &live(sink.state, &target.id)).armor, 7);
            assert_eq!(hit(sink, None, on_unit(&target.id), 7, None), 0);
            assert_eq!(live(sink.state, &target.id).damage, 0);
            assert_eq!(hit(sink, None, on_unit(&target.id), 8, None), 1);

            // §4.1: Defense Position is Armor +1 on top of the printed value.
            find_instance_mut(sink.state, &target.id).expect("on the field").position = Some(Position::Def);
            assert_eq!(unit_view(sink.state, &live(sink.state, &target.id)).armor, 8);
            assert_eq!(hit(sink, None, on_unit(&target.id), 8, None), 0);

            // Big D-fender's aura: its controller's Defense units get Armor +2 (§10.4 layer 5).
            put(sink.state, &big_dfender().id, slot(P2, Row::Units, 2));
            assert_eq!(unit_view(sink.state, &live(sink.state, &target.id)).armor, 10);
            assert_eq!(hit(sink, None, on_unit(&target.id), 10, None), 0);
            assert_eq!(hit(sink, None, on_unit(&target.id), 12, None), 2);

            // True Strike skips step 2 entirely: printed, Defense and aura Armor all.
            assert_eq!(
                hit(sink, None, on_unit(&target.id), 12, Some(json!({ "ignoreArmor": true }))),
                12
            );
            assert_eq!(amounts(sink.events, GameEventType::Damage), vec![1, 2, 12]);
        });
    }

    #[test]
    fn r346_step_2_a_source_with_pierce_skips_armor_on_a_unit_and_a_hero_and_nothing_else() {
        let mut state = game();
        let target = put(&mut state, &armoured().id, slot(P2, Row::Units, 1)); // Armor 7
        let shielded_target = put(&mut state, &shielded().id, slot(P2, Row::Units, 2));
        let wall = put(&mut state, &indestructible().id, slot(P2, Row::Units, 3));
        let source = put(&mut state, &plain().id, slot(P1, Row::Units, 1));
        find_instance_mut(&mut state, &source.id).expect("on the field").granted_keywords.push(Keyword::Pierce);
        state.players[P2].hero.armor = 5;
        put(&mut state, &anti_oneshot().id, slot(P2, Row::Backrow, 1));
        with_sink(&mut state, |sink| {
            // Printed, Defense and aura Armor all: the whole 7 lands.
            find_instance_mut(sink.state, &target.id).expect("on the field").position = Some(Position::Def);
            assert_eq!(unit_view(sink.state, &live(sink.state, &target.id)).armor, 8);
            assert_eq!(hit(sink, Some(&source.id), on_unit(&target.id), 7, None), 7);
            // The hero's Armor 5 is skipped too, and step 3's cap still clamps the 9 to 5.
            assert_eq!(hit(sink, Some(&source.id), on_hero(P2), 9, None), ANTI_ONESHOT_CAP.base);
            // Step 1 still negates the whole hit, and step 4 still takes all of it.
            assert_eq!(hit(sink, Some(&source.id), on_unit(&shielded_target.id), 7, None), 0);
            assert_eq!(live(sink.state, &shielded_target.id).divine_shield_spent, Some(true));
            assert_eq!(hit(sink, Some(&source.id), on_unit(&wall.id), 7, None), 0);

            // The same source without the keyword pays step 2 in full.
            find_instance_mut(sink.state, &source.id).expect("on the field").granted_keywords = vec![];
            assert_eq!(hit(sink, Some(&source.id), on_unit(&target.id), 7, None), 0);
        });
    }

    #[test]
    fn step_3_the_hero_cap_clamps_12_to_5_3_when_radiant_and_applies_to_a_hero_only() {
        let mut state = game();
        put(&mut state, &anti_oneshot().id, slot(P1, Row::Backrow, 1));
        let own = put(&mut state, &plain().id, slot(P1, Row::Units, 1));
        with_sink(&mut state, |sink| {
            assert_eq!(hit(sink, None, on_hero(P1), 12, None), ANTI_ONESHOT_CAP.base);
            assert_eq!(sink.state.players[P1].hero.health, HERO_HEALTH - 5);

            // A unit behind the same Anti-oneshot Armor takes the whole hit.
            assert_eq!(hit(sink, None, on_unit(&own.id), 12, None), 12);
            assert_eq!(live(sink.state, &own.id).damage, 12);

            // The other hero has no Anti-oneshot Armor of its own, so nothing clamps it.
            assert_eq!(hit(sink, None, on_hero(P2), 12, None), 12);
            assert_eq!(sink.state.players[P2].hero.health, HERO_HEALTH - 12);
        });

        let mut radiant = game();
        put_face(&mut radiant, &anti_oneshot().id, slot(P1, Row::Backrow, 1), true);
        assert_eq!(
            with_sink(&mut radiant, |sink| hit(sink, None, on_hero(P1), 12, None)),
            ANTI_ONESHOT_CAP.radiant
        );
        assert_eq!(radiant.players[P1].hero.health, HERO_HEALTH - 3);
    }

    #[test]
    fn r383_step_3_an_anti_oneshot_armor_standing_in_a_unit_zone_animated_still_caps_its_hero() {
        let mut state = game();
        put(&mut state, &anti_oneshot().id, slot(P1, Row::Units, 1));
        assert_eq!(
            with_sink(&mut state, |sink| hit(sink, None, on_hero(P1), 12, None)),
            ANTI_ONESHOT_CAP.base
        );
        assert_eq!(state.players[P1].hero.health, HERO_HEALTH - ANTI_ONESHOT_CAP.base);

        let mut radiant = game();
        put_face(&mut radiant, &anti_oneshot().id, slot(P1, Row::Units, 1), true);
        assert_eq!(
            with_sink(&mut radiant, |sink| hit(sink, None, on_hero(P1), 12, None)),
            ANTI_ONESHOT_CAP.radiant
        );
    }

    #[test]
    fn step_4_an_indestructible_unit_takes_0_stays_on_the_field_and_emits_no_damage_event() {
        let mut state = game();
        let target = put(&mut state, &indestructible().id, slot(P2, Row::Units, 3));
        let source = put(&mut state, &plain().id, slot(P1, Row::Units, 1));
        with_sink(&mut state, |sink| {
            assert_eq!(hit(sink, Some(&source.id), on_unit(&target.id), 99, None), 0);
            assert_eq!(
                hit(sink, Some(&source.id), on_unit(&target.id), 99, Some(json!({ "ignoreArmor": true }))),
                0
            );
            let now = live(sink.state, &target.id);
            assert_eq!(now.damage, 0);
            assert_eq!(now.marked_destroyed, None);
            assert_eq!(unit_view(sink.state, &now).health, 4);
            assert_eq!(
                card_at(sink.state, slot(P2, Row::Units, 3)).map(|card| card.id.clone()),
                Some(target.id.clone())
            );
            assert_eq!(events_of_type(sink.events, GameEventType::Damage).len(), 0);
        });
    }

    #[test]
    fn step_5_the_damage_event_carries_the_amount_actually_dealt() {
        let mut state = game();
        let source = put(&mut state, &plain().id, slot(P1, Row::Units, 1));
        let target = put(&mut state, &armoured().id, slot(P2, Row::Units, 1));
        with_sink(&mut state, |sink| {
            assert_eq!(
                hit(sink, Some(&source.id), on_unit(&target.id), 10, Some(json!({ "combat": true }))),
                3
            );
            assert_eq!(
                to_json(&events_of_type(sink.events, GameEventType::Damage)[0]),
                json!({ "type": "damage", "sourceId": source.id, "targetId": target.id, "amount": 3, "combat": true })
            );
            let now = live(sink.state, &target.id);
            assert_eq!(now.damage, 3);
            // R42: the hit left the unit standing, so it killed nothing and credits no killer.
            assert_eq!(now.last_damaged_by, None);

            sink.state.players[P2].hero.armor = 1;
            assert_eq!(hit(sink, None, on_hero(P2), 4, None), 3);
            assert_eq!(
                to_json(&events_of_type(sink.events, GameEventType::Damage)[1]),
                json!({ "type": "damage", "sourceId": null, "targetId": "hero-p2", "amount": 3, "combat": false })
            );
        });
    }

    #[test]
    fn step_6_an_instance_armor_reduces_to_0_emits_no_damage_event_and_a_0_hit_is_no_instance_at_all_r63() {
        // M1 has no trigger dispatch, so what step 6 makes observable is the `damage` event an
        // on-damage trigger such as Fed Fauci's Plague Counter would fire from: one per instance dealt,
        // none at all for an instance stopped before step 5.
        let mut state = game();
        let armour = put(&mut state, &armoured().id, slot(P2, Row::Units, 1));
        let shield = put(&mut state, &shielded().id, slot(P2, Row::Units, 2));
        with_sink(&mut state, |sink| {
            assert_eq!(hit(sink, None, on_unit(&armour.id), 7, None), 0);
            assert_eq!(live(sink.state, &armour.id).damage, 0);
            assert_eq!(sink.events.len(), 0);

            // A hit of 0 before step 1 is not a damage instance: the shield is still there.
            assert_eq!(hit(sink, None, on_unit(&shield.id), 0, None), 0);
            let now = live(sink.state, &shield.id);
            assert_eq!(now.divine_shield_spent, None);
            assert!(unit_has(sink.state, &now, KeywordKind::DivineShield));
            assert_eq!(sink.events.len(), 0);

            // Two instances that do land emit exactly one `damage` event each.
            assert_eq!(hit(sink, None, on_unit(&armour.id), 9, None), 2);
            assert_eq!(hit(sink, None, on_unit(&armour.id), 9, None), 2);
            assert_eq!(amounts(sink.events, GameEventType::Damage), vec![2, 2]);
        });
    }

    #[test]
    fn r85_an_effect_may_state_its_damage_has_lifesteal_without_the_source_carrying_the_keyword() {
        let mut state = game();
        // A plain 3/3 source: no Lifesteal anywhere on it (§6.1).
        let source = put(&mut state, &plain().id, slot(P1, Row::Units, 1));
        let armour = put(&mut state, &armoured().id, slot(P2, Row::Units, 1)); // Armor 7
        state.players[P1].hero.health = 20;
        with_sink(&mut state, |sink| {
            assert!(!unit_has(sink.state, &live(sink.state, &source.id), KeywordKind::Lifesteal));
            assert_eq!(
                hit(sink, Some(&source.id), on_hero(P2), 8, Some(json!({ "lifesteal": true }))),
                8
            );
            assert_eq!(sink.state.players[P1].hero.health, 28);
            // The keyword is not granted: the next hit without the flag heals nothing.
            assert_eq!(hit(sink, Some(&source.id), on_hero(P2), 2, None), 2);
            assert_eq!(sink.state.players[P1].hero.health, 28);

            // R85 heals the amount actually dealt, so Armor takes its share first (§4.4 step 2)...
            assert_eq!(
                hit(sink, Some(&source.id), on_unit(&armour.id), 9, Some(json!({ "lifesteal": true }))),
                2
            );
            assert_eq!(sink.state.players[P1].hero.health, 30);
            // ...and a hit Armor reduces to 0 heals nothing at all (R63).
            assert_eq!(
                hit(sink, Some(&source.id), on_unit(&armour.id), 7, Some(json!({ "lifesteal": true }))),
                0
            );
            assert_eq!(sink.state.players[P1].hero.health, 30);
        });
    }

    #[test]
    fn step_7_poisonous_destroys_on_1_dealt_not_on_0_and_never_affects_a_hero_r63() {
        let mut state = game();
        let source = put(&mut state, &poisonous().id, slot(P1, Row::Units, 1));
        let victim = put(&mut state, &big_body().id, slot(P2, Row::Units, 1));
        let armour = put(&mut state, &armoured().id, slot(P2, Row::Units, 2));
        with_sink(&mut state, |sink| {
            assert_eq!(hit(sink, Some(&source.id), on_unit(&victim.id), 1, None), 1);
            let now = live(sink.state, &victim.id);
            assert_eq!(now.marked_destroyed, Some(true));
            assert_eq!(unit_view(sink.state, &now).health, 9);

            // Armor 7 leaves 0 dealt, so steps 5 to 9 never run and nothing is marked.
            assert_eq!(hit(sink, Some(&source.id), on_unit(&armour.id), 1, None), 0);
            assert_eq!(live(sink.state, &armour.id).marked_destroyed, None);

            // Poisonous only affects units: the hero just takes the damage.
            assert_eq!(hit(sink, Some(&source.id), on_hero(P2), 1, None), 1);
            assert_eq!(sink.state.players[P2].hero.health, HERO_HEALTH - 1);
            assert_eq!(sink.state.result, None);
            assert_eq!(events_of_type(sink.events, GameEventType::Destroyed).len(), 0);
        });
    }

    #[test]
    fn step_8_lifesteal_heals_the_source_s_controller_s_hero_by_the_amount_dealt_after_armor() {
        let mut state = game();
        state.players[P1].hero.health = 20;
        let source = put(&mut state, &lifestealer().id, slot(P1, Row::Units, 1));
        let target = put(&mut state, &armoured().id, slot(P2, Row::Units, 1));
        with_sink(&mut state, |sink| {
            assert_eq!(hit(sink, Some(&source.id), on_unit(&target.id), 10, None), 3);
            assert_eq!(sink.state.players[P1].hero.health, 23);
            assert_eq!(sink.state.players[P2].hero.health, HERO_HEALTH);
            assert_eq!(
                to_json(&events_of_type(sink.events, GameEventType::Healed)),
                json!([{ "type": "healed", "targetId": "hero-p1", "amount": 3 }])
            );

            // A hit Armor reduces to 0 heals nothing, because step 8 never runs.
            assert_eq!(hit(sink, Some(&source.id), on_unit(&target.id), 5, None), 0);
            assert_eq!(sink.state.players[P1].hero.health, 23);
            assert_eq!(events_of_type(sink.events, GameEventType::Healed).len(), 1);
        });
    }

    #[test]
    fn step_9_trample_sends_only_the_excess_to_the_target_s_hero_from_non_combat_damage_too_r63() {
        let mut state = game();
        let source = put(&mut state, &trampler().id, slot(P1, Row::Units, 1));
        let target = put(&mut state, &plain().id, slot(P2, Row::Units, 1));
        let spare = put(&mut state, &plain().id, slot(P2, Row::Units, 2));
        with_sink(&mut state, |sink| {
            // No combat flag: Trample applies to any damage the unit deals.
            assert_eq!(hit(sink, Some(&source.id), on_unit(&target.id), 10, None), 3);
            assert_eq!(live(sink.state, &target.id).damage, 3);
            assert_eq!(sink.state.players[P2].hero.health, HERO_HEALTH - 7);
            let hits: Vec<(String, i32, bool)> = events_of_type(sink.events, GameEventType::Damage)
                .iter()
                .map(|event| match event {
                    GameEvent::Damage {
                        target_id,
                        amount,
                        combat,
                        ..
                    } => (target_id.clone(), *amount, *combat),
                    other => panic!("not a damage event: {other:?}"),
                })
                .collect();
            assert_eq!(
                hits,
                vec![(target.id.clone(), 3, false), ("hero-p2".to_string(), 7, false)]
            );

            // Nothing beyond the target's health means no second instance.
            assert_eq!(hit(sink, Some(&source.id), on_unit(&spare.id), 2, None), 2);
            assert_eq!(sink.state.players[P2].hero.health, HERO_HEALTH - 7);
            assert_eq!(events_of_type(sink.events, GameEventType::Damage).len(), 3);
        });

        // Trample plus Lifesteal heals the total damage once: 3 to the unit and 7 to the hero.
        let mut both = game();
        let drainer = put(&mut both, &trample_lifesteal().id, slot(P1, Row::Units, 1));
        let victim = put(&mut both, &plain().id, slot(P2, Row::Units, 1));
        with_sink(&mut both, |sink| {
            assert_eq!(hit(sink, Some(&drainer.id), on_unit(&victim.id), 10, None), 3);
            assert_eq!(sink.state.players[P2].hero.health, HERO_HEALTH - 7);
            assert_eq!(amounts(sink.events, GameEventType::Healed), vec![3, 7]);
            assert_eq!(sink.state.players[P1].hero.health, HERO_HEALTH + 10);
        });
    }

    #[test]
    fn step_10_cleave_hits_both_neighbours_for_the_attacker_s_attack_never_across_sides_and_through_a_divine_shield_r63()
    {
        let mut state = game();
        let attacker = put(&mut state, &cleaver().id, slot(P1, Row::Units, 3));
        let own_left = put(&mut state, &big_body().id, slot(P1, Row::Units, 2));
        let own_right = put(&mut state, &big_body().id, slot(P1, Row::Units, 4));
        let defender = put(&mut state, &big_body().id, slot(P2, Row::Units, 3));
        let left = put(&mut state, &big_body().id, slot(P2, Row::Units, 2));
        let right = put(&mut state, &big_body().id, slot(P2, Row::Units, 4));
        let all_combat = with_sink(&mut state, |sink| {
            let striking = live(sink.state, &attacker.id);
            let struck = live(sink.state, &defender.id);
            resolve_combat(sink, &striking, &AttackTarget::Unit { instance: struck });
            events_of_type(sink.events, GameEventType::Damage)
                .iter()
                .all(|event| matches!(event, GameEvent::Damage { combat: true, .. }))
        });

        assert_eq!(live(&state, &defender.id).damage, 3);
        assert_eq!(live(&state, &left.id).damage, 3);
        assert_eq!(live(&state, &right.id).damage, 3);
        assert_eq!(live(&state, &attacker.id).damage, 5); // the defender struck back
        // The attacker's own neighbours are on the other side of the field and are never cleaved.
        assert_eq!(live(&state, &own_left.id).damage, 0);
        assert_eq!(live(&state, &own_right.id).damage, 0);
        assert!(all_combat);

        // Cleave belongs to the attack, so Divine Shield on the defender does not stop it.
        let mut shield_state = game();
        let striker = put(&mut shield_state, &cleaver().id, slot(P1, Row::Units, 3));
        let shield = put(&mut shield_state, &shielded().id, slot(P2, Row::Units, 3));
        let next_to = put(&mut shield_state, &big_body().id, slot(P2, Row::Units, 2));
        let also_next_to = put(&mut shield_state, &big_body().id, slot(P2, Row::Units, 4));
        with_sink(&mut shield_state, |sink| {
            let striking = live(sink.state, &striker.id);
            let struck = live(sink.state, &shield.id);
            resolve_combat(sink, &striking, &AttackTarget::Unit { instance: struck });
        });

        let now = live(&shield_state, &shield.id);
        assert_eq!(now.damage, 0);
        assert_eq!(now.divine_shield_spent, Some(true));
        assert_eq!(live(&shield_state, &next_to.id).damage, 3);
        assert_eq!(live(&shield_state, &also_next_to.id).damage, 3);
    }
}

/// `describe("heal and lose health (§6.3, R18, R19, M2-T3)")`.
mod heal_and_lose_health_s6_3_r18_r19_m2_t3 {
    use super::*;

    #[test]
    fn heals_a_unit_only_up_to_its_max_health_and_heal_to_full_removes_all_damage() {
        let mut state = game();
        let unit = put(&mut state, &big_body().id, slot(P1, Row::Units, 1)); // 5/10
        find_instance_mut(&mut state, &unit.id).expect("on the field").damage = 4;
        with_sink(&mut state, |sink| {
            let now = live(sink.state, &unit.id);
            assert_eq!(heal_unit(sink, &now, 10), 4);
            let now = live(sink.state, &unit.id);
            assert_eq!(now.damage, 0);
            assert_eq!(unit_view(sink.state, &now).health, 10);
            assert_eq!(
                to_json(&events_of_type(sink.events, GameEventType::Healed)),
                json!([{ "type": "healed", "targetId": unit.id, "amount": 4 }])
            );

            // An undamaged unit heals nothing and emits nothing.
            assert_eq!(heal_unit(sink, &now, 5), 0);
            assert_eq!(events_of_type(sink.events, GameEventType::Healed).len(), 1);

            find_instance_mut(sink.state, &unit.id).expect("on the field").damage = 7;
            let now = live(sink.state, &unit.id);
            assert_eq!(heal_to_full(sink, &now), 7);
            let now = live(sink.state, &unit.id);
            assert_eq!(now.damage, 0);
            assert_eq!(unit_view(sink.state, &now).health, 10);
        });
    }

    #[test]
    fn heals_a_hero_with_no_cap_and_heal_up_to_n_raises_only_a_hero_below_n_r19() {
        let mut state = game();
        with_sink(&mut state, |sink| {
            assert_eq!(heal_hero(sink, P1, 5), 5);
            assert_eq!(sink.state.players[P1].hero.health, HERO_HEALTH + 5);
            assert_eq!(
                to_json(&events_of_type(sink.events, GameEventType::Healed)),
                json!([{ "type": "healed", "targetId": "hero-p1", "amount": 5 }])
            );

            sink.state.players[P2].hero.health = 12;
            assert_eq!(heal_hero_up_to(sink, P2, 30), 18);
            assert_eq!(sink.state.players[P2].hero.health, 30);
            assert_eq!(heal_hero_up_to(sink, P2, 30), 0);
            assert_eq!(sink.state.players[P2].hero.health, 30);
            assert_eq!(heal_hero_up_to(sink, P1, 30), 0);
            assert_eq!(sink.state.players[P1].hero.health, HERO_HEALTH + 5);
        });
    }

    #[test]
    fn r18_lose_health_bypasses_armor_the_hero_cap_and_the_damage_event() {
        let mut state = game();
        put(&mut state, &anti_oneshot().id, slot(P1, Row::Backrow, 1));
        state.players[P1].hero.armor = 5;
        with_sink(&mut state, |sink| {
            assert_eq!(lose_health(sink, P1, 12), 12);
            assert_eq!(sink.state.players[P1].hero.health, HERO_HEALTH - 12);
            assert_eq!(
                to_json(&events_of_type(sink.events, GameEventType::HealthLost)),
                json!([{ "type": "healthLost", "player": "p1", "amount": 12 }])
            );
            assert_eq!(events_of_type(sink.events, GameEventType::Damage).len(), 0);

            // The same 12 as damage loses 5 to Armor and is then clamped by the cap.
            assert_eq!(hit(sink, None, on_hero(P1), 12, None), ANTI_ONESHOT_CAP.base);
            assert_eq!(sink.state.players[P1].hero.health, HERO_HEALTH - 17);
        });
    }
}

/// §8 #91 Fed Fauci, trimmed to the half step 6 is responsible for: a trigger that reads the
/// `damage` event this card's own hit emits. The mana half of its text is a card script (M4).
fn fed_fauci() -> CardDef {
    json_as(json!({
        "id": "dm-fed-fauci",
        "index": "91",
        "name": "Fed Fauci (damage)",
        "set": "Core",
        "type": "Unit",
        "tags": ["Human"],
        "rarity": "Rare",
        "token": false,
        "cost": 2,
        "base": { "attack": 1, "health": 6, "keywords": [{ "kind": "Rush" }], "text": "+1 Plague Counter when this takes damage" },
        "radiant": { "attack": 2, "health": 12, "keywords": [{ "kind": "Rush" }], "text": "same" },
    }))
}

/// `describe("on-damage triggers (§4.4 step 6, M2-T3)")`.
mod on_damage_triggers_s4_4_step_6_m2_t3 {
    use super::*;

    #[test]
    fn r63_gives_fed_fauci_one_plague_counter_per_damage_instance_and_none_for_an_instance_armor_zeroed() {
        let mut state = game();
        let fauci_def = fed_fauci();
        let mut catalog = registered_catalog().clone();
        catalog.insert(fauci_def.id.clone(), fauci_def.clone());
        register_catalog(catalog);
        let script = Script {
            triggers: vec![TriggerDef::new(
                "plague-on-damage",
                &[GameEventType::Damage],
                |ctx, event| match (event, ctx.self_.as_ref()) {
                    (GameEvent::Damage { target_id, .. }, Some(me)) if *target_id == me.id => {
                        vec![plague(json_as(json!({ "amount": 1 })))]
                    }
                    _ => vec![],
                },
            )],
            ..Script::default()
        };
        let mut scripts = registered_scripts().clone();
        scripts.insert(
            fauci_def.id.clone(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        register_scripts(scripts);

        let fauci = put(&mut state, &fauci_def.id, slot(P2, Row::Units, 1));
        with_sink(&mut state, |sink| {
            // One instance of 2, one of 3: one token each, not one per point of damage.
            assert_eq!(hit(sink, None, on_unit(&fauci.id), 2, None), 2);
            settle(sink, Default::default());
            assert_eq!(live(sink.state, &fauci.id).counters.plague, Some(1));

            assert_eq!(hit(sink, None, on_unit(&fauci.id), 3, None), 3);
            settle(sink, Default::default());
            assert_eq!(live(sink.state, &fauci.id).counters.plague, Some(2));

            // An instance Armor reduces to 0 emits no `damage` event, so step 6 never runs (R63).
            find_instance_mut(sink.state, &fauci.id)
                .expect("on the field")
                .granted_keywords
                .push(Keyword::Armor { n: 7 });
            let before = sink.events.len();
            assert_eq!(hit(sink, None, on_unit(&fauci.id), 7, None), 0);
            assert_eq!(sink.events[before..].len(), 0);
            settle(sink, Default::default());
            assert_eq!(live(sink.state, &fauci.id).counters.plague, Some(2));

            // R78: the counter is the instance's, so leaving the field clears it.
            let values: Vec<i32> = events_of_type(sink.events, GameEventType::CounterChanged)
                .iter()
                .map(|event| match event {
                    GameEvent::CounterChanged { value, .. } => *value,
                    other => panic!("not a counterChanged event: {other:?}"),
                })
                .collect();
            assert_eq!(values, vec![1, 2]);
        });
    }
}
