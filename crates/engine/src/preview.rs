//! The number a formula comes to now (SPEC §10.8, §10.9, R280). A card whose text computes a number
//! from the board would leave the player the arithmetic, and the client may not (CLAUDE.md rule 7).
//! So the card's script declares `preview`, a pure read, and `view_for` carries its answer.
//!
//! It is asked only where the viewer may read the card (§10.8): their own hand in any phase (a value
//! is information, not the playability glow R195 keeps to the main phase; §9.1), a unit on top of its
//! pile, either seat's, and a backrow card face-up to the viewer or face-down for its controller (R33).
//! Never the opponent's hand, a library card, a card dormant under a Stack (R13) or one in a
//! graveyard, exile or the resolving zone: the guards below refuse those without calling the hook.
//!
//! The hook runs as the card does (its own face and controller, the zone, `yourTurn`) and reads only
//! what that controller may read, so it reveals nothing. An empty answer is no preview: no key, not `[]`.

use crate::script::{ConditionContext, ConditionZone};
use crate::state::{CardInstance, GameState};
use crate::wire::{CardType, PlayerId, PreviewValue, Row, Zone};

/// §10.8: "traps show as unknown, Field Spells are public". A backrow Trap or Field Trap is readable
/// by its current controller only until it flips face-up, which is what R33 keys on `controller`.
/// The one rule both `view_for` (what a view shows of a backrow card) and this module (what a preview
/// may be asked about) read, so the two cannot drift apart.
pub fn backrow_is_public(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    !is_face_down(state, card) || card.controller == viewer
}

/// R33, R371, R686: a backrow Trap or Field Trap that has not flipped face-up, so only its
/// controller may read it — unless it is revealed, which both players read while it stays armed.
/// `view_for` marks the controller's own view of such a card `unrevealed` from the same answer, so the
/// mark a client draws and the back the other player sees cannot disagree.
pub fn is_face_down(state: &GameState, card: &CardInstance) -> bool {
    let card_type = crate::faces::card_type_of(state, card);
    if card_type != CardType::Trap && card_type != CardType::FieldTrap {
        return false;
    }
    card.face_up != Some(true) && card.revealed != Some(true)
}

/// R280, §10.8: whether `viewer` may read `card` where the question places it.
fn may_preview(state: &GameState, card: &CardInstance, viewer: PlayerId, zone: ConditionZone) -> bool {
    if zone == ConditionZone::Hand {
        return matches!(card.zone, Zone::Hand { player } if player == viewer);
    }
    match card.zone {
        Zone::Field { row: Row::Units, .. } => !crate::zones::is_buried(state, card),
        Zone::Field { .. } => backrow_is_public(state, card, viewer),
        _ => false,
    }
}

/// R280: the labelled numbers `viewer` sees on `card`, or `None` for none. Pure: it copies what the
/// hook returns, so the view holds no reference into a script, and it calls the hook at most once.
pub fn preview_of(
    state: &GameState,
    card: &CardInstance,
    viewer: PlayerId,
    zone: ConditionZone,
) -> Option<Vec<PreviewValue>> {
    if !may_preview(state, card, viewer, zone) {
        return None;
    }
    // B5 E14, R547: a copier (Classic #57 Echo) previews the formula of the text it has, on that face.
    let script = crate::subsystems::copied_text::running_script_of(state, card);
    let hook = script.preview.as_ref()?;
    let face = crate::subsystems::copied_text::text_face_of(state, card);
    let values = hook(ConditionContext {
        state,
        self_: &face,
        controller: card.controller,
        radiant: face.radiant,
        zone,
        your_turn: state.active == card.controller,
    });
    // R372: a value the text names by a word (#93's grade letter) carries it as `display`; a value that
    // counts a set of cards (Classic+ #44, #45's Radiant "highlight targets") carries their ids.
    let copied: Vec<PreviewValue> = values
        .into_iter()
        .filter(|entry| !entry.label.is_empty())
        .map(|entry| PreviewValue {
            label: entry.label,
            value: entry.value,
            display: entry.display.filter(|display| !display.is_empty()),
            ids: entry.ids,
        })
        .collect();
    if copied.is_empty() { None } else { Some(copied) }
}
