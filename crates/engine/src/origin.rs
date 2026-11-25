//! R4592002: card deck-of-origin tracking (Infinite Assault, Scraper).
//!
//! "Started in your opponent's deck" means the card is not Created (R943) and, before its first
//! change of owner (steal, swap, keeper), its owner was the opponent. Stamped only in a game that
//! keeps the recursion log (`recursion_log::kept`, R4589000: one holding an Infinite Assault card),
//! so no other game's state or hash moves (D14).

use crate::query::is_created;
use crate::state::CardInstance;
use crate::wire::opponent_of;

/// Stamps `started_in = Some(card.owner)` on a card before its owner changes, provided the game
/// keeps the log (`kept`, read by the caller before it borrows the card), the card was not Created
/// and its origin is not yet noted.
pub fn note_origin(kept: bool, card: &mut CardInstance) {
    if kept && !is_created(card) && card.started_in.is_none() {
        card.started_in = Some(card.owner);
    }
}

/// Whether this card started the game in the opponent of its current controller's deck.
pub fn started_in_opponents_deck(card: &CardInstance) -> bool {
    !is_created(card) && card.started_in.unwrap_or(card.owner) == opponent_of(card.controller)
}
