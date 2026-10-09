//! Meditative batch 22 (issue #538): the engine proofs for R1200–R1204, on the `mb22` fixtures.
//!
//! - R1200 (ME-RANDOMTARGETS): while a Mayor acts, declared targets, `target` prompts and attack
//!   targets are drawn at random; hand picks, modes and Tributes stay choices.
//! - R1201: the Radiant Mayor's Lucky 1 rolls its controller's draws again.
//! - R1202 (ME-ATTACKSUMMON): Windfast's replacement — a Unit from hand makes the attack, bounced
//!   afterwards on the base face.
//! - R1203 (ME-ATTACKSUMMON): Windfurious Prime's rider — joiners attack first.
//! - R1204 (ME-LETHALGUARD): §4.4 step 4a opens for Units too; `redirect: "self"` sends the hit to
//!   the guard through its own Armor, each guard catching a given hit once.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::fixtures::harness::{in_hand, put, set_library, sink_for, slot};
use super::fixtures::mb22 as fx;
use super::fixtures::mb22::{act, act_result, answer_with, playing, recorder, replays_to};

fn play_action(player: PlayerId, instance_id: &str) -> Value {
    json!({ "type": "play", "instanceId": instance_id, "playerId": player })
}

fn attack_action(player: PlayerId, attacker_id: &str, target_id: &str) -> Value {
    json!({ "type": "attack", "attackerId": attacker_id, "targetId": target_id, "playerId": player })
}

fn end_turn(player: PlayerId) -> Value {
    json!({ "type": "endTurn", "playerId": player })
}

/// Every `attackDeclared`, as (attacker, target, forced, instead_of).
fn declared_attacks(events: &[GameEvent]) -> Vec<(String, String, bool, Option<String>)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AttackDeclared {
                attacker_id,
                target_id,
                forced,
                instead_of,
            } => Some((
                attacker_id.clone(),
                target_id.clone(),
                *forced,
                instead_of.clone(),
            )),
            _ => None,
        })
        .collect()
}

fn damaged(events: &[GameEvent]) -> Vec<(String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } => Some((target_id.clone(), *amount)),
            _ => None,
        })
        .collect()
}

fn redirected(events: &[GameEvent]) -> Vec<(String, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Redirected { from_id, to_id, .. } => {
                Some((from_id.clone(), to_id.clone()))
            }
            _ => None,
        })
        .collect()
}

fn on_field(state: &GameState, id: &str) -> bool {
    find_instance(state, id).is_some_and(|card| card.zone.z() == ZoneName::Field)
}

// ---------------------------------------------------------------------------
// R1200: everything random while a Mayor acts
// ---------------------------------------------------------------------------

#[test]
fn r1200_legal_actions_offer_plays_and_attacks_without_targets() {
    let mut state = playing("mb22-offers");
    put(&mut state, &fx::mayor.id, slot(P1, Row::Units, 1), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P1, 1).remove(0);
    let skirmisher = put(&mut state, &fx::skirmisher.id, slot(P1, Row::Units, 2), json!({}));
    put(&mut state, &fx::palisade.id, slot(P2, Row::Units, 1), json!({}));

    let actions = legal_actions(&state, P1);
    let plays: Vec<&ActionBody> = actions
        .iter()
        .filter(|action| {
            matches!(action, ActionBody::Play { instance_id, .. } if instance_id == &bolt.id)
        })
        .collect();
    assert_eq!(plays.len(), 1, "one play, not one per target");
    assert!(
        matches!(plays[0], ActionBody::Play { targets: None, .. }),
        "the play carries no targets: {:?}",
        plays[0]
    );
    let attacks: Vec<&ActionBody> = actions
        .iter()
        .filter(|action| {
            matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == &skirmisher.id)
        })
        .collect();
    assert_eq!(attacks.len(), 1, "one attack per attacker");
    assert!(
        matches!(attacks[0], ActionBody::Attack { target_id, .. } if target_id == RANDOM_ATTACK_TARGET),
        "the attack names the random sentinel: {:?}",
        attacks[0]
    );
}

#[test]
fn r1200_a_declared_target_is_drawn_and_replays() {
    let mut state = playing("mb22-drawn");
    put(&mut state, &fx::mayor.id, slot(P1, Row::Units, 1), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P1, 1).remove(0);
    put(&mut state, &fx::ally.id, slot(P2, Row::Units, 1), json!({}));
    let start = clone_state(&state);

    let mut rec = recorder(&state);
    let result = rec.play(play_action(P1, &fx::bolt.id));
    // The 10 damage landed on exactly one legal target, whoever the draw named.
    assert_eq!(damaged(&result.events).len(), 1);
    assert!(replays_to(&start, &rec.log, rec.state()));
}

