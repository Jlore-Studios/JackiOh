//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. Its parts are built behind `ECONOMY_LIVE` (§14.12): the ladder's
//! payout (`rewards`) runs only while `App::economy_live` is on, and it is off until the going-live
//! sub-issue. The dev tools (`api/dev.rs`, §14.10) write a dev account's own, live or not. Until
//! the switch is on, nothing else earns Jlore or grants a pack, so the pack routes, though served,
//! change only what a dev account owns.
//!
//! - `math`: the pure math, the set-track level curve (§14.7) and the pity roll (§14.4) (#1298).
//! - `store`: migration 0045's tables, each holding beside its append-only ledger (§14.11).
//! - `rewards`: the one reward hook a finished game pays through (§14.8, #1302).
//! - `track_rewards`: a set track's reward table, kept as data (§14.7, #1302).
//! - `roll`: the card pack roll, pure from a pool, the holdings and a seed: the slots' rarities,
//!   the card draw and the dupe chain (§14.3, #1301).
//! - `packs`: buying and opening card packs, each in one transaction, and their routes (#1301).
//! - `tokens`: the token bars and their tokens: what a full bar pays, what a token buys and what
//!   both read back (§14.5, #1305).

pub mod math;
pub mod packs;
pub mod rewards;
pub mod roll;
pub mod store;
pub mod tokens;
pub mod track_rewards;
