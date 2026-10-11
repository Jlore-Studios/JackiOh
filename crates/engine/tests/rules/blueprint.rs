//! Engine tests for the Blueprint systems (MB26, R1280–R1284).
//!
//! Systems tested:
//! - STACK-BASE (R1281)
//! - WIDE-TRIBUTE (R1282)
//! - LANE-STRIKE (R1283)
//! - CAPTURE (R1284)

use jackioh_engine::effects::{self, CaptureArgs};
use jackioh_engine::params::set_param;
use jackioh_engine::play_choices::tribute_value_of;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::subsystems::lethal::projected_damage;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::fixtures::blueprint::*;
use super::fixtures::harness::{put, sink_for, slot};
use super::fixtures::play_pipeline_b::{pb_act, pb_reduce, plays_of, round_trip};
use super::fixtures::prompt_harness::board;

fn give_hand(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

fn on_unit(instance: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: instance.clone(),
    }
}

fn declare(sink: &mut EngineSink<'_>, attacker_id: &str, target_id: &str) -> Option<String> {
    let attacker = find_instance(sink.state, attacker_id).unwrap().clone();
    let target = on_unit(find_instance(sink.state, target_id).unwrap());
    declare_attack(sink, &attacker, &target).err().map(|e| e.message)
}

fn force(sink: &mut EngineSink<'_>, attacker_id: &str, target_id: &str) {
    let attacker = find_instance(sink.state, attacker_id).unwrap().clone();
    let target = on_unit(find_instance(sink.state, target_id).unwrap());
    force_attack(sink, &attacker, &target);
}

fn hits(events: &[GameEvent]) -> Vec<(Option<String>, String, i32, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                combat,
                ..
            } => Some((source_id.clone(), target_id.clone(), *amount, *combat)),
            _ => None,
        })
        .collect()
}

fn run<R>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> (R, Vec<GameEvent>) {
    let mut sink = sink_for(state);
    let out = f(&mut sink);
    let events = sink.events.clone();
    (out, events)
}

