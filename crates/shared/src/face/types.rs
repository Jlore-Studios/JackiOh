//! The face model's types: what a host hands in (`FaceSource`, `LiveCard`, `FaceCall`) and what it gets back
//! (`FaceModel`, the spans of its text), all serde and, behind the `ts` feature, ts-rs (README.md, host
//! contract 2). The shapes follow what the web's `cards/model.ts` computed before this crate did, so a
//! face reads as it always has.

use indexmap::IndexMap;
use jackioh_engine::wire::{
    CardDef, CardElement, CardMark, CardType, Enchantment, Keyword, KeywordKind, PreviewValue, PrintedRarity,
    QuestView, Rarity, SetName, Tag, Tuning,
};
use serde::{Deserialize, Serialize};

/// The reader's language (#711, R711000). A face is written in it: the words around the card (a state
/// badge, a glossary rule, a Heroic Power's text) in its dictionary, and, for a reader of Chinese, every
/// card's name and text from the Chinese table (R1301).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
pub enum Locale {
    #[default]
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

// ---- the spans a face's text is made of

/// Which way a moved number went for the card's controller (B3.4, R386).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum Way {
    Better,
    Worse,
}

/// One entry of the keyword glossary (SPEC §6), in the reader's words: what a term's tooltip and a card's
/// glossary list print.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntry {
    /// The term: a keyword kind, a status, a trigger or a verb.
    pub id: String,
    pub label: String,
    /// The rule in a player's words.
    pub rule: String,
    /// The spec section it states: "§5.2", "§6.1", "§6.2" or "§6.3".
    pub section: String,
    /// The name of its row in the spec's §6 table, where that is not `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub row: Option<String>,
}

/// A stretch of a face's text, final and in the reader's words. A client maps each kind to its own
/// markup (the web to elements, Roblox to RichText) and decides nothing: a span that wraps others
/// (`card`, `mark`) holds them in `children`, already nested.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Span {
    /// Plain words.
    Text { text: String },
    /// A glossary term, printed in bold, with its entry (R2098000). A moved number can stand inside one
    /// ("Armor 3"), so it holds spans like the others.
    Keyword {
        entry: GlossaryEntry,
        children: Vec<Span>,
    },
    /// A number the view moved off its printed value (B3.4, R386): `way` says whether that is better or
    /// worse for the card's controller, `printed` is the value it printed, and `title` the words that say so.
    Number {
        text: String,
        key: String,
        printed: i32,
        way: Way,
        title: String,
    },
    /// A name that points at another card (R279), printed face `radiant` or base.
    Card {
        def_id: String,
        radiant: bool,
        children: Vec<Span>,
    },
    /// Words a Radiant face prints that its base face does not (R277).
    Mark { children: Vec<Span> },
    /// What a card's formula comes to now (R280), after its label: `text` is how it prints ("{3}"), `value`
    /// the bare number or word, `title` the sentence that explains it.
    Value {
        text: String,
        value: String,
        label: String,
        title: String,
    },
}

/// Text with its spans: `full` is the plain words (what a client measures and keys on), `spans` the same
/// words as a face prints them, with the formula values of R280 where they stand.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RichText {
    pub full: String,
    pub spans: Vec<Span>,
    /// The same words as `spans` with no card reference in them, for a client that cannot find the cards
    /// `spans` names (a name it cannot link is plain text, and a term inside it is bold again). Absent when
    /// `spans` holds no reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unlinked: Option<Vec<Span>>,
}

// ---- what a host hands in

/// A card's definition by id, for the ones the catalog does not hold: a match-made card (R77, R102, R179).
pub type DefMap = IndexMap<String, CardDef>;

/// The power a #98 Heroic Power rolled (R43, R243), by the name the view gives it (R103).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RolledPower {
    pub name: String,
}

/// R2321000: a count a card on the field keeps on a Unit, by the name of the card keeping it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct NamedTally {
    pub name: String,
    pub value: i32,
    pub goal: i32,
}

