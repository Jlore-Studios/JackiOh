//! Which cosmetics of a seat's deck a receiver may be told, and when (SPEC §10.8, §14.2; R1834003,
//! R3013002), and what a seat wears to begin with (R1834002): the decisions that freeze a loadout when a
//! match starts and keep a decklist from leaking through the loadout frames.
//!
//! R1834002: when a match starts, each seat's loadout is frozen on its row ([`Loadout`], by [`freeze_loadout`]
//! and [`SeatPicks::freeze`]): the deck's three slots and each card's picks, each resolved to a registry id the
//! account owns at that moment and the kind's default otherwise. The seat-level half travels whole, on join and
//! on reconnect. R1834003: a card's own picks travel only once a card dealt from that seat's deck, still showing
//! as dealt, is in the receiver's view ([`shown_defs`]), and once per receiver ([`cards_due`]): no decklist can
//! be read from the frames (§10.8). R3013002: a card's proficiency stamp (§14.14) is frozen ([`freeze_stamps`],
//! [`worn_stamps`]) and travels the same way, beside its picks, so another seat, a watcher and a replay's viewer
//! see it only once the card is shown.
//!
//! Which cards the decks dealt is [`dealt_cards`]: a card minted after the deal counts for no deck, so a
//! copy of a card the receiver can see names nothing about the deck it was copied from.
//!
//! Cosmetic only: nothing here enters `PlayerView`, an action, the log, a replay hash or a game record, and
//! the engine never reads it (CLAUDE.md rule 7). The server calls these functions natively (its
//! `loadout.rs` keeps the store reads and writes: a seat's picks off the ledger, the settings row and the
//! proficiency rows, which it turns into the plain records below); a client reaches them through
//! [`LoadoutCall`] and [`LoadoutAnswer`], the one JSON in and out of `jackioh-wasm-core`'s `loadout` body
//! (README.md, the host contract). The caller owns the `sent` set: it is an argument and part of the answer.
//! Nothing here draws: a freeze is a function of the records it is given and the shipped registry.

use std::collections::{BTreeMap, BTreeSet};

use jackioh_engine::{BackrowView, GameState, HandView, PLAYER_IDS, PlayerId, PlayerView, SideView};
use serde::{Deserialize, Serialize};

use crate::cosmetics::{
    Cosmetic, CosmeticKind, OwnedCosmetics, default_of, level_of_pick, offered, owns, registry, worn_level,
};
use crate::deck::DeckSlots;

// The settings keys and the kinds a loadout wears

/// The settings group the client keeps the account's picks in (R633, R1295000).
pub const COSMETICS_GROUP: &str = "cosmetics";
/// The settings group of the card display, and its Full-art cards switch (R1313000).
pub const CARDS_GROUP: &str = "cards";
pub const FULL_ART_SETTING: &str = "fullArt";
/// R2725000: a card pick's key for the augments it layers on its version.
pub const AUGMENTS_KEY: &str = "augments";

/// The kinds a deck carries a pick of (R1297000), in the registry's order.
pub const DECK_SLOT_KINDS: [CosmeticKind; 3] =
    [CosmeticKind::Table, CosmeticKind::Field, CosmeticKind::CardBack];
/// The kinds a card wears (R1295000), in the registry's order.
pub const CARD_SLOT_KINDS: [CosmeticKind; 3] =
    [CosmeticKind::Foil, CosmeticKind::Finish, CosmeticKind::Border];
/// R3335000: the kinds that dress the Starships a seat launches, picked on the account alone, in the registry's
/// order.
pub const STARSHIP_SLOT_KINDS: [CosmeticKind; 4] = CosmeticKind::STARSHIP;

/// The key a kind's pick has in the `cosmetics` settings group: the kind's own camel-case name. A version is
/// picked as a card's `fullArt` and an augment in its `augments` (R2725000).
pub fn setting_key(kind: CosmeticKind) -> &'static str {
    match kind {
        CosmeticKind::Foil => "foil",
        CosmeticKind::Finish => "finish",
        CosmeticKind::Border => "border",
        CosmeticKind::Table => "table",
        CosmeticKind::Field => "field",
        CosmeticKind::CardBack => "cardBack",
        CosmeticKind::FullArt => "fullArt",
        CosmeticKind::Portrait => "portrait",
        CosmeticKind::Version => FULL_ART_SETTING,
        CosmeticKind::Augment => AUGMENTS_KEY,
        CosmeticKind::StarshipHull => "starshipHull",
        CosmeticKind::StarshipLivery => "starshipLivery",
        CosmeticKind::StarshipTrail => "starshipTrail",
        CosmeticKind::StarshipPieceSkin => "starshipPieceSkin",
    }
}

