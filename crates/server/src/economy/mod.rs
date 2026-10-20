//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. Its parts are built behind `ECONOMY_LIVE` (§14.12): the ladder's
//! payout (`rewards`) runs only while `App::economy_live` is on, and it is off until the going-live
//! sub-issue.
//!
//! - `math`: the pure math, the set-track level curve (§14.7) and the pity roll (§14.4) (#1298).
//! - `store`: migration 0045's tables, each holding beside its append-only ledger (§14.11).
//! - `rewards`: the one reward hook a finished game pays through (§14.8, #1302).
//! - `track_rewards`: a set track's reward table, kept as data (§14.7, #1302).

pub mod math;
pub mod rewards;
pub mod store;
pub mod track_rewards;
