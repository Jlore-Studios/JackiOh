//! Steal (SPEC §6.3): take control of a card on the field. Control is a field-only notion, so the
//! card keeps its owner and still goes to that owner's hand, library, graveyard or exile when it
//! later leaves the field (R12, §3.2). Where it lands is R15, and it keeps its damage, buffs,
//! counters and position because it never leaves the field, which is what R78's reset is about.
//! What a steal does change besides `controller` is R171's: the card has entered its new
//! controller's side on this turn, so it takes the turn as its `summonedTurn` (summoning sick, §4.1)
//! and a fresh exertion. A steal that does nothing (R15, R76) changes neither.
//!
//! Port of `packages/engine/src/effects/steal.ts`.

use serde::{Deserialize, Serialize};

use crate::combat::{enter_new_side, is_active_on_field};
use crate::damage::DamageTarget;
use crate::effects::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance};
use crate::wire::{GameEvent, PlayerId, Row, opponent_of};
use crate::zones::{
    PlaceOnFieldOptions, ZoneSlot, card_at, first_entry_zone, is_open, place_on_field, remove_from_field,
    slot_of, slots_of,
};

/// Which card to steal: the pick the play or a prompt carried (R81), or an instance id a script
/// captured earlier — K-Pop Fanatic's delayed steal names its target that way (R76). Both are plain
/// data, so a card file never holds a closure over state (CLAUDE.md rule 5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StealTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

fn instance_of(ctx: &EffectContext<'_>, args: &StealTarget) -> Option<CardInstance> {
    // R174: a card named by id is aimed at the stay it had when the run began (`instance_on_its_stay`).
    if let Some(instance_id) = &args.instance_id {
        return instance_on_its_stay(ctx, instance_id);
    }
    let spec = args.target.clone().unwrap_or(TargetSpec::Chosen { index: None });
    match resolve_target(ctx, &spec) {
        Some(DamageTarget::Unit { instance }) => Some(instance),
        _ => None,
    }
}

/// R15: the same lane on the stealer's side when that zone is free, else its first free zone.
fn destination_for(ctx: &EffectContext<'_>, thief: PlayerId, from: &ZoneSlot) -> Option<ZoneSlot> {
    let same_lane = ZoneSlot {
        player: thief,
        row: from.row,
        lane: from.lane,
    };
    if is_open(ctx.state, same_lane) {
        return Some(same_lane);
    }
    first_entry_zone(ctx.state, thief, from.row)
}

/// One card to `ctx.controller`'s side. Nothing happens when the card is not on the field (control
/// means nothing off it, R12), when that player already controls it (R76), or when the row has no
/// free zone: then it stays with its owner (R15).
fn take_control(ctx: &mut EffectContext<'_>, card: &CardInstance) -> bool {
    let Some(from) = slot_of(ctx.state, card) else {
        return false;
    };
    // R13: only the top of a Stack pile is on the field. A card dormant under one — #50's chosen
    // permanent after a Stack card was played onto it — is not there to be taken.
    if !is_active_on_field(ctx.state, card) {
        return false;
    }
    if card.controller == ctx.controller {
        return false;
    }
    let previous = card.controller;

    let Some(to) = destination_for(ctx, ctx.controller, &from) else {
        return false;
    };

    let mut moving = card.clone();
    remove_from_field(ctx.state, &moving, Default::default());
    if !place_on_field(ctx.state, &mut moving, to, Default::default()) {
        // `to` was open a line ago and the card came off the other side of the field, so this cannot
        // happen; putting the card back keeps the board legal rather than losing it to a refusal.
        place_on_field(
            ctx.state,
            &mut moving,
            from,
            PlaceOnFieldOptions { stack: Some(true) },
        );
        return false;
    }

    // R171: the card has entered its new controller's side on this turn.
    let placed = find_instance(ctx.state, &moving.id).cloned().unwrap_or(moving);
    enter_new_side(ctx, &placed, previous);

    // R33: a stolen face-down trap stays face-down, and the new controller is the one who may read
    // it — the controller decides that, so `faceUp` is deliberately untouched here.
    ctx.events.push(GameEvent::ControlChanged {
        instance_id: placed.id.clone(),
        controller: to.player,
        row: to.row,
        lane: to.lane,
        former_id: None,
    });
    true
}

/// §6.3 Steal: take control of one card on the field (#36 radiant, #49, #50).
pub fn steal(args: StealTarget) -> Effect {
    Effect::new("steal", move |ctx| {
        let Some(card) = instance_of(ctx, &args) else {
            return;
        };
        take_control(ctx, &card);
    })
}

/// `stealAll`'s arguments: the row defaults to "units".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StealAllArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<Row>,
}

/// Steal every enemy card of a row: lane order (§3.2), each placed per R15, and the ones that find no
/// free zone stay with their owner. Only the top of a Stack pile is on the field, so only it is taken
/// (R13). It was #86 "Miss" Mrow's Death until patch v0.1.1 gave her the unit that destroyed her
/// instead (R361, `steal` of `query::killer_of`); no Core card calls it now, and the verb stays for a
/// card that may.
pub fn steal_all(args: StealAllArgs) -> Effect {
    Effect::new("stealAll", move |ctx| {
        let row = args.row.unwrap_or(Row::Units);
        for slot in slots_of(opponent_of(ctx.controller), row) {
            let Some(card) = card_at(ctx.state, slot).cloned() else {
                continue;
            };
            take_control(ctx, &card);
        }
    })
}
