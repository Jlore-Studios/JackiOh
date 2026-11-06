//! Buying and opening card packs (SPEC §14.1, §14.3; #1301), on the server, its one screen the
//! account page's opening (#1308): the store's grants around `economy::roll`'s pure roll, each in
//! one transaction.

use std::sync::Arc;

use jackioh_engine::wire::SHIPPED_SETS;
use jackioh_shared::economy::bars::BarsSettled;
use jackioh_shared::economy::ledger::{Counter, LedgerGrant};
use jackioh_shared::economy::math::PityCounters;
use jackioh_shared::economy::open::{
    EconomyRefusal, PackInputs, PackOpening, PackRecords, RefusalReason, open_card_pack,
};
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
    PACK_SLOTS, PACKS_OPEN_ALL_MAX, PITY_EPIC_PACKS, PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS,
    TIER_WEIGHT_TOTAL,
};
use crate::cosmetics::Cosmetic;
use crate::db::store::Tx;
use crate::economy::cosmetic_packs::level_skip_chance;
use crate::economy::grants::{Writing, counters_of, internal_refusal, write_grants, write_grants_all};
use crate::economy::idempotency::{RequestKey, Spent, keep, replayed};
use crate::economy::store::{
    CardCosmetic, EconomyReason, InventoryEntry, InventoryGrant, JloreGrant, JloreReason,
};

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

/// What Open all answers (R3212000): every pack's opening in order, each as one open answers it, so
/// the last one's pity, packs held and bars are the account's after them all.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenedPacks {
    pub openings: Vec<OpenedPack>,
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

/// R3212000: how many packs an Open all opens: a whole number from 1 to `PACKS_OPEN_ALL_MAX`.
fn count_of(body: &Value) -> Result<usize, ApiError> {
    body.get("count")
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .filter(|count| (1..=PACKS_OPEN_ALL_MAX).contains(count))
        .ok_or_else(|| {
            bad_request(format!(
                "\"count\" must be a whole number from 1 to {PACKS_OPEN_ALL_MAX}"
            ))
        })
}

fn no_pack_held() -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, "no unopened pack of that kind")
}

/// The error a pack's grant the store could not take answers: its pack spent from too few held is
/// a conflict; any other is the server's fault.
fn grant_refused(grant: &LedgerGrant) -> ApiError {
    match grant {
        LedgerGrant::Inventory { delta, .. } if *delta < 0 => no_pack_held(),
        other => internal_refusal(other),
    }
}

/// What an opening reads, under the account's lock: the packs of `pack`'s kind held, the copies of
/// each card, the cosmetics and the counters.
async fn records_of(tx: &mut Tx<'_>, profile_id: &str, pack: PackKind) -> Result<PackRecords, ApiError> {
    let (item, key) = pack.inventory();
    let held = tx
        .economy_inventory_get(profile_id)
        .await?
        .into_iter()
        .find(|entry| entry.item == item && entry.key == key)
        .map_or(0, |entry| entry.count);
    Ok(PackRecords {
        held,
        owned: owned_in(tx, profile_id).await?,
        cosmetics: tx.economy_cosmetics_get(profile_id).await?,
        counters: counters_of(&tx.economy_counters_get(profile_id).await?),
    })
}

/// What `pack`'s roll draws from beyond the seed: its pool in this catalog, the registry and the
/// catalog's version.
pub fn inputs_of(app: &App, pack: PackKind, registry: &[Cosmetic]) -> PackInputs {
    PackInputs {
        pack,
        pool: pack_pool(&app.catalog.defs, pack),
        registry: registry.to_vec(),
        catalog: app.catalog.version.clone(),
    }
}

/// An opening's answer, with the packs of its kind it left.
fn opened_of(opening: PackOpening, unopened: i64) -> OpenedPack {
    OpenedPack {
        pack: opening.pack,
        slots: opening.slots,
        pity: opening.pity,
        unopened,
        bars: opening.bars,
    }
}

