//! Rotate as a verb (SPEC §6.3 Rotate, §3.1's rotation-topology ruling, §3.2; R14, R88; §8.3 #52).
//!
//! `subsystems/rotation.rs` is that ruling in full (a Stack pile traveling whole, R13; control across
//! the center line, R12; the Reborn and Locked bounces, R14, R88, R4, R11, R831000; #52's radiant face),
//! and a second ring walk here would be a second topology. This file is the wrapper, since
//! `rotate_rings` mutates the board, which a card script may not do (CLAUDE.md rule 5). §3.1 reads
//! "left" and "right" from the ROTATING player's seat, so `perspective` defaults to the controller.

use serde::{Deserialize, Serialize};

use crate::script::Effect;
use crate::subsystems::rotation::{RotationArgs, rotate_rings};
use crate::wire::RotationDirection;

use super::targets::{PlayerSpec, player_of};

/// `rotate`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RotateArgs {
    pub direction: RotationDirection,
    /// Whose seat "left" and "right" are read from (§3.1). Default the controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<PlayerSpec>,
    /// #52's radiant bounce. Default the face that is running (§5.2), which is what #52 wants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// §6.3 Rotate: every card on the field moves one step around its ring, in the direction the play
/// declared (R81: a play choice, never a prompt). An empty board still emits its one `rotated` event,
/// which §10.10 animates.
pub fn rotate(args: RotateArgs) -> Effect {
    Effect::new("rotate", move |ctx| {
        let rotation = RotationArgs {
            direction: args.direction,
            perspective: player_of(ctx, args.perspective.unwrap_or(PlayerSpec::SelfSide)),
            radiant: Some(args.radiant.unwrap_or(ctx.radiant)),
        };
        let _ = rotate_rings(ctx, &rotation);
    })
}
