//! Attack validation: §4.2 steps 1 to 3, §4.1, §6.1 and R5, R6, R7 (M2-T2).
//! One test per refusal reason, plus the positive cases that give the refusals meaning.
//!
//! Port of `packages/engine/test/combat-validation.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{
    big_dfender, charger, deft_duelist, pacifist, plain, rusher, stacker, taunter, zero_attack,
};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

/// An `ActionInput` from its TS object literal (SURFACE §8: an object literal ports as `json!`).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

/// TS's module `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn next_nonce() -> String {
    format!("cv{}", NONCE.fetch_add(1, Ordering::SeqCst) + 1)
}

fn act(state: &GameState, body: ActionInput) -> GameState {
    let result = reduce(state, &body.with_nonce(next_nonce()));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result.state
}

/// An attack action from p1 (TS `attackVia(state, attackerId, targetId, player = "p1")`; no caller
/// names another player).
fn attack_via(state: &GameState, attacker_id: &str, target_id: &str) -> ReduceResult {
    let action: Action = json_as(json!({
        "type": "attack", "attackerId": attacker_id, "targetId": target_id, "playerId": "p1", "nonce": next_nonce()
    }));
    reduce(state, &action)
}

/// Past the mulligans, in the main phase of p1's first turn.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
        state = act(&state, input(json!({ "type": "mulligan", "keep": keep, "playerId": player })));
    }
    state
}

fn on_unit(instance: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: instance.clone(),
    }
}

fn on_hero(player: PlayerId) -> AttackTarget {
    AttackTarget::Hero { player }
}

/// §4.1: a unit that entered the field this turn is summoning sick.
fn make_sick(state: &mut GameState, unit: CardInstance) -> CardInstance {
    let turn = state.turn;
    let live = instance_mut(state, &unit.id);
    live.summoned_turn = Some(turn);
    live.clone()
}

fn instance<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no instance {id}"))
}

fn instance_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no instance {id}"))
}

fn target_name(target: &AttackTarget) -> String {
    match target {
        AttackTarget::Unit { instance, .. } => instance.id.clone(),
        AttackTarget::Hero { player, .. } => format!("hero-{player}"),
    }
}

fn names(state: &GameState, attacker_id: &str) -> Vec<String> {
    attack_targets(state, instance(state, attacker_id)).iter().map(target_name).collect()
}

/// `whyCannotAttack(state, attacker, target)` with the attacker as it stands now: its refusal, or
/// `None` (TS `null`).
fn why(state: &GameState, attacker_id: &str, target: &AttackTarget) -> Option<String> {
    why_cannot_attack(state, instance(state, attacker_id), target)
        .err()
        .map(|error| error.message)
}

/// A unit target read as it stands now.
fn unit(state: &GameState, id: &str) -> AttackTarget {
    on_unit(instance(state, id))
}

fn targets_of(state: &GameState, attacker_id: &str) -> Vec<AttackTarget> {
    attack_targets(state, instance(state, attacker_id))
}

fn refused(reason: &str) -> Option<String> {
    Some(reason.to_string())
}

mod attack_validation_the_attacker_4_2_step_1_m2_t2 {
    use super::*;

    #[test]
    fn lets_a_unit_that_has_been_on_the_field_since_last_turn_attack_a_unit_or_the_hero() {
        let mut state = new_game("legal-attack", None);
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        assert_eq!(why(&state, &attacker.id, &unit(&state, &defender.id)), None);
        assert_eq!(why(&state, &attacker.id, &on_hero(P2)), None);
        assert!(can_attack(&state, instance(&state, &attacker.id), &unit(&state, &defender.id)));
        assert!(can_attack(&state, instance(&state, &attacker.id), &on_hero(P2)));
    }

