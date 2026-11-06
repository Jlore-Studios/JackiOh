//! The collection economy's decisions (SPEC §14, #3104), pure: what a card pack and a cosmetic pack grant,
//! how pity and the token bars move, what a set-track level pays, what a finished game pays and how a quest moves, each over plain records the store
//! read and answered as grants and new records for the store to write. No store, no clock, no I/O
//! (CLAUDE.md rule 4); `crates/shared/README.md`'s host contract binds every module here.
//!
//! - [`open`]: the two decisions a host makes around an opening, [`open::open_card_pack`] and
//!   [`open::open_cosmetic_pack`], `decide(records, inputs, seed, now_ms) -> outcome` (§14.3, §14.6; R1301003,
//!   R1306003).
//! - [`roll`]: the card pack roll (§14.3, §14.4; #1301, #1339), from a pack's pool, the account's
//!   holdings and a seed to what each slot grants and the next pity counters (R1301003, R1339002).
//! - [`cosmetic_pack`]: the cosmetic pack roll (§14.6; #1306), the same shape on the cosmetics' scale.
//! - [`tiers`]: the tiered roll every drop goes through (§14.3, §14.6; R1339000).
//! - [`math`]: pity (§14.4, R1298000, R1339001) and the set-track level curve (§14.7).
//! - [`bars`]: the token bars (§14.5; R1305000, R1953000, R2728000): the points a slot adds, a bar settled
//!   into the tokens it pays, the cosmetic cycle's steps, and a dev account's fill of a bar (R1310003).
//! - [`track_rewards`]: what reaching a set-track level pays (§14.7, #1302).
//! - [`rewards`]: what a finished game pays (§14.7 to §14.9, R1483000, R1302006): [`rewards::pay_game`] over the
//!   records the store read, the smaller [`rewards::pay_xp`], [`rewards::starting_rewards`] and
//!   [`rewards::complete_quest`], and the rates, the cap and the track curve they decide with.
//! - [`quests`]: the quest pool (`quests.json`, compiled in), what a finished game counts toward each quest
//!   and the decisions over an account's quests: a top-up, a replace, a dev give (R1304000, R2210000).
//! - [`ledger`]: the rows an outcome names: counters, inventory items, a track, a quest and the
//!   [`ledger::LedgerGrant`]s.
//! - [`boundary`]: `jackioh-wasm-core`'s `economy(call)`, one [`boundary::EconomyCall`] in and one
//!   [`boundary::EconomyAnswer`] out as JSON.

pub mod bars;
pub mod boundary;
pub mod cosmetic_pack;
pub mod ledger;
pub mod math;
pub mod open;
pub mod quests;
pub mod rewards;
pub mod roll;
#[cfg(test)]
mod test_support;
pub mod tiers;
pub mod track_rewards;
