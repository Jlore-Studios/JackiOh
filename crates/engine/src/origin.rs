//! R4592002: card deck-of-origin tracking (Infinite Assault, Scraper).
//!
//! "Started in your opponent's deck" means the card is not Created (R943) and, before its first
//! change of owner (steal, swap, keeper), its owner was the opponent. Gated on the held set
//! being open (`scraper_records_open()`).

use crate::play_history::scraper_records_open;
use crate::query::is_created;
use crate::state::CardInstance;
use crate::wire::{PlayerId, opponent_of};

/// Stamps `started_in = Some(card.owner)` on a card before its owner changes, provided the card
/// was not Created and its origin is not yet noted.
pub fn note_origin(card: &mut CardInstance) {
    if scraper_records_open() && !is_created(card) && card.started_in.is_none() {
        card.started_in = Some(card.owner);
    }
}

/// The player in whose deck this card began the game, `None` for a Created card.
pub fn started_in_deck_of(card: &CardInstance) -> Option<PlayerId> {
    if is_created(card) {
        None
    } else {
        Some(card.started_in.unwrap_or(card.owner))
    }
}

/// Whether this card started the game in the opponent of its current controller's deck.
pub fn started_in_opponents_deck(card: &CardInstance) -> bool {
    !is_created(card) && card.started_in.unwrap_or(card.owner) == opponent_of(card.controller)
}
