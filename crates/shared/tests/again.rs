//! Play again after a finished co-op match (fullsend SPEC §13.9, R2092000, R1398000), pure: the book a
//! host keeps beside the machine. The HTTP round trip is `crates/server/tests/api/duos_again.rs`'s.

use jackioh_engine::LeanPick;
use jackioh_engine::wire::DuosSeat::{A1, A2};
use jackioh_shared::config::REMATCH_OFFER_TTL_MS;
use jackioh_shared::host::again::{AgainBook, AgainClaim, in_again_window};

const END: i64 = 1_700_000_000_000;
const FINISHED: &str = "finished";

#[test]
fn r1398000_the_window_runs_from_the_end_to_the_ttl_inclusive() {
    assert!(in_again_window(END, END));
    assert!(in_again_window(END, END + REMATCH_OFFER_TTL_MS));
    assert!(!in_again_window(END, END + REMATCH_OFFER_TTL_MS + 1));
}

#[test]
fn r2092000_one_seat_waits_and_the_offer_that_completes_the_pair_makes_the_game_with_both_leans() {
    let mut book = AgainBook::default();
    assert_eq!(
        book.claim(FINISHED, A1, Some(LeanPick::Newest), END),
        AgainClaim::Done { answered: None }
    );
    let AgainClaim::Make { creating, leans } = book.claim(FINISHED, A2, None, END + 1) else {
        panic!("the offer that completes the pair makes the game")
    };
    assert_eq!(leans, (Some(LeanPick::Newest), None));
    // An offer landing mid-create waits on it instead of minting a second game.
    assert_eq!(
        book.claim(FINISHED, A1, None, END + 2),
        AgainClaim::Wait { creating }
    );
    book.made(FINISHED, creating, "new-match", END + 3);
    book.release(FINISHED, creating);
    assert_eq!(
        book.claim(FINISHED, A1, None, END + 4),
        AgainClaim::Done {
            answered: Some("new-match".to_string())
        }
    );
    let seen = book.seen(FINISHED, A2, END + 4);
    assert!(seen.you_offered && seen.teammate_offered);
    assert_eq!(seen.match_id.as_deref(), Some("new-match"));
}

#[test]
fn a_create_whose_claim_is_given_back_lets_the_next_offer_make_the_game_itself() {
    let mut book = AgainBook::default();
    book.claim(FINISHED, A1, None, END);
    let AgainClaim::Make { creating, .. } = book.claim(FINISHED, A2, None, END) else {
        panic!("makes the game")
    };
    // Another claim's number does not release it.
    book.release(FINISHED, creating + 1);
    assert_eq!(book.claim(FINISHED, A1, None, END), AgainClaim::Wait { creating });
    book.release(FINISHED, creating);
    assert!(matches!(
        book.claim(FINISHED, A1, None, END),
        AgainClaim::Make { .. }
    ));
}

#[test]
fn a_game_made_after_its_entry_was_swept_remakes_the_entry() {
    let mut book = AgainBook::default();
    book.claim(FINISHED, A1, None, END);
    let AgainClaim::Make { creating, .. } = book.claim(FINISHED, A2, None, END) else {
        panic!("makes the game")
    };
    assert_eq!(book.sweep(END + REMATCH_OFFER_TTL_MS + 1, |_| false), 1);
    book.made(FINISHED, creating, "g", END + REMATCH_OFFER_TTL_MS + 2);
    assert_eq!(
        book.seen(FINISHED, A1, END + REMATCH_OFFER_TTL_MS + 2)
            .match_id
            .as_deref(),
        Some("g")
    );
}

#[test]
fn r1398000_a_stale_offer_cannot_meet_a_new_one_and_a_lapsed_offer_keeps_only_a_made_game() {
    let mut book = AgainBook::default();
    book.claim(FINISHED, A1, None, END);
    let late = END + REMATCH_OFFER_TTL_MS + 1;
    // A's offer is older than the window by the time B offers: B starts afresh.
    assert_eq!(
        book.claim(FINISHED, A2, None, late),
        AgainClaim::Done { answered: None }
    );
    assert!(!book.seen(FINISHED, A1, late).teammate_offered || book.seen(FINISHED, A2, late).you_offered);
    // A lapsed offer drops an entry with no game, and keeps the id of one with.
    assert_eq!(book.lapse(FINISHED), None);
    assert!(book.is_empty());
    book.claim(FINISHED, A1, None, END);
    let AgainClaim::Make { creating, .. } = book.claim(FINISHED, A2, None, END) else {
        panic!("makes the game")
    };
    book.made(FINISHED, creating, "kept", END);
    assert_eq!(book.lapse(FINISHED).as_deref(), Some("kept"));
    assert_eq!(book.len(), 1);
}

#[test]
fn the_sweep_keeps_the_entries_a_live_host_holds() {
    let mut book = AgainBook::default();
    book.claim("a", A1, None, END);
    book.claim("b", A1, None, END);
    let stale = END + REMATCH_OFFER_TTL_MS + 1;
    assert_eq!(book.sweep(stale, |id| id == "b"), 1);
    assert_eq!(book.len(), 1);
}

#[test]
fn the_book_reads_back_from_json() {
    let mut book = AgainBook::default();
    book.claim(FINISHED, A1, Some(LeanPick::Newest), END);
    let json = serde_json::to_string(&book).expect("serializes");
    assert_eq!(
        serde_json::from_str::<AgainBook>(&json).expect("reads back"),
        book
    );
}
