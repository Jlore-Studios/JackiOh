//! The decisions around an opening (SPEC §14.3, §14.6; #1301, #1306, #3104), each
//! `decide(records, inputs, seed, now_ms) -> outcome`: what the store read goes in as plain records, the
//! host's seed and clock beside the pool and registry the roll draws from, and the outcome is everything to
//! write as data, in the order to write it, with the answer a client reads and the log every row's `ref`
//! carries (R1301003, R1306003). The server's store code writes the outcome to Postgres under the account's
//! lock and decides nothing; a Roblox script would write the same outcome to DataStores.
//!
//! - [`open_card_pack`]: the pack spent, the roll ([`roll_opening`]), the cards and cosmetics it granted,
//!   the pity counters it moved and both token bars it filled (R1298000, R1305000, R1339002).
//! - [`open_cosmetic_pack`]: the pack spent, the roll ([`roll_cosmetic_pack`]), the Chase Shard bar's one
//!   and both token bars (R1306000, R1306002).
//!
//! Every draw is the roll's, from the seed in the order it documents, so a replayed opening grants the same.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::CHASE_SHARD_PACKS;
use crate::cosmetics::{Cosmetic, OwnedCosmetics};
use crate::economy::bars::{BarState, BarsSettled, cosmetic_bar_points, settle_bars, slot_bar_points};
use crate::economy::cosmetic_pack::{
    CosmeticHoldings, CosmeticOpeningLog, CosmeticSlot, CosmeticSlotGrant, roll_cosmetic_pack,
};
use crate::economy::ledger::{AccountCounters, CardCopies, Cause, Counter, InventoryItem, LedgerGrant};
use crate::economy::math::PityCounters;
use crate::economy::roll::{Holdings, OpeningLog, PackKind, PoolCard, Slot, SlotGrant, roll_opening};

/// Why a decision refused to open a pack.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    /// The account holds no unopened pack of that kind (a conflict: it writes nothing).
    NoPackHeld,
    /// The pack's pool holds no card (a fault of the catalog, not of the player).
    EmptyPool,
}

/// A refusal is data (the host contract): the reason and the sentence a player or a log reads.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct EconomyRefusal {
    pub reason: RefusalReason,
    pub message: String,
}

/// What the store read as a pack opens: the unopened packs of its kind the account holds, its copies of
/// each card, its cosmetics, and its account-wide counters. The same shape opens a card pack and a
/// cosmetic pack.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PackRecords {
    /// The packs of the opened kind held: a set's pack, the global pack or a cosmetic pack
    /// ([`PackKind::inventory`], §14.3).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub held: i64,
    /// Copies held of each card by catalog id; a card never owned has no entry.
    #[cfg_attr(feature = "ts", ts(type = "{ [key in string]: number }"))]
    pub owned: IndexMap<String, i64>,
    pub cosmetics: OwnedCosmetics,
    pub counters: AccountCounters,
}

/// What a card pack's roll draws from beyond the seed: the pack, its pool in catalog order (`pack_pool`),
/// the cosmetics registry and the catalog version the pool was built from (logged, so the opening replays,
/// R1301003).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PackInputs {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub pack: PackKind,
    pub pool: Vec<PoolCard>,
    pub registry: Vec<Cosmetic>,
    pub catalog: String,
}

/// What a cosmetic pack's roll draws from beyond the seed: the global pack's pool, the registry and the
/// catalog version (R1306003).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPackInputs {
    pub pool: Vec<PoolCard>,
    pub registry: Vec<Cosmetic>,
    pub catalog: String,
}

/// A card pack opened: everything to write and to answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PackOpening {
    /// The pack's name, as a request gives it.
    pub pack: String,
    /// Each slot's rarity, card and grant, in order.
    pub slots: Vec<Slot>,
    /// The pity counters after the opening (R1298000).
    pub pity: PityCounters,
    /// What the opening did to both token bars.
    pub bars: BarsSettled,
    /// Every write, in the order to make it: the pack spent, the cards, the cosmetics, the pity counters,
    /// then both bars (R1305000).
    pub grants: Vec<LedgerGrant>,
    /// Every row's `ref` (R1301003): the roll's inputs and result, which replays it.
    pub log: OpeningLog,
    /// The stamp every row shares.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub at: i64,
}

