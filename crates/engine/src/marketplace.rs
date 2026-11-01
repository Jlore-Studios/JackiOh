//! Stinkyscape's Marketplace (#1853, R1853000–R1853005; #2250, R2250002; docs/stinkyscape-set.md ST3),
//! built on the night market's shop (Meditative #42, R1000) and on #1840's Credits (`resources.rs`).
//!
//! It opens for both players the first time either gains a Resource or Credits (R1853000), so a game
//! with no Stinkyscape card never sees it: `GameState.marketplace` stays absent. At the start of each
//! player's turn it rolls them `MARKETPLACE_OFFERS_PER_TURN` offers of their own, one copy each, from
//! the table's lots (`MARKETPLACE_TABLE`, or the testkit's), each lot weighted by its count, which
//! replace the offers it rolled them before; opening rolls both players theirs (R2250002). A card's
//! price is the designer's rule, twice that on a Radiant offer, moved within a tenth of it as it is
//! rolled; each turn start quotes each Resource's and each hand-sale card's sell price within its
//! printed range (R1853002). An effect may add a lot for the rest of the game, its caster's alone, at
//! a price that never moves, renewed at its owner's turn start (R1853005).
//!
//! A player buys or sells in their own main phase with no prompt open, for Credits and no mana, with
//! the actions `marketBuy { offer }` and `marketSell { instanceId }` (R1853003). One function says
//! whether each is legal, `why_market_buy_refused` and `why_market_sell_refused`: `legal_actions`
//! lists what they pass, the reducer refuses what they refuse in their words, and the view gives the
//! viewer each offer's refusal as its `reason` (`marketplace_view`), so a greyed-out button and a
//! refusal cannot disagree (§9.3). A purchase is hidden like a gain (R97): `marketBought` names the card
//! as `addedToHand` does; a sale is public. Only a card that sells from the hand (`handSale`, R1861000)
//! is sold: a Resource pays its Credits as it is played or discarded instead (R2250001).
//!
//! Every roll and every price move draws from the match's rng, in the order this file draws them
//! (§10.7); opening and rolling report nothing (R1853000).

use crate::catalog::{def_of, find_def, made_by_a_rule, query_cost, registered_catalog};
use crate::config::{
    HAND_SALE_PRICE, HAND_SALE_RADIANT_PRICE, MARKETPLACE_CREDITS_PER_MANA,
    MARKETPLACE_CREDITS_PER_MANA_PAST, MARKETPLACE_FIXED_LOT_STOCK, MARKETPLACE_FULL_PRICE_MANA,
    MARKETPLACE_OFFER_STOCK, MARKETPLACE_OFFERS_PER_TURN, MARKETPLACE_PRICE_SPREAD_DIVISOR,
    MARKETPLACE_RADIANT_CHANCE, MARKETPLACE_RADIANT_PRICE_MULTIPLIER, MARKETPLACE_TABLE,
    MARKETPLACE_ZERO_COST_PRICE, MarketLot, MarketPool,
};
use crate::resources::{add_market_credits, draw_sell_price, resource_def_ids, spend_market_credits};
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{
    CardInstance, EngineError, GameState, MarketOffer, Marketplace, ResourceQuote, new_instance,
};
use crate::wire::{
    CardDef, GameEvent, MarketOfferView, MarketSaleView, MarketplaceView, PerPlayer, Phase, PlayerId, Zone,
    has_keyword, opponent_of,
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

/// R1861000: whether this card is in a hand and its face sells from there (`handSale`, Stinkyscape #89
/// Spice).
pub fn sells_from_hand(state: &GameState, card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Hand { .. }) && crate::scripts::flags_of(state, card).hand_sale == Some(true)
}

/// R1861000: the registered cards whose base face sells from the hand, sorted, the ones the Marketplace
/// quotes after the Resources.
fn hand_sale_def_ids() -> Vec<String> {
    let mut ids: Vec<String> = registered_catalog()
        .values()
        .filter(|def| {
            crate::scripts::registered_entry(&def.id)
                .is_some_and(|entry| entry.base.flags().hand_sale == Some(true))
        })
        .map(|def| def.id.clone())
        .collect();
    ids.sort();
    ids
}

