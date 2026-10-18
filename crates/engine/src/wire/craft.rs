//! ME-CRAFT's wire types (Meditative #17 True Craft a Card, `docs/meditative-set.md` M6 #17,
//! R880–R883): the recipe a `craft` prompt's answer carries, the block price tables the editor
//! prices from, and the preview the editor shows.
//!
//! A recipe is plain data, like every other `Selection`: it serialises as the client writes it
//! (camelCase, every `Option` optional and skipped when absent), so it survives the JSON round trip
//! a paused state takes (§10.6, R113) and names the transient definition it compiles to (R882).
//!
//! Meditative+ (#1182) adds two modes on the same prompt: the potion frame of M+ #16 Jazakus and
//! M+ #16.1 Jazakus Prime, which the recipe itself carries (`frame`), and Scratch Pad's from-scratch
//! canvas (M+ #26), which only the prompt carries (`CraftMode::Scratch`): R1182000 to R1182002.

use serde::{Deserialize, Serialize};

use crate::wire::catalog_types::{CardDef, CardType, KeywordKind};
use crate::wire::string_union;

string_union! {
    /// One hat a recipe may put an effect under: the hook the block becomes. A Unit takes `cry`,
    /// `death`, `startOfTurn` and `endOfTurn`; a Field Spell takes `cry`, `startOfTurn` and
    /// `endOfTurn`; a Spell takes `whenCast`; a Trap takes exactly one reveal hat (R883). A Potion
    /// takes `activate` and nothing else, and no other recipe takes it (R1182000).
    pub enum CraftHatKind {
        Cry = "cry",
        Death = "death",
        StartOfTurn = "startOfTurn",
        EndOfTurn = "endOfTurn",
        WhenCast = "whenCast",
        OpponentPlaysUnit = "opponentPlaysUnit",
        OpponentPlaysSpell = "opponentPlaysSpell",
        OpponentAttacks = "opponentAttacks",
        Activate = "activate",
    }
}

string_union! {
    /// R1182000, R1182001: the fixed frame a recipe is crafted in — a Potion (Meditative+ #16
    /// Jazakus, cost (1)) or a HUGE Potion (Meditative+ #16.1 Jazakus Prime, cost (3), five times a
    /// Potion's points): a Unit tagged Potion whose one hook is "Activate: Tribute this." with the
    /// effects the player builds. A recipe of Meditative #17's crafter carries none.
    pub enum CraftFrame {
        Potion = "potion",
        HugePotion = "hugePotion",
    }
}

string_union! {
    /// What a `craft` prompt asks for, beyond Meditative #17's free crafter (no mode): `scratch` is
    /// Scratch Pad's empty canvas (Meditative+ #26, R1182002) — the same recipes and budget, the
    /// client showing no presets, tooltips or hints — and `potion` and `hugePotion` the potion frame
    /// (R1182000, R1182001), whose answer must carry that frame.
    pub enum CraftMode {
        Scratch = "scratch",
        Potion = "potion",
        HugePotion = "hugePotion",
    }
}

impl CraftMode {
    /// The frame the answer's recipe must carry: the potion modes' own, none from scratch.
    pub fn frame(self) -> Option<CraftFrame> {
        match self {
            CraftMode::Scratch => None,
            CraftMode::Potion => Some(CraftFrame::Potion),
            CraftMode::HugePotion => Some(CraftFrame::HugePotion),
        }
    }
}

string_union! {
    /// One effect verb a recipe may use: the effects-library call the block becomes
    /// (`docs/meditative-set.md` M6 #17's block table, R883).
    pub enum CraftVerb {
        DamageTarget = "damageTarget",
        DamageEnemyHero = "damageEnemyHero",
        DamageRandomEnemy = "damageRandomEnemy",
        DamageEachEnemy = "damageEachEnemy",
        HealTarget = "healTarget",
        HealYourHero = "healYourHero",
        HealYourSide = "healYourSide",
        Draw = "draw",
        GainMana = "gainMana",
        GainArmor = "gainArmor",
        SummonRush = "summonRush",
        SummonFelinor = "summonFelinor",
        SummonSheep = "summonSheep",
        BuffTarget = "buffTarget",
        BuffYourUnits = "buffYourUnits",
        GrantKeyword = "grantKeyword",
        Destroy = "destroy",
        Bounce = "bounce",
        AddRandom = "addRandom",
        Discover = "discover",
        OpponentDiscards = "opponentDiscards",
        BuffRandomCard = "buffRandomCard",
        NerfRandomEnemyCard = "nerfRandomEnemyCard",
        LockRandomZone = "lockRandomZone",
    }
}