/// R1852005, R2249000: one Starship Piece fused into a Starship, on the face it fused on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct StarshipPieceRef {
    pub def_id: String,
    pub radiant: bool,
}

/// B3.1, R383: a card standing in a unit zone as a Unit, and where an "Animated on your turn" card goes
/// back to (a backrow lane).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AnimatedFace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home: Option<i32>,
}

/// R1839000, R1839001: a Dormant card, `turns` its countdown or null for none.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DormantFace {
    pub turns: Option<i32>,
}

/// R1852000, R1852004: the card is a Starship: its piece count, the name it took as it launched (null
/// while Dormant), printed in place of its own, and its pieces, oldest first.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct StarshipIn {
    pub pieces: i32,
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub list: Option<Vec<StarshipPieceRef>>,
}

/// B5 E14, R399: the Spell a copier has the text of, with its definition and the numbers it reads.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CopiesIn {
    pub def: CardDef,
    pub radiant: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<IndexMap<String, i32>>,
}

/// A hand Unit's stats as they stand (R243, `CardView.attack` and `.health`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct HandStats {
    pub attack: i32,
    pub health: i32,
}

/// What a game adds to a face (R243); its presence is what makes a face one in play. Every field is what
/// the view says about the card (CLAUDE.md rule 7).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct InPlay {
    /// R243: a Unit card's stats in its owner's hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand_stats: Option<HandStats>,
    /// B5 E38, R1438: a hand card's keywords as the view gives them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand_keywords: Option<Vec<Keyword>>,
    /// R243: the unit's text is gone (`UnitView.vanilla`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vanilla: Option<bool>,
    /// R43, R243: the power a #98 Heroic Power rolled (R752).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub power: Option<RolledPower>,
    /// R2247000: the word the card shows in place of its text (`CardView.label`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// R280: what the card's formula comes to now (`CardView.preview`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preview: Option<Vec<PreviewValue>>,
    /// B3.4, R386: the card's declared numbers as they stand now (`CardView.params`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<IndexMap<String, i32>>,
    /// B3.3, R385: the card's Brittle count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub brittle: Option<i32>,
    /// #2437: the card lies in a hand, the one zone its Temporary counts in (R637).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub in_hand: Option<bool>,
    /// R980: the card's element, while a Feng Shui acts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub element: Option<CardElement>,
    /// B3.4, R386: what Degrade and Upgrade changed on the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tuning: Option<Tuning>,
    /// B5 E39: the enchantments riding the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub enchantments: Option<Vec<Enchantment>>,
    /// B3.1, R383: the card stands in a unit zone as a Unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub animated: Option<AnimatedFace>,
    /// B5 E35: the unit has gone Berserk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub berserk: Option<bool>,
    /// R1174000: the unit is Frozen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frozen: Option<bool>,
    /// R1839000, R1839001: the card is Dormant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dormant: Option<DormantFace>,
    /// R1848000: the unit is Frozen forever.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frozen_forever: Option<bool>,
    /// R1852000, R1852004: the card is a Starship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub starship: Option<StarshipIn>,
    /// MD-B6, R943: the card was minted after the decks were built.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created: Option<bool>,
    /// R1956001: the card that generated the viewer's own Created card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_by: Option<String>,
    /// R437: the marks on the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marks: Option<Vec<CardMark>>,
    /// B5 E33, R404: the card's quest line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quest: Option<QuestView>,
    /// B5 E14, R399: the Spell a copier has the text of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copies: Option<CopiesIn>,
    /// ME-GRANT, R1107: the Death abilities other cards granted the unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grants: Option<Vec<String>>,
    /// R849000: why the viewer's own unit cannot attack now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack_reason: Option<String>,
    /// R2321000: the counts cards on the field keep on the unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tallies: Option<Vec<NamedTally>>,
    /// R979000: why a card in the viewer's own hand (or, R1571000, graveyard) cannot be played now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub play_reason: Option<String>,
}

/// A unit's numbers as they stand (`UnitView`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LiveStats {
    pub attack: i32,
    pub health: i32,
    pub max_health: i32,
    pub keywords: Vec<Keyword>,
}

