//! Unit statuses an effect sets (docs/classic-sets.md B5 E35): Berserk, and "may attack again".
//!
//! A status is not text, so neither is a keyword: a Vanilla keeps it, and R78's reset takes it off
//! with the unit leaving the field. What a Berserk unit does about it — Classic+ #19.5's forced
//! attacks on its own hero at the start and end of its controller's turn — is its own card's text,
//! written with `forced_attack_own_hero` and read through `is_berserk` (`restrictions.rs`).
//!
//! Port of `packages/engine/src/effects/statuses.ts`.

use serde::{Deserialize, Serialize};

use crate::config::BERSERK_MARK;
use crate::restrictions::can_go_berserk;
use crate::script::Effect;
use crate::state::{Exertion, find_instance_mut};
use crate::wire::{GameEvent, Row, Zone};

use super::targets::{TargetSpec, instance_of};

/// `goBerserk`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoBerserkArgs {
    pub target: TargetSpec,
}

/// B5 E35: the unit goes Berserk (Classic+ #19.2's reward for Classic+ #19.5). Nothing happens to a
/// unit that is Berserk already, is not on the field, or "can't go Berserk" (`neverBerserk`, #19.5's
/// Radiant face). Both views show it (`UnitView.berserk`), and R437's reusable mark announces it:
/// `marked`, "berserk", red.
pub fn go_berserk(args: GoBerserkArgs) -> Effect {
    Effect::new("goBerserk", move |ctx| {
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };
        if !can_go_berserk(ctx.state, &unit) {
            return;
        }
        if let Some(live) = find_instance_mut(ctx.state, &unit.id) {
            live.berserk = Some(true);
        }
        ctx.events.push(GameEvent::Marked {
            instance_id: unit.id.clone(),
            mark: BERSERK_MARK.mark.to_string(),
            color: BERSERK_MARK.color.to_string(),
            added: true,
        });
    })
}

/// `mayAttackAgain`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MayAttackAgainArgs {
    pub target: TargetSpec,
}

/// B5 E35: the unit may attack again this turn (Classic+ #73.1 Classic Golem's "after it destroys a
/// Unit"): a fresh exertion — both halves, §4.1 — and no summoning sickness for the rest of this turn,
/// so a unit that has just entered the field may attack at once. It does not grant an attack a unit
/// could not otherwise make: Attack Position, attack above 0 and the unit restrictions still apply.
pub fn may_attack_again(args: MayAttackAgainArgs) -> Effect {
    Effect::new("mayAttackAgain", move |ctx| {
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };
        if !matches!(unit.zone, Zone::Field { row: Row::Units, .. }) {
            return;
        }
        let turn = ctx.state.turn;
        if let Some(live) = find_instance_mut(ctx.state, &unit.id) {
            live.exertion = Exertion {
                attacked: false,
                switched: false,
                attacks: None,
            };
            // §4.1: sickness is "entered the field this turn" (`combat::is_sick`), which this lifts for the turn.
            if live.summoned_turn == Some(turn) {
                live.summoned_turn = None;
            }
        }
    })
}
