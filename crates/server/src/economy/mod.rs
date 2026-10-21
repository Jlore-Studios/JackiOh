//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. Its parts were built behind `ECONOMY_LIVE` (§14.12): the ladder's
//! payout (`rewards`), practice's (`practice`) and a new account's starting rewards run only while
//! `App::economy_live` is on, which it is since the going-live sub-issue (#1310, R1310000). The dev
//! tools (`api/dev.rs`, §14.10) write a dev account's own, live or not.
//!
//! - `math`: the pure math, the set-track level curve (§14.7) and pity, forced on top of a card
//!   pack's rolled slots (§14.4) (#1298, #1339).
//! - `tiers`: the tiered roll every drop goes through, one weight table per roll, its lowest tier
//!   the fallback and a floor a fallback too (§14.3, §14.6, #1339).
//! - `quests`: the daily quests, their pool kept as data, their rhythm, progress and payout (§14.9,
//!   #1304).
//! - `store`: migration 0045's tables, each holding beside its append-only ledger (§14.11), and
//!   0047's practice seeds (§14.8).
//! - `rewards`: the one reward hook a finished game pays through (§14.8, #1302).
//! - `practice`: a practice game's claimed log folded and its AI re-run before it pays (§14.8, #1303).
//! - `track_rewards`: a set track's reward table, kept as data (§14.7, #1302).
//! - `roll`: the card pack roll, pure from a pool, the holdings and a seed: the slots' rarities,
//!   the card draw and the dupe chain (§14.3, #1301).
//! - `packs`: buying and opening card packs, each in one transaction, and their routes (#1301).
//! - `tokens`: the token bars and their tokens: what a full bar pays, what a token buys and what
//!   both read back (§14.5, #1305).
//! - `cosmetic_packs`: opening a cosmetic pack the cosmetic cycle paid, its roll pure from the
//!   holdings and a seed, the Chase Shard bar it fills and what the bar buys (§14.6, #1306).

pub mod cosmetic_packs;
pub mod math;
pub mod packs;
pub mod practice;
pub mod quests;
pub mod rewards;
pub mod roll;
pub mod store;
pub mod tiers;
pub mod tokens;
pub mod track_rewards;
