//! Counters on an instance and locks on a zone (§6.3). Plague Counters live on the instance and R78
//! clears them when the card leaves the field; a Lock lives on the zone and outlives every occupant.
//! The Plague Counter rules themselves — what a placement is, the multiplier, the report — are
//! `crate::plague`'s (R471); the placement verbs are `super::plague`'s. R429's play count, which rides
//! the card through every zone, is raised here too (`raise_times_played`, R1185001).

use serde::{Deserialize, Serialize};

use super::targets::{PlayerSpec, TargetSpec, instance_of, player_of};
use crate::plague::{place_plague_on, plague_on, remove_plague};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState, find_instance_mut};
use crate::wire::{GameEvent, PLAYER_IDS, PlayerId, Row};
use crate::zones::{ZoneSlot, is_locked, lock_zone, row_size, slot_of, unlock_zone};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
        // The live card: where it stands now.
        ZoneSpec::SelfCard => ctx.live_self().and_then(|card| slot_of(ctx.state, card)),
        ZoneSpec::Chosen { index } => {
            let card = instance_of(ctx, &TargetSpec::Chosen { index: *index })?;
            slot_of(ctx.state, &card)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
        if is_locked(ctx.state, slot) {
            return;
        }
        lock_zone(&mut *ctx.state, slot);
        ctx.events.push(GameEvent::Locked {
            player: slot.player,
            row: slot.row,
            lane: slot.lane,
        });
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
        if !is_locked(ctx.state, slot) {
            return;
        }
        unlock_zone(&mut *ctx.state, slot);
        ctx.events.push(GameEvent::Unlocked {
            player: slot.player,
            row: slot.row,
            lane: slot.lane,
        });
    })
}

/// `raiseTimesPlayed`'s argument: whose cards (default "self"), of which definition, by how much.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RaiseTimesPlayedArgs {
    pub def_id: String,
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// Every card `player` owns of `def_id`, wherever it is, side by side in `find_instance`'s order
/// (hand, deck, graveyard, exile, field, the resolving zone).
fn owned_copies(state: &GameState, player: PlayerId, def_id: &str) -> Vec<CardInstance> {
    let mut out: Vec<CardInstance> = Vec::new();
    for side in PLAYER_IDS {
        let cards = &state.players[side];
        let field = cards
            .units
            .iter()
            .flatten()
            .flatten()
            .chain(cards.backrow.iter().flatten())
            .chain(cards.backrow_piles.iter().flatten().flatten())
            .chain(cards.carried.iter().flatten().flatten());
        let all = cards
            .hand
            .iter()
            .chain(cards.library.iter())
            .chain(cards.graveyard.iter())
            .chain(cards.exile.iter())
            .chain(field)
            .chain(cards.resolving.iter());
        out.extend(
            all.filter(|card| card.owner == player && card.def_id == def_id)
                .cloned(),
        );
    }
    out
}

/// R1185001 (M+ #1.1 Base Camp: "Every Reach the Summit you own goes up N Ascent levels"): R429's play
/// count goes up by `amount` on every card of `defId` the player owns, in every zone, as that many
/// plays would have raised it — only on a card whose script counts its plays (`countsPlays`, M #8's
/// Ascent level, R840). Nothing is played, so nothing resolves and nothing wins: a level of 10 or more
/// wins when the card is next played (R850). A copy made later starts at 0 (R57). Each card reports its
/// new count with `numberChanged` ("timesPlayed"), hidden where the card is (R97: a hand card from the
/// other player, a deck card from both, `card_scope::unreadable_by`). An `amount` of 0 or less does
/// nothing.
pub fn raise_times_played(args: RaiseTimesPlayedArgs) -> Effect {
    Effect::new("raiseTimesPlayed", move |ctx| {
        if args.amount <= 0 {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        for card in owned_copies(ctx.state, player, &args.def_id) {
            if crate::scripts::script_of(ctx.state, &card).flags().counts_plays != Some(true) {
                continue;
            }
            let count = crate::times_played::times_played_of(&card) + args.amount;
            let hidden_from = super::card_scope::unreadable_by(ctx.state, &card);
            if let Some(live) = find_instance_mut(ctx.state, &card.id) {
                live.times_played = Some(count);
            }
            ctx.events.push(GameEvent::NumberChanged {
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                key: "timesPlayed".to_string(),
                value: count,
                hidden_from: if hidden_from.is_empty() {
                    None
                } else {
                    Some(hidden_from)
                },
            });
        }
    })
}
