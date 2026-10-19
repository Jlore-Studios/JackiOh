//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. Only the dev tools (`api/dev.rs`, §14.10) call it yet; its parts
//! are built behind `ECONOMY_LIVE` (§14.12).
//!
//! - `math`: the pure math, the set-track level curve (§14.7) and the pity roll (§14.4) (#1298).
//! - `store`: migration 0045's tables, each holding beside its append-only ledger (§14.11).

pub mod math;
pub mod store;
