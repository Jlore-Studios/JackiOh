//! Dormant (SPEC §6.1, R1839000–R1839003): Stinkyscape's Hearthstone-style status. A Unit that prints
//! it ("Dormant", "Dormant for N turns", `StaticFlags.slumber`) enters the field Dormant, any way it
//! arrives, and until it awakens it is on the field and nothing more: it fills its zone, so it counts
//! for board space, but it is no Unit for anything that reads the Units (`zones::active_units_of`
//! leaves it out), it does not act (`zones::acts_on_field`), so it neither attacks, switches, is
//! attacked nor is targeted, no effect reaches it (`restrictions::unaffected_by`), and its auras and
//! triggers are off. The one thing that reaches it is an awakening: its countdown's end
//! (`awaken_at_turn_start`) or the `awaken` verb (`effects::awaken`), which a card's own Activate uses
//! (the Dormant Starship's "pay (3) to awaken", the one ability a Dormant card keeps, `wakes_by`).
//!
//! The identifiers say "slumber" and "awaken", never "dormant": that word already names a card beneath
//! a Stack pile in the engine (`zones::dormant_units_of`, `alt_play::is_dormant`,
//! `preview::lasting_text_dormant`), which is a place and not this status (§3.2, R13).
//!
//! The status is `CardInstance.slumber`, absent on every other card, so a game with no Dormant card
//! serializes, hashes and plays exactly as before (D14). It is a status, never text: R78's reset takes
//! it off with the card leaving the field.

use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, Slumber, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PlayerId, ZoneName};

/// R1839000: whether the card is Dormant now. Only a card on the field ever is (R78's reset takes the
/// status off as it leaves).
pub fn is_slumbering(card: &CardInstance) -> bool {
    card.slumber.is_some() && card.zone.z() == ZoneName::Field
}

/// R1839001: the turns left on a Dormant card's countdown, `None` for a card that is not Dormant or that
/// sleeps until an effect awakens it.
pub fn slumber_turns_of(card: &CardInstance) -> Option<i32> {
    card.slumber.and_then(|slumber| slumber.turns)
}

/// R1839000, R1839001: called where every field arrival passes (`zones::place_on_field`), beside the
/// Brittle start; `as_unit` is whether the card lands as a Unit (in a unit zone, or a Unit standing in
/// or carried on a backrow zone, R1081000, R446), since a Field Spell or a Trap never sleeps. A Unit
/// arriving from off the field (a hand, a deck, a graveyard, the resolving zone) whose running text
/// prints Dormant enters Dormant, with its printed countdown; "Dormant for 0 turns" is no Dormant. A
/// move from one field zone to another (a steal, a swap, a rotation) is no arrival, and a card set
/// face-down as a Trap has no text in force (R1487000), so neither starts one.
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

/// R1839002: the Activate ability a Dormant card keeps while it sleeps (`StaticFlags.slumber.wakesBy`),
/// read off its running face; `None` for a card that keeps none.
pub fn wakes_by_of(state: &GameState, card: &CardInstance) -> Option<String> {
    crate::scripts::flags_of(state, card)
        .slumber
        .and_then(|text| text.wakes_by)
}

/// R1839002: whether a Dormant card acts for its awakening alone — on the field and topping its zone
/// (a card beneath a Stack pile acts for nothing, §3.2, R13) with an ability it keeps while it sleeps.
/// `subsystems::activate` lets it activate that ability and no other.
pub fn acts_to_awaken(state: &GameState, card: &CardInstance) -> bool {
    is_slumbering(card)
        && !crate::zones::is_buried(state, card)
        // R1852003: a Dormant Starship keeps its launch, the engine's ability.
        && (wakes_by_of(state, card).is_some() || crate::starship::is_dormant_starship(card))
}

/// R1839002: the card awakens: its status comes off, it is not summoning sick (it entered the field
/// long ago, or, awakened by its own Activate on the turn it arrived, it is no sicker than one that slept
/// through its countdown), and `awakened` says so, which its own "When this awakens" answers (§6.2). False,
/// changing nothing, for a card that is not Dormant or lies beneath a Stack pile, where it acts for
/// nothing (its countdown waits there, R1839003).
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
    // §4.1: sickness is "entered the field this turn" (`combat::is_sick`), which an awakening lifts.
    if live.summoned_turn == Some(turn) {
        live.summoned_turn = None;
    }
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
/// with a countdown that tops a zone on their side counts one of their turns down, and one whose count
/// reaches 0 awakens there and then, in the order `zones::units_on_field_of` walks them (unit lanes,
/// the Units carriers hold, the Units standing in the backrow). A card beneath a Stack pile does not
/// count (R1839003), and one with no countdown waits for an effect. Nothing happens, and no event is
/// emitted, on a side with no Dormant card.
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

/// §6.2 "When this awakens" (R1839002): the trigger a Dormant card prints, answering the `awakened` of
/// its own awakening and never another card's. The card is awake by then, so it registers the trigger
/// as any card on the field does (`triggers.rs`), in R68's order with the other answers.
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
