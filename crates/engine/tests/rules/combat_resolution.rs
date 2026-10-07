//! Combat resolution (BUILD M2-T4): the First Strike step, the simultaneous step, hero targets and
//! forced attacks. SPEC §4.2's forced-attack paragraph, §4.3, §4.5, R53 and R59.
//!
//! Port of `packages/engine/test/combat-resolution.test.ts`.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_body, first_striker, moths, plain, taunter};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, sink_for, slot};

/// p1's main phase on turn 4, so nothing placed with `put` is summoning sick (§4.1).
fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

fn on_unit(instance: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: instance.clone(),
    }
}

fn graveyard_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].graveyard.iter().map(|card| card.id.clone()).collect()
}

fn field_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    active_units_of(state, player).iter().map(|unit| unit.id.clone()).collect()
}

/// The refusal a check gives, or `None` when it allows (TS `string | null`, `{ error?: string }`).
fn refusal(check: Result<(), EngineError>) -> Option<String> {
    check.err().map(|error| error.message)
}

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// The card as it stands in `state` now, owned (TS held the live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    by_id(state, id).clone()
}

/// Each `damage` event as `(sourceId, targetId, amount, combat)`.
fn hits(events: &[GameEvent]) -> Vec<(Option<String>, String, i32, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                combat,
            } => Some((source_id.clone(), target_id.clone(), *amount, *combat)),
            _ => None,
        })
        .collect()
}

fn attack_declared(events: &[GameEvent]) -> Vec<(String, String, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AttackDeclared {
                attacker_id,
                target_id,
                forced,
            } => Some((attacker_id.clone(), target_id.clone(), *forced)),
            _ => None,
        })
        .collect()
}

/// TS `sinkFor(state, events)`: run `f` on a sink over `state` and hand back what it emitted.
fn run<R>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> (R, Vec<GameEvent>) {
    let mut sink = sink_for(state);
    let out = f(&mut sink);
    let events = sink.events.clone();
    (out, events)
}

/// `declareAttack(sink, attacker, target)` with the attacker and a unit target read live off the sink.
fn declare(sink: &mut EngineSink<'_>, attacker_id: &str, target: Option<&str>) -> Option<String> {
    let attacker = live(sink.state, attacker_id);
    let target = match target {
        Some(id) => on_unit(by_id(sink.state, id)),
        None => AttackTarget::Hero { player: P2 },
    };
    refusal(declare_attack(sink, &attacker, &target))
}

/// `forceAttack(sink, attacker, onUnit(target))` with both read live off the sink.
fn force(sink: &mut EngineSink<'_>, attacker_id: &str, target_id: &str) {
    let attacker = live(sink.state, attacker_id);
    let target = on_unit(by_id(sink.state, target_id));
    force_attack(sink, &attacker, &target);
}

fn exertion(attacked: bool, switched: bool) -> Exertion {
    Exertion {
        attacked,
        switched,
        attacks: None,
    }
}

mod combat_resolution_m2_t4 {
    use super::*;

