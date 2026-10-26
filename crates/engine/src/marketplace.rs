//! Stinkyscape's Marketplace (#1853, R1853000–R1853005; docs/stinkyscape-set.md ST3), built on the night
//! market's shop (Meditative #42, R1000) and on #1840's Resources and Credits (`resources.rs`).
//!
//! It opens for both players the first time either gains a Resource or Credits (R1853000), so a game
//! with no Stinkyscape card never sees it: `GameState.marketplace` stays absent. One offering for both
//! players, rolled from a table of lots (`MARKETPLACE_TABLE`, or the testkit's), each a pool, a count
//! and a stock range, with a stock of each lot for each player; it re-rolls every
//! `MARKETPLACE_CYCLE_TURNS` turns, restocking both (R1853001). A card's price is the designer's rule,
//! twice that on a Radiant lot; each turn start moves every card's price within a tenth of it, leaves a
//! Resource's buy price as printed and quotes each Resource's sell price within its printed range
//! (R1853002). An effect may add a lot for the rest of the game, its caster's alone, at a price that
//! never moves (R1853005).
//!
//! A player buys or sells in their own main phase with no prompt open, for Credits and no mana, with
//! the actions `marketBuy { offer }` and `marketSell { instanceId }` (R1853003). One function says
//! whether each is legal, `why_market_buy_refused` and `why_market_sell_refused`: `legal_actions`
//! lists what they pass, the reducer refuses what they refuse in their words, and the view gives the
//! viewer each offer's refusal as its `reason` (`marketplace_view`), so a greyed-out button and a
//! refusal cannot disagree (§9.3). A purchase is hidden like a gain (R97): `marketBought` names the card
//! as `addedToHand` does; a sale is public, as a full tray's is (R1840001).
//!
//! Every roll and every price move draws from the match's rng, in the order this file draws them
//! (§10.7); opening and re-rolling report nothing (R1853000).

use crate::catalog::{def_of, find_def, query_cost, registered_catalog};
use crate::config::{
    MARKETPLACE_CREDITS_PER_MANA, MARKETPLACE_CREDITS_PER_MANA_PAST, MARKETPLACE_CYCLE_TURNS,
    MARKETPLACE_FIXED_LOT_STOCK, MARKETPLACE_FULL_PRICE_MANA, MARKETPLACE_PRICE_SPREAD_DIVISOR,
    MARKETPLACE_RADIANT_CHANCE, MARKETPLACE_RADIANT_PRICE_MULTIPLIER, MARKETPLACE_TABLE,
    MARKETPLACE_ZERO_COST_PRICE, MarketLot, MarketPool, RESOURCE_TRAY_CAP,
};
use crate::resources::{
    add_market_credits, draw_sell_price, receive_resource, resource_def_ids, spend_market_credits,
};
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{
    CardInstance, EngineError, GameState, MarketOffer, Marketplace, ResourceQuote, new_instance,
};
use crate::wire::{
    CardDef, GameEvent, MarketOfferView, MarketSaleView, MarketplaceView, PerPlayer, Phase, PlayerId, Zone,
};
use crate::zones::cease_to_exist;

/// R1853001: the table the Marketplace rolls: the testkit's override first, then `MARKETPLACE_TABLE`.
pub fn marketplace_table() -> &'static [MarketLot] {
    #[cfg(feature = "testkit")]
    if let Some(table) = crate::testkit::scenario::marketplace_table_override() {
        return table;
    }
    MARKETPLACE_TABLE
}

