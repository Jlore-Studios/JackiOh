//! The loadout each seat of a match wears (SPEC §14.2; R1834002, R1834003): what an opponent, a
//! watcher and a replay's viewer are shown of a player's cosmetics. Cosmetic only: nothing here
//! enters `PlayerView`, an action, the log, a replay hash or a game record, and the engine never
//! reads it (CLAUDE.md rule 7).
//!
//! R1834002: when a match starts, each seat's loadout is frozen on its row ([`freeze_loadout`]):
//! the deck's three slots (the deck's pick, else the account's, else the kind's default) and the
//! picks of each card in the deck, each resolved to a registry id the account owns at that moment
//! and the kind's default otherwise, so no unknown or unowned id ever reaches another client. A card
//! with no pick of a kind wears its default, never the account's (R1953001), so the seat's foil,
//! finish and border are always the defaults. A row from before #1834 reads as [`Loadout::defaults`].
//!
//! R1834003: the seat-level half travels whole, on join and on reconnect (`loadouts`). A card's own
//! picks travel only once a card dealt from that seat's deck, still showing as what it was dealt,
//! is in the receiver's view ([`shown_defs`]), and once per receiver ([`cards_due`]), so the frames
//! never name a card the receiver has not seen and no decklist can be read from them (§10.8).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use jackioh_engine::{BackrowView, HandView, PLAYER_IDS, PlayerId, PlayerView, SideView};

use crate::cosmetics::{Cosmetic, CosmeticKind, CosmeticRarity, default_of, full_art_picked, owns, registry};
use crate::db::store::{BotRow, PlayerSettingValue, PlayerSettingsRow, SavedDeck, StoreResult, Tx};
use crate::economy::store::{CardPicks, FullArtPick, OwnedCosmetics};

/// The settings group the client keeps the account's picks in (R633, R1295000).
pub const COSMETICS_GROUP: &str = "cosmetics";
/// The settings group of the card display, and its Full-art cards switch (R1313000).
pub const CARDS_GROUP: &str = "cards";
pub const FULL_ART_SETTING: &str = "fullArt";

/// The kinds a deck carries a pick of (R1297000), in the registry's order.
pub const DECK_SLOT_KINDS: [CosmeticKind; 3] =
    [CosmeticKind::Table, CosmeticKind::Field, CosmeticKind::CardBack];
/// The kinds a card wears (R1295000), in the registry's order.
pub const CARD_SLOT_KINDS: [CosmeticKind; 3] =
    [CosmeticKind::Foil, CosmeticKind::Finish, CosmeticKind::Border];

/// The key a kind's pick has in the `cosmetics` settings group: the kind's own camel-case name.
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
    }
}

/// R1834002: a deck's playmat, backdrop and card back as a ticket, a room or a trio freezes them
/// with the deck, each an id or absent (the account's pick, then the kind's default).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeckSlots {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_back: Option<String>,
}

impl DeckSlots {
    /// A saved deck's three picks, or `None` when it carries none.
    pub fn of_deck(deck: &SavedDeck) -> Option<DeckSlots> {
        let slots = DeckSlots {
            table: deck.table_cosmetic.clone(),
            field: deck.field_cosmetic.clone(),
            card_back: deck.card_back_cosmetic.clone(),
        };
        (slots != DeckSlots::default()).then_some(slots)
    }

    /// The deck's pick of `kind`, a deck slot's kind.
    pub fn get(&self, kind: CosmeticKind) -> Option<&str> {
        match kind {
            CosmeticKind::Table => self.table.as_deref(),
            CosmeticKind::Field => self.field.as_deref(),
            CosmeticKind::CardBack => self.card_back.as_deref(),
            _ => None,
        }
    }
}

/// The account's own picks as its settings hold them (R633): one text per kind in the
/// `cosmetics` group, by its key there ([`setting_key`]), any of them absent, and the Full-art
/// cards switch, on unless it is off. A ladder bot keeps its own the same way (R1834005).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