    #[test]
    fn a_first_strike_attacker_survives_a_defender_it_kills_4_3() {
        let mut state = board("first-strike-kill");
        let attacker = put(&mut state, &first_striker().id, slot(P1, Row::Units, 1), Default::default()); // 4/4 First Strike
        let defender = put(&mut state, &plain().id, slot(P2, Row::Units, 1), Default::default()); // 3/3

        let (refused, events) = run(&mut state, |sink| declare(sink, &attacker.id, Some(&defender.id)));
        assert_eq!(refused, None);

        // "If D is destroyed here it deals nothing": the whole combat is one hit.
        let hits = hits(&events);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0], (Some(attacker.id.clone()), defender.id.clone(), 4, true));

        assert_eq!(graveyard_ids(&state, P2), vec![defender.id.clone()]);
        assert_eq!(field_ids(&state, P1), vec![attacker.id.clone()]);
        assert_eq!(by_id(&state, &attacker.id).damage, 0);
        assert_eq!(unit_view(&state, by_id(&state, &attacker.id)).health, 4);
    }

    #[test]
    fn two_first_strikers_strike_simultaneously_in_step_1_and_both_die_4_3() {
        let mut state = board("first-strike-trade");
        let attacker = put(&mut state, &first_striker().id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(&mut state, &first_striker().id, slot(P2, Row::Units, 1), Default::default());

        let (refused, events) = run(&mut state, |sink| declare(sink, &attacker.id, Some(&defender.id)));
        assert_eq!(refused, None);

        assert_eq!(
            hits(&events),
            vec![
                (Some(attacker.id.clone()), defender.id.clone(), 4, true),
                (Some(defender.id.clone()), attacker.id.clone(), 4, true),
            ]
        );

        assert_eq!(graveyard_ids(&state, P1), vec![attacker.id.clone()]);
        assert_eq!(graveyard_ids(&state, P2), vec![defender.id.clone()]);
        assert!(field_ids(&state, P1).is_empty());
        assert!(field_ids(&state, P2).is_empty());
    }

    #[test]
    fn a_defender_in_defense_position_strikes_back_at_full_attack_4_3() {
        let mut state = board("defense-strikes-back");
        let attacker = put(&mut state, &plain().id, slot(P1, Row::Units, 1), Default::default()); // 3/3
        let defender = put(&mut state, &big_body().id, slot(P2, Row::Units, 1), Default::default()); // 5/10
        let (switched, _) = run(&mut state, |sink| {
            let now = live(sink.state, &defender.id);
            refusal(switch_position(
                sink,
                &now,
                SwitchPositionOptions {
                    spend_exertion: Some(false),
                    to: Some(Position::Def),
                },
            ))
        });
        assert_eq!(switched, None);

        // Defense grants Taunt and Armor +1 but never lowers the attack it strikes back with (§4.1).
        let defender_view = unit_view(&state, by_id(&state, &defender.id));
        assert_eq!(defender_view.position, Position::Def);
        assert_eq!(defender_view.attack, 5);
        assert_eq!(defender_view.armor, 1);

        let (refused, events) = run(&mut state, |sink| declare(sink, &attacker.id, Some(&defender.id)));
        assert_eq!(refused, None);

        let strike_back = hits(&events)
            .into_iter()
            .find(|(source, ..)| source.as_deref() == Some(defender.id.as_str()));
        let strike_back = strike_back.map(|(_, target, amount, combat)| (target, amount, combat));
        assert_eq!(strike_back, Some((attacker.id.clone(), 5, true)));

        assert_eq!(by_id(&state, &defender.id).damage, 2); // the attacker's 3 less the Defense Armor
        assert_eq!(graveyard_ids(&state, P1), vec![attacker.id.clone()]);
        assert_eq!(field_ids(&state, P2), vec![defender.id.clone()]);
    }

    #[test]
    fn a_hero_never_strikes_back_4_3() {
        let mut state = board("hero-target");
        let attacker = put(&mut state, &big_body().id, slot(P1, Row::Units, 1), Default::default()); // 5/10
        let enemy_hero = state.players.p2.hero.health;
        let own_hero = state.players.p1.hero.health;

        let (refused, events) = run(&mut state, |sink| declare(sink, &attacker.id, None));
        assert_eq!(refused, None);

        let hits = hits(&events);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0], (Some(attacker.id.clone()), "hero-p2".to_string(), 5, true));

        assert_eq!(state.players.p2.hero.health, enemy_hero - 5);
        assert_eq!(state.players.p1.hero.health, own_hero);
        assert_eq!(by_id(&state, &attacker.id).damage, 0);
        assert_eq!(field_ids(&state, P1), vec![attacker.id.clone()]);
    }

    #[test]
    fn r59_both_units_of_one_combat_die_together_the_first_death_does_not_cancel_the_exchange() {
        let mut state = board("r59-mutual-kill");
        let attacker = put(&mut state, &plain().id, slot(P1, Row::Units, 1), Default::default()); // 3/3
        let defender = put(&mut state, &plain().id, slot(P2, Row::Units, 1), Default::default()); // 3/3

        let (refused, events) = run(&mut state, |sink| declare(sink, &attacker.id, Some(&defender.id)));
        assert_eq!(refused, None);

        // The state check never runs between the two hits of one combat (§4.5, R59), so both deaths
        // land after both hits rather than the attacker's kill cancelling the strike back.
        let kinds: Vec<GameEventType> = events
            .iter()
            .map(|event| event.event_type())
            .filter(|kind| *kind == GameEventType::Damage || *kind == GameEventType::Destroyed)
            .collect();
        assert_eq!(
            kinds,
            vec![
                GameEventType::Damage,
                GameEventType::Damage,
                GameEventType::Destroyed,
                GameEventType::Destroyed,
            ]
        );
        let amounts: Vec<i32> = hits(&events).iter().map(|hit| hit.2).collect();
        assert_eq!(amounts, vec![3, 3]);

        assert_eq!(graveyard_ids(&state, P1), vec![attacker.id.clone()]);
        assert_eq!(graveyard_ids(&state, P2), vec![defender.id.clone()]);
    }
}