/// R1853002: the designer's price of a card of cost c, 10 × min(c, 4) + 5 × max(c − 4, 0) Credits, a
/// (0) card's `MARKETPLACE_ZERO_COST_PRICE`, and `MARKETPLACE_RADIANT_PRICE_MULTIPLIER` times that on
/// its Radiant face. The cost is the card's printed one (`query_cost`: an X card's is 0).
pub fn market_card_price(def: &CardDef, radiant: bool) -> i32 {
    let cost = query_cost(def);
    let price = if cost <= 0 {
        MARKETPLACE_ZERO_COST_PRICE
    } else {
        MARKETPLACE_CREDITS_PER_MANA * cost.min(MARKETPLACE_FULL_PRICE_MANA)
            + MARKETPLACE_CREDITS_PER_MANA_PAST * (cost - MARKETPLACE_FULL_PRICE_MANA).max(0)
    };
    if radiant {
        price * MARKETPLACE_RADIANT_PRICE_MULTIPLIER
    } else {
        price
    }
}

/// R1853002: what a lot of this card asks before it moves: a Resource's face's printed `buy`, the
/// Radiant face's on a Radiant lot; any other card's `market_card_price`.
fn base_price(def: &CardDef, radiant: bool) -> i32 {
    match def.resource {
        Some(resource) if radiant => resource.radiant.buy,
        Some(resource) => resource.base.buy,
        None => market_card_price(def, radiant),
    }
}

/// Whether the definition `def_id` names is a Resource, which goes to the tray and keeps its price.
fn is_resource(state: &GameState, def_id: &str) -> bool {
    find_def(Some(state), def_id).is_some_and(|def| def.resource.is_some())
}

/// R1853002: this turn's price of a lot whose base is `base`: a whole number drawn uniformly within
/// ⌊base / `MARKETPLACE_PRICE_SPREAD_DIVISOR`⌋ of it, with no draw when that spread is 0.
fn moved_price(rng: &mut Rng, base: i32) -> i32 {
    let spread = base / MARKETPLACE_PRICE_SPREAD_DIVISOR;
    if spread <= 0 {
        return base;
    }
    base - spread + rng.int(2 * spread + 1)
}

/// R1853001: the ids one lot draws from, in a fixed order: the catalog's Resources sorted, the ids named
/// that a definition is found for, or a set's cards that are no token, in catalog order.
fn pool_ids(state: &GameState, pool: MarketPool) -> Vec<String> {
    match pool {
        MarketPool::Resources => resource_def_ids(),
        MarketPool::Ids(ids) => ids
            .iter()
            .filter(|id| find_def(Some(state), id).is_some())
            .map(|id| (*id).to_string())
            .collect(),
        MarketPool::Set(set) => registered_catalog()
            .values()
            .filter(|def| def.set == set && !def.token)
            .map(|def| def.id.clone())
            .collect(),
    }
}

/// R1853001: an offering rolled from `table`, its lots in table order. For each lot, `count` times
/// while its pool has a card left: one card of the pool drawn uniformly and taken out of it (so a lot's
/// cards differ), whether it is Radiant (`MARKETPLACE_RADIANT_CHANCE`), then its stock in the lot's
/// range (no draw when the range is one number). Each lot starts at its base price.
pub fn roll_offers(state: &GameState, rng: &mut Rng, table: &[MarketLot]) -> Vec<MarketOffer> {
    let mut offers: Vec<MarketOffer> = Vec::new();
    for lot in table {
        let mut candidates = pool_ids(state, lot.pool);
        for _ in 0..lot.count {
            if candidates.is_empty() {
                break;
            }
            let def_id = candidates.remove(rng.int(candidates.len() as i32) as usize);
            let radiant = rng.chance(MARKETPLACE_RADIANT_CHANCE);
            let stock = if lot.stock_max > lot.stock_min {
                lot.stock_min + rng.int(lot.stock_max - lot.stock_min + 1)
            } else {
                lot.stock_min
            };
            let base = base_price(def_of(Some(state), &def_id), radiant);
            offers.push(MarketOffer {
                def_id,
                radiant: radiant.then_some(true),
                base,
                price: base,
                stock,
                bought: PerPlayer::new(0, 0),
                owner: None,
            });
        }
    }
    offers
}

