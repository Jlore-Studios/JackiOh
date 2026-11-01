//! Buying and opening card packs (SPEC §14.1, §14.3; #1301), on the server, its one screen the
//! account page's opening (#1308): the store's grants around `economy::roll`'s pure roll, each in
//! one transaction.
//!
//! - [`buy_pack_once`]: `PACK_PRICE_JLORE` out of the wallet and an unopened pack into the
//!   inventory; a wallet that cannot cover it is refused and changes nothing (§14.1).
//! - [`open_pack`]: under the account's lock, one unopened pack spent, its slots rolled from a
//!   fresh seed and every grant written, its cards, cosmetics, pity counters and bars, a full bar
//!   paying its tokens with the overflow kept (`economy::tokens::settle_bar`, R1305000, R1305001;
//!   the cosmetic bar step by step at each step's size, `settle_cosmetic_bar`, R2728000),
//!   a duplicate's account cosmetic of the global roll owned for the account (R1661001), with the
//!   opening's log (its catalog, cosmetics, seed and result) in the `ref` of every economy row it
//!   writes (R1301003). It answers what each bar did, its points before and after and what it
//!   paid ([`BarsSettled`]). The whole opening or nothing: every refusal returns
//!   before the commit, and the dropped `Tx` rolls it all back.
//! - The routes, all `AuthLevel::Active`: `GET /api/packs` (the pity counters, the odds and the
//!   unopened packs held), `POST /api/packs/buy` and `POST /api/packs/open`, each with `{ pack }`, a shipped set's name
//!   or `GLOBAL_PACK_NAME`, and an optional `Idempotency-Key`, under which a retry answers as its
//!   first try did and spends nothing again ([`buy_pack_once`], [`open_pack_once`], R2124000). Each
//!   spend runs in the account's turn, the account's other spends waiting theirs, and one sent past
//!   `ECONOMY_SPENDS_WAITING_MAX` is 429 `rate_limited` (`economy::spend_slots`, R2378000).

use std::sync::Arc;

use indexmap::IndexMap;
use jackioh_engine::wire::SHIPPED_SETS;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::collection::{caller_profile, owned_in};
use crate::api::crypto::SystemIds;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, ok, str};
use crate::app::App;
use crate::config::{
    CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_CYCLE, CosmeticTokenStep, GLOBAL_COSMETIC_FLOOR_RARITY,
    GLOBAL_COSMETIC_ROLL_WEIGHT, PACK_FLOOR_RARITY, PACK_PRICE_JLORE, PACK_SLOTS, PITY_EPIC_PACKS,
    PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS, TIER_WEIGHT_TOTAL,
};
use crate::cosmetics::{Cosmetic, CosmeticRarity};
use crate::db::store::{CollectionEntry, CollectionGrant, Tx};
use crate::economy::idempotency::{RequestKey, Spent, keep, replayed};
use crate::economy::math::PityCounters;
use crate::economy::roll::{Holdings, OpeningLog, PackKind, Slot, SlotGrant, pack_pool, roll_opening};
use crate::economy::store::{
    CosmeticGrant, Counter, CounterEntry, CounterGrant, EconomyReason, InventoryEntry, InventoryGrant,
    InventoryItem, JloreGrant, JloreReason,
};
use crate::economy::tiers::{card_odds, cosmetic_odds};
use crate::economy::tokens::{card_bar_points, settle_bar, settle_cosmetic_bar};

/// The collection ledger's reason for a card a pack unlocks: migration 0002's `pack`, one of its
/// closed set, as `api::collection::LAUNCH_GRANT_REASON` is its `launch`.
pub const PACK_GRANT_REASON: &str = "pack";

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

/// One token bar across an opening (R1305000): its points `before` the opening and `after` it (the
/// overflow a pay leaves kept), its `size` and how many times it `pays` in this opening. Every
/// opening moves both bars, since every slot adds points (R1999000). The cosmetic bar's `size` is
/// the size of the step it stands at after the opening (R2728000).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BarSettle {
    pub before: i64,
    pub after: i64,
    pub size: i64,
    pub pays: i64,
}