#[test]
fn a_defender_with_first_strike_hits_first_and_a_surviving_attacker_strikes_back_4_3() {
    let mut state = board("defender-first-strike");
    let attacker = put(&mut state, &big_body().id, slot(P1, Row::Units, 1), Default::default()); // 5/10, no First Strike
    let defender = put(&mut state, &first_striker().id, slot(P2, Row::Units, 1), Default::default()); // 4/4 First Strike

    let (_, events) = run(&mut state, |sink| declare(sink, &attacker.id, Some(&defender.id)));

    // The defender struck first for 4; the attacker survived on 10 health and struck back for 5.
    let damage = hits(&events);
    let amounts: Vec<i32> = damage.iter().map(|hit| hit.2).collect();
    assert_eq!(amounts, vec![4, 5]);
    assert_eq!(damage.first().and_then(|hit| hit.0.clone()), Some(defender.id.clone()));
    assert_eq!(damage.get(1).and_then(|hit| hit.0.clone()), Some(attacker.id.clone()));
    assert_eq!(unit_view(&state, by_id(&state, &attacker.id)).health, 6);
    assert_eq!(graveyard_ids(&state, P2), vec![defender.id.clone()]);
}

#[test]
fn a_defender_with_first_strike_that_kills_the_attacker_takes_nothing_back_4_3() {
    let mut state = board("defender-first-strike-kill");
    let attacker = put(&mut state, &plain().id, slot(P1, Row::Units, 1), Default::default()); // 3/3
    let defender = put(&mut state, &first_striker().id, slot(P2, Row::Units, 1), Default::default()); // 4/4 First Strike

    let (_, events) = run(&mut state, |sink| declare(sink, &attacker.id, Some(&defender.id)));

    let amounts: Vec<i32> = hits(&events).iter().map(|hit| hit.2).collect();
    assert_eq!(amounts, vec![4]);
    assert_eq!(graveyard_ids(&state, P1), vec![attacker.id.clone()]);
    assert_eq!(unit_view(&state, by_id(&state, &defender.id)).health, 4);
}

mod r53_forced_attacks_m2_t4 {
    use super::*;

    #[test]
    fn r53_moths_pulls_a_summoning_sick_enemy_into_an_attack_without_spending_its_exertion() {
        let mut state = board("moths-sick");
        let flame = put(&mut state, &moths().id, slot(P1, Row::Units, 1), Default::default()); // 1/14
        let sick = put(&mut state, &plain().id, slot(P2, Row::Units, 1), Default::default()); // 3/3
        let turn = state.turn;
        by_id_mut(&mut state, &sick.id).summoned_turn = Some(turn);

        assert!(is_sick(&state, by_id(&state, &sick.id)));
        assert_eq!(
            refusal(why_cannot_attack(&state, by_id(&state, &sick.id), &on_unit(by_id(&state, &flame.id)))),
            Some("that unit is summoning sick".to_string())
        );

        let (_, events) = run(&mut state, |sink| force(sink, &sick.id, &flame.id));

        assert_eq!(
            serde_json::to_value(events_of_type(&events, GameEventType::AttackDeclared)).expect("serialises"),
            json!([{ "type": "attackDeclared", "attackerId": sick.id, "targetId": flame.id, "forced": true }])
        );
        assert_eq!(by_id(&state, &flame.id).damage, 3);
        assert_eq!(by_id(&state, &sick.id).damage, 1);

        // No exertion spent: the unit is still free to act on its own turn (§4.2, R53).
        assert_eq!(by_id(&state, &sick.id).exertion, exertion(false, false));
        assert!(has_exertion(&state, by_id(&state, &sick.id), ExertionKind::Attack));
    }

