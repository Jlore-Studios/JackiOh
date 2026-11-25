//! The Infinite Assault (`docs/infinite-assault.md` IA0 to IA6): a daily 20-level gauntlet beside
//! the dungeon run, sharing its seeds, its `Rng` use and its fight report (CLAUDE.md rule 4). Its
//! numbers are in `crate::config` (rule 9), its content file is `src/assault.json`, and this file is
//! the contract other code relies on, as `crates/run/README.md` says.
//!
//! Every rule here is the plan's (its IA sections); each decision the plan leaves open is a ruling
//! note in issue #4579's block ([[R4579000]], [[R4579001]], [[R4579002]]).

mod attempt;
mod content;
mod day;

#[cfg(test)]
mod fight_tests;
#[cfg(test)]
mod tests;

pub use attempt::{
    assault_create, assault_fight_report, assault_fight_setup, assault_fold, assault_record, assault_reduce,
};
pub use content::{assault_catalog_content, assault_placeholder_content};
pub use day::{assault_day, assault_day_seed};

use jackioh_engine::config::Handicap;
use jackioh_engine::wire::{DeckMark, Rarity};
use serde::{Deserialize, Serialize};

use crate::config::{ASSAULT_COMMANDER_LEVELS, ASSAULT_HISTORY_DAYS, ASSAULT_LEVELS, ASSAULT_RARITY_CEILING};

/// What a day deals from: the set's cards by tag, and `assault.json`'s doctrines and Commanders.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultContent {
    pub cards: Vec<AssaultCard>,
    pub doctrines: Vec<AssaultDoctrine>,
    pub commanders: Vec<AssaultCommander>,
}

/// One card the Assault may deal: an Infinite Assault card by set and tag ([[R4579001]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultCard {
    pub id: String,
    pub rarity: Rarity,
    pub doctrine: Option<String>,
    pub commander: bool,
}

/// One doctrine: an archetype of the set, a tag on its cards ([[R4579001]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultDoctrine {
    pub id: String,
    pub name: String,
    pub line: String,
    pub enemies: Vec<String>,
}

/// One Commander: a boss of one rank, its own cards on top of its doctrine ([[R4579001]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultCommander {
    pub id: String,
    pub name: String,
    pub rank: u8,
    pub doctrine: Option<String>,
    pub cards: Vec<String>,
}

/// IA3: the upgrades, one stack at a time ([[R4579000]]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum AssaultUpgradeKind {
    OpeningCards,
    DeckSize,
    Radiant,
    Buffed,
    ManaBonus,
    ManaCap,
    HeroHealth,
    HeroArmor,
    ExtraDraw,
}

/// One held upgrade and its stacks, in `ASSAULT_UPGRADES`'s table order ([[R4579000]]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultUpgrade {
    pub kind: AssaultUpgradeKind,
    pub stacks: i32,
}

/// IA2: what one level's escalation added ([[R4579000]]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AssaultEscalation {
    Cards,
    Upgrade { upgrade: AssaultUpgradeKind },
}

/// IA2: one level of the day: its enemy, deck, marks, handicap and escalations ([[R4579000]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultLevel {
    pub level: u8,
    pub enemy: String,
    pub doctrine: Option<String>,
    pub commander: Option<String>,
    /// What this level added: nothing on level 1, one escalation on a normal level, a card step
    /// and an upgrade on a Commander level.
    pub escalations: Vec<AssaultEscalation>,
    /// Everything held at this level, the kinds with stacks above 0 in table order.
    pub upgrades: Vec<AssaultUpgrade>,
    /// Its deck's Common, Rare, Epic, Legendary and Mythic counts: the level's slots ([[R4579000]]).
    pub rarities: [i32; 5],
    pub deck: Vec<String>,
    pub marks: Vec<DeckMark>,
    pub handicap: Handicap,
}

/// IA1: a day's deal: its 20 levels ([[R4579001]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultDay {
    pub date: String,
    pub seed: String,
    pub levels: Vec<AssaultLevel>,
}

/// IA5: where the attempt's deck comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum AssaultDeckSource {
    Saved,
    Preset,
    DayDeck,
}

/// IA6: an attempt's setup: the day, the deck and its source, the start level, the attempt's
/// number ([[R4579002]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultSetup {
    pub date: String,
    pub deck: Vec<String>,
    pub source: AssaultDeckSource,
    pub start_level: u8,
    pub attempt: u32,
}

