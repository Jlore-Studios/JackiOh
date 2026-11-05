//! Cosmetic packs and the Chase Shard bar (SPEC §14.2, §14.6; #1306): opening the pack the
//! cosmetic token cycle's last step pays, the bar each opening fills and the chase it buys.
//!
//! - [`roll_cosmetic_pack`]: the pure roll, from the shipped sets' pool, the account's
//!   [`CosmeticHoldings`] and a seed to what each slot grants (R1306000, R1306001), on the cosmetics'
//!   own scale and with no pity, leaning toward the cards the account owns.
//! - [`open_cosmetic_pack`] and [`buy_chase`], either under an optional `Idempotency-Key`
//!   ([`open_cosmetic_pack_once`], [`buy_chase_once`]), under which a retry answers as its first try
//!   did and spends nothing again (R2124000).
//! - The routes, all `AuthLevel::Active`, each spend in the account's turn (`economy::spend_slots`,
//!   R2378000).
//!
//! An opening and a purchase run under the account's lock (R1301003), so neither rolls on, nor spends,
//! counters the other is moving.

use std::collections::BTreeSet;
use std::sync::Arc;

use indexmap::IndexMap;
use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::rng::Rng;
use serde::{Deserialize, Serialize};

use crate::api::collection::{caller_profile, owned_in};
use crate::api::crypto::SystemIds;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, ok_of, optional_str, str};
use crate::app::App;
use crate::config::{
    CHASE_SHARD_PACKS, COSMETIC_PACK_CARD_WEIGHTS, COSMETIC_PACK_FLOOR_RARITY, COSMETIC_PACK_SLOTS,
    LEVEL_SKIP_WEIGHT, PACK_FLOORED_SLOT, TIER_WEIGHT_TOTAL,
};
use crate::cosmetics::{Cosmetic, CosmeticRarity, cosmetic_pack_cosmetics, implying, levels_below, offered};
use crate::db::store::Tx;
use crate::economy::idempotency::{RequestKey, Spent, keep, replayed};
use crate::economy::packs::{BarsSettled, bar_before, grant_counter, grant_unlocked, settle_bars};
use crate::economy::roll::{PackKind, PoolCard, pack_pool};
use crate::economy::store::{
    CosmeticGrant, Counter, EconomyReason, InventoryGrant, InventoryItem, OwnedCosmetics,
};
use crate::economy::tiers::{TierOdds, cosmetic_odds, cosmetic_tier, tier_draw};
use crate::economy::tokens::{
    BarState, choice_of, conflict, cosmetic_bar_points, held, next_level_only, unsold,
};

// A cosmetic pack holds a slot, and a full Chase Shard bar holds a pack.
const _: () = assert!(COSMETIC_PACK_SLOTS >= 1);
const _: () = assert!(CHASE_SHARD_PACKS >= 1);

/// What the account holds as a cosmetic pack opens: its copies of each card and its cosmetics,
/// read through the sets for membership alone, so no hash order touches a draw (R1306003).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CosmeticHoldings {
    pub owned: IndexMap<String, i64>,
    /// Each per-card cosmetic owned, as `(card id, cosmetic id)`.
    pub card_cosmetics: BTreeSet<(String, String)>,
    /// Each account cosmetic owned.
    pub account_cosmetics: BTreeSet<String>,
}

impl CosmeticHoldings {
    /// The starter and default cosmetics have no row, and no pack drops them (R1306000).
    pub fn of(owned: IndexMap<String, i64>, cosmetics: &OwnedCosmetics) -> Self {
        CosmeticHoldings {
            owned,
            card_cosmetics: cosmetics
                .cards
                .iter()
                .map(|held| (held.card_id.clone(), held.cosmetic_id.clone()))
                .collect(),
            account_cosmetics: cosmetics.account.iter().cloned().collect(),
        }
    }

    /// Whether the account owns `MAX_COPIES` of `card_id`: the card is not "new" then, and a
    /// cosmetic for it grants no copy.
    fn owns(&self, card_id: &str) -> bool {
        self.owned.get(card_id).copied().unwrap_or(0) >= i64::from(MAX_COPIES)
    }

    /// Whether the account owns the per-card cosmetic `id` on `card_id`: a row of it, or of a retired entry that
    /// owns it there (R2725002, R3015006).
    fn holds(&self, card_id: &str, id: &str) -> bool {
        let row = |id: &str| self.card_cosmetics.contains(&(card_id.to_string(), id.to_string()));
        row(id) || implying(id).any(row)
    }
}

/// What one cosmetic pack slot grants (R1306000, R1306001).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CosmeticSlotGrant {
    /// An unowned cosmetic of `cosmetic_rarity`, which is the rarity rolled or the nearest one
    /// with something unowned. `card_id` is the card a per-card cosmetic goes on (`None` for an
    /// account cosmetic), and `card_granted` whether the account lacked that card, so the slot
    /// grants it too. R3015003: `below`, the levels an out-of-order pull grants on that card with its
    /// level, lowest first; empty, and left out, for every other grant.
    Cosmetic {
        cosmetic_id: String,
        cosmetic_rarity: CosmeticRarity,
        card_id: Option<String>,
        card_granted: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        below: Vec<String>,
    },
    /// A spent slot: nothing at all was left to own, so it grants nothing. Like every slot, it
    /// adds the points of the cosmetic rarity it rolled to both token bars (R1306001, R1953000,
    /// R1999000).
    Bars,
}

/// One slot of an opened cosmetic pack: the rarity it rolled and what that granted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticSlot {
    pub rarity: CosmeticRarity,
    pub grant: CosmeticSlotGrant,
}

/// An opening rolled: its slots in order and the draws it took.
#[derive(Clone, Debug, PartialEq)]
pub struct CosmeticOpening {
    pub slots: Vec<CosmeticSlot>,
    pub draws: u32,
}

/// What every economy row of an opening logs in its `ref` (R1306003, §14.11): everything the roll
/// read but the holdings before it, and its result. [`roll_cosmetic_pack`] over that catalog's
/// pool, the holdings before it and `cosmetics`, with `seed`, replays it however the live catalog
/// or registry has moved since.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticOpeningLog {
    /// The catalog version the pool was built from (`jackioh_cards::catalog_version()` then).
    pub catalog: String,
    pub seed: String,
    /// The registry's droppable entries the roll picked from ([`cosmetic_pack_cosmetics`]), whole,
    /// in registry order.
    pub cosmetics: Vec<Cosmetic>,
    pub slots: Vec<CosmeticSlot>,
    pub draws: u32,
}

