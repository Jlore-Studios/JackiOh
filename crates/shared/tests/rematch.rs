//! Rematch offers (SPEC §9.5, R672, R2685000, R1638000), pure: the book a host keeps beside the machine.
//! The HTTP round trip is `crates/server/tests/api/rematch.rs`'s; these hold the rules themselves.

use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::{LeanPick, SetName};
use jackioh_shared::config::REMATCH_OFFER_TTL_MS;
use jackioh_shared::deck::DeckSlots;
use jackioh_shared::host::rematch::{
    FinishedSeats, OfferFacts, OfferOutcome, RematchBook, RematchRefusal, STAKE_DOUBLE, STAKE_NORMAL,
    check_offer, in_rematch_window, next_seats, stakes_of,
};

const END: i64 = 1_700_000_000_000;

fn book_with(offers: &[(jackioh_engine::PlayerId, i32)], now: i64) -> RematchBook {
    let mut book = RematchBook::default();
    for (seat, stakes) in offers {
        book.offer("m1", Some(END), *seat, *stakes, None, now, "g1");
    }
    book
}

#[test]
fn the_rematch_window_runs_from_the_end_to_the_ttl_inclusive_and_never_without_an_end() {
    assert!(!in_rematch_window(None, END));
    assert!(in_rematch_window(Some(END), END));
    assert!(in_rematch_window(Some(END), END + REMATCH_OFFER_TTL_MS));
    assert!(!in_rematch_window(Some(END), END + REMATCH_OFFER_TTL_MS + 1));
}

#[test]
fn r672_the_stakes_are_a_rematch_or_a_double_or_nothing_and_nothing_else() {
    assert_eq!(stakes_of(Some(1.0)), Some(STAKE_NORMAL));
    assert_eq!(stakes_of(Some(2.0)), Some(STAKE_DOUBLE));
    for other in [None, Some(0.0), Some(3.0), Some(1.5), Some(f64::NAN)] {
        assert_eq!(stakes_of(other), None, "{other:?}");
    }
}

#[test]
fn r672_r1638000_an_offer_is_refused_before_it_is_recorded_in_the_order_stakes_state_kind_cards() {
    let ok = OfferFacts {
        stakes: STAKE_NORMAL,
        ranked: Some(false),
        finished: true,
        series_game: false,
        catalog_current: true,
    };
    assert_eq!(check_offer(&ok), Ok(()));
    // Only a ranked match can spawn a double-or-nothing, and that is asked before anything else.
    let nothing_holds = OfferFacts {
        stakes: STAKE_DOUBLE,
        ranked: None,
        finished: false,
        series_game: true,
        catalog_current: false,
    };
    assert_eq!(
        check_offer(&nothing_holds),
        Err(RematchRefusal::DoubleRequiresRanked)
    );
    let ranked_double = OfferFacts {
        ranked: Some(true),
        ..nothing_holds.clone()
    };
    assert_eq!(check_offer(&ranked_double), Err(RematchRefusal::MatchNotFinished));
    let finished = OfferFacts {
        finished: true,
        ..ranked_double
    };
    assert_eq!(check_offer(&finished), Err(RematchRefusal::SeriesGame));
    let not_series = OfferFacts {
        series_game: false,
        ..finished
    };
    assert_eq!(check_offer(&not_series), Err(RematchRefusal::UpdateRequired));
}

#[test]
fn r672_two_equal_offers_make_exactly_one_game_with_the_leans_of_both_seats() {
    let mut book = RematchBook::default();
    let first = book.offer(
        "m1",
        Some(END),
        P1,
        STAKE_NORMAL,
        Some(LeanPick::Newest),
        END + 1,
        "g1",
    );
    assert_eq!(first, OfferOutcome::Waiting { answered: None });
    let second = book.offer(
        "m1",
        Some(END),
        P2,
        STAKE_NORMAL,
        Some(LeanPick::Set(SetName::Core)),
        END + 2,
        "g2",
    );
    let OfferOutcome::Claimed { match_id, leans, .. } = second else {
        panic!("{second:?}")
    };
    assert_eq!(match_id, "g2");
    assert_eq!(
        leans,
        (Some(LeanPick::Newest), Some(LeanPick::Set(SetName::Core)))
    );
    // A third offer meets the game already made, and makes no second.
    let third = book.offer("m1", Some(END), P1, STAKE_NORMAL, None, END + 3, "g3");
    assert_eq!(
        third,
        OfferOutcome::Waiting {
            answered: Some("g2".to_string())
        }
    );
    let seen = book.seen("m1", P1, END + 3);
    assert_eq!(seen.match_id.as_deref(), Some("g2"));
    assert_eq!(
        (seen.you_offered, seen.opponent_offer),
        (Some(STAKE_NORMAL), Some(STAKE_NORMAL))
    );
}

