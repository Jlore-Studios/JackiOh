//! Evolve, as a card-script verb (SPEC §6.3's "Evolve" row, R1849001; Stinkyscape #91 Wasp, #92
//! Hornet). The card's own `evolve` hook is what evolving it does, run only by this verb, the
//! resumable way (`prompts::run_evolve`), so a question in it pauses the list that evolved the unit.

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, instance_of};
use crate::script::Effect;

/// `evolve`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EvolveArgs {
    pub target: TargetSpec,
}

/// R1849001: run the target Unit's `evolve` hook for its controller while it acts on the field.
pub fn evolve(args: EvolveArgs) -> Effect {
    Effect::new("evolve", move |ctx| {
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };
        if !crate::zones::acts_on_field(ctx.state, &unit) {
            return;
        }
        let controller = unit.controller;
        crate::prompts::run_evolve(ctx, &unit, controller);
    })
}
