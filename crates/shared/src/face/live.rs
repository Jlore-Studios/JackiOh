//! The face in play of a card the view lists, and of one it only names (SPEC §10.10): the one the board
//! draws for a card the view lists, and the one the showcase, a log line, a pile and a prompt draw for a card
//! the view names.
//!
//! A face in play is the card as the view says it stands (R243). Everything is read off the view — never
//! worked out — so none of it is a rule (CLAUDE.md rule 7). The collection's faces, the card as printed, are
//! `FaceSource`s with no `in_play`. The definition comes from the public catalog (§5.1), else from the
//! match-made definitions the view carries (R243), else from nothing: the card is drawn by its id. The R97
//! sentinel names no card, so it has no face, and the callers draw a back.

use indexmap::IndexMap;
use jackioh_engine::state::{Slumber, StarshipMemory};
use jackioh_engine::wire::{
    AnimatedView, BackrowView, CardDef, CardElement, CardMark, CardType, CardView, CopiedTextView,
    Enchantment, HandView, Keyword, PlayerView, PreviewValue, QuestView, SideView, Tag, TallyView, Tuning,
    UnitView,
};
use serde::{Deserialize, Serialize};

use super::model::{Lookup, face_model};
use super::types::{
    AnimatedFace, CopiesIn, DefMap, DormantFace, FaceModel, FaceSource, InPlay, LiveStats, NamedTally,
    RolledPower, StarshipIn, StarshipPieceRef,
};
use super::words::Words;

/// R97: the id and definition an event or a prompt carries in place of a card the viewer may not read.
pub const HIDDEN_CARD: &str = "hidden";

/// A card the view lists as far as a face reads it: a `CardView`, a `UnitView` or a face-up backrow card,
/// whichever the view gave. A field the view does not carry is absent.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LiveCard {
    pub instance_id: String,
    pub def_id: String,
    pub radiant: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub chinese: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created: Option<bool>,
    /// R3536005: the card has evolved, or is marked to enter the field evolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub evolved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub embiggen_cost: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fixed_x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub health: Option<i32>,
    /// A unit's, and what makes the card one (with `keywords`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max_health: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub keywords: Option<Vec<Keyword>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub power: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vanilla: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preview: Option<Vec<PreviewValue>>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<CardType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub brittle: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub element: Option<CardElement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<IndexMap<String, i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tuning: Option<Tuning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub enchantments: Option<Vec<Enchantment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marks: Option<Vec<CardMark>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quest: Option<QuestView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copies: Option<CopiedTextView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub animated: Option<AnimatedView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub berserk: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frozen: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slumber: Option<Slumber>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frozen_forever: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub starship: Option<StarshipMemory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grants: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tallies: Option<Vec<TallyView>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub play_reason: Option<String>,
}

impl From<&CardView> for LiveCard {
    fn from(card: &CardView) -> LiveCard {
        LiveCard {
            instance_id: card.instance_id.clone(),
            def_id: card.def_id.clone(),
            radiant: card.radiant,
            chinese: card.chinese,
            created: card.created,
            evolved: card.evolved,
            created_by: card.created_by.clone(),
            cost: Some(card.cost),
            embiggen_cost: card.embiggen_cost,
            fixed_x: card.fixed_x,
            attack: card.attack,
            health: card.health,
            keywords: card.keywords.clone(),
            power: card.power.clone(),
            label: card.label.clone(),
            preview: card.preview.clone(),
            type_: card.type_,
            tags: card.tags.clone(),
            brittle: card.brittle,
            element: card.element,
            params: card.params.clone(),
            tuning: card.tuning.clone(),
            enchantments: card.enchantments.clone(),
            marks: card.marks.clone(),
            quest: card.quest.clone(),
            copies: card.copies.clone(),
            play_reason: card.play_reason.clone(),
            ..LiveCard::default()
        }
    }
}

impl From<&UnitView> for LiveCard {
    fn from(unit: &UnitView) -> LiveCard {
        LiveCard {
            instance_id: unit.instance_id.clone(),
            def_id: unit.def_id.clone(),
            radiant: unit.radiant,
            chinese: unit.chinese,
            created: unit.created,
            evolved: unit.evolved,
            created_by: unit.created_by.clone(),
            cost: Some(unit.cost),
            attack: Some(unit.attack),
            health: Some(unit.health),
            max_health: Some(unit.max_health),
            keywords: Some(unit.keywords.clone()),
            power: unit.power.clone(),
            vanilla: unit.vanilla,
            preview: unit.preview.clone(),
            type_: unit.type_,
            tags: unit.tags.clone(),
            brittle: unit.brittle,
            element: unit.element,
            params: unit.params.clone(),
            tuning: unit.tuning.clone(),
            enchantments: unit.enchantments.clone(),
            marks: unit.marks.clone(),
            quest: unit.quest.clone(),
            copies: unit.copies.clone(),
            animated: unit.animated,
            berserk: unit.berserk,
            frozen: unit.frozen,
            slumber: unit.slumber,
            frozen_forever: unit.frozen_forever,
            starship: unit.starship.clone(),
            grants: unit.grants.clone(),
            attack_reason: unit.attack_reason.clone(),
            tallies: unit.tallies.clone(),
            ..LiveCard::default()
        }
    }
}

