//! Brittle X's count on a card instance (docs/classic-sets.md B3.3, R385, R441, R638): the readers and
//! the writes that need no sink. The count lives on the instance (`CardInstance.brittle`) and is kept in
//! every zone the card passes through, hand to field included, like `radiant` and `costMod` (R78), but
//! it only ticks on the field (R638); a copy never inherits it (R57). The start-of-turn tick and the
//! crumbling are `brittle.rs`'s.
//!
//! This module imports nothing heavier than the catalog, so the modules every field arrival and every
//! stat read pass through — `zones.rs` (`start_brittle_on_field`) and `layers.rs` (`active_brittle_count`)
//! — can read it without pulling the destroy and state-check machinery the tick needs.
//!
//! Port of `packages/engine/src/brittleCount.ts`. The writers take the card they change as
//! `&mut CardInstance` and the state as `&GameState` (TS read only `state.turn` and the catalog off it).

use crate::faces::{card_type_of, running_face};
use crate::state::{BrittleCounter, CardInstance, GameState};
use crate::tuning::{numbered_sum, tuned_count};
use crate::wire::{CardType, KeywordKind, Row, Zone};

/// B3.3 rule 1: the Brittle a card prints on its running face, as its tuning leaves it (a Degrade or
/// Upgrade of the printed number before the count has started, B3.4), or `None` when it prints none.
/// A Vanilla card prints nothing (§6.3).
pub fn printed_brittle_of(state: &GameState, card: &CardInstance) -> Option<i32> {
    if card.vanilla {
        return None;
    }
    let printed = numbered_sum(&running_face(state, card).keywords, KeywordKind::Brittle)?;
    // The same step `tuning::tuned_keywords` prints the keyword with, so the layers and the count agree.
    Some(tuned_count(card, KeywordKind::Brittle.as_str(), printed))
}

/// B3.3 rule 5: the count that is in force on the card now, or `None` for none. A count its printed
/// Brittle started is the card's text, which a Vanilla switches off while it lasts; a count an effect
/// gave stays, as §10.4 keeps every granted keyword.
pub fn active_brittle_count(card: &CardInstance) -> Option<i32> {
    let brittle = card.brittle?;
    if brittle.printed == Some(true) && card.vanilla {
        return None;
    }
    Some(brittle.count)
}

/// B3.3 rule 1, R638: "a printed Brittle starts when the card enters the field" — called where every
/// field arrival passes (`zones::place_on_field`, `zones::replace_in_zone`). A card that already has a
/// count keeps it (a count is kept in every zone, so a card that left the field and came back ticks on),
/// and one that prints no Brittle starts nothing. `from_off_field` is a card that arrives from a hand, a
/// deck, a graveyard or the resolving zone rather than from another field zone: a count it held
/// there never ticked, so its turn cycle starts now, and its first tick waits for a whole round on the
/// field (`BRITTLE_FIRST_TICK_TURNS`) however long it was held.
///
/// R687: a backrow Trap or Field Trap that enters face-down starts no count — there is no Brittle
/// while it is unrevealed (Classic+ #74). The count starts when the card reveals: its own subsystem
/// starts it with its first activation, and the `reveal` effect starts one for any card it shows.
pub fn start_brittle_on_field(state: &GameState, card: &mut CardInstance, from_off_field: bool) {
    if let Some(brittle) = card.brittle {
        if from_off_field {
            card.brittle = Some(BrittleCounter {
                since: state.turn,
                ..brittle
            });
        }
        return;
    }
    if let Zone::Field { row: Row::Backrow, .. } = card.zone {
        let kind = card_type_of(state, card);
        if (kind == CardType::Trap || kind == CardType::FieldTrap)
            && card.face_up != Some(true)
            && card.revealed != Some(true)
        {
            return;
        }
    }
    let Some(printed) = printed_brittle_of(state, card) else {
        return;
    };
    if printed <= 0 {
        return;
    }
    card.brittle = Some(BrittleCounter {
        count: printed,
        since: state.turn,
        printed: Some(true),
    });
}

/// B3.3 rule 4: "Give Brittle N" sets the count to N and starts it now, whatever the card had; the
/// count is a given one from then on, so a Vanilla keeps it.
pub fn give_brittle_count(state: &GameState, card: &mut CardInstance, count: i32) {
    card.brittle = Some(BrittleCounter {
        count: count.max(0),
        since: state.turn,
        printed: None,
    });
}

/// B3.3 rule 4: "gain +N Brittle" adds N to the count in force. A card with no count yet — a given one
/// or one its printed Brittle started — starts one now at N more than it prints (R441), which a Vanilla
/// then leaves alone as a given one.
pub fn gain_brittle_count(state: &GameState, card: &mut CardInstance, amount: i32) {
    let add = amount;
    if let (Some(active), Some(brittle)) = (active_brittle_count(card), card.brittle) {
        card.brittle = Some(BrittleCounter {
            count: (active + add).max(0),
            ..brittle
        });
        return;
    }
    let printed = printed_brittle_of(state, card).unwrap_or(0);
    card.brittle = Some(BrittleCounter {
        count: (printed + add).max(0),
        since: state.turn,
        printed: None,
    });
}

/// R441: a count that has crumbled its card is spent. R78's reset (`zones::reset_instance`) keeps a
/// Brittle count in every zone but this one, so a crumbled card that comes back — from its graveyard,
/// or a Reborn body — is not Brittle 0 for ever, crumbling again at every tick.
pub fn drop_spent_brittle(card: &mut CardInstance) {
    if card.brittle.is_some_and(|brittle| brittle.count <= 0) {
        card.brittle = None;
    }
}
