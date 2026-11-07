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
/// kept), a token bar paying out (`TokenBar`, §14.5), a dev account's own fill (`Dev`, §14.10), a finished
/// game's pay (`Game`, §14.8), a quest arriving, leaving or paying (`Quest`, §14.9) and a set track's XP and
/// the levels it reached (`Track`, §14.7).
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
    Game,
    Quest,
    Track,
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
        /// The row's `ref` when it is not the outcome's own (a level reward names its payout's match or quest).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<String>,
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
    /// `delta` Jlore to the wallet (§14.1), `cause` the game, quest or track level that paid it (or a dev tool).
    /// A pay is never a zero delta.
    Jlore {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        delta: i64,
        cause: Cause,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<String>,
    },
    /// XP, levels reached and level rewards claimed added to one set track (§14.7), each signed; the
    /// store's track row after it is [`SetTrack`] moved by them (a track not yet written starts at its first
    /// level with no XP and nothing claimed).
    Track {
        set_id: String,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        xp: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        levels: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        rewards: i64,
        cause: Cause,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<String>,
    },
    /// [[R3335001]]: XP and levels reached added to the account's Fleet track (§14.15), each signed; the store's
    /// track row after it is [`crate::economy::fleet::FleetTrack`] moved by them (a track not yet written starts at
    /// level 0 with no XP).
    Fleet {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        xp: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        levels: i64,
        cause: Cause,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<String>,
    },
    /// [[R3335001]]: a Fleet track level's reward, an account cosmetic owned from now on, its ledger reason `track`
    /// (R3335002). An account that owns it already (a dev account's fill, §14.10) keeps it, and the store writes
    /// no row.
    FleetReward {
        cosmetic_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<String>,
    },
    /// A held quest's change (§14.9): it arrives, progresses by `progress`, completes or is replaced. `at` is
    /// when the change is dated: an arrival is dated when it fell due (R1304000).
    Quest {
        quest_id: String,
        change: QuestChange,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        progress: i64,
        cause: Cause,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<String>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// When the next quest falls due (`null`: none does). A schedule, not a holding: no ledger row.
    QuestDue {
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        next_due_at: Option<i64>,
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

/// What befell a held quest (§14.9; `quest_grants.change`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "snake_case")]
pub enum QuestChange {
    Arrived,
    Progressed,
    Completed,
    Removed,
}

/// One row of `public.set_tracks` (§14.7).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SetTrack {
    pub set_id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp: i64,
    /// From `SET_TRACK_FIRST_LEVEL`.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub level: i64,
    /// How many levels have paid their rewards: from 0 to `level`.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub rewards_claimed: i64,
}

/// One row of `public.held_quests` (§14.9).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct HeldQuest {
    pub quest_id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub progress: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub arrived_at: i64,
}

/// An account's quests: the held ones, oldest first (by when they arrived, then id), and when the next is due
/// (`None`: none is).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestBook {
    pub held: Vec<HeldQuest>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub next_due_at: Option<i64>,
}
