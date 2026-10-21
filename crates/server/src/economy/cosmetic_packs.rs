//! Cosmetic packs and the Chase Shard bar (SPEC §14.2, §14.6; #1306): opening the pack the
//! cosmetic token cycle's last step pays, the bar each opening fills and the chase it buys.
//!
//! - [`roll_cosmetic_pack`]: the pure roll, from the shipped sets' pool, the account's
//!   [`CosmeticHoldings`] and a seed to what each slot grants (R1306000, R1306001). Each slot's
//!   rarity is a tiered roll on `COSMETIC_TIER_WEIGHTS`, the floored slot's at
//!   `COSMETIC_PACK_FLOOR_RARITY` (R1339000), on the cosmetics' own scale and with no pity; a slot
//!   grants one unowned cosmetic, for a card or the account, leaning toward the cards the account
//!   owns, and its card when the account lacks it, or falls to the nearest rarity with something
//!   unowned, or pays bar points.
//! - [`open_cosmetic_pack`]: under the account's lock, one unopened cosmetic pack spent, its
//!   slots rolled from a fresh seed and every grant written, its cards, cosmetics, the Chase
//!   Shard bar's one and both token bars' points for the spent slots (R1306002), with the
//!   opening's log (its catalog, cosmetics, seed and result) in the `ref` of every economy row it
//!   writes (R1306003). The whole opening or nothing: every refusal returns before the commit,
//!   and the dropped `Tx` rolls it all back.
//! - [`buy_chase`]: a full Chase Shard bar buys one Legendary or Mythic cosmetic the account
//!   chooses and empties (R1306002).
//! - The routes, all `AuthLevel::Active`: `GET /api/cosmetic-packs` (the bar, the packs held and
//!   the slots' odds), `POST /api/cosmetic-packs/open` and `POST /api/cosmetic-packs/chase`
//!   (`{ cosmeticId, cardId? }`).
//!
//! An opening and a purchase run under the account's lock (R1301003), so neither rolls on, nor
//! spends, counters the other is moving.

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
    PACK_FLOORED_SLOT,
};
use crate::cosmetics::{Cosmetic, CosmeticRarity, cosmetic_pack_cosmetics};
use crate::economy::packs::{bar_before, delta_of, grant_counter, grant_unlocked, settle_bars};
use crate::economy::roll::{PackKind, PoolCard, pack_pool};
use crate::economy::store::{
    CosmeticGrant, Counter, EconomyReason, InventoryGrant, InventoryItem, OwnedCosmetics,
};
use crate::economy::tiers::{TierOdds, cosmetic_odds, cosmetic_tier};
use crate::economy::tokens::{BarState, choice_of, conflict, held};

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
    /// The holdings of an account with `owned` copies and `cosmetics` on its ledger. The starter
    /// and default cosmetics have no row, and no pack drops them (R1306000).
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
}

/// What one cosmetic pack slot grants (R1306000, R1306001).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CosmeticSlotGrant {
    /// An unowned cosmetic of `cosmetic_rarity`, which is the rarity rolled or the nearest one
    /// with something unowned. `card_id` is the card a per-card cosmetic goes on (`None` for an
    /// account cosmetic), and `card_granted` whether the account lacked that card, so the slot
    /// grants it too.
    Cosmetic {
        cosmetic_id: String,
        cosmetic_rarity: CosmeticRarity,
        card_id: Option<String>,
        card_granted: bool,
    },
    /// A spent slot: nothing at all was left to own, so it pays both token bars' points per
    /// duplicate (R1306001).
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
/// read but the holdings before it (the catalog version its pool came from, the cosmetics it could
/// drop and the seed) and its result (the slots and the draws taken). [`roll_cosmetic_pack`] over
/// that catalog's pool, the holdings before it and `cosmetics`, with `seed`, replays it however the
/// live catalog or registry has moved since.
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
    /// The log of `opening`, rolled from the pool of catalog `catalog`, over `registry`, with
    /// `seed`.
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

/// What opening a cosmetic pack answers: its slots in order, the Chase Shard bar after it and the
/// cosmetic packs still held.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenedCosmeticPack {
    pub slots: Vec<CosmeticSlot>,
    pub shard_bar: BarState,
    pub unopened: i64,
}

/// A cosmetic pack's odds as `GET /api/cosmetic-packs` shows them (R1339000): one roll of a slot
/// and of the floored slot on `COSMETIC_TIER_WEIGHTS`, the fallback included.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPackOdds {
    pub slot: TierOdds,
    pub floored_slot: TierOdds,
}

/// What `GET /api/cosmetic-packs` reads back (R1306002, R1339000): the Chase Shard bar, the
/// cosmetic packs unopened and the slots' odds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPacks {
    pub shard_bar: BarState,
    pub unopened: i64,
    pub odds: CosmeticPackOdds,
}

