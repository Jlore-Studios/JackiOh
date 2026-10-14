//! Duos co-op (#936, fullsend SPEC S4): two players against two, as two ordinary 1v1 games side by
//! side — duel 0 is A1 (`p1`) against B1 (`p2`), duel 1 is A2 (`p1`) against B2 (`p2`). Duos is not a
//! four-seat engine: `DuosState` holds the two `GameState`s, and every card in either still sees a 1v1
//! game, so no card script changes. The wrapper adds exactly three things:
//!
//!   life pool   each seat has its own HP and its team shares a pool (`duos_life.rs`, B12–B18); after
//!               every step the wrapper copies each team's pool to the other duel (B15) and judges
//!               the game (B16).
//!   passing     a round seat may send a hand card to its teammate's hand for its team's pass cost
//!               (B19–B22).
//!   turn sync   a round is one turn in each duel. Both round seats act at once; a seat's End turn
//!               only marks it ended, and once both have, End turn is applied to duel 0, then duel 1,
//!               and the next round begins (B4–B9). A duel's players have R82's automatic turn end
//!               off, and the wrapper applies R82's rule to round seats itself (B9).
//!
//! The modules: `state` (the types and `duos_create`), `reduce` (`duos_begin`, `duos_reduce` and the
//! settle after every step), `legal` (`duos_legal_actions` and the refusals it shares with the
//! reducer, so a greyed-out button and a refusal cannot disagree, §9.3), `view` (`duos_view_for`) and
//! `replay` (`duos_hash`, `duos_fold`). The wire types are `crate::wire::duos`.
//!
//! A 1v1 game is untouched: every field Duos adds to a `GameState` is an `Option` absent outside a
//! duel, and the life helpers are today's arithmetic without one (B3).

mod legal;
mod reduce;
mod replay;
mod state;
mod view;

pub use legal::{duos_legal_actions, duos_seats_to_act};
pub use reduce::{duos_begin, duos_reduce};
pub use replay::{DuosReplayInput, DuosReplayResult, duos_fold, duos_hash};
pub use state::{DuosReduceResult, DuosState, DuosTeam, duos_create, duos_duel_seed};
pub use view::duos_view_for;

/// S4's testkit helper: a begun Duos game with every mulligan kept and round 1 open.
#[cfg(feature = "testkit")]
pub use crate::testkit::duos_scenario::duos_scenario;
