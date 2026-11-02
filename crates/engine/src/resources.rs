//! Stinkyscape's Resources and Credits (#1840, R1840000–R1840006; #2250, R2250000, R2250001;
//! docs/stinkyscape-set.md ST3, ST5).
//!
//! A Resource is a (0) Spell token whose definition carries `resource` (`CardDef.resource`): its
//! rarity and its prices. It is a hand card like any other (R2250000), and played or discarded from the
//! hand it pays its player Credits, its quote (R2250001).
//!
//! Credits (`PlayerState.market_credits`) are a public count per player, absent until first gained
//! (R1840005), named apart from Jlarna's credit line (R1223), and spent at the Marketplace (#1853),
//! which the first Resource or Credits either player gains opens (R1853000). The verbs a card script
//! composes are `effects::tray`'s; this module is the rule they share.

use crate::catalog::{find_def, registered_catalog};
use crate::config::RESOURCE_RARITY_ODDS;
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, new_instance};
use crate::wire::{GameEvent, PlayerId, PrintedRarity, ResourcePrice, Zone};

/// R1840002: the ids of every Resource the catalog holds, shipped or not (R1420's set gate does not
/// reach it), sorted.
pub fn resource_def_ids() -> Vec<String> {
    let mut ids: Vec<String> = registered_catalog()
        .values()
        .filter(|def| def.resource.is_some())
        .map(|def| def.id.clone())
        .collect();
    ids.sort();
    ids
}

/// R1840002: the rarity a Resource's definition rolls as, `None` for a card that is no Resource.
fn resource_rarity(state: &GameState, def_id: &str) -> Option<PrintedRarity> {
    find_def(Some(state), def_id)
        .and_then(|def| def.resource)
        .map(|resource| resource.rarity)
}

/// R1840002: one random Resource's id, of the catalog's or only those `pool` names: the odds over the
/// rarities they hold (one draw, walked in the table's order as `catalog::roll_weighted` walks its),
/// then one of that rarity, uniformly. With no candidate, `None` and no draw.
pub fn roll_resource(state: &GameState, rng: &mut Rng, pool: Option<&[String]>) -> Option<String> {
    let candidates: Vec<(String, PrintedRarity)> = resource_def_ids()
        .into_iter()
        .filter(|id| pool.is_none_or(|pool| pool.contains(id)))
        // R2201001: a held game rolls no Legendary or Mythic Resource.
        .filter(|id| !find_def(Some(state), id).is_some_and(crate::catalog::held_out))
        .filter_map(|id| resource_rarity(state, &id).map(|rarity| (id, rarity)))
        .collect();
    let table: Vec<(PrintedRarity, i32)> = RESOURCE_RARITY_ODDS
        .iter()
        .copied()
        .filter(|(rarity, _)| candidates.iter().any(|(_, held)| held == rarity))
        .collect();
    let (last, _) = *table.last()?;
    let total: i32 = table.iter().map(|(_, weight)| weight).sum();
    let mut at = rng.int(total);
    let mut rolled = last;
    for (rarity, weight) in &table {
        if at < *weight {
            rolled = *rarity;
            break;
        }
        at -= weight;
    }
    let of_rarity: Vec<&String> = candidates
        .iter()
        .filter(|(_, rarity)| *rarity == rolled)
        .map(|(id, _)| id)
        .collect();
    rng.pick(&of_rarity).map(|id| (*id).clone())
}

/// R1840001, R1853002: a whole number drawn uniformly in one face's sell range.
pub fn draw_sell_price(rng: &mut Rng, price: ResourcePrice) -> i32 {
    price.sell_min + rng.int(price.sell_max.max(price.sell_min) - price.sell_min + 1)
}

/// R1840001, R2250001: what a Resource fetches now: its quote this turn while the Marketplace is open,
/// with no draw (R1853002); otherwise a draw in its face's sell range. 0, with no draw, for no Resource.
pub fn sell_price(state: &GameState, rng: &mut Rng, card: &CardInstance) -> i32 {
    if let Some(quote) = crate::marketplace::sell_quote(state, card) {
        return quote;
    }
    let Some(resource) = find_def(Some(state), &card.def_id).and_then(|def| def.resource) else {
        return 0;
    };
    let price = if card.radiant {
        resource.radiant
    } else {
        resource.base
    };
    draw_sell_price(rng, price)
}

/// R1840005: raise `player`'s Credits by `amount`, reported by the public `marketCreditsChanged`. A gain
/// of 0 or less changes and reports nothing, so Credits stay absent until first gained.
pub fn add_market_credits(sink: &mut EngineSink<'_>, player: PlayerId, amount: i32) {
    if amount <= 0 {
        return;
    }
    crate::marketplace::open_marketplace(sink);
    let value = sink.state.players[player].market_credits.unwrap_or(0) + amount;
    sink.state.players[player].market_credits = Some(value);
    sink.events
        .push(GameEvent::MarketCreditsChanged { player, value });
}

/// R1853003: `player`'s Credits fall by `amount`, a price the Marketplace refused unless they held it.
pub fn spend_market_credits(sink: &mut EngineSink<'_>, player: PlayerId, amount: i32) {
    let value = sink.state.players[player].market_credits.unwrap_or(0) - amount;
    sink.state.players[player].market_credits = Some(value);
    sink.events
        .push(GameEvent::MarketCreditsChanged { player, value });
}

/// R1840000, R2250000: `player` gains a fresh Resource of `def_id`, opening the Marketplace first, into
/// their hand as any card is (hidden from the other player as a gain is, R97); a full hand burns it (R4).
pub fn gain_resource(sink: &mut EngineSink<'_>, player: PlayerId, def_id: &str, radiant: bool) {
    crate::marketplace::open_marketplace(sink);
    let mut card = new_instance(&mut *sink.state, def_id, player, Zone::Hand { player });
    card.radiant = radiant;
    let _ = crate::draw::add_to_hand(sink, &mut card);
}

/// R2250000, R2250001: whether a card is a Resource (its definition carries `resource`).
pub fn is_resource_card(state: &GameState, card: &CardInstance) -> bool {
    find_def(Some(state), &card.def_id).is_some_and(|def| def.resource.is_some())
}

/// R2250001: a Resource `player` played or discarded from their hand pays them its Credits
/// (`sell_price`). A card that is no Resource pays nothing and draws nothing.
pub fn pay_resource_credits(sink: &mut EngineSink<'_>, player: PlayerId, card: &CardInstance) {
    if !is_resource_card(sink.state, card) {
        return;
    }
    let credits = sell_price(sink.state, sink.rng, card);
    add_market_credits(sink, player, credits);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R1840002: the table's weights are the percents the rule prints.
    #[test]
    fn the_rarity_odds_add_to_100() {
        let total: i32 = RESOURCE_RARITY_ODDS.iter().map(|(_, weight)| weight).sum();
        assert_eq!(total, 100);
        assert_eq!(RESOURCE_RARITY_ODDS.len(), PrintedRarity::ALL.len());
    }
}
