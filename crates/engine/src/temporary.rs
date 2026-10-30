//! Temporary (R637): a card discarded from its owner's hand at the end of their turn. It is a card
//! keyword, not temporary mana (§2.3, `mana.rs`), so nothing here is called "temporary" without "card".
//!
//! A card is Temporary when its running face prints it or an effect granted it (`card_keywords`), so a
//! Vanilla card keeps only a granted one (§10.4). Cleanup (`turn.rs`) counts them down after every
//! end-of-turn step, so an end-of-turn effect, a trap or a delayed effect may still use them first; the
//! discard is a real one (§6.3): "whenever you discard" sees each (`end_of_turn_cleanup_settle`).
//!
//! R2179001: Temporary X counts the owner's ends of turn the card outlasts in hand: each cleanup takes
//! 1 off and the card is discarded at 0; a plain Temporary is Temporary 1. The count left rides the
//! instance (`CardInstance.temporary`) and stands in for the keyword's number, so the view shows it.

use crate::layers::card_keywords;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance_mut};
use crate::wire::{Keyword, KeywordKind, PlayerId, has_keyword};

/// R637: whether this card is discarded from a hand at the end of its owner's turn.
pub fn is_temporary_card(state: &GameState, card: &CardInstance) -> bool {
    has_keyword(&card_keywords(state, card), KeywordKind::Temporary)
}

/// R2179001: the count a keyword list gives a Temporary card — the largest number of its Temporary
/// entries, a plain one counting 1 — or `None` when it lists none.
pub fn temporary_number(keywords: &[Keyword]) -> Option<i32> {
    keywords
        .iter()
        .filter_map(|keyword| match keyword {
            Keyword::Temporary { n } => Some(n.unwrap_or(1)),
            _ => None,
        })
        .max()
}

/// R2179001: a card's keywords with its Temporary entries made one: the count left once a count-down
/// has stored one (`CardInstance.temporary`), else the entry with the largest number, so a card that
/// has not been counted down shows its number and a plain Temporary stays plain.
pub fn with_temporary_count(keywords: &[Keyword], instance: &CardInstance) -> Vec<Keyword> {
    let Some(most) = temporary_number(keywords) else {
        return keywords.to_vec();
    };
    let merged = match instance.temporary {
        Some(left) => Keyword::Temporary { n: Some(left) },
        None => keywords
            .iter()
            .find(|keyword| matches!(keyword, Keyword::Temporary { n } if n.unwrap_or(1) == most))
            .cloned()
            .unwrap_or(Keyword::TEMPORARY),
    };
    let mut out: Vec<Keyword> = Vec::with_capacity(keywords.len());
    let mut placed = false;
    for keyword in keywords {
        if keyword.kind() != KeywordKind::Temporary {
            out.push(keyword.clone());
        } else if !placed {
            out.push(merged.clone());
            placed = true;
        }
    }
    out
}

/// R637, R2179001: every Temporary card in `player`'s hand counts down by 1, in hand order, and each
/// that reaches 0 goes to their graveyard. No choice arises, so it draws nothing from the rng.
pub fn discard_temporary_cards(sink: &mut EngineSink<'_>, player: PlayerId) {
    // A snapshot: a discard splices the hand.
    let hand: Vec<CardInstance> = sink.state.players[player].hand.clone();
    for card in &hand {
        let Some(count) = temporary_number(&card_keywords(sink.state, card)) else {
            continue;
        };
        let left = count - 1;
        if left <= 0 {
            crate::effects::move_::discard_from_hand(sink, card);
        } else if let Some(live) = find_instance_mut(sink.state, &card.id) {
            live.temporary = Some(left);
        }
    }
}