/// What a face is made from: a catalog definition, and, when the card is in a game, the view's account of
/// it. A face with no `in_play` is the card as printed (SPEC §10.10); with it, as the view says it stands.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceSource {
    pub def_id: String,
    /// The definition. Absent: the catalog's, else the view's match-made ones, else the card is drawn by its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub def: Option<CardDef>,
    /// Fallbacks when no definition is available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<CardType>,
    pub radiant: bool,
    /// `CardView.cost`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub live_cost: Option<i32>,
    /// R1181003: `CardView.embiggenCost`, what a play of the Embiggened version costs now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub live_embiggen_cost: Option<i32>,
    /// R1848003: `CardView.fixedX`, the X an X-cost card is set at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fixed_x: Option<i32>,
    /// `UnitView`'s current numbers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub live: Option<LiveStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub in_play: Option<InPlay>,
    /// ME-CN, R1301: the view says the card is Chinese (`CardView.chinese`), whatever the reader's language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub chinese: Option<bool>,
    /// MD-B15, R923: the view's tags where they differ from the definition's (a granted tag).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<Tag>>,
}

// ---- what a face is

/// How a cost reads against the printed one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum CostTone {
    Base,
    Down,
    Up,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceCost {
    /// What the gem shows.
    pub text: String,
    /// `data-cost`: the live number when there is one, else `text`.
    pub value: String,
    pub tone: CostTone,
    /// The embiggen price, shown small beside the gem while the gem shows the base price; null otherwise.
    pub alt: Option<String>,
}

/// How a stat reads against the printed one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum StatTone {
    Base,
    Buffed,
    Reduced,
    Damaged,
}

/// R277: on a printed Radiant face, which stats it raised over the base face's.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Grew {
    pub attack: bool,
    pub health: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceStats {
    pub attack: i32,
    pub health: i32,
    pub max_health: i32,
    pub attack_tone: StatTone,
    pub health_tone: StatTone,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grew: Option<Grew>,
}

/// One change Degrade, Upgrade or KY's Constant made to a card, in a player's words ("+2 Attack").
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TuneChange {
    Stat {
        stat: String,
        delta: i32,
        way: Way,
    },
    Keyword {
        keyword: String,
        added: bool,
        way: Way,
    },
    Count {
        key: String,
        delta: i32,
        way: Way,
    },
    Number {
        key: String,
        printed: i32,
        value: i32,
        way: Way,
    },
    Set {
        key: String,
        value: i32,
    },
}

/// The card as a whole: every change better, every change worse, or a mix (or a number set outright).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum TuneVerdict {
    Upgraded,
    Degraded,
    Tuned,
}

/// B3.4, R386: what changed on a card, from its view's `tuning` and `params` against its printed face.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceTuning {
    pub verdict: TuneVerdict,
    /// The glyph the verdict is drawn with ("▲", "▼", "◆"), so a mark never rests on color alone.
    pub glyph: String,
    /// The word the verdict is printed with, in the reader's language ("Buffed").
    pub word: String,
    pub changes: Vec<TuneChange>,
    /// Each change in words ("+2 Attack", "Gained Rush"), parallel to `changes`.
    pub change_words: Vec<String>,
    /// All of it in one sentence: "Buffed: +2 Attack; Gained Rush".
    pub summary: String,
    /// The stat pips: which way each stat was tuned; absent for a stat no change moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack: Option<Way>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub health: Option<Way>,
    /// Keywords a Buff added, which the face prints as "+" chips.
    pub added: Vec<Keyword>,
    /// Keyword kinds a Nerf removed, which the face prints as struck chips.
    pub removed: Vec<KeywordKind>,
}

/// A keyword chip a tuned face prints at the foot of its rules box: "+Rush", struck "−Taunt" (R386).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TuningChip {
    /// "+" for a keyword a Buff added, the minus sign for one a Nerf removed.
    pub sign: String,
    /// The keyword in the face's words.
    pub text: String,
    pub added: bool,
    /// The tooltip: "Gained Rush", "Lost Taunt".
    pub title: String,
}

