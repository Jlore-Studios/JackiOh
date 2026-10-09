//! Card definitions and the vocabulary every package shares (SPEC §5, §6.1, §10.6).
//! Script and Effect types live in `crate::script`: they need GameState, which lives in the engine.
//!
//! Port of `packages/shared/src/catalog-types.ts` (part 1's type freeze, SURFACE §6.4), plus the
//! two per-seat containers of SURFACE §4.3/§6.4 (`PerPlayer`, `PerPlayerOpt`), which sit beside
//! `PlayerId` so every wire module can use them.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::wire::string_union;

string_union! {
    /// "p1" | "p2".
    pub enum PlayerId {
        P1 = "p1",
        P2 = "p2",
    }
}

/// Both seats, in seat order.
pub const PLAYER_IDS: [PlayerId; 2] = [PlayerId::P1, PlayerId::P2];

pub fn opponent_of(player: PlayerId) -> PlayerId {
    match player {
        PlayerId::P1 => PlayerId::P2,
        PlayerId::P2 => PlayerId::P1,
    }
}

impl PlayerId {
    /// `opponent_of(self)`.
    pub fn opponent(self) -> PlayerId {
        opponent_of(self)
    }

    /// The seat index: 0 for p1, 1 for p2 (TS `PLAYER_IDS.indexOf(player)`).
    pub fn seat(self) -> usize {
        match self {
            PlayerId::P1 => 0,
            PlayerId::P2 => 1,
        }
    }
}

/// `Record<PlayerId, T>` (SURFACE §4.3, §6.4): one value per seat, serialised `{ "p1": …, "p2": … }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
pub struct PerPlayer<T> {
    pub p1: T,
    pub p2: T,
}

impl<T> PerPlayer<T> {
    pub fn new(p1: T, p2: T) -> PerPlayer<T> {
        PerPlayer { p1, p2 }
    }

    /// Both seats' values, in seat order.
    pub fn iter(&self) -> impl Iterator<Item = (PlayerId, &T)> {
        [(PlayerId::P1, &self.p1), (PlayerId::P2, &self.p2)].into_iter()
    }

    pub fn map<U>(&self, mut f: impl FnMut(PlayerId, &T) -> U) -> PerPlayer<U> {
        PerPlayer {
            p1: f(PlayerId::P1, &self.p1),
            p2: f(PlayerId::P2, &self.p2),
        }
    }
}

impl<T> std::ops::Index<PlayerId> for PerPlayer<T> {
    type Output = T;

    fn index(&self, player: PlayerId) -> &T {
        match player {
            PlayerId::P1 => &self.p1,
            PlayerId::P2 => &self.p2,
        }
    }
}

impl<T> std::ops::IndexMut<PlayerId> for PerPlayer<T> {
    fn index_mut(&mut self, player: PlayerId) -> &mut T {
        match player {
            PlayerId::P1 => &mut self.p1,
            PlayerId::P2 => &mut self.p2,
        }
    }
}

/// `Partial<Record<PlayerId, T>>` (SURFACE §4.3): only a seat with a value has a key. (The serde
/// `bound` stops `#[serde(default)]` from asking `T: Default`, which a missing `Option` never needs.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
pub struct PerPlayerOpt<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub p1: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub p2: Option<T>,
}

impl<T> Default for PerPlayerOpt<T> {
    fn default() -> Self {
        PerPlayerOpt { p1: None, p2: None }
    }
}

impl<T> PerPlayerOpt<T> {
    pub fn get(&self, player: PlayerId) -> Option<&T> {
        match player {
            PlayerId::P1 => self.p1.as_ref(),
            PlayerId::P2 => self.p2.as_ref(),
        }
    }

    pub fn get_mut(&mut self, player: PlayerId) -> Option<&mut T> {
        match player {
            PlayerId::P1 => self.p1.as_mut(),
            PlayerId::P2 => self.p2.as_mut(),
        }
    }

    /// The seat's slot itself, to set or clear (`delete record[player]` is `*slot = None`).
    pub fn slot(&mut self, player: PlayerId) -> &mut Option<T> {
        match player {
            PlayerId::P1 => &mut self.p1,
            PlayerId::P2 => &mut self.p2,
        }
    }

    /// No seat has a value (TS: `Object.keys(record).length === 0`).
    pub fn is_empty(&self) -> bool {
        self.p1.is_none() && self.p2.is_none()
    }
}

string_union! {
    /// §5.1
    pub enum CardType {
        Unit = "Unit",
        Spell = "Spell",
        FieldSpell = "Field Spell",
        Trap = "Trap",
        FieldTrap = "Field Trap",
    }
}

string_union! {
    /// §5: tribes and tags. "Jlockeed" is Core #13 and #14's and the Classic+ Jlockheed cards' (R278);
    /// patch v0.2.0 adds Book (every "Book of …" card), Pancake (Classic+ #12, #13 and the eight Pancake
    /// tokens) and AI (the ten AI generated cards); the v0.2.x mechanics patch adds Plague (every card
    /// that uses Plague Counters); patch v0.2.Y adds Catalyst (Classic+ #38 Solarius and #46 Felinor
    /// Flagbearer), Prime (their Prime tokens, Classic+ #38.1 Solarius Prime and #46.1 Felinor
    /// Flagbearer Prime) and Acclaimed (Classic #80 BOOM! Big Max and Classic+ #37 Wardrum); the
    /// Meditative set adds Wincon (its #8 Reach the Summit and #20 Aestheticize the Game, which win the
    /// game another way, R1420's set).
    pub enum Tag {
        Human = "Human",
        Felinor = "Felinor",
        Ky = "KY",
        Cn = "CN",
        Fruit = "Fruit",
        CallToChaos = "Call to Chaos",
        Quickdraw = "Quickdraw",
        Jlockeed = "Jlockeed",
        Book = "Book",
        Pancake = "Pancake",
        Ai = "AI",
        Plague = "Plague",
        Catalyst = "Catalyst",
        Prime = "Prime",
        Acclaimed = "Acclaimed",
        Wincon = "Wincon",
        Token = "Token",
    }
}

