//! R3558001, R3558002: a seat's own waiting meets and lends survive `redact`, the opponent's do
//! not — a meet and a lend are their side's hand order made legible, which no seat reads of the
//! other hand (§9.1).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, dealt_game};

fn with_meets_and_lends() -> GameState {
    let mut state = dealt_game("ai-hand-order");
    state.hand_meets = Some(vec![
        HandMeet {
            player: AI,
            left: "c1".to_string(),
            right: "c2".to_string(),
            cost: Some(0),
        },
        HandMeet {
            player: HUMAN,
            left: "c3".to_string(),
            right: "c4".to_string(),
            cost: None,
        },
    ]);
    state.radiance_lends = Some(vec![
        RadianceLend {
            player: AI,
            lender: "c5".to_string(),
            card: "c6".to_string(),
        },
        RadianceLend {
            player: HUMAN,
            lender: "c7".to_string(),
            card: "c8".to_string(),
        },
    ]);
    state
}

/// R3558001, R3558002 redact keeps the seat's own lends and meets and drops the opponent's.
#[test]
fn r3558001_r3558002_redact_drops_the_opponents_lends_and_meets() {
    let redacted = redact(&with_meets_and_lends(), AI);
    assert!(
        redacted
            .hand_meets
            .as_ref()
            .is_some_and(|meets| meets.len() == 1 && meets[0].player == AI)
    );
    assert!(
        redacted
            .radiance_lends
            .as_ref()
            .is_some_and(|lends| lends.len() == 1 && lends[0].player == AI)
    );

    let mirrored = redact(&with_meets_and_lends(), HUMAN);
    assert!(
        mirrored
            .hand_meets
            .as_ref()
            .is_some_and(|meets| meets.len() == 1 && meets[0].player == HUMAN)
    );
    assert!(
        mirrored
            .radiance_lends
            .as_ref()
            .is_some_and(|lends| lends.len() == 1 && lends[0].player == HUMAN)
    );
}

/// R3558001, R3558002 redact empties to `None`: a seat with no entry of its own reads no list.
#[test]
fn r3558001_r3558002_redact_with_no_entry_of_the_seats_own_reads_no_list() {
    let mut state = dealt_game("ai-hand-order-none");
    state.hand_meets = Some(vec![HandMeet {
        player: HUMAN,
        left: "c3".to_string(),
        right: "c4".to_string(),
        cost: None,
    }]);
    state.radiance_lends = Some(vec![RadianceLend {
        player: HUMAN,
        lender: "c7".to_string(),
        card: "c8".to_string(),
    }]);
    let redacted = redact(&state, AI);
    assert_eq!(redacted.hand_meets, None);
    assert_eq!(redacted.radiance_lends, None);
}
