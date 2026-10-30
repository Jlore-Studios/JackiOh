//! Trigger a Cry (docs/classic-sets.md B5 E13, R467): the card-facing verb over `crate::cry_trigger`,
//! which owns the sequence (who runs the Cry, how its choices are asked, "this" out of a graveyard).
//! Classic #54 Rewind's Radiant triggers a Cry twice: two of these, each with its own choices.

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::cry_trigger::{cry_place_of, trigger_cry_of, trigger_random_cry_of};
use crate::damage::DamageTarget;
use crate::script::Effect;
use crate::state::{CardInstance, GameState};

/// `trigger_cry`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TriggerCryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// R1869001 (Stinkyscape #71 A'Zoth): every choice of the controller's drawn at random, as
    /// R1855001's are (`cry_trigger::trigger_random_cry_of`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<bool>,
}

/// B5 E13: trigger the Cry of the Unit `target` names (default the play's first choice) or the card
/// `instanceId` names, its choices this card's controller's, or with `random` drawn from the match rng
/// (R1869001). A pile top or a face-up Field Spell runs it as itself (R1873003), a graveyard Unit with
/// no "this"; anything else triggers nothing (`cry_trigger::cry_place_of`).
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
        if args.random == Some(true) {
            trigger_random_cry_of(ctx, &card, controller);
        } else {
            trigger_cry_of(ctx, &card, controller);
        }
    })
}

/// B5 E13: whether this card's Cry can be triggered where it lies now, a pure read for a card's target
/// check (Classic #54's "a Unit that has a Cry") and its `conditionMet`.
pub fn has_triggerable_cry(state: &GameState, card: &CardInstance) -> bool {
    cry_place_of(state, card).is_some()
}
