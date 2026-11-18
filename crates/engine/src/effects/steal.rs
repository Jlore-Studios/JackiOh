//! Steal (SPEC §6.3): take control of a card on the field. Control is a field-only notion, so the
//! card keeps its owner and still goes to that owner's library or exile when it later leaves the field
//! (R12, §3.2); to its controller's hand when bounced (R747) and to its controller's graveyard when it
//! dies, as theirs (R2618000). Where it lands is R15, and it keeps its damage, buffs,
//! counters and position because it never leaves the field, which is what R78's reset is about.
//! What a steal does change besides `controller` is R171's: the card has entered its new
//! controller's side on this turn, so it takes the turn as its `summonedTurn` (summoning sick, §4.1)
//! and a fresh exertion. A steal that does nothing (R13, R76) changes neither; one that reaches the card
//! but has no room for it destroys it where it stands instead, and nothing prevents that destroy
//! (R15, R4205000).

use serde::{Deserialize, Serialize};

use crate::combat::{enter_new_side, is_active_on_field};
use crate::effects::destroy::mark_destroyed_unpreventably;
use crate::effects::radiant::{RadiantTarget, set_radiant};
use crate::effects::targets::{TargetSpec, instance_by_id_or_spec, instance_on_its_stay};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance};
use crate::wire::{ControlHow, GameEvent, PlayerId, Row, opponent_of};
use crate::zones::{
    PlaceOnFieldOptions, ZoneSlot, card_at, first_entry_zone, in_units_row_or_standing, is_open,
    is_standing_unit, place_beneath_top, place_on_field, place_standing_on_field, remove_from_field, slot_of,
    slots_of, standing_units_of,
};

/// Which card to steal: the pick the play or a prompt carried (R81), or an instance id a script
/// captured earlier — K-Pop Fanatic's delayed steal names its target that way (R76). Both are plain
/// data, so a card file never holds a closure over state (CLAUDE.md rule 5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StealTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// R282, R4205000: `steal` makes the card Radiant once it has taken it ("Steal an enemy card. Make it
    /// Radiant.", #49's and M #97.4's Radiant faces); a card it destroyed for want of room, or did not
    /// reach, is made nothing. `give_control` reads none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// What `move_control` did with one card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoveOutcome {
    /// It changed sides.
    Moved,
    /// No steal or give reaches it: off the field, beneath a Stack pile or Dormant (R13, R1839000), or
    /// already the receiver's (R76).
    Unreached,
    /// Reached, but the receiver's side has no destination for it (R15, R3546003).
    NoRoom,
}

/// R15: the same lane on the stealer's side when that zone is free, else its first free zone.
/// R3546003: a GIGA takes the same lane whole when all four zones are free (R64), else the
/// leftmost fit (R64, R688), else nothing (the card is then destroyed by a steal, R4205000).
fn destination_for(
    ctx: &EffectContext<'_>,
    thief: PlayerId,
    card: &CardInstance,
    from: &ZoneSlot,
) -> Option<ZoneSlot> {
    let same_lane = ZoneSlot {
        player: thief,
        row: from.row,
        lane: from.lane,
    };
    if crate::giga::is_giga(card) {
        if crate::giga::fits(same_lane, |slot| is_open(ctx.state, slot)) {
            return Some(same_lane);
        }
        return crate::giga::entry_anchor(ctx.state, thief, from.row);
    }
    if is_open(ctx.state, same_lane) {
        return Some(same_lane);
    }
    first_entry_zone(ctx.state, thief, from.row)
}

/// One card to `ctx.controller`'s side, by `move_control`'s rules; a card it reaches but has no room for
/// is destroyed where it stands, and nothing prevents that destroy (R4205000): no `controlChanged`, no
/// R171 entry, no R1854002 quest step, and R59's state check collects it with the rest.
fn take_control(ctx: &mut EffectContext<'_>, card: &CardInstance) -> MoveOutcome {
    let receiver = ctx.controller;
    let moved = move_control(ctx, card, receiver, ControlHow::Steal);
    if moved == MoveOutcome::NoRoom {
        mark_destroyed_unpreventably(&mut *ctx.state, card);
    }
    moved
}

/// One card to `receiver`'s side: R15's zone, R171's entry and `controlChanged` (`how`, R1366: a steal
/// or a give). Nothing moves when the card is off the field, dormant (R13), already `receiver`'s (R76),
/// or finds no free zone (R15), which the caller tells apart: a steal destroys a card it had no room for
/// (R4205000), a give leaves it (R803).
fn move_control(
    ctx: &mut EffectContext<'_>,
    card: &CardInstance,
    receiver: PlayerId,
    how: ControlHow,
) -> MoveOutcome {
    let Some(from) = slot_of(ctx.state, card) else {
        return MoveOutcome::Unreached;
    };
    // R13: only the top of a Stack pile is on the field; a card dormant under one (#50's) is not taken.
    if !is_active_on_field(ctx.state, card) {
        return MoveOutcome::Unreached;
    }
    if card.controller == receiver {
        return MoveOutcome::Unreached;
    }
    let previous = card.controller;

    let Some(to) = destination_for(ctx, receiver, card, &from) else {
        return MoveOutcome::NoRoom;
    };

    let mut moving = card.clone();
    // R1081000: a Unit standing in a backrow zone is stolen standing.
    let stands = crate::zones::stands_in_backrow(ctx.state, card);
    remove_from_field(ctx.state, &moving, Default::default());
    let landed = if stands {
        place_standing_on_field(ctx.state, &mut moving, to, Default::default())
    } else {
        place_on_field(ctx.state, &mut moving, to, Default::default())
    };
    if !landed {
        // Cannot happen (`to` was open); putting the card back keeps the board legal, not losing it.
        let options = PlaceOnFieldOptions { stack: Some(true) };
        if stands {
            place_standing_on_field(ctx.state, &mut moving, from, options);
        } else {
            place_on_field(ctx.state, &mut moving, from, options);
        }
        return MoveOutcome::Unreached;
    }

    // R171: the card has entered its new controller's side on this turn.
    let placed = find_instance(ctx.state, &moving.id).cloned().unwrap_or(moving);
    enter_new_side(ctx, &placed, previous);

    // R33: a stolen face-down trap stays face-down, readable by its new controller: `faceUp` is untouched.
    ctx.events.push(GameEvent::ControlChanged {
        instance_id: placed.id.clone(),
        controller: to.player,
        row: to.row,
        lane: to.lane,
        former_id: None,
        how: Some(how),
    });
    // R1854002: a steal moves its thief's "steal an enemy card" quests; a give is no steal.
    if how == ControlHow::Steal {
        crate::subsystems::quests::note_steal(ctx, to.player);
    }
    MoveOutcome::Moved
}