impl CosmeticOpeningLog {
    pub fn of(catalog: &str, registry: &[Cosmetic], seed: &str, opening: &CosmeticOpening) -> Self {
        CosmeticOpeningLog {
            catalog: catalog.to_string(),
            seed: seed.to_string(),
            cosmetics: cosmetic_pack_cosmetics(registry),
            slots: opening.slots.clone(),
            draws: opening.draws,
        }
    }
}

/// What opening a cosmetic pack answers: its slots in order, the Chase Shard bar's points before
/// it and the bar after it, the cosmetic packs still held and what the opening did to both token
/// bars.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenedCosmeticPack {
    pub slots: Vec<CosmeticSlot>,
    /// The Chase Shard bar's points the opening found, read in its transaction, so a client fills
    /// the bar from here to `shard_bar` as it does the token bars (`bars`).
    pub shard_before: i64,
    pub shard_bar: BarState,
    pub unopened: i64,
    pub bars: BarsSettled,
}

/// A cosmetic pack's odds as `GET /api/cosmetic-packs` shows them (R1339000): one roll of a slot
/// and of the floored slot on `COSMETIC_TIER_WEIGHTS`, the fallback included. R3015003: and `level_skip`, the
/// chance a slot that finds a level past the next one open grants that level and every level below it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPackOdds {
    pub slot: TierOdds,
    pub floored_slot: TierOdds,
    pub level_skip: f64,
}

/// R3015003: the chance an out-of-order pull is taken where one is open, as both odds `GET`s list it.
pub fn level_skip_chance() -> f64 {
    f64::from(LEVEL_SKIP_WEIGHT) / f64::from(TIER_WEIGHT_TOTAL)
}

/// What `GET /api/cosmetic-packs` reads back (R1306002, R1339000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPacks {
    pub shard_bar: BarState,
    pub unopened: i64,
    pub odds: CosmeticPackOdds,
}

/// The pool cards a per-card cosmetic is not owned on: those it is offered (R2725001), all of them for a foil,
/// a finish or a border, a full art's own card alone (R1313000). One a retired entry owns is owned there
/// (R2725002). An account cosmetic has none. R3015003: a level only where it is the next, every level below it
/// in `droppable` owned there.
fn placements<'p>(
    cosmetic: &Cosmetic,
    droppable: &[Cosmetic],
    pool: &'p [PoolCard],
    holdings: &CosmeticHoldings,
) -> Vec<&'p str> {
    open_places(cosmetic, droppable, pool, holdings, true)
}

/// R3015003: the pool cards a level is open on out of order: offered, not owned, and some level below it not.
fn skip_placements<'p>(
    cosmetic: &Cosmetic,
    droppable: &[Cosmetic],
    pool: &'p [PoolCard],
    holdings: &CosmeticHoldings,
) -> Vec<&'p str> {
    open_places(cosmetic, droppable, pool, holdings, false)
}

/// The pool cards `cosmetic` is offered and not owned on, where every level below it is owned (`next`) or not.
fn open_places<'p>(
    cosmetic: &Cosmetic,
    droppable: &[Cosmetic],
    pool: &'p [PoolCard],
    holdings: &CosmeticHoldings,
    next: bool,
) -> Vec<&'p str> {
    if !cosmetic.kind.is_per_card() {
        return Vec::new();
    }
    let below = levels_below(droppable, cosmetic);
    pool.iter()
        .filter(|card| offered(cosmetic, Some(&card.id)))
        .filter(|card| !holdings.holds(&card.id, &cosmetic.id))
        .filter(|card| below.iter().all(|level| holdings.holds(&card.id, &level.id)) == next)
        .map(|card| card.id.as_str())
        .collect()
}

/// Whether the account can still gain `cosmetic` in order: an account cosmetic it does not own, or a
/// per-card one with a card left to own it on as its next level.
fn is_open(cosmetic: &Cosmetic, droppable: &[Cosmetic], pool: &[PoolCard], holdings: &CosmeticHoldings) -> bool {
    if cosmetic.kind.is_per_card() {
        !placements(cosmetic, droppable, pool, holdings).is_empty()
    } else {
        !holdings.account_cosmetics.contains(&cosmetic.id)
    }
}

/// A per-card cosmetic's card (R1306000): one of `placements`, each weighed by
/// `COSMETIC_PACK_CARD_WEIGHTS`. One draw; no placement is `None` and draws nothing.
pub fn pick_card<'c>(
    placements: &[&'c str],
    owned: &IndexMap<String, i64>,
    rng: &mut Rng,
) -> Option<&'c str> {
    let weight = |card_id: &str| {
        if owned.get(card_id).copied().unwrap_or(0) >= i64::from(MAX_COPIES) {
            COSMETIC_PACK_CARD_WEIGHTS.owned
        } else {
            COSMETIC_PACK_CARD_WEIGHTS.unowned
        }
    };
    if placements.is_empty() {
        return None;
    }
    let total: f64 = placements.iter().map(|card| weight(card)).sum();
    let mut point = rng.next() * total;
    for card in placements {
        point -= weight(card);
        if point < 0.0 {
            return Some(card);
        }
    }
    // Rounding can leave the point a hair past the last edge.
    placements.last().copied()
}

/// What a slot grants once its rarity `rolled` (R1306000, R1306001): the first rarity in
/// `rolled.floored_fallback_order(floor)` with a cosmetic of `droppable` still open supplies one,
/// picked uniformly (one draw); a per-card one then takes its card by [`pick_card`] (one draw), and
/// an account one has none. A Legendary or Mythic one is a chase, and drops like any other
/// (R1333000). With nothing open in any rarity the slot is spent: `Bars`, with no draw.
///
/// R3015003: a level is open in order only as its card's next. Each rarity tried that has a level open out of
/// order on some card first takes one draw below `TIER_WEIGHT_TOTAL`: below `LEVEL_SKIP_WEIGHT` it picks such a
/// level (one draw) and its card among those (one draw), and grants it with every level below it the card does
/// not own (`below`); at or past it, the rarity is tried in order as ever.
pub fn slot_grant(
    rolled: CosmeticRarity,
    floor: CosmeticRarity,
    pool: &[PoolCard],
    droppable: &[Cosmetic],
    holdings: &CosmeticHoldings,
    rng: &mut Rng,
) -> CosmeticSlotGrant {
    for tier in rolled.floored_fallback_order(floor) {
        let skips: Vec<(&Cosmetic, Vec<&str>)> = droppable
            .iter()
            .filter(|cosmetic| cosmetic.rarity == tier && cosmetic.level.is_some())
            .map(|cosmetic| (cosmetic, skip_placements(cosmetic, droppable, pool, holdings)))
            .filter(|(_, places)| !places.is_empty())
            .collect();
        if !skips.is_empty()
            && tier_draw(rng) < LEVEL_SKIP_WEIGHT
            && let Some((cosmetic, places)) = rng.pick(&skips)
            && let Some(card) = pick_card(places, &holdings.owned, rng)
        {
            return CosmeticSlotGrant::Cosmetic {
                cosmetic_id: cosmetic.id.clone(),
                cosmetic_rarity: tier,
                card_id: Some(card.to_string()),
                card_granted: !holdings.owns(card),
                below: levels_below(droppable, cosmetic)
                    .into_iter()
                    .filter(|below| !holdings.holds(card, &below.id))
                    .map(|below| below.id.clone())
                    .collect(),
            };
        }
        let open: Vec<&Cosmetic> = droppable
            .iter()
            .filter(|cosmetic| cosmetic.rarity == tier)
            .filter(|cosmetic| is_open(cosmetic, droppable, pool, holdings))
            .collect();
        let Some(cosmetic) = rng.pick(&open) else {
            continue;
        };
        // An open per-card cosmetic has a placement, so the pick finds a card.
        let card_id = pick_card(&placements(cosmetic, droppable, pool, holdings), &holdings.owned, rng)
            .map(str::to_string);
        let card_granted = card_id.as_deref().is_some_and(|card| !holdings.owns(card));
        return CosmeticSlotGrant::Cosmetic {
            cosmetic_id: cosmetic.id.clone(),
            cosmetic_rarity: tier,
            card_id,
            card_granted,
            below: Vec::new(),
        };
    }
    CosmeticSlotGrant::Bars
}

