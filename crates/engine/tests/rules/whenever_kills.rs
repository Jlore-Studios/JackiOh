//! "Whenever this destroys a Unit" (`TriggerDef.evenIfGone`, R2396000): a kill trigger that answers each
//! `destroyed` its card is R42's killer of though the card died in that combat or left the field since,
//! as the snapshot it killed with (`GameState.killSnapshots`), once, and for the player who controlled it
//! as it killed (R212). Meditative #95.1 CN Golem is its user; Classic+ #19.5 Bot Loser's and Training
//! Grounds #9 Ravager's "After this destroys a Unit" still ask that it stands (R1245).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    gone_killer, grunt, notes, playing, reborn_gone_killer, recorder, replays_to, turncoat,
};
use crate::rules::fixtures::harness::{act, put, slot};

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn in_graveyard(state: &GameState, player: PlayerId, id: &str) -> bool {
    state.players[player].graveyard.iter().any(|card| card.id == id)
}

mod r2396000_whenever_this_destroys_a_unit {
    use super::*;

    #[test]
    fn r2396000_a_killer_that_dies_in_the_trade_answers_its_kill() {
        let mut state = playing("dc-gone-trade");
        let killer = put(
            &mut state,
            &gone_killer.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let victim = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": killer.id, "targetId": victim.id, "playerId": "p1"
        })));
        assert!(in_graveyard(game.state(), P1, &killer.id));
        assert_eq!(notes(game.state()), vec![format!("kill:{}:p1", victim.id)]);
        assert!(game.state().kill_snapshots.is_none());
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r2396000_a_living_killer_answers_once() {
        let mut state = playing("dc-gone-lives");
        let killer = put(
            &mut state,
            &gone_killer.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        by_id_mut(&mut state, &killer.id).buffs = AttackHealth { attack: 0, health: 3 };
        let victim = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": killer.id, "targetId": victim.id, "playerId": "p1"
        })));
        assert!(!in_graveyard(game.state(), P1, &killer.id));
        assert_eq!(notes(game.state()), vec![format!("kill:{}:p1", victim.id)]);
        assert!(game.state().kill_snapshots.is_none());
    }

    #[test]
    fn r2396000_a_reborn_killer_answers_once() {
        let mut state = playing("dc-gone-reborn");
        let killer = put(
            &mut state,
            &reborn_gone_killer.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let victim = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": killer.id, "targetId": victim.id, "playerId": "p1"
        })));
        // Reborn brought a body back into its lane; the kill was answered once all the same.
        let back = game.state().players.p1.units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .map(|card| card.def_id.clone());
        assert_eq!(back, Some(reborn_gone_killer.id.clone()));
        assert_eq!(notes(game.state()), vec![format!("kill:{}:p1", victim.id)]);
        assert!(game.state().kill_snapshots.is_none());
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r2396000_r212_it_answers_for_the_player_who_controlled_it() {
        let mut state = playing("dc-gone-stolen");
        let killer = put(
            &mut state,
            &gone_killer.id,
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
            "type": "attack", "attackerId": killer.id, "targetId": victim.id, "playerId": "p1"
        })));
        // The turncoat's Death took the killer, which answers its kill for p1, whose it was.
        let taken = game
            .state()
            .players
            .p2
            .units
            .iter()
            .flatten()
            .any(|pile| pile.first().is_some_and(|card| card.id == killer.id));
        assert!(taken);
        assert_eq!(notes(game.state()), vec![format!("kill:{}:p1", victim.id)]);
        assert!(game.state().kill_snapshots.is_none());
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r2396000_a_defending_kill_counts() {
        let mut state = playing("dc-gone-defends");
        let killer = put(
            &mut state,
            &gone_killer.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let attacker = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(state.active, P2);
        let mut game = recorder(&state);
        game.play(input(json!({
            "type": "attack", "attackerId": attacker.id, "targetId": killer.id, "playerId": "p2"
        })));
        assert!(in_graveyard(game.state(), P1, &killer.id));
        assert_eq!(notes(game.state()), vec![format!("kill:{}:p1", attacker.id)]);
        assert!(game.state().kill_snapshots.is_none());
        assert!(replays_to(&game.start, &game.log, game.state()));
    }
}
