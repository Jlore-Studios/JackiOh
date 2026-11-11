//! The practice session (#3099, #2956 Part B; SPEC §9.9, §9.10): a game against the AI seat as a mode of
//! the host machine, in place of the web's practice core and the server's copy of it. `README.md` is the
//! contract; this file is the map.
//!
//!  - [`types`] is what crosses the boundary: the config a game is dealt from, the snapshot the page is
//!    told, the save, the kept game, the claim and the record of a game's end.
//!  - [`deal`] is practice's own setup (§9.9, §9.10, R433, R1373, R2706000): the decks, the handicaps and
//!    what `create_game` takes.
//!  - [`host`] is the machine: [`PracticeHost`], [`PracticeEvent`], [`PracticeOutput`] and [`step`].
//!  - [`replays`] is the games a host keeps (R768): which fold, how one is opened and paged.
//!  - [`claim`] is the server's check of a claim, by stepping the machine over the claimed game.
//!  - [`boundary`] is the one JSON call the browser and Roblox make.

pub mod boundary;
pub mod claim;
pub mod deal;
pub mod host;
pub mod replays;
pub mod types;

pub use claim::{Checked, ClaimRefusal, check_claim};
pub use host::{PracticeEvent, PracticeHost, PracticeOutput, ai_never_takes, catalog_of, step};
pub use types::*;