/// R1861000: the card of `player`'s that `instance_id` names and a sale may take: a card in their hand
/// that sells from there.
fn sellable_card<'a>(state: &'a GameState, player: PlayerId, instance_id: &str) -> Option<&'a CardInstance> {
    state.players[player]
        .hand
        .iter()
        .find(|card| card.id == instance_id && sells_from_hand(state, card))
}

/// R1853002: a price whose base is `base`, moved by a whole number drawn uniformly within
/// ⌊base / `MARKETPLACE_PRICE_SPREAD_DIVISOR`⌋ of it, with no draw when that spread is 0.
fn moved_price(rng: &mut Rng, base: i32) -> i32 {
    let spread = base / MARKETPLACE_PRICE_SPREAD_DIVISOR;
    if spread <= 0 {
        return base;
    }
    base - spread + rng.int(2 * spread + 1)
}

/// R1853001: the ids one lot draws from, in a fixed order: the ids named that a definition is found
/// for, or a set's cards that are no token, in catalog order, all of them or (R1881006) those that
/// print a keyword on their base face. A set lot never holds a card only a rule makes (R1857001: the
/// Dormant Starship), as no `catalog::query` pool does.
fn pool_ids(state: &GameState, pool: MarketPool) -> Vec<String> {
    let in_set = |def: &CardDef, set| def.set == set && !def.token && !made_by_a_rule(def);
    match pool {
        MarketPool::Ids(ids) => ids
            .iter()
            .filter(|id| find_def(Some(state), id).is_some())
            .map(|id| (*id).to_string())
            .collect(),
        MarketPool::Set(set) => registered_catalog()
            .values()
            .filter(|def| in_set(def, set))
            .map(|def| def.id.clone())
            .collect(),
        MarketPool::SetKeyword(set, keyword) => registered_catalog()
            .values()
            .filter(|def| in_set(def, set) && has_keyword(&def.base.keywords, keyword))
            .map(|def| def.id.clone())
            .collect(),
    }
}

/// R2250002: `player`'s offers, rolled from `table`: `MARKETPLACE_OFFERS_PER_TURN` times, while a lot
/// has a card left, one lot drawn by the counts of the lots that have (a lot's count is its share of
/// the offers, as it was of the whole offering), then one card of it drawn uniformly among those not
/// offered yet, so the offers differ; then whether it is Radiant (`MARKETPLACE_RADIANT_CHANCE`), then
/// its price moved from its base (R1853002). One copy each, `player`'s alone.
pub fn roll_offers(
    state: &GameState,
    rng: &mut Rng,
    table: &[MarketLot],
    player: PlayerId,
) -> Vec<MarketOffer> {
    let mut lots: Vec<(i32, Vec<String>)> = table
        .iter()
        .filter(|lot| lot.count > 0)
        .map(|lot| {
            // R2201001: a held game offers no Legendary or Mythic card.
            let ids = pool_ids(state, lot.pool)
                .into_iter()
                .filter(|id| !find_def(Some(state), id).is_some_and(crate::catalog::held_out))
                .collect();
            (lot.count, ids)
        })
        .collect();
    let mut offers: Vec<MarketOffer> = Vec::new();
    for _ in 0..MARKETPLACE_OFFERS_PER_TURN {
        for (_, ids) in lots.iter_mut() {
            ids.retain(|id| offers.iter().all(|offer| offer.def_id != *id));
        }
        let total: i32 = lots
            .iter()
            .filter(|(_, ids)| !ids.is_empty())
            .map(|(count, _)| count)
            .sum();
        if total <= 0 {
            break;
        }
        let mut at = rng.int(total);
        let Some((_, ids)) = lots
            .iter_mut()
            .filter(|(_, ids)| !ids.is_empty())
            .find(|(count, _)| {
                if at < *count {
                    true
                } else {
                    at -= count;
                    false
                }
            })
        else {
            break;
        };
        let def_id = ids.remove(rng.int(ids.len() as i32) as usize);
        let radiant = rng.chance(MARKETPLACE_RADIANT_CHANCE);
        let base = market_card_price(def_of(Some(state), &def_id), radiant);
        let price = moved_price(rng, base);
        offers.push(MarketOffer {
            def_id,
            radiant: radiant.then_some(true),
            base,
            price,
            stock: MARKETPLACE_OFFER_STOCK,
            bought: PerPlayer::new(0, 0),
            owner: Some(player),
            fixed: None,
        });
    }
    offers
}

