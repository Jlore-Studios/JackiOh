//! Counters on an instance and locks on a zone (§6.3). Plague Counters live on the instance and R78
//! clears them when the card leaves the field; a Lock lives on the zone and outlives every occupant.
//! The Plague Counter rules themselves — what a placement is, the multiplier, the report — are
//! `crate::plague`'s (R471); the placement verbs of patch v0.2.0 are `super::plague`'s.
//!
//! Port of `packages/engine/src/effects/counters.ts`.

use serde::{Deserialize, Serialize};

use super::targets::{PlayerSpec, TargetSpec, player_of, resolve_target};
use crate::damage::DamageTarget;
use crate::plague::{place_plague_on, plague_on, remove_plague};
use crate::script::{Effect, EffectContext};
use crate::state::CardInstance;
use crate::wire::{GameEvent, Row};
use crate::zones::{ZoneSlot, is_locked, lock_zone, row_size, slot_of, unlock_zone};

fn instance_of(ctx: &EffectContext<'_>, spec: &TargetSpec) -> Option<CardInstance> {
    match resolve_target(ctx, spec) {
        Some(DamageTarget::Unit { instance }) => Some(instance),
        _ => None,
    }
}

/// `plague`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlagueArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    pub amount: i32,
}

/// #91 Fed Fauci: add Plague Counters to a permanent, any number of them — one placement (R471, so a
/// card that multiplies what is placed on it multiplies this, and "whenever Plague Counters are placed
/// on this" answers it). A negative amount takes them off and the count floors at 0; R78 resets the
/// counter when the card leaves the field.
pub fn plague(args: PlagueArgs) -> Effect {
    Effect::new("plague", move |ctx| {
        let this = TargetSpec::SelfCard;
        let Some(card) = instance_of(ctx, args.target.as_ref().unwrap_or(&this)) else {
            return;
        };
        let amount = args.amount;
        if amount > 0 {
            place_plague_on(ctx, &card, amount);
        } else {
            remove_plague(ctx, &card, -amount);
        }
    })
}

/// `clear_plague`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClearPlagueArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// Clear every Plague Counter on a permanent.
pub fn clear_plague(args: ClearPlagueArgs) -> Effect {
    Effect::new("clearPlague", move |ctx| {
        let this = TargetSpec::SelfCard;
        let Some(card) = instance_of(ctx, args.target.as_ref().unwrap_or(&this)) else {
            return;
        };
        let on = plague_on(&card);
        remove_plague(ctx, &card, on);
    })
}

/// Which zone a Lock names: the one this card sits in, the one a named card sits in (#36 Magic
/// Jammed locks its target's zone, so the lock effect runs before the destroy that empties it), or
/// a lane by index (§3.1 "this lane").
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "of", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ZoneSpec {
    #[serde(rename = "self")]
    SelfCard,
    Chosen {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    Lane {
        row: Row,
        lane: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        player: Option<PlayerSpec>,
    },
}

fn zone_for(ctx: &EffectContext<'_>, spec: &ZoneSpec) -> Option<ZoneSlot> {
    match spec {
        ZoneSpec::Lane { row, lane, player } => {
            if *lane < 1 || *lane > row_size(*row) {
                return None;
            }
            Some(ZoneSlot {
                player: player_of(ctx, player.unwrap_or(PlayerSpec::SelfSide)),
                row: *row,
                lane: *lane,
            })
        }
        // TS read the live `ctx.self` object: the card where it stands now.
        ZoneSpec::SelfCard => ctx.live_self().and_then(|card| slot_of(ctx.state, card)),
        ZoneSpec::Chosen { index } => {
            let card = instance_of(ctx, &TargetSpec::Chosen { index: *index })?;
            slot_of(ctx.state, &card)
        }
    }
}

/// `lock`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LockArgs {
    pub zone: ZoneSpec,
}

/// §3.2 Lock: the zone accepts no summons until something unlocks it. The current occupant is
/// unaffected and the lock persists after it leaves. Nothing in Core unlocks a zone; B5 E20's Unlock
/// (`unlock` below, `effects::locks::unlock_all`) does.
pub fn lock(args: LockArgs) -> Effect {
    Effect::new("lock", move |ctx| {
        let Some(slot) = zone_for(ctx, &args.zone) else {
            return;
        };
        if is_locked(ctx.state, &slot) {
            return;
        }
        lock_zone(&mut *ctx.state, &slot);
        ctx.events.push(GameEvent::Locked {
            player: slot.player,
            row: slot.row,
            lane: slot.lane,
        });
    })
}

/// `unlock`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnlockArgs {
    pub zone: ZoneSpec,
}

/// B5 E20: a Locked zone accepts summons again (event `unlocked`). A zone that is not Locked is left
/// as it is, with no event. Its occupant is unaffected, and a card whose return a Lock stopped (an
/// animated card's home, B3.1 rule 6) goes back at its next chance.
pub fn unlock(args: UnlockArgs) -> Effect {
    Effect::new("unlock", move |ctx| {
        let Some(slot) = zone_for(ctx, &args.zone) else {
            return;
        };
        if !is_locked(ctx.state, &slot) {
            return;
        }
        unlock_zone(&mut *ctx.state, &slot);
        ctx.events.push(GameEvent::Unlocked {
            player: slot.player,
            row: slot.row,
            lane: slot.lane,
        });
    })
}