/// One tag chip of the frame, in the face's words.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TagChip {
    /// The tag, or "All Tribes" for a card with every tribal tag (R1382).
    pub tag: String,
    pub text: String,
}

/// R1852005, R2249000: a Starship's pieces on the face shown.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct StarshipFace {
    pub pieces: i32,
    pub launched: bool,
    pub list: Vec<StarshipPieceRef>,
    /// The glossary entry the pieces list's head names (the Starship keyword).
    pub term: Option<GlossaryEntry>,
}

/// B5 E14, R511: the Spell a copier's text is now.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Copying {
    pub def_id: String,
    pub name: String,
    pub radiant: bool,
}

/// A card a face's text names (R279): the printed face the hover preview shows beside it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Mention {
    pub def_id: String,
    pub radiant: bool,
}

/// R2521009: what a Resource sells for on one face, its least and its most.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceCredits {
    pub min: i32,
    pub max: i32,
}

/// Which badge a state is (SPEC §10.8): the rail draws what ends the card first, then what it is doing,
/// then the rest.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum StateBadgeKind {
    Brittle,
    Temporary,
    Quest,
    QuestStage,
    Tuned,
    ReturnAfterResolve,
    CastOnDraw,
    TargetEnemies,
    SwapsBook,
    Animated,
    Berserk,
    Frozen,
    Dormant,
    Starship,
    Created,
}

/// The icons a badge may draw, by the name the web's icon set gives them.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum BadgeIcon {
    Brittle,
    ReturnHand,
    CastOnDraw,
    Target,
    Cog,
    Sword,
}

/// A per-card state a face draws beside its printed text (SPEC §10.8): a badge and an inspect line.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct StateBadge {
    pub kind: StateBadgeKind,
    /// What the badge shows: a count or a glyph; null for an icon alone.
    pub text: Option<String>,
    /// The icon drawn under the text, when the badge has one.
    pub icon: Option<BadgeIcon>,
    /// The tooltip, the accessible name and the inspect overlay's line.
    pub words: String,
    /// `data-*` values the badge carries for tests and styles.
    pub data: IndexMap<String, String>,
}

/// R2450000: what a Quest card's rules box prints, and whether it is less than the whole text.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestBox {
    pub text: RichText,
    /// A staged Quest's one stage is less than the whole text.
    pub less: bool,
}