fn as_p1() -> HookOptions {
    HookOptions {
        controller: Some(P1),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// R1281: STACK-BASE
// ---------------------------------------------------------------------------
mod r1281_stack_base {
    use super::*;

    #[test]
    fn r1281_a_played_unit_names_the_base_zone_and_tops_it_the_base_dormant_beneath() {
        let mut state = board("r1281-base");
        register_blueprint();
        let plot = put(&mut state, "bp-plot", slot(P1, Row::Units, 1), json!({}));
        let unit = give_hand(&mut state, "bp-cheap-unit", P1);
        state.players.p1.mana.current = 5;

        let plays = plays_of(&state, &unit.id, P1);
        assert!(plays.iter().any(
            |p| matches!(p, ActionBody::Play { zone: Some(z), .. } if z.row == Row::Units && z.lane == 1)
        ));

        let state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": unit.id, "zone": { "row": "units", "lane": 1 } }),
        );

        let pile = pile_at(&state, slot(P1, Row::Units, 1)).expect("pile");
        assert_eq!(pile.len(), 2);
        assert_eq!(pile[0].id, unit.id);
        assert_eq!(pile[1].id, plot.id);
        assert!(is_buried(&state, &plot));
    }

    #[test]
    fn r1281_a_locked_or_reserved_base_zone_is_neither_offered_nor_accepted() {
        let mut state = board("r1281-lock");
        register_blueprint();
        let _plot = put(&mut state, "bp-plot", slot(P1, Row::Units, 1), json!({}));
        lock_zone(&mut state, slot(P1, Row::Units, 1));
        let unit = give_hand(&mut state, "bp-cheap-unit", P1);
        state.players.p1.mana.current = 5;

        assert!(!plays_of(&state, &unit.id, P1).iter().any(
            |p| matches!(p, ActionBody::Play { zone: Some(z), .. } if z.row == Row::Units && z.lane == 1)
        ));

        let res = pb_reduce(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": unit.id, "zone": { "row": "units", "lane": 1 } }),
        );
        assert!(
            res.error
                .unwrap_or_default()
                .contains("that units zone is not open")
        );
    }

    #[test]
    fn r1281_a_summon_never_stacks_and_the_opponent_s_base_takes_none_of_your_units() {
        let mut state = board("r1281-summon");
        register_blueprint();
        let _plot = put(&mut state, "bp-plot", slot(P1, Row::Units, 1), json!({}));
        let _p2_plot = put(&mut state, "bp-plot", slot(P2, Row::Units, 1), json!({}));

        let mut card = new_instance(&mut state, "bp-cheap-unit", P1, Zone::Hand { player: P1 });
        assert!(!place_on_field(
            &mut state,
            &mut card,
            slot(P1, Row::Units, 1),
            Default::default()
        ));

        let unit = give_hand(&mut state, "bp-cheap-unit", P1);
        state.players.p1.mana.current = 5;
        // P1's legal zones for playing are strictly within P1's side.
        let offered = legal_zones_for(&state, P1, &unit, &[]);
        assert!(!offered.is_empty());
    }

    #[test]
    fn r1281_the_base_resumes_when_the_pile_above_it_is_gone() {
        let mut state = board("r1281-resume");
        register_blueprint();
        let plot = put(&mut state, "bp-plot", slot(P1, Row::Units, 1), json!({}));
        let unit = give_hand(&mut state, "bp-cheap-unit", P1);
        state.players.p1.mana.current = 5;

        let mut state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": unit.id, "zone": { "row": "units", "lane": 1 } }),
        );

        assert!(is_buried(&state, &plot));
        let top_card = find_instance(&state, &unit.id).unwrap().clone();
        remove_from_field(&mut state, &top_card, Default::default());
        assert!(!is_buried(&state, &plot));
        assert_eq!(
            card_at(&state, slot(P1, Row::Units, 1)).map(|c| c.id.as_str()),
            Some(plot.id.as_str())
        );
    }

    #[test]
    fn r1281_the_radiant_base_buffs_the_unit_once_before_its_cry_the_base_face_never() {
        // Radiant base
        let mut state = board("r1281-radiant-buff");
        register_blueprint();
        let _plot = put(
            &mut state,
            "bp-plot",
            slot(P1, Row::Units, 1),
            json!({ "radiant": true }),
        );
        let unit = give_hand(&mut state, "bp-cry-unit", P1);
        state.players.p1.mana.current = 5;

        let res = pb_reduce(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": unit.id, "zone": { "row": "units", "lane": 1 } }),
        );
        let events = res.events;
        let upgrade_pos = events
            .iter()
            .position(|e| matches!(e, GameEvent::Upgraded { .. }));
        let damage_pos = events.iter().position(|e| matches!(e, GameEvent::Damage { .. }));
        assert!(upgrade_pos.is_some());
        assert!(damage_pos.is_some());
        assert!(upgrade_pos.unwrap() < damage_pos.unwrap());
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, GameEvent::Upgraded { .. }))
                .count(),
            1
        );

        // Base face never buffs
        let mut state_base = board("r1281-base-nobuff");
        register_blueprint();
        let _plot_base = put(&mut state_base, "bp-plot", slot(P1, Row::Units, 1), json!({}));
        let unit_base = give_hand(&mut state_base, "bp-cry-unit", P1);
        state_base.players.p1.mana.current = 5;

        let res_base = pb_reduce(
            &state_base,
            json!({ "type": "play", "playerId": "p1", "instanceId": unit_base.id, "zone": { "row": "units", "lane": 1 } }),
        );
        assert!(
            !res_base
                .events
                .iter()
                .any(|e| matches!(e, GameEvent::Upgraded { .. }))
        );
    }
}

// ---------------------------------------------------------------------------
// R1282: WIDE-TRIBUTE
// ---------------------------------------------------------------------------
mod r1282_wide_tribute {
    use super::*;

