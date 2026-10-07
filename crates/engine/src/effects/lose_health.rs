//! Lose health: not damage, so no Armor, no hero cap and no on-damage effects (R18).
//!
//! Port of `packages/engine/src/effects/loseHealth.ts`.

use serde::{Deserialize, Serialize};

use crate::script::Effect;

use super::targets::{PlayerSpec, player_of};

/// `loseHealth`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoseHealthArgs {
    pub player: PlayerSpec,
    pub amount: i32,
}

pub fn lose_health(args: LoseHealthArgs) -> Effect {
    Effect::new("loseHealth", move |ctx| {
        let player = player_of(ctx, args.player);
        crate::damage::lose_health(ctx, player, args.amount);
    })
}
