//! Stinkyscape's Marketplace (#1853, R1853000–R1853005; #2250, R2250002; #3047, R3047000–R3047004;
//! docs/stinkyscape-set.md ST3), built on the night market's shop (Meditative #42, R1000) and on #1840's
//! Credits (`resources.rs`).
//!
//! It opens for both players the first time either gains a Resource or Credits (R1853000), so a game
//! with no Stinkyscape card never sees it: `GameState.marketplace` stays absent. Each turn start rolls
//! that player their own offers, replacing those it rolled them before (opening rolls both players
//! theirs, R2250002), and quotes each Resource's sell price (R1853002), then each `paysCredits` card's
//! (R4410000). An offer may be rolled Buffed or Nerfed, which moves its price and tunes the card bought
//! (R3047002, R3047003). An effect may add a lot for the rest of the game, its caster's alone, at a price
//! that never moves (R1853005), never Buffed or Nerfed.
//!
//! A player buys in their own main phase with no prompt open, for Credits and no mana (`marketBuy
//! { offer }`, R1853003). `why_market_buy_refused` is the one rule `legal_actions`, the reducer and the
//! view all ask (§9.3). A purchase is hidden like a gain (R97) and the card bought costs (0) (R3047001).
//! Nothing sells: a `marketSell` is always refused (`NOTHING_SELLS`, R4410000). A Resource, and a
//! `paysCredits` card (S #89 Spice), pays its Credits as it is played or discarded instead (R2250001,
//! R4410000), and shows the quote in braces while the Marketplace is open (R3047004, `preview.rs`).
//!
//! Every roll and every price move draws from the match's rng, in the order this file draws them
//! (§10.7); opening and rolling report nothing (R1853000).

use crate::catalog::{def_of, find_def, made_by_a_rule, query_cost, registered_catalog, set_is_open};
use crate::config::{
    HAND_SALE_PRICE, HAND_SALE_RADIANT_PRICE, MARKETPLACE_BOUGHT_COST, MARKETPLACE_BUFF_CHANCE,
    MARKETPLACE_BUFF_PRICE_PERCENT, MARKETPLACE_CREDITS_PER_MANA, MARKETPLACE_CREDITS_PER_MANA_PAST,
    MARKETPLACE_FIXED_LOT_STOCK, MARKETPLACE_FULL_PRICE_MANA, MARKETPLACE_MARKUP_PERCENT,
    MARKETPLACE_NERF_CHANCE, MARKETPLACE_NERF_PRICE_PERCENT, MARKETPLACE_OFFER_STOCK,
    MARKETPLACE_OFFERS_PER_TURN, MARKETPLACE_PRICE_BASE, MARKETPLACE_PRICE_SPREAD_DIVISOR,
    MARKETPLACE_RADIANT_CHANCE, MARKETPLACE_RADIANT_PRICE_MULTIPLIER, MARKETPLACE_TABLE,
    MARKETPLACE_TUNE_MAX, MARKETPLACE_ZERO_COST_PRICE, MarketLot, MarketPool, PERCENT,
};
use crate::effects::tune::{TuneDirection, degrade_sparing_cost, tune_once};
use crate::resolve::{HookOptions, make_context};
use crate::resources::{draw_sell_price, resource_def_ids, spend_market_credits};
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{
    CardInstance, EngineError, GameState, MarketOffer, Marketplace, ResourceQuote, new_instance,
};
use crate::wire::{
    CardDef, GameEvent, MarketOfferView, MarketplaceView, PerPlayer, Phase, PlayerId, Zone, has_keyword,
    opponent_of,
};

/// R1853001: the table the Marketplace rolls: the testkit's override first, then `MARKETPLACE_TABLE`.
pub fn marketplace_table() -> &'static [MarketLot] {
    #[cfg(feature = "testkit")]
    if let Some(table) = crate::testkit::scenario::marketplace_table_override() {
        return table;
    }
    MARKETPLACE_TABLE
}

/// R1853002, R3047000, R3951003: the price of a card of cost c: the designer's 5 + 10 × min(c, 4) +
/// 15 × max(c − 4, 0) Credits, a (0) card's `MARKETPLACE_ZERO_COST_PRICE`, marked up to
/// `MARKETPLACE_MARKUP_PERCENT` of that and rounded to the nearest Credit, a half up; then
/// `MARKETPLACE_RADIANT_PRICE_MULTIPLIER` times that on its Radiant face: 9, 16, 27, 38, 49, 65, 81, 97,
/// 113 for (0) to (8). The cost is the card's printed one (`query_cost`: an X card's is 0).
pub fn market_card_price(def: &CardDef, radiant: bool) -> i32 {
    let cost = query_cost(def);
    let designer = if cost <= 0 {
        MARKETPLACE_ZERO_COST_PRICE
    } else {
        MARKETPLACE_PRICE_BASE
            + MARKETPLACE_CREDITS_PER_MANA * cost.min(MARKETPLACE_FULL_PRICE_MANA)
            + MARKETPLACE_CREDITS_PER_MANA_PAST * (cost - MARKETPLACE_FULL_PRICE_MANA).max(0)
    };
    let price = (designer * MARKETPLACE_MARKUP_PERCENT + PERCENT / 2) / PERCENT;
    if radiant {
        price * MARKETPLACE_RADIANT_PRICE_MULTIPLIER
    } else {
        price
    }
}

