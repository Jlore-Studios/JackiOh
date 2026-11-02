//! Duos co-op (#936, fullsend SPEC S4): two players against two, as two ordinary 1v1 games side by
//! side — duel 0 is A1 (`p1`) against B1 (`p2`), duel 1 is A2 (`p1`) against B2 (`p2`). Every card in
//! either still sees a 1v1 game, so no card script changes. The wrapper adds three things: a life pool
//! (own HP plus a team pool, `duos_life.rs`, B12–B18, copied to the other duel after every step, B15,
//! and the game judged, B16); passing a hand card to the teammate's hand (B19–B22); and turn sync, a
//! round being one turn in each duel: both round seats act at once, End turn only marks a seat ended,
//! and once both have it is applied to duel 0, then duel 1 (B4–B9; R82's automatic end is off, the
//! wrapper applying it to round seats, B9).
//!
//! `legal` shares its refusals with the reducer, so a greyed-out button and a refusal cannot disagree
//! (§9.3). A 1v1 game is untouched: every field Duos adds to a `GameState` is an `Option` absent outside
//! a duel, and the life helpers are plain arithmetic without one (B3).

mod legal;
mod reduce;
mod replay;
mod state;
mod view;

pub use legal::{duos_legal_actions, duos_seats_to_act};
pub use reduce::{DUOS_NONCE_PREFIX, duos_begin, duos_reduce, passed_card};
pub use replay::{DuosReplayInput, DuosReplayResult, duos_fold, duos_hash};
pub use state::{
    DuosBegunTurn, DuosReduceResult, DuosState, DuosTeam, duos_create, duos_create_recorded, duos_duel_seed,
};
pub use view::duos_view_for;

/// S4's testkit helper: a begun Duos game with every mulligan kept and round 1 open.
#[cfg(feature = "testkit")]
pub use crate::testkit::duos_scenario::duos_scenario;