impl LiveCard {
    /// A face-up backrow card (`BackrowView` with `faceDown` false), or `None` for a face-down one.
    pub fn from_backrow(backrow: &BackrowView) -> Option<LiveCard> {
        let BackrowView::Public(card) = backrow else {
            return None;
        };
        Some(LiveCard {
            instance_id: card.instance_id.clone(),
            def_id: card.def_id.clone(),
            radiant: card.radiant,
            chinese: card.chinese,
            created: card.created,
            evolved: card.evolved,
            created_by: card.created_by.clone(),
            cost: Some(card.cost),
            attack: card.attack,
            health: card.health,
            keywords: card.keywords.clone(),
            power: card.power.clone(),
            vanilla: card.vanilla,
            preview: card.preview.clone(),
            type_: Some(card.type_),
            tags: card.tags.clone(),
            brittle: card.brittle,
            element: card.element,
            params: card.params.clone(),
            tuning: card.tuning.clone(),
            enchantments: card.enchantments.clone(),
            marks: card.marks.clone(),
            quest: card.quest.clone(),
            copies: card.copies.clone(),
            ..LiveCard::default()
        })
    }

    /// A unit on the field, as against a card in a pile, a hand or the backrow.
    fn is_unit(&self) -> bool {
        self.max_health.is_some() && self.keywords.is_some()
    }
}

/// What a face in play needs beyond the card itself (R243): facts the view puts elsewhere.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LiveFacts {
    /// A face-up backrow card's type, straight off `BackrowView`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<CardType>,
    /// The power a Heroic Power on the field rolled (`HeroView.powers`); a hand card carries its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field_power: Option<RolledPower>,
    /// B5 E14: the definition of the Spell a copier's `copies` names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copied: Option<CardDef>,
    /// R2321000: the counts cards on the field keep on a unit, by their keepers' names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tallies: Option<Vec<NamedTally>>,
    /// #2437: the card lies in a hand, so its face carries its Temporary count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub in_hand: Option<bool>,
}