    #[test]
    fn r6_refuses_a_unit_that_has_already_acted_this_turn_while_deft_duelist_may_still_attack() {
        let mut state = new_game("exertion-spent", None);
        let attacked = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let switched = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        let duelist = put(&mut state, &deft_duelist.id, slot(P1, Row::Units, 3), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        instance_mut(&mut state, &attacked.id).exertion.attacked = true;
        assert_eq!(
            why(&state, &attacked.id, &unit(&state, &defender.id)),
            refused("that unit has already acted this turn")
        );
        assert_eq!(
            why(&state, &attacked.id, &on_hero(P2)),
            refused("that unit has already acted this turn")
        );
        assert!(targets_of(&state, &attacked.id).is_empty());

        // R6: switching to Attack Position spends the turn's exertion too.
        instance_mut(&mut state, &switched.id).exertion.switched = true;
        assert_eq!(
            why(&state, &switched.id, &unit(&state, &defender.id)),
            refused("that unit has already acted this turn")
        );

        // R49: Deft Duelist has two exertions, one attack and one switch.
        instance_mut(&mut state, &duelist.id).exertion.switched = true;
        assert_eq!(why(&state, &duelist.id, &unit(&state, &defender.id)), None);
        instance_mut(&mut state, &duelist.id).exertion.attacked = true;
        assert_eq!(
            why(&state, &duelist.id, &unit(&state, &defender.id)),
            refused("that unit has already acted this turn")
        );
    }

    #[test]
    fn r6_refuses_a_unit_in_defense_position() {
        let mut state = new_game("defense-position", None);
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        instance_mut(&mut state, &attacker.id).position = Some(Position::Def);
        assert_eq!(
            why(&state, &attacker.id, &unit(&state, &defender.id)),
            refused("only Attack-Position units may attack")
        );
        assert_eq!(
            why(&state, &attacker.id, &on_hero(P2)),
            refused("only Attack-Position units may attack")
        );
        assert!(targets_of(&state, &attacker.id).is_empty());

        instance_mut(&mut state, &attacker.id).position = Some(Position::Atk);
        assert_eq!(why(&state, &attacker.id, &unit(&state, &defender.id)), None);
    }

    #[test]
    fn refuses_a_summoning_sick_unit_without_rush_or_charge_4_1() {
        let mut state = new_game("summoning-sick", None);
        let placed = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let sick = make_sick(&mut state, placed);
        let placed = put(&mut state, &rusher.id, slot(P1, Row::Units, 2), Default::default());
        let sick_rusher = make_sick(&mut state, placed);
        let placed = put(&mut state, &charger.id, slot(P1, Row::Units, 3), Default::default());
        let sick_charger = make_sick(&mut state, placed);
        let placed = put(&mut state, &plain.id, slot(P1, Row::Units, 4), Default::default());
        let granted = make_sick(&mut state, placed);
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        assert_eq!(why(&state, &sick.id, &unit(&state, &defender.id)), refused("that unit is summoning sick"));
        assert_eq!(why(&state, &sick.id, &on_hero(P2)), refused("that unit is summoning sick"));
        assert!(targets_of(&state, &sick.id).is_empty());

        // §6.1: Rush lifts sickness for unit targets, Charge for units and the hero.
        assert_eq!(why(&state, &sick_rusher.id, &unit(&state, &defender.id)), None);
        assert_eq!(why(&state, &sick_charger.id, &unit(&state, &defender.id)), None);

        // A granted keyword counts the same as a printed one (§10.4).
        instance_mut(&mut state, &granted.id).granted_keywords.push(Keyword::Rush);
        assert_eq!(why(&state, &granted.id, &unit(&state, &defender.id)), None);
    }

    #[test]
    fn r7_refuses_a_unit_with_0_attack() {
        let mut state = new_game("zero-attack", None);
        let zero = put(&mut state, &zero_attack.id, slot(P1, Row::Units, 1), Default::default());
        let drained = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        assert_eq!(
            why(&state, &zero.id, &unit(&state, &defender.id)),
            refused("a unit with 0 attack cannot attack")
        );
        assert_eq!(why(&state, &zero.id, &on_hero(P2)), refused("a unit with 0 attack cannot attack"));
        assert!(targets_of(&state, &zero.id).is_empty());

        // A 3/3 whose attack was dragged to 0 is refused the same way (§10.4 layer 4).
        instance_mut(&mut state, &drained.id).buffs.attack = -3;
        assert_eq!(
            why(&state, &drained.id, &unit(&state, &defender.id)),
            refused("a unit with 0 attack cannot attack")
        );
        instance_mut(&mut state, &drained.id).buffs.attack = -2;
        assert_eq!(why(&state, &drained.id, &unit(&state, &defender.id)), None);
    }

    #[test]
    fn r7_never_lets_big_d_fender_0_attack_be_an_attacker() {
        let mut state = new_game("big-dfender", None);
        let big_d = put(&mut state, &big_dfender.id, slot(P1, Row::Units, 1), Default::default());
        let radiant_big_d = put(
            &mut state,
            &big_dfender.id,
            slot(P1, Row::Units, 2),
            json_as(json!({ "radiant": true })),
        );
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        for unit_id in [&big_d.id, &radiant_big_d.id] {
            assert_eq!(
                why(&state, unit_id, &unit(&state, &defender.id)),
                refused("a unit with 0 attack cannot attack")
            );
            assert_eq!(why(&state, unit_id, &on_hero(P2)), refused("a unit with 0 attack cannot attack"));
            assert!(targets_of(&state, unit_id).is_empty());

            // Not even with the sickness exemptions or a fresh turn's exertion.
            let refreshed = instance_mut(&mut state, unit_id);
            refreshed.granted_keywords.push(Keyword::Rush);
            refreshed.granted_keywords.push(Keyword::Charge);
            refreshed.exertion = Exertion {
                attacked: false,
                switched: false,
                attacks: None,
            };
            assert!(targets_of(&state, unit_id).is_empty());
        }

        // And the action layer never offers it an attack.
        let mut game = playing("big-dfender-actions");
        let on_board = put(&mut game, &big_dfender.id, slot(P1, Row::Units, 1), Default::default());
        put(&mut game, &plain.id, slot(P2, Row::Units, 1), Default::default());
        assert!(!legal_actions(&game, P1).iter().any(|action| matches!(
            action,
            ActionBody::Attack { attacker_id, .. } if *attacker_id == on_board.id
        )));
    }

    #[test]
    fn refuses_a_unit_with_cant_attack_6_1() {
        let mut state = new_game("cant-attack", None);
        let stuck = put(&mut state, &pacifist.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        assert_eq!(why(&state, &stuck.id, &unit(&state, &defender.id)), refused("that unit cannot attack"));
        assert_eq!(why(&state, &stuck.id, &on_hero(P2)), refused("that unit cannot attack"));
        assert!(targets_of(&state, &stuck.id).is_empty());

        // Granting it to a plain unit refuses that unit too.
        let granted = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        assert_eq!(why(&state, &granted.id, &unit(&state, &defender.id)), None);
        instance_mut(&mut state, &granted.id).granted_keywords.push(Keyword::CantAttack);
        assert_eq!(why(&state, &granted.id, &unit(&state, &defender.id)), refused("that unit cannot attack"));
    }

    #[test]
    fn r13_refuses_an_attack_by_a_card_dormant_under_a_stack_3_2() {
        let mut state = new_game("stack-dormant", None);
        let under = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        assert_eq!(why(&state, &under.id, &unit(&state, &defender.id)), None);

        let mut top = new_instance(&mut state, &stacker.id, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(&mut state, &mut top, slot(P1, Row::Units, 1), json_as(json!({ "stack": true }))));

        assert_eq!(why(&state, &under.id, &unit(&state, &defender.id)), refused("that unit is not on the field"));
        assert_eq!(why(&state, &under.id, &on_hero(P2)), refused("that unit is not on the field"));
        assert!(targets_of(&state, &under.id).is_empty());

        // Only the top of the pile acts.
        assert_eq!(why(&state, &top.id, &unit(&state, &defender.id)), None);
    }
}

mod attack_validation_the_target_4_2_steps_2_and_3_m2_t2 {
    use super::*;

    #[test]
    fn refuses_a_target_that_is_not_an_enemy() {
        let mut state = new_game("not-an-enemy", None);
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let friend = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        assert_eq!(why(&state, &attacker.id, &unit(&state, &friend.id)), refused("that target is not an enemy"));
        assert_eq!(why(&state, &attacker.id, &on_hero(P1)), refused("that target is not an enemy"));
        assert_eq!(why(&state, &attacker.id, &unit(&state, &attacker.id)), refused("that target is not an enemy"));
        assert_eq!(why(&state, &attacker.id, &unit(&state, &defender.id)), None);
        assert_eq!(names(&state, &attacker.id), vec![defender.id.clone(), "hero-p2".to_string()]);
    }

    #[test]
    fn refuses_rush_against_the_hero_on_its_summon_turn_while_charge_may_hit_it_6_1() {
        let mut state = new_game("rush-vs-charge", None);
        let placed = put(&mut state, &rusher.id, slot(P1, Row::Units, 1), Default::default());
        let sick_rusher = make_sick(&mut state, placed);
        let placed = put(&mut state, &charger.id, slot(P1, Row::Units, 2), Default::default());
        let sick_charger = make_sick(&mut state, placed);
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        assert_eq!(
            why(&state, &sick_rusher.id, &on_hero(P2)),
            refused("Rush cannot hit the hero on its summon turn")
        );
        assert_eq!(why(&state, &sick_rusher.id, &unit(&state, &defender.id)), None);
        assert_eq!(names(&state, &sick_rusher.id), vec![defender.id.clone()]);

        assert_eq!(why(&state, &sick_charger.id, &on_hero(P2)), None);
        assert_eq!(names(&state, &sick_charger.id), vec![defender.id.clone(), "hero-p2".to_string()]);

        // The next turn the Rush unit is no longer sick and may go face.
        instance_mut(&mut state, &sick_rusher.id).summoned_turn = None;
        assert_eq!(why(&state, &sick_rusher.id, &on_hero(P2)), None);
    }

    #[test]
    fn requires_a_taunt_unit_to_be_attacked_first_printed_or_granted_4_2_step_3() {
        let mut state = new_game("taunt-filter", None);
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let bystander = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let wall = put(&mut state, &taunter.id, slot(P2, Row::Units, 2), Default::default());

        assert_eq!(
            why(&state, &attacker.id, &unit(&state, &bystander.id)),
            refused("a Taunt unit must be attacked first")
        );
        assert_eq!(why(&state, &attacker.id, &on_hero(P2)), refused("a Taunt unit must be attacked first"));
        assert_eq!(why(&state, &attacker.id, &unit(&state, &wall.id)), None);
        assert_eq!(names(&state, &attacker.id), vec![wall.id.clone()]);

        // A granted Taunt filters exactly the same way, and a Taunt on your own side does not.
        let own = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        instance_mut(&mut state, &own.id).granted_keywords.push(Keyword::Taunt);
        instance_mut(&mut state, &bystander.id).granted_keywords.push(Keyword::Taunt);
        let mut listed = names(&state, &attacker.id);
        listed.sort();
        let mut expected = vec![bystander.id.clone(), wall.id.clone()];
        expected.sort();
        assert_eq!(listed, expected);
    }

    #[test]
    fn makes_a_defense_position_enemy_force_the_target_even_with_no_printed_taunt_4_1() {
        let mut state = new_game("defense-taunt", None);
        assert!(plain.base.keywords.is_empty());

        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let bystander = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let defending = put(&mut state, &plain.id, slot(P2, Row::Units, 2), Default::default());
        instance_mut(&mut state, &defending.id).position = Some(Position::Def);

        assert_eq!(
            why(&state, &attacker.id, &unit(&state, &bystander.id)),
            refused("a Taunt unit must be attacked first")
        );
        assert_eq!(why(&state, &attacker.id, &on_hero(P2)), refused("a Taunt unit must be attacked first"));
        assert_eq!(why(&state, &attacker.id, &unit(&state, &defending.id)), None);
        assert_eq!(names(&state, &attacker.id), vec![defending.id.clone()]);

        // Back to Attack Position and the board opens up again.
        instance_mut(&mut state, &defending.id).position = Some(Position::Atk);
        assert_eq!(
            names(&state, &attacker.id),
            vec![bystander.id.clone(), defending.id.clone(), "hero-p2".to_string()]
        );
    }

    #[test]
    fn r5_lets_a_lane_1_unit_attack_an_enemy_in_lane_5_since_attacks_are_not_lane_restricted() {
        assert!(!LANE_RESTRICTED_ATTACKS);

        let mut state = new_game("no-lane-restriction", None);
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let far = put(&mut state, &plain.id, slot(P2, Row::Units, 5), Default::default());

        assert_eq!(why(&state, &attacker.id, &unit(&state, &far.id)), None);
        assert_eq!(names(&state, &attacker.id), vec![far.id.clone(), "hero-p2".to_string()]);
    }

    #[test]
    fn attack_targets_lists_exactly_the_legal_targets_4_2() {
        let mut state = new_game("attack-targets", None);
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let first = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let third = put(&mut state, &plain.id, slot(P2, Row::Units, 3), Default::default());

        assert_eq!(
            names(&state, &attacker.id),
            vec![first.id.clone(), third.id.clone(), "hero-p2".to_string()]
        );

        instance_mut(&mut state, &third.id).granted_keywords.push(Keyword::Taunt);
        assert_eq!(names(&state, &attacker.id), vec![third.id.clone()]);

        instance_mut(&mut state, &attacker.id).exertion.attacked = true;
        assert!(targets_of(&state, &attacker.id).is_empty());
    }
}

#[test]
fn r13_refuses_a_dormant_card_under_a_stack_as_a_target_too_3_2() {
    let mut state = playing("dormant-target");
    let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
    let buried = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
    let mut top = new_instance(&mut state, &stacker.id, P2, Zone::Hand { player: P2 });
    assert!(place_on_field(&mut state, &mut top, slot(P2, Row::Units, 1), json_as(json!({ "stack": true }))));

    // The pile's top is a legal target; the card underneath is not (§3.2 "not targetable").
    assert_eq!(why(&state, &attacker.id, &unit(&state, &top.id)), None);
    assert_eq!(
        why(&state, &attacker.id, &unit(&state, &buried.id)),
        refused("that unit is not on the field")
    );
    assert!(!targets_of(&state, &attacker.id).iter().any(|target| matches!(
        target,
        AttackTarget::Unit { instance, .. } if instance.id == buried.id
    )));
}

mod attack_validation_through_the_action_layer_9_3_m2_t2 {
    use super::*;

    #[test]
    fn surfaces_each_refusal_as_the_error_of_an_attack_action() {
        let mut state = playing("attack-action-errors");
        let spent = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let defending = put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        let stuck = put(&mut state, &pacifist.id, slot(P1, Row::Units, 3), Default::default());
        let zero = put(&mut state, &zero_attack.id, slot(P1, Row::Units, 4), Default::default());
        let placed = put(&mut state, &plain.id, slot(P1, Row::Units, 5), Default::default());
        let sick = make_sick(&mut state, placed);
        let enemy = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        instance_mut(&mut state, &spent.id).exertion.attacked = true;
        instance_mut(&mut state, &defending.id).position = Some(Position::Def);

        let cases: Vec<(&str, &str)> = vec![
            (spent.id.as_str(), enemy.id.as_str()),
            (defending.id.as_str(), enemy.id.as_str()),
            (stuck.id.as_str(), enemy.id.as_str()),
            (zero.id.as_str(), enemy.id.as_str()),
            (sick.id.as_str(), enemy.id.as_str()),
            (sick.id.as_str(), "hero-p2"),
        ];
        for (attacker, target_id) in cases {
            let target = if target_id == "hero-p2" { on_hero(P2) } else { unit(&state, &enemy.id) };
            let expected = why(&state, attacker, &target);
            assert!(expected.is_some());
            assert_eq!(attack_via(&state, attacker, target_id).error, expected);
        }

        // The Taunt filter, through the same action.
        let mut other = playing("attack-action-taunt");
        let free = put(&mut other, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let bystander = put(&mut other, &plain.id, slot(P2, Row::Units, 1), Default::default());
        put(&mut other, &taunter.id, slot(P2, Row::Units, 2), Default::default());
        assert_eq!(
            why(&other, &free.id, &unit(&other, &bystander.id)),
            refused("a Taunt unit must be attacked first")
        );
        assert_eq!(
            attack_via(&other, &free.id, &bystander.id).error,
            refused("a Taunt unit must be attacked first")
        );

        // A friendly target never reaches the validator: the action layer looks it up among the
        // enemy's units and the enemy hero only.
        assert_eq!(attack_via(&other, &free.id, "hero-p1").error, refused("no target hero-p1"));
        assert_eq!(
            attack_via(&other, &free.id, &free.id).error,
            Some(format!("no target {}", free.id))
        );
    }

    #[test]
    fn resolves_a_legal_attack_and_refuses_the_same_attacker_twice_4_2_4_1() {
        let mut state = playing("attack-action-legal");
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let enemy = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        // The hero never strikes back (§4.3), so the attacker is still there for the second attempt.
        let first = attack_via(&state, &attacker.id, "hero-p2");
        assert_eq!(first.error, None);
        assert_eq!(events_of_type(&first.events, GameEventType::AttackDeclared).len(), 1);
        let forced = first.events.iter().find_map(|event| match event {
            GameEvent::AttackDeclared { forced, .. } => Some(*forced),
            _ => None,
        });
        assert_eq!(forced, Some(false));

        let again = attack_via(&first.state, &attacker.id, &enemy.id);
        assert_eq!(again.error, refused("that unit has already acted this turn"));
    }

    #[test]
    fn never_lists_an_attack_that_why_cannot_attack_refuses_and_lists_every_one_it_allows() {
        let mut state = playing("legal-actions-agree");
        let fresh = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let placed = put(&mut state, &rusher.id, slot(P1, Row::Units, 2), Default::default());
        make_sick(&mut state, placed);
        put(&mut state, &pacifist.id, slot(P1, Row::Units, 3), Default::default());
        put(&mut state, &zero_attack.id, slot(P1, Row::Units, 4), Default::default());
        let defending = put(&mut state, &plain.id, slot(P1, Row::Units, 5), Default::default());
        instance_mut(&mut state, &defending.id).position = Some(Position::Def);

        put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        put(&mut state, &taunter.id, slot(P2, Row::Units, 2), Default::default());
        let enemy_defending = put(&mut state, &plain.id, slot(P2, Row::Units, 3), Default::default());
        instance_mut(&mut state, &enemy_defending.id).position = Some(Position::Def);

        let listed: Vec<(String, String)> = legal_actions(&state, P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Attack { attacker_id, target_id } => Some((attacker_id, target_id)),
                _ => None,
            })
            .collect();
        assert!(!listed.is_empty());

        for (attacker_id, target_id) in &listed {
            let target = if target_id == "hero-p2" { on_hero(P2) } else { unit(&state, target_id) };
            assert_eq!(why(&state, attacker_id, &target), None);
        }

        // The other direction: every pair the validator allows is offered.
        let attackers: Vec<CardInstance> = (0..5)
            .filter_map(|lane| state.players.p1.units[lane].as_ref().and_then(|pile| pile.first()).cloned())
            .collect();
        let mut targets: Vec<AttackTarget> = (0..5)
            .filter_map(|lane| state.players.p2.units[lane].as_ref().and_then(|pile| pile.first()).map(on_unit))
            .collect();
        targets.push(on_hero(P2));
        let mut allowed: Vec<String> = attackers
            .iter()
            .flat_map(|attacker| {
                targets
                    .iter()
                    .filter(|target| can_attack(&state, attacker, target))
                    .map(|target| format!("{}->{}", attacker.id, target_name(target)))
                    .collect::<Vec<_>>()
            })
            .collect();
        allowed.sort();
        let mut offered: Vec<String> = listed
            .iter()
            .map(|(attacker_id, target_id)| format!("{attacker_id}->{target_id}"))
            .collect();
        offered.sort();
        assert_eq!(offered, allowed);

        // Only the Taunt units, and the sick Rush unit is among the attackers offered.
        let distinct: IndexSet<&String> = listed.iter().map(|(_, target_id)| target_id).collect();
        assert_eq!(distinct.len(), 2);
        assert!(listed.iter().any(|(attacker_id, _)| *attacker_id == fresh.id));
    }
}