#[test]
fn r672_unequal_stakes_wait_and_a_changed_mind_meets_the_other_offer() {
    let mut book = book_with(&[(P1, STAKE_NORMAL)], END + 1);
    let mismatch = book.offer("m1", Some(END), P2, STAKE_DOUBLE, None, END + 2, "g2");
    assert_eq!(mismatch, OfferOutcome::Waiting { answered: None });
    let seen = book.seen("m1", P2, END + 2);
    assert_eq!(
        (seen.you_offered, seen.opponent_offer),
        (Some(STAKE_DOUBLE), Some(STAKE_NORMAL))
    );
    assert_eq!(seen.match_id, None);
    let agreed = book.offer("m1", Some(END), P2, STAKE_NORMAL, None, END + 3, "g3");
    assert!(matches!(agreed, OfferOutcome::Claimed { .. }), "{agreed:?}");
}

#[test]
fn r2685000_an_offer_past_the_finished_matchs_window_lapses_and_records_nothing() {
    let mut book = RematchBook::default();
    let late = END + REMATCH_OFFER_TTL_MS + 1;
    assert_eq!(
        book.offer("m1", Some(END), P1, STAKE_NORMAL, None, late, "g1"),
        OfferOutcome::Lapsed { answered: None }
    );
    assert!(book.is_empty());
    // A finished row with no end is outside the window.
    assert_eq!(
        book.offer("m1", None, P1, STAKE_NORMAL, None, END, "g1"),
        OfferOutcome::Lapsed { answered: None }
    );
    assert!(book.is_empty());
}

#[test]
fn r2685000_an_old_offer_cannot_meet_a_new_one_but_a_game_made_inside_the_window_keeps_its_id() {
    // A's offer inside the window; B's matching one past it: no game, and A's entry is dropped.
    let mut book = book_with(&[(P1, STAKE_NORMAL)], END + 1);
    let late = END + REMATCH_OFFER_TTL_MS + 1;
    assert_eq!(
        book.offer("m1", Some(END), P2, STAKE_NORMAL, None, late, "g2"),
        OfferOutcome::Lapsed { answered: None }
    );
    assert!(book.is_empty());

    // An entry that made a game inside the window still answers where to go.
    let mut made = book_with(&[(P1, STAKE_NORMAL), (P2, STAKE_NORMAL)], END + 1);
    assert_eq!(
        made.offer("m1", Some(END), P1, STAKE_NORMAL, None, late, "g9"),
        OfferOutcome::Lapsed {
            answered: Some("g1".to_string())
        }
    );
    assert_eq!(made.len(), 1);
}

#[test]
fn a_failed_create_clears_the_claim_of_that_entry_only() {
    let mut book = RematchBook::default();
    book.offer("m1", Some(END), P1, STAKE_NORMAL, None, END + 1, "g1");
    let OfferOutcome::Claimed { generation, .. } =
        book.offer("m1", Some(END), P2, STAKE_NORMAL, None, END + 2, "g2")
    else {
        panic!("a game was claimed")
    };
    // Another entry's generation does not release it.
    book.release("m1", generation + 1);
    assert_eq!(book.seen("m1", P1, END + 3).match_id.as_deref(), Some("g2"));
    book.release("m1", generation);
    assert_eq!(book.seen("m1", P1, END + 3).match_id, None);
    // The seats re-offer and meet again.
    let again = book.offer("m1", Some(END), P1, STAKE_NORMAL, None, END + 4, "g3");
    assert!(matches!(again, OfferOutcome::Claimed { .. }), "{again:?}");
}

#[test]
fn an_entry_whose_latest_offer_is_stale_is_dropped_when_read_but_one_with_a_game_stays() {
    let mut book = book_with(&[(P1, STAKE_NORMAL)], END + 1);
    let stale = END + 1 + REMATCH_OFFER_TTL_MS + 1;
    assert_eq!(book.seen("m1", P1, stale).you_offered, None);
    assert!(book.is_empty());

    let mut made = book_with(&[(P1, STAKE_NORMAL), (P2, STAKE_NORMAL)], END + 1);
    assert_eq!(made.seen("m1", P1, stale).match_id.as_deref(), Some("g1"));
    assert_eq!(made.len(), 1);
}

