//! The shared logic's numbers (CLAUDE.md rule 9): the server re-exports the ones its own code and
//! `serverConfig.ts` name (`crates/server/src/config.rs`), and the hosts that call this crate through
//! `jackioh-wasm-core` read them from its `constants` export, so none restates one.

/// Milliseconds in a second: rows and clocks are stamped in epoch milliseconds, and the series' pick
/// clock is stated in seconds.
pub const MS_PER_SECOND: i64 = 1000;

/// SPEC §11 R330: game wins that take a Conquest series — one with each deck of a trio, since a deck
/// that has won is locked. `tests/series.rs` asserts it equals the validator's
/// `TRIO_DECKS`. (R259's Best of 3 needed 2.)
pub const SERIES_WINS_NEEDED: i32 = 3;
/// SPEC §11 R334: the most games a series plays, drawn games included. Without a draw a series is
/// decided by its fifth game at the latest (2 × `SERIES_WINS_NEEDED` − 1: a side is then at three
/// wins), so the cap leaves room for two drawn games in the longest series and stops a run of draws
/// from holding both players for ever. At the cap more wins takes the series and equal wins is a
/// series draw. (R259's Best of 3 played at most 3.)
pub const SERIES_MAX_GAMES: i32 = 7;
/// SPEC §11 R333: how long both players have to pick their deck for the next game of a series.
pub const SERIES_PICK_SECONDS: i64 = 60;
