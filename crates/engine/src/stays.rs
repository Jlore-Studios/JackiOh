//! A card's stay in a zone, read off the event stream (SPEC §10.3, R174, R212).
//!
//! §10.3 has every visible change emit an event, so the events after a moment say what happened since:
//!   * R174: has a card left the field since this point? A Reborn body is back under the same id by
//!     the time the state check returns (§4.5 step 4), so only the `destroyed` it went out with tells.
//!   * R212: did a card arrive where it is now after an event, or change hands after it? The loop
//!     hands an event to the triggers some time after it happened, so the board can hold a Reborn
//!     body, a card drawn since or a unit a Death stole since; the events still owed say which.
//!
//! No instance field records a stay (docs/polish), and a sequence a prompt splits resumes in a later
//! action whose events begin after the pause, so R174 is also kept in state: `exit_mark` when a
//! sequence begins, `left_field_after` against `GameState.field_exits` whatever action it resumes in.

use indexmap::{IndexMap, IndexSet};
use serde_json::Value;

use crate::state::{FieldExits, GameState, UncoveredNote};
use crate::wire::{GameEvent, LibraryOverflowOutcome, PlayerId, opponent_of};

pub use crate::state::EventStay;

/// R174: the field's departures so far, as a mark to ask `left_field_after` against later.
pub fn exit_mark(state: &GameState) -> u32 {
    state.field_exits.as_ref().map_or(0, |exits| exits.count)
}

/// R174, R212: the stay an event happened on, when it carries one: the play pipeline's `cardPlayed`,
/// `summoned` and `cardResolved` name the played card, and the loop can hand them to a response well
/// after they happened (a cast's `cardResolved` waits for the list that cast it, R70). A card that has left the field since, even one back through Reborn, is
/// not the card the event is about (R83). `None` for an event with no mark, judged from its dispatch.
pub fn event_mark(event: &GameEvent) -> Option<u32> {
    match event {
        GameEvent::CardPlayed { exits_from, .. }
        | GameEvent::Summoned { exits_from, .. }
        | GameEvent::CardResolved { exits_from, .. } => *exits_from,
        _ => None,
    }
}

/// The fields of an event that name a card: every one of these that holds a string.
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

/// Every card id an event names: the card it is about, the source and target of a hit, and so on.
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

/// R174: a card has just left the field — died, bounced, exiled, returned to a library, or ceased to
/// exist there (replaced by a Transform, fused away). Called from `zones::move_to_zone` and
/// `zones::cease_to_exist`, the two funnels every such move goes through, so a reader that names the
/// card by the id an event carried finds it gone.
pub fn note_field_exit(state: &mut GameState, instance_id: &str) {
    let exits = state.field_exits.get_or_insert_with(FieldExits::default);
    exits.count += 1;
    let count = exits.count;
    exits.last.insert(instance_id.to_string(), count);
}

/// R174: whether a card has left the field since `mark` — even if it is back on it now, bounced and
/// replayed or returned by Reborn, since what came back is a new arrival (R78, R83). A change of
/// control is not leaving (R171), and neither is a Vanilla (§6.3).
pub fn left_field_after(state: &GameState, mark: u32, instance_id: &str) -> bool {
    state
        .field_exits
        .as_ref()
        .and_then(|exits| exits.last.get(instance_id))
        .copied()
        .unwrap_or(0)
        > mark
}

/// R174: whether a card has left the field in the events since `from` — died, was bounced or exiled,
/// or was replaced by a Transform. §10.3 has every visible change emit an event, so this list is the
/// whole of what can take a card off the field.
pub fn left_field_since(events: &[GameEvent], from: usize, instance_id: &str) -> bool {
    for event in events.iter().skip(from) {
        match event {
            GameEvent::Destroyed { instance_id: id, .. }
            | GameEvent::Bounced { instance_id: id, .. }
            | GameEvent::Exiled { instance_id: id, .. } => {
                if id == instance_id {
                    return true;
                }
            }
            // A Replace puts a new card in the old one's place; a Vanilla (§6.3) names the same card on
            // both sides of the event, and a card whose text went away has not left anything.
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

/// §3.2, R153, R212: a card has just been taken off the field, and `resumed` is the card beneath it in
/// its Stack pile that is the pile's top now, if it was on top of one. A dormant card registers nothing
/// (R153), so the resumed card did not see what happened before it resumed, and no event reports a
/// resume: it is kept here against the card whose leaving caused it, for `uncovered_by` to read.
///
/// A note belongs to one removal. It lasts while that removal's report is still owed to the loop
/// (`note_reported`) and while the card that left has not moved again (`note_moved`), and goes once
/// both have happened: a later move of the same card is no removal from a pile's top, so a card that
/// resumed long before is not taken for one that resumed after it. Every removal is such a move.
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

/// R212: a card has moved zones again (`zones::remove_from_any_zone`), so the note of its last removal
/// from a pile's top is done once the loop has dispatched that removal's report; until then the
/// report still reads it, and the note goes with the report (`note_reported`).
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

/// R212: the loop has dispatched an event (`triggers::dispatch_event`). A removal it reports has been
/// answered: its note stays for the rest of that removal's reports (a death's `enteredGraveyard`
/// after its `destroyed`) until the card moves again, or goes now if it already has.
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
/// R13) — the removals a Stack note can hang on. Arrivals are not read: they name no removal.
fn removals_in(event: &GameEvent) -> Vec<String> {
    match event {
        GameEvent::Destroyed { instance_id, .. }
        | GameEvent::EnteredGraveyard { instance_id, .. }
        | GameEvent::Exiled { instance_id, .. }
        | GameEvent::Bounced { instance_id, .. }
        | GameEvent::ShuffledIn { instance_id, .. }
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

/// R212, §3.2: the card the removal an event reports uncovered in its Stack pile — which resumed then,
/// after the event and whatever came before it (`note_uncovered`).
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
    /// Every card the events moved between zones, or brought into existence, or replaced — and, read
    /// against a state, every dormant card a removal they report uncovered in its Stack pile (§3.2).
    pub moved: IndexSet<String>,
    /// The controller each card had before the first change of control the events show. A change of
    /// control always hands a card to the other player (a steal of your own card does nothing, R76),
    /// so the controller before it is the opponent of the one it went to.
    pub controller_before: IndexMap<String, PlayerId>,
}

/// The ids a zone-changing event moves. `transformed` moves both — the old card ceases to exist and
/// its replacement arrives (§6.3 Replace) — while a Fuse keeps the instance of a target on the field
/// (R77), so only the ingredients that ceased to exist are moved by it. A change of control, a
/// rotation and a board swap are not moves: the card stays on the field (R171, R174).
fn moved_by(event: &GameEvent) -> Vec<String> {
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
        | GameEvent::ShuffledIn { instance_id, .. } => vec![instance_id.clone()],
        // R316: as in `removals_in`, an existing card a full library refused has moved; a copy never
        // created has not.
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

/// R212: read the events that followed an event, oldest first. With `state`, a card that resumed as a
/// Stack pile's top because a card the events report leaving it (`uncovered_by`) counts as moved too:
/// it was dormant when the event happened, registering nothing (§3.2, R153), and it comes back into
/// play the way a Reborn body does.
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
