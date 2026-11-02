//! Echo: SPEC §6.3's Echo row, §10.5 step 6 and R30's Twinspell lifetime — what a resolution owes,
//! where the repeats wait, and where the card lands once they are done (BUILD M3-T2, R30, R70, R113).
//!
//! The repeats outstanding are state, never a loop variable: one `EchoItem` per instance on
//! `state.echoQueue` (§10.1), taken one at a time, so a prompt inside one repeat pauses the rest (§9.3,
//! R113). `view_for.rs` reads the grant for R169's badge. A cast is the same pipeline (R70), entered by
//! `resolve::cast_card` through `play_steps::cast_through_pipeline`.
//!
//! Asking a repeat's fresh prompts (§10.6, R81) stays with the pipeline driver in `play_steps.rs`, the
//! one side that may open prompts.

use indexmap::IndexSet;
use serde_json::Value;

use crate::faces::card_type_of;
use crate::mana::modifier_is_live;
use crate::modifiers::{install_lasting_modifiers, remove_modifier};
use crate::numbers::NumberedKey;
use crate::script::{EngineSink, HookArgs};
use crate::state::{
    CardInstance, EchoItem, GameState, ModifierKind, PlayerModifier, find_instance, find_instance_mut,
};
use crate::stays::{exit_mark, left_field_after};
use crate::subsystems::copied_text::copied_echo;
use crate::tuning::tuned_count;
use crate::wire::{CardType, GameEvent, PlayerId, Selection, Zone};
use crate::zones::{MoveToZoneOptions, OffFieldZone, exile_to_pile, move_to_zone, report_graveyard_landing};

/// A snapshot of the card as it stands now, for a call that takes the card by value while the state is
/// borrowed mutably (the callee finds the live card by its id).
fn snapshot(state: &GameState, instance_id: &str) -> Option<CardInstance> {
    find_instance(state, instance_id).cloned()
}

// The queue (§10.1 `echoQueue`)

/// The repeats this instance still owes, as one queue entry per instance (§10.1 `EchoItem`).
fn echo_entry(state: &GameState, instance_id: &str) -> Option<usize> {
    state
        .echo_queue
        .iter()
        .position(|item| item.instance_id == instance_id)
}

/// How many repeats this instance is still owed.
pub fn echo_repeats_owed(state: &GameState, instance_id: &str) -> i32 {
    let remaining = echo_entry(state, instance_id).map_or(0, |at| state.echo_queue[at].remaining);
    0.max(remaining)
}

/// Add repeats to this instance's entry, creating it when it has none. The id and `seq` come from
/// `state.nextId`/`state.nextSeq`, as every queue entry's do, so a replay builds the queue the live
/// game had (R68).
pub fn add_echo_repeats(
    sink: &mut EngineSink<'_>,
    card: &CardInstance,
    controller: PlayerId,
    count: i32,
) -> i32 {
    let state = &mut *sink.state;
    if count <= 0 {
        return echo_repeats_owed(state, &card.id);
    }

    if let Some(at) = echo_entry(state, &card.id) {
        let existing = &mut state.echo_queue[at];
        existing.remaining += count;
        return existing.remaining;
    }
    state.echo_queue.push(EchoItem {
        id: format!("e{}", state.next_id),
        seq: state.next_seq,
        instance_id: card.id.clone(),
        controller,
        remaining: count,
    });
    state.next_id += 1;
    state.next_seq += 1;
    count
}

/// Take one repeat off this instance's entry, dropping the entry once it owes nothing.
pub fn take_echo_repeat(state: &mut GameState, instance_id: &str) -> bool {
    let Some(at) = echo_entry(state, instance_id) else {
        return false;
    };
    if state.echo_queue[at].remaining <= 0 {
        return false;
    }
    state.echo_queue[at].remaining -= 1;
    if state.echo_queue[at].remaining <= 0 {
        state.echo_queue.remove(at);
    }
    true
}

/// Forget every repeat this instance is owed: it is no longer there to re-resolve (a trap took it
/// off, it ceased to exist), so the entry would otherwise wait in `state.echoQueue` for ever.
pub fn drop_echo_repeats(state: &mut GameState, instance_id: &str) {
    state.echo_queue.retain(|item| item.instance_id != instance_id);
}

// What a resolution owes (§6.3, R30)

/// §6.1: the card's own printed Echo X (`staticFlags.echo`), plus, for a card that has a copied Spell's
/// text (B5 E14, Classic #57 Echo, R546), the Echo X that text prints; fused scripts read the state
/// (SURFACE §6.6). R802: a script that computes its Echo X (`Script.echo_x`) prints the larger of that
/// and `staticFlags.echo`, read once as the card is played (§10.5 step 4), so the X stays fixed.
pub fn printed_echo(card: &CardInstance, state: &GameState) -> i32 {
    let computed = crate::scripts::script_of(state, card)
        .echo_x
        .as_ref()
        .map_or(0, |hook| {
            hook(HookArgs {
                state,
                self_: card,
                radiant: card.radiant,
            })
        });
    let printed = crate::scripts::flags_of(state, card)
        .echo
        .unwrap_or(0)
        .max(computed);
    // B3.4: Echo X is a numbered keyword Degrade and Upgrade move, read through the card's tuning.
    let own = 0.max(tuned_count(card, NumberedKey::Echo.as_str(), printed));
    own + copied_echo(state, card)
}

