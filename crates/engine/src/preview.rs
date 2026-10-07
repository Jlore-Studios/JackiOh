//! The number a formula comes to now (SPEC §10.8, §10.9, R280). A card whose text computes a number
//! from the board — #31's Fib(cost+1), #70's sum over missing health and exile, #40 radiant's twice
//! the exile — would leave the player to do the arithmetic, and the client may not (CLAUDE.md rule 7).
//! So the engine does it: the card's script declares `preview`, a pure read, and `view_for` carries
//! its answer on the card's view as `preview`. This module is the one place that asks the hook.
//!
//! Where a card carries it is where the viewer may read the card, and nowhere else (§10.8):
//!   - the viewer's own hand, in any phase — a value is information, not the playability glow R195
//!     keeps to the viewer's own main phase, and the hand's faces are the viewer's to read (§9.1);
//!   - a unit on top of its pile, either seat's (the field is public);
//!   - a backrow card of either seat that is face-up to the viewer: a Field Spell, a fired Field Trap,
//!     and a face-down Trap or Field Trap for its controller alone (R33).
//!
//! Never on the opponent's hand, a library card, a card dormant under a Stack (R13) or a card in a
//! graveyard, exile or the resolving zone. `view_for` asks only in the three places above; the guards
//! below refuse the rest again, without calling the hook, so a caller that asks about the wrong card
//! learns nothing either.
//!
//! The hook is asked about the card as it runs: its own face (`radiant`), its own controller — the
//! other seat's for a public card of theirs, since the number is the card's and not the viewer's —
//! the zone, and `yourTurn` for that controller. It reads only what that controller may read (§9.1),
//! and everything it may read is public or the viewer's own wherever it is shown, so it reveals
//! nothing. An empty answer, or no hook, is no preview: the key is absent rather than `[]`.
//!
//! Port of `packages/engine/src/preview.ts`.

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
/// `backrow_is_public` above is this plus the controller's exception, and `view_for` marks the
/// controller's own view of such a card `unrevealed` from the same answer, so the mark a client draws
/// and the back the other player sees cannot disagree.
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