impl AccountPicks {
    /// What `row` holds of the account's picks; no row holds none, and the switch is on.
    pub fn of_settings(row: Option<&PlayerSettingsRow>) -> AccountPicks {
        let Some(row) = row else {
            return AccountPicks::default();
        };
        let mut picks = BTreeMap::new();
        if let Some(group) = row.groups.get(COSMETICS_GROUP) {
            for kind in CARD_SLOT_KINDS.into_iter().chain(DECK_SLOT_KINDS) {
                if let Some(PlayerSettingValue::Text(id)) = group.values.get(setting_key(kind)) {
                    picks.insert(setting_key(kind).to_string(), id.clone());
                }
            }
        }
        let full_art = !matches!(
            row.groups
                .get(CARDS_GROUP)
                .and_then(|group| group.values.get(FULL_ART_SETTING)),
            Some(PlayerSettingValue::Bool(false))
        );
        AccountPicks { picks, full_art }
    }

    /// The account's pick of `kind`.
    pub fn get(&self, kind: CosmeticKind) -> Option<&str> {
        self.picks.get(setting_key(kind)).map(String::as_str)
    }
}

/// R1834002: what a seat wears on its side and on every card that has no pick of its own, each a
/// registry id of its kind: the `loadouts` frame's half for one seat. Its foil, finish and border
/// are their kinds' defaults (R1953001), kept so the frame keeps its shape.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
    /// Every kind's default in `registry`.
    pub fn defaults_of(registry: &[Cosmetic]) -> SeatLoadout {
        SeatLoadout {
            table: default_of(registry, CosmeticKind::Table).to_string(),
            field: default_of(registry, CosmeticKind::Field).to_string(),
            card_back: default_of(registry, CosmeticKind::CardBack).to_string(),
            foil: default_of(registry, CosmeticKind::Foil).to_string(),
            finish: default_of(registry, CosmeticKind::Finish).to_string(),
            border: default_of(registry, CosmeticKind::Border).to_string(),
        }
    }

    /// The seat's id of `kind`, a deck or card slot's kind.
    pub fn get(&self, kind: CosmeticKind) -> &str {
        match kind {
            CosmeticKind::Table => &self.table,
            CosmeticKind::Field => &self.field,
            CosmeticKind::CardBack => &self.card_back,
            CosmeticKind::Foil => &self.foil,
            CosmeticKind::Finish => &self.finish,
            CosmeticKind::Border => &self.border,
            CosmeticKind::FullArt | CosmeticKind::Portrait => "",
        }
    }

    fn set(&mut self, kind: CosmeticKind, id: String) {
        match kind {
            CosmeticKind::Table => self.table = id,
            CosmeticKind::Field => self.field = id,
            CosmeticKind::CardBack => self.card_back = id,
            CosmeticKind::Foil => self.foil = id,
            CosmeticKind::Finish => self.finish = id,
            CosmeticKind::Border => self.border = id,
            CosmeticKind::FullArt | CosmeticKind::Portrait => {}
        }
    }

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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Loadout {
    pub seat: SeatLoadout,
    #[serde(default)]
    pub cards: Vec<CardPicks>,
}

impl Loadout {
    /// What a seat wears when nothing is frozen for it: every kind's default, no card of its own.
    pub fn defaults() -> Loadout {
        Loadout {
            seat: SeatLoadout::defaults_of(registry()),
            cards: Vec::new(),
        }
    }

    /// The card `card_id`'s own entry, if it has one.
    pub fn card(&self, card_id: &str) -> Option<&CardPicks> {
        self.cards.iter().find(|card| card.card_id == card_id)
    }
}

/// R1834003: one seat's cards, as the `cardLoadouts` frame and a replay page carry them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeatCards {
    pub seat: PlayerId,
    pub cards: Vec<CardPicks>,
}

impl CardPicks {
    /// The card's own pick of `kind`, a card slot's kind.
    pub fn get(&self, kind: CosmeticKind) -> Option<&str> {
        match kind {
            CosmeticKind::Foil => self.foil.as_deref(),
            CosmeticKind::Finish => self.finish.as_deref(),
            CosmeticKind::Border => self.border.as_deref(),
            _ => None,
        }
    }

    /// Sets the card's own pick of `kind`, a card slot's kind.
    pub fn set(&mut self, kind: CosmeticKind, id: Option<String>) {
        match kind {
            CosmeticKind::Foil => self.foil = id,
            CosmeticKind::Finish => self.finish = id,
            CosmeticKind::Border => self.border = id,
            _ => {}
        }
    }
}