/// IA6: what the attempt waits on ([[R4579002]]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum AssaultPhase {
    Briefing,
    Fight,
    Lost,
    Repelled,
    Abandoned,
}

/// IA6: an attempt: its setup, the level it stands on, its phase, the levels it beat and its turns
/// ([[R4579002]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultState {
    pub setup: AssaultSetup,
    pub level: u8,
    pub phase: AssaultPhase,
    pub beaten: Vec<u8>,
    pub turns: i32,
}

/// IA6: what an attempt does: fight, a fight's result, or abandon ([[R4579002]]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AssaultAction {
    Fight,
    FightOver { won: bool, turns: i32 },
    Abandon,
}

/// What the caller hands `createGame` for the fight being played: the human is P1, the enemy P2
/// ([[R4579002]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultFightSetup {
    /// The level's game seed: `{day seed}-L{level}-A{attempt}`.
    pub seed: String,
    pub human_deck: Vec<String>,
    pub human_handicap: Handicap,
    pub enemy_deck: Vec<String>,
    pub enemy_handicap: Handicap,
    pub enemy_marks: Vec<DeckMark>,
    pub level: AssaultLevel,
}

/// IA6: the day's record: the highest level beaten, the checkpoints reached, the attempts started
/// and the attempt that first repelled the Assault, if any ([[R4579002]]).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultRecord {
    pub date: String,
    pub best: u8,
    pub checkpoints: Vec<u8>,
    pub attempts: u32,
    pub repelled_on: Option<u32>,
}

/// IA6: an attempt as one file: its setup, the day's record at its start and the action log, with
/// the catalog version the caller folds under ([[R3496000]]); nothing here reads it
/// ([[R4579002]]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultFile {
    pub version: String,
    pub setup: AssaultSetup,
    pub record: AssaultRecord,
    pub actions: Vec<AssaultAction>,
}

/// The numbers the screens show, so the client states none (rule 9).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AssaultConstants {
    pub levels: u8,
    pub commander_levels: Vec<u8>,
    pub checkpoints: Vec<u8>,
    pub history_days: u32,
}

/// The numbers the screens show, so the client states none (rule 9).
pub fn assault_constants() -> AssaultConstants {
    AssaultConstants {
        levels: ASSAULT_LEVELS,
        commander_levels: ASSAULT_COMMANDER_LEVELS.to_vec(),
        checkpoints: checkpoints(),
        history_days: ASSAULT_HISTORY_DAYS,
    }
}

/// The position of a rarity in the deck's slots, matched by name; a Token prints none and takes
/// none ([[R4579001]]).
pub(crate) fn slot_of(rarity: Rarity) -> Option<usize> {
    jackioh_engine::wire::PrintedRarity::ALL
        .iter()
        .position(|printed| printed.as_str() == rarity.as_str())
}

/// IA2: the rarest slot a level may hold, as a slot index: the last `ASSAULT_RARITY_CEILING` row at
/// or below the level ([[R4579000]]).
pub(crate) fn ceiling_at(level: u8) -> usize {
    let mut ceiling = 1;
    for (from, rarity) in ASSAULT_RARITY_CEILING {
        if level >= from {
            ceiling = match rarity {
                jackioh_engine::wire::PrintedRarity::Common => 0,
                jackioh_engine::wire::PrintedRarity::Rare => 1,
                jackioh_engine::wire::PrintedRarity::Epic => 2,
                jackioh_engine::wire::PrintedRarity::Legendary => 3,
                jackioh_engine::wire::PrintedRarity::Mythic => 4,
            };
        }
    }
    ceiling
}

/// The rank index of a Commander level, or none on a normal level ([[R4579000]]).
pub(crate) fn commander_rank(level: u8) -> Option<usize> {
    ASSAULT_COMMANDER_LEVELS.iter().position(|at| *at == level)
}

/// IA6: the levels an attempt may start at besides 1: each Commander level below the top, plus one
/// ([[R4579002]]).
pub(crate) fn checkpoints() -> Vec<u8> {
    ASSAULT_COMMANDER_LEVELS
        .iter()
        .filter(|at| **at < ASSAULT_LEVELS)
        .map(|at| at + 1)
        .collect()
}
