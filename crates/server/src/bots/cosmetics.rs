//! The ladder bots' cosmetics (R1834005; #1834): a bot wears a loadout drawn as a person's, so its
//! seat never stands out (R1452), from a wardrobe drawn from the pool's seed, never a collection
//! (R1454: it owns nothing and earns nothing).

use serde::{Deserialize, Serialize};

use crate::economy::store::{CardPicks, OwnedCosmetics};
use crate::loadout::AccountPicks;

/// R1834005: what a bot may wear and what it wears (`bots.cosmetics`, migration 0058): its
/// wardrobe, the cosmetics it holds past the starters, in the ledger's shape but never in the ledger;
/// its account picks, as a person's settings hold theirs; its cards' own picks; and when it last drew
/// them (epoch ms). Its decks' slots ride on the decks (`BotDeck::slots`, `FrozenDeck::slots`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BotCosmetics {
    pub wardrobe: OwnedCosmetics,
    pub account: AccountPicks,
    #[serde(default)]
    pub cards: Vec<CardPicks>,
    pub drawn_at: i64,
}
