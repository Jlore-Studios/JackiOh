//! Destroy and Sacrifice (SPEC §6.3, §4.5). Destroy only marks the card: the next state check moves
//! it, which is where Indestructible, Reborn and the Death order live. Sacrifice is immediate.
//!
//! `destroy_all` and `destroy_adjacent_to` are the same mark over a scope (§3.1, §3.2), so a sweep and
//! a single destroy are collected by the one state check that follows the whole effect (R59).
//!
//! Port of `packages/engine/src/effects/destroy.ts`.

use serde::{Deserialize, Serialize};

use super::targets::{BoardScope, TargetSpec, adjacent_to_aimed, cards_in_scope_aimed, instance_of};
use crate::damage::already_killed;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::state_check::sacrifice_now;
use crate::wire::TargetAim;
use crate::wire::{Row, Zone, ZoneName};

/// The mark every destroy leaves (§6.3, §4.5 step 1). A destroy is not a damage instance, so no
/// unit's hit can be the lethal one any more: R42's "a death whose lethal damage instance came from
/// this unit" and R89's `killerId` read `lastDamagedBy`, and a hit that landed earlier and did not
/// kill must not be credited with a death this effect caused. A unit something had already killed
/// before the destroy landed — a hit took it to 0 or less, a Poisonous hit marked it — was killed by
/// that, and a destroy on a dead unit changes nothing (`damage::already_killed`, R42). Poisonous marks
/// inside the damage instance itself (`damage.rs` step 7) and so keeps its source.
///
/// TS wrote through the live card object; here the card is found again by its id and marked where it
/// stands, read as it stands now.
fn mark_destroyed(state: &mut GameState, card: &CardInstance) {
    let Some(current) = find_instance(state, &card.id) else {
        return;
    };
    let killed =
        matches!(current.zone, Zone::Field { row: Row::Units, .. }) && already_killed(state, current);
    let Some(live) = find_instance_mut(state, &card.id) else {
        return;
    };
    live.marked_destroyed = Some(true);
    if !killed {
        live.last_damaged_by = None;
    }
}

/// `destroy`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DestroyArgs {
    pub target: TargetSpec,
}

/// §6.3 Destroy: mark the card and stop. §4.5 step 1 collects it at the next state check, moves it
/// to its owner's graveyard (R12), fires its Death trigger and lets Indestructible ignore the mark
/// (R46). Nothing here moves a card, so several destroys in one effect die together (R59).
pub fn destroy(args: DestroyArgs) -> Effect {
    Effect::new("destroy", move |ctx| {
        let Some(card) = instance_of(ctx, &args.target) else {
            return;
        };
        if card.zone.z() != ZoneName::Field {
            return;
        }
        mark_destroyed(&mut *ctx.state, &card);
    })
}

/// §6.3 Destroy, board-wide: mark every card the scope names and stop (#2 Bigot, #17 Flood,
/// #43 Big Felinor, #88 Twisting Nether). Marking and only marking is the whole point — §4.5 step 1
/// then collects the entire board in ONE state check, which is what R59 requires of a sweep: no
/// Death trigger of the first victim fires while the rest are still standing.
///
/// Indestructible is deliberately NOT filtered out of the scope. "Indestructibles survive" is a
/// consequence of §4.5's mark resolution, not of this walk: that step drops the mark, switches the
/// unit to Attack Position and suppresses its Taunt for the turn (R46). A scope that skipped
/// Indestructible units would quietly lose all three, leaving a warded blocker in Defense Position
/// after a Twisting Nether that R46 says should have been knocked flat.
///
/// `rows` defaults to `["units"]`; a sweep over permanents passes `["units", "backrow"]` (§6.3).
pub fn destroy_all(args: BoardScope) -> Effect {
    Effect::new("destroyAll", move |ctx| {
        // MD-B1, R940: a harmful walk — an immune card in a tribal scope is passed by.
        for card in cards_in_scope_aimed(ctx, &args, TargetAim::Harm) {
            mark_destroyed(&mut *ctx.state, &card);
        }
    })
}

/// TS `{ target: TargetSpec } & BoardScope`: `destroy_adjacent_to`'s arguments, the scope's fields
/// beside `target`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DestroyAdjacentToArgs {
    pub target: TargetSpec,
    #[serde(flatten)]
    pub scope: BoardScope,
}

/// §3.1 Adjacent destroy: mark lanes N-1 and N+1 on the target's own side and row, never the target
/// itself and never across sides (#16 Hit Job radiant). The card pairs this with a plain
/// `destroy({ target: { of: "chosen" } })` in the same effect list, so the target and its
/// neighbours are all marked before the state check and die together under R59.
///
/// A target off the field, or one with no neighbours, fizzles silently and the card still resolves.
pub fn destroy_adjacent_to(args: DestroyAdjacentToArgs) -> Effect {
    Effect::new("destroyAdjacentTo", move |ctx| {
        let DestroyAdjacentToArgs { target, scope } = &args;
        for card in adjacent_to_aimed(ctx, target, scope, TargetAim::Harm) {
            mark_destroyed(&mut *ctx.state, &card);
        }
    })
}

/// `sacrifice`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SacrificeArgs {
    pub target: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_enemy: Option<bool>,
}

/// §6.3 Sacrifice: your own card goes from the field to the graveyard at once, bypassing
/// Indestructible, and it counts as a death — the destroyed counter (R55), the `destroyed` event,
/// the Death trigger, which reads the card as it was just before it left (R78), and §6.1's Reborn,
/// which brings a sacrificed Reborn unit back to its zone at 1 health as it would any other first
/// death. That is §4.5's pass for one card, so it is `state_check::sacrifice_now` rather than a copy
/// of it. A unit token vanishes instead of entering a graveyard (R11). Tribute may aim it at an enemy
/// unit (#55), which `allowEnemy` says.
pub fn sacrifice(args: SacrificeArgs) -> Effect {
    Effect::new("sacrifice", move |ctx| {
        let Some(card) = instance_of(ctx, &args.target) else {
            return;
        };
        if card.zone.z() != ZoneName::Field {
            return;
        }
        if card.controller != ctx.controller && args.allow_enemy != Some(true) {
            return;
        }
        sacrifice_now(ctx, &card);
    })
}
