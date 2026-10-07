//! Temporary (R637): a card that is discarded from its owner's hand at the end of their turn. The
//! keyword is a card keyword and not temporary mana (§2.3, `mana.rs`), which is a different thing that
//! only shares the word, so nothing here or in the card data is called "temporary" without "card".
//!
//! A card is Temporary when its running face prints it or an effect granted it (`card_keywords`, the
//! set a card in a hand is made of), so a Vanilla card has lost the printed one and keeps a given one,
//! as §10.4 keeps every granted keyword. Cleanup (`turn.rs`) discards them after every end-of-turn
//! step, so an end-of-turn effect, a trap and a delayed effect may still play or use them first; the
//! discard is a real one (§6.3), so "whenever you discard" sees each card, and `endOfTurnCleanupSettle`
//! answers its events like cleanup's others.
//!
//! Port of `packages/engine/src/temporary.ts`.

use crate::layers::card_keywords;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState};
use crate::wire::{KeywordKind, PlayerId, has_keyword};

/// R637: whether this card is discarded from a hand at the end of its owner's turn.
pub fn is_temporary_card(state: &GameState, card: &CardInstance) -> bool {
    has_keyword(&card_keywords(state, card), KeywordKind::Temporary)
}

/// R637: every Temporary card in `player`'s hand goes to their graveyard, in hand order. Deterministic
/// and drawing nothing from the rng, like a whole-hand discard (`move_::discard_hand`): no choice arises.
pub fn discard_temporary_cards(sink: &mut EngineSink<'_>, player: PlayerId) {
    // A snapshot: a discard splices the hand.
    let hand: Vec<CardInstance> = sink.state.players[player].hand.clone();
    for card in &hand {
        if is_temporary_card(sink.state, card) {
            crate::effects::move_::discard_from_hand(sink, card);
        }
    }
}
