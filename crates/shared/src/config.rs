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

/// #2437, R637, R2179001: the Temporary count at which a card in a hand is discarded at its owner's next end of
/// turn. A plain Temporary is Temporary 1; the face model's Temporary badge counts above it (`face/state.rs`).
pub const TEMPORARY_LAST_COUNT: i32 = 1;

/// Glitch (issue #170, SPEC §7) prints corrupted words (`face/glitch.rs`): the fixed seed every host jumbles with,
/// so every client prints the same jumble.
pub const GLITCH_SEED: u32 = 170;
/// Out of [`GLITCH_ODDS_SCALE`]: below this a Glitch character becomes a glyph.
pub const GLITCH_SWAP_IN: u32 = 45;
/// Out of [`GLITCH_ODDS_SCALE`]: below `GLITCH_SWAP_IN` plus this a Glitch character gains a combining mark.
pub const GLITCH_MARK_IN: u32 = 35;
pub const GLITCH_ODDS_SCALE: u32 = 100;

// ---------------------------------------------------------------------------------------------
// The match host's numbers (#3101): the clocks (SPEC §9.5, R79, R268, R744), the relays (R738) and
// the rematch window (R672, R2685000) that `host` decides with. `crates/server/src/config.rs`
// re-exports them, so `serverConfig.ts` and the server's code name them as before.
// ---------------------------------------------------------------------------------------------

/// R79: the turn clock belongs to the active player and ends a stalled turn.
pub const TURN_CLOCK_SECONDS: i64 = 75;
/// R79: a prompt held by the non-active player (e.g. a trap firing on the opponent's turn).
pub const PROMPT_CLOCK_SECONDS: i64 = 30;
/// SPEC §11 R268: the mulligan clock. Both seats' mulligans are open at once (R265), so one deadline
/// runs for both, armed when the window opens and never re-armed when one seat answers; on expiry
/// every seat still owing is timed out and keeps its whole hand. Longer than the prompt clock because
/// a mulligan reads a whole opening hand, shorter than the turn clock because nothing is played.
pub const MULLIGAN_CLOCK_SECONDS: i64 = 45;
/// R79: grace window after a disconnect before `disconnectExpired` ends the match as a loss.
pub const DISCONNECT_GRACE_SECONDS: i64 = 60;
/// R79, R389: hard wall-clock ceiling; reaching it ends the match as a draw via `ceilingReached`.
/// Patch v0.2.0 doubled the turn cap to 60 player-turns (B4.3), which at a full turn clock is 75
/// minutes, so the ceiling doubled with it and a slow game ends on the cap, not the clock.
pub const MATCH_CEILING_MINUTES: i64 = 120;
/// SPEC §11 R776000: how long a clock's server action whose log row the store failed to write waits
/// before its clock runs out again, so a store that keeps failing is retried at this pace until the
/// row lands, the match ends or the actor stops, never in a loop.
pub const MATCH_SERVER_ACTION_RETRY_MS: i64 = 1_000;
/// R1452: how long a bot waits to decide again when its move could not be written to the log, so a
/// store that keeps failing is retried at this pace, never in a loop.
pub const BOT_WRITE_RETRY_MS: i64 = 1_000;
/// SPEC §11 R738: the least time between two `aim` frames the host relays for one seat. Aims that
/// arrive faster are coalesced, never queued: the newest waits out the interval and goes alone, so
/// the opponent's arrow always ends where the sender's aim ended, at most this long behind it.
/// Ten a second follows a hover from target to target and bounds what a scripted client can push.
/// R2739001's `choiceHover` relays are coalesced per seat by the same interval, on a window of their
/// own.
pub const AIM_RELAY_INTERVAL_MS: i64 = 100;
/// SPEC §11 R672: how long one seat's rematch offer stands after a non-series match ends. Ten
/// minutes: long enough to read the result screen and press the button, short enough that a stale
/// offer cannot summon a game long after both players moved on.
pub const REMATCH_OFFER_TTL_MS: i64 = 600_000;
/// R672: a normal rematch moves the rating once.
pub const STAKE_NORMAL: i32 = 1;
/// R672: a double-or-nothing rematch moves it twice. Ranked matches only.
pub const STAKE_DOUBLE: i32 = 2;