    #[test]
    fn r1282_offers_own_units_and_cheap_permanents_of_both_rows_and_sides_never_a_dear_enemy_or_dormant_card()
    {
        let mut state = board("r1282-offers");
        register_blueprint();
        let u_cheap = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 1), json!({}));
        let u_dear = put(&mut state, "bp-dear-unit", slot(P1, Row::Units, 2), json!({}));
        let eu_cheap = put(&mut state, "bp-cheap-unit", slot(P2, Row::Units, 1), json!({}));
        let eu_dear = put(&mut state, "bp-dear-unit", slot(P2, Row::Units, 2), json!({}));
        let b_cheap = put(&mut state, "bp-cheap-field", slot(P1, Row::Backrow, 1), json!({}));
        let eb_cheap = put(&mut state, "bp-cheap-field", slot(P2, Row::Backrow, 1), json!({}));

        // Dormant card under a stack: top is on field, plot is placed beneath top
        let top = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 3), json!({}));
        let plot = give_hand(&mut state, "bp-plot", P1);
        let mut plot_card = find_instance(&state, &plot.id).unwrap().clone();
        place_beneath_top(&mut state, &mut plot_card, slot(P1, Row::Units, 3));

        let church = give_hand(&mut state, "bp-church", P1);
        let offered = legal_tribute_units(&state, P1, &church);
        let ids: Vec<String> = offered.iter().map(|c| c.id.clone()).collect();

        assert!(ids.contains(&u_cheap.id));
        assert!(ids.contains(&u_dear.id)); // own units of any cost
        assert!(ids.contains(&eu_cheap.id));
        assert!(ids.contains(&b_cheap.id));
        assert!(ids.contains(&eb_cheap.id));
        assert!(ids.contains(&top.id));

        assert!(!ids.contains(&eu_dear.id)); // enemy dear unit not offered
        assert!(!ids.contains(&plot.id)); // dormant card not offered
    }

    #[test]
    fn r1282_pays_by_sacrifice_to_each_owner_s_graveyard_and_the_church_stays_its_player_s() {
        let mut state = board("r1282-pays");
        register_blueprint();
        let u1 = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 1), json!({}));
        let u2 = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 2), json!({}));
        let b1 = put(&mut state, "bp-cheap-field", slot(P1, Row::Backrow, 1), json!({}));
        let eu = put(&mut state, "bp-cheap-unit", slot(P2, Row::Units, 1), json!({}));
        let eb = put(&mut state, "bp-cheap-field", slot(P2, Row::Backrow, 1), json!({}));

        let church = give_hand(&mut state, "bp-church", P1);
        state.players.p1.mana.current = 10;

        let tributes = vec![
            u1.id.clone(),
            u2.id.clone(),
            b1.id.clone(),
            eu.id.clone(),
            eb.id.clone(),
        ];
        let state = pb_act(
            &state,
            json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": church.id,
                "zone": { "row": "units", "lane": 1 },
                "tributes": tributes,
            }),
        );

        let p1_grave_ids: Vec<String> = state.players.p1.graveyard.iter().map(|c| c.id.clone()).collect();
        let p2_grave_ids: Vec<String> = state.players.p2.graveyard.iter().map(|c| c.id.clone()).collect();

        assert!(p1_grave_ids.contains(&u1.id));
        assert!(p1_grave_ids.contains(&u2.id));
        assert!(p1_grave_ids.contains(&b1.id));

        assert!(p2_grave_ids.contains(&eu.id));
        assert!(p2_grave_ids.contains(&eb.id));

        assert_eq!(
            card_at(&state, slot(P1, Row::Units, 1)).map(|c| c.id.as_str()),
            Some(church.id.as_str())
        );
    }

    #[test]
    fn r1282_every_offered_set_is_minimal_with_a_card_worth_two() {
        let mut state = board("r1282-minimal");
        register_blueprint();
        let _w2 = put(&mut state, "bp-worth-two", slot(P1, Row::Units, 1), json!({}));
        let _u1 = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 2), json!({}));
        let _u2 = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 3), json!({}));
        let _u3 = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 4), json!({}));
        let _u4 = put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, 5), json!({}));

        let church = give_hand(&mut state, "bp-church", P1);
        let sets = legal_tribute_sets(&state, P1, &church);

        for set in sets {
            let total_worth: i32 = set
                .iter()
                .map(|id| tribute_value_of(&state, find_instance(&state, id).unwrap()))
                .sum();
            assert_eq!(total_worth, 5);
        }
    }

    #[test]
    fn r1282_cheap_reads_its_declared_number() {
        let mut state = board("r1282-param");
        register_blueprint();
        let _eu = put(&mut state, "bp-cheap-unit", slot(P2, Row::Units, 1), json!({}));
        let church = give_hand(&mut state, "bp-church", P1);

        set_param(find_instance_mut(&mut state, &church.id).unwrap(), "cheap", 0);
        let church_updated = find_instance(&state, &church.id).unwrap();

        let offered = legal_tribute_units(&state, P1, church_updated);
        // Cost of bp-cheap-unit is 1, cheap param is 0, so enemy cheap unit is NOT offered
        assert!(!offered.iter().any(|c| c.controller == P2));
    }

    #[test]
    fn r1282_legal_actions_lists_19_380_church_plays_on_a_full_board() {
        let mut state = board("r1282-full-board");
        register_blueprint();
        // 5 own units
        for lane in 1..=5 {
            put(&mut state, "bp-cheap-unit", slot(P1, Row::Units, lane), json!({}));
        }
        // 5 own backrows
        for lane in 1..=5 {
            put(
                &mut state,
                "bp-cheap-field",
                slot(P1, Row::Backrow, lane),
                json!({}),
            );
        }
        // 5 enemy units
        for lane in 1..=5 {
            put(&mut state, "bp-cheap-unit", slot(P2, Row::Units, lane), json!({}));
        }
        // 5 enemy backrows
        for lane in 1..=5 {
            put(
                &mut state,
                "bp-cheap-field",
                slot(P2, Row::Backrow, lane),
                json!({}),
            );
        }

        let church = give_hand(&mut state, "bp-church", P1);
        state.players.p1.mana.current = 10;

        let plays = plays_of(&state, &church.id, P1);
        assert_eq!(plays.len(), 19_380);
    }
}

