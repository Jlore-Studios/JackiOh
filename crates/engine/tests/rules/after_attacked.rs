//! "After this is attacked" (`Script.afterAttacked`): the defender's hook, run once the state check
//! that closes each combat it was the target of has run — a declared attack's (§4.2 step 5) and a
//! forced one's (R53) — also when it died there, then on the snapshot it fought with (R78, R89);
//! never for a Cleave splash (no attack on it) nor for an attack on the hero. The attacker's hook
//! runs first. Meditative #49.3 AI Girlfriend (MD-C26, R1026) is its user.

use jackioh_engine::combat::AFTER_ATTACKED_WORK;
use jackioh_engine::effects::forced_attacks_on;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    answer, cleave_veteran, defender, defender_asker, grunt, notes, playing, recorder, replays_to,
    round_trip, veteran,
};
use crate::rules::fixtures::harness::{put, sink_for, slot};

/// An `ActionInput` from its TS object literal (SURFACE §8: an object literal ports as `json!`).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

mod script_after_attacked {
    use super::*;

    #[test]
    fn r1026_a_declared_attack_runs_the_defenders_hook_after_the_check() {
        let mut state = playing("dc-attacked-declared");
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
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
        assert_eq!(
            notes(game.state()),
            vec![format!("attacked:{}:false:true:field", striker.id)]
        );
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r1026_a_defender_that_died_runs_it_on_its_snapshot() {
        let mut state = playing("dc-attacked-died");
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        by_id_mut(&mut state, &striker.id).buffs = AttackHealth { attack: 4, health: 0 };
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
        // The defender died in the combat; the hook ran on the snapshot it fought with, whose zone
        // is still the field — while the attacker that killed it survived.
        assert_eq!(
            notes(game.state()),
            vec![format!("attacked:{}:false:true:field", striker.id)]
        );
        assert!(
            game.state()
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.id == target.id)
        );
    }

    #[test]
    fn r1026_a_forced_attack_runs_it_flagged_forced() {
        let mut state = playing("dc-attacked-forced");
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(
            &mut state,
            &defender.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[forced_attacks_on(json_as(
                json!({ "target": { "of": "instance", "instanceId": target.id }, "attackers": "enemy" }),
            ))],
            &mut make_context(&mut sink, None, by(P2)),
        );
        assert_eq!(
            notes(sink.state),
            vec![format!("attacked:{}:true:true:field", striker.id)]
        );
    }

    #[test]
    fn r1026_cleave_splash_does_not_run_it() {
        let mut state = playing("dc-attacked-cleave");
        let striker = put(
            &mut state,
            &cleave_veteran.id,
            slot(P1, Row::Units, 2),
            Default::default(),
        );
        let middle = put(&mut state, &grunt.id, slot(P2, Row::Units, 2), Default::default());
        // Adjacent to the target: splashed by the Cleave, but never attacked.
        put(
            &mut state,
            &defender.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": middle.id, "playerId": "p1"
        })));
        // The attacker's hook ran for its combat; the splashed defender's never did: a Cleave
        // splash is damage, not an attack on it.
        assert!(
            notes(game.state())
                .iter()
                .all(|entry| !entry.starts_with("attacked:"))
        );
    }

    #[test]
    fn r1026_an_attack_on_the_hero_runs_nothing() {
        let mut state = playing("dc-attacked-hero");
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        put(
            &mut state,
            &defender.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": "hero-p2", "playerId": "p1"
        })));
        assert!(notes(game.state()).is_empty());
    }

    #[test]
    fn r1026_the_attackers_hook_runs_first() {
        let mut state = playing("dc-attacked-order");
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
        assert_eq!(
            notes(game.state()),
            vec![
                format!("after:{}::true:false:field", target.id),
                format!("attacked:{}:false:true:field", striker.id),
            ]
        );
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r1026_r113_a_question_in_it_survives_a_round_trip() {
        let mut state = playing("dc-attacked-pause");
        let striker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let target = put(
            &mut state,
            &defender_asker.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": target.id, "playerId": "p1"
        })));
        let paused = game.state().clone();
        assert_eq!(notes(&paused), vec!["attacked:before"]);
        assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(P2));
        assert_eq!(owed_work(&paused, Some(AFTER_ATTACKED_WORK)).len(), 1);
        let resumed = answer(&round_trip(&paused)).state;
        assert_eq!(
            notes(&resumed),
            vec![
                "attacked:before".to_string(),
                "attacked:answered:true".to_string(),
                "attacked:tail".to_string(),
            ]
        );
        assert!(resumed.work.is_empty());
        let pending = paused.pending.as_ref().expect("expected a prompt");
        game.play(input(json!({
            "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": "p2"
        })));
        assert!(replays_to(&game.start, &game.log, game.state()));
    }
}