/// R4410000: the refusal of every `marketSell`: nothing sells at the Marketplace.
pub const NOTHING_SELLS: &str = "nothing sells at the Marketplace";

/// R4410000: the registered cards whose base face pays Credits as a Resource does (`paysCredits`), sorted.
fn credit_paying_def_ids() -> Vec<String> {
    let mut ids: Vec<String> = registered_catalog()
        .values()
        .filter(|def| {
            crate::scripts::registered_entry(&def.id)
                .is_some_and(|entry| entry.base.flags().pays_credits == Some(true))
        })
        .map(|def| def.id.clone())
        .collect();
    ids.sort();
    ids
}

/// R3047002: `price` after `buffs` Buffs and `nerfs` Nerfs, each one `MARKETPLACE_BUFF_PRICE_PERCENT` or
/// `MARKETPLACE_NERF_PRICE_PERCENT` percent of the price before it, compounding, rounded down once at the
/// end (a whole number of Credits, as the spread's is, R1853002). The product is worked out whole in a
/// wider integer, so a long run of steps neither rounds each step nor overflows.
pub fn tuned_price(price: i32, buffs: i32, nerfs: i32) -> i32 {
    let mut scaled = i128::from(price);
    let mut whole = 1_i128;
    for (steps, percent) in [
        (buffs, MARKETPLACE_BUFF_PRICE_PERCENT),
        (nerfs, MARKETPLACE_NERF_PRICE_PERCENT),
    ] {
        for _ in 0..steps.max(0) {
            scaled *= i128::from(percent);
            whole *= i128::from(PERCENT);
        }
    }
    i32::try_from(scaled / whole).unwrap_or(i32::MAX)
}

