//! Buying and opening card packs (SPEC §14.1, §14.3; #1301), on the server, its one screen the
//! account page's opening (#1308): the store's grants around `economy::roll`'s pure roll, each in
//! one transaction.

use std::sync::Arc;

use jackioh_engine::wire::SHIPPED_SETS;
use jackioh_shared::economy::bars::BarsSettled;
use jackioh_shared::economy::ledger::LedgerGrant;
use jackioh_shared::economy::math::PityCounters;
use jackioh_shared::economy::open::{EconomyRefusal, PackInputs, PackRecords, RefusalReason, open_card_pack};
use jackioh_shared::economy::roll::{PackKind, Slot, SlotGrant, pack_pool};
use jackioh_shared::economy::tiers::{card_odds, cosmetic_odds};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::collection::{caller_profile, owned_in};
use crate::api::crypto::SystemIds;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, ok, str};
use crate::app::App;
use crate::config::{
    GLOBAL_COSMETIC_FLOOR_RARITY, GLOBAL_COSMETIC_ROLL_WEIGHT, PACK_FLOOR_RARITY, PACK_PRICE_JLORE,
    PACK_SLOTS, PITY_EPIC_PACKS, PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS, TIER_WEIGHT_TOTAL,
};
use crate::cosmetics::Cosmetic;
use crate::economy::grants::{Writing, counters_of, internal_refusal, write_grants};
use crate::economy::idempotency::{RequestKey, Spent, keep, replayed};
use crate::economy::store::{EconomyReason, InventoryEntry, InventoryGrant, JloreGrant, JloreReason};

/// What buying a pack answers: the pack, the wallet after it and the packs of its kind held.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoughtPack {
    pub pack: String,
    pub jlore: i64,
    pub unopened: i64,
}

/// What opening a pack answers: the pack, its slots in order, the pity counters after it, the
/// packs of its kind still held and what the opening did to both token bars.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenedPack {
    pub pack: String,
    pub slots: Vec<Slot>,
    pub pity: PityCounters,
    pub unopened: i64,
    pub bars: BarsSettled,
}

/// The unopened card packs the account holds (§14.3), for the pack screen's batch opening: one
/// `{ pack, count }` per kind it holds any of, the global pack first, then each shipped set's.
fn unopened_from(inventory: &[InventoryEntry]) -> Vec<Value> {
    std::iter::once(PackKind::Global)
        .chain(SHIPPED_SETS.into_iter().map(PackKind::Set))
        .filter_map(|pack| {
            let (item, key) = pack.inventory();
            let count = inventory
                .iter()
                .find(|entry| entry.item == item && entry.key == key)
                .map_or(0, |entry| entry.count);
            (count > 0).then(|| json!({ "pack": pack.name(), "count": count }))
        })
        .collect()
}

fn pack_of(body: &Value) -> Result<PackKind, ApiError> {
    let name = str(body, "pack")?;
    PackKind::parse(&name).ok_or_else(|| bad_request(format!("no card pack is named \"{name}\"")))
}

fn no_pack_held() -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, "no unopened pack of that kind")
}

/// §14.3: buys a pack for `PACK_PRICE_JLORE`, in one transaction; a wallet that cannot cover it is 409 and
/// writes nothing (§14.1). A keyed buy (R2124000) takes the account's lock first, so a retry racing its first
/// try waits for it (after its turn, R2378000), then answers a committed key as it did then and keeps a new
/// buy's answer in its transaction. A buy with no key takes no lock and keeps nothing.
pub async fn buy_pack_once(
    app: &App,
    profile_id: &str,
    pack: PackKind,
    request: Option<&RequestKey>,
) -> Result<Spent<BoughtPack>, ApiError> {
    let (item, key) = pack.inventory();
    let reference = Some(pack.name().to_string());
    let at = crate::app::now_ms();
    let mut tx = app.db.begin(Some(profile_id)).await?;
    if request.is_some() {
        tx.economy_lock_account(profile_id).await?;
    }
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
    let jlore = tx
        .economy_jlore_grant(&JloreGrant {
            profile_id: profile_id.to_string(),
            delta: -i64::from(PACK_PRICE_JLORE),
            reason: JloreReason::PackPurchase,
            reference: reference.clone(),
            dev: false,
            at,
        })
        .await?
        .ok_or_else(|| ApiError::new(ApiErrorCode::Conflict, "not enough Jlore for a card pack"))?;
    let unopened = tx
        .economy_inventory_grant(&InventoryGrant {
            profile_id: profile_id.to_string(),
            item,
            key,
            delta: 1,
            reason: EconomyReason::PackPurchase,
            reference,
            dev: false,
            at,
        })
        .await?
        .ok_or_else(|| ApiError::internal("the inventory refused a bought pack"))?;
    let bought = BoughtPack {
        pack: pack.name().to_string(),
        jlore,
        unopened,
    };
    keep(&mut tx, profile_id, request, &bought).await?;
    tx.commit().await?;
    tracing::info!(event = "packs.bought", "profileId" = %profile_id, pack = pack.name());
    Ok(Spent::Now(bought))
}

/// §14.3: opens one unopened pack of `pack`'s kind, in one transaction. Under the account's lock (R1301003)
/// it rolls the opening from `seed` (`jackioh_shared::economy::open::open_card_pack`) and writes every grant. Every economy row's `ref`
/// is the opening's log; the collection's rows have no `ref` and share the opening's `at`. No pack held is
/// 409, writing nothing. With no key, [`open_pack_once`]'s opening.
pub async fn open_pack(
    app: &App,
    profile_id: &str,
    pack: PackKind,
    registry: &[Cosmetic],
    seed: &str,
) -> Result<OpenedPack, ApiError> {
    open_pack_once(app, profile_id, pack, registry, seed, None)
        .await?
        .now()
}

