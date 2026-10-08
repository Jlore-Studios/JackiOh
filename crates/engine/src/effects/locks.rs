//! The Lock variants and Unlock (docs/classic-sets.md B5 E20; SPEC §3.2 Lock). A Lock lives on the zone
//! and outlives every occupant: it evicts nothing, and the zone takes no summon, play or return until an
//! Unlock opens it again. `effects/counters.rs` keeps the single `lock` and `unlock`; these are the
//! forms the new cards name — a whole lane (Classic #71 Lane Eater), the zone a permanent was just
//! played into (Classic #84 Lockdown, Classic+ #34 Memory Leak), a random zone not already Locked
//! (Classic+ #34), the firing trap's own zone (Classic+ #1 Doom Shroom) and every zone (Classic+ #77
//! Anti-Softlock), and a random Locked zone to Unlock (Meditative #27 Clip-Farming Lawyer).
//!
//! Port of `packages/engine/src/effects/locks.ts`.

use serde::{Deserialize, Serialize};

use crate::script::{Effect, EffectContext};
use crate::state::find_instance;
use crate::stays::{event_mark, left_field_after};
use crate::wire::{GameEvent, Row, ZoneName};
use crate::work::EVENT_KEY;
use crate::zones::{ZoneSlot, is_locked, lock_zone, row_size, slot_of, slots_of, unlock_zone};

use super::targets::{ScopeSide, TargetSpec, instance_of, sides_of};

/// Which sides and rows a zone-wide Lock or Unlock covers: both sides and both rows by default.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoneScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<ScopeSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Row>>,
}

const BOTH_ROWS: &[Row] = &[Row::Units, Row::Backrow];

/// The zones a scope covers, in R68's walk: side by side, units then backrow, lane 1 upward.
fn zones_in_scope(ctx: &EffectContext<'_>, scope: &ZoneScope) -> Vec<ZoneSlot> {
    let rows: Vec<Row> = scope.rows.clone().unwrap_or_else(|| BOTH_ROWS.to_vec());
    sides_of(ctx, scope.side)
        .into_iter()
        .flat_map(|player| {
            rows.iter()
                .flat_map(move |row| slots_of(player, *row))
                .collect::<Vec<ZoneSlot>>()
        })
        .collect()
}

fn lock_one(ctx: &mut EffectContext<'_>, slot: &ZoneSlot) {
    if is_locked(ctx.state, slot) {
        return;
    }
    lock_zone(ctx.state, slot);
    ctx.events.push(GameEvent::Locked {
        player: slot.player,
        row: slot.row,
        lane: slot.lane,
    });
}

fn unlock_one(ctx: &mut EffectContext<'_>, slot: &ZoneSlot) {
    if !is_locked(ctx.state, slot) {
        return;
    }
    unlock_zone(ctx.state, slot);
    ctx.events.push(GameEvent::Unlocked {
        player: slot.player,
        row: slot.row,
        lane: slot.lane,
    });
}

/// Which lane: the one a card stands in (`self` by default, or a named card), or a number
/// (TS `number | TargetSpec`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum LaneSpec {
    Lane(i32),
    Card(TargetSpec),
}

fn lane_of(ctx: &EffectContext<'_>, spec: &LaneSpec) -> Option<i32> {
    let card = match spec {
        LaneSpec::Lane(lane) => {
            return if *lane >= 1 && *lane <= row_size(Row::Units) {
                Some(*lane)
            } else {
                None
            };
        }
        // TS read the live `ctx.self`: the card as it stands now.
        LaneSpec::Card(TargetSpec::SelfCard) => ctx.live_self().cloned(),
        LaneSpec::Card(other) => instance_of(ctx, other),
    }?;
    slot_of(ctx.state, &card).map(|at| at.lane)
}

/// `lockLane`'s argument: `{ lane? } & ZoneScope` (TS default `{}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LockLaneArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<LaneSpec>,
    #[serde(flatten)]
    pub scope: ZoneScope,
}

/// B5 E20, §3.1: Lock a whole lane — every zone of it the scope covers, both sides' unit and backrow
/// zones by default (Classic #71's "Lock this lane"; its Radiant "the enemy side of this lane" is
/// `side: "enemy"`). "This lane" is the lane the card running the script stands in. A zone Locked
/// already stays as it is; a Lock evicts nothing, so the card that locked its own zone stays in it.
pub fn lock_lane(args: LockLaneArgs) -> Effect {
    Effect::new("lockLane", move |ctx| {
        let spec = args.lane.clone().unwrap_or(LaneSpec::Card(TargetSpec::SelfCard));
        let Some(lane) = lane_of(ctx, &spec) else {
            return;
        };
        for slot in zones_in_scope(ctx, &args.scope) {
            if slot.lane == lane {
                lock_one(ctx, &slot);
            }
        }
    })
}