/// R1853001, R1853002: the Marketplace's turn-start work, in this order. On a re-roll the lots an
/// effect added stay, restocked, after a fresh roll of the table, and the cycle starts this turn. Then
/// each rolled lot that is no Resource moves its price, in offer order, and each of the catalog's
/// Resources is quoted, its base face then its Radiant face, in sorted order.
fn refresh(sink: &mut EngineSink<'_>, reroll: bool) {
    let Some(mut market) = sink.state.marketplace.take() else {
        return;
    };
    let state = &*sink.state;
    if reroll {
        let mut offers = roll_offers(state, sink.rng, marketplace_table());
        offers.extend(
            market
                .offers
                .into_iter()
                .filter(|offer| offer.owner.is_some())
                .map(|offer| MarketOffer {
                    bought: PerPlayer::new(0, 0),
                    ..offer
                }),
        );
        market.offers = offers;
        market.rolled_turn = state.turn;
    }
    for offer in market.offers.iter_mut() {
        if offer.owner.is_none() && !is_resource(state, &offer.def_id) {
            offer.price = moved_price(sink.rng, offer.base);
        }
    }
    market.quotes = resource_def_ids()
        .into_iter()
        .filter_map(|def_id| {
            let resource = find_def(Some(state), &def_id)?.resource?;
            let base = draw_sell_price(sink.rng, resource.base);
            let radiant = draw_sell_price(sink.rng, resource.radiant);
            Some(ResourceQuote {
                def_id,
                base,
                radiant,
            })
        })
        .collect();
    sink.state.marketplace = Some(market);
}

/// R1853000: open the Marketplace for both players if it is not open yet, rolling its first offering
/// this turn. Nothing reports it.
pub fn open_marketplace(sink: &mut EngineSink<'_>) {
    if sink.state.marketplace.is_some() {
        return;
    }
    sink.state.marketplace = Some(Marketplace {
        rolled_turn: sink.state.turn,
        offers: Vec::new(),
        quotes: Vec::new(),
    });
    refresh(sink, true);
}

/// R1853001, R1853002: at the start of each turn an open Marketplace moves its prices and quotes, and
/// re-rolls once `MARKETPLACE_CYCLE_TURNS` turns have passed since its last roll. A closed one draws
/// nothing.
pub fn marketplace_turn_start(sink: &mut EngineSink<'_>) {
    let Some(market) = &sink.state.marketplace else {
        return;
    };
    let reroll = sink.state.turn - market.rolled_turn >= MARKETPLACE_CYCLE_TURNS;
    refresh(sink, reroll);
}

/// R1853002: what this tray Resource sells for at an open Marketplace this turn, on its own face; `None`
/// while the Marketplace is closed or quotes no such Resource.
pub fn sell_quote(state: &GameState, card: &CardInstance) -> Option<i32> {
    let quote = state
        .marketplace
        .as_ref()?
        .quotes
        .iter()
        .find(|quote| quote.def_id == card.def_id)?;
    Some(if card.radiant { quote.radiant } else { quote.base })
}

/// R1853001, R1853005: the copies of a lot `player` may still buy this cycle: none of a lot that is
/// another player's, else its stock less what they have bought of it.
pub fn market_left(offer: &MarketOffer, player: PlayerId) -> i32 {
    if offer.owner.is_some_and(|owner| owner != player) {
        return 0;
    }
    (offer.stock - offer.bought[player]).max(0)
}

/// R1853003: the open Marketplace, after a main-phase deal's checks (`trade::why_not_in_main_phase`).
fn open_for_deals(state: &GameState, player: PlayerId) -> Result<&Marketplace, EngineError> {
    crate::trade::why_not_in_main_phase(state, player)?;
    state
        .marketplace
        .as_ref()
        .ok_or_else(|| EngineError::new("the Marketplace is not open"))
}

