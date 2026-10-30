//! Mana effects: temporary mana now, and a change to a player's next refresh (§2.3).
//!
//! The engine's own `crate::mana::gain_mana` is reached by its full path; `prelude.rs` resolves a
//! card's bare `gain_mana`/`refresh_mana` to these verbs.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::mana::{NEXT_REFRESH_MODIFIER_ID, mana_event, refresh_some_mana};
use crate::script::Effect;
use crate::wire::{GameEvent, opponent_of};

/// `gainMana`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GainManaArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// Temporary mana, which may take current above max (§2.3).
pub fn gain_mana(args: GainManaArgs) -> Effect {
    Effect::new("gainMana", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let side = &mut ctx.sink.state.players[player];
        crate::mana::gain_mana(side, args.amount);
        let event = mana_event(player, side);
        ctx.sink.events.push(event);
    })
}

/// `refreshMana`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RefreshManaArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Refresh, R364 (#78 /fullsend): give back up to `amount` spent mana, never past max; a refresh
/// that gives nothing announces nothing.
pub fn refresh_mana(args: RefreshManaArgs) -> Effect {
    Effect::new("refreshMana", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let side = &mut ctx.sink.state.players[player];
        let before = side.mana.current;
        refresh_some_mana(side, args.amount);
        if side.mana.current != before {
            let event = mana_event(player, side);
            ctx.sink.events.push(event);
        }
    })
}

/// `nextTurnMana`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct NextTurnManaArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// Hinder and Efficiency Dividend: change a player's next refresh, which floors at 0 (§2.3).
///
/// §6.3 Mana stores it as a modifier, so it is a badge (R169) under `NEXT_REFRESH_MODIFIER_ID` while
/// not 0, which `modifierChanged` names as it appears, changes or goes (here, or at the refresh that
/// spends it, `turn::start_turn`). A change of 0 (#24 with X of 1 or less) announces nothing.
pub fn next_turn_mana(args: NextTurnManaArgs) -> Effect {
    Effect::new("nextTurnMana", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let side = &mut ctx.sink.state.players[player];
        let before = side.mana.next_turn_mod;
        side.mana.next_turn_mod += args.amount;
        if side.mana.next_turn_mod == before {
            return;
        }
        let added = side.mana.next_turn_mod != 0;
        ctx.sink.events.push(GameEvent::ModifierChanged {
            player,
            modifier_id: NEXT_REFRESH_MODIFIER_ID.to_string(),
            added,
        });
    })
}

/// R1854003: "Steal your enemy's mana crystals" (Stinkyscape #7 The Greatest Heist's last reward),
/// by `mana::move_mana_crystals`. Each side whose mana changed is announced; a steal from a player
/// with no maximum mana announces nothing.
pub fn steal_mana_crystals() -> Effect {
    Effect::new("stealManaCrystals", move |ctx| {
        let thief = ctx.controller;
        let victim = opponent_of(thief);
        let (lost, gained) = crate::mana::move_mana_crystals(ctx.sink.state, thief, victim);
        if lost > 0 {
            let event = mana_event(victim, &ctx.sink.state.players[victim]);
            ctx.sink.events.push(event);
        }
        if gained > 0 {
            let event = mana_event(thief, &ctx.sink.state.players[thief]);
            ctx.sink.events.push(event);
        }
    })
}
