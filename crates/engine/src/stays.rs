//! A card's stay in a zone, read off the event stream (SPEC §10.3, R174, R212).
//!
//! §10.3 has every visible change emit an event, so the events after a moment say what happened since:
//!   * R174: has a card left the field since this point? A Reborn body is back under the same id by
//!     the time the state check returns (§4.5 step 4), so only the `destroyed` it went out with tells.
//!   * R212: did a card arrive where it is now after an event, or change hands after it?
//!
//! No instance field records a stay (docs/polish), and a sequence a prompt splits resumes in a later
//! action whose events begin after the pause, so R174 is also kept in state (`exit_mark`).

use indexmap::{IndexMap, IndexSet};
use serde_json::Value;

use crate::state::{FieldExits, GameState, UncoveredNote};
use crate::wire::{GameEvent, LibraryOverflowOutcome, PlayerId, opponent_of};

pub use crate::state::EventStay;

/// R174: the field's departures so far, as a mark to ask `left_field_after` against later.
pub fn exit_mark(state: &GameState) -> u32 {
    state.field_exits.as_ref().map_or(0, |exits| exits.count)
}

/// R174, R212: the stay a play-pipeline event happened on, which the loop can hand to a response well
/// after (a cast's `cardResolved` waits for the list that cast it, R70); a card that has left the field
/// since is not the card it is about (R83).
pub fn event_mark(event: &GameEvent) -> Option<u32> {
    match event {
        GameEvent::CardPlayed { exits_from, .. }
        | GameEvent::Summoned { exits_from, .. }
        | GameEvent::CardResolved { exits_from, .. } => *exits_from,
        _ => None,
    }
}

/// The fields of an event that name a card.
const CARD_FIELDS: &[&str] = &[
    "instanceId",
    "newInstanceId",
    "resultInstanceId",
    "sourceId",
    "targetId",
    "killerId",
    "attackerId",
    "byInstanceId",
];

/// Every card id an event names.
pub fn cards_named_by(event: &GameEvent) -> Vec<String> {
    let fields = serde_json::to_value(event).unwrap_or(Value::Null);
    let mut named: Vec<String> = CARD_FIELDS
        .iter()
        .filter_map(|field| fields.get(*field).and_then(Value::as_str).map(str::to_string))
        .collect();
    if let GameEvent::Fused { instance_ids, .. } = event {
        named.extend(instance_ids.iter().cloned());
    }
    named
}

/// The stays an event happened on, for a trigger queued on it now (`EventStay`).
pub fn event_stay_of(state: &GameState, event: &GameEvent) -> EventStay {
    EventStay {
        from: event_mark(event).unwrap_or_else(|| exit_mark(state)),
        ids: cards_named_by(event),
    }
}

/// R174: a card has just left the field. Called from `zones::move_to_zone` and `zones::cease_to_exist`,
/// the two funnels every such move goes through.
pub fn note_field_exit(state: &mut GameState, instance_id: &str) {
    let exits = state.field_exits.get_or_insert_with(FieldExits::default);
    exits.count += 1;
    let count = exits.count;
    exits.last.insert(instance_id.to_string(), count);
}

/// R174: whether a card has left the field since `mark`, even if it is back now: what came back is a
/// new arrival (R78, R83). A change of control is not leaving (R171), nor is a Vanilla (§6.3).
pub fn left_field_after(state: &GameState, mark: u32, instance_id: &str) -> bool {
    state
        .field_exits
        .as_ref()
        .and_then(|exits| exits.last.get(instance_id))
        .copied()
        .unwrap_or(0)
        > mark
}