#[test]
fn r1200_a_target_prompt_is_answered_at_once_for_either_player() {
    let mut state = playing("mb22-prompt");
    put(&mut state, &fx::mayor.id, slot(P1, Row::Units, 1), json!({}));
    let asker = in_hand(&mut state, &fx::asker.id, P1, 1).remove(0);
    state = act(&state, play_action(P1, &asker.id));
    assert!(
        state.pending.is_none(),
        "p1's target prompt was answered at once"
    );
    assert!(on_field(&state, &asker.id));

    state = act(&state, end_turn(P1));
    let asker = in_hand(&mut state, &fx::asker.id, P2, 1).remove(0);
    state = act(&state, play_action(P2, &asker.id));
    assert!(
        state.pending.is_none(),
        "p2's target prompt was answered at once"
    );
    assert!(on_field(&state, &asker.id));
}

#[test]
fn r1200_hand_picks_and_modes_stay_choices() {
    let mut state = playing("mb22-tricky");
    put(&mut state, &fx::mayor.id, slot(P1, Row::Units, 1), json!({}));
    let tricky = in_hand(&mut state, &fx::tricky.id, P1, 1).remove(0);
    let pick = in_hand(&mut state, &fx::ally.id, P1, 1).remove(0);

    let actions = legal_actions(&state, P1);
    let plays: Vec<&ActionBody> = actions
        .iter()
        .filter(|action| {
            matches!(action, ActionBody::Play { instance_id, .. } if instance_id == &tricky.id)
        })
        .collect();
    assert_eq!(plays.len(), 2, "one play per mode, not per target");
    for play in &plays {
        match play {
            ActionBody::Play { targets, modes, .. } => {
                assert_eq!(
                    targets.as_ref().map(Vec::len),
                    Some(1),
                    "the hand pick stays: {play:?}"
                );
                assert!(modes.is_some(), "the mode stays: {play:?}");
            }
            _ => panic!("a play: {play:?}"),
        }
    }
    // The play resolves with its hand pick while its declared target is drawn.
    state = act(
        &state,
        json!({
            "type": "play",
            "instanceId": tricky.id,
            "playerId": "p1",
            "targets": [{ "pick": "instance", "instanceId": pick.id }],
            "modes": ["red"],
        }),
    );
    assert!(
        find_instance(&state, &tricky.id).is_some_and(|card| card.zone.z() == ZoneName::Graveyard)
    );
}

#[test]
fn r1200_an_attack_target_is_drawn_and_taunt_holds() {
    let mut state = playing("mb22-taunt");
    put(&mut state, &fx::mayor.id, slot(P1, Row::Units, 1), json!({}));
    let skirmisher = put(&mut state, &fx::skirmisher.id, slot(P1, Row::Units, 2), json!({}));
    let palisade = put(&mut state, &fx::palisade.id, slot(P2, Row::Units, 1), json!({}));
    put(&mut state, &fx::ally.id, slot(P2, Row::Units, 2), json!({}));

    let result = act_result(&state, attack_action(P1, &fx::skirmisher.id, "random"));
    assert!(result.error.is_none(), "{:?}", result.error);
    let declared = declared_attacks(&result.events);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].1, palisade.id, "Taunt holds the draw");
    assert_eq!(
        damaged(&result.events),
        vec![(palisade.id.clone(), 3), (skirmisher.id.clone(), 2)],
        "the combat struck and struck back"
    );
}

#[test]
fn r1200_a_named_target_is_refused_and_the_view_flags_it() {
    let mut state = playing("mb22-named");
    put(&mut state, &fx::mayor.id, slot(P1, Row::Units, 1), json!({}));
    let skirmisher = put(&mut state, &fx::skirmisher.id, slot(P1, Row::Units, 2), json!({}));

    let result = act_result(&state, attack_action(P1, &fx::skirmisher.id, "hero-p2"));
    assert_eq!(
        result.error,
        Some("targets are drawn at random while a Mayor acts".to_string())
    );
    assert_eq!(view_for(&state, P1).random_targets, Some(true));
    assert_eq!(view_for(&state, P2).random_targets, Some(true));

    // The sentinel with no Mayor is refused the other way round.
    let mut bare = playing("mb22-bare");
    let skirmisher = put(&mut bare, &fx::skirmisher.id, slot(P1, Row::Units, 1), json!({}));
    let result = act_result(&bare, attack_action(P1, &fx::skirmisher.id, RANDOM_ATTACK_TARGET));
    assert_eq!(
        result.error,
        Some("no target random".to_string())
    );
}