/// The whole opening (R1306003, R1339002). Slot by slot, its draws in this order and no other: the
/// slot's tier, then its cosmetic's pick and, for a per-card one, its card's, each only when there
/// is something to pick. A slot's tier is a tiered roll on `COSMETIC_TIER_WEIGHTS`, the one at
/// `PACK_FLOORED_SLOT` floored at `COSMETIC_PACK_FLOOR_RARITY`, and falls no lower than that floor
/// while anything there or above is open (R1339000, R1306001). Ownership moves slot by slot, so a
/// grant earlier in the pack is owned later in it (R1306000).
pub fn roll_cosmetic_pack(
    pool: &[PoolCard],
    holdings: &CosmeticHoldings,
    registry: &[Cosmetic],
    seed: &str,
) -> CosmeticOpening {
    let droppable = cosmetic_pack_cosmetics(registry);
    let mut rng = Rng::new(seed, 0);
    let mut holdings = holdings.clone();
    let mut slots = Vec::with_capacity(COSMETIC_PACK_SLOTS);
    for at in 0..COSMETIC_PACK_SLOTS {
        let floor = (at == PACK_FLOORED_SLOT).then_some(COSMETIC_PACK_FLOOR_RARITY);
        let rarity = cosmetic_tier(floor, &mut rng);
        let grant = slot_grant(
            rarity,
            floor.unwrap_or(CosmeticRarity::Common),
            pool,
            &droppable,
            &holdings,
            &mut rng,
        );
        if let CosmeticSlotGrant::Cosmetic {
            cosmetic_id,
            card_id,
            card_granted,
            below,
            ..
        } = &grant
        {
            match card_id {
                Some(card_id) => {
                    for cosmetic_id in below.iter().chain([cosmetic_id]) {
                        holdings
                            .card_cosmetics
                            .insert((card_id.clone(), cosmetic_id.clone()));
                    }
                    if *card_granted {
                        *holdings.owned.entry(card_id.clone()).or_insert(0) += 1;
                    }
                }
                None => {
                    holdings.account_cosmetics.insert(cosmetic_id.clone());
                }
            }
        }
        slots.push(CosmeticSlot { rarity, grant });
    }
    CosmeticOpening {
        slots,
        draws: rng.cursor(),
    }
}

fn no_pack_held() -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, "no unopened cosmetic pack")
}

/// §14.6: opens one unopened cosmetic pack, in one transaction. Under the account's lock (R1301003) it
/// rolls the opening from `seed` ([`roll_cosmetic_pack`] over the shipped sets' pool) and writes every
/// grant: the pack spent, the new cards (the collection and its ledger, reason `pack`), the cosmetics, the
/// Chase Shard bar's one, and both token bars' points, each full bar paying its tokens with the overflow
/// kept (R1305000, R1306001). Every economy row's `ref` is the opening's log; the collection's rows have no
/// `ref` and share the opening's `at`. No pack held is 409, writing nothing. With no key,
/// [`open_cosmetic_pack_once`]'s opening.
pub async fn open_cosmetic_pack(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    seed: &str,
) -> Result<OpenedCosmeticPack, ApiError> {
    open_cosmetic_pack_once(app, profile_id, registry, seed, None)
        .await?
        .now()
}