/// R174: whether a card has left the field in the events since `from` (§10.3: every way off emits one).
pub fn left_field_since(events: &[GameEvent], from: usize, instance_id: &str) -> bool {
    for event in events.iter().skip(from) {
        match event {
            GameEvent::Destroyed { instance_id: id, .. }
            | GameEvent::Bounced { instance_id: id, .. }
            | GameEvent::Exiled { instance_id: id, .. }
            // R1850003: a card held beneath another has left the field.
            | GameEvent::Held { instance_id: id, .. } => {
                if id == instance_id {
                    return true;
                }
            }
            // A Vanilla (§6.3) names the same card on both sides: it has not left.
            GameEvent::Transformed {
                instance_id: id,
                new_instance_id,
                ..
            } if id == instance_id && new_instance_id != instance_id => {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// §3.2, R153, R212: a card has just been taken off the field, and `resumed` tops its Stack pile now. A
/// dormant card registers nothing (R153) and no event reports a resume, so it is noted against the card
/// that left, for `uncovered_by`, until that removal is reported (`note_reported`) and the card moves
/// again (`note_moved`), so a card that resumed long before is not taken for a later one.
pub fn note_uncovered(state: &mut GameState, removed_id: &str, resumed: Option<&str>) {
    note_moved(state, removed_id);
    let Some(resumed) = resumed else {
        return;
    };
    let exits = state.field_exits.get_or_insert_with(FieldExits::default);
    exits.uncovered.get_or_insert_with(IndexMap::new).insert(
        removed_id.to_string(),
        UncoveredNote {
            resumed: resumed.to_string(),
            reported: None,
            moved_on: None,
        },
    );
}

/// R212: a card has moved zones again: its last removal's note goes now if its report was dispatched,
/// else with the report (`note_reported`).
pub fn note_moved(state: &mut GameState, instance_id: &str) {
    let Some(uncovered) = state
        .field_exits
        .as_mut()
        .and_then(|exits| exits.uncovered.as_mut())
    else {
        return;
    };
    let Some(reported) = uncovered.get(instance_id).map(|note| note.reported == Some(true)) else {
        return;
    };
    if reported {
        uncovered.shift_remove(instance_id);
    } else if let Some(note) = uncovered.get_mut(instance_id) {
        note.moved_on = Some(true);
    }
}

/// R212: the loop has dispatched an event (`triggers::dispatch_event`). A removal it reports keeps its
/// note for its other reports (a death's `enteredGraveyard` after its `destroyed`) until the card moves.
pub fn note_reported(state: &mut GameState, event: &GameEvent) {
    let Some(uncovered) = state
        .field_exits
        .as_mut()
        .and_then(|exits| exits.uncovered.as_mut())
    else {
        return;
    };
    for id in removals_in(event) {
        let Some(moved_on) = uncovered.get(&id).map(|note| note.moved_on == Some(true)) else {
            continue;
        };
        if moved_on {
            uncovered.shift_remove(&id);
        } else if let Some(note) = uncovered.get_mut(&id) {
            note.reported = Some(true);
        }
    }
}

/// The cards an event reports leaving the field, or changing hands (a steal takes only a pile's top,
/// R13) — the removals a Stack note can hang on.
fn removals_in(event: &GameEvent) -> Vec<String> {
    match event {
        GameEvent::Destroyed { instance_id, .. }
        | GameEvent::EnteredGraveyard { instance_id, .. }
        | GameEvent::Exiled { instance_id, .. }
        | GameEvent::Bounced { instance_id, .. }
        | GameEvent::ShuffledIn { instance_id, .. }
        | GameEvent::Held { instance_id, .. }
        | GameEvent::ControlChanged { instance_id, .. } => vec![instance_id.clone()],
        // R316: an existing card a full library refused left where it was, like a `shuffledIn`; a copy
        // that was never created left nothing.
        GameEvent::LibraryOverflow {
            instance_id, outcome, ..
        } => {
            if *outcome == LibraryOverflowOutcome::NotCreated {
                vec![]
            } else {
                vec![instance_id.clone()]
            }
        }
        GameEvent::Fused {
            instance_ids,
            result_instance_id,
            ..
        } => instance_ids
            .iter()
            .filter(|id| *id != result_instance_id)
            .cloned()
            .collect(),
        _ => vec![],
    }
}

/// R212, §3.2: the cards the removals an event reports uncovered in their Stack piles (`note_uncovered`).
pub fn uncovered_by(state: &GameState, event: &GameEvent) -> Vec<String> {
    let Some(uncovered) = state
        .field_exits
        .as_ref()
        .and_then(|exits| exits.uncovered.as_ref())
    else {
        return vec![];
    };
    removals_in(event)
        .into_iter()
        .filter_map(|id| uncovered.get(&id).map(|note| note.resumed.clone()))
        .collect()
}

/// What a run of events did to the cards it names (R212).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaterMoves {
    /// Every card the events moved, made or replaced, and, read against a state, every dormant card a
    /// removal they report uncovered in its Stack pile (§3.2).
    pub moved: IndexSet<String>,
    /// The controller each card had before the first change of control the events show: the opponent
    /// of the one it went to, since a steal of your own card does nothing (R76).
    pub controller_before: IndexMap<String, PlayerId>,
}

/// The ids a zone-changing event moves: both of a `transformed` (§6.3 Replace), and a Fuse's ingredients
/// but not its target, whose instance stays (R77). A change of control, a rotation and a board swap are
/// not moves: the card stays on the field (R171, R174).
pub(crate) fn moved_by(event: &GameEvent) -> Vec<String> {
    match event {
        GameEvent::CardPlayed { instance_id, .. }
        | GameEvent::Summoned { instance_id, .. }
        | GameEvent::Destroyed { instance_id, .. }
        | GameEvent::EnteredGraveyard { instance_id, .. }
        | GameEvent::Exiled { instance_id, .. }
        | GameEvent::Bounced { instance_id, .. }
        | GameEvent::Burned { instance_id, .. }
        | GameEvent::Discarded { instance_id, .. }
        | GameEvent::Drawn { instance_id, .. }
        | GameEvent::AddedToHand { instance_id, .. }
        | GameEvent::ShuffledIn { instance_id, .. }
        | GameEvent::Held { instance_id, .. } => vec![instance_id.clone()],
        // R316: as in `removals_in`.
        GameEvent::LibraryOverflow {
            instance_id, outcome, ..
        } => {
            if *outcome == LibraryOverflowOutcome::NotCreated {
                vec![]
            } else {
                vec![instance_id.clone()]
            }
        }
        GameEvent::Transformed {
            instance_id,
            new_instance_id,
            ..
        } => vec![instance_id.clone(), new_instance_id.clone()],
        GameEvent::Fused {
            instance_ids,
            result_instance_id,
            ..
        } => instance_ids
            .iter()
            .filter(|id| *id != result_instance_id)
            .cloned()
            .collect(),
        _ => vec![],
    }
}

/// B5 E5: whether `event` sends the card `id` toward a graveyard (its landing is reported after it).
fn sends_toward_graveyard(event: &GameEvent, id: &str) -> bool {
    matches!(
        event,
        GameEvent::Discarded { instance_id, .. }
        | GameEvent::Burned { instance_id, .. }
        | GameEvent::Destroyed { instance_id, .. }
        if instance_id == id
    )
}

/// B5 E5: whether `event` reports where such a move landed `id`: its graveyard, or the exile or
/// library bottom a replacement sent it to.
fn is_landing_report(event: &GameEvent, id: &str) -> bool {
    matches!(
        event,
        GameEvent::EnteredGraveyard { instance_id, .. }
        | GameEvent::Exiled { instance_id, .. }
        | GameEvent::ShuffledIn { instance_id, .. }
        if instance_id == id
    )
}

/// R212, R862: whether the card `id`, which `event` itself sent toward a graveyard, still stands where
/// that move landed it, read off `later`. The landing's report (B5 E5) is that same move, so the card
/// answers `event` from there (Meditative #22.1's discard clause answers its own `discarded`) unless it
/// moved again. `false` when `event` sent it nowhere toward a graveyard: `moves_in`'s answer stands.
pub fn stands_where_it_landed(
    event: &GameEvent,
    later: &[GameEvent],
    id: &str,
    state: Option<&GameState>,
) -> bool {
    if !sends_toward_graveyard(event, id) {
        return false;
    }
    let first_move = later
        .iter()
        .position(|after| moved_by(after).iter().any(|moved| moved == id));
    let Some(first_move) = first_move else {
        return true;
    };
    if !is_landing_report(&later[first_move], id) {
        return false;
    }
    let since_landing = later
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != first_move)
        .map(|(_, after)| after);
    !moves_in(since_landing, state).moved.contains(id)
}

/// R212: read the events that followed an event, oldest first. With `state`, a card that resumed as a
/// Stack pile's top (`uncovered_by`) counts as moved too: it was dormant then (§3.2, R153).
pub fn moves_in<'a>(
    events: impl IntoIterator<Item = &'a GameEvent>,
    state: Option<&GameState>,
) -> LaterMoves {
    let mut moved: IndexSet<String> = IndexSet::new();
    let mut controller_before: IndexMap<String, PlayerId> = IndexMap::new();
    for event in events {
        for id in moved_by(event) {
            moved.insert(id);
        }
        if let Some(state) = state {
            for id in uncovered_by(state, event) {
                moved.insert(id);
            }
        }
        if let GameEvent::ControlChanged {
            instance_id,
            controller,
            ..
        } = event
            && !controller_before.contains_key(instance_id)
        {
            controller_before.insert(instance_id.clone(), opponent_of(*controller));
        }
    }
    LaterMoves {
        moved,
        controller_before,
    }
}
