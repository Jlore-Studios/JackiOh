//! The ladder bots' I/O (SPEC §9.5, §9.12, R1453, R1454, R1451, R1452, R641000; #638, #3103): what the
//! server does for the bots the queue seats in a ranked match or Conquest series when no person is
//! available. What a bot decides is `jackioh_shared::bots` (and, at its seat, the match host's machine,
//! `jackioh_shared::host`), pure; this module reads the store and writes it.
//!
//! - `pool`: the active bot lifecycle (R1454): the refresh that sizes the pool and deals decks, trios and
//!   grinders, and the store rows as the pure decisions read them.
//! - `cosmetics`: loadout (R1834005), which waits for the cosmetics registry's move into
//!   `jackioh-shared` (#3104); `calibrate`: seed ratings (R748000).
//!
//! A bot is a profile of kind `bot` (migration 0032) with a `bots` row and an R610 rating.
//! Nothing a client reads tells its seat from a person's (CLAUDE.md rule 7, R1452).

pub mod calibrate;
pub mod cosmetics;
pub mod pool;
