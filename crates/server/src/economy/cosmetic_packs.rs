//! Cosmetic packs and the Chase Shard bar (SPEC §14.2, §14.6; #1306): opening the pack the
//! cosmetic token cycle's last step pays, the bar each opening fills and the chase it buys.
//!
//! - The roll and the decision around it are `jackioh_shared::economy` (`cosmetic_pack::roll_cosmetic_pack`,
//!   `open::open_cosmetic_pack`, #3104): the pure roll from the shipped sets' pool, the account's holdings and
//!   a seed to what each slot grants (R1306000, R1306001), and every write an opening makes. This module
//!   reads the records, calls the decision and writes its grants.
//! - [`open_cosmetic_pack`] and [`buy_chase`], either under an optional `Idempotency-Key`
//!   ([`open_cosmetic_pack_once`], [`buy_chase_once`]), under which a retry answers as its first try
//!   did and spends nothing again (R2124000).
//! - The routes, all `AuthLevel::Active`, each spend in the account's turn (`economy::spend_slots`,
//!   R2378000).
//!
//! An opening and a purchase run under the account's lock (R1301003), so neither rolls on, nor spends,
//! counters the other is moving.

use std::sync::Arc;

use jackioh_shared::economy::bars::{BarState, BarsSettled};
use jackioh_shared::economy::cosmetic_pack::{CosmeticSlot, CosmeticSlotGrant};
use jackioh_shared::economy::ledger::{Counter, InventoryItem, LedgerGrant};
use jackioh_shared::economy::open::{
    CosmeticPackInputs, PackRecords, RefusalReason, open_cosmetic_pack as decide,
};
use jackioh_shared::economy::roll::{PackKind, pack_pool};
use jackioh_shared::economy::tiers::{TierOdds, cosmetic_odds};
use serde::{Deserialize, Serialize};

use crate::api::collection::{caller_profile, owned_in};
use crate::api::crypto::SystemIds;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, ok_of, optional_str, str};
use crate::app::App;
use crate::config::{CHASE_SHARD_PACKS, COSMETIC_PACK_FLOOR_RARITY};
use crate::cosmetics::Cosmetic;
use crate::db::store::Tx;
use crate::economy::grants::{Writing, counters_of, grant_counter, internal_refusal, write_grants};
use crate::economy::idempotency::{RequestKey, Spent, keep, replayed};
use crate::economy::store::{CosmeticGrant, EconomyReason};
use crate::economy::tokens::{choice_of, conflict, held, unsold};

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
/// and of the floored slot on `COSMETIC_TIER_WEIGHTS`, the fallback included.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPackOdds {
    pub slot: TierOdds,
    pub floored_slot: TierOdds,
}

/// What `GET /api/cosmetic-packs` reads back (R1306002, R1339000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPacks {
    pub shard_bar: BarState,
    pub unopened: i64,
    pub odds: CosmeticPackOdds,
}

fn no_pack_held() -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, "no unopened cosmetic pack")
}

/// §14.6: opens one unopened cosmetic pack, in one transaction. Under the account's lock (R1301003) it
/// rolls the opening from `seed` (`jackioh_shared`'s `open_cosmetic_pack` over the shipped sets' pool) and
/// writes every grant: the pack spent, the new cards (the collection and its ledger, reason `pack`), the cosmetics, the
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
    let packs = held(
        &tx.economy_inventory_get(profile_id).await?,
        InventoryItem::CosmeticPack,
        "",
    );
    let records = PackRecords {
        held: packs,
        owned: owned_in(&mut tx, profile_id).await?,
        cosmetics: tx.economy_cosmetics_get(profile_id).await?,
        counters: counters_of(&tx.economy_counters_get(profile_id).await?),
    };
    let inputs = CosmeticPackInputs {
        pool: pack_pool(&app.catalog.defs, PackKind::Global),
        registry: registry.to_vec(),
        catalog: app.catalog.version.clone(),
    };
    let at = crate::app::now_ms();
    let opening = decide(&records, &inputs, seed, at).map_err(|refusal| match refusal.reason {
        RefusalReason::NoPackHeld => no_pack_held(),
        RefusalReason::EmptyPool => ApiError::internal(refusal.message),
    })?;
    let reference = Some(serde_json::to_string(&opening.log)?);

    let unopened = write_grants(
        &mut tx,
        &Writing {
            profile_id,
            reference: &reference,
            dev: false,
            at: opening.at,
        },
        &opening.grants,
        |grant| match grant {
            LedgerGrant::Inventory { delta, .. } if *delta < 0 => no_pack_held(),
            other => internal_refusal(other),
        },
    )
    .await?
    .ok_or_else(|| ApiError::internal("an opening spends its pack first"))?;

    let spent = opening
        .slots
        .iter()
        .filter(|slot| slot.grant == CosmeticSlotGrant::Bars)
        .count();
    let cards = opening
        .grants
        .iter()
        .find_map(|grant| match grant {
            LedgerGrant::Cards { cards } => Some(cards.len()),
            _ => None,
        })
        .unwrap_or(0);
    let opened = OpenedCosmeticPack {
        slots: opening.slots,
        shard_before: opening.shard_before,
        shard_bar: opening.shard_bar,
        unopened,
        bars: opening.bars,
    };
    keep(&mut tx, profile_id, request, &opened).await?;

    tx.commit().await?;
    tracing::info!(
        event = "cosmetic_packs.opened",
        "profileId" = %profile_id,
        cards,
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
    let bar = counters_of(&tx.economy_counters_get(profile_id).await?).chase_shard;
    let owned = owned_in(&mut tx, profile_id).await?;
    let cosmetics = tx.economy_cosmetics_get(profile_id).await?;
    let choice = choice_of(cosmetic, card_id, &owned, &cosmetics);
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
    let counters = counters_of(&tx.economy_counters_get(profile_id).await?);
    let inventory = tx.economy_inventory_get(profile_id).await?;
    Ok(CosmeticPacks {
        shard_bar: BarState {
            points: counters.chase_shard,
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
