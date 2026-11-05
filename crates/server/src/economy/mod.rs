//! The economy (SPEC §14): Jlore, packs and tokens, pity and the bars, set tracks, daily quests and
//! the cosmetics an account owns. The ladder's payout (`rewards`), practice's (`practice`) and a new
//! account's starting rewards run only while `App::economy_live` is on (`ECONOMY_LIVE`, §14.12;
//! #1310, R1310000). The dev tools (`api/dev.rs`, §14.10) write a dev account's own, live or not.
//!
//! The rolls, pity, the token bars, the set-track curve and its reward table, and the cosmetics registry are
//! decided in `jackioh_shared` (`economy`, `cosmetics`; #3104): each is `decide(records, inputs, seed, now_ms)
//! -> outcome` over plain records, and this module reads the records from the store and writes the outcome.
//!
//! Its modules: `quests`, the daily quests (§14.9, #1304);
//! `store`, migration 0045's holdings and ledgers (§14.11) and 0047's practice seeds (§14.8); `grants`, an
//! outcome's grants written to them (#3104); `rewards`, the
//! one reward hook (§14.8, #1302); `practice`, a practice claim's check (§14.8, #1303); `packs`, buying and
//! opening card packs (#1301); `tokens`, the token bars and their tokens (§14.5, #1305); `cosmetic_packs`,
//! cosmetic packs and the Chase Shard bar (§14.6, #1306); `idempotency`, a spend's `Idempotency-Key`
//! (§14.11, #2124, R2124000); `spend_slots`, one spend at a time per account (§14.11, #2378, R2378000);
//! `news`, the news entries and the reward each pays once (§14.13, #2875, R2875000); and `proficiency`,
//! each card's plays and level, counted with a game's pay (§14.14, #3013, R3013000).

pub mod cosmetic_packs;
pub mod grants;
pub mod idempotency;
pub mod news;
pub mod packs;
pub mod practice;
pub mod proficiency;
pub mod quests;
pub mod rewards;
pub mod spend_slots;
pub mod store;
pub mod tokens;
