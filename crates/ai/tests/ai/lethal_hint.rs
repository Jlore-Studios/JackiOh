//! R2102000: the practice AI's lethal hint (`lethal_hint`, SPEC §9.9): whether the human has a
//! sure win this turn by what they can see, and the same answer whatever the AI's hidden cards are.
//!
//! The human in p2 has Tempo Timmy 3/3 and Mr. Vanilla 4/4 ready, 7 to the face; the AI in p1 has an
//! empty field and the observe tests' two sets of hidden cards (#2 Speciesist and #11 Tempo Timmy against
//! #53 Reno and #20 Pointmaster, a face-down #41 Sheepish against #60 Bear Honeypot, the same cost).
//! `observe.rs` holds the property over real states.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{HUMAN, register_cards};

struct Hidden {
    hand: [&'static str; 2],
    library: [&'static str; 3],
    trap: &'static str,
}

const SET_A: Hidden = Hidden {
    hand: ["core-002", "core-011"],
    library: ["core-005", "core-016", "core-047"],
    trap: "core-041",
};

const SET_B: Hidden = Hidden {
    hand: ["core-053", "core-020"],
    library: ["core-063", "core-044", "core-072"],
    trap: "core-060",
};

fn board(seed: &str, active: PlayerId, hidden: &Hidden, trapped: bool, health: i32) -> GameState {
    register_cards();
    let backrow = if trapped {
        json!([{ "def": hidden.trap, "faceUp": false }])
    } else {
        json!([])
    };
    scenario(json!({
        "seed": seed,
        "active": active,
        "p1": { "hand": hidden.hand, "library": hidden.library, "backrow": backrow, "health": health },
        "p2": { "field": ["core-011", "core-008"] },
    }))
    .state()
    .clone()
}

/// R2102000: two swings for 7 at an AI on 6, with nothing in the way, light the hint
#[test]
fn r2102000_a_board_the_human_wins_this_turn_lights_the_hint() {
    let state = board("lethal-hint", PlayerId::P2, &SET_A, false, 6);
    assert!(lethal_hint(&state, HUMAN, None));
}

/// R2102000: 7 to the face of an AI on 20 is no win, so the hint stays dark
#[test]
fn r2102000_a_board_the_human_cannot_win_this_turn_lights_nothing() {
    let state = board("lethal-hint", PlayerId::P2, &SET_A, false, 20);
    assert!(!lethal_hint(&state, HUMAN, None));
}

/// R2102000: a clock that has run out (the browser's cap) stops the search, which lights nothing
#[test]
fn r2102000_a_search_the_clock_stops_lights_nothing() {
    let state = board("lethal-hint", PlayerId::P2, &SET_A, false, 6);
    let stopped = || true;
    assert!(!lethal_hint(&state, HUMAN, Some(&stopped)));
    let running = || false;
    assert!(lethal_hint(&state, HUMAN, Some(&running)));
}

/// R2102000: on the AI's turn the human can win nothing this turn, whatever their board would deal
#[test]
fn r2102000_off_the_humans_own_turn_the_hint_is_dark() {
    let state = board("lethal-hint", PlayerId::P1, &SET_A, false, 6);
    assert!(!lethal_hint(&state, HUMAN, None));
}

/// R2102000, R185: states that differ only in the AI's hidden cards and the seed redact alike and give
/// one answer
#[test]
fn r2102000_two_states_that_differ_only_in_the_ais_hidden_cards_give_one_answer() {
    for trapped in [false, true] {
        for health in [6, 20] {
            let a = board("lethal-hint-a", PlayerId::P2, &SET_A, trapped, health);
            let b = board("lethal-hint-b", PlayerId::P2, &SET_B, trapped, health);
            let case = format!("trapped {trapped}, health {health}");
            assert_ne!(hash_state(&a), hash_state(&b), "{case}");
            assert_eq!(
                hash_state(&redact(&a, HUMAN)),
                hash_state(&redact(&b, HUMAN)),
                "{case}"
            );
            let hint = lethal_hint(&a, HUMAN, None);
            assert_eq!(lethal_hint(&b, HUMAN, None), hint, "{case}");
            if !trapped {
                assert_eq!(hint, health == 6, "{case}");
            }
        }
    }
}

// R4595000: an Infinite Assault enemy's deck is dealt from its own set, which the game names, so the
// hint (and the AI) resample its unseen cards from Infinite Assault cards alone.

const ASSAULT_A: Hidden = Hidden {
    hand: ["infiniteassault-001", "infiniteassault-002"],
    library: [
        "infiniteassault-003",
        "infiniteassault-004",
        "infiniteassault-005",
    ],
    trap: "infiniteassault-029",
};

const ASSAULT_B: Hidden = Hidden {
    hand: ["infiniteassault-006", "infiniteassault-007"],
    library: [
        "infiniteassault-008",
        "infiniteassault-009",
        "infiniteassault-010",
    ],
    trap: "infiniteassault-148",
};

/// `board`, the AI's deck named as dealt from the Infinite Assault (R4595000).
fn assault_board(seed: &str, hidden: &Hidden, trapped: bool, health: i32) -> GameState {
    let mut state = board(seed, PlayerId::P2, hidden, trapped, health);
    state.players.p1.deck_sets = Some(vec![SetName::InfiniteAssault]);
    state
}

/// The AI's cards the human cannot see, in one determinization of `state` from the human's seat.
fn sampled_hidden(state: &GameState, rng: &mut Rng) -> Vec<String> {
    let world = determinize(&redact(state, HUMAN), HUMAN, rng, DeterminizeOptions::default());
    let ai = &world.players.p1;
    ai.hand
        .iter()
        .chain(ai.library.iter())
        .chain(ai.backrow.iter().flatten())
        .map(|card| card.def_id.clone())
        .collect()
}

/// R4595000: the hint's worlds deal an Assault deck's hand, library and face-down card from Infinite
/// Assault cards alone, the face-down card from its Traps of the cost it shows; a deck that names no
/// set samples every set
#[test]
fn r4595000_the_lethal_hint_against_an_assault_deck_samples_infinite_assault_cards() {
    let assault = assault_board("assault-hint", &ASSAULT_A, true, 6);
    let mut rng = Rng::new("r4595000", 0);
    for _ in 0..LETHAL_HINT_DETERMINIZATIONS {
        let hidden = sampled_hidden(&assault, &mut rng);
        assert_eq!(hidden.len(), 6);
        for id in &hidden {
            let def = find_def(None, id).expect("a catalog card");
            assert_eq!(def.set, SetName::InfiniteAssault, "{id}");
            assert!(!def.is_token(), "{id}");
        }
        let trap = find_def(None, &hidden[5]).expect("the face-down card");
        assert!(
            matches!(trap.type_, CardType::Trap | CardType::FieldTrap),
            "{}",
            trap.id
        );
    }
    let mut unnamed = assault.clone();
    unnamed.players.p1.deck_sets = None;
    let mut rng = Rng::new("r4595000", 0);
    let other_sets = (0..LETHAL_HINT_DETERMINIZATIONS)
        .flat_map(|_| sampled_hidden(&unnamed, &mut rng))
        .any(|id| find_def(None, &id).is_some_and(|def| def.set != SetName::InfiniteAssault));
    assert!(other_sets, "a deck that names no set samples every set");
}

/// R4595000, R185: against an Assault deck the hint still answers from what the human sees alone,
/// whichever Infinite Assault cards the AI hides, and lights a lethal no face-down card can stop
#[test]
fn r4595000_the_lethal_hint_against_an_assault_deck_reads_only_what_the_human_sees() {
    for trapped in [false, true] {
        for health in [6, 20] {
            let a = assault_board("assault-hint-a", &ASSAULT_A, trapped, health);
            let b = assault_board("assault-hint-b", &ASSAULT_B, trapped, health);
            let case = format!("trapped {trapped}, health {health}");
            assert_eq!(
                hash_state(&redact(&a, HUMAN)),
                hash_state(&redact(&b, HUMAN)),
                "{case}"
            );
            let hint = lethal_hint(&a, HUMAN, None);
            assert_eq!(lethal_hint(&b, HUMAN, None), hint, "{case}");
            if !trapped {
                assert_eq!(hint, health == 6, "{case}");
            }
        }
    }
}

/// R4595000 (the sweep), R185: an Assault board whose Glitch Specter (IA #32) reads its controller's
/// face-down cards, redacted for the human, counts the hidden card as face-down and determinizes
/// without reading the placeholder's definition
#[test]
fn r4595000_a_redacted_face_down_card_counts_for_glitch_specter() {
    register_cards();
    let mut state = scenario(json!({
        "seed": "assault-specter",
        "active": PlayerId::P2,
        "p1": {
            "hand": ASSAULT_A.hand,
            "library": ASSAULT_A.library,
            "field": ["infiniteassault-032"],
            "backrow": [{ "def": ASSAULT_A.trap, "faceUp": false }],
        },
        "p2": { "field": ["core-011"] },
    }))
    .state()
    .clone();
    state.players.p1.deck_sets = Some(vec![SetName::InfiniteAssault]);
    let public = redact(&state, HUMAN);
    assert_eq!(
        jackioh_engine::face_down::face_down_count(&public, PlayerId::P1),
        1
    );
    let mut rng = Rng::new("r4595000-specter", 0);
    let world = determinize(&public, HUMAN, &mut rng, DeterminizeOptions::default());
    assert_eq!(
        jackioh_engine::face_down::face_down_count(&world, PlayerId::P1),
        1
    );
}