string_union! {
    /// §8: Core's by mechanical complexity, Classic's and Classic+'s the designer's; every token carries "Token".
    pub enum Rarity {
        Common = "Common",
        Rare = "Rare",
        Epic = "Epic",
        Legendary = "Legendary",
        Mythic = "Mythic",
        Token = "Token",
    }
}

string_union! {
    /// A rarity a card prints: every rarity but Token. A token's printed one is display only (B2.5).
    /// (TS `Exclude<Rarity, "Token">`.)
    pub enum PrintedRarity {
        Common = "Common",
        Rare = "Rare",
        Epic = "Epic",
        Legendary = "Legendary",
        Mythic = "Mythic",
    }
}

string_union! {
    /// §5: Core, Classic and Classic+ ship (R380); Meditative is in the catalog and ships with the
    /// last part of its patch (R1420); Boss and Boss-X are reserved.
    pub enum SetName {
        Core = "Core",
        Classic = "Classic",
        ClassicPlus = "Classic+",
        Meditative = "Meditative",
        Boss = "Boss",
        BossX = "Boss-X",
    }
}

/// The sets that ship, in catalog order. A pool that names no set draws from all of them (R380),
/// and only from them: a set the catalog holds that is not listed here is in no such pool, in no
/// deck and on no list a player reads until it is (R1420).
pub const SHIPPED_SETS: [SetName; 3] = [SetName::Core, SetName::Classic, SetName::ClassicPlus];

/// Every set the catalog orders, shipped or not, in catalog order (B2.2): a set keeps its place
/// when it ships, so a seeded pick over a pool replays the same before and after (R1420).
pub const CATALOG_SETS: [SetName; 4] = [
    SetName::Core,
    SetName::Classic,
    SetName::ClassicPlus,
    SetName::Meditative,
];

/// R1420: whether a set ships, i.e. whether `SHIPPED_SETS` lists it.
pub fn set_ships(set: SetName) -> bool {
    SHIPPED_SETS.contains(&set)
}

/// R1371: the newest set that ships, the last entry of `SHIPPED_SETS` (catalog order): Classic+
/// until the Meditative set ships, then whichever set ships after it. "More cards from the newest
/// set" (R1372, R1373) leans a random deck on it, and every caller asks this function at deal time
/// rather than naming a set. A set previewed under the testkit (R1420) is not shipped, so it is
/// never the newest: a test that leans on one names it.
pub fn newest_shipped_set() -> SetName {
    SHIPPED_SETS[SHIPPED_SETS.len() - 1]
}

/// §5: 0 to 6, 100 (Ceaseless Void), X, or "A embiggen B".
///
/// TS `number | "X" | { base: number; embiggen: number }`, serialised exactly so (a number, the
/// string `"X"`, or the object).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "../../../apps/web/src/wire/generated/",
        type = "number | \"X\" | { base: number, embiggen: number, }"
    )
)]
pub enum CardCost {
    /// A printed number.
    Fixed(i32),
    /// "X": the player chooses X (R348), or the card says what X is.
    X,
    /// "A embiggen B": pay `base`, or `embiggen` for the embiggened play (R81).
    Embiggen { base: i32, embiggen: i32 },
}

impl Serialize for CardCost {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Embiggen {
            base: i32,
            embiggen: i32,
        }
        match *self {
            CardCost::Fixed(n) => serializer.serialize_i32(n),
            CardCost::X => serializer.serialize_str("X"),
            CardCost::Embiggen { base, embiggen } => Embiggen { base, embiggen }.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for CardCost {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Fixed(i32),
            Text(String),
            Embiggen { base: i32, embiggen: i32 },
        }
        match Raw::deserialize(deserializer)? {
            Raw::Fixed(n) => Ok(CardCost::Fixed(n)),
            Raw::Text(text) if text == "X" => Ok(CardCost::X),
            Raw::Text(text) => Err(D::Error::custom(format!(
                "a card cost is a number, \"X\" or {{ base, embiggen }}, got {text:?}"
            ))),
            Raw::Embiggen { base, embiggen } => Ok(CardCost::Embiggen { base, embiggen }),
        }
    }
}