/// The pool cards a per-card cosmetic is not owned on: all of them, or a full art's own card
/// alone (R1313000). An account cosmetic has none.
fn placements<'p>(cosmetic: &Cosmetic, pool: &'p [PoolCard], holdings: &CosmeticHoldings) -> Vec<&'p str> {
    if !cosmetic.kind.is_per_card() {
        return Vec::new();
    }
    pool.iter()
        .filter(|card| cosmetic.card.as_deref().is_none_or(|names| names == card.id))
        .filter(|card| {
            !holdings
                .card_cosmetics
                .contains(&(card.id.clone(), cosmetic.id.clone()))
        })
        .map(|card| card.id.as_str())
        .collect()
}

/// Whether the account can still gain `cosmetic`: an account cosmetic it does not own, or a
/// per-card one with a card left to own it on.
fn is_open(cosmetic: &Cosmetic, pool: &[PoolCard], holdings: &CosmeticHoldings) -> bool {
    if cosmetic.kind.is_per_card() {
        !placements(cosmetic, pool, holdings).is_empty()
    } else {
        !holdings.account_cosmetics.contains(&cosmetic.id)
    }
}

/// A per-card cosmetic's card (R1306000): one of `placements`, each weighed by
/// `COSMETIC_PACK_CARD_WEIGHTS`, `.owned` when the account owns the card and `.unowned` when it
/// does not. One draw; no placement is `None` and draws nothing.
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
/// `rolled.fallback_order()` with a cosmetic of `droppable` still open, with every rarity at or
/// above `floor` ahead of those below it, supplies one, picked uniformly (one draw); a per-card
/// one then takes its card by [`pick_card`] (one draw), and an account one has none. A Legendary
/// or Mythic one is a chase, and drops like any other (R1333000). With nothing open in any rarity
/// the slot is spent: `Bars`, with no draw.
pub fn slot_grant(
    rolled: CosmeticRarity,
    floor: CosmeticRarity,
    pool: &[PoolCard],
    droppable: &[Cosmetic],
    holdings: &CosmeticHoldings,
    rng: &mut Rng,
) -> CosmeticSlotGrant {
    let mut order = rolled.fallback_order();
    // Stable, so each side keeps `fallback_order`'s order.
    order.sort_by_key(|rarity| *rarity < floor);
    for tier in order {
        let open: Vec<&Cosmetic> = droppable
            .iter()
            .filter(|cosmetic| cosmetic.rarity == tier)
            .filter(|cosmetic| is_open(cosmetic, pool, holdings))
            .collect();
        let Some(cosmetic) = rng.pick(&open) else {
            continue;
        };
        // An open per-card cosmetic has a placement, so the pick finds a card.
        let card_id =
            pick_card(&placements(cosmetic, pool, holdings), &holdings.owned, rng).map(str::to_string);
        let card_granted = card_id.as_deref().is_some_and(|card| !holdings.owns(card));
        return CosmeticSlotGrant::Cosmetic {
            cosmetic_id: cosmetic.id.clone(),
            cosmetic_rarity: tier,
            card_id,
            card_granted,
        };
    }
    CosmeticSlotGrant::Bars
}

