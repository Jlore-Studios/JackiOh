//! What a player may know of their own library (SPEC §10.8, R310–R312): which cards are left in it,
//! never in what order (§9.1 hides the order, not the contents), and only the cards they were shown.
//!
//!   - R310: the list is a multiset. One entry per definition and face, with a count, sorted by the
//!     definition's printed cost (R65's `query_cost`), then its name, then its id, base face first.
//!     It carries no instance id, no position, no live cost and no rolled power.
//!   - R311: each library card carries `knownAs`, what its owner was shown of it as it went in. The list
//!     reads that record and never the card itself, so a change made inside a library, where nobody sees
//!     it (#28, #42, #95, #98), shows nothing.
//!   - R312: a card with no record was never shown to its owner, and is counted as unknown until it leaves
//!     (a library Pocket Chaos swapped, #87; a card Transmogulate put in one, #83, R177; even one a prompt
//!     has since revealed, #51). It may show less than its owner knows, never more.
//!
//! The record is written where a card goes in openly (`state::create_game`, `setup::finish_mulligan`,
//! `draw::shuffle_into_library`); a path that forgets to write it shows a card back, never a card.
//! R433: a deck its player was DEALT (All Random's, R258; practice's) gets no record from `create_game`
//! (`CreateGameOptions.dealt`); what goes in openly afterwards is recorded as for any deck.

use std::cmp::Ordering;

use indexmap::IndexMap;

use crate::catalog::{def_of, query_cost};
use crate::state::{CardInstance, GameState, KnownAs};
use crate::wire::{LibraryEntryView, LibraryView, PlayerId};

/// R311: the card went into its owner's library openly; record what they were shown of it.
pub fn show_to_owner(card: &mut CardInstance) {
    card.known_as = Some(KnownAs {
        def_id: card.def_id.clone(),
        radiant: card.radiant,
    });
}

/// R312: the card's owner was never shown it (a library that changed hands, #87).
pub fn hide_from_owner(card: &mut CardInstance) {
    card.known_as = None;
}

/// Plain UTF-16 code-unit order, the same in every runtime: not `localeCompare`, and not the UTF-8
/// bytes, since a name may carry non-ASCII.
fn compare_text(a: &str, b: &str) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    a.encode_utf16().cmp(b.encode_utf16())
}

/// R310: the order of the list. Printed cost (R65), name, id, then base before Radiant — read off the
/// definition and the face alone, so it says nothing about where a card lies.
fn compare_entries(state: &GameState, a: &LibraryEntryView, b: &LibraryEntryView) -> Ordering {
    if a.def_id != b.def_id {
        let left = def_of(Some(state), &a.def_id);
        let right = def_of(Some(state), &b.def_id);
        let by_cost = query_cost(left) - query_cost(right);
        if by_cost != 0 {
            return by_cost.cmp(&0);
        }
        let by_name = compare_text(&left.name, &right.name);
        if by_name != Ordering::Equal {
            return by_name;
        }
        return compare_text(&a.def_id, &b.def_id);
    }
    a.radiant.cmp(&b.radiant).then(a.created.cmp(&b.created))
}

/// R310–R312: `player`'s own library as they may know it. Pure: reads the state, shares nothing.
pub fn own_library_view(state: &GameState, player: PlayerId) -> LibraryView {
    let mut counts: IndexMap<String, LibraryEntryView> = IndexMap::new();
    let mut unknown: i32 = 0;
    for card in &state.players[player].library {
        let Some(known) = &card.known_as else {
            unknown += 1;
            continue;
        };
        // MD-B6, R943: a Created card the owner knows is marked, so it groups apart.
        let created = if card.created == Some(true) {
            Some(true)
        } else {
            None
        };
        let key = format!(
            "{}:{}:{}",
            if known.radiant { "R" } else { "B" },
            if created == Some(true) { "C" } else { "-" },
            known.def_id
        );
        match counts.get_mut(&key) {
            Some(entry) => entry.count += 1,
            None => {
                counts.insert(
                    key,
                    LibraryEntryView {
                        def_id: known.def_id.clone(),
                        radiant: known.radiant,
                        count: 1,
                        created,
                    },
                );
            }
        }
    }
    let mut cards: Vec<LibraryEntryView> = counts.into_values().collect();
    cards.sort_by(|a, b| compare_entries(state, a, b));
    LibraryView { cards, unknown }
}
