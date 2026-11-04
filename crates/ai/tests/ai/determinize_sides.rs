//! R2826000: each side's hidden cards are sampled against what that side has shown (SPEC §9.9, §2.6,
//! R185). Real openings: `create_game` mints p1's deck c1..c25 and p2's c26..c50 (R73).

use indexmap::IndexSet;
use jackioh_ai::{HIDDEN_DEF_ID, redact};
use jackioh_engine::testkit::{
    CardInstance, CreateGameOptions, GameState, PlayerId, Zone, begin_game, create_game, create_rng,
};

use super::support::{AI, HUMAN, card_by_id, dealt_game, det, every_card, ids, random_decks, register_cards};

/// Moves `count` cards (all when `None`) from the top of `side`'s library to its graveyard.
fn to_graveyard(state: &mut GameState, side: PlayerId, count: Option<usize>) {
    let len = state.players[side].library.len();
    let moved: Vec<CardInstance> = state.players[side]
        .library
        .drain(..count.unwrap_or(len))
        .collect();
    for mut card in moved {
        card.zone = Zone::Graveyard { player: side };
        state.players[side].graveyard.push(card);
    }
}

/// A real opening whose AI deck was dealt (R433), so the AI was shown none of its library (R312); ten
/// of its library cards in its graveyard, the opponent's whole library in the opponent's. The two
/// `random_decks` share no card.
fn dealt_opening(seed: &str) -> GameState {
    register_cards();
    let mut state = begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: random_decks(seed),
        dealt: Some(vec![AI]),
        ..CreateGameOptions::default()
    }))
    .state;
    to_graveyard(&mut state, AI, Some(10));
    to_graveyard(&mut state, HUMAN, None);
    state
}

fn shown_by(public: &GameState, side: PlayerId) -> IndexSet<String> {
    every_card(public)
        .into_iter()
        .filter(|card| card.owner == side && card.def_id != HIDDEN_DEF_ID)
        .map(|card| card.def_id.clone())
        .collect()
}

fn def_in(world: &GameState, id: &str) -> String {
    card_by_id(world, id)
        .map(|card| card.def_id.clone())
        .unwrap_or_default()
}

#[test]
fn r2826000_a_hidden_card_of_the_seats_own_library_never_takes_a_def_its_own_side_shows() {
    let state = dealt_opening("det-sides-own");
    let public = redact(&state, AI);
    let library = ids(&public.players[AI].library);
    assert!(
        library.len() >= 10,
        "the dealt library keeps {} cards",
        library.len()
    );
    assert!(
        public.players[AI]
            .library
            .iter()
            .all(|card| card.def_id == HIDDEN_DEF_ID),
        "R312, R433: the AI was shown none of its dealt library"
    );
    let own = shown_by(&public, AI);
    assert!(own.len() >= 10, "p1 shows {} cards", own.len());
    for k in 0..100 {
        let world = det(&public, AI, &mut create_rng(&format!("det-sides-own:{k}"), 0));
        let defs: Vec<String> = library.iter().map(|id| def_in(&world, id)).collect();
        for (id, def_id) in library.iter().zip(&defs) {
            assert_ne!(def_id, HIDDEN_DEF_ID, "seed {k}: {id} kept its placeholder");
            assert!(
                !own.contains(def_id),
                "seed {k}: {id} sampled {def_id}, which p1 shows"
            );
        }
        let distinct: IndexSet<&String> = defs.iter().collect();
        assert_eq!(
            distinct.len(),
            defs.len(),
            "seed {k}: one deck holds a card once (§2.6)"
        );
    }
}

#[test]
fn r2826000_the_seats_own_library_may_take_a_def_only_the_opponent_shows() {
    let state = dealt_opening("det-sides-theirs");
    let public = redact(&state, AI);
    let library = ids(&public.players[AI].library);
    let own = shown_by(&public, AI);
    let theirs = shown_by(&public, HUMAN);
    assert!(theirs.len() >= 15, "p2 shows {} cards", theirs.len());
    assert!(
        theirs.iter().all(|id| !own.contains(id)),
        "the two decks share no card"
    );
    let mut taken = 0;
    for k in 0..40 {
        let world = det(&public, AI, &mut create_rng(&format!("det-sides-theirs:{k}"), 0));
        taken += library
            .iter()
            .filter(|id| theirs.contains(&def_in(&world, id)))
            .count();
    }
    assert!(taken > 0, "the AI's own library never took a card only p2 shows");
}

#[test]
fn r2826000_r73_a_card_from_the_opponents_deck_in_the_seats_library_takes_no_def_the_opponent_shows() {
    let mut state = dealt_game("det-sides-r73");
    // #87's swap, as `claim_library` makes it: the cards change hands and their new owner was shown none.
    let swapped: Vec<CardInstance> = state.players[HUMAN].library.drain(..10).collect();
    let mut moved: Vec<String> = Vec::new();
    for mut card in swapped {
        card.zone = Zone::Library { player: AI };
        card.owner = AI;
        card.controller = AI;
        card.known_as = None;
        moved.push(card.id.clone());
        state.players[AI].library.push(card);
    }
    to_graveyard(&mut state, HUMAN, None);
    let public = redact(&state, AI);
    let theirs = shown_by(&public, HUMAN);
    assert!(theirs.len() >= 5, "p2 shows {} cards", theirs.len());
    let hand = ids(&public.players[HUMAN].hand);
    for k in 0..100 {
        let world = det(&public, AI, &mut create_rng(&format!("det-sides-r73:{k}"), 0));
        let defs: Vec<String> = moved.iter().chain(&hand).map(|id| def_in(&world, id)).collect();
        for (id, def_id) in moved.iter().zip(&defs) {
            assert!(
                !theirs.contains(def_id),
                "seed {k}: {id} sampled {def_id}, which p2 shows"
            );
        }
        let distinct: IndexSet<&String> = defs.iter().collect();
        assert_eq!(
            distinct.len(),
            defs.len(),
            "seed {k}: one deck holds a card once (§2.6)"
        );
    }
}
