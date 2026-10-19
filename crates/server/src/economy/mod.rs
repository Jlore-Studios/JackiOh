//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. Nothing calls it yet; its parts are built behind `ECONOMY_LIVE`
//! (§14.12).
//!
//! - `store`: migration 0045's tables, each holding beside its append-only ledger (§14.11).

pub mod store;
