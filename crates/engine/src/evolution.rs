//! Evolve and Evolves (SPEC §6.2, §6.3; R1849001, R3536000–R3536005): a permanent's `Evolve:`
//! text runs once, the first time it evolves; `Evolves when …` is a standing condition the settled
//! state check reads, or the card's own trigger evolving itself; "evolve all cards in your deck"
//! marks library cards to evolve as they enter the field.

use crate::multipliers::{Multiplied, extra_runs};
use crate::script::{EngineSink, HookArgs};
use crate::state::{CardInstance, find_instance_mut};
use crate::wire::{GameEvent, PlayerId};

/// R3536000: whether `card` can evolve now: it has not evolved yet and its text prints an Evolve
/// hook. A Vanilla card's script is empty, so it never evolves.
pub fn can_evolve(state: &crate::state::GameState, card: &CardInstance) -> bool {
    card.evolved != Some(true) && crate::scripts::script_of(state, card).evolve.is_some()
}

/// R3536000, R3536003, R3536004, R3536005: mark `card` evolved, count it for its controller and
/// report it; returns how many times its Evolve text runs in all — once, plus the highest acting
/// evolution multiplier (R3536003). The mark lands before anything runs, so a holding condition
/// fires once and the once-only rule counts evolvings, not repeats.
fn begin(sink: &mut EngineSink<'_>, card: &CardInstance) -> usize {
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.evolved = Some(true);
        live.enters_evolved = None;
    }
    let evolutions = sink.state.players[card.controller].evolutions.unwrap_or(0);
    sink.state.players[card.controller].evolutions = Some(evolutions.saturating_add(1));
    sink.events.push(GameEvent::Evolved {
        player: card.controller,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
    });
    1 + extra_runs(sink.state, card.controller, Multiplied::Evolutions).max(0) as usize
}

/// R1849001, R3536000: the `evolve` verb's body: evolve `card` — mark it, run its Evolve text once
/// inline now (the resumable run, so a question in it pauses the list that evolved it) and queue
/// the multiplier's extra runs. A card with no Evolve text, one that already evolved, or one
/// neither acting on the field nor lying in a graveyard evolves into nothing.
pub fn evolve_now(sink: &mut EngineSink<'_>, card: &CardInstance) {
    if !crate::zones::acts_on_field(sink.state, card) && card.zone.z() != crate::wire::ZoneName::Graveyard {
        return;
    }
    if !can_evolve(sink.state, card) {
        return;
    }
    let runs = begin(sink, card);
    for _ in 1..runs {
        crate::triggers::queue_evolve_run(sink, card);
    }
    if let Some(live) = crate::state::find_instance(sink.state, &card.id).cloned() {
        crate::prompts::run_evolve(sink, &live, live.controller);
    }
}

/// R3536002: "Evolve all cards in your deck": mark every library card of `player` that prints an
/// Evolve text to enter the field evolved. No event: the library is hidden (CLAUDE.md rule 7).
pub fn mark_library(sink: &mut EngineSink<'_>, player: PlayerId) {
    let ids: Vec<String> = sink.state.players[player]
        .library
        .iter()
        .filter(|card| can_evolve(sink.state, card))
        .map(|card| card.id.clone())
        .collect();
    for id in ids {
        if let Some(live) = find_instance_mut(sink.state, &id) {
            live.enters_evolved = Some(true);
        }
    }
}

/// R3536001, R3536002: the settled board's evolutions, read in R68's order: a marked card evolves
/// as it entered, and a card whose `evolves_when` holds evolves. A face-down or Dormant card keeps
/// its mark until its text acts (`lasting_text_off`).
pub fn evolve_standing(sink: &mut EngineSink<'_>) {
    for card in crate::state_check::permanents_in_play(sink.state) {
        if !crate::zones::acts_on_field(sink.state, &card) {
            continue;
        }
        if crate::preview::lasting_text_off(sink.state, &card) {
            continue;
        }
        let marked = card.enters_evolved == Some(true);
        if marked && let Some(live) = find_instance_mut(sink.state, &card.id) {
            live.enters_evolved = None;
        }
        let holds = !marked
            && can_evolve(sink.state, &card)
            && crate::scripts::script_of(sink.state, &card)
                .evolves_when
                .as_ref()
                .is_some_and(|when| {
                    when(HookArgs {
                        state: sink.state,
                        self_: &card,
                        radiant: card.radiant,
                    })
                });
        if (marked && can_evolve(sink.state, &card)) || holds {
            let runs = begin(sink, &card);
            for _ in 0..runs {
                crate::triggers::queue_evolve_run(sink, &card);
            }
        }
    }
}
