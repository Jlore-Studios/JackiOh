//! Trigger a Cry (docs/classic-sets.md B5 E13, R467): the card-facing verb over `crate::cry_trigger`,
//! which owns the sequence — who runs the Cry, how its choices are asked, and what "this" means out of
//! a graveyard. Classic #54 Rewind: "Trigger the Cry of one of your Units on the field or in your
//! graveyard"; its Radiant triggers any Unit's twice, which is two of these, each with its own choices.
//!
//! Port of `packages/engine/src/effects/cry.ts`.

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::cry_trigger::{cry_place_of, trigger_cry_of};
use crate::damage::DamageTarget;
use crate::script::Effect;
use crate::state::{CardInstance, GameState};

/// `trigger_cry`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TriggerCryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// B5 E13: trigger the Cry of the Unit `target` names (default the play's first choice) or the card
/// `instanceId` names, for this card's controller, who makes the Cry's choices. A Unit on top of a
/// unit pile runs it as itself; one in a graveyard runs it with no "this". Anything else — a card with
/// no Cry, a dormant card, a card that has left — triggers nothing (`cry_trigger::cry_place_of`).
pub fn trigger_cry(args: TriggerCryArgs) -> Effect {
    Effect::new("triggerCry", move |ctx| {
        let card = match &args.instance_id {
            Some(instance_id) => instance_on_its_stay(ctx, instance_id),
            None => {
                let first_choice = TargetSpec::Chosen { index: None };
                match resolve_target(ctx, args.target.as_ref().unwrap_or(&first_choice)) {
                    Some(DamageTarget::Unit { instance }) => Some(instance),
                    _ => None,
                }
            }
        };
        let Some(card) = card else {
            return;
        };
        let controller = ctx.controller;
        trigger_cry_of(ctx, &card, controller);
    })
}

/// B5 E13: whether this card's Cry can be triggered where it lies now — a Unit with a Cry on top of a
/// unit pile or in a graveyard. A pure read, for a card's target check (Classic #54's "a Unit that
/// has a Cry") and its `conditionMet`.
pub fn has_triggerable_cry(state: &GameState, card: &CardInstance) -> bool {
    cry_place_of(state, card).is_some()
}