// The loadout

/// R2655000: a card's own full-art pick, the cosmetics registry's (`cosmetics::FullArtPick`, where its home
/// is, beside `full_art_picked` that judges it): re-exported here, so `loadout::FullArtPick` still names it.
pub use crate::cosmetics::FullArtPick;

/// R1834000: one row of `public.card_cosmetic_picks` (migration 0061): what one card of the account
/// wears of its own: a foil, a finish, a border, its full-art pick ([`FullArtPick`], R2655000), which may
/// name a version, and the augments layered on it (migration 0079, R2725000). A kind
/// with no pick is absent and wears its default (R1953001; the Full-art cards setting, for `full_art`). An
/// id is kept as written, an unknown one too, and judged only where it is worn ([`freeze_loadout`],
/// R1834002).
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
    /// The pick of a kind a card wears of its own: a foil, a finish or a border; absent for any other kind.
    pub fn get(&self, kind: CosmeticKind) -> Option<&str> {
        match kind {
            CosmeticKind::Foil => self.foil.as_deref(),
            CosmeticKind::Finish => self.finish.as_deref(),
            CosmeticKind::Border => self.border.as_deref(),
            _ => None,
        }
    }

    /// Sets the pick of a kind a card wears of its own; any other kind changes nothing.
    pub fn set(&mut self, kind: CosmeticKind, id: Option<String>) {
        match kind {
            CosmeticKind::Foil => self.foil = id,
            CosmeticKind::Finish => self.finish = id,
            CosmeticKind::Border => self.border = id,
            _ => {}
        }
    }

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
/// are their kinds' defaults (R1953001), kept so the frame keeps its shape. R3335000: and what dresses the
/// Starships it launches, the account's four Starship picks; a loadout frozen before #3335 has none, and reads
/// each kind's default.
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
    #[serde(default = "default_starship_hull")]
    pub starship_hull: String,
    #[serde(default = "default_starship_livery")]
    pub starship_livery: String,
    #[serde(default = "default_starship_trail")]
    pub starship_trail: String,
    #[serde(default = "default_starship_piece_skin")]
    pub starship_piece_skin: String,
}

