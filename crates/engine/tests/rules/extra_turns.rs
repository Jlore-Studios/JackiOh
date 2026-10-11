//! ME-TURN's lost refreshes and extra turns (SPEC §2.2, §2.3, R844–R847): Meditative #18,
//! #19 and #19.1's systems, proved here through fixture-free effects so the engine owns them.
//!
//! Port of nothing shipped: no shipped card loses a refresh or owes a turn.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{gain_mana, lose_refreshes, next_turn_mana, take_extra_turn};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::new_game;

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act_full(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    reduce(state, &input.with_nonce(format!("extra{nonce}")))
}

fn act(state: &GameState, body: Value) -> GameState {
    let result = act_full(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// Past both mulligans, in p1's main phase on turn 1, with no automatic turn end on either seat
/// (R82 would otherwise run a zero-mana turn on by itself).
fn playing(seed: &str) -> GameState {
    let decks = (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21));
    let mut state = begin_game(&new_game(&format!("extra-turns-{seed}"), Some(decks))).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": player }),
        );
    }
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

/// Apply one effect the way `resolve.ts` does, and hand back the events it emitted.
fn run(state: &mut GameState, effect: Effect, options: HookOptions) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, options);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn as_p1() -> HookOptions {
    HookOptions {
        controller: Some(PlayerId::P1),
        ..Default::default()
    }
}

fn as_p2() -> HookOptions {
    HookOptions {
        controller: Some(PlayerId::P2),
        ..Default::default()
    }
}

fn lose(state: &mut GameState, turns: i32, options: HookOptions) -> Vec<GameEvent> {
    run(state, lose_refreshes(json_as(json!({ "turns": turns }))), options)
}

fn end_turn(state: &GameState) -> GameState {
    let active = state.active;
    act(state, json!({ "type": "endTurn", "playerId": active }))
}

fn turn_started_extra(events: &[GameEvent]) -> Vec<Option<bool>> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::TurnStarted { extra, .. } => Some(*extra),
            _ => None,
        })
        .collect()
}

mod r844_lost_refreshes {
    use super::*;

    #[test]
    fn r844_the_next_refresh_gives_0_and_spends_the_rider_then_the_next_is_normal() {
        let mut state = playing("lost-one");
        assert_eq!(state.turn, 1);
        lose(&mut state, 1, as_p1());
        state.players.p1.mana.next_turn_mod = 2;

        // Turn 3 is p1's next turn: the refresh gives 0 and spends the rider with it.
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P2);
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.turn, 3);
        assert_eq!(state.players.p1.mana.current, 0);
        assert_eq!(state.players.p1.mana.max, 2);
        assert_eq!(state.players.p1.mana.next_turn_mod, 0);
        assert_eq!(state.players.p1.lost_refresh_through, None);

        // Turn 5 is normal again.
        state = end_turn(&state);
        state = end_turn(&state);
        assert_eq!(state.turn, 5);
        assert_eq!(state.players.p1.mana.current, 3);
    }

    #[test]
    fn r844_mana_gained_during_a_lost_turn_still_adds() {
        let mut state = playing("lost-gain");
        lose(&mut state, 1, as_p1());
        state = end_turn(&state);
        state = end_turn(&state);
        assert_eq!(state.players.p1.mana.current, 0);

        run(&mut state, gain_mana(json_as(json!({ "amount": 2 }))), as_p1());
        assert_eq!(state.players.p1.mana.current, 2);
    }

    #[test]
    fn r844_overlapping_losses_keep_the_latest_end() {
        let mut state = playing("lost-overlap");
        let started = state.players.p1.turns_started;
        lose(&mut state, 1, as_p1());
        assert_eq!(state.players.p1.lost_refresh_through, Some(started + 1));
        lose(&mut state, 3, as_p1());
        assert_eq!(state.players.p1.lost_refresh_through, Some(started + 3));
        // A shorter loss on top changes nothing.
        lose(&mut state, 1, as_p1());
        assert_eq!(state.players.p1.lost_refresh_through, Some(started + 3));
    }

    #[test]
    fn r844_both_views_list_the_badge() {
        let mut state = playing("lost-badge");
        lose(&mut state, 2, as_p1());
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            // P1's side is `you` for p1 and `opponent` for p2; p2's side carries no badge.
            let (side, other) = if viewer == PlayerId::P1 {
                (&view.you, &view.opponent)
            } else {
                (&view.opponent, &view.you)
            };
            let badges: Vec<&ModifierView> = side
                .modifiers
                .iter()
                .filter(|badge| badge.id == "lostRefresh")
                .collect();
            assert_eq!(badges.len(), 1, "one lostRefresh badge for {viewer}");
            assert_eq!(
                badges[0].label, "Your next 2 refreshes give 0 mana",
                "for {viewer}"
            );
            assert!(
                other.modifiers.iter().all(|badge| badge.id != "lostRefresh"),
                "no badge on the other side for {viewer}"
            );
        }
        // One lost turn reads singular.
        let mut one = playing("lost-badge-one");
        lose(&mut one, 1, as_p1());
        let view = view_for(&one, PlayerId::P1);
        let badge = view
            .you
            .modifiers
            .iter()
            .find(|badge| badge.id == "lostRefresh")
            .expect("the badge");
        assert_eq!(badge.label, "Your next refresh gives 0 mana");
    }
}

mod r845_extra_turn_basics {
    use super::*;

