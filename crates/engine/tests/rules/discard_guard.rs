//! R800: the discard guard (`Script.discard_guard`, Meditative #1 Disruptive Disruptor's Aura, "You
//! can't be forced to discard cards during your opponent's turn").
//!
//! What is pinned: while it is not the guarded player's turn, every discard an effect makes from their
//! hand (`discard`, `discard_random`, `discard_hand`) moves no card, draws no random number (R129) and
//! is reported by one public `discardPrevented { player, count }`; on their own turn the same discard
//! happens; a discard paid as a price (an Activate's cost, R384, `discard_random_cards`; a targeting
//! cost, R450, and Temporary's own-turn discard, R637, both `discard_from_hand`) never asks; a guard
//! only guards from the field and only the players it names; two guards report once. The fixture is
//! registered here, prefixed `dg-` and indexed 720, so no shared fixture grows for it (BUILD §0).

use jackioh_engine::draw::discard_guarded;
use jackioh_engine::effects::{
    discard, discard_from_hand, discard_hand, discard_random, discard_random_cards,
};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::testkit::*;

use super::fixtures::catalog::spell_def;
use super::fixtures::combat::plain;
use super::fixtures::harness::{in_hand, new_game, put, slot};

/// A Field Spell whose Aura guards its controller (Meditative #1's, without the Radiant Cry).
fn guard() -> CardDef {
    spell_def(
        720,
        json!({ "id": "dg-guard", "index": "720", "name": "Discard Guard (fixture)", "type": "Field Spell" }),
    )
}

fn game() -> GameState {
    let mut state = new_game("discard-guard-test", None);
    let mut catalog = registered_catalog().clone();
    let def = guard();
    catalog.insert(def.id.clone(), def.clone());
    register_catalog(catalog);
    let script = Script {
        discard_guard: Some(read_hook(|_args| vec![DrawLimitPlayer::SelfSide])),
        ..Script::default()
    };
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        def.id,
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
    state.players.p1.hand.clear();
    state.players.p2.hand.clear();
    state
}

