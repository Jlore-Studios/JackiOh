//! The economy's JSON boundary: `jackioh-wasm-core`'s `economy(call)` takes one [`EconomyCall`] and answers one
//! [`EconomyAnswer`], so the browser and a host that is not Rust (the C module's, #3093) make the same
//! decisions the server makes natively (`crates/shared/README.md`, the host contract). [`apply`] is the one
//! function behind it. A call names the records the store read and the host's seed and clock; a pool, a
//! registry or a catalog version it leaves out is the one compiled in.

use serde::{Deserialize, Serialize};

use crate::cosmetics::{Cosmetic, registry, shipped_defs};
use crate::economy::bars::{BarFill, BarKind, fill_bar};
use crate::economy::ledger::AccountCounters;
use crate::economy::open::{
    CosmeticPackInputs, CosmeticPackOpening, EconomyRefusal, PackInputs, PackOpening, PackRecords,
    open_card_pack, open_cosmetic_pack,
};
use crate::economy::roll::{PackKind, PoolCard, pack_pool};
use crate::economy::track_rewards::{LevelReward, reward_for_level};

/// One call of the economy's decisions, as one JSON object tagged by `call`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "call", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum EconomyCall {
    /// [`open_card_pack`]. `pool` is the pack's cards in catalog order; left out, it is `pack`'s pool in the
    /// compiled-in catalog.
    OpenCardPack {
        records: PackRecords,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        pack: PackKind,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        pool: Option<Vec<PoolCard>>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        registry: Option<Vec<Cosmetic>>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        catalog: Option<String>,
        seed: String,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// [`open_cosmetic_pack`]. `pool` left out is the global pack's in the compiled-in catalog.
    OpenCosmeticPack {
        records: PackRecords,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        pool: Option<Vec<PoolCard>>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        registry: Option<Vec<Cosmetic>>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        catalog: Option<String>,
        seed: String,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// [`fill_bar`]: a dev account's fill of one token bar.
    FillBar { bar: BarKind, counters: AccountCounters },
    /// [`reward_for_level`]: what a set-track level pays.
    LevelReward {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        level: i64,
    },
}

/// What an [`EconomyCall`] answers, as one JSON object with one key: an opening to write, a refusal to tell the
/// player, or a read.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum EconomyAnswer {
    /// `openCardPack`: the opening, to write and to answer.
    Opened(Box<PackOpening>),
    /// `openCosmeticPack`: the opening, to write and to answer.
    CosmeticOpened(Box<CosmeticPackOpening>),
    /// `fillBar`: the writes of the fill.
    Filled(BarFill),
    /// `levelReward`.
    Reward(LevelReward),
    /// A decision that did not apply: nothing to write.
    Refused(EconomyRefusal),
}

/// Makes `call`: the decision it names, over its records, answered as data. Nothing is read but the call
/// and what is compiled in, and nothing is kept.
pub fn apply(call: &EconomyCall) -> EconomyAnswer {
    match call {
        EconomyCall::OpenCardPack {
            records,
            pack,
            pool,
            registry: supplied,
            catalog,
            seed,
            now,
        } => {
            let inputs = PackInputs {
                pack: *pack,
                pool: pool.clone().unwrap_or_else(|| pack_pool(shipped_defs(), *pack)),
                registry: supplied.clone().unwrap_or_else(|| registry().to_vec()),
                catalog: catalog.clone().unwrap_or_else(compiled_catalog),
            };
            match open_card_pack(records, &inputs, seed, *now) {
                Ok(opening) => EconomyAnswer::Opened(Box::new(opening)),
                Err(refusal) => EconomyAnswer::Refused(refusal),
            }
        }
        EconomyCall::OpenCosmeticPack {
            records,
            pool,
            registry: supplied,
            catalog,
            seed,
            now,
        } => {
            let inputs = CosmeticPackInputs {
                pool: pool
                    .clone()
                    .unwrap_or_else(|| pack_pool(shipped_defs(), PackKind::Global)),
                registry: supplied.clone().unwrap_or_else(|| registry().to_vec()),
                catalog: catalog.clone().unwrap_or_else(compiled_catalog),
            };
            match open_cosmetic_pack(records, &inputs, seed, *now) {
                Ok(opening) => EconomyAnswer::CosmeticOpened(Box::new(opening)),
                Err(refusal) => EconomyAnswer::Refused(refusal),
            }
        }
        EconomyCall::FillBar { bar, counters } => EconomyAnswer::Filled(fill_bar(*bar, counters)),
        EconomyCall::LevelReward { level } => EconomyAnswer::Reward(reward_for_level(*level)),
    }
}

/// The catalog version compiled in: the one an opening logs when its call names none.
fn compiled_catalog() -> String {
    jackioh_cards::catalog_version().to_string()
}