/// A unit keyword (§6.1). Armor and Lucky carry a number; Armor sums across sources (§10.4).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind")]
pub enum Keyword {
    Taunt,
    Rush,
    Charge,
    #[serde(rename = "First Strike")]
    FirstStrike,
    Poisonous,
    Lifesteal,
    Reborn,
    #[serde(rename = "Divine Shield")]
    DivineShield,
    Trample,
    Cleave,
    /// R346: its damage ignores Armor (§4.4 step 2), on a unit or on a spell.
    Pierce,
    Indestructible,
    Immutable,
    Stack,
    #[serde(rename = "Can't attack")]
    CantAttack,
    Armor {
        n: i32,
    },
    Lucky {
        n: i32,
    },
    /// R383: a Field Spell, Trap or Field Trap that steps into a unit zone as a Unit (B3.1).
    Animated,
    /// R383: animated at its controller's start of turn, back in its backrow zone at their cleanup.
    #[serde(rename = "Animated on your turn")]
    AnimatedOnYourTurn,
    /// R385: printed Brittle N — the count starts when the card enters the field (B3.3).
    Brittle {
        n: i32,
    },
    /// §4.4: a Spell its controller casts deals N more damage per hit (E6). Printed "Spell Damage +N".
    #[serde(rename = "Spell Damage")]
    SpellDamage {
        n: i32,
    },
    /// E35: a Spell can't target this and doesn't affect it.
    #[serde(rename = "Immune to Spells")]
    ImmuneToSpells,
    /// R636: a Unit may attack twice each turn.
    Windfury,
    /// R637: a card discarded from its owner's hand at the end of their turn. Not temporary mana (§2.3).
    Temporary,
    /// A Unit may attack and switch position in the same turn (R49).
    Deft,
}

string_union! {
    /// `Keyword["kind"]`.
    pub enum KeywordKind {
        Taunt = "Taunt",
        Rush = "Rush",
        Charge = "Charge",
        FirstStrike = "First Strike",
        Poisonous = "Poisonous",
        Lifesteal = "Lifesteal",
        Reborn = "Reborn",
        DivineShield = "Divine Shield",
        Trample = "Trample",
        Cleave = "Cleave",
        Pierce = "Pierce",
        Indestructible = "Indestructible",
        Immutable = "Immutable",
        Stack = "Stack",
        CantAttack = "Can't attack",
        Armor = "Armor",
        Lucky = "Lucky",
        Animated = "Animated",
        AnimatedOnYourTurn = "Animated on your turn",
        Brittle = "Brittle",
        SpellDamage = "Spell Damage",
        ImmuneToSpells = "Immune to Spells",
        Windfury = "Windfury",
        Temporary = "Temporary",
        Deft = "Deft",
    }
}

/// Every keyword kind. R21's random pool is the narrower list in engine config.
pub const KEYWORD_KINDS: &[KeywordKind] = KeywordKind::ALL;

impl Keyword {
    /// `keyword.kind`.
    pub fn kind(&self) -> KeywordKind {
        match self {
            Keyword::Taunt => KeywordKind::Taunt,
            Keyword::Rush => KeywordKind::Rush,
            Keyword::Charge => KeywordKind::Charge,
            Keyword::FirstStrike => KeywordKind::FirstStrike,
            Keyword::Poisonous => KeywordKind::Poisonous,
            Keyword::Lifesteal => KeywordKind::Lifesteal,
            Keyword::Reborn => KeywordKind::Reborn,
            Keyword::DivineShield => KeywordKind::DivineShield,
            Keyword::Trample => KeywordKind::Trample,
            Keyword::Cleave => KeywordKind::Cleave,
            Keyword::Pierce => KeywordKind::Pierce,
            Keyword::Indestructible => KeywordKind::Indestructible,
            Keyword::Immutable => KeywordKind::Immutable,
            Keyword::Stack => KeywordKind::Stack,
            Keyword::CantAttack => KeywordKind::CantAttack,
            Keyword::Armor { .. } => KeywordKind::Armor,
            Keyword::Lucky { .. } => KeywordKind::Lucky,
            Keyword::Animated => KeywordKind::Animated,
            Keyword::AnimatedOnYourTurn => KeywordKind::AnimatedOnYourTurn,
            Keyword::Brittle { .. } => KeywordKind::Brittle,
            Keyword::SpellDamage { .. } => KeywordKind::SpellDamage,
            Keyword::ImmuneToSpells => KeywordKind::ImmuneToSpells,
            Keyword::Windfury => KeywordKind::Windfury,
            Keyword::Temporary => KeywordKind::Temporary,
            Keyword::Deft => KeywordKind::Deft,
        }
    }

    /// `"n" in keyword ? keyword.n : undefined`: the number a numbered keyword carries.
    pub fn n(&self) -> Option<i32> {
        match *self {
            Keyword::Armor { n }
            | Keyword::Lucky { n }
            | Keyword::Brittle { n }
            | Keyword::SpellDamage { n } => Some(n),
            _ => None,
        }
    }

    /// The keyword of `kind` carrying `n` when the kind is numbered (`n` is ignored otherwise):
    /// TS's `{ kind, n }` / `{ kind }` literal.
    pub fn of_kind(kind: KeywordKind, n: i32) -> Keyword {
        match kind {
            KeywordKind::Taunt => Keyword::Taunt,
            KeywordKind::Rush => Keyword::Rush,
            KeywordKind::Charge => Keyword::Charge,
            KeywordKind::FirstStrike => Keyword::FirstStrike,
            KeywordKind::Poisonous => Keyword::Poisonous,
            KeywordKind::Lifesteal => Keyword::Lifesteal,
            KeywordKind::Reborn => Keyword::Reborn,
            KeywordKind::DivineShield => Keyword::DivineShield,
            KeywordKind::Trample => Keyword::Trample,
            KeywordKind::Cleave => Keyword::Cleave,
            KeywordKind::Pierce => Keyword::Pierce,
            KeywordKind::Indestructible => Keyword::Indestructible,
            KeywordKind::Immutable => Keyword::Immutable,
            KeywordKind::Stack => Keyword::Stack,
            KeywordKind::CantAttack => Keyword::CantAttack,
            KeywordKind::Armor => Keyword::Armor { n },
            KeywordKind::Lucky => Keyword::Lucky { n },
            KeywordKind::Animated => Keyword::Animated,
            KeywordKind::AnimatedOnYourTurn => Keyword::AnimatedOnYourTurn,
            KeywordKind::Brittle => Keyword::Brittle { n },
            KeywordKind::SpellDamage => Keyword::SpellDamage { n },
            KeywordKind::ImmuneToSpells => Keyword::ImmuneToSpells,
            KeywordKind::Windfury => Keyword::Windfury,
            KeywordKind::Temporary => Keyword::Temporary,
            KeywordKind::Deft => Keyword::Deft,
        }
    }
}