/// A cosmetic pack opened: everything to write and to answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticPackOpening {
    pub slots: Vec<CosmeticSlot>,
    /// The Chase Shard bar's points the opening found, so a client fills the bar from here to `shard_bar`.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub shard_before: i64,
    pub shard_bar: BarState,
    pub bars: BarsSettled,
    /// Every write, in the order to make it: the pack spent, the cards, the cosmetics, the Chase Shard
    /// bar's one, then both bars (R1306001, R1306002).
    pub grants: Vec<LedgerGrant>,
    /// Every row's `ref` (R1306003).
    pub log: CosmeticOpeningLog,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub at: i64,
}

fn no_pack_held(message: &str) -> EconomyRefusal {
    EconomyRefusal {
        reason: RefusalReason::NoPackHeld,
        message: message.to_string(),
    }
}

/// The cards `granted` unlocked, one copy per entry, as the collection takes them: the copies each adds and
/// the quantity held after, cards in the order they first appear; none when none unlocked.
fn unlocked(owned: &IndexMap<String, i64>, granted: impl Iterator<Item = String>) -> Option<LedgerGrant> {
    let mut copies: IndexMap<String, i64> = IndexMap::new();
    for card_id in granted {
        *copies.entry(card_id).or_insert(0) += 1;
    }
    (!copies.is_empty()).then(|| LedgerGrant::Cards {
        cards: copies
            .into_iter()
            .map(|(card_id, delta)| CardCopies {
                quantity: owned.get(&card_id).copied().unwrap_or(0) + delta,
                card_id,
                delta,
            })
            .collect(),
    })
}

/// §14.3: opens one unopened pack of `inputs.pack`'s kind. The roll is [`roll_opening`] from `seed` over
/// the holdings in `records`, and the outcome carries every write: no pack held is a refusal, writing
/// nothing; a pool with no card is one too. A cosmetic that is an account cosmetic (the global roll's,
/// R1661001) is owned on no card.
pub fn open_card_pack(
    records: &PackRecords,
    inputs: &PackInputs,
    seed: &str,
    now_ms: i64,
) -> Result<PackOpening, EconomyRefusal> {
    if records.held < 1 {
        return Err(no_pack_held("no unopened pack of that kind"));
    }
    let holdings = Holdings {
        owned: records.owned.clone(),
        cosmetics: records.cosmetics.cards.clone(),
        account_cosmetics: records.cosmetics.account.clone(),
        pity: records.counters.pity,
    };
    let opening =
        roll_opening(&inputs.pool, &holdings, &inputs.registry, seed).ok_or_else(|| EconomyRefusal {
            reason: RefusalReason::EmptyPool,
            message: format!("the {} pack's pool holds no card", inputs.pack.name()),
        })?;
    let log = OpeningLog::of(inputs.pack, &inputs.catalog, &inputs.registry, seed, &opening);

    let (item, key) = inputs.pack.inventory();
    let mut grants = vec![LedgerGrant::Inventory {
        item,
        key,
        delta: -1,
        cause: Cause::PackOpen,
    }];
    grants.extend(unlocked(
        &records.owned,
        opening
            .slots
            .iter()
            .filter(|slot| slot.grant == SlotGrant::Card)
            .map(|slot| slot.card_id.clone()),
    ));
    for slot in &opening.slots {
        if let SlotGrant::Cosmetic { cosmetic_id, .. } = &slot.grant {
            // The global roll's account cosmetic is owned for the account, on no card (R1661001).
            let for_account = inputs
                .registry
                .iter()
                .any(|cosmetic| cosmetic.id == *cosmetic_id && !cosmetic.kind.is_per_card());
            grants.push(LedgerGrant::Cosmetic {
                card_id: (!for_account).then(|| slot.card_id.clone()),
                cosmetic_id: cosmetic_id.clone(),
            });
        }
    }
    let pity = records.counters.pity;
    for (counter, before, after) in [
        (Counter::PityEpic, pity.epic, opening.pity.epic),
        (Counter::PityLegendary, pity.legendary, opening.pity.legendary),
        (Counter::PityMythic, pity.mythic, opening.pity.mythic),
    ] {
        grants.extend(LedgerGrant::counter(
            counter,
            after as i64 - before as i64,
            Cause::PackOpen,
        ));
    }
    let (bars, settled) = settle_bars(&records.counters, slot_bar_points(&opening.slots));
    grants.extend(settled);
    Ok(PackOpening {
        pack: inputs.pack.name().to_string(),
        slots: opening.slots,
        pity: opening.pity,
        bars,
        grants,
        log,
        at: now_ms,
    })
}

