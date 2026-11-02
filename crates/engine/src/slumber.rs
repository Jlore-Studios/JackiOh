//! Dormant (SPEC §6.1, R1839000–R1839003): Stinkyscape's Hearthstone-style status. A Unit that prints
//! it (`StaticFlags.slumber`) enters the field Dormant, any way it arrives, and until it awakens it only
//! fills its zone: it is no Unit for anything that reads the Units (`zones::active_units_of`), it does
//! not act (`zones::acts_on_field`), no effect reaches it (`restrictions::unaffected_by`), and its
//! auras and triggers are off. Only an awakening reaches it: its countdown's end or `effects::awaken`.
//!
//! The identifiers say "slumber" and "awaken", never "dormant": that word already names a card beneath
//! a Stack pile in the engine (`zones::dormant_units_of`), a place and not this status (§3.2, R13).
//!
//! `CardInstance.slumber` is absent on every other card, so a game with no Dormant card serializes,
//! hashes and plays exactly as before (D14). It is a status, never text: R78's reset takes it off with
//! the card leaving the field.

use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, Slumber, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PlayerId, ZoneName};

/// R1839000: whether the card is Dormant now (only a card on the field ever is).
pub fn is_slumbering(card: &CardInstance) -> bool {
    card.slumber.is_some() && card.zone.z() == ZoneName::Field
}

/// R1839001: the turns left on a Dormant card's countdown; `None` if not Dormant or awaiting an effect.
pub fn slumber_turns_of(card: &CardInstance) -> Option<i32> {
    card.slumber.and_then(|slumber| slumber.turns)
}

/// R1839000, R1839001: a Unit (`as_unit`: in a unit zone, or standing in or carried on a backrow zone,
/// R1081000, R446) arriving from off the field whose running text prints Dormant enters Dormant, with
/// its printed countdown; "Dormant for 0 turns" is no Dormant. A field-to-field move is no arrival, and
/// a card set face-down as a Trap has no text in force (R1487000), so neither starts one.
pub fn start_slumber_on_field(
    state: &GameState,
    card: &mut CardInstance,
    from_off_field: bool,
    as_unit: bool,
) {
    if !from_off_field || !as_unit || card.set_as.is_some() || card.slumber.is_some() {
        return;
    }
    let Some(text) = crate::scripts::flags_of(state, card).slumber else {
        return;
    };
    if text.turns.is_some_and(|turns| turns <= 0) {
        return;
    }
    card.slumber = Some(Slumber { turns: text.turns });
}

/// R1839002: the Activate ability a Dormant card keeps while it sleeps (`StaticFlags.slumber.wakesBy`).
pub fn wakes_by_of(state: &GameState, card: &CardInstance) -> Option<String> {
    crate::scripts::flags_of(state, card)
        .slumber
        .and_then(|text| text.wakes_by)
}

/// R1839002: whether a Dormant card acts for its awakening alone, topping its zone with an ability it
/// keeps while it sleeps; `subsystems::activate` lets it activate that ability and no other.
pub fn acts_to_awaken(state: &GameState, card: &CardInstance) -> bool {
    is_slumbering(card)
        && !crate::zones::is_buried(state, card)
        // R1852003: a Dormant Starship keeps its launch, the engine's ability.
        && (wakes_by_of(state, card).is_some() || crate::starship::is_dormant_starship(card))
}

/// R1839002: the card awakens, summoning sick for the rest of this turn (R2259000), and `awakened` says
/// so. False, changing nothing, for a card that is not Dormant or lies beneath a Stack pile, where it
/// acts for nothing (its countdown waits there, R1839003).
pub fn awaken_card(sink: &mut EngineSink<'_>, instance_id: &str) -> bool {
    let Some(card) = find_instance(sink.state, instance_id) else {
        return false;
    };
    if !is_slumbering(card) || crate::zones::is_buried(sink.state, card) {
        return false;
    }
    let turn = sink.state.turn;
    let Some(live) = find_instance_mut(sink.state, instance_id) else {
        return false;
    };
    live.slumber = None;
    // R2259000, §4.1: sickness is "entered the field this turn" (`combat::is_sick`), which an awakening
    // starts afresh, whenever the card arrived.
    live.summoned_turn = Some(turn);
    let event = GameEvent::Awakened {
        player: live.controller,
        instance_id: live.id.clone(),
        def_id: live.def_id.clone(),
    };
    sink.events.push(event);
    // R1852003: a Starship's awakening is its launch, whatever awakened it.
    crate::starship::launch(sink, instance_id);
    true
}

/// R1839001: the start of `player`'s turn, right after the Brittle tick (§2.2, R62): every Dormant card
/// with a countdown that tops a zone on their side counts down, and one reaching 0 awakens there and
/// then, in `zones::units_on_field_of`'s order. A card beneath a Stack pile does not count (R1839003).
pub fn awaken_at_turn_start(sink: &mut EngineSink<'_>, player: PlayerId) {
    let counting: Vec<String> = crate::zones::units_on_field_of(sink.state, player)
        .into_iter()
        .filter(|card| card.controller == player && slumber_turns_of(card).is_some())
        .map(|card| card.id.clone())
        .collect();
    for id in counting {
        let Some(live) = find_instance_mut(sink.state, &id) else {
            continue;
        };
        let Some(turns) = slumber_turns_of(live) else {
            continue;
        };
        if turns > 1 {
            live.slumber = Some(Slumber {
                turns: Some(turns - 1),
            });
            continue;
        }
        awaken_card(sink, &id);
    }
}

/// §6.2 "When this awakens" (R1839002): the trigger a Dormant card prints, answering only its own
/// `awakened`. The card is awake by then, so it registers the trigger as any card does, in R68's order.
pub fn when_this_awakens(
    id: impl Into<String>,
    run: impl Fn(&mut crate::script::EffectContext<'_>, &GameEvent) -> Vec<crate::script::Effect>
    + Send
    + Sync
    + 'static,
) -> crate::script::TriggerDef {
    crate::script::TriggerDef::new(id, &[crate::wire::GameEventType::Awakened], run).with_when(
        |ctx, event| {
            matches!(
                event,
                GameEvent::Awakened { instance_id, .. }
                    if ctx.self_.as_ref().is_some_and(|this| &this.id == instance_id)
            )
        },
    )
}
