//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. The ladder's payout (`rewards`), practice's (`practice`) and a new
//! account's starting rewards run only while `App::economy_live` is on (`ECONOMY_LIVE`, §14.12;
//! #1310, R1310000). The dev tools (`api/dev.rs`, §14.10) write a dev account's own, live or not.
//!
//! Its modules: `math`, the set-track level curve (§14.7) and pity (§14.4) (#1298, #1339); `tiers`, the
//! tiered roll every drop goes through (§14.3, §14.6, #1339); `quests`, the daily quests (§14.9, #1304);
//! `store`, migration 0045's holdings and ledgers (§14.11) and 0047's practice seeds (§14.8); `rewards`, the
//! one reward hook (§14.8, #1302); `practice`, a practice claim's check (§14.8, #1303); `track_rewards`, a
//! set track's reward table (§14.7, #1302); `roll`, the card pack roll (§14.3, #1301); `packs`, buying and
//! opening card packs (#1301); `tokens`, the token bars and their tokens (§14.5, #1305); `cosmetic_packs`,
//! cosmetic packs and the Chase Shard bar (§14.6, #1306); `idempotency`, a spend's `Idempotency-Key`
//! (§14.11, #2124, R2124000); and `spend_slots`, one spend at a time per account (§14.11, #2378, R2378000).

pub mod cosmetic_packs;
pub mod idempotency;
pub mod math;
pub mod packs;
pub mod practice;
pub mod quests;
pub mod rewards;
pub mod roll;
pub mod spend_slots;
pub mod store;
#[cfg(test)]
mod test_support;
pub mod tiers;
pub mod tokens;
pub mod track_rewards;