/// R2250002: roll `player` their offers: the offers rolled for them before go, their new ones follow
/// the rest, and the lots an effect added for them are renewed (R1853005).
fn reroll_for(sink: &mut EngineSink<'_>, player: PlayerId) {
    let Some(mut market) = sink.state.marketplace.take() else {
        return;
    };
    let fresh = roll_offers(sink.state, sink.rng, marketplace_table(), player);
    market
        .offers
        .retain(|offer| offer.owner != Some(player) || offer.fixed == Some(true));
    for offer in market.offers.iter_mut() {
        if offer.owner == Some(player) {
            offer.bought[player] = 0;
        }
    }
    market.offers.extend(fresh);
    sink.state.marketplace = Some(market);
}

/// R1853002: each of the catalog's Resources quoted, its base face then its Radiant face, in sorted
/// order, then each card that sells from the hand the same way (R1861000).
fn requote(sink: &mut EngineSink<'_>) {
    let Some(mut market) = sink.state.marketplace.take() else {
        return;
    };
    let state = &*sink.state;
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
    for def_id in hand_sale_def_ids() {
        let base = draw_sell_price(sink.rng, HAND_SALE_PRICE);
        let radiant = draw_sell_price(sink.rng, HAND_SALE_RADIANT_PRICE);
        market.quotes.push(ResourceQuote {
            def_id,
            base,
            radiant,
        });
    }
    sink.state.marketplace = Some(market);
}

/// R1853000, R2250002: open the Marketplace for both players if it is not open yet: the player whose
/// turn it is gets their offers, then the other player theirs, then the quotes. Nothing reports it.
pub fn open_marketplace(sink: &mut EngineSink<'_>) {
    if sink.state.marketplace.is_some() {
        return;
    }
    sink.state.marketplace = Some(Marketplace {
        offers: Vec::new(),
        quotes: Vec::new(),
    });
    let active = sink.state.active;
    reroll_for(sink, active);
    reroll_for(sink, opponent_of(active));
    requote(sink);
}

/// R2250002, R1853002: at the start of `player`'s turn an open Marketplace rolls them their offers and
/// quotes its prices. A closed one draws nothing.
pub fn marketplace_turn_start(sink: &mut EngineSink<'_>, player: PlayerId) {
    if sink.state.marketplace.is_none() {
        return;
    }
    reroll_for(sink, player);
    requote(sink);
}

/// R1853002: what this card (a Resource, or a hand card that sells from there, R1861000) fetches at an
/// open Marketplace this turn, on its own face; `None` while the Marketplace is closed or quotes no such
/// card.
pub fn sell_quote(state: &GameState, card: &CardInstance) -> Option<i32> {
    let quote = state
        .marketplace
        .as_ref()?
        .quotes
        .iter()
        .find(|quote| quote.def_id == card.def_id)?;
    Some(if card.radiant { quote.radiant } else { quote.base })
}

/// R2250002: the lots offered to `player` (their rolled offers and the lots an effect added for them),
/// in the Marketplace's order, each with its place there. A buy names a lot by its position in this
/// list (`marketBuy { offer }`), so another player's lots, hidden from `player`, never move the index.
pub fn offered_to(market: &Marketplace, player: PlayerId) -> impl Iterator<Item = (usize, &MarketOffer)> {
    market
        .offers
        .iter()
        .enumerate()
        .filter(move |(_, offer)| offer.owner.is_none_or(|owner| owner == player))
}

/// R1853005, R2250002: the copies of a lot `player` may still buy: none of a lot that is another
/// player's, else its stock less what they have bought of it.
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

