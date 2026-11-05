//! Which cosmetics of a seat's deck a receiver may be told, and when (SPEC §10.8, §14.2; R1834003,
//! R3013002): the decision that keeps a decklist from leaking through the loadout frames.
//!
//! R1834002: when a match starts, each seat's loadout is frozen on its row ([`Loadout`]): the seat-level
//! half, and the picks and stamp of each card of its deck that wears something of its own. The seat-level
//! half travels whole, on join and on reconnect. R1834003: a card's own picks travel only once a card dealt
//! from that seat's deck, still showing as dealt, is in the receiver's view ([`shown_defs`]), and once per
//! receiver ([`cards_due`]): no decklist can be read from the frames (§10.8). R3013002: a card's proficiency
//! stamp (§14.14) is frozen and travels the same way, beside its picks, so another seat, a watcher and a
//! replay's viewer see it only once the card is shown.
//!
//! Which cards the decks dealt is [`dealt_cards`]: a card minted after the deal counts for no deck, so a
//! copy of a card the receiver can see names nothing about the deck it was copied from.
//!
//! Cosmetic only: nothing here enters `PlayerView`, an action, the log, a replay hash or a game record, and
//! the engine never reads it (CLAUDE.md rule 7). The server calls these functions natively (its
//! `loadout.rs` keeps the store, the registry and the freezing); a client reaches them through
//! [`LoadoutCall`] and [`LoadoutAnswer`], the one JSON in and out of `jackioh-wasm-core`'s `loadout` body
//! (README.md, the host contract). The caller owns the `sent` set: it is an argument and part of the answer.

use std::collections::{BTreeMap, BTreeSet};

use jackioh_engine::{BackrowView, GameState, HandView, PLAYER_IDS, PlayerId, PlayerView, SideView};
use serde::{Deserialize, Serialize};

use crate::deck::DeckSlots;

// The loadout

/// R2655000: a card's own full-art pick: `false` (no full art), `true` (its Epic full art, as every
/// pick before #2655 named it) or a full art's registry id, which travels as the id's text. An id
/// is kept as written and judged where it is worn, as a foil's is (`cosmetics::full_art_picked`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
pub enum FullArtPick {
    On(bool),
    Id(String),
}

impl From<bool> for FullArtPick {
    fn from(on: bool) -> FullArtPick {
        FullArtPick::On(on)
    }
}

impl FullArtPick {
    /// Whether the pick prints a full art at all: `true` or any id.
    pub fn prints(&self) -> bool {
        !matches!(self, FullArtPick::On(false))
    }

    /// The id the pick names, if it names one rather than a switch.
    pub fn id(&self) -> Option<&str> {
        match self {
            FullArtPick::Id(id) => Some(id),
            FullArtPick::On(_) => None,
        }
    }

    /// The pick a row of `public.card_cosmetic_picks` holds: its `full_art` switch, and the id
    /// migration 0072 keeps beside a switch that is on.
    pub fn of_row(full_art: Option<bool>, id: Option<String>) -> Option<FullArtPick> {
        match (full_art, id) {
            (Some(true), Some(id)) => Some(FullArtPick::Id(id)),
            (Some(on), _) => Some(FullArtPick::On(on)),
            (None, _) => None,
        }
    }
}

/// R1834000: one row of `public.card_cosmetic_picks` (migration 0061): what one card of the account
/// wears of its own: a foil, a finish, a border, its full-art pick ([`FullArtPick`], R2655000), which may
/// name a version, and the augments layered on it (migration 0079, R2725000). A kind
/// with no pick is absent and wears its default (R1953001; the Full-art cards setting, for `full_art`). An
/// id is kept as written, an unknown one too, and judged only where it is worn (the server's
/// `loadout::freeze_loadout`, R1834002).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardPicks {
    pub card_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub foil: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub finish: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub border: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub full_art: Option<FullArtPick>,
    /// R2725000: the augments the card layers on its version, by id, as written: none is no pick.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub augments: Vec<String>,
}

impl CardPicks {
    /// Whether the card has no pick of its own at all, which the store keeps as no row.
    pub fn is_empty(&self) -> bool {
        self.foil.is_none()
            && self.finish.is_none()
            && self.border.is_none()
            && self.full_art.is_none()
            && self.augments.is_empty()
    }
}

/// R3013002: the proficiency stamp one card of a seat's deck wears (§14.14), a stamp its level had
/// unlocked as the match froze it (R3013001).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardStamp {
    pub card_id: String,
    /// The stamp's level, 1 to 5. An existing stored shape: its width is not changed (README.md, the host
    /// contract's integers).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub stamp: i64,
}

