//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. Until it goes live (§14.12), nothing earns Jlore or grants a
//! pack, so the pack routes, though served, change nothing an account owns.
//!
//! - `math`: the pure math, the set-track level curve (§14.7) and the pity roll (§14.4) (#1298).
//! - `store`: migration 0045's tables, each holding beside its append-only ledger (§14.11).
//! - `roll`: the card pack roll, pure from a pool, the holdings and a seed: the slots' rarities,
//!   the card draw and the dupe chain (§14.3, #1301).
//! - `packs`: buying and opening card packs, each in one transaction, and their routes (#1301).

pub mod math;
pub mod packs;
pub mod roll;
pub mod store;
