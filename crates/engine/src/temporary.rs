//! Temporary (R637): a card that is discarded from its owner's hand at the end of their turn. The
//! keyword is a card keyword and not temporary mana (§2.3, `mana.rs`), which is a different thing that
//! only shares the word, so nothing here or in the card data is called "temporary" without "card".
//!
//! A card is Temporary when its running face prints it or an effect granted it (`card_keywords`, the
//! set a card in a hand is made of), so a Vanilla card has lost the printed one and keeps a given one,
//! as §10.4 keeps every granted keyword. Cleanup (`turn.rs`) counts them down after every end-of-turn
//! step, so an end-of-turn effect, a trap and a delayed effect may still play or use them first; the
//! discard is a real one (§6.3), so "whenever you discard" sees each card, and `endOfTurnCleanupSettle`
//! answers its events like cleanup's others.
//!
//! R2179001: Temporary X. The number is the count of its owner's ends of turn the card outlasts in
//! their hand: each cleanup takes 1 off, and the card is discarded once it reaches 0. A plain Temporary
//! is Temporary 1, discarded at the first, as before the number. What is left after a count-down rides
//! the instance (`CardInstance.temporary`) and stands in for the keyword's number from then on, so the
//! view shows the count as it stands (`layers::card_keywords`).
//!
//! Port of `packages/engine/src/temporary.ts`.

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
/// that reaches 0 goes to their graveyard. Deterministic and drawing nothing from the rng, like a
/// whole-hand discard (`move_::discard_hand`): no choice arises.
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
