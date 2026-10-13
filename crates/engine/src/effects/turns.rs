//! Turn effects: lost refreshes and owed extra turns (SPEC §2.2, §2.3, ME-TURN, R844–R847).
//!
//! Port of nothing shipped: Meditative #18, #19 and #19.1 are the first cards that lose a refresh
//! or owe a turn. A card file reaches these as `jackioh_engine::effects::{lose_refreshes,
//! take_extra_turn}` or through `jackioh_engine::prelude`.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::turn::EXTRA_TURN_MODIFIER_ID;
use crate::wire::GameEvent;

/// `loseRefreshes`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoseRefreshesArgs {
    pub turns: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R844: "lose all mana next N turns" — the caster's next N refreshes give 0 (and spend their
/// riders with them, `mana::refreshed_mana`). The loss is a `turns_started` index through which
/// the refresh is lost, so overlapping losses keep the latest end. A change of 0 changes nothing,
/// and announces nothing.
pub fn lose_refreshes(args: LoseRefreshesArgs) -> Effect {
    Effect::new("loseRefreshes", move |ctx| {
        if args.turns <= 0 {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let side = &mut ctx.sink.state.players[player];
        let through = side.turns_started + args.turns.max(0);
        if side.lost_refresh_through.is_some_and(|old| old >= through) {
            return;
        }
        side.lost_refresh_through = Some(through);
        ctx.sink.events.push(GameEvent::ModifierChanged {
            player,
            modifier_id: crate::mana::LOST_REFRESH_MODIFIER_ID.to_string(),
            added: true,
        });
    })
}

/// `takeExtraTurn`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TakeExtraTurnArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// R847: set by Temporal Rift — at most one granted turn per player per game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rift: Option<bool>,
}

/// R845–R847: owe the player one extra turn, taken when their turn ends by starting their turn
/// again. With `rift` set and the player's Rift flag already set, nothing is granted and nothing
/// is reported; otherwise `rift` sets the flag first. The count is never stored at 0.
pub fn take_extra_turn(args: TakeExtraTurnArgs) -> Effect {
    Effect::new("takeExtraTurn", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        if args.rift == Some(true) {
            let side = &ctx.sink.state.players[player];
            if side.rift_extra_turn == Some(true) {
                return;
            }
            ctx.sink.state.players[player].rift_extra_turn = Some(true);
        }
        let side = &mut ctx.sink.state.players[player];
        side.extra_turns = Some(side.extra_turns.unwrap_or(0) + 1);
        ctx.sink.events.push(GameEvent::ModifierChanged {
            player,
            modifier_id: EXTRA_TURN_MODIFIER_ID.to_string(),
            added: true,
        });
    })
}