/// [`open_pack`] under an `Idempotency-Key` (R2124000): right after the account's lock, before the
/// pack held is read (a retry finds the pack its first try spent gone), a key already committed
/// answers the opening it kept, and an opening's answer is kept in its own transaction.
pub async fn open_pack_once(
    app: &App,
    profile_id: &str,
    pack: PackKind,
    registry: &[Cosmetic],
    seed: &str,
    request: Option<&RequestKey>,
) -> Result<Spent<OpenedPack>, ApiError> {
    let (item, key) = pack.inventory();
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
    let held = tx
        .economy_inventory_get(profile_id)
        .await?
        .into_iter()
        .find(|entry| entry.item == item && entry.key == key)
        .map_or(0, |entry| entry.count);
    let records = PackRecords {
        held,
        owned: owned_in(&mut tx, profile_id).await?,
        cosmetics: tx.economy_cosmetics_get(profile_id).await?,
        counters: counters_of(&tx.economy_counters_get(profile_id).await?),
    };
    let inputs = PackInputs {
        pack,
        pool: pack_pool(&app.catalog.defs, pack),
        registry: registry.to_vec(),
        catalog: app.catalog.version.clone(),
    };
    let at = crate::app::now_ms();
    let opening = open_card_pack(&records, &inputs, seed, at).map_err(refused)?;
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
        .filter(|slot| matches!(slot.grant, SlotGrant::Bars { .. }))
        .count();
    let cards = opening
        .slots
        .iter()
        .filter(|slot| slot.grant == SlotGrant::Card)
        .map(|slot| slot.card_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let opened = OpenedPack {
        pack: opening.pack,
        slots: opening.slots,
        pity: opening.pity,
        unopened,
        bars: opening.bars,
    };
    keep(&mut tx, profile_id, request, &opened).await?;

    tx.commit().await?;
    tracing::info!(
        event = "packs.opened",
        "profileId" = %profile_id,
        pack = pack.name(),
        cards,
        spent,
    );
    Ok(Spent::Now(opened))
}

/// The error a decision's refusal answers: no pack held is a conflict, writing nothing; a pool with no card
/// is the catalog's fault.
fn refused(refusal: EconomyRefusal) -> ApiError {
    match refusal.reason {
        RefusalReason::NoPackHeld => ApiError::new(ApiErrorCode::Conflict, refusal.message),
        RefusalReason::EmptyPool => ApiError::internal(refusal.message),
    }
}

/// `GET /api/packs` (`AuthLevel::Active`): the caller's pity counters and the odds, for the pack
/// screen to show (§14.4), and the unopened packs it holds. The odds are one roll's on the tier
/// tables, the fallback included (R1339000), and a duplicate's global roll (R1661001) is its
/// `chance` and its `tiers`, the table floored at `GLOBAL_COSMETIC_FLOOR_RARITY`.
pub async fn get_packs(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let pity = counters_of(&tx.economy_counters_get(&profile.id).await?).pity;
    let inventory = tx.economy_inventory_get(&profile.id).await?;
    tx.commit().await?;
    Ok(ok(json!({
        "price": PACK_PRICE_JLORE,
        "slots": PACK_SLOTS,
        "pity": pity,
        "thresholds": {
            "epic": PITY_EPIC_PACKS,
            "legendary": PITY_LEGENDARY_PACKS,
            "mythic": PITY_MYTHIC_PACKS,
        },
        "odds": {
            "slot": card_odds(None),
            "flooredSlot": card_odds(Some(PACK_FLOOR_RARITY)),
            "duplicateCosmetic": cosmetic_odds(None),
            "globalCosmetic": {
                "chance": f64::from(GLOBAL_COSMETIC_ROLL_WEIGHT) / f64::from(TIER_WEIGHT_TOTAL),
                "tiers": cosmetic_odds(Some(GLOBAL_COSMETIC_FLOOR_RARITY)),
            },
        },
        "unopened": unopened_from(&inventory),
    })))
}

/// `POST /api/packs/buy` (`AuthLevel::Active`): `{ pack }`, with an optional `Idempotency-Key`
/// (R2124000); [`buy_pack_once`], in the account's spend turn (R2378000).
pub async fn post_buy(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let pack = pack_of(&req.body)?;
    let request = RequestKey::of(app, &req, "/api/packs/buy")?;
    let _turn = app.spends.claim(&profile.id).await?;
    buy_pack_once(app, &profile.id, pack, request.as_ref())
        .await?
        .answer()
}

/// `POST /api/packs/open` (`AuthLevel::Active`): `{ pack }`; [`open_pack`] over the shipped
/// registry, from a fresh seed (`SystemIds::seed`, 16 random bytes), so no seed the account can
/// read foretells a later roll (§14.11), with an optional `Idempotency-Key` (R2124000);
/// [`open_pack_once`], in the account's spend turn (R2378000).
pub async fn post_open(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let pack = pack_of(&req.body)?;
    let request = RequestKey::of(app, &req, "/api/packs/open")?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    let _turn = app.spends.claim(&profile.id).await?;
    open_pack_once(
        app,
        &profile.id,
        pack,
        &registry,
        &SystemIds.seed(),
        request.as_ref(),
    )
    .await?
    .answer()
}