/// R1853003: why `player` may not buy the lot at index `offer` now, or `Ok`. The one rule:
/// `legal_actions`, the reducer and the view all ask it. In order: a live game in their own main phase
/// with no prompt open, an open Marketplace, a lot at that index offered to them, a copy of it left to
/// them this cycle, the Credits for its price, and room for the card — in their Resource tray for a
/// Resource (`RESOURCE_TRAY_CAP`), else in their hand (`hand_cap_of`).
pub fn why_market_buy_refused(state: &GameState, player: PlayerId, offer: i32) -> Result<(), EngineError> {
    let market = open_for_deals(state, player)?;
    let Some(lot) = usize::try_from(offer)
        .ok()
        .and_then(|at| market.offers.get(at))
        .filter(|lot| lot.owner.is_none_or(|owner| owner == player))
    else {
        return Err(EngineError::new(format!("no offer {offer} at the Marketplace")));
    };
    let name = &def_of(Some(state), &lot.def_id).name;
    if market_left(lot, player) <= 0 {
        return Err(EngineError::new(format!(
            "you have bought this cycle's {} {name}",
            lot.stock
        )));
    }
    let credits = state.players[player].market_credits.unwrap_or(0);
    if credits < lot.price {
        return Err(EngineError::new(format!(
            "{name} costs {} Credits and you have {credits}",
            lot.price
        )));
    }
    let side = &state.players[player];
    if is_resource(state, &lot.def_id) {
        if side.resource_tray.as_ref().map_or(0, Vec::len) as i32 >= RESOURCE_TRAY_CAP {
            return Err(EngineError::new("your Resource tray is full"));
        }
    } else if side.hand.len() as i32 >= crate::query::hand_cap_of(state, player) {
        return Err(EngineError::new("your hand is full"));
    }
    Ok(())
}

/// R1853003: why `player` may not sell the tray Resource `instance_id` names now, or `Ok`: the first
/// checks a buy makes, then the card in their Resource tray and a quote for it this turn.
pub fn why_market_sell_refused(
    state: &GameState,
    player: PlayerId,
    instance_id: &str,
) -> Result<(), EngineError> {
    open_for_deals(state, player)?;
    let Some(card) = state.players[player]
        .resource_tray
        .iter()
        .flatten()
        .find(|card| card.id == instance_id)
    else {
        return Err(EngineError::new(format!(
            "no Resource {instance_id} in your Resource tray"
        )));
    };
    if sell_quote(state, card).is_none() {
        let name = &def_of(Some(state), &card.def_id).name;
        return Err(EngineError::new(format!(
            "{name} has no price at the Marketplace"
        )));
    }
    Ok(())
}

/// R1853003: the buy. Refused as `why_market_buy_refused` refuses it; otherwise a fresh card of the
/// lot, on the lot's face, is reported bought (`marketBought`, hidden as `addedToHand` is), its price
/// is paid in Credits (`marketCreditsChanged`), the copy is counted against `player`'s stock, and the
/// card arrives: a Resource in their tray (`resourceGained`), any other card in their hand
/// (`addedToHand`). No mana is paid and nothing is played.
pub fn apply_market_buy(sink: &mut EngineSink<'_>, player: PlayerId, offer: i32) -> Result<(), EngineError> {
    why_market_buy_refused(sink.state, player, offer)?;
    let at = offer as usize;
    let Some(lot) = sink
        .state
        .marketplace
        .as_ref()
        .and_then(|market| market.offers.get(at))
        .cloned()
    else {
        return Err(EngineError::new(format!("no offer {offer} at the Marketplace")));
    };
    let resource = is_resource(sink.state, &lot.def_id);
    let zone = if resource {
        Zone::Tray { player }
    } else {
        Zone::Hand { player }
    };
    let mut card = new_instance(&mut *sink.state, &lot.def_id, player, zone);
    card.radiant = lot.radiant == Some(true);
    sink.events.push(GameEvent::MarketBought {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        radiant: lot.radiant,
        credits: lot.price,
    });
    spend_market_credits(sink, player, lot.price);
    if let Some(bought) = sink
        .state
        .marketplace
        .as_mut()
        .and_then(|market| market.offers.get_mut(at))
    {
        bought.bought[player] += 1;
    }
    if resource {
        receive_resource(sink, player, card);
    } else {
        let _ = crate::draw::add_to_hand(sink, &mut card);
    }
    Ok(())
}