// R3335000: what a seat frozen before #3335 wears on its Starships, each kind's default in the shipped registry.
fn default_starship_hull() -> String {
    default_of(registry(), CosmeticKind::StarshipHull).to_string()
}
fn default_starship_livery() -> String {
    default_of(registry(), CosmeticKind::StarshipLivery).to_string()
}
fn default_starship_trail() -> String {
    default_of(registry(), CosmeticKind::StarshipTrail).to_string()
}
fn default_starship_piece_skin() -> String {
    default_of(registry(), CosmeticKind::StarshipPieceSkin).to_string()
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

    /// What a seat wears when nothing is frozen for it: each kind's default in `registry`.
    pub fn defaults_of(registry: &[Cosmetic]) -> SeatLoadout {
        SeatLoadout {
            table: default_of(registry, CosmeticKind::Table).to_string(),
            field: default_of(registry, CosmeticKind::Field).to_string(),
            card_back: default_of(registry, CosmeticKind::CardBack).to_string(),
            foil: default_of(registry, CosmeticKind::Foil).to_string(),
            finish: default_of(registry, CosmeticKind::Finish).to_string(),
            border: default_of(registry, CosmeticKind::Border).to_string(),
            starship_hull: default_of(registry, CosmeticKind::StarshipHull).to_string(),
            starship_livery: default_of(registry, CosmeticKind::StarshipLivery).to_string(),
            starship_trail: default_of(registry, CosmeticKind::StarshipTrail).to_string(),
            starship_piece_skin: default_of(registry, CosmeticKind::StarshipPieceSkin).to_string(),
        }
    }

    /// The id of a kind's slot, empty for a kind a seat wears no slot of.
    pub fn get(&self, kind: CosmeticKind) -> &str {
        match kind {
            CosmeticKind::Table => &self.table,
            CosmeticKind::Field => &self.field,
            CosmeticKind::CardBack => &self.card_back,
            CosmeticKind::Foil => &self.foil,
            CosmeticKind::Finish => &self.finish,
            CosmeticKind::Border => &self.border,
            CosmeticKind::StarshipHull => &self.starship_hull,
            CosmeticKind::StarshipLivery => &self.starship_livery,
            CosmeticKind::StarshipTrail => &self.starship_trail,
            CosmeticKind::StarshipPieceSkin => &self.starship_piece_skin,
            CosmeticKind::FullArt
            | CosmeticKind::Portrait
            | CosmeticKind::Version
            | CosmeticKind::Augment => "",
        }
    }

    /// Puts `id` in the slot of `kind`; a kind a seat wears no slot of changes nothing.
    fn set(&mut self, kind: CosmeticKind, id: String) {
        match kind {
            CosmeticKind::Table => self.table = id,
            CosmeticKind::Field => self.field = id,
            CosmeticKind::CardBack => self.card_back = id,
            CosmeticKind::Foil => self.foil = id,
            CosmeticKind::Finish => self.finish = id,
            CosmeticKind::Border => self.border = id,
            CosmeticKind::StarshipHull => self.starship_hull = id,
            CosmeticKind::StarshipLivery => self.starship_livery = id,
            CosmeticKind::StarshipTrail => self.starship_trail = id,
            CosmeticKind::StarshipPieceSkin => self.starship_piece_skin = id,
            CosmeticKind::FullArt
            | CosmeticKind::Portrait
            | CosmeticKind::Version
            | CosmeticKind::Augment => {}
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
    /// What a seat wears when nothing is frozen for it (a row from before #1834): every kind's default in the
    /// shipped registry, no card of its own.
    pub fn defaults() -> Loadout {
        Loadout {
            seat: SeatLoadout::defaults_of(registry()),
            cards: Vec::new(),
            stamps: Vec::new(),
        }
    }

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

// The picks a loadout is frozen from

/// The account's own picks as its settings hold them (R633): one text per kind in the `cosmetics` group, by
/// its key there ([`setting_key`]), any of them absent, and the Full-art cards switch, on unless it is off.
/// A ladder bot keeps its own the same way (R1834005).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AccountPicks {
    #[serde(default)]
    pub picks: BTreeMap<String, String>,
    pub full_art: bool,
}

impl Default for AccountPicks {
    fn default() -> AccountPicks {
        AccountPicks {
            picks: BTreeMap::new(),
            full_art: true,
        }
    }
}

/// What a host reads of an account's settings for its picks, as a plain record: the text values of the
/// `cosmetics` group by key ([`COSMETICS_GROUP`]; a value that is not text is left out) and the
/// [`FULL_ART_SETTING`] switch of the `cards` group ([`CARDS_GROUP`]) when it holds a boolean. The server builds it
/// from a `PlayerSettingsRow`; a host with no such row builds it from its own store.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AccountSettings {
    #[serde(default)]
    pub cosmetics: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub full_art: Option<bool>,
}

impl AccountPicks {
    /// What `settings` hold of the account's picks: the text of each kind's key, any other value or key left
    /// out; no settings hold none, and the switch is on unless they turn it off.
    pub fn of_settings(settings: Option<&AccountSettings>) -> AccountPicks {
        let Some(settings) = settings else {
            return AccountPicks::default();
        };
        let mut picks = BTreeMap::new();
        for kind in CARD_SLOT_KINDS
            .into_iter()
            .chain(DECK_SLOT_KINDS)
            .chain(STARSHIP_SLOT_KINDS)
        {
            if let Some(id) = settings.cosmetics.get(setting_key(kind)) {
                picks.insert(setting_key(kind).to_string(), id.clone());
            }
        }
        AccountPicks {
            picks,
            full_art: settings.full_art != Some(false),
        }
    }

    pub fn get(&self, kind: CosmeticKind) -> Option<&str> {
        self.picks.get(setting_key(kind)).map(String::as_str)
    }
}

impl DeckSlots {
    /// The pick of a kind: the playmat, the backdrop or the card back, absent for any other kind.
    pub fn get(&self, kind: CosmeticKind) -> Option<&str> {
        match kind {
            CosmeticKind::Table => self.table.as_deref(),
            CosmeticKind::Field => self.field.as_deref(),
            CosmeticKind::CardBack => self.card_back.as_deref(),
            _ => None,
        }
    }
}

/// R3013000: one row of an account's card proficiency (migration 0081, §14.14): how often it played the card
/// from hand in games that pay rewards, the level those plays reached, and the stamp the card wears, one its
/// level unlocked (R3013001), `None` for none. An existing stored shape: its widths are not changed
/// (README.md, the host contract's integers).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardProficiency {
    pub card_id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub plays: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub level: i64,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub stamp: Option<i64>,
}