/// R1834002: what a seat wears on its side and on every card that has no pick of its own, each a
/// registry id of its kind: the `loadouts` frame's half for one seat. Its foil, finish and border
/// are their kinds' defaults (R1953001), kept so the frame keeps its shape.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SeatLoadout {
    pub table: String,
    pub field: String,
    pub card_back: String,
    pub foil: String,
    pub finish: String,
    pub border: String,
}

impl SeatLoadout {
    /// The deck slots as a deck would carry them, each one picked: what a rematch replays (R672).
    pub fn deck_slots(&self) -> DeckSlots {
        DeckSlots {
            table: Some(self.table.clone()),
            field: Some(self.field.clone()),
            card_back: Some(self.card_back.clone()),
        }
    }
}

/// R1834002: one seat's frozen loadout: its seat-level half, and the cards of its deck that wear
/// something of their own, by card id. A card left out wears the seat's picks and no full art.
/// R3013002: and the cards of its deck that wear a stamp, by card id; a loadout frozen before #3013,
/// and a bot's, has none, and a loadout without one keeps its shape.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Loadout {
    pub seat: SeatLoadout,
    #[serde(default)]
    pub cards: Vec<CardPicks>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stamps: Vec<CardStamp>,
}

impl Loadout {
    pub fn card(&self, card_id: &str) -> Option<&CardPicks> {
        self.cards.iter().find(|card| card.card_id == card_id)
    }

    /// R3013002: the stamp `card_id` wears, if any.
    pub fn stamp(&self, card_id: &str) -> Option<&CardStamp> {
        self.stamps.iter().find(|stamp| stamp.card_id == card_id)
    }
}

/// R1834003: one seat's cards, as the `cardLoadouts` frame and a replay page carry them. R3013002: with
/// the stamps of those cards that wear one, left out when none does.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SeatCards {
    pub seat: PlayerId,
    pub cards: Vec<CardPicks>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stamps: Vec<CardStamp>,
}

// The decision

/// R1834003: every card dealt from a deck in a pristine game, by instance id, seat and dealt card id.
/// Minted cards (R943 Created tokens/copies) are excluded. Owner can change later (R12);
/// cosmetics are only sent for cards still showing what was dealt.
pub fn dealt_cards(state: &GameState) -> BTreeMap<String, (PlayerId, String)> {
    let mut dealt = BTreeMap::new();
    for seat in [PlayerId::P1, PlayerId::P2] {
        let side = &state.players[seat];
        let field = side
            .units
            .iter()
            .flatten()
            .flatten()
            .chain(side.backrow.iter().flatten())
            .chain(side.carried.iter().flatten().flatten());
        let cards = side
            .library
            .iter()
            .chain(&side.hand)
            .chain(&side.graveyard)
            .chain(&side.exile)
            .chain(&side.resolving)
            .chain(field);
        for card in cards.filter(|card| card.created.is_none()) {
            dealt.insert(card.id.clone(), (card.owner, card.def_id.clone()));
        }
    }
    dealt
}

/// The cards a side of a view shows face up, each `(instance id, def id)`. A face-down card, the
/// opponent's hand, a library, the events and the prompts name nothing here.
fn shown_cards(side: &SideView) -> impl Iterator<Item = (&str, &str)> {
    let units = side
        .units
        .iter()
        .flatten()
        .chain(side.carried.iter().flatten().flatten())
        .map(|unit| (unit.instance_id.as_str(), unit.def_id.as_str()));
    let backrow = side.backrow.iter().flatten().filter_map(|card| match card {
        BackrowView::Public(card) => Some((card.instance_id.as_str(), card.def_id.as_str())),
        BackrowView::FaceDown(_) => None,
    });
    let hand = match &side.hand {
        HandView::Cards(cards) => cards.as_slice(),
        HandView::Count { .. } => &[],
    };
    let piles = hand
        .iter()
        .chain(&side.graveyard)
        .chain(&side.exile)
        .chain(&side.resolving)
        .map(|card| (card.instance_id.as_str(), card.def_id.as_str()));
    units.chain(backrow).chain(piles)
}

/// R1834003: the `(seat, card)` pairs `view` shows: each card in it dealt from `seat`'s deck (`dealt`,
/// [`dealt_cards`]) and still shown as the card it was dealt as, wherever it is now (a card
/// that changed hands counts for the deck that dealt it). A card minted later or transformed names no
/// pair, so a pair never says more than the view it is read from: a card of that deck, seen.
pub fn shown_defs(
    view: &PlayerView,
    dealt: &BTreeMap<String, (PlayerId, String)>,
) -> BTreeSet<(PlayerId, String)> {
    [&view.you, &view.opponent]
        .into_iter()
        .flat_map(shown_cards)
        .filter_map(|(instance, def)| {
            let (seat, dealt_as) = dealt.get(instance)?;
            (dealt_as == def).then(|| (*seat, def.to_string()))
        })
        .collect()
}