/// How much Echo one rider grants now. A rider a permanent installed (`sourceId`, #79 Twinspell) is
/// that permanent's lasting effect (§5.1, R209): it grants only while the permanent is on the field
/// under the rider's player, and what its face says now (§5.2: a Twinspell #49 made Radiant says
/// "Echo +2"). A rider with no source (an engine or test fixture) grants its own `amount`.
pub fn echo_grant_of(state: &GameState, player: PlayerId, modifier: &PlayerModifier) -> i32 {
    let ModifierKind::EchoNextSpell { amount, source_id } = &modifier.kind else {
        return 0;
    };
    let Some(source_id) = source_id else {
        return 0.max(*amount);
    };
    let Some(source) = find_instance(state, source_id) else {
        return 0;
    };
    if !matches!(source.zone, Zone::Field { .. }) || source.controller != player {
        return 0;
    }
    // R386: the grant is the permanent's declared number `echoGain` where it declares one.
    0.max(
        crate::scripts::flags_of(state, source)
            .echo_grant
            .map(|printed| crate::params::declared_or(state, source, "echoGain", printed))
            .unwrap_or(*amount),
    )
}

/// R30: Twinspell grants Echo to the next Spell and "stays until a spell is played, then goes to the
/// GY": the grant is spent when it applies and the Field Spell follows. Only a Spell takes it, so a
/// permanent leaves it armed; R70 makes a cast Spell take it like any other.
///
/// R209: one grant per Twinspell on the field: every rider a permanent owns is spent, only the first
/// counts, so a Twinspell that left and came back between two state checks still echoes once.
pub fn granted_echo(sink: &mut EngineSink<'_>, player: PlayerId, card: &CardInstance) -> i32 {
    if card_type_of(sink.state, card) != CardType::Spell {
        return 0;
    }
    // R209: a Twinspell that arrived since the last state check — summoned mid-effect ahead of a cast
    // on draw — already stands on the field, so its rider is installed before the Spell reads them.
    install_lasting_modifiers(sink);

    let mut granted = 0;
    let mut spent: IndexSet<String> = IndexSet::new();
    let mods: Vec<PlayerModifier> = sink.state.players[player].mods.clone();
    for modifier in &mods {
        let ModifierKind::EchoNextSpell { source_id, .. } = &modifier.kind else {
            continue;
        };
        if !modifier_is_live(sink.state, modifier) {
            continue;
        }
        let amount = echo_grant_of(sink.state, player, modifier);
        remove_modifier(sink, player, &modifier.id);
        let Some(source_id) = source_id else {
            granted += amount;
            continue;
        };
        if amount <= 0 || spent.contains(source_id) {
            continue;
        }
        spent.insert(source_id.clone());
        granted += amount;

        let Some(mut source) = snapshot(sink.state, source_id) else {
            continue;
        };
        if !matches!(source.zone, Zone::Field { .. }) {
            continue;
        }
        // B5 E5: to its graveyard, or wherever a replacement sends it.
        let moved = move_to_zone(
            sink.state,
            &mut source,
            OffFieldZone::Graveyard,
            MoveToZoneOptions::default(),
        );
        let landed = snapshot(sink.state, &source.id).unwrap_or(source);
        report_graveyard_landing(sink, &landed, moved);
    }
    granted
}

/// Work out what this resolution owes and put it on the queue: the card's printed `Echo X` plus R30's
/// grant. Returns how many repeats are owed now. Called exactly once per resolution, as §10.5 step 4
/// plays the card (from hand or cast) and sets the run's `echoQueued` flag (R178), because it
/// *consumes* the grant.
pub fn queue_echo_repeats(sink: &mut EngineSink<'_>, card: &CardInstance, player: PlayerId) -> i32 {
    let printed = printed_echo(card, sink.state);
    let granted = granted_echo(sink, player, card);
    add_echo_repeats(sink, card, player, printed + granted)
}

// Where the card lands afterwards, and saying so (§10.5 step 7, R17)

/// R178: the mark a resolving Spell's own "exile this" leaves (#39, #44, #72, #76, #87, #97). §10.5
/// step 7 lands Spells "to the GY or exile" after step 6's Echo repeats (§6.2: "the same instance
/// re-resolve"), so such a Spell stays resolving until step 7 sends it to exile. It lives in the card's
/// own `memory`, which is JSON (§10.1), and step 7 clears it as it acts.
pub const EXILE_ON_LANDING: &str = "@exileOnLanding";

/// R178: send this resolving Spell to exile when §10.5 step 7 lands it, rather than now.
pub fn exile_on_landing(card: &mut CardInstance) {
    card.memory
        .insert(EXILE_ON_LANDING.to_string(), Value::Bool(true));
}