/// R1834002: what one seat's loadout is frozen from, read as its match starts. A ladder bot's are
/// its wardrobe and the picks it drew (R1834005), read off its row, so a bot's seat freezes through
/// the same function a person's does. R3013002: and the stamps the account's cards wear, which a bot
/// has none of (R1454).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase", default)]
pub struct SeatPicks {
    pub owned: OwnedCosmetics,
    pub account: AccountPicks,
    pub cards: Vec<CardPicks>,
    pub stamps: Vec<CardStamp>,
}

impl SeatPicks {
    /// [`freeze_loadout`] against the shipped registry, for the deck `deck` slots and `deck_cards`,
    /// with the stamps those cards wear ([`freeze_stamps`]).
    pub fn freeze(&self, deck: Option<&DeckSlots>, deck_cards: &[String]) -> Loadout {
        Loadout {
            stamps: freeze_stamps(&self.stamps, deck_cards),
            ..freeze_loadout(
                registry(),
                &self.owned,
                &self.account,
                deck,
                &self.cards,
                deck_cards,
            )
        }
    }
}

/// R1834002: a seat's loadout as its match starts, from the account's ledger (`owned`), its settings'
/// picks (`account`), the deck's slots (`deck`, none for an All Random seat), its cards' own picks
/// (`card_picks`) and the cards of its deck (`deck_cards`). Every id that leaves is one of `registry`, of
/// its slot's kind, owned where it is worn; anything else is the kind's default:
///
///  - a deck slot wears the deck's pick when there is one and it is owned (the deck's word is final);
///    with no pick, the account's if owned;
///  - a card slot wears its kind's default, never the account's foil, finish or border;
///  - R3335000: a Starship slot wears the account's pick if owned (no deck carries one);
///  - each card of the deck wears its own pick of a kind when owned on the card (R1655000, R1953001);
///  - R3015005: it wears one level, frozen by its id with no augments: the level its own pick names, an older
///    shape mapped as [`level_of_pick`] maps it (a retired full art's id, `true`, a version with augments),
///    else Full art's first level on a full-art card while the Full-art cards setting is on; the highest of that
///    level and the ones below it that the card is offered and owns ([`worn_level`]), and none when no level is.
///    A card is listed only where it differs from the seat.
pub fn freeze_loadout(
    registry: &[Cosmetic],
    owned: &OwnedCosmetics,
    account: &AccountPicks,
    deck: Option<&DeckSlots>,
    card_picks: &[CardPicks],
    deck_cards: &[String],
) -> Loadout {
    let entry_of = |kind: CosmeticKind, id: &str| {
        registry
            .iter()
            .find(|cosmetic| cosmetic.kind == kind && cosmetic.id == id)
    };
    // `id` when it is of `kind` and owned where it is worn.
    let owned_pick = |kind: CosmeticKind, id: Option<&str>, card: Option<&str>| -> Option<String> {
        let cosmetic = entry_of(kind, id?)?;
        owns(cosmetic, card, owned).then(|| cosmetic.id.clone())
    };
    let default = |kind: CosmeticKind| default_of(registry, kind).to_string();

    let mut seat = SeatLoadout::defaults_of(registry);
    for kind in DECK_SLOT_KINDS {
        let worn = match deck.and_then(|deck| deck.get(kind)) {
            Some(picked) => owned_pick(kind, Some(picked), None),
            None => owned_pick(kind, account.get(kind), None),
        };
        seat.set(kind, worn.unwrap_or_else(|| default(kind)));
    }
    for kind in STARSHIP_SLOT_KINDS {
        let worn = owned_pick(kind, account.get(kind), None);
        seat.set(kind, worn.unwrap_or_else(|| default(kind)));
    }

    let distinct: BTreeSet<&str> = deck_cards.iter().map(String::as_str).collect();
    let mut cards = Vec::new();
    for card in distinct {
        let own = card_picks.iter().find(|picks| picks.card_id == card);
        let mut entry = CardPicks {
            card_id: card.to_string(),
            ..CardPicks::default()
        };
        for kind in CARD_SLOT_KINDS {
            let worn = own
                .and_then(|picks| owned_pick(kind, picks.get(kind), Some(card)))
                .unwrap_or_else(|| default(kind));
            if worn != seat.get(kind) {
                entry.set(kind, Some(worn));
            }
        }
        let pick = own
            .and_then(|picks| picks.full_art.clone())
            .unwrap_or(FullArtPick::On(account.full_art));
        let augments = own.map_or(&[][..], |picks| picks.augments.as_slice());
        let named = level_of_pick(registry, card, Some(&pick), augments);
        if let Some(level) = worn_level(registry, named, |level| {
            offered(level, Some(card)) && owns(level, Some(card), owned)
        }) {
            entry.full_art = Some(FullArtPick::Id(level.id.clone()));
        }
        if !entry.is_empty() {
            cards.push(entry);
        }
    }
    Loadout {
        seat,
        cards,
        stamps: Vec::new(),
    }
}