/// R1834003: the cards' own picks `shown` makes due to a receiver that has been sent `sent`
/// already: each new pair's entry in its seat's loadout (`loadouts`, seat order), by seat, the
/// pairs then marked sent. A pair whose card has no entry of its own is marked and sends nothing.
/// R3013002: a pair's stamp, when its card wears one, goes with it by the same rule.
pub fn cards_due(
    shown: &BTreeSet<(PlayerId, String)>,
    loadouts: &(Loadout, Loadout),
    sent: &mut BTreeSet<(PlayerId, String)>,
) -> Vec<SeatCards> {
    let mut due: Vec<SeatCards> = Vec::new();
    for pair in shown {
        if !sent.insert(pair.clone()) {
            continue;
        }
        let (seat, card) = pair;
        let loadout = match seat {
            PlayerId::P1 => &loadouts.0,
            PlayerId::P2 => &loadouts.1,
        };
        let (entry, stamp) = (loadout.card(card), loadout.stamp(card));
        if entry.is_none() && stamp.is_none() {
            continue;
        }
        let at = match due.iter().position(|cards| cards.seat == *seat) {
            Some(at) => at,
            None => {
                due.push(SeatCards {
                    seat: *seat,
                    cards: Vec::new(),
                    stamps: Vec::new(),
                });
                due.len() - 1
            }
        };
        let Some(cards) = due.get_mut(at) else {
            continue;
        };
        cards.cards.extend(entry.cloned());
        cards.stamps.extend(stamp.cloned());
    }
    due.sort_by_key(|cards| PLAYER_IDS.iter().position(|seat| *seat == cards.seat));
    due
}

// The boundary

/// One call of this module as a host makes it through `jackioh-wasm-core`'s `loadout` body: the function
/// and its arguments as one JSON object, `{ "call": "shownDefs", "view": …, "dealt": … }`. The host holds
/// what is long-lived (the `dealt` map of a match, each receiver's `sent` set, the frozen loadouts) and
/// hands it back on every call (README.md, the host contract). The server calls the functions themselves.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "call", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LoadoutCall {
    /// [`dealt_cards`]: read once, off the state a match begins from.
    DealtCards { state: Box<GameState> },
    /// [`shown_defs`].
    ShownDefs {
        view: Box<PlayerView>,
        #[cfg_attr(feature = "ts", ts(type = "{ [instanceId: string]: [PlayerId, string] }"))]
        dealt: BTreeMap<String, (PlayerId, String)>,
    },
    /// [`cards_due`]: `sent` is what this receiver has been sent already, and the answer carries it back
    /// with the new pairs marked.
    CardsDue {
        shown: BTreeSet<(PlayerId, String)>,
        loadouts: Box<(Loadout, Loadout)>,
        #[serde(default)]
        sent: BTreeSet<(PlayerId, String)>,
    },
}

/// What a [`LoadoutCall`] answers, as one JSON object with one key.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum LoadoutAnswer {
    /// `dealtCards`: every card the decks dealt, by instance id.
    Dealt(
        #[cfg_attr(feature = "ts", ts(type = "{ [instanceId: string]: [PlayerId, string] }"))]
        BTreeMap<String, (PlayerId, String)>,
    ),
    /// `shownDefs`: the `(seat, card)` pairs the view shows.
    Shown(BTreeSet<(PlayerId, String)>),
    /// `cardsDue`: the entries to send, and the receiver's `sent` set to keep.
    Due(CardsDue),
}

/// `cardsDue`'s answer: [`cards_due`]'s result, and the `sent` set it left.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardsDue {
    pub cards: Vec<SeatCards>,
    pub sent: BTreeSet<(PlayerId, String)>,
}

/// Makes `call`: the function it names, over its arguments, answered as data. Nothing is read but the
/// call, and nothing is kept: the next call carries the `sent` set this one answered.
pub fn apply(call: &LoadoutCall) -> LoadoutAnswer {
    match call {
        LoadoutCall::DealtCards { state } => LoadoutAnswer::Dealt(dealt_cards(state)),
        LoadoutCall::ShownDefs { view, dealt } => LoadoutAnswer::Shown(shown_defs(view, dealt)),
        LoadoutCall::CardsDue {
            shown,
            loadouts,
            sent,
        } => {
            let mut sent = sent.clone();
            let cards = cards_due(shown, loadouts, &mut sent);
            LoadoutAnswer::Due(CardsDue { cards, sent })
        }
    }
}