/// The zone the permanent an arrival event names was just put into: a `summoned` event names it; a
/// `cardPlayed` names the card, whose zone is where it stands, if it still stands there on the stay the
/// play put it on (R174, R212). A Spell's play puts nothing anywhere.
fn zone_played_into(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<ZoneSlot> {
    let instance_id = match event {
        GameEvent::Summoned {
            player, row, lane, ..
        } => {
            return Some(ZoneSlot {
                player: *player,
                row: *row,
                lane: *lane,
            });
        }
        GameEvent::CardPlayed { instance_id, .. } | GameEvent::CardResolved { instance_id, .. } => {
            instance_id
        }
        _ => return None,
    };
    let card = find_instance(ctx.state, instance_id)?;
    if card.zone.z() != ZoneName::Field {
        return None;
    }
    if let Some(mark) = event_mark(event)
        && left_field_after(ctx.state, mark, &card.id)
    {
        return None;
    }
    slot_of(ctx.state, card)
}

/// `lockPlayedZone`'s argument (TS default `{}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LockPlayedZoneArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<GameEvent>,
}

/// B5 E20: Lock the zone a permanent was just played into (Classic #84 Lockdown's "After a permanent is
/// played, Lock its zone", Classic+ #34). `event` is the trigger's own (`ctx.event`, read from the
/// context's captured event when omitted): a `summoned`, a `cardPlayed` or a `cardResolved`. A played
/// Spell, or a permanent already gone from where it landed, locks nothing.
pub fn lock_played_zone(args: LockPlayedZoneArgs) -> Effect {
    Effect::new("lockPlayedZone", move |ctx| {
        // The captured event is JSON in the data bag (SURFACE §4.4.10): anything that does not read as
        // an event is no event, as TS's `typeof event !== "object"` guard had it.
        let event = match &args.event {
            Some(event) => Some(event.clone()),
            None => ctx
                .data
                .get(EVENT_KEY)
                .and_then(|value| serde_json::from_value::<GameEvent>(value.clone()).ok()),
        };
        let Some(event) = event else {
            return;
        };
        if let Some(slot) = zone_played_into(ctx, &event) {
            lock_one(ctx, &slot);
        }
    })
}

/// B5 E20: Lock one random zone the scope covers that is not Locked already — an occupied one is fine,
/// a Lock evicts nothing (Classic+ #34 Memory Leak: "a random zone on your opponent's side", `side:
/// "enemy"`). One uniform draw from the match rng among the candidates (R60); none left, nothing.
pub fn lock_random_zone(args: ZoneScope) -> Effect {
    Effect::new("lockRandomZone", move |ctx| {
        let open: Vec<ZoneSlot> = zones_in_scope(ctx, &args)
            .into_iter()
            .filter(|slot| !is_locked(ctx.state, slot))
            .collect();
        if open.is_empty() {
            return;
        }
        let at = ctx.rng.int(open.len() as i32);
        if let Some(slot) = open.get(at as usize) {
            lock_one(ctx, &slot.clone());
        }
    })
}

/// R900: Unlock one random Locked zone the scope covers, both sides and both rows by default (Meditative
/// #27 Clip-Farming Lawyer's "Unlock a random zone"), the mirror of `lock_random_zone`. One uniform draw
/// from the match rng among the Locked candidates (R60), an occupied one as fair a pick as an empty one;
/// none Locked, nothing is drawn (R129).
pub fn unlock_random_zone(args: ZoneScope) -> Effect {
    Effect::new("unlockRandomZone", move |ctx| {
        let locked: Vec<ZoneSlot> = zones_in_scope(ctx, &args)
            .into_iter()
            .filter(|slot| is_locked(ctx.state, slot))
            .collect();
        if locked.is_empty() {
            return;
        }
        let at = ctx.rng.int(locked.len() as i32);
        if let Some(slot) = locked.get(at as usize) {
            unlock_one(ctx, &slot.clone());
        }
    })
}

/// B5 E20: Lock the zone the card running the script stands in — a firing trap's own backrow zone
/// (Classic+ #1 Doom Shroom's "Lock this zone"). The trap is still in it while its list runs, and is
/// consumed after (`traps::consume_trap`), leaving the zone Locked behind it. A card its own list moved
/// off the field first locks nothing.
pub fn lock_own_zone() -> Effect {
    Effect::new("lockOwnZone", move |ctx| {
        // TS read the live `ctx.self`, whose zone is where the card stands now.
        let slot = ctx.live_self().and_then(|this| slot_of(ctx.state, this));
        if let Some(slot) = slot {
            lock_one(ctx, &slot);
        }
    })
}

/// B5 E20: Unlock every Locked zone the scope covers — both sides and both rows by default (Classic+
/// #77 Anti-Softlock's "Unlock every zone") — one `unlocked` event per zone opened, in R68's walk.
pub fn unlock_all(args: ZoneScope) -> Effect {
    Effect::new("unlockAll", move |ctx| {
        for slot in zones_in_scope(ctx, &args) {
            unlock_one(ctx, &slot);
        }
    })
}