/// R1834002: a seat's loadout as its match starts, from the account's ledger (`owned`), its
/// settings' picks (`account`), the deck's slots (`deck`, none for an All Random seat), its cards'
/// own picks (`card_picks`) and the cards of its deck (`deck_cards`). Every id that leaves is one
/// of `registry`, of its slot's kind, that the account owns where it is worn; anything else is the
/// kind's default:
///
///  - a deck slot wears the deck's pick when the deck carries one, owned, and the default when it
///    is not (the deck's word is final, as the deck builder draws it); else the account's pick,
///    owned; else the default;
///  - a card slot wears its kind's default: since R1953001 a card with no pick of its own never
///    wears the account's foil, finish or border, which nothing sets any more;
///  - each card of the deck wears its own pick of a kind when that is owned on the card, and the
///    default when it has none, does not own it or picks an id the registry has none of for its
///    kind, as its owner's client draws it (R1655000, R1953001). Its full art is the one its own
///    pick names, else its Epic when the account's Full-art cards setting is on, and prints when
///    the account owns that full art on the card: `true` for the Epic, its id for a Legendary or a
///    Mythic (R2655000). A card is listed only where it differs from the seat.
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
        if let Some(full_art) =
            full_art_picked(registry, card, &pick).filter(|full_art| owns(full_art, Some(card), owned))
        {
            entry.full_art = Some(if full_art.rarity == CosmeticRarity::Epic {
                FullArtPick::On(true)
            } else {
                FullArtPick::Id(full_art.id.clone())
            });
        }
        if !entry.is_empty() {
            cards.push(entry);
        }
    }
    Loadout { seat, cards }
}

/// R1834002: what one seat's loadout is frozen from, read as its match starts: what the account
/// owns, its settings' picks and its cards' own picks. A ladder bot's are its wardrobe and the picks
/// it drew (R1834005), read off its row, so a bot's seat freezes through the same function a
/// person's does.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeatPicks {
    pub owned: OwnedCosmetics,
    pub account: AccountPicks,
    pub cards: Vec<CardPicks>,
}

impl SeatPicks {
    /// The picks of `profile_id`'s seat inside the caller's transaction: `bot`'s own when the seat
    /// is a ladder bot's (none before it drew any), the account's ledger, settings and card picks
    /// otherwise.
    pub async fn read(t: &mut Tx<'_>, profile_id: &str, bot: Option<&BotRow>) -> StoreResult<SeatPicks> {
        if let Some(bot) = bot {
            return Ok(bot
                .cosmetics
                .as_ref()
                .map(|cosmetics| SeatPicks {
                    owned: cosmetics.wardrobe.clone(),
                    account: cosmetics.account.clone(),
                    cards: cosmetics.cards.clone(),
                })
                .unwrap_or_default());
        }
        let owned = t.economy_cosmetics_get(profile_id).await?;
        let settings = t.player_settings_get(profile_id).await?;
        let cards = t.economy_card_picks_get(profile_id).await?;
        Ok(SeatPicks {
            owned,
            account: AccountPicks::of_settings(settings.as_ref()),
            cards,
        })
    }

    /// [`freeze_loadout`] against the shipped registry, for the deck `deck` slots and `deck_cards`.
    pub fn freeze(&self, deck: Option<&DeckSlots>, deck_cards: &[String]) -> Loadout {
        freeze_loadout(
            registry(),
            &self.owned,
            &self.account,
            deck,
            &self.cards,
            deck_cards,
        )
    }
}

/// The cards a side of a view shows face up, each `(instance id, def id)`: its units, carried
/// units and face-up backrow, its graveyard, exile and resolving cards, and its hand where the
/// view lists it. A face-down card, the opponent's hand, a library, the events and the prompts
/// name nothing here.
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