/// [`open_cosmetic_pack`] under an `Idempotency-Key` (R2124000): right after the account's lock,
/// before the pack held is read (a retry finds the pack its first try spent gone), a key already
/// committed answers the opening it kept, and an opening's answer is kept in its own transaction.
pub async fn open_cosmetic_pack_once(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    seed: &str,
    request: Option<&RequestKey>,
) -> Result<Spent<OpenedCosmeticPack>, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
    let inventory = tx.economy_inventory_get(profile_id).await?;
    if held(&inventory, InventoryItem::CosmeticPack, "") < 1 {
        return Err(no_pack_held());
    }
    let counters = tx.economy_counters_get(profile_id).await?;
    let holdings = CosmeticHoldings::of(
        owned_in(&mut tx, profile_id).await?,
        &tx.economy_cosmetics_get(profile_id).await?,
    );
    let pool = pack_pool(&app.catalog.defs, PackKind::Global);
    let opening = roll_cosmetic_pack(&pool, &holdings, registry, seed);
    let reference = Some(serde_json::to_string(&CosmeticOpeningLog::of(
        &app.catalog.version,
        registry,
        seed,
        &opening,
    ))?);
    let at = crate::app::now_ms();

    let unopened = tx
        .economy_inventory_grant(&InventoryGrant {
            profile_id: profile_id.to_string(),
            item: InventoryItem::CosmeticPack,
            key: String::new(),
            delta: -1,
            reason: EconomyReason::PackOpen,
            reference: reference.clone(),
            dev: false,
            at,
        })
        .await?
        .ok_or_else(no_pack_held)?;

    let mut unlocked: IndexMap<String, i64> = IndexMap::new();
    for slot in &opening.slots {
        if let CosmeticSlotGrant::Cosmetic {
            card_id: Some(card_id),
            card_granted: true,
            ..
        } = &slot.grant
        {
            *unlocked.entry(card_id.clone()).or_insert(0) += 1;
        }
    }
    grant_unlocked(&mut tx, profile_id, &holdings.owned, &unlocked, at).await?;

    for slot in &opening.slots {
        if let CosmeticSlotGrant::Cosmetic {
            cosmetic_id,
            card_id,
            below,
            ..
        } = &slot.grant
        {
            // R3015003: an out-of-order pull's levels below it first, each a row of its own.
            for cosmetic_id in below.iter().chain([cosmetic_id]) {
                let granted = tx
                    .economy_cosmetics_grant(&CosmeticGrant {
                        profile_id: profile_id.to_string(),
                        card_id: card_id.clone(),
                        cosmetic_id: cosmetic_id.clone(),
                        delta: 1,
                        reason: EconomyReason::PackOpen,
                        reference: reference.clone(),
                        dev: false,
                        at,
                    })
                    .await?;
                if !granted {
                    return Err(ApiError::internal(format!(
                        "{cosmetic_id} was already owned on {card_id:?}"
                    )));
                }
            }
        }
    }

    // The Chase Shard bar (R1306002): one per opening, and a full bar holds.
    let shard_before = bar_before(&counters, Counter::ChaseShardBar);
    let mut shard = shard_before;
    if shard < CHASE_SHARD_PACKS {
        grant_counter(
            &mut tx,
            profile_id,
            Counter::ChaseShardBar,
            1,
            EconomyReason::PackOpen,
            &reference,
            at,
        )
        .await?;
        shard += 1;
    }

    // Each slot's points are its rolled cosmetic rarity's, not the one it fell to, whatever it
    // grants (R1306001, R1953000, R1999000).
    let slot_points: Vec<i64> = opening
        .slots
        .iter()
        .map(|slot| cosmetic_bar_points(slot.rarity))
        .collect();
    let spent = opening
        .slots
        .iter()
        .filter(|slot| slot.grant == CosmeticSlotGrant::Bars)
        .count();
    let bars = settle_bars(&mut tx, profile_id, &counters, slot_points, &reference, at).await?;
    let opened = OpenedCosmeticPack {
        slots: opening.slots,
        shard_before,
        shard_bar: BarState {
            points: shard,
            size: CHASE_SHARD_PACKS,
        },
        unopened,
        bars,
    };
    keep(&mut tx, profile_id, request, &opened).await?;

    tx.commit().await?;
    tracing::info!(
        event = "cosmetic_packs.opened",
        "profileId" = %profile_id,
        cards = unlocked.len(),
        spent,
    );
    Ok(Spent::Now(opened))
}

/// §14.6: a full Chase Shard bar buys any chase cosmetic (Legendary or Mythic, R1333000) the account does
/// not own, per-card for a card the account owns (a full art names its own card alone, R1313000) or on the
/// account, in one transaction: the cosmetic's grant, then the bar emptied (R1306002). An unknown cosmetic
/// id is 400. Refuses with 409, writing nothing, when the bar is not full, the cosmetic sits below
/// Legendary, a per-card cosmetic names no owned card, an account cosmetic is given one, or it is already
/// owned (a ledger row, or a default or starter, which §14.2 gives every account), in that order. With no
/// key, [`buy_chase_once`]'s purchase.
pub async fn buy_chase(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    cosmetic_id: &str,
    card_id: Option<&str>,
) -> Result<(), ApiError> {
    buy_chase_once(app, profile_id, registry, cosmetic_id, card_id, None)
        .await?
        .now()?;
    Ok(())
}

/// [`buy_chase`] under an `Idempotency-Key` (R2124000), answering the bar and packs after it, read
/// in its transaction: right after the account's lock, before the bar is read (a retry finds the
/// bar its first try emptied), a key already committed answers what it kept, and a purchase's
/// answer is kept in its own transaction.
pub async fn buy_chase_once(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    cosmetic_id: &str,
    card_id: Option<&str>,
    request: Option<&RequestKey>,
) -> Result<Spent<CosmeticPacks>, ApiError> {
    let cosmetic = registry
        .iter()
        .find(|entry| entry.id == cosmetic_id)
        .ok_or_else(|| bad_request(format!("no cosmetic is named \"{cosmetic_id}\"")))?;
    if let Some(why) = unsold(cosmetic, card_id) {
        return Err(bad_request(why));
    }
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
    let bar = bar_before(
        &tx.economy_counters_get(profile_id).await?,
        Counter::ChaseShardBar,
    );
    let owned = owned_in(&mut tx, profile_id).await?;
    let cosmetics = tx.economy_cosmetics_get(profile_id).await?;
    let choice = choice_of(cosmetic, card_id, &owned, &cosmetics);
    next_level_only(registry, cosmetic, card_id, &cosmetics)?;
    if bar < CHASE_SHARD_PACKS {
        return Err(conflict("the Chase Shard bar is not full"));
    }
    // The registry's chase mark, a Legendary or Mythic cosmetic's but a Jlorious one's, which only
    // reaching Jlorious grants (R2755000).
    if !cosmetic.chase {
        return Err(conflict(
            "the Chase Shard bar buys only a chase: a Legendary or Mythic cosmetic, but the Jlorious one",
        ));
    }
    if !choice.fits {
        return Err(conflict("that cosmetic fits no card held for it"));
    }
    if choice.owned {
        return Err(conflict("that cosmetic is already owned"));
    }
    let at = crate::app::now_ms();
    let granted = tx
        .economy_cosmetics_grant(&CosmeticGrant {
            profile_id: profile_id.to_string(),
            card_id: card_id.map(str::to_string),
            cosmetic_id: cosmetic_id.to_string(),
            delta: 1,
            reason: EconomyReason::ChaseShard,
            reference: None,
            dev: false,
            at,
        })
        .await?;
    if !granted {
        return Err(conflict("that cosmetic is already owned"));
    }
    grant_counter(
        &mut tx,
        profile_id,
        Counter::ChaseShardBar,
        -bar,
        EconomyReason::ChaseShard,
        &None,
        at,
    )
    .await?;
    let packs = cosmetic_packs_in(&mut tx, profile_id).await?;
    keep(&mut tx, profile_id, request, &packs).await?;
    tx.commit().await?;
    tracing::info!(event = "cosmetic_packs.chase_bought", "profileId" = %profile_id, cosmetic = cosmetic_id);
    Ok(Spent::Now(packs))
}

async fn read_cosmetic_packs(app: &App, profile_id: &str) -> Result<CosmeticPacks, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let packs = cosmetic_packs_in(&mut tx, profile_id).await?;
    tx.commit().await?;
    Ok(packs)
}

