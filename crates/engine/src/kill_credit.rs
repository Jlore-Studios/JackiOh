//! A kill credited to another unit (R42, R412): Classic+ #19.2 Jungle Loser's Radiant face credits the
//! kill its attack makes to Classic+ #19.5 Bot Loser, whose "Whenever this destroys a Unit" then fires.
//!
//! R42's killer is set the moment a hit dooms its victim (`damage::credit_killer`), and it is what the
//! `destroyed` event names and every kill trigger reads. A credit moves that name: while one is in
//! force on the striking unit, its lethal hit on the named victim names the paired unit instead.
//! Nothing else about the hit changes — its source is still the striker. The record is plain JSON in
//! the striker's `memory` (under a key no card script writes), written and cleared around one effect by
//! `effects/kill_credit.rs`, and R78's reset takes it off a unit that leaves the field.
//!
//! Port of `packages/engine/src/killCredit.ts`.

use serde::{Deserialize, Serialize};

use crate::state::CardInstance;

pub const KILL_CREDIT_KEY: &str = "@killCredit";

/// One credit: a lethal hit on `victimId` is credited to `toId`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct KillCredit {
    pub victim_id: String,
    pub to_id: String,
}

/// R42, R412: whom a lethal hit by `source` on `victim` names as its killer.
///
/// The memory bag is read defensively (SURFACE §4.4.10): a list of `{ victimId, toId }` records,
/// the first naming the victim wins; anything else there is no credit.
pub fn credited_killer_id(source: &CardInstance, victim: &CardInstance) -> String {
    let credit = source
        .memory
        .get(KILL_CREDIT_KEY)
        .and_then(|credits| credits.as_array())
        .and_then(|credits| {
            credits
                .iter()
                .find(|each| each.get("victimId").and_then(|id| id.as_str()) == Some(victim.id.as_str()))
        });
    match credit.and_then(|credit| credit.get("toId")).and_then(|id| id.as_str()) {
        Some(to_id) => to_id.to_string(),
        None => source.id.clone(),
    }
}
