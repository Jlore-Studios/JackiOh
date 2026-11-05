//! What a ticket, a room or a series freezes of a player's decks, and the seat a match starts from
//! (R259, R642, R1834002). These are the shapes the server stores and the series rules read, so a
//! change to one is a change to a stored row.

use jackioh_engine::PlayerId;
use serde::{Deserialize, Serialize};

/// R1834002: a deck's playmat, backdrop and card back as a ticket, a room or a trio freezes them
/// with the deck, each an id or absent (the account's pick, then the kind's default).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DeckSlots {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub card_back: Option<String>,
}

/// One deck as a match or a series freezes it: the cards and the name the player gave them. `portrait`
/// freezes the deck's hero portrait with them (R642), and R1834002's `slots` its playmat, backdrop and card
/// back; each is absent on rows frozen before it existed or on a deck that carried none, and absence stays
/// absent (a missing `portrait` reads as `vanilla` downstream).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FrozenDeck {
    pub name: String,
    pub cards: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::absent_or_null"
    )]
    #[cfg_attr(feature = "ts", ts(optional, type = "string | null"))]
    pub portrait: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slots: Option<DeckSlots>,
}

/// R259: a Conquest player's trio, frozen at enqueue (or at room create/join).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FrozenTrio {
    pub name: String,
    pub decks: (FrozenDeck, FrozenDeck, FrozenDeck),
}

/// One seat of a match as it starts: who, which seat and the deck (R642, R1834002).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MatchSeat {
    pub profile_id: String,
    pub player: PlayerId,
    pub deck: Vec<String>,
    /// R642: the portrait dealt to this seat at match creation. Absent reads as `vanilla`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub portrait: Option<String>,
    /// R1834002: the slots of the deck this seat plays, frozen with it (a ticket's, a room's, a
    /// trio's picked deck's, a bot's); absent for none, and for an All Random seat, which wears the
    /// account's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub deck_slots: Option<DeckSlots>,
}
