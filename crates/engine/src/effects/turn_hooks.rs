//! "Trigger your End of turn effects" (docs/meditative-set.md M5, ME-TRIG (c); R824): Meditative #12
//! Fear Mongerer's Cry, and its Radiant face's "N times".
//!
//! The verb is a card-facing name for `triggers::queue_turn_hook_rounds`, which owns the sequence: the
//! controller's end-of-turn hooks are queued as §2.2's end-of-turn trigger step would queue them, and
//! the loop that settles the list this effect stands in resolves them after it, in R68's order. The turn
//! does not end, the trap window and the delayed effects do not run (R62), and a second round is queued
//! only once the first has popped.

use serde::{Deserialize, Serialize};

use crate::script::Effect;

/// `trigger_turn_hooks`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