/// §6.3 Steal: take control of one card on the field (#36 radiant, #49, #50), or destroy it where it
/// stands when the taker's side has no room for it (R4205000; R963's Allure); with `radiant`, then make
/// the card it took Radiant (R282).
pub fn steal(args: StealTarget) -> Effect {
    Effect::new("steal", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, args.instance_id.as_deref(), args.target.as_ref())
        else {
            return;
        };
        if take_control(ctx, &card) == MoveOutcome::Moved && args.radiant == Some(true) {
            let target = RadiantTarget {
                instance_id: Some(card.id.clone()),
                target: None,
            };
            (set_radiant(target).apply)(ctx);
        }
    })
}

/// R1423: give a card on the field to its controller's opponent, the mirror of `steal`, with the same
/// placing and no-ops (R13, R15, R171; Meditative #6, #30), except that a card the receiver has no room
/// for stays where it is (R803), where a steal destroys it (R4205000).
pub fn give_control(args: StealTarget) -> Effect {
    Effect::new("giveControl", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, args.instance_id.as_deref(), args.target.as_ref())
        else {
            return;
        };
        let receiver = opponent_of(card.controller);
        move_control(ctx, &card, receiver, ControlHow::Give);
    })
}

/// `stealAll`'s arguments: the row defaults to "units".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StealAllArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<Row>,
}

/// Steal every enemy card of a row in lane order (§3.2), each per R15: what fits is taken and the rest
/// destroyed, in that order (R68, R4205000); only a Stack pile's top is reached (R13). No Core card
/// calls it (#86 steals `query::killer_of`, R361); it stays for a card that may.
pub fn steal_all(args: StealAllArgs) -> Effect {
    Effect::new("stealAll", move |ctx| {
        let row = args.row.unwrap_or(Row::Units);
        let enemy = opponent_of(ctx.controller);
        // R1081000: a Unit standing in a backrow zone is the units row's, after its zones (§6.1 Animated).
        let mut cards: Vec<CardInstance> = slots_of(enemy, row)
            .into_iter()
            .filter_map(|slot| card_at(ctx.state, slot))
            .filter(|card| !(row == Row::Backrow && is_standing_unit(ctx.state, card)))
            .cloned()
            .collect();
        if row == Row::Units {
            cards.extend(standing_units_of(ctx.state, enemy).into_iter().cloned());
        }
        for card in cards {
            take_control(ctx, &card);
        }
    })
}

/// MD-F16, R1284: capture an opponent's card, moving it dormant beneath the holder under the holder's controller.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CaptureArgs {
    pub instance_id: String,
}

pub fn capture(args: CaptureArgs) -> Effect {
    Effect::new("capture", move |ctx| {
        let Some(holder_live) = ctx.self_.as_ref().and_then(|c| find_instance(ctx.state, &c.id)) else {
            return;
        };
        let Some(holder_slot) = slot_of(ctx.state, holder_live) else {
            return;
        };
        // R1081000: a captor standing in a backrow zone captures as one in a unit zone does.
        if !in_units_row_or_standing(ctx.state, holder_live) || !is_active_on_field(ctx.state, holder_live) {
            return;
        }
        let Some(target_card) = instance_on_its_stay(ctx, &args.instance_id) else {
            return;
        };
        // R1081000: likewise a target standing in the backrow.
        if !in_units_row_or_standing(ctx.state, &target_card) || !is_active_on_field(ctx.state, &target_card)
        {
            return;
        }
        if target_card.id == holder_live.id || target_card.controller == holder_live.controller {
            return;
        }
        // R3546000: nothing captures with a GIGA, neither as captor nor as captive.
        if crate::giga::is_giga(holder_live) || crate::giga::is_giga(&target_card) {
            return;
        }

        let previous = target_card.controller;
        let mut moving = target_card.clone();
        remove_from_field(ctx.state, &moving, Default::default());
        if !place_beneath_top(ctx.state, &mut moving, holder_slot) {
            return;
        }

        let placed = find_instance(ctx.state, &moving.id).cloned().unwrap_or(moving);
        enter_new_side(ctx, &placed, previous);

        ctx.events.push(GameEvent::ControlChanged {
            instance_id: placed.id.clone(),
            controller: holder_slot.player,
            row: holder_slot.row,
            lane: holder_slot.lane,
            former_id: None,
            how: Some(ControlHow::Steal),
        });
        // R1854002: a capture is a steal, for the quests.
        crate::subsystems::quests::note_steal(ctx, holder_slot.player);
    })
}
