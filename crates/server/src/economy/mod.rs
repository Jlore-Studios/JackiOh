//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. Its parts are built behind `ECONOMY_LIVE` (§14.12): the ladder's
//! payout (`rewards`) and practice's (`practice`) run only while `App::economy_live` is on, and it is
//! off until the going-live sub-issue.
//!
//! - `math`: the pure math, the set-track level curve (§14.7) and the pity roll (§14.4) (#1298).
//! - `store`: migration 0045's tables, each holding beside its append-only ledger (§14.11), and
//!   0046's practice seeds (§14.8).
//! - `rewards`: the one reward hook a finished game pays through (§14.8, #1302).
//! - `practice`: a practice game's claimed log folded and its AI re-run before it pays (§14.8, #1303).
//! - `track_rewards`: a set track's reward table, kept as data (§14.7, #1302).

pub mod math;
pub mod practice;
pub mod rewards;
pub mod store;
pub mod track_rewards;
