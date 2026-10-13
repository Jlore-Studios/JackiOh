//! What a player may know of their own library (SPEC §10.8, R310–R312): which cards are left in it,
//! never in what order, and only the cards they were shown going in.
//!
//! A player built their deck, drew and returned their own opening hand, and watched every card that
//! went in openly after that: a CN-Virus's copies, an Unstable Clone Machine's, the virus the
//! opponent's CN-Viral Injection shuffled in (a play both players saw, whose text names the card). So
//! the list is theirs to read, as a list without order: §9.1 hides the order, not the contents.
//!
//! Three rules, and this module owns them:
//!   - R310: the list is a multiset. One entry per definition and face, with a count, sorted by the
//!     definition's printed cost (R65's `query_cost`), then its name, then its id, base face first:
//!     an order the cards alone decide, so two libraries holding the same cards in any order give the
//!     same list. It carries no instance id, no position, no live cost and no rolled power.
//!   - R311: each library card carries `knownAs`, what its owner was shown of it as it went in. The
//!     list reads that record and never the card itself, so a change made inside a library, where
//!     nobody sees it (#28 or #42 making a card Radiant, #95's discount, #98's roll), shows nothing:
//!     the card is listed with the face it went in with. Only the openings named below write it.
//!   - R312: a card with no record was never shown to its owner, and is counted as unknown. That is
//!     every card of a library Pocket Chaos swapped (#87: its new owner never saw it; `hide_from_owner`
//!     clears the record the old owner had) and every card Transmogulate put in a library (#83: a new
//!     instance, and the `transformed` event names it to nobody, R177). A card stays unknown until it
//!     leaves the library, even one a prompt has since revealed (#51's options): the list may show
//!     less than its owner could piece together, never more.
//!
//! The record is written where a card goes in openly: the starting deck (`state::create_game`), a
//! mulligan's returns (`setup::finish_mulligan`) and a shuffle-in (`draw::shuffle_into_library`, which
//! every Core shuffle goes through). A path that forgets to write it shows a card back, never a card.
//!
//! R433 (rewrites R310's dealt decks): a deck its player was DEALT rather than built — All Random's
//! (R258), practice's fresh random deck — was never shown to them, so `create_game` writes no record
//! for the seats it is told were dealt one (`CreateGameOptions.dealt`). Every card of that library
//! counts as unknown until it leaves; what goes in openly afterwards (a mulligan's returns, a
//! shuffle-in) is recorded as for any deck, so the list shows only what its owner has been shown.
//!
//! Port of `packages/engine/src/ownLibrary.ts`.

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

/// Plain code-unit order: the same in every runtime, unlike `localeCompare`. (TS `<` compares UTF-16
/// code units, so the comparison walks them, not the UTF-8 bytes: a name may carry non-ASCII.)
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
    // `Number(a.radiant) - Number(b.radiant)`: base (false) before Radiant (true).
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
