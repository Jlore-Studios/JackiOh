//! The ladder bots (SPEC §9.5, §9.12, R1453, R1454, R1451, R1452, R641000; #638): opponents the
//! queue seats in a ranked match or Conquest series when no person is available.
//!
//! - `persona`: emotes (R645, R1344); `seat`: decision and think time; `signals`: habits (R639000).
//! - `aim`: aim arrow (R738); `calibrate`: seed ratings (R748000); `cosmetics`: loadout (R1834005).
//! - `fallback`: pairing (R1453); `names`: username (R1454); `pick`: Conquest deck (R641000).
//! - `pool`: active bot lifecycle, decks, and trio (R1454, R641000).
//!
//! A bot is a profile of kind `bot` (migration 0032) with a `bots` row and an R610 rating.
//! Nothing a client reads tells its seat from a person's (CLAUDE.md rule 7, R1452).

pub mod aim;
pub mod calibrate;
pub mod cosmetics;
pub mod fallback;
pub mod names;
pub mod persona;
pub mod pick;
pub mod pool;
pub mod seat;
pub mod signals;
