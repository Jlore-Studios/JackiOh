//! Evolve, as a card-script verb (SPEC §6.3's "Evolve" row, R1849001; Stinkyscape #91 The
//! Propagation's Wasp, #92 The Propagation's Hornet: "After this attacks, summon a copy of this, then
//! evolve. Evolve: gain Flying").
//!
//! "Evolve" is a keyword whose meaning each card prints: the card's own `evolve` hook (`Script.evolve`)
//! is what evolving it does, and nothing but this verb runs it. A thin wrapper, as `effects/combat.rs`'s
//! are: the hook runs the resumable way through `prompts::run_evolve`, so a question in it pauses the
//! list that evolved the unit like any other.

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

/// R1849001: "evolve" — run the target Unit's `evolve` hook, for its controller, while it acts on the
/// field. A Unit with no such hook, or one no longer on the field, evolves into nothing, silently.
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