/// The face in play of a card the view lists: its live cost, and whatever else the view says about it (R243).
pub fn live_source(
    card: &LiveCard,
    def: Option<CardDef>,
    name: Option<String>,
    facts: &LiveFacts,
) -> FaceSource {
    let unit = card.is_unit();
    let mut in_play = InPlay::default();
    if !unit {
        if let (Some(attack), Some(health)) = (card.attack, card.health) {
            in_play.hand_stats = Some(super::types::HandStats { attack, health });
        }
        // B5 E38, R1438: a hand card's keywords as the view gives them, a given Lucky included.
        in_play.hand_keywords = card.keywords.clone();
    }
    // R43, R243: in hand the power rides on the card; on the field the hero's power list names it.
    in_play.power = card
        .power
        .as_ref()
        .map(|name| RolledPower { name: name.clone() })
        .or_else(|| facts.field_power.clone());
    // R2247000: the word the card shows in place of its text (Anomaly Found!'s board code).
    in_play.label = card.label.clone();
    // R243, §6.3 Vanilla: a backrow card's text can be gone too (`BackrowView.vanilla`).
    if card.vanilla == Some(true) {
        in_play.vanilla = Some(true);
    }
    // R280: what the card's formula comes to now, where the view says (never in the collection).
    in_play.preview = card.preview.clone().filter(|preview| !preview.is_empty());
    // B3.4, R386: the numbers the card has now fill its text's `{key}`s, and what changed is marked.
    in_play.params = card.params.clone();
    in_play.tuning = card.tuning.clone();
    // B3.3, R385: the Brittle count; B5 E39: the enchantments riding the card.
    in_play.brittle = card.brittle;
    // #2437: a hand card's face carries the count its Temporary has left (R2179001).
    if !unit && facts.in_hand == Some(true) {
        in_play.in_hand = Some(true);
    }
    // R980: the card's element, while a Feng Shui acts.
    in_play.element = card.element;
    in_play.enchantments = card.enchantments.clone().filter(|list| !list.is_empty());
    // R437: the marks on it, which the inspect overlays spell out.
    in_play.marks = card.marks.clone().filter(|list| !list.is_empty());
    // B3.1, R383: a Field Spell, Trap or Field Trap standing in a unit zone as a Unit.
    in_play.animated = card
        .animated
        .filter(|_| unit)
        .map(|animated| AnimatedFace { home: animated.home });
    // B5 E35: a Berserk unit. R1174000: a Frozen unit; R1848000: one Frozen forever.
    in_play.berserk = (unit && card.berserk == Some(true)).then_some(true);
    in_play.frozen = (unit && card.frozen == Some(true)).then_some(true);
    // R1839000, R1839001: a Dormant card, with its countdown.
    in_play.dormant = card
        .slumber
        .filter(|_| unit)
        .map(|slumber| DormantFace { turns: slumber.turns });
    in_play.frozen_forever = (unit && card.frozen_forever == Some(true)).then_some(true);
    // R1852000, R1852004: a Starship, with the pieces fused into it and the name it launched under.
    // R2249000: and the pieces themselves, which its face lists in place of its text.
    if let (true, Some(ship)) = (unit, &card.starship) {
        in_play.starship = Some(StarshipIn {
            pieces: ship.pieces.len() as i32,
            name: ship.name.clone(),
            list: Some(
                ship.pieces
                    .iter()
                    .map(|piece| StarshipPieceRef {
                        def_id: piece.def_id.clone(),
                        radiant: piece.radiant == Some(true),
                    })
                    .collect(),
            ),
        });
    }
    // MD-B6, R943: a Created card, wherever the view carries the mark; R1956001: and the card that made it.
    if card.created == Some(true) {
        in_play.created = Some(true);
    }
    // R3536005: an Evolved card, wherever the view carries the mark.
    if card.evolved == Some(true) {
        in_play.evolved = Some(true);
    }
    in_play.created_by = card.created_by.clone();
    // ME-GRANT, R1107: the Death abilities other cards granted the unit, as its lines read them.
    if unit {
        in_play.grants = card.grants.clone().filter(|list| !list.is_empty());
        // R849000: the inspect notes spell it out.
        in_play.attack_reason = card.attack_reason.clone();
    }
    // R2321000: and the counts the field keeps on the unit.
    in_play.tallies = facts.tallies.clone().filter(|list| !list.is_empty());
    // R979000: a hand card, or (R1571000) one in the viewer's own graveyard, says why it cannot be played.
    if !unit {
        in_play.play_reason = card.play_reason.clone();
    }
    // B5 E33, R404: a quest line; B5 E14, R399: the Spell text a copier has, with the numbers it reads.
    in_play.quest = card.quest.clone();
    if let (Some(copies), Some(copied)) = (&card.copies, &facts.copied) {
        in_play.copies = Some(CopiesIn {
            def: copied.clone(),
            radiant: copies.radiant,
            params: copies.params.clone(),
        });
    }
    FaceSource {
        def_id: card.def_id.clone(),
        def,
        name,
        // B2.7: the type the card has now, where it differs from its definition's (Blood Moon's Radiant face).
        type_: card.type_.or(facts.type_),
        radiant: card.radiant,
        live_cost: card.cost,
        live_embiggen_cost: card.embiggen_cost,
        fixed_x: card.fixed_x,
        live: if unit {
            Some(LiveStats {
                attack: card.attack.unwrap_or(0),
                health: card.health.unwrap_or(0),
                max_health: card.max_health.unwrap_or(0),
                keywords: card.keywords.clone().unwrap_or_default(),
            })
        } else {
            None
        },
        in_play: Some(in_play),
        // R1301: the view says the card is shown in Chinese; never a card the viewer may not read.
        chinese: Some(card.chinese == Some(true)),
        // MD-B15, R923: the view's tags where they differ from the definition's (a granted tag).
        tags: card.tags.clone(),
    }
}

/// The face in play of a card the view lists.
pub fn live_face(
    words: Words,
    card: &LiveCard,
    def: Option<CardDef>,
    name: Option<String>,
    facts: &LiveFacts,
    defs: &DefMap,
) -> FaceModel {
    face_model(words, &live_source(card, def, name, facts), &Lookup { defs })
}

/// A card an event or a prompt names (R243): its definition as the game shows it, at `cost` when one is given.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct NamedCard {
    pub def_id: String,
    pub radiant: bool,
    /// The instance, so a card the view still lists is drawn as it stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub instance_id: Option<String>,
    /// A cost to show when the view lists the card nowhere (a play's `costPaid`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<i32>,
}

fn sides(view: &PlayerView) -> [&SideView; 2] {
    [&view.you, &view.opponent]
}