/// What an opening did to both token bars (R1305000, R1305001), for the client to animate from
/// where each stood to where the opening left it: the cosmetic bar's `cosmetic_paid` are the
/// cycle's steps it paid, in order, so its length is `cosmetic.pays`, `cosmetic_sizes` the size of
/// each step its sweep crosses (R2728000), and `slot_points` are the points each slot added to
/// each bar, in slot order, every slot's its rolled rarity's, so the client can send each slot's
/// points to the bars without counting them (R1953000, R1999000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BarsSettled {
    pub card: BarSettle,
    pub cosmetic: BarSettle,
    pub cosmetic_paid: Vec<CosmeticTokenStep>,
    /// R2728000: the size of each step the cosmetic bar's sweep crosses: each step paid, in order,
    /// then the step it stands at after (`cosmetic.size`), so `cosmetic.pays + 1` entries.
    pub cosmetic_sizes: Vec<i64>,
    pub slot_points: Vec<i64>,
}

/// The inventory row a pack is counted in (§14.3): a set's under its set's name, the global
/// pack's under no key.
fn inventory_of(pack: PackKind) -> (InventoryItem, String) {
    match pack {
        PackKind::Set(set) => (InventoryItem::SetPack, set.as_str().to_string()),
        PackKind::Global => (InventoryItem::GlobalPack, String::new()),
    }
}

/// The account's pity counters (R1298000: one set, under no pack key); a counter never written
/// is 0.
fn pity_from(counters: &[CounterEntry]) -> PityCounters {
    let value = |counter: Counter| {
        counters
            .iter()
            .find(|entry| entry.counter == counter && entry.pack.is_empty())
            .map_or(0, |entry| usize::try_from(entry.value).unwrap_or_default())
    };
    PityCounters {
        epic: value(Counter::PityEpic),
        legendary: value(Counter::PityLegendary),
        mythic: value(Counter::PityMythic),
    }
}

/// The unopened card packs the account holds (§14.3), for the pack screen's batch opening: one
/// `{ pack, count }` per kind it holds any of, the global pack first, then each shipped set's.
fn unopened_from(inventory: &[InventoryEntry]) -> Vec<Value> {
    std::iter::once(PackKind::Global)
        .chain(SHIPPED_SETS.into_iter().map(PackKind::Set))
        .filter_map(|pack| {
            let (item, key) = inventory_of(pack);
            let count = inventory
                .iter()
                .find(|entry| entry.item == item && entry.key == key)
                .map_or(0, |entry| entry.count);
            (count > 0).then(|| json!({ "pack": pack.name(), "count": count }))
        })
        .collect()
}

/// The pack a request body names (`{ pack }`).
fn pack_of(body: &Value) -> Result<PackKind, ApiError> {
    let name = str(body, "pack")?;
    PackKind::parse(&name).ok_or_else(|| bad_request(format!("no card pack is named \"{name}\"")))
}

fn no_pack_held() -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, "no unopened pack of that kind")
}

/// A count as a ledger delta.
pub(crate) fn delta_of(count: usize) -> Result<i64, ApiError> {
    i64::try_from(count).map_err(|_| ApiError::internal(format!("{count} does not fit a ledger delta")))
}

