//! The rows an economy outcome names (SPEC §14.11): the counters and inventory items an account holds, and
//! the [`LedgerGrant`]s a decision answers in the order the store writes them. A grant is a change of one holding;
//! the store writes it beside its ledger row (Postgres on the server, a DataStore on a host without one),
//! and decides nothing (`crates/shared/README.md`, the host contract).

use serde::{Deserialize, Serialize};

use crate::economy::math::PityCounters;

/// What an inventory row counts (§14.3, §14.5, §14.6). A set's card pack is keyed by its set, the
/// global card pack and a cosmetic pack by nothing (`""`), a card token by nothing (any new card) or
/// a rarity, and a cosmetic token by its rarity.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "snake_case")]
pub enum InventoryItem {
    SetPack,
    GlobalPack,
    CosmeticPack,
    CardToken,
    CosmeticToken,
}

/// One of an account's counters (§14.4–§14.6): a pity counter per rarity, each token bar's points,
/// the cosmetic token cycle's steps paid, and the Chase Shard bar's cosmetic packs opened.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "snake_case")]
pub enum Counter {
    PityEpic,
    PityLegendary,
    PityMythic,
    CardTokenBar,
    CosmeticTokenBar,
    CosmeticTokenCycle,
    ChaseShardBar,
}

impl Counter {
    /// A pity counter (§14.4), the only kind a pack key may name: its account keeps one over all
    /// its card packs (`""`) or one per kind of pack, as the pity math's sub-issue rules.
    pub fn is_pity(self) -> bool {
        matches!(
            self,
            Counter::PityEpic | Counter::PityLegendary | Counter::PityMythic
        )
    }
}

/// Why a grant was written, which the store's ledger reason names: the opening itself (`PackOpen`: the pack
/// spent, the cards and cosmetics it granted, the counters it moved and the points a bar that did not pay
/// kept), a token bar paying out (`TokenBar`, §14.5) or a dev account's own fill (`Dev`, §14.10).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    PackOpen,
    TokenBar,
    Dev,
}

/// What an account's counters hold as an opening begins (§14.4–§14.6), the store's read of its account-wide
/// rows: the pity counters (R1298000), each token bar's points (§14.5), the cosmetic cycle's steps paid
/// (R2728000) and the Chase Shard bar's cosmetic packs opened (§14.6). A counter never written is 0.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AccountCounters {
    pub pity: PityCounters,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub card_bar: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub cosmetic_bar: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub cosmetic_cycle: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub chase_shard: i64,
}

/// One card of an opening's unlocked cards: the copies it adds and the quantity the account then holds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardCopies {
    pub card_id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub delta: i64,
    /// The copies held after the opening: the collection's absolute quantity.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub quantity: i64,
}

/// One change an opening or a fill makes to what an account holds, in the order the store writes them. A
/// store writes each as the holding's new value and a ledger row (§14.11) carrying the opening's log as its
/// `ref` and the outcome's `at`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LedgerGrant {
    /// `delta` of `item` under `key` (a pack spent is -1). A store refuses one that would take the count
    /// below zero.
    Inventory {
        item: InventoryItem,
        key: String,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        delta: i64,
        cause: Cause,
    },
    /// Cards the opening unlocked, to the collection: the quantities and their ledger's deltas.
    Cards { cards: Vec<CardCopies> },
    /// A cosmetic owned from now on: per card with `card_id`, else on the account. A store refuses one
    /// already owned.
    Cosmetic {
        card_id: Option<String>,
        cosmetic_id: String,
    },
    /// `delta` to an account-wide counter. Never a zero delta: a ledger row records a change.
    Counter {
        counter: Counter,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        delta: i64,
        cause: Cause,
    },
}

impl LedgerGrant {
    /// A counter's change, or nothing for a zero one.
    pub fn counter(counter: Counter, delta: i64, cause: Cause) -> Option<LedgerGrant> {
        (delta != 0).then_some(LedgerGrant::Counter {
            counter,
            delta,
            cause,
        })
    }
}