#[test]
fn r1200_choices_return_once_the_mayor_leaves() {
    let mut state = playing("mb22-leaves");
    let mayor = put(&mut state, &fx::mayor.id, slot(P1, Row::Units, 1), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P1, 1).remove(0);
    put(&mut state, &fx::ally.id, slot(P2, Row::Units, 1), json!({}));
    assert_eq!(view_for(&state, P1).random_targets, Some(true));

    find_instance_mut(&mut state, &mayor.id)
        .expect("the Mayor")
        .zone = Zone::Graveyard { player: P1 };
    assert_eq!(view_for(&state, P1).random_targets, None);
    let plays = legal_actions(&state, P1)
        .into_iter()
        .filter(|action| {
            matches!(action, ActionBody::Play { instance_id, .. } if instance_id == &bolt.id)
        })
        .count();
    assert_eq!(plays, 3, "bolt at either hero and the ally again");
}

// ---------------------------------------------------------------------------
// R1201: Lucky rolls its controller's draws again
// ---------------------------------------------------------------------------

#[test]
fn r1201_lucky_keeps_the_aimed_side_the_opponent_rolls_once() {
    // The Radiant Mayor's controller: a Harm draw keeps the aimed (enemy) side.
    let mut state = playing("mb22-lucky");
    put(
        &mut state,
        &fx::mayor.id,
        slot(P1, Row::Units, 1),
        json!({ "radiant": true }),
    );
    let bolt = in_hand(&mut state, &fx::bolt.id, P1, 1).remove(0);
    let ally = put(&mut state, &fx::ally.id, slot(P2, Row::Units, 1), json!({}));
    let result = act_result(&state, play_action(P1, &fx::bolt.id));
    assert!(result.error.is_none(), "{:?}", result.error);
    let hit = damaged(&result.events);
    assert_eq!(hit.len(), 1);
    assert!(
        hit[0].0 == "hero-p2" || hit[0].0 == ally.id,
        "the aimed side kept: {:?}",
        hit
    );

    // The opponent's draw under the same Mayor rolls once, with no Lucky of its own.
    let smite = in_hand(&mut state, &fx::smite.id, P2, 1).remove(0);
    let result = act_result(
        &result.state,
        json!({
            "type": "play",
            "instanceId": smite.id,
            "playerId": "p2",
            "targets": [{ "pick": "hero", "player": "p1" }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    let hit = damaged(&result.events);
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].0, "hero-p1", "one roll, wherever it lands: {hit:?}");
}

// ---------------------------------------------------------------------------
// R1202: a Unit from hand makes the attack
// ---------------------------------------------------------------------------

/// Hands cleared to exactly these Units: the vanilla hands would ask a pick of their own.
fn clear_hands(state: &mut GameState) {
    state.players[P1].hand.clear();
    state.players[P2].hand.clear();
}

#[test]
fn r1202_a_unit_from_hand_makes_the_attack_and_is_bounced() {
    let mut state = playing("mb22-sub");
    let windfast = put(&mut state, &fx::windfast.id, slot(P1, Row::Units, 1), json!({}));
    clear_hands(&mut state);
    let offered = in_hand(&mut state, &fx::ally.id, P1, 1).remove(0);

    let result = act_result(&state, attack_action(P1, &fx::windfast.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(result.state.pending.is_none());
    let declared = declared_attacks(&result.events);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].0, offered.id, "the summoned Unit fought");
    assert!(!declared[0].2, "declared, not forced");
    assert_eq!(
        declared[0].3.as_deref(),
        Some(windfast.id.as_str()),
        "instead of Windfast"
    );
    // The 1/4 dealt 1 to the hero — heroes strike nothing back — and went home.
    assert_eq!(result.state.players[P2].hero.health, HERO_HEALTH - 1);
    assert!(
        result.state.players[P1]
            .hand
            .iter()
            .any(|card| card.id == offered.id),
        "bounced"
    );
    // Windfast spent its exertion without emitting its own declaration.
    assert!(
        find_instance(&result.state, &windfast.id)
            .expect("Windfast")
            .exertion
            .attacked
    );
}

#[test]
fn r1202_several_units_ask_one_asks_nothing() {
    let mut state = playing("mb22-pick");
    let windfast = put(&mut state, &fx::windfast.id, slot(P1, Row::Units, 1), json!({}));
    clear_hands(&mut state);
    let first = in_hand(&mut state, &fx::ally.id, P1, 1).remove(0);
    in_hand(&mut state, &fx::wall.id, P1, 1);

    let result = act_result(&state, attack_action(P1, &fx::windfast.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    let pending = result.state.pending.clone().expect("the hand pick");
    assert_eq!(pending.kind, PromptKind::Hand);
    let result = answer_with(
        &result.state,
        json!([{ "pick": "instance", "instanceId": first.id }]),
    );
    let declared = declared_attacks(&result.events);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].0, first.id);
    assert_eq!(result.state.players[P2].hero.health, HERO_HEALTH - 1);
    assert!(
        result.state.players[P1]
            .hand
            .iter()
            .any(|card| card.id == first.id),
        "the pick went home too"
    );

    // One Unit is no prompt (R129).
    let mut state = playing("mb22-no-pick");
    let windfast = put(&mut state, &fx::windfast.id, slot(P1, Row::Units, 1), json!({}));
    clear_hands(&mut state);
    in_hand(&mut state, &fx::ally.id, P1, 1);
    let result = act_result(&state, attack_action(P1, &fx::windfast.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(result.state.pending.is_none());
    assert_eq!(declared_attacks(&result.events).len(), 1);
}

#[test]
fn r1202_no_unit_or_zone_attacks_itself() {
    // No Unit in hand: Windfast attacks itself.
    let mut state = playing("mb22-self");
    let windfast = put(&mut state, &fx::windfast.id, slot(P1, Row::Units, 1), json!({}));
    clear_hands(&mut state);
    in_hand(&mut state, &fx::bolt.id, P1, 1);
    let result = act_result(&state, attack_action(P1, &fx::windfast.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    let declared = declared_attacks(&result.events);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].0, windfast.id);
    assert_eq!(declared[0].3, None);
    assert_eq!(result.state.players[P2].hero.health, HERO_HEALTH - 1);

    // No open zone: Windfast attacks itself too.
    let mut state = playing("mb22-full");
    let windfast = put(&mut state, &fx::windfast.id, slot(P1, Row::Units, 1), json!({}));
    for lane in 2..=5 {
        put(&mut state, &fx::ally.id, slot(P1, Row::Units, lane), json!({}));
    }
    clear_hands(&mut state);
    in_hand(&mut state, &fx::ally.id, P1, 1);
    let result = act_result(&state, attack_action(P1, &fx::windfast.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    let declared = declared_attacks(&result.events);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].0, windfast.id);
    assert_eq!(result.state.players[P2].hero.health, HERO_HEALTH - 1);
}

#[test]
fn r1202_a_forced_attack_is_replaced_on_the_opponents_turn() {
    let mut state = playing("mb22-forced");
    clear_hands(&mut state);
    let windfast = put(&mut state, &fx::windfast.id, slot(P2, Row::Units, 1), json!({}));
    let offered = in_hand(&mut state, &fx::ally.id, P2, 1).remove(0);
    state = act(&state, end_turn(P1));

    let mut sink = sink_for(&mut state);
    force_attack(&mut sink, &windfast, &AttackTarget::Hero { player: P1 });
    let declared = declared_attacks(&sink.events);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].0, offered.id, "the summoned Unit fought");
    assert!(declared[0].2, "forced, as Windfast's was");
    assert_eq!(declared[0].3.as_deref(), Some(windfast.id.as_str()));
    assert_eq!(sink.state.players[P1].hero.health, HERO_HEALTH - 1);
    assert!(
        sink.state.players[P2]
            .hand
            .iter()
            .any(|card| card.id == offered.id),
        "bounced"
    );
}

#[test]
fn r1202_windfury_summons_twice_and_the_invariant_monitor_is_clean() {
    let mut state = playing("mb22-fury");
    let windfast = put(&mut state, &fx::windfast.id, slot(P1, Row::Units, 1), json!({}));
    clear_hands(&mut state);
    let offered = in_hand(&mut state, &fx::ally.id, P1, 1).remove(0);

    let mut monitor = create_invariant_monitor(&state);
    let mut fought: Vec<String> = Vec::new();
    for _ in 0..2 {
        let action = ActionBody::Attack {
            attacker_id: windfast.id.clone(),
            target_id: "hero-p2".to_string(),
        };
        assert_eq!(monitor.before(&state, P1, &action), Vec::<String>::new());
        let result = act_result(&state, attack_action(P1, &fx::windfast.id, "hero-p2"));
        assert!(result.error.is_none(), "{:?}", result.error);
        assert_eq!(
            monitor.after(&result.events, &result.state),
            Vec::<String>::new()
        );
        fought.extend(declared_attacks(&result.events).into_iter().map(|declared| declared.0));
        state = result.state;
    }
    // Each of Windfury's two attacks summoned its own — the same Unit twice, bounced between.
    assert_eq!(fought, vec![offered.id.clone(), offered.id.clone()]);
    assert_eq!(state.players[P2].hero.health, HERO_HEALTH - 2);
    assert!(state.players[P1].hand.iter().any(|card| card.id == offered.id));
}

// ---------------------------------------------------------------------------
// R1203: joiners attack first
// ---------------------------------------------------------------------------

#[test]
fn r1203_joiners_attack_first_in_lane_order() {
    let mut state = playing("mb22-join");
    let prime = put(&mut state, &fx::prime.id, slot(P1, Row::Units, 3), json!({}));
    clear_hands(&mut state);
    in_hand(&mut state, &fx::ally.id, P1, 1);
    in_hand(&mut state, &fx::wall.id, P1, 1);
    set_library(&mut state, P1, &[] as &[&str]);

    let result = act_result(&state, attack_action(P1, &fx::prime.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    let declared = declared_attacks(&result.events);
    assert_eq!(declared.len(), 3, "two joiners, then the Prime: {declared:?}");
    assert!(declared[0].2 && declared[1].2, "the joiners are forced");
    assert!(!declared[2].2, "the Prime's own attack is declared");
    assert_eq!(declared[2].0, prime.id);
    // In lane order: the joiners hold increasing lanes in the order they attacked.
    let lanes: Vec<i32> = declared[..2]
        .iter()
        .map(|joiner| {
            slot_of(
                &result.state,
                &find_instance(&result.state, &joiner.0).expect("joiner"),
            )
            .expect("a slot")
            .lane
        })
        .collect();
    assert!(lanes[0] < lanes[1], "lane order: {lanes:?}");
    // All three hit: 1 + 1 + 5.
    assert_eq!(result.state.players[P2].hero.health, HERO_HEALTH - 7);
    // The joiners stay; nothing returns.
    assert_eq!(
        result.state.players[P1].units.iter().flatten().count(),
        3,
        "the Prime and its two joiners hold the field"
    );
}

#[test]
fn r1203_a_fallen_target_cancels_the_attack_exertion_spent() {
    let mut state = playing("mb22-fallen");
    let prime = put(&mut state, &fx::prime.id, slot(P1, Row::Units, 1), json!({}));
    let ally = put(&mut state, &fx::ally.id, slot(P2, Row::Units, 1), json!({}));
    put(&mut state, &fx::snapper.id, slot(P2, Row::Backrow, 1), json!({}));
    clear_hands(&mut state);
    in_hand(&mut state, &fx::wall.id, P1, 1);

    let result = act_result(&state, attack_action(P1, &fx::prime.id, &fx::ally.id));
    assert!(result.error.is_none(), "{:?}", result.error);
    // The target had left the field before its combat: cancelled, exertion spent.
    let cancelled: Vec<&GameEvent> = result
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::AttackCancelled { .. }))
        .collect();
    assert_eq!(cancelled.len(), 1, "attackCancelled is emitted");
    match &cancelled[0] {
        GameEvent::AttackCancelled {
            attacker_id,
            by_instance_id,
            ..
        } => {
            assert_eq!(attacker_id, &prime.id);
            assert_eq!(by_instance_id, &prime.id);
        }
        _ => unreachable!(),
    }
    assert!(damaged(&result.events).is_empty(), "no combat resolved");
    assert!(
        find_instance(&result.state, &prime.id)
            .expect("the Prime")
            .exertion
            .attacked
    );
    // … and the joiners never mustered: the pool is untouched.
    assert_eq!(result.state.players[P1].hand.len(), 1);
}

#[test]
fn r1203_fewer_zones_fewer_joiners_no_pool_no_draw() {
    // One open zone, three Units waiting: one joiner.
    let mut state = playing("mb22-room");
    let prime = put(&mut state, &fx::prime.id, slot(P1, Row::Units, 1), json!({}));
    for lane in 2..=4 {
        put(&mut state, &fx::ally.id, slot(P1, Row::Units, lane), json!({}));
    }
    clear_hands(&mut state);
    in_hand(&mut state, &fx::wall.id, P1, 1);
    in_hand(&mut state, &fx::skirmisher.id, P1, 1);
    set_library(&mut state, P1, &["mb22-ally"]);
    let result = act_result(&state, attack_action(P1, &fx::prime.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    let declared = declared_attacks(&result.events);
    assert_eq!(declared.len(), 2, "one joiner, then the Prime: {declared:?}");
    assert!(declared[0].2);

    // The deck alone is a pool too.
    let mut state = playing("mb22-deck");
    let prime = put(&mut state, &fx::prime.id, slot(P1, Row::Units, 1), json!({}));
    clear_hands(&mut state);
    set_library(&mut state, P1, &["mb22-ally", "mb22-wall"]);
    let result = act_result(&state, attack_action(P1, &fx::prime.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(declared_attacks(&result.events).len(), 3);

    // No pool, no draw: the Prime fights alone and the rng cursor does not move.
    let mut state = playing("mb22-empty");
    let prime = put(&mut state, &fx::prime.id, slot(P1, Row::Units, 1), json!({}));
    clear_hands(&mut state);
    set_library(&mut state, P1, &[] as &[&str]);
    let before = state.rng_cursor;
    let result = act_result(&state, attack_action(P1, &fx::prime.id, "hero-p2"));
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(declared_attacks(&result.events).len(), 1);
    assert_eq!(result.state.rng_cursor, before, "no pool, no draw");
    assert_eq!(result.state.players[P2].hero.health, HERO_HEALTH - 5);
}

// ---------------------------------------------------------------------------
// R1204: the lethal guard
// ---------------------------------------------------------------------------

#[test]
fn r1204_a_lethal_hit_reaches_the_guard_through_its_armor() {
    let mut state = playing("mb22-catch");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let ally = put(&mut state, &fx::ally.id, slot(P1, Row::Units, 2), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P2, 1).remove(0);

    state = act(&state, end_turn(P1));
    let result = act_result(
        &state,
        json!({
            "type": "play",
            "instanceId": bolt.id,
            "playerId": "p2",
            "targets": [{ "pick": "instance", "instanceId": ally.id }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    // 10 damage aimed at the 4-health ally reached the guard as 7, through its Armor 3.
    assert_eq!(
        find_instance(&result.state, &ally.id).expect("the ally").damage,
        0,
        "the ally is untouched"
    );
    assert_eq!(damaged(&result.events), vec![(guard.id.clone(), 7)], "the guard took 7");
    assert_eq!(
        redirected(&result.events),
        vec![(ally.id.clone(), guard.id.clone())]
    );
}

#[test]
fn r1204_shield_indestructible_and_survivable_hits_are_not_caught() {
    // A Divine Shield takes the hit first: no window opens.
    let mut state = playing("mb22-screen");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let screen = put(&mut state, &fx::screen.id, slot(P1, Row::Units, 2), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P2, 1).remove(0);
    state = act(&state, end_turn(P1));
    let result = act_result(
        &state,
        json!({
            "type": "play",
            "instanceId": bolt.id,
            "playerId": "p2",
            "targets": [{ "pick": "instance", "instanceId": screen.id }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(
        find_instance(&result.state, &screen.id).expect("the screen").damage,
        0
    );
    assert!(redirected(&result.events).is_empty());
    assert_eq!(
        find_instance(&result.state, &guard.id).expect("the guard").damage,
        0
    );

    // An Indestructible unit takes nothing: no window opens.
    let mut state = playing("mb22-bastion");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let bastion = put(&mut state, &fx::bastion.id, slot(P1, Row::Units, 2), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P2, 1).remove(0);
    state = act(&state, end_turn(P1));
    let result = act_result(
        &state,
        json!({
            "type": "play",
            "instanceId": bolt.id,
            "playerId": "p2",
            "targets": [{ "pick": "instance", "instanceId": bastion.id }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(redirected(&result.events).is_empty());

    // A survivable hit is not caught.
    let mut state = playing("mb22-whole");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let wall = put(&mut state, &fx::wall.id, slot(P1, Row::Units, 2), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P2, 1).remove(0);
    state = act(&state, end_turn(P1));
    let result = act_result(
        &state,
        json!({
            "type": "play",
            "instanceId": bolt.id,
            "playerId": "p2",
            "targets": [{ "pick": "instance", "instanceId": wall.id }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(find_instance(&result.state, &wall.id).expect("the wall").damage, 10);
    assert!(redirected(&result.events).is_empty());
}

#[test]
fn r1204_a_second_guard_catches_what_kills_the_first() {
    let mut state = playing("mb22-second");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let guard2 = put(&mut state, &fx::guard2.id, slot(P1, Row::Units, 2), json!({}));
    let ally = put(&mut state, &fx::ally.id, slot(P1, Row::Units, 3), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P2, 1).remove(0);
    state = act(&state, end_turn(P1));

    let result = act_result(
        &state,
        json!({
            "type": "play",
            "instanceId": bolt.id,
            "playerId": "p2",
            "targets": [{ "pick": "instance", "instanceId": ally.id }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    // The 10 would kill the first guard through its armor too, so the second catches it: each
    // guard caught the hit once, the ally never took it.
    assert_eq!(
        redirected(&result.events),
        vec![
            (ally.id.clone(), guard.id.clone()),
            (guard.id.clone(), guard2.id.clone())
        ]
    );
    assert_eq!(find_instance(&result.state, &ally.id).expect("the ally").damage, 0);
    assert!(on_field(&result.state, &guard.id), "the first guard stands");
    assert_eq!(
        find_instance(&result.state, &guard2.id).expect("the second guard").damage,
        7
    );
}

#[test]
fn r1204_the_hero_is_guarded_enemy_units_are_not() {
    // 30 damage on the hero is lethal: the guard takes it through its Armor 3.
    let mut state = playing("mb22-hero");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let smite = in_hand(&mut state, &fx::smite.id, P2, 1).remove(0);
    state = act(&state, end_turn(P1));
    let result = act_result(
        &state,
        json!({
            "type": "play",
            "instanceId": smite.id,
            "playerId": "p2",
            "targets": [{ "pick": "hero", "player": "p1" }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.state.players[P1].hero.health, HERO_HEALTH);
    assert_eq!(damaged(&result.events), vec![(guard.id.clone(), 27)], "30 through Armor 3");

    // A lethal hit on an enemy unit is no friendly ally: nothing answers.
    let mut state = playing("mb22-foe");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let ally = put(&mut state, &fx::ally.id, slot(P2, Row::Units, 1), json!({}));
    let bolt = in_hand(&mut state, &fx::bolt.id, P1, 1).remove(0);
    let result = act_result(
        &state,
        json!({
            "type": "play",
            "instanceId": bolt.id,
            "playerId": "p1",
            "targets": [{ "pick": "instance", "instanceId": ally.id }],
        }),
    );
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(redirected(&result.events).is_empty());
    assert!(
        find_instance(&result.state, &ally.id)
            .is_some_and(|card| card.zone.z() == ZoneName::Graveyard),
        "the enemy ally died unguarded"
    );
    assert_eq!(find_instance(&result.state, &guard.id).expect("the guard").damage, 0);
}

#[test]
fn r1204_trample_into_the_guard_tramples_into_its_hero() {
    let mut state = playing("mb22-trample");
    let guard = put(&mut state, &fx::guard.id, slot(P1, Row::Units, 1), json!({}));
    let ally = put(&mut state, &fx::ally.id, slot(P1, Row::Units, 2), json!({}));
    state = act(&state, end_turn(P1));
    let ram = put(&mut state, &fx::ram.id, slot(P2, Row::Units, 1), json!({}));

    let result = act_result(&state, attack_action(P2, &fx::ram.id, &fx::ally.id));
    assert!(result.error.is_none(), "{:?}", result.error);
    // The guard caught the 10 into its 5 health and died of the 7; the excess of 5 tramples on.
    assert_eq!(find_instance(&result.state, &ally.id).expect("the ally").damage, 0);
    assert_eq!(
        redirected(&result.events),
        vec![(ally.id.clone(), guard.id.clone())]
    );
    assert_eq!(result.state.players[P1].hero.health, HERO_HEALTH - 5);
}
