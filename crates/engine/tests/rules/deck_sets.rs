//! R4595000: a game's setup may name, per seat, the sets its deck is dealt from (`deckSets`). The
//! state keeps them, the view shows them on both seats, a fold and a Glitch reset keep them, a deck
//! with a card of another set is refused, and a game that names none stores nothing and hashes as
//! before.
//!
//! Fixtures: the vanilla decks `fx-1`… and `fx-21`…, Core Units.

use jackioh_engine::subsystems::glitch::reset_match;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{setup_catalog, sink_for};

type Sets = PerPlayerOpt<Vec<SetName>>;

fn options(seed: &str, sets: Option<Sets>) -> CreateGameOptions {
    setup_catalog();
    CreateGameOptions {
        seed: seed.to_string(),
        decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
        deck_sets: sets,
        ..Default::default()
    }
}

fn on_p2(list: Vec<SetName>) -> Sets {
    PerPlayerOpt {
        p1: None,
        p2: Some(list),
    }
}

/// R4595000: a seat's named sets are kept in the state and shown to both seats; the other seat's are not
#[test]
fn r4595000_named_sets_are_kept_and_shown_to_both_seats() {
    let state = begin_game(&create_game(&options(
        "ds-kept",
        Some(on_p2(vec![SetName::Core])),
    )))
    .state;
    assert_eq!(state.players.p2.deck_sets, Some(vec![SetName::Core]));
    assert_eq!(state.players.p1.deck_sets, None);
    assert_eq!(
        view_for(&state, PlayerId::P1).opponent.deck_sets,
        Some(vec![SetName::Core])
    );
    assert_eq!(
        view_for(&state, PlayerId::P2).you.deck_sets,
        Some(vec![SetName::Core])
    );
    assert_eq!(view_for(&state, PlayerId::P2).opponent.deck_sets, None);
}

/// R4595000: a fold handed the same sets is the game played; one handed none is not
#[test]
fn r4595000_a_fold_keeps_the_named_sets() {
    let sets = on_p2(vec![SetName::Core]);
    let live = begin_game(&create_game(&options("ds-fold", Some(sets.clone())))).state;
    let input = |deck_sets: Option<Sets>| ReplayInput {
        seed: "ds-fold".into(),
        decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
        deck_sets,
        ..Default::default()
    };
    assert_eq!(hash_state(&fold(&input(Some(sets))).state), hash_state(&live));
    assert_ne!(hash_state(&fold(&input(None)).state), hash_state(&live));
}

/// R4595000: a Glitch reset deals the game again with the sets it named
#[test]
fn r4595000_a_glitch_reset_keeps_the_named_sets() {
    let mut state = begin_game(&create_game(&options(
        "ds-glitch",
        Some(on_p2(vec![SetName::Core])),
    )))
    .state;
    {
        let mut sink = sink_for(&mut state);
        reset_match(&mut sink);
    }
    assert_eq!(state.resets, Some(1));
    assert_eq!(state.players.p2.deck_sets, Some(vec![SetName::Core]));
}

/// R4595000: a deck holding a card of a set its seat does not name is refused, with the reason
#[test]
fn r4595000_a_card_outside_the_named_sets_is_refused() {
    let opts = options("ds-refused", Some(on_p2(vec![SetName::Classic])));
    let error = validate_setup(&opts, registered_catalog(), SetupKind::New)
        .expect_err("a Core deck is not dealt from Classic");
    assert!(error.message.contains("R4595000"), "{}", error.message);
    assert!(error.message.starts_with("p2:"), "{}", error.message);
}

/// R4595000: a game that names no sets, or an empty list, stores nothing and hashes as before
#[test]
fn r4595000_no_named_sets_store_nothing_and_hash_as_before() {
    let plain = create_game(&options("ds-none", None));
    let empty = create_game(&options(
        "ds-none",
        Some(PerPlayerOpt {
            p1: Some(Vec::new()),
            p2: None,
        }),
    ));
    assert_eq!(plain.players.p1.deck_sets, None);
    assert_eq!(empty.players.p1.deck_sets, None);
    assert_eq!(hash_state(&plain), hash_state(&empty));
    assert!(!serde_json::to_string(&plain).unwrap().contains("deckSets"));
}
