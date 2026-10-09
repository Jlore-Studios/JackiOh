//! R437: a card that something still to come is aimed at carries a mark in both players' views —
//! #50 K-Pop Fanatic's pending steal marks its target, drawn by the client as a corruption sparkle in
//! the mark's colour ("purple"), one effect reusable in other colours for other marks (SPEC §10.8).
//!
//! A mark is made by the effect that waits: `effects::delay`'s `delay({ …, watch, mark })` marks the
//! card it watches (R174) as it schedules itself. A delayed destroy of a scope (Classic #20's Radiant
//! face, R750) marks every card the scope names instead, kept in step with the board each time events
//! are collected (`sync_marks`), so a card that arrives while it waits is marked and one that leaves is
//! not. A mark lasts exactly as long as the delayed effect that made it does, so the record here is
//! tied to the entry's id and holds nothing the entry does not: when the effect resolves at its R62
//! point, fizzles, or is dropped because its card left the field
//! (`zones::forget_watchers`, R76, R174), the entry is gone and so is the mark. `sweep_marks` notices it
//! at the next point the resolution loop collects events and says so with a `marked` event
//! (`added: false`), after whatever took the entry away; the one that made the mark said
//! `added: true`. `view_for` reads `marks_on`, which reads the live entries, so the view never shows a
//! mark whose effect has gone, even before the sweep has run.
//!
//! Hidden information: the mark was made by a public play aimed at a card its maker chose, so it
//! tells neither player anything the play did not (R437). The event names the card and follows R97
//! (`view_for::redact_event`): a face-down target is the sentinel to the player who may not read it,
//! whose view carries the mark on the zone's back instead (R33).
//!
//! Port of `packages/engine/src/marks.ts`. TS's `MarkSink = { state, events }` is the engine's
//! `EngineSink` (SURFACE §6.5).

use crate::script::EngineSink;
use crate::state::{DelayedEffect, GameState, MarkRecord};
use crate::wire::{CardMark, GameEvent};

fn marked_event(record: &MarkRecord, added: bool) -> GameEvent {
    GameEvent::Marked {
        instance_id: record.instance_id.clone(),
        mark: record.mark.clone(),
        color: record.color.clone(),
        added,
    }
}

/// R437: mark the card a delayed effect is aimed at (`DelayedEffect.watch`) for as long as the effect
/// waits. A delayed effect aimed at no card marks nothing.
pub fn mark_delayed(sink: &mut EngineSink<'_>, entry: &DelayedEffect, mark: &CardMark) {
    let Some(watch) = entry.watch.as_ref() else {
        return;
    };
    let record = MarkRecord {
        instance_id: watch.clone(),
        mark: mark.mark.clone(),
        color: mark.color.clone(),
        delayed_id: entry.id.clone(),
    };
    sink.events.push(marked_event(&record, true));
    sink.state.marks.get_or_insert_with(Vec::new).push(record);
}

/// R750: the marks of a delayed effect that names every card of a scope (`delayed_id`), made to match
/// `ids`, the cards the scope names now: a card that left it loses its mark and a card that came into
/// it gains one, each with a `marked` event, in that order. Nothing changes, and nothing is said, when
/// they already match.
pub fn sync_marks(sink: &mut EngineSink<'_>, delayed_id: &str, mark: &CardMark, ids: &[String]) {
    let marks: Vec<MarkRecord> = sink.state.marks.clone().unwrap_or_default();
    let is_gone = |record: &MarkRecord| record.delayed_id == delayed_id && !ids.contains(&record.instance_id);
    let gone: Vec<MarkRecord> = marks.iter().filter(|record| is_gone(record)).cloned().collect();
    let added: Vec<MarkRecord> = ids
        .iter()
        .filter(|id| {
            !marks
                .iter()
                .any(|record| record.delayed_id == delayed_id && record.instance_id == **id)
        })
        .map(|instance_id| MarkRecord {
            instance_id: instance_id.clone(),
            mark: mark.mark.clone(),
            color: mark.color.clone(),
            delayed_id: delayed_id.to_string(),
        })
        .collect();
    if gone.is_empty() && added.is_empty() {
        return;
    }
    let mut kept: Vec<MarkRecord> = marks.into_iter().filter(|record| !is_gone(record)).collect();
    kept.extend(added.iter().cloned());
    for record in &gone {
        sink.events.push(marked_event(record, false));
    }
    for record in &added {
        sink.events.push(marked_event(record, true));
    }
    if kept.is_empty() {
        sink.state.marks = None;
    } else {
        sink.state.marks = Some(kept);
    }
}

/// R1141: a mark of a hand watch's lasts while that card is still watched — its stay in the hand.
fn waiting(state: &GameState, record: &MarkRecord) -> bool {
    state.delayed.iter().any(|entry| {
        entry.id == record.delayed_id
            && entry
                .hand_watch
                .as_ref()
                .is_none_or(|ids| ids.contains(&record.instance_id))
    })
}

/// R437: drop the marks whose delayed effect is no longer waiting — it resolved, fizzled, or was
/// forgotten as its card left the field — each with a `marked` event (`added: false`), in the order
/// the marks were made. Called where the resolution loop collects events (`triggers::collect_events`),
/// so the removal follows the event that ended it and is dispatched with the rest.
pub fn sweep_marks(sink: &mut EngineSink<'_>) {
    let Some(marks) = sink.state.marks.clone() else {
        return;
    };
    let state: &GameState = sink.state;
    let still: Vec<bool> = marks.iter().map(|record| waiting(state, record)).collect();
    if still.iter().all(|kept| *kept) {
        return;
    }
    let mut kept: Vec<MarkRecord> = Vec::new();
    for (record, stays) in marks.into_iter().zip(still) {
        if stays {
            kept.push(record);
        } else {
            sink.events.push(marked_event(&record, false));
        }
    }
    if kept.is_empty() {
        sink.state.marks = None;
    } else {
        sink.state.marks = Some(kept);
    }
}

/// R437: the marks a card carries now, in the order they were made — only those still waiting.
pub fn marks_on(state: &GameState, instance_id: &str) -> Vec<CardMark> {
    state
        .marks
        .iter()
        .flatten()
        .filter(|record| record.instance_id == instance_id && waiting(state, record))
        .map(|record| CardMark {
            mark: record.mark.clone(),
            color: record.color.clone(),
        })
        .collect()
}