/// R1853003: the sale. Refused as `why_market_sell_refused` refuses it; otherwise the Resource ceases
/// to exist, the public `marketSold` names it and its quote, and its Credits are gained.
pub fn apply_market_sell(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    instance_id: &str,
) -> Result<(), EngineError> {
    why_market_sell_refused(sink.state, player, instance_id)?;
    let Some(mut card) = sink.state.players[player]
        .resource_tray
        .iter()
        .flatten()
        .find(|card| card.id == instance_id)
        .cloned()
    else {
        return Err(EngineError::new(format!(
            "no Resource {instance_id} in your Resource tray"
        )));
    };
    let credits = sell_quote(sink.state, &card).unwrap_or(0);
    cease_to_exist(sink.state, &mut card);
    sink.events.push(GameEvent::MarketSold {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        radiant: card.radiant.then_some(true),
        credits,
    });
    add_market_credits(sink, player, credits);
    Ok(())
}

/// R1853005: "add a lot for the rest of the game": the Marketplace opens if it is closed, then one card
/// of `pool` is drawn among the ids a definition is found for (none found: nothing is added) and
/// offered to `player` alone at `price` Credits, never Radiant, its price never moving, with
/// `MARKETPLACE_FIXED_LOT_STOCK` copies a cycle, renewed at every re-roll.
pub fn add_fixed_market_lot(sink: &mut EngineSink<'_>, player: PlayerId, pool: &[String], price: i32) {
    open_marketplace(sink);
    let found: Vec<&String> = pool
        .iter()
        .filter(|id| find_def(Some(&*sink.state), id).is_some())
        .collect();
    let Some(def_id) = sink.rng.pick(&found).map(|id| (*id).clone()) else {
        return;
    };
    let price = price.max(0);
    if let Some(market) = sink.state.marketplace.as_mut() {
        market.offers.push(MarketOffer {
            def_id,
            radiant: None,
            base: price,
            price,
            stock: MARKETPLACE_FIXED_LOT_STOCK,
            bought: PerPlayer::new(0, 0),
            owner: Some(player),
        });
    }
}

/// R1853003, §10.8: the Marketplace as `viewer` reads it, or nothing while it is closed: the turn it
/// re-rolls on; every lot offered to them (not another player's own), with its index, this turn's
/// price, the copies left to them this cycle and, while they may act (their turn, the main phase, no
/// prompt open), the refusal of a buy `legal_actions` does not list; and each Resource in their own
/// tray with a quote, at that quote. The offering and its prices are public; the stock and the sales
/// are the viewer's own (R97).
pub fn marketplace_view(state: &GameState, viewer: PlayerId) -> Option<MarketplaceView> {
    let market = state.marketplace.as_ref()?;
    let may_act = state.result.is_none()
        && state.phase == Phase::Main
        && state.pending.is_none()
        && state.active == viewer;
    let offers = market
        .offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| offer.owner.is_none_or(|owner| owner == viewer))
        .map(|(at, offer)| {
            let index = at as i32;
            let reason = if may_act {
                why_market_buy_refused(state, viewer, index)
                    .err()
                    .map(|refusal| refusal.message)
                    .filter(|message| !message.is_empty())
            } else {
                None
            };
            MarketOfferView {
                offer: index,
                def_id: offer.def_id.clone(),
                radiant: offer.radiant,
                price: offer.price,
                left: market_left(offer, viewer),
                fixed: offer.owner.map(|_| true),
                reason,
            }
        })
        .collect();
    let sales = state.players[viewer]
        .resource_tray
        .iter()
        .flatten()
        .filter_map(|card| {
            Some(MarketSaleView {
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                radiant: card.radiant.then_some(true),
                credits: sell_quote(state, card)?,
            })
        })
        .collect();
    Some(MarketplaceView {
        rerolls_on: market.rolled_turn + MARKETPLACE_CYCLE_TURNS,
        offers,
        sales,
    })
}