/// [`CosmeticPacks`] as `tx` reads them. A chase's purchase reads its answer through it before its
/// commit.
async fn cosmetic_packs_in(tx: &mut Tx<'_>, profile_id: &str) -> Result<CosmeticPacks, ApiError> {
    let counters = tx.economy_counters_get(profile_id).await?;
    let inventory = tx.economy_inventory_get(profile_id).await?;
    Ok(CosmeticPacks {
        shard_bar: BarState {
            points: bar_before(&counters, Counter::ChaseShardBar),
            size: CHASE_SHARD_PACKS,
        },
        unopened: held(&inventory, InventoryItem::CosmeticPack, ""),
        odds: CosmeticPackOdds {
            slot: cosmetic_odds(None),
            floored_slot: cosmetic_odds(Some(COSMETIC_PACK_FLOOR_RARITY)),
            level_skip: level_skip_chance(),
        },
    })
}

/// `GET /api/cosmetic-packs` (`AuthLevel::Active`): [`read_cosmetic_packs`].
pub async fn get_cosmetic_packs(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    ok_of(&read_cosmetic_packs(app, &profile.id).await?)
}

/// `POST /api/cosmetic-packs/open` (`AuthLevel::Active`): [`open_cosmetic_pack`] over the shipped
/// registry, from a fresh seed (`SystemIds::seed`, 16 random bytes), so no seed the account can
/// read foretells a later roll (§14.11), with an optional `Idempotency-Key` (R2124000);
/// [`open_cosmetic_pack_once`], in the account's spend turn (R2378000).
pub async fn post_open(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let request = RequestKey::of(app, &req, "/api/cosmetic-packs/open")?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    let _turn = app.spends.claim(&profile.id).await?;
    open_cosmetic_pack_once(app, &profile.id, &registry, &SystemIds.seed(), request.as_ref())
        .await?
        .answer()
}

