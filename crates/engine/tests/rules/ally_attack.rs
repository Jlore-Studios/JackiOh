//! "After one of your Units attacks" (`Script.allyAttack`, R2199000): run for each card on the attacker's
//! side once the state check closing each of its combats has run (§4.2 step 5; a forced one's, R53),
//! after the attacker's `afterAttack` and the defender's `afterAttacked` in R68's order; never for an
//! attack called off before it fought (R44), nor for the other side. Training Grounds #31 Level Up! uses it.

use jackioh_engine::combat::ALLY_ATTACK_WORK;
use jackioh_engine::effects::forced_attacks_on;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    ally_asker, ally_noter, answer, defender, grunt, notes, pawn, playing, recorder, replays_to, round_trip,
    veteran, wall,
};
use crate::rules::fixtures::harness::{put, sink_for, slot};

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

mod r2199000_ally_attack {
    use super::*;

    #[test]
    fn r2199000_a_declared_attack_runs_its_sides_hook_after_the_check() {
        let mut state = playing("ally-declared");
        put(
            &mut state,
            &ally_noter.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(&mut state, &wall.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": target.id, "playerId": "p1"
        })));
        assert_eq!(
            notes(game.state()),
            vec![format!("ally:{}:false:true:p1", striker.id)]
        );
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r2199000_it_runs_after_the_attackers_and_the_defenders_own_hooks() {
        let mut state = playing("ally-order");
        put(
            &mut state,
            &ally_noter.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let striker = put(
            &mut state,
            &veteran.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let target = put(
            &mut state,
            &defender.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": target.id, "playerId": "p1"
        })));
        let entries = notes(game.state());
        let kinds: Vec<&str> = entries
            .iter()
            .map(|entry| entry.split(':').next().unwrap())
            .collect();
        assert_eq!(kinds, vec!["after", "attacked", "ally"]);
    }

    #[test]
    fn r2199000_an_attacker_that_died_is_reported_as_not_surviving() {
        let mut state = playing("ally-died");
        put(
            &mut state,
            &ally_noter.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(
            &mut state,
            &defender.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        by_id_mut(&mut state, &target.id).buffs = AttackHealth { attack: 5, health: 0 };
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": target.id, "playerId": "p1"
        })));
        assert!(notes(game.state()).contains(&format!("ally:{}:false:false:p1", striker.id)));
    }

    #[test]
    fn r53_a_forced_attack_runs_it_flagged_forced() {
        let mut state = playing("ally-forced");
        put(
            &mut state,
            &ally_noter.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(&mut state, &wall.id, slot(P2, Row::Units, 1), Default::default());
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[forced_attacks_on(json_as(
                json!({ "target": { "of": "instance", "instanceId": target.id }, "attackers": "enemy" }),
            ))],
            &mut make_context(&mut sink, None, by(P2)),
        );
        assert_eq!(
            notes(sink.state),
            vec![format!("ally:{}:true:true:p1", striker.id)]
        );
    }

    #[test]
    fn r44_an_attack_called_off_before_it_fought_runs_nothing() {
        let mut state = playing("ally-cancelled");
        put(
            &mut state,
            &ally_noter.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        put(
            &mut state,
            &pawn.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(&mut state, &wall.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": target.id, "playerId": "p1"
        })));
        assert!(
            game.state()
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.def_id == pawn.id)
        );
        assert!(
            notes(game.state())
                .iter()
                .all(|entry| !entry.starts_with("ally:"))
        );
    }

    #[test]
    fn r2199000_the_other_sides_cards_never_answer_it() {
        let mut state = playing("ally-other-side");
        put(
            &mut state,
            &ally_noter.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(&mut state, &wall.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": target.id, "playerId": "p1"
        })));
        assert!(notes(game.state()).is_empty());
    }

    #[test]
    fn r2199000_r113_a_question_in_it_survives_a_round_trip() {
        let mut state = playing("ally-pause");
        put(
            &mut state,
            &ally_asker.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(&mut state, &wall.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": target.id, "playerId": "p1"
        })));
        let paused = game.state().clone();
        assert_eq!(notes(&paused), vec!["ally:before"]);
        assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(P1));
        assert_eq!(owed_work(&paused, Some(ALLY_ATTACK_WORK)).len(), 1);
        let resumed = answer(&round_trip(&paused)).state;
        assert_eq!(
            notes(&resumed),
            vec![
                "ally:before".to_string(),
                "ally:answered:true".to_string(),
                "ally:tail".to_string()
            ]
        );
        assert!(resumed.work.is_empty());
        let pending = paused.pending.as_ref().expect("expected a prompt");
        game.play(input(json!({
            "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": "p1"
        })));
        assert!(replays_to(&game.start, &game.log, game.state()));
    }
}
