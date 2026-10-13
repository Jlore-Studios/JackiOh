//! The generic aura verb (#464 gap 8): a stat `mod` and an `applies` filter, declared as data.
//! `J.aura({ mod = { attack = 2 }, applies = { side = "self" } })` answers one aura entry whose
//! filter the host evaluates in Rust — no Luau call per stat read — while
//! `applies = "my_filter"` names a face predicate called per candidate unit, for the shapes data
//! cannot say. Either way the aura hook answers a list of these, like `defense_armor_aura`.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[derive(ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct DefenseArmorArgs {
    #[serde(default)]
    pub armor: i32,
    #[serde(default)]
    pub n: Option<i32>,
}

/// Which units a data-declared aura touches, read off the instance alone (an aura's `applies`
/// reads instance data only, or the layers recurse).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[derive(ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct AuraFilter {
    /// Whose units, relative to the aura source's controller. Default the source's own.
    #[serde(default)]
    pub side: Option<jackioh_engine::effects::ScopeSide>,
    /// Which rows. Default the units row.
    #[serde(default)]
    pub rows: Option<Vec<jackioh_engine::Row>>,
    /// Only units of this def (Rush Token Farm's Rush Tokens).
    #[serde(default)]
    pub def_id: Option<String>,
    /// Every unit but those of this def (Spikey Pillow's radiant mercy).
    #[serde(default)]
    pub except_def_id: Option<String>,
}

/// A generic aura's filter: data, or the name of a face predicate over the candidate unit.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(ts_rs::TS)]
#[serde(untagged)]
pub enum AuraApplies {
    Filter(AuraFilter),
    Predicate(String),
}

/// A generic aura entry, as data.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct AuraArgs {
    #[serde(rename = "mod")]
    pub mod_: jackioh_engine::script::StatMod,
    pub applies: AuraApplies,
}

verbs! {
    verb!(defense_armor_aura, DefenseArmorArgs);
    verb!(defense_armor, DefenseArmorArgs);
    verb!(aura, AuraArgs);
}
