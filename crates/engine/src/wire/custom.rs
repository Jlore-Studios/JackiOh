//! #915's wire types (SPEC §12, R915000–R915009): a custom card as the account stores it, the
//! creator edits it and a game carries it.
//!
//! A custom card is a frame, which the engine turns into a `CardDef` (`custom::custom_def`), and a
//! script in the card kit, which only a host that runs Luau can turn into `CardScripts`
//! (`custom::CustomScriptHost`). The engine never reads the builder's model: it is the creator's own
//! data, kept beside the card and dropped when a game is dealt.

use serde::{Deserialize, Serialize};

use crate::wire::catalog_types::{CardType, KeywordKind, PrintedRarity};

/// R915001: one number of a custom card's text (`{key}`), with its value on each face.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CustomParam {
    pub key: String,
    pub base: i32,
    pub radiant: i32,
}

/// R915001: a custom card's frame, everything but its script, within §12.2's limits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CustomFrame {
    pub name: String,
    #[serde(rename = "type")]
    #[cfg_attr(feature = "ts", ts(rename = "type"))]
    pub type_: CardType,
    pub cost: i32,
    pub rarity: PrintedRarity,
    /// A Unit's attack and health; a Spell has neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub health: Option<i32>,
    /// A Radiant Unit's attack and health; absent, twice the base stats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub radiant_attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub radiant_health: Option<i32>,
    /// A Unit's keywords, each once, from `CUSTOM_KEYWORDS`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<KeywordKind>>", optional))]
    pub keywords: Vec<KeywordKind>,
    /// The card's text, its numbers written `{key}`.
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CustomParam>>", optional))]
    pub params: Vec<CustomParam>,
}

/// R915000: a custom card: its id (`custom-…`), the count of its saves, its frame, its card-kit
/// source and, for the creator alone, the builder's model of it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CustomCard {
    pub id: String,
    pub version: i32,
    pub frame: CustomFrame,
    /// The card-kit Luau (§12.3). Empty, the card has no script: a vanilla card.
    #[serde(default)]
    pub source: String,
    /// The no-code builder's model, which only the creator reads (§12.2); absent for a card edited in
    /// code beyond what the builder shows. A game never carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub builder: Option<serde_json::Value>,
}
