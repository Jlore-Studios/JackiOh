//! Switch position as an effect: 5pek Controller and friends spend no exertion (R20).
//!
//! Port of `packages/engine/src/effects/position.ts`.

use serde::{Deserialize, Serialize};

use crate::combat::{SwitchPositionOptions, switch_position};
use crate::damage::DamageTarget;
use crate::script::Effect;
use crate::state::CardInstance;
use crate::wire::Position;
use crate::zones::active_units_of;

use super::targets::{PlayerSpec, TargetSpec, player_of, resolve_target};

/// `switchPositionOf`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwitchPositionOfArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Position>,
    pub target: TargetSpec,
}

pub fn switch_position_of(args: SwitchPositionOfArgs) -> Effect {
    Effect::new("switchPosition", move |ctx| {
        let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, &args.target) else {
            return;
        };
        let _ = switch_position(
            ctx,
            &instance,
            SwitchPositionOptions {
                spend_exertion: Some(false),
                to: args.to,
            },
        );
    })
}

/// `switchAllPositions`' `side`: `PlayerSpec | "both"`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SwitchAllPositionsSide {
    #[serde(rename = "self")]
    SelfSide,
    #[serde(rename = "enemy")]
    Enemy,
    #[serde(rename = "both")]
    Both,
}

/// `switchAllPositions`' argument; TS's default is `{ side: "both" }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SwitchAllPositionsArgs {
    pub side: SwitchAllPositionsSide,
}

impl Default for SwitchAllPositionsArgs {
    fn default() -> Self {
        SwitchAllPositionsArgs {
            side: SwitchAllPositionsSide::Both,
        }
    }
}

/// #48: switch every unit, or only one side's, spending no exertion (R20).
pub fn switch_all_positions(args: SwitchAllPositionsArgs) -> Effect {
    Effect::new("switchAllPositions", move |ctx| {
        let players = match args.side {
            SwitchAllPositionsSide::Both => vec![
                player_of(ctx, PlayerSpec::SelfSide),
                player_of(ctx, PlayerSpec::Enemy),
            ],
            SwitchAllPositionsSide::SelfSide => vec![player_of(ctx, PlayerSpec::SelfSide)],
            SwitchAllPositionsSide::Enemy => vec![player_of(ctx, PlayerSpec::Enemy)],
        };
        for player in players {
            // The list as it stands before the first switch, as TS walked the array it was handed.
            let units: Vec<CardInstance> = active_units_of(ctx.state, player).into_iter().cloned().collect();
            for unit in units {
                let _ = switch_position(
                    ctx,
                    &unit,
                    SwitchPositionOptions {
                        spend_exertion: Some(false),
                        ..SwitchPositionOptions::default()
                    },
                );
            }
        }
    })
}
