//! R2783000: the marker a card's hook emits as it begins to resolve (`hookResolving`, §10.3), so the
//! client has a moment to anchor the card's own animation on. Presentation only: the marker is withheld
//! from the trigger loop (R1361's way), takes no number in R68's order and moves no hash, and a seat
//! that could not read the card where it stood as the hook began never sees it (`view_for`).
//!
//! Each first run of a Cry, a Death, a start-of-turn or end-of-turn hook, a Whenever trigger and an
//! Activate is marked once, just before its effects, whatever they then do (a hook that fizzles is
//! marked too); a continuation after a prompt is the same run and has none. A card set face-down by
//! its Cloaked, its text inert (R1040), marks nothing, though the play pipeline runs its Cry.
//!
//! R5189000 marks the rest the same way: an `onPlayHook`, a start-of-game clause and a "When this enters
//! your hand" clause by their names here, an attack's five hooks as `combat::run_engine_hook` runs each,
//! and a delayed effect at its R62 point through `ResumeOptions.mark` (`turn::run_resume_for`). Those
//! markers take no room in the loop cap's count either (`loop_cap::leave_uncounted`), so a capped loop
//! trips where it did before they were marked.

use crate::script::{ACTIVATION_HOOK_PREFIX, EngineSink};
use crate::state::{CardInstance, find_instance};
use crate::wire::{GameEvent, HookKind, PlayerId};

/// The marker's kind for a hook a `Resume` names, or `None` for a hook no marker reports by its name (a
/// step a prompt's answer re-enters, `revealed`, `evolve`, …). R5189000: an attack's hooks are marked by
/// `combat::run_engine_hook`, which runs them with no `Resume`, and a delayed effect, whose `Resume`
/// names its step's hook (`delayed`, `resume`, …), through `ResumeOptions.mark`.
pub fn hook_kind_of(hook: &str) -> Option<HookKind> {
    match hook {
        "cry" => Some(HookKind::Cry),
        "death" => Some(HookKind::Death),
        "startOfTurn" | "startOfOpponentTurn" => Some(HookKind::StartOfTurn),
        "endOfTurn" => Some(HookKind::EndOfTurn),
        "onPlayHook" => Some(HookKind::OnPlayHook),
        "startOfGame" => Some(HookKind::StartOfGame),
        "entersHand" => Some(HookKind::EntersHand),
        _ if hook.starts_with(ACTIVATION_HOOK_PREFIX) => Some(HookKind::Activate),
        _ => None,
    }
}

/// R5189000: whether `hook` is one R2783000 marked first, whose markers the loop cap counts as it always
/// did; the markers of the hooks R5189000 added take no room in its count (`loop_cap::leave_uncounted`).
pub fn first_marked(hook: HookKind) -> bool {
    matches!(
        hook,
        HookKind::Cry
            | HookKind::Death
            | HookKind::StartOfTurn
            | HookKind::EndOfTurn
            | HookKind::Whenever
            | HookKind::Activate
    )
}

/// Emit `card`'s marker for `hook`, run by `controller`. The card is read as it stands now (as it
/// died, for a Death hook's snapshot of a card no longer anywhere), and the seats that cannot read it
/// there (R177's `unreadable_by`) are recorded on the marker, which their views then withhold. A card
/// whose own text is dormant as the hook's run reads it (set by its Cloaked and not yet revealing,
/// R783004, R1040) marks nothing: the play pipeline still runs a set card's Cry, which the card's script
/// answers with nothing, so no hook of its resolves.
pub fn mark_hook(sink: &mut EngineSink<'_>, card: &CardInstance, controller: PlayerId, hook: HookKind) {
    if crate::alt_play::is_dormant(card) {
        return;
    }
    let live = find_instance(sink.state, &card.id).unwrap_or(card);
    let def_id = live.def_id.clone();
    let hidden = crate::effects::card_scope::unreadable_by(sink.state, live);
    push_marker(sink, card.id.clone(), def_id, controller, hook, hidden);
}

/// R127: the marker of a hook that runs with no card as its `self`: a Cry out of a graveyard (R467), an
/// Activate whose card its cost took. R3182000: `sourceId` is still the card the hook belongs to
/// (`card`), so the view judges the marker by it as it judges the card's other events (R97), and the
/// seats that cannot read that card where it stands now are recorded as `mark_hook` records them; it is
/// `""`, as on `rolled` (R2652001), only for a hook no card holds at all.
pub fn mark_cardless_hook(
    sink: &mut EngineSink<'_>,
    card: Option<&str>,
    def_id: &str,
    controller: PlayerId,
    hook: HookKind,
) {
    let hidden = card
        .and_then(|id| find_instance(sink.state, id))
        .map(|live| crate::effects::card_scope::unreadable_by(sink.state, live))
        .unwrap_or_default();
    push_marker(
        sink,
        card.unwrap_or_default().to_string(),
        def_id.to_string(),
        controller,
        hook,
        hidden,
    );
}

fn push_marker(
    sink: &mut EngineSink<'_>,
    source_id: String,
    def_id: String,
    controller: PlayerId,
    hook: HookKind,
    hidden: Vec<PlayerId>,
) {
    // R3925000: at the loop cap card text is off, so no hook resolves to be marked.
    if crate::loop_cap::capped(sink) {
        return;
    }
    sink.events.push(GameEvent::HookResolving {
        source_id,
        def_id,
        controller,
        hook,
        hidden_from: (!hidden.is_empty()).then_some(hidden),
    });
    // R2783000: nothing answers it, so the frontier never takes it (R1361's way, as `rolled`'s).
    let at = sink.events.len() - 1;
    crate::triggers::withhold_report(sink, at);
    // R5189000: and a marker of a hook it added leaves the loop cap's count as it was.
    if !first_marked(hook) {
        crate::loop_cap::leave_uncounted(sink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r2783000_names_the_hooks_a_marker_reports_and_no_other() {
        assert_eq!(hook_kind_of("cry"), Some(HookKind::Cry));
        assert_eq!(hook_kind_of("death"), Some(HookKind::Death));
        assert_eq!(hook_kind_of("startOfTurn"), Some(HookKind::StartOfTurn));
        assert_eq!(hook_kind_of("startOfOpponentTurn"), Some(HookKind::StartOfTurn));
        assert_eq!(hook_kind_of("endOfTurn"), Some(HookKind::EndOfTurn));
        assert_eq!(
            hook_kind_of(&crate::script::activation_hook("power#2")),
            Some(HookKind::Activate)
        );
        for other in ["delayed", "afterAttack", "resume", ""] {
            assert_eq!(hook_kind_of(other), None, "{other}");
        }
    }

    /// R5189000: the three hooks a `Resume` runs by name are marked by it; an attack's hooks and a
    /// delayed effect are marked where they run, and only R2783000's six are counted by the loop cap.
    #[test]
    fn r5189000_names_the_hooks_a_resume_marks() {
        assert_eq!(hook_kind_of("onPlayHook"), Some(HookKind::OnPlayHook));
        assert_eq!(hook_kind_of("startOfGame"), Some(HookKind::StartOfGame));
        assert_eq!(hook_kind_of("entersHand"), Some(HookKind::EntersHand));
        for other in [
            "whenAttacks",
            "afterAttack",
            "afterAttacked",
            "allyAttacked",
            "allyAttack",
            "delayed",
        ] {
            assert_eq!(hook_kind_of(other), None, "{other}");
        }
        let first: Vec<HookKind> = HookKind::ALL
            .iter()
            .copied()
            .filter(|kind| first_marked(*kind))
            .collect();
        assert_eq!(first, HookKind::ALL[..6]);
    }
}
