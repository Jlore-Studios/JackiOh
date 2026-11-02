//! Destroy and Sacrifice (SPEC §6.3, §4.5). Destroy only marks the card: the next state check moves
//! it, which is where Indestructible, Reborn and the Death order live. Sacrifice is immediate.
//!
//! `destroy_all` and `destroy_adjacent_to` are the same mark over a scope (§3.1, §3.2), so a sweep and
//! a single destroy are collected by the one state check that follows the whole effect (R59).

use serde::{Deserialize, Serialize};

use super::targets::{BoardScope, TargetSpec, adjacent_to_aimed, cards_in_scope_aimed, instance_of};
use crate::damage::already_killed;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::state_check::sacrifice_now;
use crate::wire::TargetAim;
use crate::wire::ZoneName;

/// The mark every destroy leaves (§6.3, §4.5 step 1). A destroy is not a damage instance, so an earlier
/// hit that did not kill must not be credited with this death (R42's lethal source and R89's `killerId`
/// read `lastDamagedBy`); a unit something had already killed (a hit to 0, a Poisonous mark) keeps its
/// killer (`damage::already_killed`, R42).
fn mark_destroyed(state: &mut GameState, card: &CardInstance) {
    let Some(current) = find_instance(state, &card.id) else {
        return;
    };
    // R1081000: a Unit standing in the backrow keeps its killer's credit as one in a unit zone does.
    let killed = crate::zones::in_units_row_or_standing(state, current) && already_killed(state, current);
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DestroyArgs {
    pub target: TargetSpec,
}

/// §6.3 Destroy: mark the card and stop. §4.5 step 1 collects it at the next state check, moves it
/// to its controller's graveyard as theirs (R2618000), fires its Death trigger and lets Indestructible ignore the mark
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

/// §6.3 Destroy, board-wide (#2, #17, #43, #88 Twisting Nether): mark only, so §4.5 step 1 collects
/// the board in ONE state check (R59) and no Death trigger fires while the rest still stand.
/// Indestructible is deliberately NOT filtered out: §4.5's mark resolution drops its mark, puts it in
/// Attack Position and suppresses its Taunt (R46). `rows` defaults to `["units"]` (§6.3).
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DestroyAdjacentToArgs {
    pub target: TargetSpec,
    #[serde(flatten)]
    pub scope: BoardScope,
}

/// §3.1 Adjacent destroy: mark lanes N-1 and N+1 on the target's own side and row, never the target
/// itself nor across sides (#16 Hit Job radiant, which pairs it with a plain `destroy` so all die
/// together under R59). A target off the field, or with no neighbours, fizzles silently.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SacrificeArgs {
    pub target: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_enemy: Option<bool>,
}

/// §6.3 Sacrifice: your own card goes from the field to the graveyard at once, bypassing
/// Indestructible, and counts as a death in full — the destroyed counter (R55), the Death trigger
/// reading the card as it was (R78), §6.1's Reborn — which is §4.5's pass for one card (`sacrifice_now`).
/// A unit token vanishes instead (R11). Tribute may aim it at an enemy unit (#55): `allowEnemy`.
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