// ---------------------------------------------------------------------------
// R1283: LANE-STRIKE
// ---------------------------------------------------------------------------
mod r1283_lane_strike {
    use super::*;

    #[test]
    fn r1283_struck_back_across_its_lane_it_hits_first_for_three_times_its_attack() {
        let mut state = board("r1283-strike-back");
        register_blueprint();
        let attacker = put(&mut state, "bp-big-unit", slot(P1, Row::Units, 1), json!({})); // 4/10
        let bunker = put(&mut state, "bp-bunker", slot(P2, Row::Units, 1), json!({})); // 5/10, First Strike, mult 3

        let (refused, events) = run(&mut state, |sink| declare(sink, &attacker.id, &bunker.id));
        assert_eq!(refused, None);

        // Bunker hits first for 5 * 3 = 15, destroying attacker before it can hit back
        let damage = hits(&events);
        assert_eq!(damage.len(), 1);
        assert_eq!(
            damage[0],
            (Some(bunker.id.clone()), attacker.id.clone(), 15, true)
        );
        assert_eq!(
            find_instance(&state, &attacker.id).unwrap().zone,
            Zone::Graveyard { player: P1 }
        );
        assert!(card_at(&state, slot(P1, Row::Units, 1)).is_none());
    }

    #[test]
    fn r1283_from_another_lane_it_hits_for_its_attack() {
        let mut state = board("r1283-diff-lane");
        register_blueprint();
        let attacker = put(&mut state, "bp-big-unit", slot(P1, Row::Units, 2), json!({})); // 4/10 in lane 2
        let bunker = put(&mut state, "bp-bunker", slot(P2, Row::Units, 1), json!({})); // Bunker in lane 1

        // Attacker in lane 2 attacks Bunker in lane 1
        let (refused, events) = run(&mut state, |sink| declare(sink, &attacker.id, &bunker.id));
        assert_eq!(refused, None);

        // Multiplier is 1 because different lanes! Bunker deals 5 damage, attacker survives and strikes back for 4 (reduced to 1 by Bunker's Armor 3)
        let damage = hits(&events);
        assert_eq!(damage.len(), 2);
        assert_eq!(damage[0], (Some(bunker.id.clone()), attacker.id.clone(), 5, true));
        assert_eq!(damage[1], (Some(attacker.id.clone()), bunker.id.clone(), 1, true));
    }

    #[test]
    fn r1283_armor_and_divine_shield_apply_after_the_multiplier() {
        let mut state = board("r1283-armor");
        register_blueprint();
        let attacker = put(
            &mut state,
            "bp-armored-big-unit",
            slot(P1, Row::Units, 1),
            json!({}),
        ); // 4/20 Armor 2 in lane 1
        let bunker = put(&mut state, "bp-bunker", slot(P2, Row::Units, 1), json!({}));

        let (_, events) = run(&mut state, |sink| declare(sink, &attacker.id, &bunker.id));
        // Hit was 15 -> Armor absorbs 2 -> 13 damage lands
        let damage = hits(&events);
        assert_eq!(
            damage[0],
            (Some(bunker.id.clone()), attacker.id.clone(), 13, true)
        );
    }