pub fn keyword_key(keyword: &Keyword) -> String {
    match keyword.n() {
        Some(n) => format!("{} {}", keyword.kind().as_str(), n),
        None => keyword.kind().as_str().to_string(),
    }
}

pub fn has_keyword(keywords: &[Keyword], kind: KeywordKind) -> bool {
    keywords.iter().any(|k| k.kind() == kind)
}

/// Total Armor across every source (§10.4).
pub fn armor_of(keywords: &[Keyword]) -> i32 {
    keywords.iter().fold(0, |sum, k| match k {
        Keyword::Armor { n } => sum + n,
        _ => sum,
    })
}

/// `{ attack: number; health: number }`: the shape of `CardFace.xStats`, `CardInstance.buffs` and
/// `CardInstance.statsOverride` (TS writes it inline each time).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AttackHealth {
    pub attack: i32,
    pub health: i32,
}

/// One side of a card: the base form or the radiant form (§5). Spells have no stats; an Animated
/// backrow card (B3.1) prints the attack and health of the Unit it becomes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardFace {
    /// B2.7: the face's own type, when it differs from the card's (Classic+ #22 Blood Moon's Radiant
    /// face is a Field Trap). The card's type is its running face's (§5.2). Absent: the card's `type`.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<CardType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub health: Option<i32>,
    /// B2.7: "[3X/3X]" stats (Classic+ #69 Buff Billy): the Unit is summoned with `statsOverride` of
    /// these multiples of the X it was played for. The printed `attack`/`health` are then 0/0, as the
    /// Ghoul Token's are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub x_stats: Option<AttackHealth>,
    pub keywords: Vec<Keyword>,
    /// The face's printed text: the base face's §8 cell, or the Radiant face's cell read by §8's
    /// Conventions and written out in full (R277), so a client can print it whole and mark what differs.
    /// A tunable number (`CardDef.params`, B3.4) is written `{key}`, filled in by `fill_params`.
    pub text: String,
}

string_union! {
    /// Which way a declared number is better for the card's controller (`Param.better`).
    pub enum ParamBetter {
        Up = "up",
        Down = "down",
    }
}

string_union! {
    /// `Param.tunedOn`, the one face a tuning may move a number on: the Radiant face for a number
    /// only it prints (R749), the base face for a number only it prints (R1431).
    pub enum ParamTunedOn {
        Radiant = "radiant",
        Base = "base",
    }
}

string_union! {
    /// `"base" | "radiant"`: which face of a card (`fillParams`'s `face`, and every reader that
    /// indexes `def[face]`).
    pub enum FaceKind {
        Base = "base",
        Radiant = "radiant",
    }
}

/// B3.4 rule 5: a number on a card that Degrade, Upgrade and KY's Constant may move. The face texts
/// write it as `{key}`; the view carries an instance's current values; scripts read `param(ctx, key)`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Param {
    /// The name the texts write as `{key}`, unique within the card.
    pub key: String,
    /// Its printed value on the base face.
    pub base: i32,
    /// Its printed value on the Radiant face.
    pub radiant: i32,
    /// Which way is better for the card's controller: an Upgrade moves it this way, a Degrade the other.
    pub better: ParamBetter,
    /// How far one Degrade or Upgrade moves it (B3.4: 1 up to 5, 2 for 6–12, a quarter above).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub step: Option<i32>,
    /// It never goes below this (an amount never drops below 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<i32>,
    /// It never goes above this (100 for a percentage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<i32>,
    /// R749, R1431: the one face a Degrade, an Upgrade or KY's Constant may move it on, for a number only
    /// that face prints; on the other face it always reads its printed value. Absent, both faces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tuned_on: Option<ParamTunedOn>,
    /// R1430: the power the number belongs to, named by the id of the card's Activate ability that is
    /// that power (#98's stored power name, R103, R752). A Degrade, an Upgrade or KY's Constant reaches
    /// it only while the card has that ability (`ActivationDecl.has`); while it has another, the number
    /// keeps whatever tuning it has, so the power brings it back. Absent, the card's number whatever
    /// power it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub power: Option<String>,
}

impl Param {
    /// `param[face]`: its printed value on that face.
    pub fn on(&self, face: FaceKind) -> i32 {
        match face {
            FaceKind::Base => self.base,
            FaceKind::Radiant => self.radiant,
        }
    }
}

/// A tunable number in a face's text (B3.4 rule 5, R482): `{key}` is the number alone ("Deal {damage}
/// damage."); `{key|singular|plural}` is the number and the words that agree with it ("Draw
/// {draw|card|cards}." prints "Draw 1 card." and "Draw 2 cards."), so a text reads right at every
/// value a Degrade or an Upgrade can move it to.
///
/// The TS regular expression, kept as text for the record: `param_placeholders` and `fill_params`
/// scan for exactly what it matches (there is no regex crate in the pure crates, SURFACE §3).
pub const PARAM_PLACEHOLDER: &str = r"\{([A-Za-z][A-Za-z0-9]*)(?:\|([^|{}]*)\|([^|{}]*))?\}";

