//! Echo: SPEC §6.3's Echo row, §10.5 step 6 and R30's Twinspell lifetime — what a resolution owes,
//! where the repeats wait, and where the card lands once they are done (BUILD M3-T2, R30, R70, R113).
//!
//! §6.3: "Echo X | Recast this card X more times | Play resolves, then the same instance re-resolves
//! X times with fresh mode/target prompts; Twinspell grants Echo +1 to the next spell." The repeats
//! outstanding are state, never a loop variable: one `EchoItem` per instance on `state.echoQueue`
//! (§10.1), taken one at a time, so a prompt inside one repeat pauses the rest (§9.3, R113).
//!
//! Why Echo is its own module rather than a section of `play_steps.rs`: what the play pipeline and
//! its readers share here is state and rules, not prompting — the queue, the printed `Echo X`, R30's
//! grant and where a resolved card goes (§10.5 step 7) — and `view_for.rs` reads the grant for R169's
//! badge. A cast is the same pipeline (R70: "a cast Spell does use Twinspell's Echo"), entered by
//! `resolve::cast_card` through `play_steps::cast_through_pipeline`.
//!
//! Asking a repeat's fresh prompts (§10.6 "an Echo repeat of Glowy Jelly Bean reopens its hand
//! pick", R81) is the other half of step 6, and it stays with the pipeline driver in `play_steps.rs`,
//! the one side that may open prompts.
//!
//! Port of `packages/engine/src/echo.ts`.

use indexmap::IndexSet;
use serde_json::Value;

use crate::config::TUNE_MIN_AMOUNT;
use crate::faces::card_type_of;
use crate::mana::modifier_is_live;
use crate::modifiers::{install_lasting_modifiers, remove_modifier};
use crate::script::{EngineSink, HookArgs};
use crate::state::{
    CardInstance, EchoItem, GameState, ModifierKind, PlayerModifier, find_instance, find_instance_mut,
};
use crate::stays::{exit_mark, left_field_after};
use crate::subsystems::copied_text::copied_echo;
use crate::wire::{CardType, GameEvent, PlayerId, Zone};
use crate::zones::{MoveResult, MoveToZoneOptions, OffFieldZone, move_to_zone, report_graveyard_landing};

/// TS `tuning.tunedCount(card, "Echo", printed)`, a private copy (fullsend rule 5). B3.4, R386: the
/// value a numbered keyword has now — its printed value (or the value KY's Constant set outright,
/// `tuning.set`) moved by the card's tuning steps for its key. A number the card does not print (0) is
/// never tuned into existence. Never below TUNED_FLOOR (= TUNE_MIN_AMOUNT).
fn tuned_echo(card: &CardInstance, printed: i32) -> i32 {
    const KEY: &str = "Echo";
    if printed <= 0 {
        return printed;
    }
    let set = card
        .tuning
        .as_ref()
        .and_then(|tuning| tuning.set.as_ref())
        .and_then(|set| set.get(KEY).copied());
    let step = card
        .tuning
        .as_ref()
        .and_then(|tuning| tuning.x.as_ref())
        .and_then(|x| x.get(KEY).copied())
        .unwrap_or(0);
    if set.is_none() && step == 0 {
        return printed;
    }
    TUNE_MIN_AMOUNT.max(set.unwrap_or(printed) + step)
}

/// A snapshot of the card as it stands now, for a call that takes the card by value while the state is
/// borrowed mutably (the callee finds the live card by its id).
fn snapshot(state: &GameState, instance_id: &str) -> Option<CardInstance> {
    find_instance(state, instance_id).cloned()
}

// ---------------------------------------------------------------------------
// The queue (§10.1 `echoQueue`)
// ---------------------------------------------------------------------------

/// The repeats this instance still owes, as one queue entry per instance (§10.1 `EchoItem`).
fn echo_entry(state: &GameState, instance_id: &str) -> Option<usize> {
    state
        .echo_queue
        .iter()
        .position(|item| item.instance_id == instance_id)
}

/// How many repeats this instance is still owed, for a caller that only wants to know.
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
/// off, it ceased to exist), so the entry would otherwise wait in `state.echoQueue` for ever. The
/// repeats were queued as the card was played (§10.5 step 4), before anything could remove it.
pub fn drop_echo_repeats(state: &mut GameState, instance_id: &str) {
    state.echo_queue.retain(|item| item.instance_id != instance_id);
}

// ---------------------------------------------------------------------------
// What a resolution owes (§6.3, R30)
// ---------------------------------------------------------------------------

/// §6.1: the card's own printed Echo X (`staticFlags.echo`), plus, given the state, for a card that has
/// a copied Spell's text (B5 E14, Classic #57 Echo, R546), the Echo X that text prints.
///
/// TS's `state` was optional; every caller passed it, and Rust needs it to read the card's script
/// (fused scripts are built from the state, SURFACE §6.6), so it is required here.
///
/// R802: a card whose script computes its Echo X (`Script.echo_x`, Meditative #5) prints the larger of
/// that and `staticFlags.echo`. The pipeline reads this once, as the card is played (§10.5 step 4,
/// `queue_echo_repeats`), so the X it queues stays fixed while the card resolves.
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
    let own = 0.max(tuned_echo(card, printed));
    own + copied_echo(state, card)
}