    #[test]
    fn r1283_a_forced_attack_across_its_lane_multiplies() {
        let mut state = board("r1283-forced");
        register_blueprint();
        let bunker = put(&mut state, "bp-bunker", slot(P1, Row::Units, 1), json!({}));
        let enemy = put(&mut state, "bp-dear-unit", slot(P2, Row::Units, 1), json!({}));

        let (_, events) = run(&mut state, |sink| force(sink, &bunker.id, &enemy.id));
        let damage = hits(&events);
        assert_eq!(damage[0], (Some(bunker.id.clone()), enemy.id.clone(), 15, true));
    }

    #[test]
    fn r1283_cleave_hits_and_non_combat_damage_are_not_multiplied() {
        let mut state = board("r1283-cleave");
        register_blueprint();
        let cleaver = put(
            &mut state,
            "bp-cleaving-bunker",
            slot(P1, Row::Units, 2),
            json!({}),
        ); // 2 attack, Cleave, Trample, mult 3
        let e1 = put(&mut state, "bp-dear-unit", slot(P2, Row::Units, 1), json!({}));
        let e2 = put(&mut state, "bp-big-unit", slot(P2, Row::Units, 2), json!({})); // 10 health, absorbs all 6
        let e3 = put(&mut state, "bp-dear-unit", slot(P2, Row::Units, 3), json!({}));

        let (_, events) = run(&mut state, |sink| force(sink, &cleaver.id, &e2.id));
        let damage = hits(&events);
        // Main target e2 in lane 2 takes 2 * 3 = 6 damage
        assert!(
            damage
                .iter()
                .any(|h| h.0 == Some(cleaver.id.clone()) && h.1 == e2.id && h.2 == 6)
        );
        // Cleave targets e1 and e3 take 2 damage (not multiplied)
        assert!(
            damage
                .iter()
                .any(|h| h.0 == Some(cleaver.id.clone()) && h.1 == e1.id && h.2 == 2)
        );
        assert!(
            damage
                .iter()
                .any(|h| h.0 == Some(cleaver.id.clone()) && h.1 == e3.id && h.2 == 2)
        );
    }

    #[test]
    fn r1283_my_pawn_s_projection_reads_the_multiplier() {
        let mut state = board("r1283-pawn");
        register_blueprint();
        let cleaver = put(
            &mut state,
            "bp-cleaving-bunker",
            slot(P1, Row::Units, 1),
            json!({}),
        );
        let enemy_same = put(&mut state, "bp-dear-unit", slot(P2, Row::Units, 1), json!({})); // 4 health
        let enemy_diff = put(&mut state, "bp-dear-unit", slot(P2, Row::Units, 2), json!({})); // 4 health

        // Cleaver has 2 attack, Trample, multiplier 3 across lane:
        // Across lane 1: hit is 2 * 3 = 6. Trample excess over 4 health is 2 to defending hero.
        assert_eq!(projected_damage(&state, &cleaver, &on_unit(&enemy_same)), 2);
        // Different lane: hit is 2 * 1 = 2. No excess over 4 health, so 0 to defending hero.
        assert_eq!(projected_damage(&state, &cleaver, &on_unit(&enemy_diff)), 0);
    }

    #[test]
    fn r1283_multiplier_reads_its_declared_number() {
        let mut state = board("r1283-param");
        register_blueprint();
        let bunker = put(&mut state, "bp-bunker", slot(P1, Row::Units, 1), json!({}));
        set_param(
            find_instance_mut(&mut state, &bunker.id).unwrap(),
            "multiplier",
            4,
        );
        let enemy = put(&mut state, "bp-dear-unit", slot(P2, Row::Units, 1), json!({}));

        let (_, events) = run(&mut state, |sink| force(sink, &bunker.id, &enemy.id));
        let damage = hits(&events);
        assert_eq!(damage[0], (Some(bunker.id.clone()), enemy.id.clone(), 20, true));
    }
}

// ---------------------------------------------------------------------------
// R1284: CAPTURE
// ---------------------------------------------------------------------------
mod r1284_capture {
    use super::*;