/// One placeholder `paramPlaceholders` found: its key and, for the agreeing form, both wordings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ParamPlaceholder {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub one: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub many: Option<String>,
}

/// One match of `PARAM_PLACEHOLDER` at byte `start` of `text`: its byte span and its groups.
struct PlaceholderMatch<'t> {
    end: usize,
    key: &'t str,
    words: Option<(&'t str, &'t str)>,
}

/// `PARAM_PLACEHOLDER` anchored at `start` (which holds `{`). The key class excludes `|` and `}`, so
/// the key is maximal and needs no backtracking; the agreeing group's two classes exclude `|{}`.
fn placeholder_at(text: &str, start: usize) -> Option<PlaceholderMatch<'_>> {
    let bytes = text.as_bytes();
    let mut at = start + 1;
    if !bytes.get(at).is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    let key_start = at;
    at += 1;
    while bytes.get(at).is_some_and(u8::is_ascii_alphanumeric) {
        at += 1;
    }
    let key = &text[key_start..at];
    match bytes.get(at) {
        Some(b'}') => Some(PlaceholderMatch {
            end: at + 1,
            key,
            words: None,
        }),
        Some(b'|') => {
            let word = |from: usize| {
                let mut to = from;
                while bytes.get(to).is_some_and(|b| !matches!(b, b'|' | b'{' | b'}')) {
                    to += 1;
                }
                to
            };
            let one_start = at + 1;
            let one_end = word(one_start);
            if bytes.get(one_end) != Some(&b'|') {
                return None;
            }
            let many_start = one_end + 1;
            let many_end = word(many_start);
            if bytes.get(many_end) != Some(&b'}') {
                return None;
            }
            Some(PlaceholderMatch {
                end: many_end + 1,
                key,
                words: Some((&text[one_start..one_end], &text[many_start..many_end])),
            })
        }
        _ => None,
    }
}

/// Every non-overlapping match, left to right, as `String.prototype.matchAll` with the `g` flag finds them.
fn placeholders(text: &str) -> Vec<(usize, PlaceholderMatch<'_>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(offset) = text[at..].find('{') {
        let start = at + offset;
        match placeholder_at(text, start) {
            Some(found) => {
                at = found.end;
                out.push((start, found));
            }
            None => at = start + 1,
        }
    }
    out
}

/// Every placeholder a text writes, in order: its key and, for the agreeing form, both wordings.
pub fn param_placeholders(text: &str) -> Vec<ParamPlaceholder> {
    placeholders(text)
        .into_iter()
        .map(|(_, found)| match found.words {
            None => ParamPlaceholder {
                key: found.key.to_string(),
                one: None,
                many: None,
            },
            Some((one, many)) => ParamPlaceholder {
                key: found.key.to_string(),
                one: Some(one.to_string()),
                many: Some(many.to_string()),
            },
        })
        .collect()
}

/// A face's text with its placeholders filled in: from `values` when given (an instance's current
/// numbers), else from the face's printed values; `{key|singular|plural}` takes the singular wording
/// at 1 and the plural at any other value. Unknown keys are left as written. Pure, so the client, the
/// tests and R277's diff all fill a text the same way.
pub fn fill_params(def: &CardDef, face: FaceKind, values: Option<&IndexMap<String, i32>>) -> String {
    let text = match face {
        FaceKind::Base => &def.base.text,
        FaceKind::Radiant => &def.radiant.text,
    };
    let params = match &def.params {
        Some(params) if !params.is_empty() => params,
        _ => return text.clone(),
    };
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for (start, found) in placeholders(text) {
        out.push_str(&text[copied..start]);
        copied = found.end;
        let Some(param) = params.iter().find(|p| p.key == found.key) else {
            out.push_str(&text[start..found.end]);
            continue;
        };
        let value = values
            .and_then(|v| v.get(found.key).copied())
            .unwrap_or_else(|| param.on(face));
        match found.words {
            None => out.push_str(&value.to_string()),
            Some((one, many)) => {
                out.push_str(&value.to_string());
                out.push(' ');
                out.push_str(if value == 1 { one } else { many });
            }
        }
    }
    out.push_str(&text[copied..]);
    out
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardDef {
    /// Catalog id, e.g. "core-043"; transient defs (Fuse, Craft a Card) use "t-<n>".
    pub id: String,
    /// §5: "43", token "51.1", shared token "T-rush".
    pub index: String,
    pub name: String,
    pub set: SetName,
    #[serde(rename = "type")]
    pub type_: CardType,
    pub tags: Vec<Tag>,
    pub rarity: Rarity,
    /// B2.5: the rarity a token prints (the Classic+ tokens the designer rated), for the card frame and
    /// the summon sting only. A token's `rarity` stays "Token", so no pool ever finds one by rarity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub printed_rarity: Option<PrintedRarity>,
    pub token: bool,
    pub cost: CardCost,
    /// R279: the cards and tokens this card's text names, by id — a name in its base or Radiant text,
    /// alone or plural, or a name before its parenthesis (#95's "Call to Chaos"). A client links each
    /// such name to the card it names. Absent when the text names none. A fused definition's is the
    /// union of its ingredients' (R102).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refs: Option<Vec<String>>,
    /// B3.4 rule 5: the numbers on this card Degrade, Upgrade and KY's Constant may move.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<Vec<Param>>,
    /// E36: the non-blank, non-comment lines of this card's script file, imports excluded. Frozen
    /// gameplay data since v0.3.0 (SURFACE §7.5): C+ #44, C+ #45 and Classic #48 read it. Public (the
    /// inspect overlay prints it) and part of the card's patch history (B4.2). Absent while the card
    /// has no script.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub loc: Option<i32>,
    /// R349: this card prints no Radiant form of its own (the Ghoul Token, §7). Its `radiant` face is
    /// the fallback the rule gives it — the base face with its attack and health doubled, the same
    /// keywords and text — and a summon's X/X (`statsOverride`) doubles with it at runtime. Absent on
    /// every card that prints a Radiant form, a fused definition included (R77 sums the ingredients'
    /// Radiant forms). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub radiant_fallback: Option<bool>,
    /// R179, R468, R469: a fused definition's ingredients, in ingredient order — the definition each
    /// was, and `radiant` when it went into both of the fused forms on its Radiant face ("fuse a random
    /// Radiant card"). Only a Fuse writes it. While the list is short the id spells it out too; past
    /// `FUSED_ID_CAP` the id is a digest of it, and this list is what rebuilds the scripts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ingredients: Option<Vec<FusedIngredient>>,
    /// R882: a crafted definition's recipe (Meditative #17 True Craft a Card) — what its scripts
    /// rebuild from, as `ingredients` is for a fused one. Only the crafter writes it. Optional and
    /// skipped when absent, so a game that never crafts one serialises, hashes and replays as
    /// before (D14: no golden trace moves).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub craft: Option<crate::wire::craft::CraftRecipe>,
    pub base: CardFace,
    pub radiant: CardFace,
}

impl CardDef {
    /// `def[face]`.
    pub fn face(&self, face: FaceKind) -> &CardFace {
        match face {
            FaceKind::Base => &self.base,
            FaceKind::Radiant => &self.radiant,
        }
    }
}

/// R179, R469: one ingredient of a fused definition (`CardDef.ingredients`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FusedIngredient {
    pub def_id: String,
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub radiant: Option<bool>,
}