/// One keyword block: the keyword and, for Armor, its N. Every other keyword takes no N (R883).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftKeyword {
    pub kind: KeywordKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub n: Option<i32>,
}

/// One effect block: the verb and, for the verbs that take one, its N, its keyword
/// (`grantKeyword`) or its card type (`addRandom`, `discover`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftEffect {
    pub verb: CraftVerb,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub n: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub keyword: Option<KeywordKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub card_type: Option<CardType>,
}

/// One hat and the effects snapped beneath it, in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftHat {
    pub hat: CraftHatKind,
    pub effects: Vec<CraftEffect>,
}

/// What a `craft` prompt's answer carries (`Selection::Craft`): the card to make, within the
/// prompt's point and lines-of-code budgets (R880).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftRecipe {
    pub cost: i32,
    #[serde(rename = "type")]
    pub type_: CardType,
    pub adjective: String,
    pub noun: String,
    pub attack: i32,
    pub health: i32,
    pub keywords: Vec<CraftKeyword>,
    pub echo: i32,
    pub hats: Vec<CraftHat>,
    /// R1182000: the potion frame the recipe is crafted in. Absent on every recipe of Meditative
    /// #17's crafter and Scratch Pad's, so their JSON, their digest and the id it names are what
    /// they were before the field existed (R882, D9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frame: Option<CraftFrame>,
}

impl CraftRecipe {
    /// "Pure Closure": the name the card is made under (R883); a Potion is "Pure Closure Potion"
    /// and a HUGE one "HUGE Pure Closure Potion" (R1182000, R1182001).
    pub fn name(&self) -> String {
        match self.frame {
            None => format!("{} {}", self.adjective, self.noun),
            Some(CraftFrame::Potion) => format!("{} {} Potion", self.adjective, self.noun),
            Some(CraftFrame::HugePotion) => format!("HUGE {} {} Potion", self.adjective, self.noun),
        }
    }
}

/// One row of the verb price table: the price is `points + per_n × N`, halved (rounded up) when
/// `halve` is set; `lines` counts the `TargetDecl` line a targeted verb's declaration adds (R883).
/// Serialize only: the table lives in `config.rs`, and nothing reads it back from JSON.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftVerbPrice {
    pub verb: CraftVerb,
    pub points: i32,
    pub per_n: i32,
    pub halve: bool,
    pub lines: i32,
    pub targeted: bool,
    pub takes_n: bool,
}

/// One row of the keyword price table: `points`, or `per_n × N` for Armor. `spell` marks the
/// keywords a Spell may take (Lifesteal, Pierce); a Field Spell or Trap takes none (R883).
/// Serialize only: the table lives in `config.rs`, and nothing reads it back from JSON.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftKeywordPrice {
    pub kind: KeywordKind,
    pub points: i32,
    pub per_n: i32,
    pub spell: bool,
}

/// One row of the hat price table: `lines` for the hook, `multiplier` on the points of the
/// effects under it, and the card types that may take it (R883). Serialize only: the table lives
/// in `config.rs`, and nothing reads it back from JSON.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftHatPrice {
    pub hat: CraftHatKind,
    pub lines: i32,
    pub multiplier: i32,
    #[cfg_attr(feature = "ts", ts(as = "Vec<CardType>"))]
    pub types: &'static [CardType],
}

/// What the editor shows (`craft_preview`): the engine's own verdict on the recipe — validity,
/// one sentence per problem, the points and lines used against their budgets, and the card it
/// would make (R880). The client decides nothing (CLAUDE.md rule 7).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CraftPreview {
    pub valid: bool,
    pub reasons: Vec<String>,
    pub points: i32,
    pub points_budget: i32,
    pub loc: i32,
    pub loc_budget: i32,
    pub def: CardDef,
}