/// R3047002: how many times in a row a roll at `chance` lands, at most `MARKETPLACE_TUNE_MAX`: one draw
/// for each roll made, so none is made past the bound.
pub fn repeated_rolls(rng: &mut Rng, chance: f64) -> i32 {
    let mut landed = 0;
    while landed < MARKETPLACE_TUNE_MAX && rng.chance(chance) {
        landed += 1;
    }
    landed
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

/// R1853001: the ids one lot draws from, in a fixed order: the named ids with a definition, or (R3571001)
/// those of a set that is open, or a set's non-token cards in catalog order, all or (R1881006) those
/// printing a keyword on their base face, or (R3869004) the cards of open sets carrying a tag, by
/// their name when the lot says, never a card only a rule makes (R1857001's Dormant Starship), as in no
/// `catalog::query` pool.
fn pool_ids(state: &GameState, pool: MarketPool) -> Vec<String> {
    let in_set = |def: &CardDef, set| def.set == set && !def.token && !made_by_a_rule(def);
    match pool {
        MarketPool::Ids(ids) => ids
            .iter()
            .filter(|id| find_def(Some(state), id).is_some())
            .map(|id| (*id).to_string())
            .collect(),
        MarketPool::OpenIds(ids) => ids
            .iter()
            .filter(|id| find_def(Some(state), id).is_some_and(|def| set_is_open(def.set)))
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
        // R3869004: a lot read off its cards' tag, and its name when it names one.
        MarketPool::Tagged {
            sets,
            tag,
            tokens,
            name,
        } => registered_catalog()
            .values()
            .filter(|def| {
                sets.contains(&def.set)
                    && set_is_open(def.set)
                    && def.is_token() == tokens
                    && !made_by_a_rule(def)
                    && def.tags.contains(&tag)
                    && name.is_none_or(|(text, holds)| def.name.contains(text) == holds)
            })
            .map(|def| def.id.clone())
            .collect(),
    }
}

/// R2250002, R3047000, R3047002: `player`'s offers from `table`, one copy each: `MARKETPLACE_OFFERS_PER_TURN`
/// times, while a lot has a card left, a lot drawn by the counts of those that have, then a card of it
/// drawn uniformly among those not offered yet (so the offers differ), then whether it is Radiant
/// (`MARKETPLACE_RADIANT_CHANCE`), then how many Buffs it takes (`MARKETPLACE_BUFF_CHANCE`, rolled again
/// while one lands), then, with none, how many Nerfs (`MARKETPLACE_NERF_CHANCE`, the same), then its price
/// moved from its base (R1853002), which the Radiant doubling and the Buffs or Nerfs have set.
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
        let buffs = repeated_rolls(rng, MARKETPLACE_BUFF_CHANCE);
        let nerfs = if buffs == 0 {
            repeated_rolls(rng, MARKETPLACE_NERF_CHANCE)
        } else {
            0
        };
        let base = tuned_price(
            market_card_price(def_of(Some(state), &def_id), radiant),
            buffs,
            nerfs,
        );
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
            buffs: (buffs > 0).then_some(buffs),
            nerfs: (nerfs > 0).then_some(nerfs),
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
/// order, then each card that pays Credits as a Resource does the same way (R4410000), in the place and
/// order R1861000's hand-sale cards were quoted in, so a game draws as it did.
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
    for def_id in credit_paying_def_ids() {
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

/// R2250002, R1853002: `player`'s turn start rolls their offers and the quotes; a closed one draws nothing.
pub fn marketplace_turn_start(sink: &mut EngineSink<'_>, player: PlayerId) {
    if sink.state.marketplace.is_none() {
        return;
    }
    reroll_for(sink, player);
    requote(sink);
}

/// R1853002: what this card (a Resource, or a card that pays Credits as one does, R4410000) fetches at an
/// open Marketplace this turn, on its own face; `None` while it is closed or quotes no such card.
/// R3548002: a Warhero Token quotes the game-wide worth whatever the Marketplace holds, so the
/// hand's braces always show it.
pub fn sell_quote(state: &GameState, card: &CardInstance) -> Option<i32> {
    if let Some(worth) = crate::warhero::worth_of(state, card) {
        return Some(worth);
    }
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

/// R1853005, R2250002: the copies of a lot `player` may still buy, none of another player's lot.
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

/// R1853003: why `player` may not buy the lot at index `offer` of `offered_to` now, or `Ok`: the one
/// rule `legal_actions`, the reducer and the view all ask. In order: their own main phase with no prompt
/// open, an open Marketplace, a lot at that index, a copy left to them, the Credits, room in their hand.
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

/// R3047003: the Buffs and Nerfs a lot carried, landed on the card bought, now in `player`'s hand, as
/// every Buff and Nerf is (§6.3, R386): each Buff one Upgrade, as Book of Buff's, then each Nerf one
/// Degrade that spares the cost row (`degrade_sparing_cost`), so the card keeps costing (0) (R3047001).
/// Each draws its row from the match rng and reports as any Buff or Nerf does, after `marketBought`.
fn land_offer_tuning(sink: &mut EngineSink<'_>, player: PlayerId, instance_id: &str, lot: &MarketOffer) {
    let Some(card) = crate::state::find_instance(sink.state, instance_id).cloned() else {
        return;
    };
    let mut ctx = make_context(
        sink,
        Some(&card),
        HookOptions {
            controller: Some(player),
            ..HookOptions::default()
        },
    );
    for _ in 0..lot.buffs.unwrap_or(0).max(0) {
        tune_once(&mut ctx, &card, TuneDirection::Upgrade, true);
    }
    for _ in 0..lot.nerfs.unwrap_or(0).max(0) {
        degrade_sparing_cost(&mut ctx, &card);
    }
}

/// R1853003, R3047001, R3047003: the buy, refused as `why_market_buy_refused` refuses it: a fresh card on
/// the lot's face is reported bought (`marketBought`, hidden as `addedToHand` is), its price paid in
/// Credits, the copy counted against `player`'s stock, and the card added to their hand costing
/// `MARKETPLACE_BOUGHT_COST`, (0), with the lot's Buffs and Nerfs landed on it. Nothing is played.
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
    card.cost_override = Some(MARKETPLACE_BOUGHT_COST);
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
    if crate::draw::add_to_hand(sink, &mut card) == crate::draw::AddToHandOutcome::Hand {
        land_offer_tuning(sink, player, &card.id, &lot);
    }
    Ok(())
}

/// R1853005: "add a lot for the rest of the game": the Marketplace opens if closed, then one card of
/// `pool` with a definition is drawn and offered to `player` alone at a fixed `price`, never Radiant,
/// with `MARKETPLACE_FIXED_LOT_STOCK` copies renewed at each of their turn starts (R2250002).
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
            buffs: None,
            nerfs: None,
        });
    }
}

/// R1853003, R2250002, §10.8: the Marketplace as `viewer` reads it, `None` while closed: each lot
/// offered to them with its index, price and copies left and, while they may act, the refusal of a buy
/// `legal_actions` does not list. Nothing sells, so `sales` is always empty (R4410000). Another
/// player's offers are theirs alone (R97).
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
                buffs: offer.buffs,
                nerfs: offer.nerfs,
                reason,
            }
        })
        .collect();
    Some(MarketplaceView {
        offers,
        sales: Vec::new(),
    })
}
