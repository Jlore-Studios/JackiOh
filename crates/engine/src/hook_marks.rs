//! R2783000: the marker a card's hook emits as it begins to resolve (`hookResolving`, §10.3), so the
//! client has a moment to anchor the card's own animation on. Presentation only: the marker is withheld
//! from the trigger loop (R1361's way), takes no number in R68's order and moves no hash, and a seat
//! that could not read the card where it stood as the hook began never sees it (`view_for`).
//!
//! Each first run of a Cry, a Death, a start-of-turn or end-of-turn hook, a Whenever trigger and an
//! Activate is marked once, just before its effects, whatever they then do (a hook that fizzles is
//! marked too); a continuation after a prompt is the same run and has none.

use crate::script::{ACTIVATION_HOOK_PREFIX, EngineSink};
use crate::state::{CardInstance, find_instance};
use crate::wire::{GameEvent, HookKind, PlayerId};

/// The marker's kind for a hook a `Resume` names, or `None` for a hook no marker reports (a delayed
/// effect, `onPlayHook`, an attack's hooks, a step a prompt's answer re-enters, …).
pub fn hook_kind_of(hook: &str) -> Option<HookKind> {
    match hook {
        "cry" => Some(HookKind::Cry),
        "death" => Some(HookKind::Death),
        "startOfTurn" | "startOfOpponentTurn" => Some(HookKind::StartOfTurn),
        "endOfTurn" => Some(HookKind::EndOfTurn),
        _ if hook.starts_with(ACTIVATION_HOOK_PREFIX) => Some(HookKind::Activate),
        _ => None,
    }
}

/// Emit `card`'s marker for `hook`, run by `controller`. The card is read as it stands now (as it
/// died, for a Death hook's snapshot of a card no longer anywhere), and the seats that cannot read it
/// there (R177's `unreadable_by`) are recorded on the marker, which their views then withhold.
pub fn mark_hook(sink: &mut EngineSink<'_>, card: &CardInstance, controller: PlayerId, hook: HookKind) {
    let live = find_instance(sink.state, &card.id).unwrap_or(card);
    let def_id = live.def_id.clone();
    let hidden = crate::effects::card_scope::unreadable_by(sink.state, live);
    push_marker(sink, card.id.clone(), def_id, controller, hook, hidden);
}

/// R127: the marker of a Cry that runs with no card (one out of a graveyard, R467), which names its
/// definition alone, `sourceId` `""` as on `rolled` (R2652001).
pub fn mark_cardless_hook(sink: &mut EngineSink<'_>, def_id: &str, controller: PlayerId, hook: HookKind) {
    push_marker(
        sink,
        String::new(),
        def_id.to_string(),
        controller,
        hook,
        Vec::new(),
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
        for other in [
            "delayed",
            "onPlayHook",
            "afterAttack",
            "resume",
            "startOfGame",
            "",
        ] {
            assert_eq!(hook_kind_of(other), None, "{other}");
        }
    }
}
