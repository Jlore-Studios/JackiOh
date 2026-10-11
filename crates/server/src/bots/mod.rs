//! The ladder bots (SPEC §9.5, §9.12, R1447–R1449, R1451; #638): the opponents the queue seats in a ranked
//! Best of 1 or All Random match when no person is there to pair with.
//!
//! - `persona`: their emotes, the server's port of the practice AI's R645 personas (R1344).
//! - `seat`: what the match actor runs when a bot owes a move: the decision and the think time.
//! - `fallback`: which waiting ticket pairs with a bot, and with which (R1448).
//! - `pool`: how many bots there are, who they are and what they play (R1449).
//!
//! A bot is a profile of kind `bot` (server-only, migration 0032) with a `bots` row and an R610
//! rating. Nothing a client reads tells its seat from a person's (CLAUDE.md rule 7, R1447).

pub mod fallback;
pub mod persona;
pub mod pool;
pub mod seat;