#[test]
fn the_sweep_drops_old_entries_with_no_live_host_and_keeps_the_rest() {
    let mut book = book_with(&[(P1, STAKE_NORMAL)], END + 1);
    book.offer("m2", Some(END), P1, STAKE_NORMAL, None, END + 1, "g1");
    let stale = END + 1 + REMATCH_OFFER_TTL_MS + 1;
    assert_eq!(book.sweep(END + 2, |_| false), 0);
    assert_eq!(book.sweep(stale, |id| id == "m2"), 1);
    assert_eq!(book.len(), 1);
    assert_eq!(book.sweep(stale, |_| false), 1);
    assert!(book.is_empty());
}

#[test]
fn the_book_reads_back_from_json() {
    let mut book = book_with(&[(P1, STAKE_NORMAL)], END + 1);
    book.offer(
        "m2",
        Some(END),
        P2,
        STAKE_DOUBLE,
        Some(LeanPick::Newest),
        END + 1,
        "g1",
    );
    let json = serde_json::to_string(&book).expect("the book serializes");
    let back: RematchBook = serde_json::from_str(&json).expect("the book reads back");
    assert_eq!(back, book);
}

fn finished() -> FinishedSeats {
    let slots = |name: &str| DeckSlots {
        table: Some(format!("table-{name}")),
        field: None,
        card_back: Some(format!("back-{name}")),
    };
    FinishedSeats {
        players: ("alice".to_string(), "bob".to_string()),
        decks: (
            vec!["a1".to_string(), "a2".to_string()],
            vec!["b1".to_string(), "b2".to_string()],
        ),
        portraits: Some(("vanilla".to_string(), "nightowl".to_string())),
        slots: Some((slots("alice"), slots("bob"))),
    }
}

#[test]
fn r1989000_a_best_of_1_rematch_swaps_the_seats_with_each_profiles_deck_portrait_and_slots() {
    let (first, second) = next_seats(&finished(), false, "seed", (None, None));
    assert_eq!((first.profile_id.as_str(), first.player), ("bob", P1));
    assert_eq!((second.profile_id.as_str(), second.player), ("alice", P2));
    assert_eq!(first.deck, vec!["b1", "b2"]);
    assert_eq!(second.deck, vec!["a1", "a2"]);
    assert_eq!(first.portrait.as_deref(), Some("nightowl"));
    assert_eq!(second.portrait.as_deref(), Some("vanilla"));
    assert_eq!(
        first.deck_slots.as_ref().and_then(|slots| slots.table.as_deref()),
        Some("table-bob")
    );
    assert_eq!(
        second
            .deck_slots
            .as_ref()
            .and_then(|slots| slots.card_back.as_deref()),
        Some("back-alice")
    );
    // A match that froze no portraits or loadouts replays none.
    let bare = FinishedSeats {
        portraits: None,
        slots: None,
        ..finished()
    };
    let (first, second) = next_seats(&bare, false, "seed", (None, None));
    assert_eq!(
        (
            first.portrait,
            first.deck_slots,
            second.portrait,
            second.deck_slots
        ),
        (None, None, None, None)
    );
}

#[test]
fn r1372_r924000_an_all_random_rematch_deals_fresh_decks_and_portraits_and_wears_no_frozen_slots() {
    jackioh_cards::register_all();
    let (first, second) = next_seats(&finished(), true, "seed", (Some(LeanPick::Newest), None));
    assert_eq!(
        (first.profile_id.as_str(), second.profile_id.as_str()),
        ("bob", "alice")
    );
    assert_eq!(first.deck.len(), jackioh_engine::DECK_SIZE as usize);
    assert_eq!(second.deck.len(), jackioh_engine::DECK_SIZE as usize);
    assert_ne!(first.deck, vec!["b1", "b2"]);
    assert!(first.portrait.is_some() && second.portrait.is_some());
    assert_eq!((first.deck_slots, second.deck_slots), (None, None));
    // The same seed deals the same decks, and the new p1 leans on the old p2's pick.
    let again = next_seats(&finished(), true, "seed", (Some(LeanPick::Newest), None));
    assert_eq!(
        (first.deck.clone(), second.deck.clone()),
        (again.0.deck, again.1.deck)
    );
    let other = next_seats(
        &finished(),
        true,
        "seed",
        (None, Some(LeanPick::Set(SetName::Core))),
    );
    assert_ne!(other.0.deck, first.deck);
}