/// What a card face prints (docs/polish/6-cards.md, Surface B), final and in the reader's words, computed
/// from a catalog definition and the view. A client lays it out and decides nothing.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceModel {
    pub def_id: String,
    /// False when no catalog def was available (the unknown-card fallback).
    pub known: bool,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: CardType,
    pub tags: Vec<Tag>,
    pub rarity: Option<Rarity>,
    /// B2.5, R503: the rarity a token prints, which its frame shows in place of Token's. Display only.
    pub printed_rarity: Option<PrintedRarity>,
    /// R1177001: the card is a token.
    pub token: bool,
    pub index: Option<String>,
    pub set: Option<SetName>,
    pub radiant: bool,
    pub cost: FaceCost,
    /// Units only: live when `live` was given, else the printed face; null for non-units and unknown stats.
    pub stats: Option<FaceStats>,
    /// What the rules box prints in full; on a Radiant face the stretches the base text lacks are `mark`s.
    pub text: RichText,
    /// A short form of the text, for a box that cannot hold it (issues #814, #1017): a card's own short
    /// form, or a Quest's one stage (R2450000). Absent when the face prints its whole text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub short: Option<RichText>,
    /// The Summit's header and next Ascent line (#814), for a board that gives its level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub summit: Option<RichText>,
    /// The Summit's Ascent level as the view gives it (`CardView.preview`, R280).
    pub ascent: Option<i32>,
    /// The face's text is longer than any box holds, so it clamps (#1017): its short form is the whole text.
    pub clamp: bool,
    /// The card has a short form for this face (issues #814, #1017), whether or not it cuts anything.
    pub has_short_form: bool,
    /// R2450000: a Quest card's rules box, or null for a face that prints its text as it is.
    pub quest_box: Option<QuestBox>,
    /// R279: the cards and tokens this card's text names (`CardDef.refs`).
    pub refs: Vec<String>,
    /// R279: the cards the text names, in the order it first names them, each once per face and the card
    /// itself left out, since the hover preview already shows it. The preview's column of named cards.
    pub mentions: Vec<Mention>,
    /// R280: in play, what the card's formula comes to now, each printed in braces after its label.
    pub values: Vec<PreviewValue>,
    /// Live keywords when `live` was given, a hand card's as the view gives them, else the printed face's.
    pub keywords: Vec<Keyword>,
    /// Each of `keywords` in the face's words ("Armor 2", on a Chinese face "护甲2").
    pub keyword_words: Vec<String>,
    /// What joins a list of keywords in the face's words: ", ", or "，" on a Chinese face.
    pub keyword_join: String,
    pub in_play: bool,
    /// R243, §6.3 Vanilla: the unit's text is gone.
    pub vanilla: bool,
    /// Keywords the card has now that its printed face does not print, printed after its text.
    pub gained: Vec<Keyword>,
    /// `gained` as the rules box prints it ("Rush, Taunt"), with the terms in bold.
    pub gained_line: RichText,
    /// ME-GRANT, R1107: the Death abilities other cards granted the unit, each on its own line.
    pub grants: Vec<RichText>,
    /// The collection's text for this face when the face in play prints something else.
    pub printed: Option<RichText>,
    pub brittle: Option<i32>,
    pub temporary: Option<i32>,
    pub element: Option<CardElement>,
    pub tuning: Option<FaceTuning>,
    /// The keyword chips a tuned face prints at the foot of its box.
    pub tuning_chips: Vec<TuningChip>,
    pub enchantments: Vec<Enchantment>,
    pub animated: Option<AnimatedFace>,
    pub berserk: bool,
    pub frozen: bool,
    pub dormant: Option<DormantFace>,
    pub frozen_forever: bool,
    pub starship: Option<StarshipFace>,
    /// R3337001: the code of the board an Anomaly Found! in play holds (its label, R2247000), whose
    /// picture its art draws; absent for any other card and for a printed face.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub anomaly: Option<String>,
    pub created: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_by: Option<String>,
    /// E36: the lines of code of the card's script.
    pub loc: Option<i32>,
    /// R2521009: a Resource's sale prices.
    pub credits: Option<FaceCredits>,
    /// R2521009: "Credits: 2–5"; empty on any other face.
    pub credits_line: String,
    pub marks: Vec<CardMark>,
    pub quest: Option<QuestView>,
    pub copying: Option<Copying>,
    /// The type line in the face's words: "Unit", "Token Unit", on a Chinese face "衍生物单位" (R1177001).
    pub type_line: String,
    /// The tags the frame prints, in the face's words, "All Tribes" in place of every tribal tag (R1382).
    pub tag_chips: Vec<TagChip>,
    /// The states the card is in, as badges, in rail order.
    pub badges: Vec<StateBadge>,
    /// The glossary entries the face names: its text's terms, then its keywords, then its states'.
    pub glossary: Vec<GlossaryEntry>,
    /// ME-CN, R1301: the face prints its words in Chinese. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub chinese: Option<bool>,
    /// R711000: the view flagged the card Chinese (R1301), as opposed to a Chinese reader's own language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub flagged: Option<bool>,
    /// R1301: on a Chinese face, the name the English face prints, which picks the art's motif (R503).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub english_name: Option<String>,
    /// R1301: on a Chinese face, the text the English face prints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub english_text: Option<String>,
    /// R849000: in play, why the viewer's own unit cannot attack now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack_reason: Option<String>,
    /// R2321000: in play, the counts cards on the field keep on the unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tallies: Option<Vec<NamedTally>>,
    /// R979000: in play, why a card in the viewer's own hand (or graveyard) cannot be played now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub play_reason: Option<String>,
}