/// §14.3: buys a pack for `PACK_PRICE_JLORE`, in one transaction: the wallet's debit, then the
/// pack, unopened, into the inventory, each with its ledger row. A wallet that cannot cover the
/// price is 409 and writes nothing (§14.1). Under an `Idempotency-Key` (R2124000): a keyed buy
/// takes the account's lock first, so a retry racing its first try waits for it (through the route
/// it waits its turn first, R2378000), then answers a key it already committed as it did then, and
/// keeps the answer of one it buys in the same transaction. A buy with no key takes no lock and
/// keeps nothing, as before keys.
pub async fn buy_pack_once(
    app: &App,
    profile_id: &str,
    pack: PackKind,
    request: Option<&RequestKey>,
) -> Result<Spent<BoughtPack>, ApiError> {
    let (item, key) = inventory_of(pack);
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

/// Adds `delta` to one of the account's counters for an opening; a zero delta writes nothing,
/// since a ledger row records a change.
pub(crate) async fn grant_counter(
    tx: &mut Tx<'_>,
    profile_id: &str,
    counter: Counter,
    delta: i64,
    reason: EconomyReason,
    reference: &Option<String>,
    at: i64,
) -> Result<(), ApiError> {
    if delta == 0 {
        return Ok(());
    }
    tx.economy_counters_grant(&CounterGrant {
        profile_id: profile_id.to_string(),
        counter,
        pack: String::new(),
        delta,
        reason,
        reference: reference.clone(),
        dev: false,
        at,
    })
    .await?
    .ok_or_else(|| ApiError::internal(format!("the counter {counter:?} refused {delta}")))?;
    Ok(())
}

/// One account-wide counter's value before the opening: a counter never written is 0.
pub(crate) fn bar_before(counters: &[CounterEntry], counter: Counter) -> i64 {
    counters
        .iter()
        .find(|entry| entry.counter == counter && entry.pack.is_empty())
        .map_or(0, |entry| entry.value)
}

/// The step the cosmetic token cycle stands at after `position` steps paid (§14.5).
pub(crate) fn cycle_at(position: i64) -> CosmeticTokenStep {
    COSMETIC_TOKEN_CYCLE[(position % COSMETIC_TOKEN_CYCLE.len() as i64) as usize]
}

/// What the cosmetic token cycle pays at `position`, its steps paid so far (§14.5): a Rare, an Epic
/// or a Legendary cosmetic token (R2728000), or a cosmetic pack. A pack opening's bar and a dev
/// account's fill (`api::dev::fill_bar`, R1310003) both pay through it.
pub(crate) fn cycle_step(position: i64) -> (InventoryItem, String) {
    match cycle_at(position) {
        CosmeticTokenStep::Rare => (
            InventoryItem::CosmeticToken,
            CosmeticRarity::Rare.as_str().to_string(),
        ),
        CosmeticTokenStep::Epic => (
            InventoryItem::CosmeticToken,
            CosmeticRarity::Epic.as_str().to_string(),
        ),
        CosmeticTokenStep::Legendary => (
            InventoryItem::CosmeticToken,
            CosmeticRarity::Legendary.as_str().to_string(),
        ),
        CosmeticTokenStep::CosmeticPack => (InventoryItem::CosmeticPack, String::new()),
    }
}

/// Writes the cards an opening unlocked, `unlocked` copies per card over the copies `owned` before
/// it: the collection's absolute quantities, then its ledger's deltas (reason [`PACK_GRANT_REASON`]),
/// as `api::collection::grant_cards` writes them. Nothing unlocked writes nothing. A card pack's
/// opening and a cosmetic pack's both write through it (R1306000).
pub(crate) async fn grant_unlocked(
    tx: &mut Tx<'_>,
    profile_id: &str,
    owned: &IndexMap<String, i64>,
    unlocked: &IndexMap<String, i64>,
    at: i64,
) -> Result<(), ApiError> {
    if unlocked.is_empty() {
        return Ok(());
    }
    let quantities: Vec<CollectionEntry> = unlocked
        .iter()
        .map(|(card_id, delta)| CollectionEntry {
            card_id: card_id.clone(),
            quantity: owned.get(card_id).copied().unwrap_or(0) + delta,
        })
        .collect();
    let grants: Vec<CollectionGrant> = unlocked
        .iter()
        .map(|(card_id, delta)| CollectionGrant {
            profile_id: profile_id.to_string(),
            card_id: card_id.clone(),
            delta: *delta,
            reason: PACK_GRANT_REASON.to_string(),
            dev: false,
            at,
        })
        .collect();
    tx.collection_upsert_quantities(profile_id, &quantities).await?;
    tx.collection_append_grants(&grants).await?;
    Ok(())
}

/// The points each of a card pack's slots adds to each token bar, in slot order: its slot's card
/// rarity's ([`card_bar_points`]), whatever it grants (R1953000, R1999000).
pub fn slot_bar_points(slots: &[Slot]) -> Vec<i64> {
    slots.iter().map(|slot| card_bar_points(slot.rarity)).collect()
}

/// Both token bars' points for an opening's slots, `slot_points` each slot's to each bar
/// ([`slot_bar_points`], R1953000), each bar paying when it reaches its size
/// (R1305000, R1305001): the points accrue, a bar that reaches its size pays its tokens and keeps
/// the overflow, and the cosmetic bar's payouts are the cycle's next steps, each at its own size
/// (`settle_cosmetic_bar`, R2728000), whose stored position then advances. Each bar's write
/// carries `reference`, the opening's log, and every row shares `at`; `counters` is the account's
/// before the opening. It answers what each bar did, for the
/// opening's answer. A card pack's opening and a cosmetic pack's both settle through it (R1306001).
pub(crate) async fn settle_bars(
    tx: &mut Tx<'_>,
    profile_id: &str,
    counters: &[CounterEntry],
    slot_points: Vec<i64>,
    reference: &Option<String>,
    at: i64,
) -> Result<BarsSettled, ApiError> {
    // Every slot's points go to both bars (R1953000, R1999000).
    let gained: i64 = slot_points.iter().sum();
    let card_bar_before = bar_before(counters, Counter::CardTokenBar);
    let cosmetic_bar_before = bar_before(counters, Counter::CosmeticTokenBar);
    let cycle_before = bar_before(counters, Counter::CosmeticTokenCycle);

    // The card token bar (R1305000): the slots' points accrue, and a bar that reaches its size
    // pays card tokens and keeps the overflow, in this transaction. The paying write replaces the
    // accrual row, so still one row per bar per opening.
    let (card_pays, card_remainder) = settle_bar(card_bar_before, gained, CARD_TOKEN_BAR_SIZE);
    if card_pays > 0 {
        grant_counter(
            tx,
            profile_id,
            Counter::CardTokenBar,
            card_remainder - card_bar_before,
            EconomyReason::TokenBar,
            reference,
            at,
        )
        .await?;
        tx.economy_inventory_grant(&InventoryGrant {
            profile_id: profile_id.to_string(),
            item: InventoryItem::CardToken,
            key: String::new(),
            delta: card_pays,
            reason: EconomyReason::TokenBar,
            reference: reference.clone(),
            dev: false,
            at,
        })
        .await?
        .ok_or_else(|| ApiError::internal("the inventory refused the card bar's tokens"))?;
    } else {
        grant_counter(
            tx,
            profile_id,
            Counter::CardTokenBar,
            gained,
            EconomyReason::PackOpen,
            reference,
            at,
        )
        .await?;
    }

    // The cosmetic token bar (R1305000, R1305001): as above, and each payout is the cycle's next
    // step, paid at its own size (R2728000), whose stored position then advances by the payouts.
    let (cosmetic_paid, cosmetic_remainder) = settle_cosmetic_bar(cosmetic_bar_before, gained, cycle_before);
    let cosmetic_pays = delta_of(cosmetic_paid.len())?;
    let resting = cycle_at(cycle_before + cosmetic_pays);
    if cosmetic_pays > 0 {
        grant_counter(
            tx,
            profile_id,
            Counter::CosmeticTokenBar,
            cosmetic_remainder - cosmetic_bar_before,
            EconomyReason::TokenBar,
            reference,
            at,
        )
        .await?;
        for paid in 0..cosmetic_pays {
            let (item, key) = cycle_step(cycle_before + paid);
            tx.economy_inventory_grant(&InventoryGrant {
                profile_id: profile_id.to_string(),
                item,
                key,
                delta: 1,
                reason: EconomyReason::TokenBar,
                reference: reference.clone(),
                dev: false,
                at,
            })
            .await?
            .ok_or_else(|| ApiError::internal("the inventory refused the cosmetic bar's step"))?;
        }
        grant_counter(
            tx,
            profile_id,
            Counter::CosmeticTokenCycle,
            cosmetic_pays,
            EconomyReason::TokenBar,
            reference,
            at,
        )
        .await?;
    } else {
        grant_counter(
            tx,
            profile_id,
            Counter::CosmeticTokenBar,
            gained,
            EconomyReason::PackOpen,
            reference,
            at,
        )
        .await?;
    }
    Ok(BarsSettled {
        card: BarSettle {
            before: card_bar_before,
            after: card_remainder,
            size: CARD_TOKEN_BAR_SIZE,
            pays: card_pays,
        },
        cosmetic: BarSettle {
            before: cosmetic_bar_before,
            after: cosmetic_remainder,
            size: resting.bar_size(),
            pays: cosmetic_pays,
        },
        cosmetic_sizes: cosmetic_paid
            .iter()
            .chain([&resting])
            .map(|step| step.bar_size())
            .collect(),
        cosmetic_paid,
        slot_points,
    })
}

/// §14.3: opens one unopened pack of `pack`'s kind, in one transaction. Under the account's lock
/// (R1301003) it reads the pack held, the collection, the per-card and account cosmetics and the
/// counters (the pity counters and both bars with the cycle's position), rolls the opening from
/// `seed` (`roll::roll_opening`) and writes every grant: the pack spent, the new cards (the
/// collection and its ledger, reason `pack`), the cosmetics, a per-card one on its slot's card and
/// an account one on none (R1661001), each pity counter that moved, both bars' points for every
/// slot, each its slot's rarity's whatever it grants (R1953000, R1999000), and each bar that
/// reached its size paying its tokens with the overflow kept (R1305000, R1305001). Every economy
/// row's `ref` is the opening's log; the collection's rows have no `ref` and share the opening's
/// `at`. No pack held is 409. Every refusal and failure returns before the commit, so it writes
/// nothing. With no key, [`open_pack_once`]'s opening.
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
    let (item, key) = inventory_of(pack);
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
    if held < 1 {
        return Err(no_pack_held());
    }
    let counters = tx.economy_counters_get(profile_id).await?;
    let owned_cosmetics = tx.economy_cosmetics_get(profile_id).await?;
    let holdings = Holdings {
        owned: owned_in(&mut tx, profile_id).await?,
        cosmetics: owned_cosmetics.cards,
        account_cosmetics: owned_cosmetics.account,
        pity: pity_from(&counters),
    };
    let opening = roll_opening(&pack_pool(&app.catalog.defs, pack), &holdings, registry, seed)
        .ok_or_else(|| ApiError::internal(format!("the {} pack's pool holds no card", pack.name())))?;
    let reference = Some(serde_json::to_string(&OpeningLog::of(
        pack,
        &app.catalog.version,
        registry,
        seed,
        &opening,
    ))?);
    let at = crate::app::now_ms();

    let unopened = tx
        .economy_inventory_grant(&InventoryGrant {
            profile_id: profile_id.to_string(),
            item,
            key,
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
        if slot.grant == SlotGrant::Card {
            *unlocked.entry(slot.card_id.clone()).or_insert(0) += 1;
        }
    }
    grant_unlocked(&mut tx, profile_id, &holdings.owned, &unlocked, at).await?;

    for slot in &opening.slots {
        if let SlotGrant::Cosmetic { cosmetic_id, .. } = &slot.grant {
            // The global roll's account cosmetic is owned for the account, on no card (R1661001).
            let for_account = registry
                .iter()
                .any(|cosmetic| cosmetic.id == *cosmetic_id && !cosmetic.kind.is_per_card());
            let card_id = (!for_account).then(|| slot.card_id.clone());
            let owned = tx
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
            if !owned {
                return Err(ApiError::internal(format!(
                    "{cosmetic_id} was already owned on {card_id:?}"
                )));
            }
        }
    }

    for (counter, before, after) in [
        (Counter::PityEpic, holdings.pity.epic, opening.pity.epic),
        (
            Counter::PityLegendary,
            holdings.pity.legendary,
            opening.pity.legendary,
        ),
        (Counter::PityMythic, holdings.pity.mythic, opening.pity.mythic),
    ] {
        let delta = delta_of(after)? - delta_of(before)?;
        grant_counter(
            &mut tx,
            profile_id,
            counter,
            delta,
            EconomyReason::PackOpen,
            &reference,
            at,
        )
        .await?;
    }

    let slot_points = slot_bar_points(&opening.slots);
    let spent = opening
        .slots
        .iter()
        .filter(|slot| matches!(slot.grant, SlotGrant::Bars { .. }))
        .count();

    let bars = settle_bars(&mut tx, profile_id, &counters, slot_points, &reference, at).await?;
    let opened = OpenedPack {
        pack: pack.name().to_string(),
        slots: opening.slots,
        pity: opening.pity,
        unopened,
        bars,
    };
    keep(&mut tx, profile_id, request, &opened).await?;

    tx.commit().await?;
    tracing::info!(
        event = "packs.opened",
        "profileId" = %profile_id,
        pack = pack.name(),
        cards = unlocked.len(),
        spent,
    );
    Ok(Spent::Now(opened))
}

/// `GET /api/packs` (`AuthLevel::Active`): the caller's pity counters and the odds, for the pack
/// screen to show (§14.4): the price and slots, each counter's threshold, and one roll's odds on
/// the tier tables, the fallback included (R1339000): a slot's, the floored slot's and a
/// duplicate's cosmetic, and a duplicate's global roll (R1661001), its `chance`,
/// `GLOBAL_COSMETIC_ROLL_WEIGHT` over `TIER_WEIGHT_TOTAL`, and its `tiers`, the table floored at
/// `GLOBAL_COSMETIC_FLOOR_RARITY`. It also lists the unopened packs the caller holds (`unopened`,
/// [`unopened_from`]).
pub async fn get_packs(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let pity = pity_from(&tx.economy_counters_get(&profile.id).await?);
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
