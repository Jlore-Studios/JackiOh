//! Rotate as a verb (SPEC §6.3 Rotate, §3.1's rotation-topology ruling, §3.2; R14, R88; §8.3 #52).
//!
//! `subsystems/rotation.rs` is that ruling in full: both rings turning one step together, the whole
//! board read first so a rotation is atomic; a Stack pile travelling whole (§3.2, R13); `controller`
//! changing only across the centre line, `owner` never (R12); a Locked or Reborn-reserved destination
//! bouncing the card to its OWNER's hand (R14, R88, R4's hand cap, R11's vanishing token); and #52's
//! radiant face replacing an outbound crossing with that bounce at `costOverride` 0.
//!
//! None of that is here: a second ring walk would be a second topology. This file is the wrapper,
//! because `rotate_rings` takes an `EngineSink` and mutates the board, which a card script may not do
//! (CLAUDE.md rule 5).
//!
//! THE TWO DEFAULTS: §3.1 reads "left" and "right" from the ROTATING player's seat, so `perspective`
//! is the controller; and #52's two faces differ only in `ctx.radiant`, which `rotate_rings` honours
//! through its own `radiant` argument. So `rotate({ direction })` is the complete call.

use serde::{Deserialize, Serialize};

use crate::script::Effect;
use crate::subsystems::rotation::{RotationArgs, rotate_rings};
use crate::wire::RotationDirection;

use super::targets::{PlayerSpec, player_of};

/// `rotate`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
/// declared (R81 makes #52's direction a play choice, never a prompt).
///
/// There is no fizzle case to write: a board with nothing on it rotates nothing, and the subsystem
/// still emits its one `rotated` event, which is what §10.10 animates.
pub fn rotate(args: RotateArgs) -> Effect {
    Effect::new("rotate", move |ctx| {
        let rotation = RotationArgs {
            direction: args.direction,
            perspective: player_of(ctx, args.perspective.unwrap_or(PlayerSpec::SelfSide)),
            radiant: Some(args.radiant.unwrap_or(ctx.radiant)),
        };
        // `EffectContext` derefs to `EngineSink`, so the subsystem takes the context as it stands.
        let _ = rotate_rings(ctx, &rotation);
    })
}