/// R1834003: the `(seat, card)` pairs `view` shows: each card in it that was dealt from `seat`'s
/// deck (`dealt`, `actor::engine::dealt_cards`) and still shows as the card it was dealt as,
/// wherever it is now (a card that changed hands counts for the deck that dealt it). A card minted
/// later and one transformed into another card name no pair, so a pair never says more about a
/// deck than the view it is read from: a card of that deck, seen.
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
        let Some(entry) = loadout.card(card) else {
            continue;
        };
        match due.iter_mut().find(|cards| cards.seat == *seat) {
            Some(cards) => cards.cards.push(entry.clone()),
            None => due.push(SeatCards {
                seat: *seat,
                cards: vec![entry.clone()],
            }),
        }
    }
    due.sort_by_key(|cards| PLAYER_IDS.iter().position(|seat| *seat == cards.seat));
    due
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmetics::CosmeticRarity;
    use crate::economy::store::CardCosmetic;

    fn entry(id: &str, kind: CosmeticKind, default: bool, starter: bool) -> Cosmetic {
        Cosmetic {
            id: id.to_owned(),
            kind,
            name: id.to_owned(),
            rarity: if default || kind.is_per_card() {
                CosmeticRarity::Common
            } else {
                CosmeticRarity::Epic
            },
            default,
            starter,
            chase: false,
            card: None,
        }
    }

    /// Each kind's default, one starter and one earned item per slot kind, and a full art.
    fn test_registry() -> Vec<Cosmetic> {
        let mut cosmetics = Vec::new();
        for (kind, prefix) in [
            (CosmeticKind::Foil, "foil"),
            (CosmeticKind::Finish, "finish"),
            (CosmeticKind::Border, "border"),
            (CosmeticKind::Table, "table"),
            (CosmeticKind::Field, "field"),
            (CosmeticKind::CardBack, "card-back"),
        ] {
            cosmetics.push(entry(&format!("{prefix}-default"), kind, true, true));
            cosmetics.push(entry(&format!("{prefix}-starter"), kind, false, true));
            cosmetics.push(entry(&format!("{prefix}-earned"), kind, false, false));
        }
        cosmetics.push(Cosmetic {
            card: Some("core-001".to_owned()),
            rarity: CosmeticRarity::Epic,
            ..entry("full-art-core-001", CosmeticKind::FullArt, false, false)
        });
        cosmetics
    }

    fn account(pairs: &[(CosmeticKind, &str)], full_art: bool) -> AccountPicks {
        AccountPicks {
            picks: pairs
                .iter()
                .map(|(kind, id)| (setting_key(*kind).to_owned(), (*id).to_owned()))
                .collect(),
            full_art,
        }
    }

    fn cards(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn r1834002_deck_then_account_then_default() {
        let registry = test_registry();
        let owned = OwnedCosmetics::default();
        let picks = account(
            &[
                (CosmeticKind::Table, "table-starter"),
                (CosmeticKind::Field, "field-starter"),
                (CosmeticKind::Foil, "foil-starter"),
            ],
            true,
        );
        // The deck's pick beats the account's; the account's fills a slot the deck leaves empty;
        // the default fills one neither picks.
        let deck = DeckSlots {
            table: Some("table-default".to_owned()),
            field: None,
            card_back: None,
        };
        let loadout = freeze_loadout(&registry, &owned, &picks, Some(&deck), &[], &cards(&["core-002"]));
        assert_eq!(loadout.seat.table, "table-default");
        assert_eq!(loadout.seat.field, "field-starter");
        assert_eq!(loadout.seat.card_back, "card-back-default");
        // R1953001: a card slot is its default, whatever the account's settings still hold.
        assert_eq!(loadout.seat.foil, "foil-default");
        assert_eq!(loadout.seat.finish, "finish-default");
        assert!(loadout.cards.is_empty());
        // No deck (All Random): the account's, then the default.
        let random = freeze_loadout(&registry, &owned, &picks, None, &[], &[]);
        assert_eq!(random.seat.table, "table-starter");
        // No settings at all: every default, the full-art switch on.
        let bare = AccountPicks::of_settings(None);
        assert!(bare.full_art);
        assert_eq!(
            freeze_loadout(&registry, &owned, &bare, None, &[], &[]).seat,
            SeatLoadout::defaults_of(&registry)
        );
    }

    #[test]
    fn r1834002_an_unknown_or_unowned_pick_freezes_as_the_default() {
        let registry = test_registry();
        let none = OwnedCosmetics::default();
        let picks = account(
            &[
                (CosmeticKind::Table, "table-earned"),
                (CosmeticKind::Field, "field-unheard-of"),
                (CosmeticKind::CardBack, "table-starter"),
                (CosmeticKind::Border, "border-earned"),
            ],
            true,
        );
        let loadout = freeze_loadout(&registry, &none, &picks, None, &[], &[]);
        assert_eq!(loadout.seat, SeatLoadout::defaults_of(&registry));
        // An unowned deck pick is the default too, never the account's pick under it.
        let deck = DeckSlots {
            table: Some("table-earned".to_owned()),
            ..DeckSlots::default()
        };
        let starter = account(&[(CosmeticKind::Table, "table-starter")], true);
        assert_eq!(
            freeze_loadout(&registry, &none, &starter, Some(&deck), &[], &[])
                .seat
                .table,
            "table-default"
        );
        // Owned on the account, the same picks are worn.
        let owned = OwnedCosmetics {
            cards: Vec::new(),
            account: vec!["table-earned".to_owned()],
        };
        assert_eq!(
            freeze_loadout(&registry, &owned, &picks, None, &[], &[])
                .seat
                .table,
            "table-earned"
        );
        // A per-card kind is never owned on the account, so an earned border picked for every card
        // is the default on the seat.
        assert_eq!(
            freeze_loadout(&registry, &owned, &picks, None, &[], &[])
                .seat
                .border,
            "border-default"
        );
    }

    #[test]
    fn r1834002_per_card_only_for_owned_deck_cards() {
        let registry = test_registry();
        let owned = OwnedCosmetics {
            cards: vec![
                CardCosmetic {
                    card_id: "core-001".to_owned(),
                    cosmetic_id: "full-art-core-001".to_owned(),
                },
                CardCosmetic {
                    card_id: "core-002".to_owned(),
                    cosmetic_id: "foil-earned".to_owned(),
                },
            ],
            account: Vec::new(),
        };
        let picks = account(&[(CosmeticKind::Finish, "finish-starter")], true);
        let card_picks = vec![
            // Owned on its card: worn.
            CardPicks {
                card_id: "core-002".to_owned(),
                foil: Some("foil-earned".to_owned()),
                ..CardPicks::default()
            },
            // Not owned on this card: the default, which the seat already wears.
            CardPicks {
                card_id: "core-003".to_owned(),
                foil: Some("foil-earned".to_owned()),
                ..CardPicks::default()
            },
            // The account's own finish: the card's pick, since the seat wears the default (R1953001).
            CardPicks {
                card_id: "core-004".to_owned(),
                finish: Some("finish-starter".to_owned()),
                ..CardPicks::default()
            },
            // An id the registry does not know: the default, the card's word final.
            CardPicks {
                card_id: "core-005".to_owned(),
                finish: Some("finish-from-a-newer-client".to_owned()),
                ..CardPicks::default()
            },
            // Not in the deck: never frozen.
            CardPicks {
                card_id: "core-009".to_owned(),
                border: Some("border-starter".to_owned()),
                ..CardPicks::default()
            },
        ];
        let deck = cards(&[
            "core-001", "core-002", "core-002", "core-003", "core-004", "core-005",
        ]);
        let loadout = freeze_loadout(&registry, &owned, &picks, None, &card_picks, &deck);
        assert_eq!(loadout.seat.finish, "finish-default");
        assert_eq!(
            loadout.cards,
            vec![
                CardPicks {
                    card_id: "core-001".to_owned(),
                    full_art: Some(FullArtPick::On(true)),
                    ..CardPicks::default()
                },
                CardPicks {
                    card_id: "core-002".to_owned(),
                    foil: Some("foil-earned".to_owned()),
                    ..CardPicks::default()
                },
                CardPicks {
                    card_id: "core-004".to_owned(),
                    finish: Some("finish-starter".to_owned()),
                    ..CardPicks::default()
                },
            ]
        );
        // The full art follows the card's own switch first, then the account's setting, and needs
        // the cosmetic owned.
        let off = account(&[], false);
        assert!(
            freeze_loadout(&registry, &owned, &off, None, &[], &deck)
                .card("core-001")
                .is_none()
        );
        let on_for_the_card = vec![CardPicks {
            card_id: "core-001".to_owned(),
            full_art: Some(FullArtPick::On(true)),
            ..CardPicks::default()
        }];
        assert_eq!(
            freeze_loadout(&registry, &owned, &off, None, &on_for_the_card, &deck)
                .card("core-001")
                .and_then(|card| card.full_art.clone()),
            Some(FullArtPick::On(true))
        );
        assert!(
            freeze_loadout(
                &registry,
                &OwnedCosmetics::default(),
                &picks,
                None,
                &on_for_the_card,
                &deck
            )
            .card("core-001")
            .is_none()
        );
    }

    #[test]
    fn r2655000_a_card_freezes_its_picked_tier_when_owned_and_nothing_when_not() {
        let mut registry = test_registry();
        for (id, rarity) in [
            ("full-art-core-001-animated", CosmeticRarity::Legendary),
            ("full-art-core-001-signed", CosmeticRarity::Mythic),
        ] {
            registry.push(Cosmetic {
                card: Some("core-001".to_owned()),
                rarity,
                chase: true,
                ..entry(id, CosmeticKind::FullArt, false, false)
            });
        }
        let owning = |ids: &[&str]| OwnedCosmetics {
            cards: ids
                .iter()
                .map(|id| CardCosmetic {
                    card_id: "core-001".to_owned(),
                    cosmetic_id: (*id).to_owned(),
                })
                .collect(),
            account: Vec::new(),
        };
        let deck = cards(&["core-001", "core-002"]);
        let frozen = |owned: &OwnedCosmetics, account_on: bool, pick: Option<FullArtPick>| {
            let picks: Vec<CardPicks> = pick
                .map(|pick| CardPicks {
                    card_id: "core-001".to_owned(),
                    full_art: Some(pick),
                    ..CardPicks::default()
                })
                .into_iter()
                .collect();
            freeze_loadout(&registry, owned, &account(&[], account_on), None, &picks, &deck)
                .card("core-001")
                .and_then(|card| card.full_art.clone())
        };
        let id = |id: &str| FullArtPick::Id(id.to_owned());
        let every = owning(&[
            "full-art-core-001",
            "full-art-core-001-animated",
            "full-art-core-001-signed",
        ]);
        // An owned Legendary or Mythic freezes as its id; the Epic as `true`, whether picked by its
        // id or by `true`; with the setting off the card's own pick still prints it.
        assert_eq!(
            frozen(&every, false, Some(id("full-art-core-001-signed"))),
            Some(id("full-art-core-001-signed"))
        );
        assert_eq!(
            frozen(&every, true, Some(id("full-art-core-001-animated"))),
            Some(id("full-art-core-001-animated"))
        );
        assert_eq!(
            frozen(&every, true, Some(id("full-art-core-001"))),
            Some(FullArtPick::On(true))
        );
        assert_eq!(
            frozen(&every, false, Some(FullArtPick::On(true))),
            Some(FullArtPick::On(true))
        );
        // With no pick of its own the card wears its Epic when the setting is on, never a rarer tier.
        assert_eq!(frozen(&every, true, None), Some(FullArtPick::On(true)));
        assert_eq!(frozen(&every, false, None), None);
        assert_eq!(frozen(&owning(&["full-art-core-001-signed"]), true, None), None);
        // A tier not owned, another card's, an unknown id or `false` freeze nothing.
        let epic_only = owning(&["full-art-core-001"]);
        assert_eq!(
            frozen(&epic_only, true, Some(id("full-art-core-001-signed"))),
            None
        );
        assert_eq!(frozen(&every, true, Some(id("full-art-core-002-signed"))), None);
        assert_eq!(
            frozen(&every, true, Some(id("full-art-from-a-newer-client"))),
            None
        );
        assert_eq!(frozen(&every, true, Some(FullArtPick::On(false))), None);
        // The frozen pick travels as `true` or the id's text, as the client reads `fullArt`.
        assert_eq!(
            serde_json::to_value(FullArtPick::On(true)).ok(),
            Some(serde_json::json!(true))
        );
        assert_eq!(
            serde_json::to_value(id("full-art-core-001-signed")).ok(),
            Some(serde_json::json!("full-art-core-001-signed"))
        );
    }

    #[test]
    fn r1834002_the_settings_row_reads_the_cosmetics_group_and_the_full_art_switch() {
        use crate::db::store::PlayerSettingsGroup;
        use indexmap::IndexMap;
        let group = |values: &[(&str, PlayerSettingValue)]| PlayerSettingsGroup {
            at: 1,
            values: values
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
        };
        let mut groups = IndexMap::new();
        groups.insert(
            COSMETICS_GROUP.to_owned(),
            group(&[
                ("foil", PlayerSettingValue::Text("foil-starter".to_owned())),
                (
                    "cardBack",
                    PlayerSettingValue::Text("card-back-starter".to_owned()),
                ),
                ("border", PlayerSettingValue::Bool(true)),
            ]),
        );
        groups.insert(
            CARDS_GROUP.to_owned(),
            group(&[(FULL_ART_SETTING, PlayerSettingValue::Bool(false))]),
        );
        let row = PlayerSettingsRow {
            profile_id: "p".to_owned(),
            groups,
        };
        let picks = AccountPicks::of_settings(Some(&row));
        assert_eq!(picks.get(CosmeticKind::Foil), Some("foil-starter"));
        assert_eq!(picks.get(CosmeticKind::CardBack), Some("card-back-starter"));
        assert_eq!(picks.get(CosmeticKind::Border), None);
        assert_eq!(picks.get(CosmeticKind::Table), None);
        assert!(!picks.full_art);
    }

    #[test]
    fn r1834003_shown_defs_follow_the_deal_and_skip_hidden() {
        use crate::actor::engine::{self, Pinned};
        let decks = (
            engine::deal_random_deck("loadout-test:p1-deck", None),
            engine::deal_random_deck("loadout-test:p2-deck", None),
        );
        let args = engine::create_game_args("loadout-test", &decks, None, None, None);
        let state =
            engine::begin_game(Pinned::default(), &engine::create_game(Pinned::default(), &args)).state;
        let dealt = engine::dealt_cards(&state);
        // Every card of both decks was dealt, each to its own seat.
        assert_eq!(dealt.len(), decks.0.len() + decks.1.len());
        for (seat, deck) in [(PlayerId::P1, &decks.0), (PlayerId::P2, &decks.1)] {
            let mut of_seat: Vec<&str> = dealt
                .values()
                .filter(|(owner, _)| *owner == seat)
                .map(|(_, def)| def.as_str())
                .collect();
            let mut listed: Vec<&str> = deck.iter().map(String::as_str).collect();
            of_seat.sort_unstable();
            listed.sort_unstable();
            assert_eq!(of_seat, listed, "{seat}");
        }
        let view = engine::view_for(Pinned::default(), &state, PlayerId::P1);
        let HandView::Cards(hand) = &view.you.hand else {
            panic!("p1 reads its own hand");
        };
        assert!(!hand.is_empty());
        let shown = shown_defs(&view, &dealt);
        // p1's hand shows, p2's hand and both libraries do not.
        let expected: BTreeSet<(PlayerId, String)> = hand
            .iter()
            .map(|card| (PlayerId::P1, card.def_id.clone()))
            .collect();
        assert_eq!(shown, expected);
        // A card that shows as something it was not dealt as (a transform), or that was not dealt
        // at all, names nothing.
        let first = hand[0].def_id.clone();
        let mut changed = dealt.clone();
        for card in hand.iter().filter(|card| card.def_id == first) {
            changed.insert(card.instance_id.clone(), (PlayerId::P1, "core-999".to_owned()));
        }
        let transformed = shown_defs(&view, &changed);
        assert!(!transformed.contains(&(PlayerId::P1, first)));
        assert_eq!(transformed.len() + 1, shown.len());
        assert!(shown_defs(&view, &BTreeMap::new()).is_empty());
    }

    #[test]
    fn r1834003_cards_due_sends_each_pair_once_and_only_entries() {
        let mut first = Loadout::defaults();
        first.cards.push(CardPicks {
            card_id: "core-001".to_owned(),
            full_art: Some(FullArtPick::On(true)),
            ..CardPicks::default()
        });
        let mut second = Loadout::defaults();
        second.cards.push(CardPicks {
            card_id: "core-002".to_owned(),
            foil: Some("foil-cracked-ice".to_owned()),
            ..CardPicks::default()
        });
        let loadouts = (first, second);
        let mut sent = BTreeSet::new();
        let shown: BTreeSet<(PlayerId, String)> = [
            (PlayerId::P2, "core-002".to_owned()),
            (PlayerId::P1, "core-001".to_owned()),
            (PlayerId::P1, "core-003".to_owned()),
        ]
        .into_iter()
        .collect();
        let due = cards_due(&shown, &loadouts, &mut sent);
        assert_eq!(
            due,
            vec![
                SeatCards {
                    seat: PlayerId::P1,
                    cards: vec![loadouts.0.cards[0].clone()],
                },
                SeatCards {
                    seat: PlayerId::P2,
                    cards: vec![loadouts.1.cards[0].clone()],
                },
            ]
        );
        assert_eq!(sent.len(), 3);
        // Nothing twice.
        assert!(cards_due(&shown, &loadouts, &mut sent).is_empty());
        // The seat's picks for core-002 never travel for p1's core-002.
        let mut fresh = BTreeSet::new();
        let theirs_only: BTreeSet<(PlayerId, String)> =
            [(PlayerId::P1, "core-002".to_owned())].into_iter().collect();
        assert!(cards_due(&theirs_only, &loadouts, &mut fresh).is_empty());
    }
}
