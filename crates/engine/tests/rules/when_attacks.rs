//! "When this attacks" (`Script.whenAttacks`, R2285000): the attacker's hook, run before any damage of
//! each attack it makes — a declared attack's once its trap window has closed (§4.2 step 4), a forced
//! one's before its combat (R53) — as a whole effect list followed by its state check, parkable like any
//! hook (R113). An attack whose attacker left the field in it does not fight (R220), and one whose
//! target it transformed fights the Unit that took the target's place (R2285002). Classic #13 Boots on
//! the Ground (R2396000), Classic #45 Nature Titan, Classic+ #73.1 Classic Golem and Stinkyscape #75
//! Badger are its users.

use jackioh_engine::effects::forced_attacks_on;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    answer, grunt, kamikaze, notes, pawn, playing, recorder, replays_to, round_trip, shifter, striker,
    striker_asker, wall,
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

mod script_when_attacks {
    use super::*;

    #[test]
    fn r2285000_a_declared_attack_runs_the_hook_before_any_damage() {
        let mut state = playing("dc-when-declared");
        let attacker = put(
            &mut state,
            &striker.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1"
        })));
        // The hook read the enemy hero at 30, before the striker's 2 landed.
        assert_eq!(notes(game.state()), vec!["when:hero-p2:false:30"]);
        assert_eq!(game.state().players.p2.hero.health, 28);
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r2285000_r53_a_forced_attack_runs_it_too_flagged_forced() {
        let mut state = playing("dc-when-forced");
        put(
            &mut state,
            &striker.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[forced_attacks_on(json_as(
                json!({ "target": { "of": "enemyHero" }, "attackers": "self" }),
            ))],
            &mut make_context(&mut sink, None, by(P1)),
        );
        assert_eq!(notes(sink.state), vec!["when:hero-p2:true:30"]);
        assert_eq!(sink.state.players.p2.hero.health, 28);
    }

    #[test]
    fn r2285000_r44_an_attack_called_off_in_its_trap_window_runs_no_hook() {
        let mut state = playing("dc-when-cancelled");
        let attacker = put(
            &mut state,
            &striker.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut state,
            &pawn.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1"
        })));
        assert!(notes(game.state()).is_empty());
        assert_eq!(game.state().players.p2.hero.health, 30);
    }

    #[test]
    fn r2285000_defending_runs_nothing_the_hook_is_the_attackers_alone() {
        let mut state = playing("dc-when-defends");
        let attacker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(
            &mut state,
            &striker.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": defender.id, "playerId": "p1"
        })));
        assert!(notes(game.state()).is_empty());
    }

    #[test]
    fn r2285000_r220_an_attacker_its_own_hook_destroyed_does_not_fight() {
        let mut state = playing("dc-when-gone");
        let attacker = put(
            &mut state,
            &kamikaze.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let target = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": target.id, "playerId": "p1"
        })));
        let state = game.state();
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.id == attacker.id)
        );
        assert_eq!(find_instance(state, &target.id).map(|card| card.damage), Some(0));
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r2285000_r113_a_question_inside_the_hook_pauses_the_attack_and_the_rest_and_the_combat_survive_a_round_trip()
     {
        let mut state = playing("dc-when-pause");
        let attacker = put(
            &mut state,
            &striker_asker.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1"
        })));
        let paused = game.state().clone();
        assert_eq!(notes(&paused), vec!["when:before"]);
        assert_eq!(owed_work(&paused, Some(WHEN_ATTACKS_WORK)).len(), 1);
        assert_eq!(
            paused.players.p2.hero.health, 30,
            "no damage before the hook is done"
        );
        let resumed = answer(&round_trip(&paused)).state;
        assert_eq!(
            notes(&resumed),
            vec!["when:before", "when:answered:false", "when:tail"]
        );
        assert_eq!(resumed.players.p2.hero.health, 28);
        assert!(resumed.work.is_empty());
        let pending = paused.pending.as_ref().expect("expected a prompt");
        game.play(input(json!({
            "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": "p1"
        })));
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r2285002_a_target_the_hook_transforms_is_fought_as_the_unit_that_took_its_place() {
        let mut state = playing("dc-when-transform");
        let attacker = put(
            &mut state,
            &shifter.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let target = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": target.id, "playerId": "p1"
        })));
        let state = game.state();
        let wall_now = state.players.p2.units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned()
            .expect("the wall stands where the grunt stood");
        assert_eq!(wall_now.def_id, wall.id);
        assert_eq!(wall_now.damage, 3);
        assert_eq!(
            find_instance(state, &attacker.id).map(|card| card.damage),
            Some(1)
        );
        assert!(replays_to(&game.start, &game.log, game.state()));
    }
}
