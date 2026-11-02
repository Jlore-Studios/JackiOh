//! Unit statuses an effect sets (docs/classic-sets.md B5 E35): Berserk, "may attack again", and
//! Frozen (§6.3 Freeze, R1174000) — and the one an effect takes off, Dormant (§6.3 Awaken, R1839002).
//!
//! A status is not text, so a Vanilla keeps Berserk (Frozen it removes, as Hearthstone's Silence does,
//! R1174000), and R78's reset takes either off as the unit leaves the field. Classic+ #19.5's forced
//! attacks are its own card's text (`forced_attack_own_hero`, `is_berserk` in `restrictions.rs`).

use serde::{Deserialize, Serialize};

use crate::config::BERSERK_MARK;
use crate::restrictions::can_go_berserk;
use crate::script::Effect;
use crate::state::{Exertion, find_instance_mut};
use crate::wire::GameEvent;

use crate::wire::TargetAim;

use super::targets::{BoardScope, TargetSpec, cards_in_scope_aimed, instance_of};

/// `goBerserk`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GoBerserkArgs {
    pub target: TargetSpec,
}

/// B5 E35: the unit goes Berserk (Classic+ #19.2's reward), unless it is already, is off the field or
/// "can't go Berserk" (`neverBerserk`). R437's reusable mark announces it.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MayAttackAgainArgs {
    pub target: TargetSpec,
    /// R2521002 (Stinkyscape #94 Tmack Trav): the fresh exertion alone — a unit still summoning sick
    /// stays sick, so Rush keeps it off the hero on its summon turn. Every refresh does so since
    /// R2657004, so this says what the verb does anyway. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_sick: Option<bool>,
    /// R767000: the sickness lifted for the rest of the turn, as every refresh did before R2657004 —
    /// only the scripts kept for those versions' replays (Classic+ #73.1 Classic Golem's v0.5.9f) say
    /// so. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lift_sick: Option<bool>,
}

/// B5 E35: the unit may attack again this turn (Classic+ #73.1 Classic Golem's "after it destroys a
/// Unit"): a fresh exertion — both halves, §4.1. It grants no attack a unit could not otherwise make:
/// Attack Position, attack above 0 and the unit restrictions still apply. R2657004: summoning sickness
/// lasts the whole turn a unit entered the field, whatever attacks an effect gives it, so a unit that
/// entered this turn attacks again as its keywords let it: a Rush unit a Unit only (§6.1), never the hero
/// (R2521002, Tmack Trav). `lift_sick` is for kept scripts alone.
pub fn may_attack_again(args: MayAttackAgainArgs) -> Effect {
    Effect::new("mayAttackAgain", move |ctx| {
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };
        // R1081000: a Unit standing in the backrow attacks, so it may attack again.
        if !crate::zones::in_units_row_or_standing(ctx.state, &unit) {
            return;
        }
        let turn = ctx.state.turn;
        if let Some(live) = find_instance_mut(ctx.state, &unit.id) {
            live.exertion = Exertion {
                attacked: false,
                switched: false,
                attacks: None,
            };
            // §4.1: sickness is "entered the field this turn" (`combat::is_sick`), which a refresh leaves
            // (R2657004); a script kept from before lifts it for the turn (R767000).
            if args.lift_sick == Some(true)
                && args.keep_sick != Some(true)
                && live.summoned_turn == Some(turn)
            {
                live.summoned_turn = None;
            }
        }
    })
}

/// `freeze`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FreezeArgs {
    pub target: TargetSpec,
    /// R1848000 (Stinkyscape #2 Absolute Zero): "Freeze a Unit forever" — the freeze never thaws
    /// (`freeze::freeze_unit_forever`). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forever: Option<bool>,
}

/// R1174000: a Unit in a unit zone, standing in the backrow (R1081000) or carried (R446).
fn freezable(state: &crate::state::GameState, card: &crate::state::CardInstance) -> bool {
    crate::zones::in_units_row_or_standing(state, card) || crate::zones::is_carried(state, card)
}

/// §6.3 Freeze (R1174000): `target`'s Unit loses its next attack or position switch. A Unit Immune to
/// Spells is passed by (`instance_of`); an Immutable one is not, since a status is not text.
/// R1848000: with `forever` it never thaws.
pub fn freeze(args: FreezeArgs) -> Effect {
    Effect::new("freeze", move |ctx| {
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };
        if !freezable(ctx.state, &unit) {
            return;
        }
        if args.forever == Some(true) {
            crate::freeze::freeze_unit_forever(&mut ctx.sink, &unit.id);
        } else {
            crate::freeze::freeze_unit(&mut ctx.sink, &unit.id);
        }
    })
}

/// §6.3 Freeze over a scope (R1174000; M+ #24 Bob), a harmful walk (R940).
pub fn freeze_all(args: BoardScope) -> Effect {
    Effect::new("freezeAll", move |ctx| {
        for unit in cards_in_scope_aimed(ctx, &args, TargetAim::Harm) {
            if freezable(ctx.state, &unit) {
                crate::freeze::freeze_unit(&mut ctx.sink, &unit.id);
            }
        }
    })
}

/// `awaken`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AwakenArgs {
    pub target: TargetSpec,
}

/// §6.3 Awaken (R1839002): the Dormant card `target` names awakens now (`slumber::awaken_card`). The
/// one verb that reaches a Dormant card, so it skips `instance_of`, which passes one by (R1839000).
/// Nothing happens to a card beneath a Stack pile (R1839003).
pub fn awaken(args: AwakenArgs) -> Effect {
    Effect::new("awaken", move |ctx| {
        let id = match &args.target {
            TargetSpec::SelfCard => ctx.live_self().map(|card| card.id.clone()),
            TargetSpec::Instance { instance_id } => Some(instance_id.clone()),
            TargetSpec::Chosen { index } => match ctx.targets.get(index.unwrap_or(0)) {
                Some(crate::wire::Selection::Instance { instance_id }) => Some(instance_id.clone()),
                _ => None,
            },
            TargetSpec::SelfHero | TargetSpec::EnemyHero => None,
        };
        if let Some(id) = id {
            crate::slumber::awaken_card(&mut ctx.sink, &id);
        }
    })
}