    #[test]
    fn r1284_an_opponent_s_played_unit_goes_beneath_the_captor_under_its_controller() {
        let mut state = board("r1284-capture");
        register_blueprint();
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let captor = put(&mut state, "bp-captor", slot(P1, Row::Units, 1), json!({}));

        state.turn = 4;
        state.active = P2;
        state.players.p2.mana.current = 5;

        let unit = give_hand(&mut state, "bp-cheap-unit", P2);
        let _spare = give_hand(&mut state, "bp-cheap-unit", P2); // Keep R82 away
        let res = pb_reduce(
            &state,
            json!({ "type": "play", "playerId": "p2", "instanceId": unit.id, "zone": { "row": "units", "lane": 1 } }),
        );
        assert_eq!(res.error, None);
        let state = res.state;

        // Captured unit is now beneath captor at P1 lane 1
        let pile = pile_at(&state, slot(P1, Row::Units, 1)).expect("captor pile");
        assert_eq!(pile.len(), 2);
        assert_eq!(pile[0].id, captor.id);
        assert_eq!(pile[1].id, unit.id);

        let captured = &pile[1];
        assert_eq!(captured.owner, P2);
        assert_eq!(captured.controller, P1);
        assert_eq!(captured.summoned_turn, Some(4));

        // ControlChanged event emitted
        assert!(res.events.iter().any(|e| matches!(
            e,
            GameEvent::ControlChanged {
                instance_id,
                controller,
                row,
                lane,
                how: Some(ControlHow::Steal),
                ..
            } if instance_id == &unit.id && *controller == P1 && *row == Row::Units && *lane == 1
        )));

        // P2's unit zone is empty
        assert!(card_at(&state, slot(P2, Row::Units, 1)).is_none());
    }

    #[test]
    fn r1284_a_summon_and_the_captor_s_own_player_s_play_are_not_captured() {
        let mut state = board("r1284-summon-noop");
        register_blueprint();
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let _captor = put(&mut state, "bp-captor", slot(P1, Row::Units, 1), json!({}));

        // Summon on P2's side is not a play
        let mut summon_card = new_instance(&mut state, "bp-cheap-unit", P2, Zone::Hand { player: P2 });
        place_on_field(
            &mut state,
            &mut summon_card,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        assert_eq!(pile_at(&state, slot(P1, Row::Units, 1)).unwrap().len(), 1);

        // P1's own play is not opponent's play
        let own_unit = give_hand(&mut state, "bp-cheap-unit", P1);
        let _spare = give_hand(&mut state, "bp-cheap-unit", P1);
        state.players.p1.mana.current = 5;
        let res = pb_reduce(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": own_unit.id, "zone": { "row": "units", "lane": 2 } }),
        );
        assert_eq!(pile_at(&res.state, slot(P1, Row::Units, 1)).unwrap().len(), 1);
        assert_eq!(pile_at(&res.state, slot(P1, Row::Units, 2)).unwrap().len(), 1);
    }

    #[test]
    fn r1284_damage_is_kept_and_the_newest_capture_lies_directly_beneath() {
        let mut state = board("r1284-damage-kept");
        register_blueprint();
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let captor = put(&mut state, "bp-captor", slot(P1, Row::Units, 1), json!({}));

        state.turn = 4;
        state.active = P2;
        state.players.p2.mana.current = 10;

        // Play unit A
        let u_a = give_hand(&mut state, "bp-cheap-unit", P2);
        let _spare1 = give_hand(&mut state, "bp-cheap-unit", P2);
        let _spare2 = give_hand(&mut state, "bp-cheap-unit", P2);
        let mut state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p2", "instanceId": u_a.id, "zone": { "row": "units", "lane": 1 } }),
        );
        // Apply damage to A
        find_instance_mut(&mut state, &u_a.id).unwrap().damage = 1;

        // Play unit B
        let u_b = give_hand(&mut state, "bp-cheap-unit", P2);
        let state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p2", "instanceId": u_b.id, "zone": { "row": "units", "lane": 1 } }),
        );