/// `POST /api/cosmetic-packs/chase` (`AuthLevel::Active`): `{ cosmeticId, cardId? }` over the
/// shipped registry, with an optional `Idempotency-Key` (R2124000); [`buy_chase_once`], in the
/// account's spend turn (R2378000), answering the bar and packs after it.
pub async fn post_chase(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let cosmetic_id = str(&req.body, "cosmeticId")?;
    let card_id = optional_str(&req.body, "cardId")?;
    let request = RequestKey::of(app, &req, "/api/cosmetic-packs/chase")?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    let _turn = app.spends.claim(&profile.id).await?;
    buy_chase_once(
        app,
        &profile.id,
        &registry,
        &cosmetic_id,
        card_id.as_deref(),
        request.as_ref(),
    )
    .await?
    .answer()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmetics::CosmeticKind;
    use crate::cosmetics::CosmeticRarity::{Common, Epic, Legendary, Mythic, Rare};
    use crate::economy::store::CardCosmetic;
    use crate::economy::test_support::{foil, four_foils};
    use crate::economy::tiers::RATE_TOLERANCE;
    use jackioh_engine::wire::PrintedRarity;

    /// Draws enough for a rate to settle inside `RATE_TOLERANCE`.
    const DRAWS: usize = 20_000;

    fn card(id: &str, rarity: PrintedRarity) -> PoolCard {
        PoolCard {
            id: id.to_owned(),
            rarity,
        }
    }

    /// A cosmetic of `kind` a pack may drop.
    fn cosmetic(id: &str, kind: CosmeticKind, rarity: CosmeticRarity) -> Cosmetic {
        Cosmetic {
            id: id.to_owned(),
            kind,
            name: id.to_owned(),
            rarity,
            default: false,
            starter: false,
            chase: rarity.is_chase(),
            jlorious: false,
            card: None,
            offer: Vec::new(),
            level: None,
        }
    }

    /// An account cosmetic a pack may drop: a playmat.
    fn table(id: &str, rarity: CosmeticRarity) -> Cosmetic {
        cosmetic(id, CosmeticKind::Table, rarity)
    }

    fn full_art(id: &str, rarity: CosmeticRarity, card_id: &str) -> Cosmetic {
        Cosmetic {
            card: Some(card_id.to_owned()),
            ..cosmetic(id, CosmeticKind::FullArt, rarity)
        }
    }

    fn pool_of(count: usize) -> Vec<PoolCard> {
        (0..count)
            .map(|at| {
                card(
                    &format!("card-{at}"),
                    *PrintedRarity::ALL.get(at % 5).unwrap_or(&PrintedRarity::Common),
                )
            })
            .collect()
    }

    fn owning(ids: &[&str]) -> IndexMap<String, i64> {
        ids.iter()
            .map(|id| ((*id).to_owned(), i64::from(MAX_COPIES)))
            .collect()
    }

    fn wearing(card_id: &str, cosmetics: &[&str]) -> CosmeticHoldings {
        CosmeticHoldings::of(
            IndexMap::new(),
            &OwnedCosmetics {
                cards: cosmetics
                    .iter()
                    .map(|cosmetic_id| CardCosmetic {
                        card_id: card_id.to_owned(),
                        cosmetic_id: (*cosmetic_id).to_owned(),
                    })
                    .collect(),
                account: Vec::new(),
            },
        )
    }

    fn named(grant: &CosmeticSlotGrant) -> Option<&str> {
        match grant {
            CosmeticSlotGrant::Cosmetic { cosmetic_id, .. } => Some(cosmetic_id),
            CosmeticSlotGrant::Bars => None,
        }
    }

    /// Each share of `counts` out of `draws` lands within `RATE_TOLERANCE` of `odds`, commonest
    /// first.
    fn assert_lands_on(counts: [usize; 5], draws: usize, odds: TierOdds, what: &str) {
        let chances = [odds.common, odds.rare, odds.epic, odds.legendary, odds.mythic];
        for (tier, (count, chance)) in counts.into_iter().zip(chances).enumerate() {
            let share = count as f64 / draws as f64;
            assert!(
                (share - chance).abs() <= RATE_TOLERANCE,
                "{what}: tier {tier}'s share {share} outside tolerance of {chance}"
            );
        }
    }

    #[test]
    fn r1339000_r1306000_every_slot_rolls_the_cosmetic_table_the_first_floored_at_epic() {
        // Nothing to drop: every slot is spent at the tier it rolled, on its tier's draw alone.
        let pool = pool_of(10);
        let openings = DRAWS / COSMETIC_PACK_SLOTS;
        let mut floored = [0_usize; 5];
        let mut others = [0_usize; 5];
        for seed in 0..openings {
            let opening = roll_cosmetic_pack(
                &pool,
                &CosmeticHoldings::default(),
                &[],
                &format!("r1339000-{seed}"),
            );
            assert_eq!(opening.slots.len(), COSMETIC_PACK_SLOTS);
            assert_eq!(opening.draws as usize, COSMETIC_PACK_SLOTS);
            for (at, slot) in opening.slots.iter().enumerate() {
                assert_eq!(slot.grant, CosmeticSlotGrant::Bars);
                let tier = usize::from(slot.rarity.tier());
                if at == PACK_FLOORED_SLOT {
                    assert!(slot.rarity >= COSMETIC_PACK_FLOOR_RARITY, "{slot:?}");
                    floored[tier] += 1;
                } else {
                    others[tier] += 1;
                }
            }
        }
        assert_lands_on(
            floored,
            openings,
            cosmetic_odds(Some(COSMETIC_PACK_FLOOR_RARITY)),
            "the floored slot",
        );
        assert_lands_on(
            others,
            openings * (COSMETIC_PACK_SLOTS - 1),
            cosmetic_odds(None),
            "the other slots",
        );
    }

    #[test]
    fn r1339002_a_cosmetic_pack_draws_one_tier_per_slot_first() {
        let pool = pool_of(40);
        let mut registry = four_foils();
        registry.push(table("table-drop", CosmeticRarity::Rare));
        for seed in 0..50 {
            let seed = format!("r1339002-{seed}");
            let opening = roll_cosmetic_pack(&pool, &CosmeticHoldings::default(), &registry, &seed);
            // Each slot's tier is the draw at the cursor its slot starts on, and its grant takes one
            // draw for the cosmetic and one more for a per-card cosmetic's card.
            let mut cursor = 0;
            for (at, slot) in opening.slots.iter().enumerate() {
                let floor = (at == PACK_FLOORED_SLOT).then_some(COSMETIC_PACK_FLOOR_RARITY);
                assert_eq!(
                    slot.rarity,
                    cosmetic_tier(floor, &mut Rng::new(&seed, cursor)),
                    "{seed}, slot {at}"
                );
                cursor += match &slot.grant {
                    CosmeticSlotGrant::Bars => 1,
                    CosmeticSlotGrant::Cosmetic { card_id: None, .. } => 2,
                    CosmeticSlotGrant::Cosmetic { card_id: Some(_), .. } => 3,
                };
            }
            assert_eq!(opening.draws, cursor, "{seed}");
        }
    }

    #[test]
    fn r1306000_common_cosmetics_drop_from_a_cosmetic_pack() {
        let pool = pool_of(40);
        let registry = four_foils();
        let mut common = 0;
        for seed in 0..200 {
            let opening = roll_cosmetic_pack(
                &pool,
                &CosmeticHoldings::default(),
                &registry,
                &format!("r1306000-{seed}"),
            );
            assert_eq!(opening.slots.len(), COSMETIC_PACK_SLOTS);
            let floored = &opening.slots[PACK_FLOORED_SLOT];
            assert!(floored.rarity >= COSMETIC_PACK_FLOOR_RARITY, "{floored:?}");
            for (at, slot) in opening.slots.iter().enumerate() {
                let CosmeticSlotGrant::Cosmetic { cosmetic_rarity, .. } = &slot.grant else {
                    panic!("an open registry spends no slot: {slot:?}");
                };
                if at == PACK_FLOORED_SLOT {
                    assert!(*cosmetic_rarity >= COSMETIC_PACK_FLOOR_RARITY, "{slot:?}");
                }
                if *cosmetic_rarity == Common {
                    common += 1;
                }
            }
        }
        assert!(common > 0, "no Common cosmetic in 200 openings");
    }

    #[test]
    fn r1306000_a_pack_drops_account_cosmetics_but_no_default_or_starter() {
        let default = Cosmetic {
            default: true,
            ..table("table-default", CosmeticRarity::Common)
        };
        let starter = Cosmetic {
            starter: true,
            ..foil("foil-starter", CosmeticRarity::Rare)
        };
        let registry = [
            default,
            starter,
            table("table-drop", CosmeticRarity::Common),
            foil("foil-drop", CosmeticRarity::Common),
            full_art("full-art-drop", CosmeticRarity::Epic, "card-1"),
        ];
        let ids: Vec<String> = cosmetic_pack_cosmetics(&registry)
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        // R2725000: a full art is retired, so no pack drops one.
        assert_eq!(ids, ["table-drop", "foil-drop"]);

        // An account cosmetic drops for the account, on no card, and once.
        let only_table = [table("table-drop", CosmeticRarity::Common)];
        let opening = roll_cosmetic_pack(
            &pool_of(10),
            &CosmeticHoldings::default(),
            &only_table,
            "r1306000",
        );
        let drops: Vec<&CosmeticSlotGrant> = opening.slots.iter().map(|slot| &slot.grant).collect();
        assert_eq!(
            drops[0],
            &CosmeticSlotGrant::Cosmetic {
                cosmetic_id: "table-drop".to_owned(),
                cosmetic_rarity: Common,
                card_id: None,
                card_granted: false,
                below: Vec::new(),
            }
        );
        assert!(drops[1..].iter().all(|grant| **grant == CosmeticSlotGrant::Bars));
    }

    #[test]
    fn r1306000_a_cosmetic_for_an_unowned_card_grants_it() {
        let pool = pool_of(10);
        let droppable = [full_art("full-art-3", CosmeticRarity::Epic, "card-3")];
        let mut rng = Rng::new("r1306000", 0);
        let unowned = slot_grant(
            Epic,
            Epic,
            &pool,
            &droppable,
            &CosmeticHoldings::default(),
            &mut rng,
        );
        assert_eq!(
            unowned,
            CosmeticSlotGrant::Cosmetic {
                cosmetic_id: "full-art-3".to_owned(),
                cosmetic_rarity: Epic,
                card_id: Some("card-3".to_owned()),
                card_granted: true,
                below: Vec::new(),
            }
        );
        let holdings = CosmeticHoldings {
            owned: owning(&["card-3"]),
            ..CosmeticHoldings::default()
        };
        let owned = slot_grant(Epic, Epic, &pool, &droppable, &holdings, &mut rng);
        assert_eq!(
            owned,
            CosmeticSlotGrant::Cosmetic {
                cosmetic_id: "full-art-3".to_owned(),
                cosmetic_rarity: Epic,
                card_id: Some("card-3".to_owned()),
                card_granted: false,
                below: Vec::new(),
            }
        );
        // A full art on a card that is not in the pool has nowhere to go.
        let stray = [full_art("full-art-x", CosmeticRarity::Epic, "not-a-card")];
        assert_eq!(
            slot_grant(Epic, Epic, &pool, &stray, &CosmeticHoldings::default(), &mut rng),
            CosmeticSlotGrant::Bars
        );
    }

    #[test]
    fn r1306000_ownership_moves_slot_by_slot() {
        // One card and two foils: the first slot grants the card with the first foil, the second
        // finds the card owned.
        let pool = [card("c", PrintedRarity::Epic)];
        let registry = [
            foil("foil-a", CosmeticRarity::Epic),
            foil("foil-b", CosmeticRarity::Epic),
        ];
        let opening = roll_cosmetic_pack(&pool, &CosmeticHoldings::default(), &registry, "r1306000");
        let grants: Vec<&CosmeticSlotGrant> = opening.slots.iter().map(|slot| &slot.grant).collect();
        let cosmetics: Vec<&str> = grants.iter().filter_map(|grant| named(grant)).collect();
        assert_eq!(cosmetics.len(), COSMETIC_PACK_SLOTS.min(2));
        assert_eq!(cosmetics.iter().collect::<BTreeSet<_>>().len(), cosmetics.len());
        for (at, grant) in grants.iter().enumerate() {
            match grant {
                CosmeticSlotGrant::Cosmetic { card_granted, .. } => assert_eq!(*card_granted, at == 0),
                CosmeticSlotGrant::Bars => assert!(at >= 2),
            }
        }
    }

    #[test]
    fn r1306000_the_card_leans_toward_owned_cards() {
        let owned = owning(&["owned"]);
        let places = ["owned", "new"];
        let mut rng = Rng::new("r1306000-lean", 0);
        let leaned = (0..DRAWS)
            .filter(|_| pick_card(&places, &owned, &mut rng) == Some("owned"))
            .count();
        let share = leaned as f64 / DRAWS as f64;
        let expected = COSMETIC_PACK_CARD_WEIGHTS.owned
            / (COSMETIC_PACK_CARD_WEIGHTS.owned + COSMETIC_PACK_CARD_WEIGHTS.unowned);
        assert!(
            (share - expected).abs() <= RATE_TOLERANCE,
            "owned share {share} outside tolerance of {expected}"
        );
        assert_eq!(rng.cursor() as usize, DRAWS);
        assert_eq!(pick_card(&[], &owned, &mut rng), None);
        assert_eq!(rng.cursor() as usize, DRAWS);
    }

    #[test]
    fn r1306001_a_tier_with_nothing_unowned_falls_below_then_above() {
        let pool = [card("c", PrintedRarity::Common)];
        let foils = four_foils();
        let none = CosmeticHoldings::default();
        let grant = |rolled, droppable: &[Cosmetic], holdings: &CosmeticHoldings| {
            let mut rng = Rng::new("r1306001", 0);
            slot_grant(rolled, Common, &pool, droppable, holdings, &mut rng)
        };
        let tier_of = |grant: &CosmeticSlotGrant| match grant {
            CosmeticSlotGrant::Cosmetic { cosmetic_rarity, .. } => Some(*cosmetic_rarity),
            CosmeticSlotGrant::Bars => None,
        };

        // No Legendary or Mythic cosmetic is in the registry: both fall to Epic.
        let below_legendary = &foils[..3];
        assert_eq!(tier_of(&grant(Mythic, below_legendary, &none)), Some(Epic));
        assert_eq!(tier_of(&grant(Legendary, below_legendary, &none)), Some(Epic));
        // With every Epic owned, to Rare, and so on down.
        let epic_owned = wearing("c", &["foil-epic"]);
        assert_eq!(tier_of(&grant(Mythic, below_legendary, &epic_owned)), Some(Rare));
        let rare_owned = wearing("c", &["foil-epic", "foil-rare"]);
        assert_eq!(
            tier_of(&grant(Legendary, below_legendary, &rare_owned)),
            Some(Common)
        );
        // A rolled tier with something unowned keeps it.
        assert_eq!(tier_of(&grant(Legendary, &foils, &epic_owned)), Some(Legendary));
        // A tier below with nothing at all steps up: a Common roll with no Common cosmetic.
        assert_eq!(tier_of(&grant(Common, &foils[1..], &none)), Some(Rare));
        assert_eq!(tier_of(&grant(Common, &foils[2..], &rare_owned)), Some(Legendary));

        // Nothing unowned at all spends the slot, and draws nothing.
        let mut rng = Rng::new("r1306001", 0);
        let everything = wearing("c", &["foil-common", "foil-rare", "foil-epic", "foil-legendary"]);
        for rolled in CosmeticRarity::ALL {
            assert_eq!(
                slot_grant(rolled, Common, &pool, &foils, &everything, &mut rng),
                CosmeticSlotGrant::Bars
            );
            assert_eq!(
                slot_grant(rolled, Common, &pool, &[], &none, &mut rng),
                CosmeticSlotGrant::Bars
            );
        }
        assert_eq!(rng.cursor(), 0);
    }

    #[test]
    fn r1306001_the_guaranteed_slot_stays_epic_or_better_while_one_is_open() {
        let pool = [card("c", PrintedRarity::Common)];
        let rare_and_legendary = [
            foil("foil-rare", CosmeticRarity::Rare),
            foil("foil-legendary", CosmeticRarity::Legendary),
        ];
        let none = CosmeticHoldings::default();
        let mut rng = Rng::new("r1306001", 0);
        let tier_of = |grant: CosmeticSlotGrant| match grant {
            CosmeticSlotGrant::Cosmetic { cosmetic_rarity, .. } => Some(cosmetic_rarity),
            CosmeticSlotGrant::Bars => None,
        };
        let floor = COSMETIC_PACK_FLOOR_RARITY;
        // An Epic roll with no Epic open takes the Legendary before it drops to the Rare ...
        assert_eq!(
            tier_of(slot_grant(
                Epic,
                floor,
                &pool,
                &rare_and_legendary,
                &none,
                &mut rng
            )),
            Some(Legendary)
        );
        // ... where a slot with no such floor falls below first.
        assert_eq!(
            tier_of(slot_grant(
                Epic,
                Common,
                &pool,
                &rare_and_legendary,
                &none,
                &mut rng
            )),
            Some(Rare)
        );
        // With nothing Epic or better open, it takes what is left.
        let only_rare = &rare_and_legendary[..1];
        assert_eq!(
            tier_of(slot_grant(Mythic, floor, &pool, only_rare, &none, &mut rng)),
            Some(Rare)
        );
    }

    #[test]
    fn r1306003_the_same_seed_and_holdings_roll_the_same_opening() {
        let pool = pool_of(40);
        let holdings = CosmeticHoldings {
            owned: owning(&["card-0", "card-1", "card-2"]),
            ..wearing("card-0", &["foil-common"])
        };
        let mut registry = four_foils();
        registry.push(table("table-drop", CosmeticRarity::Rare));
        let first = roll_cosmetic_pack(&pool, &holdings, &registry, "r1306003");
        assert_eq!(first.slots.len(), COSMETIC_PACK_SLOTS);
        assert_eq!(roll_cosmetic_pack(&pool, &holdings, &registry, "r1306003"), first);
        assert_ne!(
            roll_cosmetic_pack(&pool, &holdings, &registry, "r1306003-other"),
            first
        );
        assert!(first.draws as usize >= COSMETIC_PACK_SLOTS);
        assert!(first.draws as usize <= COSMETIC_PACK_SLOTS * 3);
    }

    #[test]
    fn r1306003_the_log_holds_the_result_and_replays_after_the_registry_moves() {
        let pool = pool_of(40);
        let holdings = CosmeticHoldings::default();
        let mut registry = four_foils();
        registry.push(Cosmetic {
            starter: true,
            ..foil("foil-starter", CosmeticRarity::Rare)
        });
        registry.push(table("table-drop", CosmeticRarity::Common));
        let opening = roll_cosmetic_pack(&pool, &holdings, &registry, "r1306003");
        let log = CosmeticOpeningLog::of("catalog-v1", &registry, "r1306003", &opening);
        let log: CosmeticOpeningLog =
            serde_json::from_str(&serde_json::to_string(&log).expect("a log")).expect("the log read back");
        assert_eq!(log.catalog, "catalog-v1");
        assert_eq!(log.seed, "r1306003");
        // The result itself is logged, and only the droppable entries are.
        assert_eq!(log.slots, opening.slots);
        assert_eq!(log.draws, opening.draws);
        assert_eq!(log.cosmetics, cosmetic_pack_cosmetics(&registry));
        assert!(log.cosmetics.iter().all(|entry| !entry.starter));
        // The live registry moves on: everything becomes a starter, so a roll over it now drops
        // nothing. The logged entries still replay the opening exactly.
        let moved: Vec<Cosmetic> = registry
            .iter()
            .map(|entry| Cosmetic {
                starter: true,
                ..entry.clone()
            })
            .collect();
        assert_ne!(roll_cosmetic_pack(&pool, &holdings, &moved, &log.seed), opening);
        let replayed = roll_cosmetic_pack(&pool, &holdings, &log.cosmetics, &log.seed);
        assert_eq!(replayed, opening);
    }

    #[test]
    fn r1333000_a_chase_drops_from_a_cosmetic_pack_slot_that_lands_on_its_tier() {
        let registry = [
            foil("foil-epic", CosmeticRarity::Epic),
            foil("foil-legendary", CosmeticRarity::Legendary),
        ];
        assert!(registry[1].chase);
        assert_eq!(cosmetic_pack_cosmetics(&registry), registry);

        let pool = pool_of(10);
        let none = CosmeticHoldings::default();
        let mut rng = Rng::new("r1333000", 0);
        for (rolled, dropped) in [
            (Legendary, "foil-legendary"),
            (Mythic, "foil-legendary"),
            (Epic, "foil-epic"),
        ] {
            let grant = slot_grant(rolled, Common, &pool, &registry, &none, &mut rng);
            assert_eq!(named(&grant), Some(dropped), "{rolled}");
        }

        // Opened whole, the chase drops, and only from a slot that rolled Legendary or Mythic.
        let pool = pool_of(40);
        let mut chases = 0;
        for seed in 0..200 {
            let opening = roll_cosmetic_pack(&pool, &none, &registry, &format!("r1333000-{seed}"));
            for slot in &opening.slots {
                if named(&slot.grant) == Some("foil-legendary") {
                    assert!(slot.rarity.is_chase(), "{slot:?}");
                    chases += 1;
                }
            }
        }
        assert!(chases > 0, "no chase in 200 openings");
    }

    #[test]
    fn r2715000_an_epic_slot_drops_a_card_s_animated_art() {
        // A card's still Epic full art held and its Animated art, an Epic too, not: handed both, the Epic
        // slot drops the Animated art on its card, a pack slot like any other Epic per-card cosmetic.
        // R2725000: both are retired, so the shipped pool lists neither.
        let droppable = [
            full_art("full-art-a", CosmeticRarity::Epic, "card-1"),
            full_art("animated-a", CosmeticRarity::Epic, "card-1"),
        ];
        assert!(cosmetic_pack_cosmetics(&droppable).is_empty());
        let pool = pool_of(10);
        let holdings = CosmeticHoldings {
            owned: owning(&["card-1"]),
            ..wearing("card-1", &["full-art-a"])
        };
        let mut rng = Rng::new("r2715000", 0);
        assert_eq!(
            slot_grant(Epic, Epic, &pool, &droppable, &holdings, &mut rng),
            CosmeticSlotGrant::Cosmetic {
                cosmetic_id: "animated-a".to_owned(),
                cosmetic_rarity: Epic,
                card_id: Some("card-1".to_owned()),
                card_granted: false,
                below: Vec::new(),
            }
        );
        // With both held the slot has nothing left to drop.
        let both = CosmeticHoldings {
            owned: owning(&["card-1"]),
            ..wearing("card-1", &["full-art-a", "animated-a"])
        };
        assert_eq!(
            slot_grant(Epic, Epic, &pool, &droppable, &both, &mut rng),
            CosmeticSlotGrant::Bars
        );
    }

    #[test]
    fn r2725001_a_pack_places_a_version_on_an_offered_card() {
        use crate::cosmetics::{VERSION_FULL_ART_ID, registry};
        let droppable: Vec<Cosmetic> = cosmetic_pack_cosmetics(registry())
            .into_iter()
            .filter(|cosmetic| cosmetic.id == VERSION_FULL_ART_ID)
            .collect();
        assert_eq!(droppable.len(), 1);
        // Speciesist, a Common Unit, is not offered the Full art version; The Mother Pancake, a Legendary Unit, is.
        let pool = [
            PoolCard {
                id: "core-002".to_owned(),
                rarity: PrintedRarity::Common,
            },
            PoolCard {
                id: "classicplus-012".to_owned(),
                rarity: PrintedRarity::Legendary,
            },
        ];
        let holdings = CosmeticHoldings {
            owned: owning(&["core-002", "classicplus-012"]),
            ..CosmeticHoldings::default()
        };
        for seed in ["r2725001-a", "r2725001-b", "r2725001-c"] {
            let mut rng = Rng::new(seed, 0);
            assert_eq!(
                slot_grant(Legendary, Legendary, &pool, &droppable, &holdings, &mut rng),
                CosmeticSlotGrant::Cosmetic {
                    cosmetic_id: VERSION_FULL_ART_ID.to_owned(),
                    cosmetic_rarity: Legendary,
                    card_id: Some("classicplus-012".to_owned()),
                    card_granted: false,
                    below: Vec::new(),
                }
            );
        }
        // R2725002: its retired still Epic full art owns the version there, so nothing is left to place.
        let legacy = CosmeticHoldings {
            owned: owning(&["core-002", "classicplus-012"]),
            ..wearing("classicplus-012", &["full-art-the-mother-pancake"])
        };
        let mut rng = Rng::new("r2725001-d", 0);
        assert_eq!(
            slot_grant(Legendary, Legendary, &pool, &droppable, &legacy, &mut rng),
            CosmeticSlotGrant::Bars
        );
    }
}