/// Which resolution is finishing, so the event can be emitted for a card that no longer exists.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedCard {
    pub instance_id: String,
    pub def_id: String,
    /// The player who played or cast it, which `cardResolved` reports.
    pub player: PlayerId,
    /// The mana actually charged, as `cardPlayed` reports it: after every modifier, with R65's X and
    /// embiggen prices, and 0 for a cast (R70). R89: a trigger answering this event reads it ON the
    /// event, as the instance may be reset between steps 4 and 7 (#60 Bear Honeypot's R56 threshold).
    pub cost_paid: i32,
    /// The face the card was played with (after §10.5 step 3's Gifted Program hook), for a card that
    /// has ceased to exist by step 7 (R34, R57).
    pub radiant: bool,
    /// R174, R61: the field's departures once §10.5 step 4 had put a permanent on the field. "Still in
    /// play" is asked of that stay: a played unit back through Reborn by step 7 is a new arrival (R83),
    /// so #60's tokens do not attack it and #85 does not fuse it away.
    pub placed_from: Option<u32>,
    /// R119: the permanents that arrived on the field while the play resolved, which do not answer it.
    pub arrived_during: Option<Vec<String>>,
    /// R1180005: the enemies the Spell targeted as it resolved, in order, taken off the card before it
    /// lands (`targeted::take_spell_targets`). Empty for anything that noted none.
    pub targeted: Vec<Selection>,
    /// R2179002: the card was cast on draw, a cast that is no play, which `cardResolved` reports.
    pub on_draw: bool,
}

// §10.5 step 7, both halves, for a play and for a cast (R70) alike. The card lands "to the GY or
// exile" only now, after the script and every Echo repeat, so a repeat that reads a graveyard does
// not find it there. Then `cardResolved`, once per resolution (R17; R61: after a played permanent's
// Cry); `permanent` says whether the card is still in play, and a card that left play is still named.

/// R61, R174: whether the card the play put on the field is still in play, on that same stay.
fn still_in_play(sink: &EngineSink<'_>, resolved: &ResolvedCard) -> bool {
    let Some(card) = find_instance(sink.state, &resolved.instance_id) else {
        return false;
    };
    if !matches!(card.zone, Zone::Field { .. }) {
        return false;
    }
    match resolved.placed_from {
        None => true,
        Some(from) => !left_field_after(sink.state, from, &card.id),
    }
}

pub fn land_after_resolution(sink: &mut EngineSink<'_>, resolved: &ResolvedCard) {
    let card = snapshot(sink.state, &resolved.instance_id);
    let resolving = card
        .as_ref()
        .is_some_and(|card| matches!(card.zone, Zone::Resolving { .. }));
    let exiles = card
        .as_ref()
        .is_some_and(|card| card.memory.get(EXILE_ON_LANDING) == Some(&Value::Bool(true)));

    if let Some(mut card) = card
        && resolving
    {
        if exiles {
            // R178: the Spell's own "exile this", carried out at the step §10.5 gives it.
            if let Some(live) = find_instance_mut(sink.state, &card.id) {
                live.memory.shift_remove(EXILE_ON_LANDING);
            }
            exile_to_pile(sink, snapshot(sink.state, &card.id).unwrap_or(card));
        } else {
            // B5 E5: its graveyard, or wherever a replacement sends it (Classic #50's exile, #60's library).
            let moved = move_to_zone(
                sink.state,
                &mut card,
                OffFieldZone::Graveyard,
                MoveToZoneOptions::default(),
            );
            let landed = snapshot(sink.state, &card.id).unwrap_or(card);
            report_graveyard_landing(sink, &landed, moved);
        }
    }

    let permanent = still_in_play(sink, resolved);
    let radiant =
        find_instance(sink.state, &resolved.instance_id).map_or(resolved.radiant, |card| card.radiant);
    let arrived_during = match &resolved.arrived_during {
        Some(arrived) if !arrived.is_empty() => Some(arrived.clone()),
        _ => None,
    };
    // R174, R212: the stays step 7 read `permanent` on. A cast's event waits for the loop of the
    // effect that cast it (R70), so a response judges the card's stay from here, not the dispatch.
    let exits_from = exit_mark(sink.state);
    // R1180005: the card that answers a Spell that targeted an enemy, named once the Spell has landed;
    // R1180006 bounds the chain of answers.
    let answer = crate::targeted::name_spell_answer(
        sink.state,
        resolved.player,
        &resolved.instance_id,
        &resolved.targeted,
    );
    sink.events.push(GameEvent::CardResolved {
        player: resolved.player,
        instance_id: resolved.instance_id.clone(),
        def_id: resolved.def_id.clone(),
        permanent,
        cost_paid: resolved.cost_paid,
        radiant: Some(radiant),
        arrived_during,
        exits_from: Some(exits_from),
        answer,
        on_draw: resolved.on_draw.then_some(true),
    });
}
