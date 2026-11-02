//! The autopilot (`subsystems/autopilot.rs`; Meditative+ #18 Cognitive Offloading, issue #1183,
//! docs/meditative-plus-set.md MP5): "the AI plays the rest of your turn", engaged on its caster's own turn
//! only and driven by `reduce` (R1183002), each action `choose`'s pure pick of state and player (R1183003),
//! each through `fixtures::outside_data` (#2273). Stinkyscape's autopilot aura (R1844000) is `ai_wheel.rs`'s.

use jackioh_engine::config::AUTOPILOT_MAX_ACTIONS;
use jackioh_engine::effects::autopilot;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::outside_data::{
    ask_enemy, offload, offload_on_draw, refill, register_outside_data, zap,
};

const FILLER: &str = "fx-10";

fn filler(count: usize) -> Value {
    json!(vec![FILLER; count])
}

mod r1183002_the_autopilot {
    use super::*;

    /// p1 to act on turn 9 with `hand` and a ready 2/2 in lane 1; p2 with an empty board and a card
    /// to play, so their turn does not end by itself.
    fn board(seed: &str, hand: Value) -> Scenario {
        register_outside_data();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "field": ["fx-3"], "library": filler(6), "mana": 4 },
            "p2": { "hand": ["fx-5"], "library": filler(6) },
        }))
    }

    #[test]
    fn r1183002_it_plays_the_rest_of_the_turn_and_ends_it() {
        let mut s = board("od-auto", json!([offload.id, "fx-1"]));
        let turn = s.state().turn;
        s.play(offload.id.as_str(), json!({}));
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::AutopilotEngaged { player: P1, turn: t } if *t == turn
        )));
        assert!(
            (1..=5).any(|lane| s.unit(P1, lane).is_some_and(|unit| unit.def_id == "fx-1")),
            "the hand's Unit was played"
        );
        s.expect_health("p2", HERO_HEALTH - 2);
        assert_eq!(s.state().active, P2, "the turn was ended");
        assert_eq!(
            s.state().players[P1].autopilot,
            None,
            "the status ended with the turn"
        );
    }

    #[test]
    fn r1183002_on_the_opponents_turn_it_does_nothing() {
        register_outside_data();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let mut events = Vec::new();
        {
            let state = s.state_mut();
            let (seed, cursor) = (state.seed.clone(), state.rng_cursor);
            let mut rng = Rng::new(&seed, cursor);
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..HookOptions::default()
                },
            );
            (autopilot().apply)(&mut ctx);
        }
        assert!(events.is_empty(), "no autopilot, no event");
        assert_eq!(s.state().players[P1].autopilot, None);
        assert_eq!(s.state().players[P2].autopilot, None);
    }

    #[test]
    fn r1183002_drawn_at_the_start_of_its_turn_it_plays_that_turn() {
        register_outside_data();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": ["fx-5"], "library": [offload_on_draw.id, FILLER, FILLER, FILLER] },
        }));
        s.end_turn();
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: P2, .. }))
        );
        assert!(
            (1..=5).any(|lane| s.unit(P2, lane).is_some_and(|unit| unit.def_id == "fx-5")),
            "the autopilot played p2's Unit"
        );
        assert_eq!(s.state().active, P1, "and ended p2's turn");
    }

    #[test]
    fn r1183002_a_searchs_step_leaves_the_turn_it_hands_the_autopilot_owed() {
        register_outside_data();
        let s = scenario(json!({
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": ["fx-5"], "library": [offload_on_draw.id, FILLER, FILLER, FILLER] },
        }));
        let end_turn = Action::new(ActionBody::EndTurn, P1, "search:end".to_string());
        let played = reduce(s.state(), &end_turn);
        let owed = reduce_unpiloted(s.state(), &end_turn);
        assert_eq!(
            played.state.active, P1,
            "a game's End turn plays p2's turn through"
        );
        assert!(owed.error.is_none());
        assert!(
            owed.events
                .iter()
                .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: P2, .. })),
            "the draw cast it and engaged the autopilot all the same"
        );
        assert_eq!(owed.state.active, P2, "the step stops where p2's turn begins");
        assert_eq!(
            subsystems::autopilot::due(&owed.state),
            Some(P2),
            "owed, not played"
        );
        assert!(
            owed.state.players[P2]
                .hand
                .iter()
                .any(|card| card.def_id == "fx-5"),
            "nothing was played for p2"
        );
    }

    #[test]
    fn r1183002_without_an_autopilot_a_searchs_step_is_reduces() {
        let s = board("od-unpiloted", json!(["fx-1"]));
        for body in legal_actions(s.state(), P1) {
            let action = Action::new(body, P1, "search:same".to_string());
            let (game, search) = (reduce(s.state(), &action), reduce_unpiloted(s.state(), &action));
            assert_eq!(hash_state(&search.state), hash_state(&game.state), "{action:?}");
            assert_eq!(search.events, game.events, "{action:?}");
            assert_eq!(search.error, game.error, "{action:?}");
        }
    }

    #[test]
    fn r1183002_the_seat_waits_locked_out_while_the_other_player_answers() {
        let mut s = board("od-auto-wait", json!([offload.id, ask_enemy.id]));
        s.play(offload.id.as_str(), json!({}));
        // The autopilot played the question for p2 and waits on its answer, still p1's turn.
        let pending = s.state().pending.clone().expect("p2's question is open");
        assert_eq!(pending.player_id, P2);
        assert_eq!(s.state().active, P1);
        assert!(s.state().players[P1].autopilot.is_some());
        // p1's own actions are refused meanwhile, its end of turn among them.
        s.expect_refused_with(|s| s.end_turn(), "prompt");
        assert_eq!(
            legal_actions(s.state(), P1)
                .iter()
                .filter(|action| action.action_type() != ActionType::Concede)
                .count(),
            0,
            "p1 may only concede"
        );
        // p2's answer hands the turn back to the autopilot, which ends it.
        s.answer(json!("left"));
        assert_eq!(s.state().active, P2);
        assert_eq!(s.state().players[P1].autopilot, None);
    }

    #[test]
    fn r1183002_the_status_is_absent_from_a_game_without_it() {
        register_outside_data();
        let s = scenario(json!({ "p1": { "hand": [FILLER] }, "p2": { "hand": [FILLER] } }));
        let text = serde_json::to_string(s.state()).expect("serializes");
        assert!(!text.contains("autopilot"), "D9: no key, no hash change");
    }

    #[test]
    fn r1183002_both_seats_read_the_autopilot_event() {
        let mut s = board("od-auto-view", json!([offload.id]));
        s.play(offload.id.as_str(), json!({}));
        for viewer in [P1, P2] {
            let events = serde_json::to_value(view_for(s.state(), viewer).events).expect("serializes");
            assert!(
                events.to_string().contains("\"autopilotEngaged\""),
                "{viewer:?} reads it"
            );
        }
    }
}

