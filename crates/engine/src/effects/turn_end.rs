//! Ending a turn from an effect (docs/classic-sets.md B5 E10, R456): Classic+ #26 Tommy Tempo's "End
//! your turn" and its Radiant "You may take one more action. Then your turn ends", and the AI card Rate
//! Limit's "After it resolves, their turn ends".
//!
//! Neither verb ends anything by itself. Each puts a "your turn ends" rider on the player whose turn it
//! is (`modifiers::cut_turn_short`, a `turnEnds` modifier that lasts this turn), and `reduce.rs` ends the
//! turn once it is due: the rest of the effect list resolves, then everything the action set off (the
//! §10.3 loop), and only then does the turn end, as if its player had pressed End turn — every
//! end-of-turn step runs (§2.2, R62), after `turnCutShort`. "One more action" counts that player's
//! main-phase actions (a play, an attack, a position switch, an activation) and ends the turn once the
//! last one has resolved; ending the turn themselves uses it up. On the other player's turn there is no
//! turn of that player's to end, and nothing happens.
//!
//! Port of `packages/engine/src/effects/turnEnd.ts`.

use serde::{Deserialize, Serialize};

use crate::modifiers::cut_turn_short;
use crate::script::Effect;

use super::targets::{PlayerSpec, player_of};

/// `endTurn`'s argument (TS default `{}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EndTurnArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// B5 E10, R456: `player`'s turn ends as soon as what is resolving now has resolved. "self" (the
/// default) is the controller's own turn (Tommy Tempo); "enemy" is the opponent's, whose play set a
/// trap off (Rate Limit: the play that set it off resolves first).
pub fn end_turn(args: EndTurnArgs) -> Effect {
    Effect::new("endTurn", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let by = ctx.self_.as_ref().map(|this| this.id.clone());
        cut_turn_short(ctx, player, 0, by);
    })
}

/// `endTurnAfterActions`'s argument (TS default `{}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EndTurnAfterActionsArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// B5 E10, R456: `player` may take `actions` more main-phase actions (default 1), then their turn ends
/// — once the last of them has resolved, prompts included. With a rider already on the turn, the
/// sooner end holds.
pub fn end_turn_after_actions(args: EndTurnAfterActionsArgs) -> Effect {
    Effect::new("endTurnAfterActions", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let by = ctx.self_.as_ref().map(|this| this.id.clone());
        cut_turn_short(ctx, player, args.actions.unwrap_or(1).max(0), by);
    })
}
