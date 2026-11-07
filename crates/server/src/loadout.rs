//! The loadout each seat of a match wears (SPEC §14.2; R1834002, R1834003): what an opponent, a
//! watcher and a replay's viewer are shown of a player's cosmetics. Cosmetic only: nothing here
//! enters `PlayerView`, an action, the log, a replay hash or a game record, and the engine never
//! reads it (CLAUDE.md rule 7).
//!
//! Every decision about it is `jackioh-shared`'s (`loadout`, #3102, #3103), which the browser and Roblox
//! call through `jackioh-wasm-core`: what a seat wears when its match starts ([`freeze_loadout`],
//! [`SeatPicks::freeze`], R1834002), the stamps its cards wear ([`freeze_stamps`], [`worn_stamps`],
//! R3013002), and which of its cards' own picks a receiver is sent, and when ([`shown_defs`],
//! [`cards_due`], R1834003). This file names them as it always has, and keeps the store: it reads a
//! seat's picks off the ledger, the settings row and the proficiency rows ([`read_seat_picks`]) and
//! turns the rows into the plain records the decisions read ([`AccountPicksExt::of_row`],
//! [`DeckSlotsExt::of_deck`]).

pub use jackioh_shared::deck::DeckSlots;
pub use jackioh_shared::loadout::{
    AUGMENTS_KEY, AccountPicks, AccountSettings, CARD_SLOT_KINDS, CARDS_GROUP, COSMETICS_GROUP, CardStamp,
    DECK_SLOT_KINDS, FULL_ART_SETTING, Loadout, STARSHIP_SLOT_KINDS, SeatCards, SeatLoadout, SeatPicks, cards_due,
    freeze_loadout, freeze_stamps, setting_key, shown_defs, worn_stamps,
};

use crate::db::store::{BotRow, PlayerSettingValue, PlayerSettingsRow, SavedDeck, StoreResult, Tx};

/// What the server reads of a deck's [`DeckSlots`] off a saved deck's row.
pub trait DeckSlotsExt {
    /// A saved deck's three picks, or `None` when it carries none.
    fn of_deck(deck: &SavedDeck) -> Option<DeckSlots>;
}

impl DeckSlotsExt for DeckSlots {
    fn of_deck(deck: &SavedDeck) -> Option<DeckSlots> {
        let slots = DeckSlots {
            table: deck.table_cosmetic.clone(),
            field: deck.field_cosmetic.clone(),
            card_back: deck.card_back_cosmetic.clone(),
        };
        (slots != DeckSlots::default()).then_some(slots)
    }
}

/// What the server reads of the account's [`AccountPicks`] off its settings row.
pub trait AccountPicksExt {
    /// What `row` holds of the account's picks; no row holds none, and the switch is on.
    fn of_row(row: Option<&PlayerSettingsRow>) -> AccountPicks;
}

impl AccountPicksExt for AccountPicks {
    fn of_row(row: Option<&PlayerSettingsRow>) -> AccountPicks {
        AccountPicks::of_settings(row.map(settings_of).as_ref())
    }
}

/// The row's `cosmetics` group's text values and its `cards` group's Full-art cards switch, as the plain
/// record `AccountPicks::of_settings` reads (R633): a value of another type is left out.
fn settings_of(row: &PlayerSettingsRow) -> AccountSettings {
    let cosmetics = row
        .groups
        .get(COSMETICS_GROUP)
        .map(|group| {
            group
                .values
                .iter()
                .filter_map(|(key, value)| match value {
                    PlayerSettingValue::Text(id) => Some((key.clone(), id.clone())),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let full_art = match row
        .groups
        .get(CARDS_GROUP)
        .and_then(|group| group.values.get(FULL_ART_SETTING))
    {
        Some(PlayerSettingValue::Bool(on)) => Some(*on),
        _ => None,
    };
    AccountSettings { cosmetics, full_art }
}

/// The picks of `profile_id`'s seat inside the caller's transaction: `bot`'s own when the seat is a ladder
/// bot's (none before it drew any, R1834005), the account's ledger, settings and card picks otherwise
/// (R3013002: and the stamps its proficiency rows wear). The seat then freezes through
/// [`SeatPicks::freeze`], a bot's through the same function a person's does (R1834002).
pub async fn read_seat_picks(
    t: &mut Tx<'_>,
    profile_id: &str,
    bot: Option<&BotRow>,
) -> StoreResult<SeatPicks> {
    if let Some(bot) = bot {
        return Ok(bot
            .cosmetics
            .as_ref()
            .map(|cosmetics| SeatPicks {
                owned: cosmetics.wardrobe.clone(),
                account: cosmetics.account.clone(),
                cards: cosmetics.cards.clone(),
                stamps: Vec::new(),
            })
            .unwrap_or_default());
    }
    let owned = t.economy_cosmetics_get(profile_id).await?;
    let settings = t.player_settings_get(profile_id).await?;
    let cards = t.economy_card_picks_get(profile_id).await?;
    let proficiency = t.economy_card_proficiency_get(profile_id).await?;
    Ok(SeatPicks {
        owned,
        account: AccountPicks::of_row(settings.as_ref()),
        cards,
        stamps: worn_stamps(&proficiency),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmetics::CosmeticKind;

    /// The decisions' own tests are `jackioh-shared`'s (`tests/loadout_freeze.rs`); this holds the row read.
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
        let picks = AccountPicks::of_row(Some(&row));
        assert_eq!(picks.get(CosmeticKind::Foil), Some("foil-starter"));
        assert_eq!(picks.get(CosmeticKind::CardBack), Some("card-back-starter"));
        assert_eq!(picks.get(CosmeticKind::Border), None);
        assert_eq!(picks.get(CosmeticKind::Table), None);
        assert!(!picks.full_art);
        // No row: nothing picked and the switch on.
        assert_eq!(AccountPicks::of_row(None), AccountPicks::default());
    }
}