    #[test]
    fn r53_a_forced_attack_ignores_position_sickness_and_the_taunt_rule() {
        let mut state = board("forced-ignores-rules");
        let guard = put(&mut state, &taunter().id, slot(P1, Row::Units, 1), Default::default()); // 2/5 Taunt
        let flame = put(&mut state, &moths().id, slot(P1, Row::Units, 2), Default::default()); // 1/14
        let defending = put(&mut state, &plain().id, slot(P2, Row::Units, 1), Default::default());
        let sick = put(&mut state, &plain().id, slot(P2, Row::Units, 2), Default::default());
        let blocked = put(&mut state, &plain().id, slot(P2, Row::Units, 3), Default::default());
        let (switched, _) = run(&mut state, |sink| {
            let now = live(sink.state, &defending.id);
            refusal(switch_position(
                sink,
                &now,
                SwitchPositionOptions {
                    spend_exertion: Some(false),
                    to: Some(Position::Def),
                },
            ))
        });
        assert_eq!(switched, None);
        let turn = state.turn;
        by_id_mut(&mut state, &sick.id).summoned_turn = Some(turn);

        // Every one of the three is refused by §4.2 steps 1 to 3.
        assert!(
            unit_view(&state, by_id(&state, &guard.id))
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Taunt)
        );
        let target = on_unit(by_id(&state, &flame.id));
        assert_eq!(
            refusal(why_cannot_attack(&state, by_id(&state, &defending.id), &target)),
            Some("only Attack-Position units may attack".to_string())
        );
        assert_eq!(
            refusal(why_cannot_attack(&state, by_id(&state, &sick.id), &target)),
            Some("that unit is summoning sick".to_string())
        );
        assert_eq!(
            refusal(why_cannot_attack(&state, by_id(&state, &blocked.id), &target)),
            Some("a Taunt unit must be attacked first".to_string())
        );

        let (_, events) = run(&mut state, |sink| {
            force(sink, &defending.id, &flame.id);
            force(sink, &sick.id, &flame.id);
            force(sink, &blocked.id, &flame.id);
        });

        let declared = attack_declared(&events);
        let attackers: Vec<String> = declared.iter().map(|(attacker, ..)| attacker.clone()).collect();
        assert_eq!(attackers, vec![defending.id.clone(), sick.id.clone(), blocked.id.clone()]);
        assert!(declared.iter().all(|(.., forced)| *forced));