/// §14.6: opens one unopened cosmetic pack. The roll is [`roll_cosmetic_pack`] from `seed`, and the outcome
/// carries every write: the pack spent, the new cards, the cosmetics, the Chase Shard bar's one (a full bar
/// holds, R1306002) and both token bars' points, each full bar paying its tokens with the overflow kept
/// (R1305000, R1306001). No pack held is a refusal, writing nothing.
pub fn open_cosmetic_pack(
    records: &PackRecords,
    inputs: &CosmeticPackInputs,
    seed: &str,
    now_ms: i64,
) -> Result<CosmeticPackOpening, EconomyRefusal> {
    if records.held < 1 {
        return Err(no_pack_held("no unopened cosmetic pack"));
    }
    let holdings = CosmeticHoldings::of(records.owned.clone(), &records.cosmetics);
    let opening = roll_cosmetic_pack(&inputs.pool, &holdings, &inputs.registry, seed);
    let log = CosmeticOpeningLog::of(&inputs.catalog, &inputs.registry, seed, &opening);

    let mut grants = vec![LedgerGrant::Inventory {
        item: InventoryItem::CosmeticPack,
        key: String::new(),
        delta: -1,
        cause: Cause::PackOpen,
    }];
    grants.extend(unlocked(
        &records.owned,
        opening.slots.iter().filter_map(|slot| match &slot.grant {
            CosmeticSlotGrant::Cosmetic {
                card_id: Some(card_id),
                card_granted: true,
                ..
            } => Some(card_id.clone()),
            _ => None,
        }),
    ));
    for slot in &opening.slots {
        if let CosmeticSlotGrant::Cosmetic {
            cosmetic_id, card_id, ..
        } = &slot.grant
        {
            grants.push(LedgerGrant::Cosmetic {
                card_id: card_id.clone(),
                cosmetic_id: cosmetic_id.clone(),
            });
        }
    }
    // The Chase Shard bar (R1306002): one per opening, and a full bar holds.
    let shard_before = records.counters.chase_shard;
    let mut shard = shard_before;
    if shard < CHASE_SHARD_PACKS {
        grants.extend(LedgerGrant::counter(Counter::ChaseShardBar, 1, Cause::PackOpen));
        shard += 1;
    }
    // Each slot's points are its rolled cosmetic rarity's, not the one it fell to, whatever it grants
    // (R1306001, R1953000, R1999000).
    let slot_points = opening
        .slots
        .iter()
        .map(|slot| cosmetic_bar_points(slot.rarity))
        .collect();
    let (bars, settled) = settle_bars(&records.counters, slot_points);
    grants.extend(settled);
    Ok(CosmeticPackOpening {
        slots: opening.slots,
        shard_before,
        shard_bar: BarState {
            points: shard,
            size: CHASE_SHARD_PACKS,
        },
        bars,
        grants,
        log,
        at: now_ms,
    })
}