        let pile = pile_at(&state, slot(P1, Row::Units, 1)).unwrap();
        assert_eq!(pile.len(), 3);
        assert_eq!(pile[0].id, captor.id);
        assert_eq!(pile[1].id, u_b.id); // B directly beneath captor
        assert_eq!(pile[2].id, u_a.id); // A pushed deeper
        assert_eq!(pile[2].damage, 1); // damage preserved
    }

    #[test]
    fn r1284_a_card_it_uncovered_resumes_on_its_side() {
        let mut state = board("r1284-uncover");
        register_blueprint();
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let _captor = put(&mut state, "bp-captor", slot(P1, Row::Units, 1), json!({}));
        let plot = put(&mut state, "bp-plot", slot(P2, Row::Units, 1), json!({}));

        state.turn = 4;
        state.active = P2;
        state.players.p2.mana.current = 5;

        let unit = give_hand(&mut state, "bp-cheap-unit", P2);
        let _spare = give_hand(&mut state, "bp-cheap-unit", P2);
        let state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p2", "instanceId": unit.id, "zone": { "row": "units", "lane": 1 } }),
        );

        // Unit was captured, so P2's plot uncovers and resumes on P2's side!
        assert_eq!(
            card_at(&state, slot(P2, Row::Units, 1)).map(|c| c.id.as_str()),
            Some(plot.id.as_str())
        );
        assert!(!is_buried(&state, &plot));
    }

    #[test]
    fn r1284_when_the_captor_leaves_the_newest_capture_resumes_under_its_controller() {
        let mut state = board("r1284-leaves");
        register_blueprint();
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let captor = put(&mut state, "bp-captor", slot(P1, Row::Units, 1), json!({}));

        state.turn = 4;
        state.active = P2;
        state.players.p2.mana.current = 10;

        let u_a = give_hand(&mut state, "bp-cheap-unit", P2);
        let _spare1 = give_hand(&mut state, "bp-cheap-unit", P2);
        let _spare2 = give_hand(&mut state, "bp-cheap-unit", P2);
        let mut state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p2", "instanceId": u_a.id, "zone": { "row": "units", "lane": 1 } }),
        );
        let u_b = give_hand(&mut state, "bp-cheap-unit", P2);
        let mut state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p2", "instanceId": u_b.id, "zone": { "row": "units", "lane": 1 } }),
        );

        // Remove captor from field
        let captor_instance = find_instance(&state, &captor.id).unwrap().clone();
        remove_from_field(&mut state, &captor_instance, Default::default());

        // The newest capture (u_b) resumes at P1 lane 1 under P1
        let top = card_at(&state, slot(P1, Row::Units, 1)).expect("resumed unit");
        assert_eq!(top.id, u_b.id);
        assert_eq!(top.controller, P1);
    }

    #[test]
    fn r1284_a_buried_card_is_not_captured() {
        let mut state = board("r1284-buried-noop");
        register_blueprint();
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let captor = put(&mut state, "bp-captor", slot(P1, Row::Units, 1), json!({}));
        let _top = put(&mut state, "bp-cheap-unit", slot(P2, Row::Units, 1), json!({}));
        let plot = give_hand(&mut state, "bp-plot", P2);
        let mut plot_card = find_instance(&state, &plot.id).unwrap().clone();
        place_beneath_top(&mut state, &mut plot_card, slot(P2, Row::Units, 1));

        // Attempting to capture plot (which is buried)
        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, Some(&captor), as_p1());
        (effects::capture(CaptureArgs {
            instance_id: plot.id.clone(),
        })
        .apply)(&mut ctx);

        // Plot is still in P2's pile
        assert_eq!(pile_at(ctx.state, slot(P1, Row::Units, 1)).unwrap().len(), 1);
        assert_eq!(pile_at(ctx.state, slot(P2, Row::Units, 1)).unwrap().len(), 2);
    }

    #[test]
    fn r1284_the_state_round_trips_json_and_folds_from_its_log() {
        let mut state = board("r1284-roundtrip");
        register_blueprint();
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let _captor = put(&mut state, "bp-captor", slot(P1, Row::Units, 1), json!({}));
        state.turn = 4;
        state.active = P2;
        state.players.p2.mana.current = 5;
        let unit = give_hand(&mut state, "bp-cheap-unit", P2);
        let _spare = give_hand(&mut state, "bp-cheap-unit", P2);
        let state = pb_act(
            &state,
            json!({ "type": "play", "playerId": "p2", "instanceId": unit.id, "zone": { "row": "units", "lane": 1 } }),
        );

        let round_tripped = round_trip(&state);
        assert_eq!(
            serde_json::to_value(&round_tripped).unwrap(),
            serde_json::to_value(&state).unwrap()
        );
    }
}