/// R3013002: the stamps of `stamps` that cards of the deck (`deck_cards`) wear, one per card, in card
/// order: a card the deck did not deal is never listed, so no frozen stamp names a card outside it.
pub fn freeze_stamps(stamps: &[CardStamp], deck_cards: &[String]) -> Vec<CardStamp> {
    let distinct: BTreeSet<&str> = deck_cards.iter().map(String::as_str).collect();
    distinct
        .into_iter()
        .filter_map(|card| stamps.iter().find(|stamp| stamp.card_id == card).cloned())
        .collect()
}

/// R3013001: the stamps the account's proficiency rows wear, each one its level unlocked.
pub fn worn_stamps(rows: &[CardProficiency]) -> Vec<CardStamp> {
    rows.iter()
        .filter_map(|row| {
            let stamp = row.stamp.filter(|stamp| *stamp <= row.level)?;
            Some(CardStamp {
                card_id: row.card_id.clone(),
                stamp,
            })
        })
        .collect()
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
    /// [`SeatPicks::freeze`]: a seat's loadout as its match starts, from what the host read of the seat
    /// (its ledger, settings' picks, card picks and stamps), the slots of the deck it plays (none for an All
    /// Random seat) and the cards of that deck. The shipped registry judges the picks.
    FreezeSeat {
        picks: Box<SeatPicks>,
        #[serde(default)]
        deck: Option<DeckSlots>,
        deck_cards: Vec<String>,
    },
    /// [`Loadout::defaults`]: what a seat wears when nothing is frozen for it.
    DefaultLoadout,
    /// [`AccountPicks::of_settings`]: the account's picks, from what its settings hold of them.
    AccountPicks {
        #[serde(default)]
        settings: Option<AccountSettings>,
    },
    /// [`worn_stamps`]: the stamps an account's proficiency rows wear.
    WornStamps { rows: Vec<CardProficiency> },
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
    /// `freezeSeat`, `defaultLoadout`: a seat's loadout.
    Frozen(Box<Loadout>),
    /// `accountPicks`: the account's picks.
    Account(AccountPicks),
    /// `wornStamps`: the stamps worn.
    Stamps(Vec<CardStamp>),
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
        LoadoutCall::FreezeSeat {
            picks,
            deck,
            deck_cards,
        } => LoadoutAnswer::Frozen(Box::new(picks.freeze(deck.as_ref(), deck_cards))),
        LoadoutCall::DefaultLoadout => LoadoutAnswer::Frozen(Box::new(Loadout::defaults())),
        LoadoutCall::AccountPicks { settings } => {
            LoadoutAnswer::Account(AccountPicks::of_settings(settings.as_ref()))
        }
        LoadoutCall::WornStamps { rows } => LoadoutAnswer::Stamps(worn_stamps(rows)),
    }
}