/// How much Echo one rider grants now. A rider a permanent installed (`sourceId`, #79 Twinspell) is
/// that permanent's lasting effect (§5.1, R209), so it grants only while the permanent is on the
/// field under the rider's player, and it grants what the permanent's face says NOW: §5.2 has a card
/// made Radiant on the field run its radiant text from then on, so a Twinspell #49 radiant steals
/// says "Echo +2" (`staticFlags.echoGrant`) whatever its base face installed. A rider with no source
/// (an engine or test fixture) grants its own `amount`. Returns 0 for a rider whose permanent has
/// gone, which the state check would end anyway (R209).
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
/// GY". The grant is spent here — when it applies — and the Field Spell that gave it follows. Only a
/// Spell takes it ("the next Spell you play"), so a cast or played permanent leaves it armed, and
/// R70 makes a cast Spell take it like any other.
///
/// R209: one grant per Twinspell on the field. Every rider a permanent owns is spent with it, but only
/// the first counts, so a Twinspell that left the field and came back between two state checks —
/// the second stay installing a second rider before the first was ended — still echoes once.
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

/// Work out what this resolution owes and put it on the queue, once: the card's printed `Echo X`
/// plus whatever R30's grant adds. Returns how many repeats are owed now, so a caller that has to
/// decide whether to hand off a tail at all can ask in one line.
///
/// Called exactly once per resolution — §10.5 step 4 calls it as the card is played, for a play from
/// hand and for a cast alike, and sets the run's `echoQueued` flag (R178: a Spell gains its Echo as
/// it is played) — because it *consumes* the grant.
pub fn queue_echo_repeats(sink: &mut EngineSink<'_>, card: &CardInstance, player: PlayerId) -> i32 {
    let printed = printed_echo(card, sink.state);
    let granted = granted_echo(sink, player, card);
    add_echo_repeats(sink, card, player, printed + granted)
}

// ---------------------------------------------------------------------------
// Where the card lands afterwards, and saying so (§10.5 step 7, R17)
// ---------------------------------------------------------------------------

/// R178: the mark a resolving Spell's own "exile this" leaves (#39, #44, #72, #76, #87, #97). §10.5
/// puts "Spells go to the GY or exile" at step 7, after step 6's Echo repeats, and §6.2 has "the same
/// instance re-resolve" — so a Spell that says "exile this" is still resolving until step 7, which
/// sends it to exile instead of the graveyard. Moving it at once had it leave before its own
/// repeats: Twinspell's grant (R30) was never taken and the repeats never came.
///
/// It lives in the card's own `memory`, which is JSON (§10.1), and step 7 clears it as it acts.
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
    /// The mana actually charged, which is the same number `cardPlayed` reports: after every modifier,
    /// with R65's X and embiggen prices included, and 0 for a cast (R70: "a cast is free and counts as
    /// a play … with cost paid 0"). The caller passes it because R89 is the hazard — a trigger
    /// answering this event must find what it needs ON the event, since the instance may have been
    /// reset between step 4 and step 7 — so #60 Bear Honeypot's "costing 1 or less" (R56: "the cost
    /// actually paid after modifiers") is never looked up off the board.
    pub cost_paid: i32,
    /// The face the card was played with (after §10.5 step 3's Gifted Program hook), for a card that
    /// has ceased to exist by step 7 — Sheepish transformed it at step 4 — so the event can still say
    /// what resolved (R34, R57).
    pub radiant: bool,
    /// R174, R61: the field's departures once §10.5 step 4 had put a permanent on the field. "Still in
    /// play" is asked of that stay: a played unit that died in its own resolution (its Cry, or the check
    /// after step 6) and is back through Reborn by step 7 is a new arrival (R83), not the card that was
    /// played, so #60's tokens do not attack it and #85 does not fuse it away.
    pub placed_from: Option<u32>,
    /// R119: the permanents that arrived on the field while the play resolved, which do not answer it.
    pub arrived_during: Option<Vec<String>>,
}

// §10.5 step 7, both halves, for a play and for a cast (R70) alike.
//
// Where the card lands: "Spells go to the GY or exile" — and only now, after the script and every
// Echo repeat have run, so a repeat that reads a graveyard does not find the card already in it
// (the M3 review filed the other order as B-4). A card a script sent somewhere else, and a
// permanent that step 4 put on the field, have already left `resolving`, so nothing moves for them.
//
// Then `cardResolved`, the moment §10.5 step 7 and R17 name: "Unstable Clone Machine and Bear
// Honeypot fire after resolution; Unlicensed Experimentation fires after a played permanent's Cry
// (R61)", as against Sheepish, which answers step 4's `summoned` and costs the card its Cry. It is
// emitted exactly once per resolution — this is step 7, which runs after step 6 has drained every
// repeat — and `permanent` says whether the card is still in play, which is R61's distinction. A
// card that has left play by now (a Spell in its graveyard, a unit a trap took, a token that ceased
// to exist) reports `permanent: false`, and the event still names it, so a trap can see what
// resolved. `costPaid` rides along for the same reason: R89 has a trigger read the event rather
// than the board, so the caller hands over the number it charged (0 for a cast, R70) and #60 Bear
// Honeypot's R56 threshold never re-derives a cost from an instance step 7 may have reset.

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
            let mut card = snapshot(sink.state, &card.id).unwrap_or(card);
            if move_to_zone(
                sink.state,
                &mut card,
                OffFieldZone::Exile,
                MoveToZoneOptions::default(),
            ) == MoveResult::Moved
            {
                sink.state.counters.exiled += 1;
            }
            sink.events.push(GameEvent::Exiled {
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                owner: card.owner,
            });
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
    sink.events.push(GameEvent::CardResolved {
        player: resolved.player,
        instance_id: resolved.instance_id.clone(),
        def_id: resolved.def_id.clone(),
        permanent,
        cost_paid: resolved.cost_paid,
        radiant: Some(radiant),
        arrived_during,
        exits_from: Some(exits_from),
    });
}