mod r1183003_what_the_autopilot_takes {
    use super::*;

    #[test]
    fn r1183003_its_own_prompt_is_answered_by_the_best_outcome() {
        register_outside_data();
        let mut s = scenario(json!({
            "seed": "od-zap",
            "p1": { "hand": [offload.id, zap.id], "library": filler(6), "mana": 4 },
            "p2": { "hand": ["fx-5"], "field": ["fx-6"], "library": filler(6) },
        }));
        s.play(offload.id.as_str(), json!({}));
        // 3 damage kills p2's only 2/2 (the enemy board cleared), before any other aim.
        assert!(s.unit(P2, 1).is_none(), "p2's Unit was the target");
        s.expect_health("p1", HERO_HEALTH);
        assert_eq!(s.state().active, P2);
    }

    #[test]
    fn r1183003_the_bound_hands_the_rest_of_the_turn_to_a_timeout() {
        register_outside_data();
        let mut s = scenario(json!({
            "seed": "od-bound",
            "p1": { "hand": [offload.id, refill.id], "library": filler(6) },
            "p2": { "hand": ["fx-5"], "library": filler(6) },
        }));
        s.play(offload.id.as_str(), json!({}));
        let refills = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if *def_id == refill.id))
            .count();
        assert_eq!(
            refills, AUTOPILOT_MAX_ACTIONS as usize,
            "one action each, up to the bound"
        );
        assert_eq!(s.state().active, P2, "then the turn ended");
    }

    #[test]
    fn r1183003_the_same_state_plays_the_same_turn() {
        let run = || {
            let mut s = super::autopilot_board("od-same");
            s.play(offload.id.as_str(), json!({}));
            (hash_state(s.state()), s.events().to_vec())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn r1183003_each_choice_is_a_legal_action() {
        let s = super::autopilot_board("od-legal");
        let mut state = s.state().clone();
        state.players[P1].autopilot = Some(Autopilot {
            turn: state.turn,
            actions: 0,
            aura_actions: None,
        });
        let chosen = jackioh_engine::subsystems::autopilot::choose(&state, P1).expect("an action");
        assert!(legal_actions(&state, P1).contains(&chosen));
        // It never concedes, activates, switches or emotes.
        assert!(matches!(
            chosen.action_type(),
            ActionType::Play | ActionType::Attack | ActionType::Answer | ActionType::EndTurn
        ));
    }
}

/// For determinism and legality: a Unit and a target prompt in hand, a ready Unit on each side.
fn autopilot_board(seed: &str) -> Scenario {
    register_outside_data();
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [offload.id, "fx-1", zap.id], "field": ["fx-3"], "library": filler(6), "mana": 4 },
        "p2": { "hand": ["fx-5"], "field": ["fx-6"], "library": filler(6) },
    }))
}