/// `Readonly<Record<string, CardDef>>`, by catalog id, in catalog order.
pub type CardDefs = IndexMap<String, CardDef>;

/// TS `T | T[]`: a query or filter field that takes one value or several.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T: PartialEq> OneOrMany<T> {
    /// The values as a list (`[x].flat()`).
    pub fn as_slice(&self) -> &[T] {
        match self {
            OneOrMany::One(one) => std::slice::from_ref(one),
            OneOrMany::Many(many) => many,
        }
    }

    /// Whether `value` is the one value or among the several (`[x].flat().includes(value)`).
    pub fn includes(&self, value: &T) -> bool {
        self.as_slice().contains(value)
    }
}

/// `{ min?: number; max?: number }`: `CatalogQuery.costRange` and `TargetFilter.costRange`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CostRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<i32>,
}

/// §5.1: the one query every random pool and Discover goes through. Every field narrows; `{}` is every
/// non-token card of every set (R380: a pool that names no set draws from all of them).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CatalogQuery {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost_range: Option<CostRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub not_tags: Option<Vec<Tag>>,
    /// R1422: has at least one of these tags ("a random Human, Book, CN, or AI-Generated card"), where
    /// `tags` asks for every one of its tags. A Fruit or Prime tag here takes its tokens as `tags`
    /// does (R382, R1421).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub any_tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rarity: Option<OneOrMany<Rarity>>,
    /// A set, or several ("Classic or Classic+"). Absent is every set that ships (R380, R1420).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<OneOrMany<SetName>>,
    /// R387: never these definitions, by catalog id — a card's own id, so it never generates itself
    /// (§5.1, B4.1). An index is unique only within its set, so a pool never excludes by index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub exclude_def_id: Option<OneOrMany<String>>,
    /// R382: tokens may come out of this pool beside the cards — Dropshipping's "(including tokens)".
    /// Without it a pool holds no token, except that a Fruit pool holds the Grapes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub with_tokens: Option<bool>,
}

string_union! {
    /// §10.6. E18 adds: `number` (a number from a fixed range, Classic #18), `answer` (one of a
    /// multiple-choice problem's options, Classic+ #42), `cell` (a board cell, Classic+ #62), `reward`
    /// (a completed quest's reward, Classic #90) and `pick` (a budgeted pick of several cards from a pile,
    /// Classic #44). A mode prompt the other player holds is a `mode` prompt with their id.
    pub enum PromptKind {
        Discover = "discover",
        Target = "target",
        Mode = "mode",
        Mulligan = "mulligan",
        Hand = "hand",
        Zone = "zone",
        Tribute = "tribute",
        Direction = "direction",
        X = "x",
        Embiggen = "embiggen",
        Number = "number",
        Answer = "answer",
        Cell = "cell",
        Reward = "reward",
        Pick = "pick",
        /// ME-CRAFT (Meditative #17, R880): the block editor's answer, a `Selection::Craft`.
        Craft = "craft",
    }
}

string_union! {
    /// A field row (§3).
    pub enum Row {
        Units = "units",
        Backrow = "backrow",
    }
}

string_union! {
    /// `Zone["z"]`: the name of a zone, without its side, row or lane.
    pub enum ZoneName {
        Hand = "hand",
        Library = "library",
        Graveyard = "graveyard",
        Exile = "exile",
        Field = "field",
        Resolving = "resolving",
        Gone = "gone",
    }
}

