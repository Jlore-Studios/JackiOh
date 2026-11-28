//! Destroy and Sacrifice (SPEC §6.3, §4.5). Destroy only marks the card: the next state check moves
//! it, which is where Indestructible, Reborn and the Death order live. Sacrifice is immediate.
//!
//! `destroy_all` and `destroy_adjacent_to` are the same mark over a scope (§3.1, §3.2), so a sweep and
//! a single destroy are collected by the one state check that follows the whole effect (R59).

use serde::{Deserialize, Serialize};

use super::targets::{
    BoardScope, PlayerSpec, TargetSpec, adjacent_to_aimed, cards_in_scope_aimed, instance_of, player_of,
};
use crate::damage::already_killed;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::state_check::sacrifice_now;
use crate::wire::{Row, TargetAim, ZoneName};
use crate::zones::ZoneSlot;

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

/// R4205000: the mark a steal with no room for the card leaves (§6.3 Steal). The same mark as any
/// destroy, so R59's one state check collects it in R68's order, but one nothing prevents: §4.5 drops no
/// Indestructible mark of it (R46) and offers it no "would die" replacement; a destroy-to-exile aura
/// still exiles it (R1849002).
pub(crate) fn mark_destroyed_unpreventably(state: &mut GameState, card: &CardInstance) {
    mark_destroyed(state, card);
    if let Some(live) = find_instance_mut(state, &card.id) {
        live.unpreventable_destroy = Some(true);
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
        // R4586004: a warded face-down card is passed over.
        if crate::face_down::face_down_warded(ctx.state, &card, ctx.controller) {
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
            // R4586004: a warded face-down card is passed over.
            if crate::face_down::face_down_warded(ctx.state, &card, ctx.controller) {
                continue;
            }
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

/// `crush`'s arguments: the zone whose Units it destroys, its side relative to the controller.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CrushArgs {
    pub player: PlayerSpec,
    /// The zone's row; absent is the unit row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<Row>,
    pub lane: i32,
    /// R5238003: only the Units lying beneath the running card in that zone's pile, never the running
    /// card itself nor one above it; none when the running card is not in the zone. Absent: every Unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beneath_only: Option<bool>,
}

/// R5238003: the ids beneath the running card in `slot`, in the zone's order (`zones::zone_contents`, a
/// pile's top first): every card after it; none when it is not in the zone.
fn beneath_running(ctx: &crate::script::EffectContext<'_>, slot: ZoneSlot) -> Vec<String> {
    let Some(me) = ctx.self_.as_ref().map(|card| card.id.clone()) else {
        return Vec::new();
    };
    let contents = crate::zones::zone_contents(&*ctx.state, slot);
    match contents.iter().position(|card| card.id == me) {
        Some(at) => contents[at + 1..].iter().map(|card| card.id.clone()).collect(),
        None => Vec::new(),
    }
}

/// R5048000: the Units a crush of `slot` destroys: the zone's cards that are Units (a pile, a Unit
/// standing in a backrow zone; R4665000: a card lying on a unit zone is none), else the GIGA Unit
/// covering it. A set Trap, a Field Spell or any other card that is no Unit is not among them.
pub fn crushed_in(state: &GameState, slot: ZoneSlot) -> Vec<CardInstance> {
    let mut below = crate::zones::zone_contents(state, slot);
    if below.is_empty()
        && let Some(giga) = crate::giga::covering(state, slot)
    {
        below.push(giga.clone());
    }
    below
        .into_iter()
        .filter(|card| crate::zones::in_units_row_or_standing(state, card))
        .collect()
}

/// R5048000 Crush (S+ #121 Toxopid's Legs): destroy every Unit in the named zone (`crushed_in`), as
/// §6.3 Destroy does: a mark the next state check collects (R59), Indestructible ignoring it (R46), so
/// a card summoned there once the check has run finds the zone free. No keyword: a card's text calls it.
/// R5117000: a Spell's crush passes a Unit Immune to Spells by (R4453000), which keeps its zone.
pub fn crush(args: CrushArgs) -> Effect {
    Effect::new("crush", move |ctx| {
        let slot = ZoneSlot {
            player: player_of(ctx, args.player),
            row: args.row.unwrap_or(Row::Units),
            lane: args.lane,
        };
        // R5238003: with `beneath_only`, the running card and whatever lies above it stand.
        let beneath: Option<Vec<String>> =
            (args.beneath_only == Some(true)).then(|| beneath_running(ctx, slot));
        for card in crushed_in(ctx.state, slot) {
            if beneath.as_ref().is_some_and(|ids| !ids.contains(&card.id)) {
                continue;
            }
            if crate::restrictions::unaffected_by(ctx, &card) {
                continue;
            }
            if beneath.is_some() {
                crush_dormant(ctx, card);
                continue;
            }
            mark_destroyed(&mut *ctx.state, &card);
        }
    })
}

/// R5238003: a card beneath the running card lies dormant there, off the field for §4.5 (R13), so no
/// state check would collect a mark on it while the running card stands on top: it goes to its owner's
/// graveyard at once, by `send_to_graveyard`'s plain move, its text dormant so no Death runs. An
/// Indestructible one stays (R46).
fn crush_dormant(ctx: &mut crate::script::EffectContext<'_>, mut card: CardInstance) {
    if crate::layers::unit_has(ctx.state, &card, crate::wire::KeywordKind::Indestructible) {
        return;
    }
    let moved = crate::zones::move_to_zone(
        ctx.state,
        &mut card,
        crate::zones::OffFieldZone::Graveyard,
        Default::default(),
    );
    let landed = find_instance(ctx.state, &card.id).cloned().unwrap_or(card);
    crate::zones::report_graveyard_landing(ctx, &landed, moved);
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
