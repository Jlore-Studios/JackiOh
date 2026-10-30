//! "Trigger your End of turn effects" (docs/meditative-set.md M5, ME-TRIG (c); R824): Meditative #12
//! Fear Mongerer's Cry, and its Radiant face's "N times": a name for `triggers::queue_turn_hook_rounds`,
//! which queues the controller's end-of-turn hooks as §2.2's end-of-turn step would, resolved after
//! this effect's list in R68's order. The turn does not end, the trap window and delayed effects do not
//! run (R62), and a second round is queued only once the first has popped.

use serde::{Deserialize, Serialize};

use crate::script::Effect;

/// `trigger_turn_hooks`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TriggerTurnHooksArgs {
    /// How many rounds, one after another; 0 or less triggers nothing.
    pub times: i32,
}

/// R824: queue this card's controller's End of turn effects `times` times, now.
pub fn trigger_turn_hooks(args: TriggerTurnHooksArgs) -> Effect {
    Effect::new("triggerTurnHooks", move |ctx| {
        let controller = ctx.controller;
        crate::triggers::queue_turn_hook_rounds(ctx, controller, args.times);
    })
}