/// A zone a card can sit in. Field zones name a side, a row and a lane (§3).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "z", rename_all = "camelCase")]
pub enum Zone {
    Hand {
        player: PlayerId,
    },
    Library {
        player: PlayerId,
    },
    Graveyard {
        player: PlayerId,
    },
    Exile {
        player: PlayerId,
    },
    Field {
        player: PlayerId,
        row: Row,
        lane: i32,
    },
    Resolving {
        player: PlayerId,
    },
    /// R11: a unit token that left the field, or any card that ceased to exist (R86).
    Gone {
        player: PlayerId,
    },
}

impl Zone {
    /// `zone.z`.
    pub fn z(&self) -> ZoneName {
        match self {
            Zone::Hand { .. } => ZoneName::Hand,
            Zone::Library { .. } => ZoneName::Library,
            Zone::Graveyard { .. } => ZoneName::Graveyard,
            Zone::Exile { .. } => ZoneName::Exile,
            Zone::Field { .. } => ZoneName::Field,
            Zone::Resolving { .. } => ZoneName::Resolving,
            Zone::Gone { .. } => ZoneName::Gone,
        }
    }

    /// `zone.player`.
    pub fn player(&self) -> PlayerId {
        match *self {
            Zone::Hand { player }
            | Zone::Library { player }
            | Zone::Graveyard { player }
            | Zone::Exile { player }
            | Zone::Field { player, .. }
            | Zone::Resolving { player }
            | Zone::Gone { player } => player,
        }
    }

    /// The pile zone `{ z, player }` for a name other than "field" (which needs a row and a lane:
    /// `None`).
    pub fn pile(z: ZoneName, player: PlayerId) -> Option<Zone> {
        match z {
            ZoneName::Hand => Some(Zone::Hand { player }),
            ZoneName::Library => Some(Zone::Library { player }),
            ZoneName::Graveyard => Some(Zone::Graveyard { player }),
            ZoneName::Exile => Some(Zone::Exile { player }),
            ZoneName::Resolving => Some(Zone::Resolving { player }),
            ZoneName::Gone => Some(Zone::Gone { player }),
            ZoneName::Field => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ZoneRef {
    pub player: PlayerId,
    pub row: Row,
    pub lane: i32,
}

string_union! {
    /// `TargetFilter.side`.
    pub enum FilterSide {
        Ally = "ally",
        Enemy = "enemy",
        Any = "any",
    }
}

string_union! {
    /// `TargetFilter.of`: what kind of thing a pick may be.
    pub enum FilterOf {
        Unit = "unit",
        Hero = "hero",
        Backrow = "backrow",
        Hand = "hand",
        Zone = "zone",
        Graveyard = "graveyard",
    }
}

/// Which cards a declared choice may pick (§10.6, R81).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TargetFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<FilterSide>,
    /// `graveyard`: a card in a graveyard on the named side (Classic #54's "on the field or in your graveyard").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub of: Option<Vec<FilterOf>>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub not_tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub exclude_self: Option<bool>,
    /// The card's cost as R65 reads it where it is now (a hand card at its hand cost).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost_range: Option<CostRange>,
    /// A unit with damage above 0 (Classic+ #32.1 Execute).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub damaged: Option<bool>,
    /// A card with at least one Plague Counter on it (Classic #78 Mutate Spell).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plague: Option<bool>,
    /// The name of a predicate in the declaring script's `targetChecks`, for a filter no field above
    /// can say (Classic #32's lane rule, #48's lines of code). Data, so a declaration stays JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub check: Option<String>,
}

string_union! {
    /// `TargetDecl.aim`: whether the pick is beneficial ("help") or harmful ("harm").
    pub enum TargetAim {
        Harm = "harm",
        Help = "help",
    }
}

/// What a card asks for as part of its own play (R81).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TargetDecl {
    /// `Extract<PromptKind, "target" | "hand" | "zone" | "tribute">`.
    #[cfg_attr(feature = "ts", ts(type = "\"target\" | \"hand\" | \"zone\" | \"tribute\""))]
    pub kind: PromptKind,
    pub min: i32,
    pub max: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<TargetFilter>,
    /// For Tribute: how many tributes the play costs (Sheep Tokens count 2, §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub amount: Option<i32>,
    /// The modes this declaration belongs to, when it belongs to some only: the play asks for it only
    /// when one of its chosen modes is listed here, and takes nothing for it otherwise (R90). #24
    /// Efficiency Dividend's target is its damage and heal modes' — "deal X damage to a target; heal a
    /// target 2X" — and its mana mode names none (§8 Conventions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub for_modes: Option<Vec<String>>,
    /// Whether the target pick is beneficial ("help") or harmful ("harm").
    /// Defaults to "harm". Used by random casts with `targetEnemies` to aim at
    /// friendly targets when beneficial and enemies when harmful (R656).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub aim: Option<TargetAim>,
    /// The play needs this pick: while the board offers fewer than `min` options for it, the play is
    /// refused and `legalActions` never offers it, where R90 would let it play and fizzle (R703, #63
    /// Plastic Surgery). A cast is never refused (R70), so a cast with no option still fizzles.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub required: Option<bool>,
}

impl TargetDecl {
    /// A declaration of `kind` with `min`/`max` and the filter written as the TS object literal
    /// (`json!({ "side": "enemy", "of": ["unit"] })`; `Value::Null` for none). Panics, naming the
    /// JSON, when the literal is not a `TargetFilter`: a card file's mistake, found by its tests.
    pub fn new(kind: PromptKind, min: i32, max: i32, filter: serde_json::Value) -> TargetDecl {
        let filter = if filter.is_null() {
            None
        } else {
            match serde_json::from_value::<TargetFilter>(filter.clone()) {
                Ok(parsed) => Some(parsed),
                Err(error) => panic!("not a TargetFilter ({error}): {filter}"),
            }
        };
        TargetDecl {
            kind,
            min,
            max,
            filter,
            amount: None,
            for_modes: None,
            aim: None,
            required: None,
        }
    }