        // All three struck for their full 3, the Defense-Position one included.
        assert_eq!(by_id(&state, &flame.id).damage, 9);
        assert_eq!(by_id(&state, &defending.id).damage, 0); // Moths' 1 is eaten by the Defense Armor
        assert_eq!(by_id(&state, &sick.id).damage, 1);
        assert_eq!(by_id(&state, &blocked.id).damage, 1);
    }

    #[test]
    fn r53_an_ordinary_attack_spends_the_attackers_exertion_and_a_forced_one_does_not() {
        let mut state = board("forced-exertion");
        let ordinary = put(&mut state, &big_body().id, slot(P1, Row::Units, 1), Default::default()); // 5/10
        let forced = put(&mut state, &big_body().id, slot(P1, Row::Units, 2), Default::default()); // 5/10
        let first = put(&mut state, &plain().id, slot(P2, Row::Units, 1), Default::default());
        let second = put(&mut state, &plain().id, slot(P2, Row::Units, 2), Default::default());
        let third = put(&mut state, &plain().id, slot(P2, Row::Units, 3), Default::default());

        let mut sink = sink_for(&mut state);
        assert_eq!(declare(&mut sink, &ordinary.id, Some(&first.id)), None);
        assert!(by_id(sink.state, &ordinary.id).exertion.attacked);
        assert!(!has_exertion(sink.state, by_id(sink.state, &ordinary.id), ExertionKind::Attack));
        assert_eq!(
            declare(&mut sink, &ordinary.id, Some(&second.id)),
            Some("that unit has already acted this turn".to_string())
        );

        force(&mut sink, &forced.id, &second.id);
        assert_eq!(by_id(sink.state, &forced.id).exertion, exertion(false, false));
        assert!(has_exertion(sink.state, by_id(sink.state, &forced.id), ExertionKind::Attack));

        // The exertion is still there, so the same unit can still make its own attack this turn.
        assert_eq!(declare(&mut sink, &forced.id, Some(&third.id)), None);
        assert!(by_id(sink.state, &forced.id).exertion.attacked);
        assert_eq!(
            graveyard_ids(sink.state, P2),
            vec![first.id.clone(), second.id.clone(), third.id.clone()]
        );
    }

    #[test]
    fn r53_each_forced_attack_is_its_own_combat_followed_by_its_own_state_check() {
        let mut state = board("forced-own-combat");
        let target = put(&mut state, &big_body().id, slot(P1, Row::Units, 1), Default::default()); // 5/10
        let first = put(&mut state, &plain().id, slot(P2, Row::Units, 1), Default::default()); // 3/3
        let second = put(&mut state, &plain().id, slot(P2, Row::Units, 2), Default::default()); // 3/3

        let attackers = vec![live(&state, &first.id), live(&state, &second.id)];
        let on_target = on_unit(by_id(&state, &target.id));
        let (_, events) = run(&mut state, |sink| force_attacks_on(sink, &attackers, &on_target, None));

        // A state check between the two combats, so the first attacker is already dead and buried
        // when the second attack is declared (§4.5, R53).
        let kinds: Vec<GameEventType> = events
            .iter()
            .map(|event| event.event_type())
            .filter(|kind| *kind == GameEventType::AttackDeclared || *kind == GameEventType::Destroyed)
            .collect();
        assert_eq!(
            kinds,
            vec![
                GameEventType::AttackDeclared,
                GameEventType::Destroyed,
                GameEventType::AttackDeclared,
                GameEventType::Destroyed,
            ]
        );

        assert_eq!(graveyard_ids(&state, P2), vec![first.id.clone(), second.id.clone()]);
        assert_eq!(by_id(&state, &target.id).damage, 6);
        assert_eq!(unit_view(&state, by_id(&state, &target.id)).health, 4);
    }

    #[test]
    fn r53_a_sequence_of_forced_attackers_stops_once_the_target_is_gone() {
        let mut state = board("forced-stops-when-gone");
        let target = put(&mut state, &taunter().id, slot(P1, Row::Units, 1), Default::default()); // 2/5
        let first = put(&mut state, &plain().id, slot(P2, Row::Units, 1), Default::default());
        let second = put(&mut state, &plain().id, slot(P2, Row::Units, 2), Default::default());
        let third = put(&mut state, &plain().id, slot(P2, Row::Units, 3), Default::default());

        let attackers: Vec<CardInstance> = active_units_of(&state, P2).iter().map(|unit| (*unit).clone()).collect();
        let on_target = on_unit(by_id(&state, &target.id));
        let (_, events) = run(&mut state, |sink| force_attacks_on(sink, &attackers, &on_target, None));

        // Two 3-attack hits finish a 5-health target, so the third attacker is never pulled in.
        let declared: Vec<String> = attack_declared(&events).into_iter().map(|(attacker, ..)| attacker).collect();
        assert_eq!(declared, vec![first.id.clone(), second.id.clone()]);
        assert_eq!(graveyard_ids(&state, P1), vec![target.id.clone()]);
        assert_eq!(by_id(&state, &third.id).damage, 0);
        assert_eq!(
            field_ids(&state, P2),
            vec![first.id.clone(), second.id.clone(), third.id.clone()]
        );
        assert_eq!(
            vec![by_id(&state, &first.id).damage, by_id(&state, &second.id).damage],
            vec![2, 2]
        );
    }
}
