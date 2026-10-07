//! "After this attacks" (`Script.afterAttack`): the attacker's hook, run once the state check that closes
//! each of its combats has run — a declared attack's (§4.2 step 5) and a forced one's (R53) — also when
//! it died there, then on the snapshot it fought with (R78, R89); never for an attack called off before
//! it fought (R44). Classic #13 Boots on the Ground, Classic+ #73.1 Classic Golem and Core #32 Prem
//! Panther (R426) are its users.
//!
//! Port of `packages/engine/test/after-attack.test.ts`.

use jackioh_engine::effects::forced_attacks_on;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    answer, cleave_veteran, death_asker, grunt, notes, pawn, playing, reborn_veteran, recorder, replays_to,
    round_trip, turncoat, veteran, veteran_asker, wall,
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

/// The id of the top card of a unit pile, if there is one.
fn top_id(pile: &Option<Pile>) -> Option<&str> {
    pile.as_ref()
        .and_then(|pile| pile.first())
        .map(|card| card.id.as_str())
}

mod script_after_attack {
    use super::*;

    #[test]
    fn a_declared_attack_the_hook_runs_once_the_check_has_closed_with_the_units_it_destroyed() {
        let mut state = playing("dc-after-declared");
        let striker = put(
            &mut state,
            &veteran.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        by_id_mut(&mut state, &striker.id).buffs = AttackHealth { attack: 1, health: 3 };
        let victim = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": victim.id, "playerId": "p1"
        })));
        assert_eq!(
            notes(game.state()),
            vec![format!("after:{}:{}:true:false:field", victim.id, victim.id)]
        );
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn an_attacker_that_died_in_its_combat_runs_the_hook_on_the_snapshot_it_fought_with() {
        let mut state = playing("dc-after-died");
        let striker = put(
            &mut state,
            &veteran.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let blocker = put(&mut state, &wall.id, slot(P2, Row::Units, 1), Default::default());
        by_id_mut(&mut state, &blocker.id).buffs = AttackHealth { attack: 4, health: 0 };
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": blocker.id, "playerId": "p1"
        })));
        assert_eq!(
            notes(game.state()),
            vec![format!("after:{}::false:false:field", blocker.id)]
        );
        let graveyard: Vec<&str> = game
            .state()
            .players
            .p1
            .graveyard
            .iter()
            .map(|card| card.id.as_str())
            .collect();
        assert!(graveyard.contains(&striker.id.as_str()));
    }

    #[test]
    fn a_forced_attack_runs_it_too_flagged_forced_an_attack_called_off_before_it_fought_does_not() {
        let mut state = playing("dc-after-forced");
        put(
            &mut state,
            &veteran.id,
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
        assert_eq!(notes(sink.state), vec!["after:hero-p2::true:true:field"]);

        let mut cancelled = playing("dc-after-cancelled");
        let idle = put(
            &mut cancelled,
            &veteran.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut cancelled,
            &pawn.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        let mut game = recorder(&cancelled);
        game.play(input(json!({
            "type": "attack", "attackerId": idle.id, "targetId": "hero-p2", "playerId": "p1"
        })));
        assert!(notes(game.state()).is_empty());
        assert_eq!(game.state().players.p2.hero.health, 30);
    }

    #[test]
    fn r113_a_question_inside_the_hook_pauses_it_the_rest_and_the_check_after_it_survive_a_round_trip() {
        let mut state = playing("dc-after-pause");
        let striker = put(
            &mut state,
            &veteran_asker.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": "hero-p2", "playerId": "p1"
        })));
        let paused = game.state().clone();
        assert_eq!(notes(&paused), vec!["after:before"]);
        assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(P1));
        assert_eq!(owed_work(&paused, Some(AFTER_ATTACK_WORK)).len(), 1);
        let resumed = answer(&round_trip(&paused)).state;
        assert_eq!(
            notes(&resumed),
            vec!["after:before", "after:answered:true", "after:tail"]
        );
        assert!(resumed.work.is_empty());
        let pending = paused.pending.as_ref().expect("expected a prompt");
        game.play(input(json!({
            "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": "p1"
        })));
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r426_cleave_the_hook_lists_every_unit_the_attack_destroyed_the_cleaves_kills_with_the_defenders() {
        let mut state = playing("dc-after-cleave");
        let striker = put(
            &mut state,
            &cleave_veteran.id,
            slot(P1, Row::Units, 2),
            Default::default(),
        );
        let lanes: Vec<String> = [1, 2, 3]
            .into_iter()
            .map(|lane| {
                put(
                    &mut state,
                    &grunt.id,
                    slot(P2, Row::Units, lane),
                    Default::default(),
                )
                .id
            })
            .collect();
        let (left, middle, right) = (&lanes[0], &lanes[1], &lanes[2]);
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": middle, "playerId": "p1"
        })));
        let entry = notes(game.state()).first().cloned().unwrap_or_default();
        let mut destroyed: Vec<String> = entry
            .split(':')
            .nth(1)
            .unwrap_or_default()
            .split('+')
            .map(String::from)
            .collect();
        destroyed.sort();
        let mut expected = vec![left.clone(), middle.clone(), right.clone()];
        expected.sort();
        assert_eq!(destroyed, expected);
        assert!(entry.ends_with(":true:p1"));
    }

    #[test]
    fn r426_the_hook_acts_for_the_player_who_attacked_a_death_in_the_check_that_takes_the_attacker_leaves_it_theirs()
     {
        let mut state = playing("dc-after-stolen");
        let striker = put(
            &mut state,
            &cleave_veteran.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let victim = put(
            &mut state,
            &turncoat.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": victim.id, "playerId": "p1"
        })));
        // The turncoat's Death took the attacker; it survived on the stay it attacked from, and p1 attacked.
        assert!(
            game.state()
                .players
                .p2
                .units
                .iter()
                .any(|pile| top_id(pile) == Some(striker.id.as_str()))
        );
        assert_eq!(notes(game.state()), vec![format!("after:{}:true:p1", victim.id)]);
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r426_a_reborn_body_is_a_new_arrival_the_attacker_that_died_in_the_combat_did_not_survive_it() {
        let mut state = playing("dc-after-reborn");
        let striker = put(
            &mut state,
            &reborn_veteran.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let victim = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": victim.id, "playerId": "p1"
        })));
        let top = game.state().players.p1.units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .map(|card| card.def_id.clone());
        assert_eq!(top, Some(reborn_veteran.id.clone()));
        assert_eq!(notes(game.state()), vec![format!("after:{}:false:p1", victim.id)]);
    }

    #[test]
    fn r113_a_deaths_question_in_the_check_before_it_owes_the_hook_behind_the_death() {
        let mut state = playing("dc-after-death-asks");
        let striker = put(
            &mut state,
            &veteran.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let doomed = put(
            &mut state,
            &death_asker.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": striker.id, "targetId": doomed.id, "playerId": "p1"
        })));
        let paused = game.state().clone();
        assert_eq!(notes(&paused), vec!["death:ask"]);
        assert_eq!(owed_work(&paused, Some(AFTER_ATTACK_WORK)).len(), 1);
        let resumed = answer(&round_trip(&paused)).state;
        // The Death finishes, then the hook — with the kill the check made, and its attacker alive.
        assert_eq!(
            notes(&resumed),
            vec![
                "death:ask".to_string(),
                "death:answered".to_string(),
                format!("after:{}:{}:true:false:field", doomed.id, doomed.id),
            ]
        );
        assert!(resumed.work.is_empty());
    }
}