/// What an opening's log line counts: the distinct cards it unlocked, and the slots that spent a
/// duplicate on the bars.
fn opened_counts(opened: &OpenedPack) -> (usize, usize) {
    let cards = opened
        .slots
        .iter()
        .filter(|slot| slot.grant == SlotGrant::Card)
        .map(|slot| slot.card_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let spent = opened
        .slots
        .iter()
        .filter(|slot| matches!(slot.grant, SlotGrant::Bars { .. }))
        .count();
    (cards, spent)
}

/// R3212000: `records` as `opening` left them, as the store would read them back once its grants
/// are written: the packs held, each card's copies, the cosmetics owned and the counters. The roll
/// only looks these up, so where a new entry falls in its list does not matter.
fn records_after(records: &mut PackRecords, pack: PackKind, opening: &PackOpening) {
    let (pack_item, pack_key) = pack.inventory();
    for grant in &opening.grants {
        match grant {
            LedgerGrant::Inventory { item, key, delta, .. } if *item == pack_item && *key == pack_key => {
                records.held += delta;
            }
            LedgerGrant::Cards { cards } => {
                for card in cards {
                    records.owned.insert(card.card_id.clone(), card.quantity);
                }
            }
            LedgerGrant::Cosmetic { card_id, cosmetic_id } => match card_id {
                Some(card_id) => records.cosmetics.cards.push(CardCosmetic {
                    card_id: card_id.clone(),
                    cosmetic_id: cosmetic_id.clone(),
                }),
                None => records.cosmetics.account.push(cosmetic_id.clone()),
            },
            LedgerGrant::Counter { counter, delta, .. } => {
                let counters = &mut records.counters;
                match counter {
                    // The pity counters are the opening's own, set below.
                    Counter::PityEpic | Counter::PityLegendary | Counter::PityMythic => {}
                    Counter::CardTokenBar => counters.card_bar += delta,
                    Counter::CosmeticTokenBar => counters.cosmetic_bar += delta,
                    Counter::CosmeticTokenCycle => counters.cosmetic_cycle += delta,
                    Counter::ChaseShardBar => counters.chase_shard += delta,
                }
            }
            // A card pack's opening writes nothing else; a token the bars pay is no pack.
            _ => {}
        }
    }
    records.counters.pity = opening.pity;
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
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
    let inputs = inputs_of(app, pack, registry);
    let opened = open_pack_in(&mut tx, profile_id, &inputs, seed, crate::app::now_ms()).await?;
    keep(&mut tx, profile_id, request, &opened).await?;

    tx.commit().await?;
    let (cards, spent) = opened_counts(&opened);
    tracing::info!(
        event = "packs.opened",
        "profileId" = %profile_id,
        pack = pack.name(),
        cards,
        spent,
    );
    Ok(Spent::Now(opened))
}

/// One opening in the caller's transaction, which holds the account's lock (R1301003): reads the
/// records, rolls the opening from `seed` (`jackioh_shared::economy::open::open_card_pack`) and
/// writes every grant, each economy row's `ref` the opening's log and every row stamped `at`. No
/// pack held is 409, writing nothing.
pub async fn open_pack_in(
    tx: &mut Tx<'_>,
    profile_id: &str,
    inputs: &PackInputs,
    seed: &str,
    at: i64,
) -> Result<OpenedPack, ApiError> {
    let records = records_of(tx, profile_id, inputs.pack).await?;
    let opening = open_card_pack(&records, inputs, seed, at).map_err(refused)?;
    let reference = Some(serde_json::to_string(&opening.log)?);
    let unopened = write_grants(
        tx,
        &Writing {
            profile_id,
            reference: &reference,
            dev: false,
            at: opening.at,
        },
        &opening.grants,
        grant_refused,
    )
    .await?
    .ok_or_else(|| ApiError::internal("an opening spends its pack first"))?;
    Ok(opened_of(opening, unopened))
}

/// R3212000: one opening per seed, in order, in the caller's transaction, which holds the account's
/// lock: the records are read once and each pack rolls on what the one before it left
/// ([`records_after`]), so the openings are the ones [`open_pack_in`] would roll from the same seeds
/// one after another. Every grant is then written at once ([`write_grants_all`]), in a fixed number
/// of statements whatever the count. Fewer packs held than seeds is 409, writing nothing.
pub async fn open_packs_in(
    tx: &mut Tx<'_>,
    profile_id: &str,
    inputs: &PackInputs,
    seeds: &[String],
    at: i64,
) -> Result<Vec<OpenedPack>, ApiError> {
    let mut records = records_of(tx, profile_id, inputs.pack).await?;
    let mut written = Vec::with_capacity(seeds.len());
    let mut opened = Vec::with_capacity(seeds.len());
    for seed in seeds {
        let opening = open_card_pack(&records, inputs, seed, at).map_err(refused)?;
        records_after(&mut records, inputs.pack, &opening);
        written.push((
            Some(serde_json::to_string(&opening.log)?),
            opening.grants.clone(),
            opening.at,
        ));
        opened.push(opened_of(opening, records.held));
    }
    let outcomes: Vec<(Writing<'_>, &[LedgerGrant])> = written
        .iter()
        .map(|(reference, grants, at)| {
            (
                Writing {
                    profile_id,
                    reference,
                    dev: false,
                    at: *at,
                },
                grants.as_slice(),
            )
        })
        .collect();
    write_grants_all(tx, &outcomes, grant_refused).await?;
    Ok(opened)
}

/// R3212000: Open all, one pack of `pack`'s kind per seed in one spend: under the account's lock and
/// an `Idempotency-Key` as [`open_pack_once`], then [`open_packs_in`] at one stamp, its answer kept
/// in its own transaction.
pub async fn open_packs_once(
    app: &App,
    profile_id: &str,
    pack: PackKind,
    registry: &[Cosmetic],
    seeds: &[String],
    request: Option<&RequestKey>,
) -> Result<Spent<OpenedPacks>, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
    let inputs = inputs_of(app, pack, registry);
    let openings = open_packs_in(&mut tx, profile_id, &inputs, seeds, crate::app::now_ms()).await?;
    let opened = OpenedPacks { openings };
    keep(&mut tx, profile_id, request, &opened).await?;
    tx.commit().await?;
    let (cards, spent) = opened
        .openings
        .iter()
        .map(opened_counts)
        .fold((0, 0), |(cards, spent), (more, also)| {
            (cards + more, spent + also)
        });
    tracing::info!(
        event = "packs.opened_all",
        "profileId" = %profile_id,
        pack = pack.name(),
        count = opened.openings.len(),
        cards,
        spent,
    );
    Ok(Spent::Now(opened))
}

/// [`open_packs_once`] with no key.
pub async fn open_packs(
    app: &App,
    profile_id: &str,
    pack: PackKind,
    registry: &[Cosmetic],
    seeds: &[String],
) -> Result<OpenedPacks, ApiError> {
    open_packs_once(app, profile_id, pack, registry, seeds, None)
        .await?
        .now()
}

/// The error a decision's refusal answers: no pack held is a conflict, writing nothing; a pool with no card
/// is the catalog's fault.
fn refused(refusal: EconomyRefusal) -> ApiError {
    match refusal.reason {
        RefusalReason::NoPackHeld => ApiError::new(ApiErrorCode::Conflict, refusal.message),
        // The quest refusals are other decisions'; an opening never meets them.
        _ => ApiError::internal(refusal.message),
    }
}

/// `GET /api/packs` (`AuthLevel::Active`): the caller's pity counters and the odds, for the pack
/// screen to show (§14.4), and the unopened packs it holds. The odds are one roll's on the tier
/// tables, the fallback included (R1339000), and a duplicate's global roll (R1661001) is its
/// `chance` and its `tiers`, the table floored at `GLOBAL_COSMETIC_FLOOR_RARITY`. R3015003: `levelSkip` is the
/// chance a duplicate that finds a level past the next one open drops it with every level below it.
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
            "levelSkip": level_skip_chance(),
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

/// `POST /api/packs/open-all` (`AuthLevel::Active`): `{ pack, count }`, `count` packs of one kind
/// (1 to `PACKS_OPEN_ALL_MAX`) in one spend (R3212000): [`open_packs_once`] over the shipped
/// registry, each pack from a fresh seed of its own as `POST /api/packs/open`'s, with an optional
/// `Idempotency-Key` of its own route (R2124000), in the account's spend turn (R2378000). The count
/// only bounds the batch: every roll is the server's. 409 while the economy is off, and when fewer
/// are held than asked, writing nothing.
pub async fn post_open_all(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    if !app.economy_live {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "Packs can't be opened all at once yet.",
        ));
    }
    let pack = pack_of(&req.body)?;
    let count = count_of(&req.body)?;
    let request = RequestKey::of(app, &req, "/api/packs/open-all")?;
    let registry = crate::cosmetics::shipped()
        .map_err(|error| ApiError::internal(format!("cosmetics.json: {error}")))?;
    let _turn = app.spends.claim(&profile.id).await?;
    let seeds: Vec<String> = (0..count).map(|_| SystemIds.seed()).collect();
    open_packs_once(app, &profile.id, pack, &registry, &seeds, request.as_ref())
        .await?
        .answer()
}