    #[test]
    fn r845_the_same_player_goes_again_with_turn_max_mana_and_a_draw() {
        let mut state = playing("extra-basic");
        let hand = state.players.p1.hand.len();
        let library = state.players.p1.library.len();
        run(&mut state, take_extra_turn(json_as(json!({}))), as_p1());

        let result = act_full(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert!(result.error.is_none());
        state = result.state;
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.turn, 2);
        assert_eq!(state.players.p1.turns_started, 2);
        assert_eq!(state.players.p1.extra_turns, None);
        assert_eq!(state.players.p1.mana.max, 2);
        assert_eq!(turn_started_extra(&result.events), vec![Some(true)]);
        assert_eq!(state.players.p1.hand.len(), hand + 1);
        assert_eq!(state.players.p1.library.len(), library - 1);
    }

    #[test]
    fn r845_the_turn_cap_counts_an_extra_turn() {
        let mut state = playing("extra-cap");
        state.turn = 59;
        run(&mut state, take_extra_turn(json_as(json!({}))), as_p1());

        // The owed turn starts again as turn 60 …
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.turn, 60);
        assert!(state.result.is_none());

        // … and ending it hits the cap.
        state = end_turn(&state);
        let result = state.result.expect("the cap ends the game");
        assert_eq!(result.winner, Winner::Draw);
        assert_eq!(result.reason, GameOverReason::TurnCap);
    }

    #[test]
    fn r845_the_gainers_next_turn_mana_lands_on_it() {
        let mut state = playing("extra-rider");
        run(
            &mut state,
            next_turn_mana(json_as(json!({ "amount": 2 }))),
            as_p1(),
        );
        run(&mut state, take_extra_turn(json_as(json!({}))), as_p1());

        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        // Turn 2's max (2) plus the rider (2).
        assert_eq!(state.players.p1.mana.current, 4);
        assert_eq!(state.players.p1.mana.next_turn_mod, 0);
    }

    #[test]
    fn r845_r844_an_extra_turn_is_one_of_the_lost_turns() {
        let mut state = playing("extra-lost");
        lose(&mut state, 1, as_p1());
        run(&mut state, take_extra_turn(json_as(json!({}))), as_p1());

        // The extra turn's refresh is the lost one.
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.players.p1.mana.current, 0);
        assert_eq!(state.players.p1.lost_refresh_through, None);

        // The turn after is normal.
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P2);
        state = end_turn(&state);
        assert_eq!(state.players.p1.mana.current, 3);
    }
}

mod r846_owed_turns {
    use super::*;

    #[test]
    fn r846_gained_on_the_opponents_turn_it_follows_your_next_turn() {
        let mut state = playing("extra-opponent");
        // P2 to move: p1 gains a turn from it.
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P2);
        run(&mut state, take_extra_turn(json_as(json!({}))), as_p1());
        assert_eq!(state.players.p1.extra_turns, Some(1));

        // P2's turn ends into p1's normal turn, which the owed turn follows.
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.players.p1.extra_turns, Some(1));
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.players.p1.extra_turns, None);
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P2);
    }

    #[test]
    fn r846_two_owed_are_taken_one_after_another() {
        let mut state = playing("extra-two");
        run(&mut state, take_extra_turn(json_as(json!({}))), as_p1());
        run(&mut state, take_extra_turn(json_as(json!({}))), as_p1());
        assert_eq!(state.players.p1.extra_turns, Some(2));

        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.players.p1.extra_turns, Some(1));
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.players.p1.extra_turns, None);
        state = end_turn(&state);
        assert_eq!(state.active, PlayerId::P2);
    }
}

mod r847_rift_flag {
    use super::*;

    fn rift() -> Effect {
        take_extra_turn(json_as(json!({ "rift": true })))
    }

    #[test]
    fn r847_a_second_rift_grants_nothing_and_each_player_has_a_flag() {
        let mut state = playing("rift-twice");
        let first = run(&mut state, rift(), as_p1());
        assert_eq!(state.players.p1.extra_turns, Some(1));
        assert_eq!(state.players.p1.rift_extra_turn, Some(true));
        assert!(
            first
                .iter()
                .any(|event| matches!(event, GameEvent::ModifierChanged { added: true, .. })),
            "the first Rift reports its grant"
        );

        // A second Rift resolves but grants no turn and reports nothing.
        let second = run(&mut state, rift(), as_p1());
        assert_eq!(state.players.p1.extra_turns, Some(1));
        assert!(second.is_empty(), "no grant, no event");

        // Each player has their own flag: p2's Rift still grants.
        run(&mut state, rift(), as_p2());
        assert_eq!(state.players.p2.extra_turns, Some(1));
        assert_eq!(state.players.p2.rift_extra_turn, Some(true));
        assert_eq!(state.players.p1.extra_turns, Some(1));
    }

    #[test]
    fn r847_both_views_show_extra_turns_and_the_flag() {
        let mut state = playing("rift-view");
        run(&mut state, rift(), as_p1());
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let side = if viewer == PlayerId::P1 {
                &view.you
            } else {
                &view.opponent
            };
            assert_eq!(side.extra_turns, Some(1), "p1 owes a turn for {viewer}");
            assert_eq!(
                side.rift_extra_turn,
                Some(true),
                "p1's Rift flag shows for {viewer}"
            );
            assert!(
                side.modifiers
                    .iter()
                    .any(|badge| badge.id == "extraTurns" && badge.label == "Extra turns owed: 1"),
                "the owed-turns badge for {viewer}"
            );
        }
    }
}
