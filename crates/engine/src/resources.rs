//! Stinkyscape's Resources and Credits (#1840, R1840000–R1840006; docs/stinkyscape-set.md ST3, ST5).
//!
//! A Resource is a (0) Spell token whose definition carries `resource` (`CardDef.resource`): its
//! rarity and its prices. It never takes a hand slot: each player has a Resource tray beside the hand
//! (`PlayerState.resource_tray`, the zone `Zone::Tray`), holding at most `RESOURCE_TRAY_CAP`, which
//! no hand rule reaches — not the hand cap, a discard, a steal, a "fill your hand", a draw, the
//! mulligan or a pass (R1840000). A Resource gained into a full tray is sold at once for a price drawn
//! in its face's sell range, and ceases to exist (R1840001). A tray Resource is played with the `play`
//! action, as from a hand (R1840003, `play_steps.rs`); its owner reads the tray and the other player
//! its count (`view_for.rs`).
//!
//! "Gain a random Resource" rolls `RESOURCE_RARITY_ODDS` over the rarities its pool holds, then picks
//! uniformly among the Resources of the rarity rolled (R1840002). The pool is every Resource in the
//! catalog, or the ids a card names ("only gives Narcor and Red Narcor").
//!
//! Credits (`PlayerState.market_credits`) are a public count per player, absent until first gained
//! (R1840005), and named apart from Jlarna's credit line (R1223). The verbs a card script composes are
//! `effects::tray`'s; this module is the rule they share.

use crate::catalog::{find_def, registered_catalog};
use crate::config::{RESOURCE_RARITY_ODDS, RESOURCE_TRAY_CAP};
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, new_instance};
use crate::wire::{GameEvent, PlayerId, PrintedRarity, Zone};

/// R1840000: put `card` in its owner's Resource tray, last, as the newest Resource there.
pub fn put_in_tray(state: &mut GameState, card: &mut CardInstance) {
    card.zone = Zone::Tray { player: card.owner };
    state.players[card.owner]
        .resource_tray
        .get_or_insert_with(Vec::new)
        .push(card.clone());
}

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

/// R1840002: one random Resource's id. The candidates are the catalog's Resources, only those `pool`
/// names when it names some; the roll is `RESOURCE_RARITY_ODDS` over the rarities the candidates hold
/// (one draw over their weights' sum, walked in the table's order, as `catalog::roll_weighted` walks
/// its table), then one Resource of that rarity, uniformly. With no candidate, `None` and no draw.
pub fn roll_resource(state: &GameState, rng: &mut Rng, pool: Option<&[String]>) -> Option<String> {
    let candidates: Vec<(String, PrintedRarity)> = resource_def_ids()
        .into_iter()
        .filter(|id| pool.is_none_or(|pool| pool.contains(id)))
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

/// R1840001: what a sale of this Resource fetches now: a whole number drawn uniformly in its face's
/// sell range, the Radiant face's for a Radiant Resource. 0 for a card that is no Resource, with no
/// draw. The Marketplace (#1853) will price it later.
pub fn sell_price(state: &GameState, rng: &mut Rng, card: &CardInstance) -> i32 {
    let Some(resource) = find_def(Some(state), &card.def_id).and_then(|def| def.resource) else {
        return 0;
    };
    let price = if card.radiant {
        resource.radiant
    } else {
        resource.base
    };
    price.sell_min + rng.int(price.sell_max.max(price.sell_min) - price.sell_min + 1)
}

/// R1840005: raise `player`'s Credits by `amount` and report the new count with the public
/// `marketCreditsChanged`. A gain of 0 or less changes and reports nothing, so Credits stay absent
/// until first gained.
pub fn add_market_credits(sink: &mut EngineSink<'_>, player: PlayerId, amount: i32) {
    if amount <= 0 {
        return;
    }
    let value = sink.state.players[player].market_credits.unwrap_or(0) + amount;
    sink.state.players[player].market_credits = Some(value);
    sink.events
        .push(GameEvent::MarketCreditsChanged { player, value });
}

/// R1840000, R1840001: `player` gains a fresh Resource of `def_id`, Radiant when `radiant` says so.
/// Into their tray with `resourceGained`; into a full one it is sold at once instead, ceasing to exist,
/// with `resourceSold` and the Credits it fetched.
pub fn gain_resource(sink: &mut EngineSink<'_>, player: PlayerId, def_id: &str, radiant: bool) {
    let mut card = new_instance(&mut *sink.state, def_id, player, Zone::Tray { player });
    card.radiant = radiant;
    let held = sink.state.players[player]
        .resource_tray
        .as_ref()
        .map_or(0, Vec::len) as i32;
    let flag = radiant.then_some(true);
    if held >= RESOURCE_TRAY_CAP {
        let credits = sell_price(sink.state, sink.rng, &card);
        card.zone = Zone::Gone { player };
        sink.events.push(GameEvent::ResourceSold {
            player,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            radiant: flag,
            credits,
        });
        add_market_credits(sink, player, credits);
        return;
    }
    put_in_tray(sink.state, &mut card);
    sink.events.push(GameEvent::ResourceGained {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        radiant: flag,
    });
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