/// The whole opening (R1306003, R1339002), pure from the pool, the account's holdings before it,
/// the cosmetics registry and the opening's seed. Slot by slot, its draws in this order and no
/// other: the slot's tier, then its cosmetic's pick and, for a per-card one, its card's, each only
/// when there is something to pick. Each slot's tier is a tiered roll on `COSMETIC_TIER_WEIGHTS`,
/// the one at `PACK_FLOORED_SLOT` floored at `COSMETIC_PACK_FLOOR_RARITY`, and falls no lower than
/// that floor while anything there or above is open (R1339000, R1306001). Ownership moves slot by
/// slot, so a cosmetic or a card granted earlier in the pack is owned later in it (R1306000).
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
            ..
        } = &grant
        {
            match card_id {
                Some(card_id) => {
                    holdings
                        .card_cosmetics
                        .insert((card_id.clone(), cosmetic_id.clone()));
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

/// §14.6: opens one unopened cosmetic pack, in one transaction. Under the account's lock
/// (R1301003) it reads the pack held, the collection, the cosmetics and the counters, rolls the
/// opening from `seed` ([`roll_cosmetic_pack`] over the shipped sets' pool) and writes every
/// grant: the pack spent, the new cards (the collection and its ledger, reason `pack`), the
/// cosmetics, the Chase Shard bar's one unless it is full (R1306002), and both token bars' points
/// for the spent slots, each bar that reached its size paying its tokens with the overflow kept
/// (R1305000, R1306001). Every economy row's `ref` is the opening's log; the collection's rows
/// have no `ref` and share the opening's `at`. No pack held is 409. Every refusal and failure
/// returns before the commit, so it writes nothing.
pub async fn open_cosmetic_pack(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    seed: &str,
) -> Result<OpenedCosmeticPack, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
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

    // The new cards, summed per card.
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
            cosmetic_id, card_id, ..
        } = &slot.grant
        {
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

    // The Chase Shard bar (R1306002): one per opening, and a full bar holds.
    let mut shard = bar_before(&counters, Counter::ChaseShardBar);
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

    let spent = delta_of(
        opening
            .slots
            .iter()
            .filter(|slot| slot.grant == CosmeticSlotGrant::Bars)
            .count(),
    )?;
    settle_bars(&mut tx, profile_id, &counters, spent, &reference, at).await?;

    tx.commit().await?;
    tracing::info!(
        event = "cosmetic_packs.opened",
        "profileId" = %profile_id,
        cards = unlocked.len(),
        spent,
    );
    Ok(OpenedCosmeticPack {
        slots: opening.slots,
        shard_bar: BarState {
            points: shard,
            size: CHASE_SHARD_PACKS,
        },
        unopened,
    })
}

/// §14.6: a full Chase Shard bar buys any chase cosmetic, a Legendary or Mythic one (R1333000),
/// the account does not own: a per-card one for a card the account owns (a full art names its own
/// card alone, R1313000) or an account one, in one transaction: the cosmetic's grant, then the
/// bar emptied (reason `chase_shard`), with the same `at`; one commit (R1306002). An unknown
/// cosmetic id is 400. Refuses with 409, writing nothing, when the bar is not full, the cosmetic
/// sits below Legendary, a per-card cosmetic names no owned card, an account cosmetic is given
/// one, or it is already owned (a ledger row, or a default or starter, which §14.2 gives every
/// account), in that order.
pub async fn buy_chase(
    app: &App,
    profile_id: &str,
    registry: &[Cosmetic],
    cosmetic_id: &str,
    card_id: Option<&str>,
) -> Result<(), ApiError> {
    let cosmetic = registry
        .iter()
        .find(|entry| entry.id == cosmetic_id)
        .ok_or_else(|| bad_request(format!("no cosmetic is named \"{cosmetic_id}\"")))?;
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    let bar = bar_before(
        &tx.economy_counters_get(profile_id).await?,
        Counter::ChaseShardBar,
    );
    let owned = owned_in(&mut tx, profile_id).await?;
    let cosmetics = tx.economy_cosmetics_get(profile_id).await?;
    let choice = choice_of(cosmetic, card_id, &owned, &cosmetics);
    if bar < CHASE_SHARD_PACKS {
        return Err(conflict("the Chase Shard bar is not full"));
    }
    if !cosmetic.rarity.is_chase() {
        return Err(conflict(
            "the Chase Shard bar buys only a Legendary or Mythic cosmetic",
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
    tx.commit().await?;
    tracing::info!(event = "cosmetic_packs.chase_bought", "profileId" = %profile_id, cosmetic = cosmetic_id);
    Ok(())
}

/// Reads [`CosmeticPacks`] for `get_cosmetic_packs`: the bar from the counters, the packs from the
/// inventory and the odds from `COSMETIC_TIER_WEIGHTS`. Read-only, like the collection.
async fn read_cosmetic_packs(app: &App, profile_id: &str) -> Result<CosmeticPacks, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let counters = tx.economy_counters_get(profile_id).await?;
    let inventory = tx.economy_inventory_get(profile_id).await?;
    tx.commit().await?;
    Ok(CosmeticPacks {
        shard_bar: BarState {
            points: bar_before(&counters, Counter::ChaseShardBar),
            size: CHASE_SHARD_PACKS,
        },
        unopened: held(&inventory, InventoryItem::CosmeticPack, ""),
        odds: CosmeticPackOdds {
            slot: cosmetic_odds(None),
            floored_slot: cosmetic_odds(Some(COSMETIC_PACK_FLOOR_RARITY)),
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
/// read foretells a later roll (§14.11).
pub async fn post_open(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    ok_of(&open_cosmetic_pack(app, &profile.id, &registry, &SystemIds.seed()).await?)
}

/// `POST /api/cosmetic-packs/chase` (`AuthLevel::Active`): `{ cosmeticId, cardId? }` over the
/// shipped registry; [`buy_chase`], answering the bar and packs after it.
pub async fn post_chase(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let cosmetic_id = str(&req.body, "cosmeticId")?;
    let card_id = optional_str(&req.body, "cardId")?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    buy_chase(app, &profile.id, &registry, &cosmetic_id, card_id.as_deref()).await?;
    ok_of(&read_cosmetic_packs(app, &profile.id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmetics::CosmeticKind;
    use crate::cosmetics::CosmeticRarity::{Common, Epic, Legendary, Mythic, Rare};
    use crate::economy::store::CardCosmetic;
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
            card: None,
        }
    }

    fn foil(id: &str, rarity: CosmeticRarity) -> Cosmetic {
        cosmetic(id, CosmeticKind::Foil, rarity)
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

    /// One droppable foil of each cosmetic rarity the registry has.
    fn four_foils() -> Vec<Cosmetic> {
        vec![
            foil("foil-common", CosmeticRarity::Common),
            foil("foil-rare", CosmeticRarity::Rare),
            foil("foil-epic", CosmeticRarity::Epic),
            foil("foil-legendary", CosmeticRarity::Legendary),
        ]
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

    /// Holdings owning `cosmetics` on `card_id`.
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

    /// The cosmetic a grant names.
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
        assert_eq!(ids, ["table-drop", "foil-drop", "full-art-drop"]);

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
        // Each cosmetic once.
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
        // No placement is no card and no draw.
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
        // At least a rarity's draw per slot, and no more than three.
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
}
