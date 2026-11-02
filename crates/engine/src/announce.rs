//! The announce window's record (docs/classic-sets.md B5 E1, R448): the plays and casts that have been
//! paid for and announced (`cardAnnounced`) but not yet moved by §10.5 step 4.
//!
//! §10.5 gains a step between 3 and 4 (`play_steps::announce_step`): the engine announces the play and
//! runs a window in which Counters answer it, traps first, then the other triggers it woke (§10.3). The
//! card waits in its player's resolving zone (Hearthstone's stack), so nothing that reaches a hand
//! reaches it (R448). This leaf reads and writes `state.announcing` alone, for the Counter verb
//! (`effects::move_::counter_play`), the trap dispatch and trigger queue (a response to a canceled
//! announce finds no card: a trap stays set, a trigger fizzles), `view_for` and the AI's redaction.

use crate::state::{AnnounceRecord, CardInstance, GameState};
use crate::wire::{PlayerId, ZoneName};

/// Every open announce, outermost first (a cast a responder makes announces inside the window).
pub fn open_announces(state: &GameState) -> &[AnnounceRecord] {
    state.announcing.as_deref().unwrap_or(&[])
}

/// The open announce of this card, or `None` when its window is closed or never opened.
pub fn announce_of<'a>(state: &'a GameState, instance_id: &str) -> Option<&'a AnnounceRecord> {
    open_announces(state)
        .iter()
        .find(|record| record.instance_id == instance_id)
}

/// R448: whether this card's play is still announced and uncanceled — what a Counter can still
/// answer. False once a Counter has canceled it (the rest find no card and stay set), and once
/// step 4 has moved it (the window is over).
pub fn is_announce_live(state: &GameState, instance_id: &str) -> bool {
    announce_of(state, instance_id).is_some_and(|record| record.countered != Some(true))
}

/// The innermost announce still live: what a Counter that names no card answers.
pub fn innermost_live_announce(state: &GameState) -> Option<&AnnounceRecord> {
    open_announces(state)
        .iter()
        .rev()
        .find(|record| record.countered != Some(true))
}

/// Open a card's announce (§10.5 between steps 3 and 4).
pub fn begin_announce(state: &mut GameState, record: AnnounceRecord) {
    let mut open: Vec<AnnounceRecord> = open_announces(state)
        .iter()
        .filter(|open| open.instance_id != record.instance_id)
        .cloned()
        .collect();
    open.push(record);
    state.announcing = Some(open);
}

/// Close a card's announce once its window is over, and return its record. The field goes when the
/// last one closes, so a state with no window open hashes as it did before the record existed.
pub fn end_announce(state: &mut GameState, instance_id: &str) -> Option<AnnounceRecord> {
    let record = announce_of(state, instance_id).cloned();
    let rest: Vec<AnnounceRecord> = open_announces(state)
        .iter()
        .filter(|open| open.instance_id != instance_id)
        .cloned()
        .collect();
    if rest.is_empty() {
        state.announcing = None;
    } else {
        state.announcing = Some(rest);
    }
    record
}

/// R448: a Counter canceled this announce. Returns false when there was nothing live to cancel.
pub fn mark_countered(state: &mut GameState, instance_id: &str) -> bool {
    let Some(record) = state
        .announcing
        .as_mut()
        .and_then(|open| open.iter_mut().find(|record| record.instance_id == instance_id))
    else {
        return false;
    };
    if record.countered == Some(true) {
        return false;
    }
    record.countered = Some(true);
    true
}

/// R448, R97, R227: a card waiting in the resolving zone to be set face-down is read by its player
/// alone, as the face-down card it is about to be; a card being played face-up is public there (R98).
pub fn announced_face_down_to(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    if card.zone.z() != ZoneName::Resolving {
        return false;
    }
    announce_of(state, &card.id)
        .is_some_and(|record| record.face_down == Some(true) && record.player != viewer)
}