    /// `{ kind: "target", min, max, filter }` (SURFACE §7.1).
    pub fn target(min: i32, max: i32, filter: serde_json::Value) -> TargetDecl {
        TargetDecl::new(PromptKind::Target, min, max, filter)
    }

    /// `{ kind: "hand", min, max, filter }`.
    pub fn hand(min: i32, max: i32, filter: serde_json::Value) -> TargetDecl {
        TargetDecl::new(PromptKind::Hand, min, max, filter)
    }

    /// `{ kind: "zone", min, max, filter }`.
    pub fn zone(min: i32, max: i32, filter: serde_json::Value) -> TargetDecl {
        TargetDecl::new(PromptKind::Zone, min, max, filter)
    }

    /// `{ kind: "tribute", min, max, filter }`.
    pub fn tribute(min: i32, max: i32, filter: serde_json::Value) -> TargetDecl {
        TargetDecl::new(PromptKind::Tribute, min, max, filter)
    }
}

/// A choice among fixed options that travels in the play (R81): a mode, a direction, or E18's number
/// (Classic #18's 0 to 10, whose options are the numbers themselves).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ModeDecl {
    /// `Extract<PromptKind, "mode" | "direction" | "number">`.
    #[cfg_attr(feature = "ts", ts(type = "\"mode\" | \"direction\" | \"number\""))]
    pub kind: PromptKind,
    pub options: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def_with(text: &str, params: Vec<Param>) -> CardDef {
        let face = CardFace {
            type_: None,
            attack: None,
            health: None,
            x_stats: None,
            keywords: vec![],
            text: text.into(),
        };
        CardDef {
            id: "core-999".into(),
            index: "999".into(),
            name: "Test".into(),
            set: SetName::Core,
            type_: CardType::Spell,
            tags: vec![],
            rarity: Rarity::Common,
            printed_rarity: None,
            token: false,
            cost: CardCost::Fixed(1),
            refs: None,
            params: Some(params),
            loc: None,
            radiant_fallback: None,
            ingredients: None,
            craft: None,
            base: face.clone(),
            radiant: face,
        }
    }

    fn param(key: &str, base: i32, radiant: i32) -> Param {
        Param {
            key: key.into(),
            base,
            radiant,
            better: ParamBetter::Up,
            step: None,
            min: None,
            max: None,
            tuned_on: None,
            power: None,
        }
    }

    #[test]
    fn fill_params_fills_numbers_and_agreeing_words() {
        let def = def_with(
            "Draw {draw|card|cards}. Deal {damage} damage. {nope} {x|a}",
            vec![param("draw", 1, 2), param("damage", 3, 6)],
        );
        assert_eq!(
            fill_params(&def, FaceKind::Base, None),
            "Draw 1 card. Deal 3 damage. {nope} {x|a}"
        );
        assert_eq!(
            fill_params(&def, FaceKind::Radiant, None),
            "Draw 2 cards. Deal 6 damage. {nope} {x|a}"
        );
        let values: IndexMap<String, i32> = [("damage".to_string(), 9)].into_iter().collect();
        assert_eq!(
            fill_params(&def, FaceKind::Base, Some(&values)),
            "Draw 1 card. Deal 9 damage. {nope} {x|a}"
        );
    }

    #[test]
    fn param_placeholders_reads_every_match_in_order() {
        let found = param_placeholders("{a|b{c} {d1} {e|f|g} {9x}");
        assert_eq!(
            found,
            vec![
                ParamPlaceholder {
                    key: "c".into(),
                    one: None,
                    many: None
                },
                ParamPlaceholder {
                    key: "d1".into(),
                    one: None,
                    many: None
                },
                ParamPlaceholder {
                    key: "e".into(),
                    one: Some("f".into()),
                    many: Some("g".into())
                },
            ]
        );
    }

    #[test]
    fn card_costs_and_keywords_round_trip_as_ts_writes_them() {
        for (json, cost) in [
            ("3", CardCost::Fixed(3)),
            ("\"X\"", CardCost::X),
            (
                "{\"base\":2,\"embiggen\":4}",
                CardCost::Embiggen { base: 2, embiggen: 4 },
            ),
        ] {
            assert_eq!(serde_json::from_str::<CardCost>(json).unwrap(), cost);
            assert_eq!(serde_json::to_string(&cost).unwrap(), json);
        }
        let armor: Keyword = serde_json::from_str("{\"kind\":\"Armor\",\"n\":2}").unwrap();
        assert_eq!(armor, Keyword::Armor { n: 2 });
        assert_eq!(keyword_key(&armor), "Armor 2");
        assert_eq!(
            serde_json::to_string(&Keyword::CantAttack).unwrap(),
            "{\"kind\":\"Can't attack\"}"
        );
        assert_eq!(serde_json::to_string(&PlayerId::P2).unwrap(), "\"p2\"");
        let zone = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 3,
        };
        assert_eq!(
            serde_json::to_string(&zone).unwrap(),
            "{\"z\":\"field\",\"player\":\"p1\",\"row\":\"units\",\"lane\":3}"
        );
    }
}