/// R1853003: why `player` may not buy the lot at index `offer` of those offered to them (`offered_to`)
/// now, or `Ok`. The one rule: `legal_actions`, the reducer and the view all ask it. In order: a live
/// game in their own main phase with no prompt open, an open Marketplace, a lot at that index offered
/// to them, a copy of it left to them, the Credits for its price, and room for the card in their hand
/// (`hand_cap_of`).
pub fn why_market_buy_refused(state: &GameState, player: PlayerId, offer: i32) -> Result<(), EngineError> {
    let market = open_for_deals(state, player)?;
    let Some((_, lot)) = usize::try_from(offer)
        .ok()
        .and_then(|at| offered_to(market, player).nth(at))
    else {
        return Err(EngineError::new(format!("no offer {offer} at the Marketplace")));
    };
    let name = &def_of(Some(state), &lot.def_id).name;
    if market_left(lot, player) <= 0 {
        return Err(EngineError::new(format!("you have bought {name}")));
    }
    let credits = state.players[player].market_credits.unwrap_or(0);
    if credits < lot.price {
        return Err(EngineError::new(format!(
            "{name} costs {} Credits and you have {credits}",
            lot.price
        )));
    }
    if crate::query::room_in(state, player, crate::zones::OffFieldZone::Hand) == 0 {
        return Err(EngineError::new("your hand is full"));
    }
    Ok(())
}

/// R1853003, R1861000: why `player` may not sell the hand card `instance_id` names now, or `Ok`: the
/// first checks a buy makes, then a card in their hand that sells from there and a quote for it this
/// turn. A Resource is not sold: it pays its Credits as it is played or discarded (R2250001).
pub fn why_market_sell_refused(
    state: &GameState,
    player: PlayerId,
    instance_id: &str,
) -> Result<(), EngineError> {
    open_for_deals(state, player)?;
    let Some(card) = sellable_card(state, player, instance_id) else {
        return Err(EngineError::new(format!(
            "no card {instance_id} in your hand sells at the Marketplace"
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
/// card arrives in their hand (`addedToHand`). No mana is paid and nothing is played.
pub fn apply_market_buy(sink: &mut EngineSink<'_>, player: PlayerId, offer: i32) -> Result<(), EngineError> {
    why_market_buy_refused(sink.state, player, offer)?;
    let Some((at, lot)) = sink.state.marketplace.as_ref().and_then(|market| {
        offered_to(market, player)
            .nth(offer as usize)
            .map(|(at, lot)| (at, lot.clone()))
    }) else {
        return Err(EngineError::new(format!("no offer {offer} at the Marketplace")));
    };
    let mut card = new_instance(&mut *sink.state, &lot.def_id, player, Zone::Hand { player });
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
    let _ = crate::draw::add_to_hand(sink, &mut card);
    Ok(())
}

/// R1853003, R1861000: the sale. Refused as `why_market_sell_refused` refuses it; otherwise the hand
/// card ceases to exist, the public `marketSold` names it and its quote, and its Credits are gained.
pub fn apply_market_sell(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    instance_id: &str,
) -> Result<(), EngineError> {
    why_market_sell_refused(sink.state, player, instance_id)?;
    let Some(mut card) = sellable_card(sink.state, player, instance_id).cloned() else {
        return Err(EngineError::new(format!(
            "no card {instance_id} in your hand sells at the Marketplace"
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
/// `MARKETPLACE_FIXED_LOT_STOCK` copies, renewed at each of their turn starts (R2250002).
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
            fixed: Some(true),
        });
    }
}

/// R1853003, R2250002, §10.8: the Marketplace as `viewer` reads it, or nothing while it is closed: every
/// lot offered to them (their rolled offers and the lots an effect added for them), with its index, its
/// price, the copies left to them and, while they may act (their turn, the main phase, no prompt open),
/// the refusal of a buy `legal_actions` does not list; and each card in their own hand that sells from
/// there (R1861000), at its quote. Another player's offers are theirs alone (R97).
pub fn marketplace_view(state: &GameState, viewer: PlayerId) -> Option<MarketplaceView> {
    let market = state.marketplace.as_ref()?;
    let may_act = state.result.is_none()
        && state.phase == Phase::Main
        && state.pending.is_none()
        && state.active == viewer;
    let offers = offered_to(market, viewer)
        .enumerate()
        .map(|(at, (_, offer))| {
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
                fixed: offer.fixed,
                reason,
            }
        })
        .collect();
    let sales = state.players[viewer]
        .hand
        .iter()
        .filter(|card| sells_from_hand(state, card))
        .filter_map(|card| {
            Some(MarketSaleView {
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                radiant: card.radiant.then_some(true),
                credits: sell_quote(state, card)?,
            })
        })
        .collect();
    Some(MarketplaceView { offers, sales })
}