/// Apply one effect as `controller`'s, the way `resolve.rs` does, and hand back the events it emitted.
fn run(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(controller),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// One engine call on a sink over the state, its rng from the state's cursor, and the events it made.
fn on_sink(state: &mut GameState, call: impl FnOnce(&mut EngineSink<'_>)) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        call(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn guarded_backrow(state: &mut GameState, player: PlayerId, lane: i32) -> CardInstance {
    put(state, &guard().id, slot(player, Row::Backrow, lane), Default::default())
}

/// How many cards the events say were discarded, and whether a guard reported anything.
fn discards(events: &[GameEvent]) -> (usize, bool) {
    let count = |kind: GameEventType| events.iter().filter(|event| event.event_type() == kind).count();
    (
        count(GameEventType::Discarded),
        count(GameEventType::DiscardPrevented) > 0,
    )
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

#[test]
fn r800_an_effect_discard_off_turn_moves_nothing_draws_no_rng_and_reports() {
    let mut state = game();
    guarded_backrow(&mut state, PlayerId::P1, 1);
    let hand = in_hand(&mut state, &plain.id, PlayerId::P1, 3);
    state.active = PlayerId::P2;
    let cursor = state.rng_cursor;

    let random = run(
        &mut state,
        discard_random(json_as(json!({ "count": 2, "player": "enemy" }))),
        PlayerId::P2,
    );
    assert_eq!(
        random,
        vec![GameEvent::DiscardPrevented {
            player: PlayerId::P1,
            count: 2,
        }]
    );
    assert_eq!(state.rng_cursor, cursor);
    assert_eq!(ids(&state.players.p1.hand), ids(&hand));

    let whole = run(
        &mut state,
        discard_hand(json_as(json!({ "player": "enemy" }))),
        PlayerId::P2,
    );
    assert_eq!(
        whole,
        vec![GameEvent::DiscardPrevented {
            player: PlayerId::P1,
            count: 3,
        }]
    );

    let named = run(
        &mut state,
        discard(json_as(
            json!({ "target": { "of": "instance", "instanceId": hand[0].id } }),
        )),
        PlayerId::P2,
    );
    assert_eq!(
        named,
        vec![GameEvent::DiscardPrevented {
            player: PlayerId::P1,
            count: 1,
        }]
    );
    assert_eq!(ids(&state.players.p1.hand), ids(&hand));
    assert!(state.players.p1.graveyard.is_empty());
}

#[test]
fn r800_a_random_discard_asking_more_than_the_hand_reports_what_it_could_have_taken() {
    let mut state = game();
    guarded_backrow(&mut state, PlayerId::P1, 1);
    in_hand(&mut state, &plain.id, PlayerId::P1, 1);
    state.active = PlayerId::P2;

    let events = run(
        &mut state,
        discard_random(json_as(json!({ "count": 3, "player": "enemy" }))),
        PlayerId::P2,
    );
    assert_eq!(
        events,
        vec![GameEvent::DiscardPrevented {
            player: PlayerId::P1,
            count: 1,
        }]
    );

    // An empty hand loses nothing either way, so nothing is reported.
    state.players.p1.hand.clear();
    let empty = run(
        &mut state,
        discard_random(json_as(json!({ "count": 2, "player": "enemy" }))),
        PlayerId::P2,
    );
    assert!(empty.is_empty());
}

#[test]
fn r800_on_the_guarded_players_own_turn_the_discard_happens() {
    let mut state = game();
    guarded_backrow(&mut state, PlayerId::P1, 1);
    in_hand(&mut state, &plain.id, PlayerId::P1, 3);
    state.active = PlayerId::P1;
    assert!(!discard_guarded(&state, PlayerId::P1));

    let events = run(
        &mut state,
        discard_random(json_as(json!({ "count": 2 }))),
        PlayerId::P1,
    );
    assert_eq!(discards(&events), (2, false));
    assert_eq!(state.players.p1.hand.len(), 1);
    assert_eq!(state.players.p1.graveyard.len(), 2);
}

#[test]
fn r800_a_price_still_discards() {
    let mut state = game();
    guarded_backrow(&mut state, PlayerId::P1, 1);
    let hand = in_hand(&mut state, &plain.id, PlayerId::P1, 3);
    state.active = PlayerId::P2;
    assert!(discard_guarded(&state, PlayerId::P1));

    // An Activate's random discard (R384).
    let cost = on_sink(&mut state, |sink| discard_random_cards(sink, PlayerId::P1, 1));
    assert_eq!(discards(&cost), (1, false));
    assert_eq!(state.players.p1.hand.len(), 2);

    // A targeting cost (R450) and Temporary's discard (R637).
    let kept = state.players.p1.hand[0].clone();
    let named = on_sink(&mut state, |sink| discard_from_hand(sink, &kept));
    assert_eq!(discards(&named), (1, false));
    assert_eq!(state.players.p1.hand.len(), 1);
    assert_eq!(state.players.p1.graveyard.len(), 2);
    assert!(ids(&state.players.p1.graveyard).contains(&kept.id));
    assert!(ids(&hand).contains(&kept.id));
}

#[test]
fn r800_a_guard_in_the_hand_does_nothing_and_two_guards_report_once() {
    let mut state = game();
    in_hand(&mut state, &guard().id, PlayerId::P1, 1);
    in_hand(&mut state, &plain.id, PlayerId::P1, 2);
    state.active = PlayerId::P2;
    assert!(!discard_guarded(&state, PlayerId::P1));

    let events = run(
        &mut state,
        discard_random(json_as(json!({ "count": 1, "player": "enemy" }))),
        PlayerId::P2,
    );
    assert_eq!(discards(&events), (1, false));
    assert_eq!(state.players.p1.hand.len(), 2);

    guarded_backrow(&mut state, PlayerId::P1, 1);
    guarded_backrow(&mut state, PlayerId::P1, 2);
    let twice = run(
        &mut state,
        discard_hand(json_as(json!({ "player": "enemy" }))),
        PlayerId::P2,
    );
    assert_eq!(
        twice,
        vec![GameEvent::DiscardPrevented {
            player: PlayerId::P1,
            count: 2,
        }]
    );
    assert_eq!(state.players.p1.hand.len(), 2);
}

#[test]
fn r800_a_guard_binds_only_the_players_it_names() {
    let mut state = game();
    guarded_backrow(&mut state, PlayerId::P1, 1);
    in_hand(&mut state, &plain.id, PlayerId::P2, 2);
    state.active = PlayerId::P1;
    assert!(!discard_guarded(&state, PlayerId::P2));

    let events = run(
        &mut state,
        discard_hand(json_as(json!({ "player": "enemy" }))),
        PlayerId::P1,
    );
    assert_eq!(discards(&events), (2, false));
    assert!(state.players.p2.hand.is_empty());
}
