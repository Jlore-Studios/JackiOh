//! The Assault's content ([[R4579001]]): the set's cards by set and tag, and the shipped
//! stand-ins until the release. Parsed on each call: a pure crate keeps no statics.

use jackioh_engine::catalog::deckable;
use jackioh_engine::wire::CardDefs;
use serde::Deserialize;

use crate::assault::{AssaultCard, AssaultCommander, AssaultContent, AssaultDoctrine};
use crate::config::{ASSAULT_COMMANDER_TAG, ASSAULT_SET};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContentFile {
    pub(super) doctrines: Vec<AssaultDoctrine>,
    pub(super) commanders: Vec<AssaultCommander>,
    pub(super) stand_in_commanders: Vec<AssaultCommander>,
}

/// `assault.json`: the doctrines, the Commanders (until C10 writes the real ones) and the
/// stand-ins. Parsed on each call, as `run_placeholder_content` parses its file.
pub(super) fn content_file() -> ContentFile {
    serde_json::from_str(include_str!("../assault.json")).expect("assault.json is valid content")
}

/// IA4: the set's cards (`ASSAULT_SET`) by tag: a card's doctrine is the first of the file's
/// doctrines whose id one of its tags names, and a card is a Commander's when it carries
/// `ASSAULT_COMMANDER_TAG`. A token never deals. The Commanders are
/// the file's whose every card is a Commander card here, so no Commander ships half its cards.
pub fn assault_catalog_content(catalog: &CardDefs) -> AssaultContent {
    let file = content_file();
    let mut cards = Vec::new();
    for def in catalog.values() {
        if def.set != ASSAULT_SET || def.is_token() {
            continue;
        }
        let doctrine = file
            .doctrines
            .iter()
            .find(|doctrine| def.tags.iter().any(|tag| tag.as_str() == doctrine.id))
            .map(|doctrine| doctrine.id.clone());
        let commander = def.tags.contains(&ASSAULT_COMMANDER_TAG);
        cards.push(AssaultCard {
            id: def.id.clone(),
            rarity: def.rarity,
            doctrine,
            commander,
        });
    }
    let commanders = file
        .commanders
        .into_iter()
        .filter(|commander| {
            commander
                .cards
                .iter()
                .all(|id| cards.iter().any(|card| &card.id == id && card.commander))
        })
        .collect();
    AssaultContent {
        cards,
        doctrines: file.doctrines,
        commanders,
    }
}

/// IA4: legal decks from shipped cards standing in for each doctrine, so the mode is whole before
/// the set is: the stand-ins whose every card is `deckable` in the catalog, every `deckable`
/// card in catalog order, a stand-in's cards flagged with its doctrine, and every other card
/// dealt the next group in turn (neutral, then the doctrines in file order).
pub fn assault_placeholder_content(catalog: &CardDefs) -> AssaultContent {
    let file = content_file();
    let commanders: Vec<AssaultCommander> = file
        .stand_in_commanders
        .into_iter()
        .filter(|commander| {
            commander
                .cards
                .iter()
                .all(|id| catalog.get(id).is_some_and(deckable))
        })
        .collect();
    let mut cards = Vec::new();
    let mut counter = 0;
    for def in catalog.values() {
        if !deckable(def) {
            continue;
        }
        if let Some(commander) = commanders
            .iter()
            .find(|commander| commander.cards.contains(&def.id))
        {
            cards.push(AssaultCard {
                id: def.id.clone(),
                rarity: def.rarity,
                doctrine: commander.doctrine.clone(),
                commander: true,
            });
        } else {
            let group = counter % (file.doctrines.len() + 1);
            counter += 1;
            cards.push(AssaultCard {
                id: def.id.clone(),
                rarity: def.rarity,
                doctrine: if group == 0 {
                    None
                } else {
                    Some(file.doctrines[group - 1].id.clone())
                },
                commander: false,
            });
        }
    }
    AssaultContent {
        cards,
        doctrines: file.doctrines,
        commanders,
    }
}