/// Every card the view shows face up, with whether it lies in a hand.
fn listed(view: &PlayerView) -> Vec<(LiveCard, bool)> {
    let mut out: Vec<(LiveCard, bool)> = Vec::new();
    for side in sides(view) {
        out.extend(
            side.units
                .iter()
                .flatten()
                .map(|unit| (LiveCard::from(unit), false)),
        );
        out.extend(
            side.backrow
                .iter()
                .flatten()
                .filter_map(LiveCard::from_backrow)
                .map(|card| (card, false)),
        );
        out.extend(
            side.graveyard
                .iter()
                .chain(&side.exile)
                .chain(&side.resolving)
                .map(|card| (LiveCard::from(card), false)),
        );
        if let HandView::Cards(cards) = &side.hand {
            out.extend(cards.iter().map(|card| (LiveCard::from(card), true)));
        }
    }
    out
}

/// The card the view shows face up under this instance id, or `None` when it shows it nowhere (which is also
/// what a hidden card gives).
fn card_in_view(view: &PlayerView, instance_id: &str) -> Option<LiveCard> {
    if instance_id == HIDDEN_CARD {
        return None;
    }
    listed(view)
        .into_iter()
        .map(|(card, _)| card)
        .find(|card| card.instance_id == instance_id)
}

/// The powers the heroes' lists name, by instance id (`HeroView.powers`, R243).
fn field_power(view: &PlayerView, instance_id: &str) -> Option<RolledPower> {
    sides(view)
        .iter()
        .flat_map(|side| side.hero.powers.iter())
        .find(|power| power.instance_id == instance_id)
        .map(|power| RolledPower {
            name: power.name.clone(),
        })
}

/// A card's definition: the catalog's, else the view's match-made ones (R243).
fn def_for(view: &PlayerView, id: &str) -> Option<CardDef> {
    let defs = view.defs.clone().unwrap_or_default();
    Lookup { defs: &defs }.def(id)
}

/// The face of a card the view lists, with the facts the view puts elsewhere read from it.
fn listed_face(words: Words, view: &PlayerView, card: &LiveCard, in_hand: bool, defs: &DefMap) -> FaceModel {
    let lookup = Lookup { defs };
    let tallies: Vec<NamedTally> = card
        .tallies
        .iter()
        .flatten()
        .map(|tally| NamedTally {
            name: lookup
                .def(&tally.def_id)
                .map_or_else(|| tally.def_id.clone(), |def| def.name),
            value: tally.value,
            goal: tally.goal,
        })
        .collect();
    let facts = LiveFacts {
        type_: None,
        field_power: field_power(view, &card.instance_id),
        copied: card.copies.as_ref().and_then(|copies| lookup.def(&copies.def_id)),
        tallies: Some(tallies),
        in_hand: Some(in_hand),
    };
    let def = lookup.def(&card.def_id);
    let name = def.as_ref().map(|def| def.name.clone());
    live_face(words, card, def, name, &facts, defs)
}

/// Every face the view shows, by instance id: the cards it lists face up on the field, in the piles and in the
/// viewer's own hand. A card the viewer may not read (the sentinel) has none.
pub fn view_faces(words: Words, view: &PlayerView) -> IndexMap<String, FaceModel> {
    let defs: DefMap = view.defs.clone().unwrap_or_default();
    let mut out = IndexMap::new();
    for (card, in_hand) in listed(view) {
        if card.def_id == HIDDEN_CARD || card.def_id.is_empty() {
            continue;
        }
        out.insert(
            card.instance_id.clone(),
            listed_face(words, view, &card, in_hand, &defs),
        );
    }
    out
}

/// The face in play of a card an event names, or `None` for the sentinel. A card the view still lists is drawn
/// as it stands there; one it no longer lists is its definition as the game shows it.
pub fn named_face(words: Words, view: &PlayerView, named: &NamedCard) -> Option<FaceModel> {
    if named.def_id == HIDDEN_CARD || named.def_id.is_empty() {
        return None;
    }
    let defs: DefMap = view.defs.clone().unwrap_or_default();
    if let Some(card) = named.instance_id.as_deref().and_then(|id| card_in_view(view, id))
        && card.def_id == named.def_id
    {
        let in_hand = listed(view)
            .iter()
            .any(|(listed, in_hand)| *in_hand && listed.instance_id == card.instance_id);
        return Some(listed_face(words, view, &card, in_hand, &defs));
    }
    let def = def_for(view, &named.def_id);
    let face_type = def.as_ref().map(|def| {
        let face = if named.radiant { &def.radiant } else { &def.base };
        face.type_.unwrap_or(def.type_)
    });
    let name = def.as_ref().map(|def| def.name.clone());
    let source = FaceSource {
        def_id: named.def_id.clone(),
        def,
        name,
        type_: face_type,
        radiant: named.radiant,
        live_cost: named.cost,
        in_play: Some(InPlay::default()),
        ..FaceSource::default()
    };
    Some(face_model(words, &source, &Lookup { defs: &defs }))
}
