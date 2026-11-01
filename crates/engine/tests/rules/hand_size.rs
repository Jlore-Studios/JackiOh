//! ME-HANDCAP, R1143 (docs/meditative-set.md M5, Group E's systems; Meditative #79 Touched by KY): a
//! player's hand size for the rest of the game. `PlayerState.handCap` is absent until an effect sets it
//! (`set_hand_cap`), so a game that never sets one serializes and hashes as before (D14), and every
//! reader of the hand cap asks `hand_cap_of` instead of `HAND_CAP`, and every test of a full hand asks
//! `room_in`: a draw, an add, a take from the other player's hand. The setting is the latest one, at most `HAND_CAP_MAX`; a hand above a lowered
//! one keeps its cards, and both views show it.

use jackioh_engine::effects::{add_to_hand, give_from_hand, set_hand_cap};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{Bench, events_of_type, in_hand, set_library};
use crate::rules::fixtures::prompt_harness::board;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// Apply one effect for `controller` outside any card, then settle, as a resolution does.
fn run(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut sink = Bench::new(state);
    {
        let mut engine = sink.sink(state);
        {
            let mut ctx = make_context(
                &mut engine,
                None,
                HookOptions {
                    controller: Some(controller),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut engine, Default::default());
    }
    state.rng_cursor = sink.rng.cursor();
    sink.events
}

/// `count` §2.4 draws of `player`'s.
fn draws(state: &mut GameState, player: PlayerId, count: i32) -> Vec<GameEvent> {
    let mut sink = Bench::new(state);
    {
        let mut engine = sink.sink(state);
        jackioh_engine::draw::draw(&mut engine, player, count);
        settle(&mut engine, Default::default());
    }
    state.rng_cursor = sink.rng.cursor();
    sink.events
}

fn sets(cap: i32) -> Effect {
    set_hand_cap(json_as(json!({ "cap": cap })))
}

mod r1143_a_hand_size_for_the_rest_of_the_game {
    use super::*;

    #[test]
    fn r1143_unset_reads_hand_cap_and_serializes_without_it() {
        let state = board("hand-size-unset");
        assert_eq!(state.players[P1].hand_cap, None);
        assert_eq!(hand_cap_of(&state, P1), HAND_CAP);
        assert_eq!(hand_cap_of(&state, P2), HAND_CAP);
        // D14: no key at all, so a game that never sets one hashes as it did.
        let side = serde_json::to_value(&state.players[P1]).expect("a player serializes");
        assert!(side.get("handCap").is_none());
        assert_eq!(view_for(&state, P1).you.hand_cap, None);
    }

    #[test]
    fn r1143_latest_setting_wins_capped_at_hand_cap_max() {
        let mut state = board("hand-size-latest");
        run(&mut state, sets(12), P1);
        assert_eq!(hand_cap_of(&state, P1), 12);
        run(&mut state, sets(11), P1);
        assert_eq!(hand_cap_of(&state, P1), 11);
        run(&mut state, sets(99), P1);
        assert_eq!(hand_cap_of(&state, P1), HAND_CAP_MAX);
        assert_eq!(HAND_CAP_MAX, 14);
        // The other player's is their own.
        assert_eq!(hand_cap_of(&state, P2), HAND_CAP);
        let side = serde_json::to_value(&state.players[P1]).expect("a player serializes");
        assert_eq!(side["handCap"], json!(14));
    }

    #[test]
    fn r1143_a_draw_fills_an_eleventh_slot_and_burns_past_the_cap() {
        let mut state = board("hand-size-draw");
        in_hand(&mut state, &plain.id, P1, HAND_CAP);
        set_library(&mut state, P1, &[plain.id.clone(), plain.id.clone()]);
        run(&mut state, sets(11), P1);
        let events = draws(&mut state, P1, 2);
        assert_eq!(state.players[P1].hand.len(), 11);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 1);
        assert_eq!(state.players[P1].graveyard.len(), 1);
    }

    #[test]
    fn r1143_an_add_and_a_take_read_it() {
        let mut state = board("hand-size-add");
        in_hand(&mut state, &plain.id, P1, HAND_CAP);
        let theirs = in_hand(&mut state, &plain.id, P2, 1);
        run(&mut state, sets(11), P1);
        // An add fills the eleventh slot.
        let added = run(&mut state, add_to_hand(json_as(json!({ "defId": plain.id }))), P1);
        assert_eq!(state.players[P1].hand.len(), 11);
        assert!(events_of_type(&added, GameEventType::Burned).is_empty());
        // A card taken from the other hand meets the full hand and burns into the taker's graveyard.
        let taken = run(
            &mut state,
            give_from_hand(json_as(json!({ "from": "enemy", "cards": "all" }))),
            P1,
        );
        assert_eq!(state.players[P1].hand.len(), 11);
        assert_eq!(events_of_type(&taken, GameEventType::Burned).len(), 1);
        assert_eq!(
            state.players[P1]
                .graveyard
                .iter()
                .map(|card| card.id.clone())
                .collect::<Vec<String>>(),
            vec![theirs[0].id.clone()]
        );
    }

    #[test]
    fn r1143_a_lower_setting_keeps_the_cards_held() {
        let mut state = board("hand-size-lower");
        in_hand(&mut state, &plain.id, P1, HAND_CAP);
        set_library(&mut state, P1, std::slice::from_ref(&plain.id));
        run(&mut state, sets(8), P1);
        // Nothing is discarded: only what arrives next is burned.
        assert_eq!(state.players[P1].hand.len() as i32, HAND_CAP);
        let events = draws(&mut state, P1, 1);
        assert_eq!(state.players[P1].hand.len() as i32, HAND_CAP);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 1);
    }

    #[test]
    fn r1143_room_in_is_the_cap_less_the_hand_and_none_above_a_lowered_cap() {
        let mut state = board("hand-size-room");
        state.players[P1].hand.clear();
        assert_eq!(room_in(&state, P1, OffFieldZone::Hand), HAND_CAP);
        in_hand(&mut state, &plain.id, P1, 3);
        assert_eq!(room_in(&state, P1, OffFieldZone::Hand), HAND_CAP - 3);
        run(&mut state, sets(12), P1);
        assert_eq!(room_in(&state, P1, OffFieldZone::Hand), 12 - 3);
        // A hand above a lowered cap keeps its cards and has no room, never less than none.
        in_hand(&mut state, &plain.id, P1, 7);
        run(&mut state, sets(8), P1);
        assert_eq!(state.players[P1].hand.len(), 10);
        assert_eq!(room_in(&state, P1, OffFieldZone::Hand), 0);
        // A graveyard and an exile pile have no cap.
        assert_eq!(room_in(&state, P1, OffFieldZone::Graveyard), i32::MAX);
        assert_eq!(room_in(&state, P1, OffFieldZone::Exile), i32::MAX);
    }

    #[test]
    fn r1143_both_views_show_it() {
        let mut state = board("hand-size-views");
        run(&mut state, sets(12), P1);
        assert_eq!(view_for(&state, P1).you.hand_cap, Some(12));
        assert_eq!(view_for(&state, P2).opponent.hand_cap, Some(12));
        assert_eq!(view_for(&state, P2).you.hand_cap, None);
        assert_eq!(view_for(&state, P1).opponent.hand_cap, None);
    }

    #[test]
    fn r1143_a_set_hand_size_replays_from_the_state() {
        let mut state = board("hand-size-round-trip");
        run(&mut state, sets(13), P2);
        let round_tripped: GameState =
            serde_json::from_value(serde_json::to_value(&state).expect("a state serializes"))
                .expect("a state parses");
        assert_eq!(round_tripped, state);
        assert_eq!(hand_cap_of(&round_tripped, P2), 13);
    }
}
