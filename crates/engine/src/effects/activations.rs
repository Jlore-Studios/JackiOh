//! Another card's Activate (Stinkyscape #26 ZolarKeth): "It can't use its own Activate" refuses a
//! card the use of its abilities (R1876000), and "Activate your Universities" runs a card's ability
//! as the running card's effect (R1876001), with `spendUse` only while the ability has a use left
//! this turn, spending it (R2521003).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::effects::targets::{TargetSpec, instance_by_id_or_spec};
use crate::prompts::{HookInstance, HookResumableOptions, run_hook_resumable};
use crate::script::{Effect, activation_hook};
use crate::state::find_instance_mut;
use crate::subsystems::activate::{
    ACTIVATION_DATA_KEY, ActivationPaid, abilities_of, ability_uses_this_turn, is_acting_on_field, mark_use,
    uses_allowed,
};
use crate::zones::acts_on_field;

/// R1876000: where a card whose own Activate is refused keeps the refusal. R78's reset clears it with
/// the rest of `memory` when the card leaves the field.
pub const OWN_ACTIVATE_REFUSED_KEY: &str = "ownActivateRefused";

/// `refuseOwnActivate`'s and `runActivation`'s argument: the card, by id or by a target spec (the
/// first chosen one by default); `ability` names one of its abilities for `runActivation`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ActivationOfArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ability: Option<String>,
    /// R2521003 (S #26 ZolarKeth's "A University can be Activated at most once each turn"):
    /// `runActivation` runs the ability only when it has not been used this turn, and then spends
    /// every use it has this turn, a Buff's extra ones included (R884000), so the card is Activated
    /// once that turn. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend_use: Option<bool>,
}

/// R1876000: "It can't use its own Activate" — the card named, on the field, is refused every use of
/// its abilities while it stays there (`subsystems::activate` reads the mark). No zone is Locked, and
/// another card may still run its ability (`run_activation`).
pub fn refuse_own_activate(args: ActivationOfArgs) -> Effect {
    Effect::new("refuseOwnActivate", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, args.instance_id.as_deref(), args.target.as_ref())
        else {
            return;
        };
        if !acts_on_field(ctx.state, &card) {
            return;
        }
        if let Some(live) = find_instance_mut(ctx.state, &card.id) {
            live.memory
                .insert(OWN_ACTIVATE_REFUSED_KEY.to_string(), Value::Bool(true));
        }
    })
}

/// R1876001: "Activate your Universities" — the named card's ability (`ability`, else its first that
/// declares no target or mode) runs its effect as an ability of the running card, for the running
/// card's controller, with the card as its `self` on the face it wears. No cost, no play (R384), and
/// without `spend_use` no use spent and neither its once-a-turn nor a refusal of its own use
/// (R1876000) read. An ability that declares a target or a mode is never run, nor a card that does not
/// act on the field (R13).
///
/// R2521003: with `spend_use` the ability runs only while it has a use left this turn (`uses_allowed`
/// against `ability_uses_this_turn`, B3.2 rules 1, 3 and 9) and the run spends one (`mark_use`), so the
/// card's own Activate and every run with `spend_use` share them. Still no cost, no `activated` event.
pub fn run_activation(args: ActivationOfArgs) -> Effect {
    Effect::new("runActivation", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, args.instance_id.as_deref(), args.target.as_ref())
        else {
            return;
        };
        if !is_acting_on_field(ctx.state, &card) {
            return;
        }
        let Some(decl) = abilities_of(ctx.state, &card).into_iter().find(|decl| {
            decl.targets.is_empty()
                && decl.modes.is_empty()
                && args.ability.as_ref().is_none_or(|ability| *ability == decl.id)
        }) else {
            return;
        };
        if args.spend_use == Some(true) {
            if ability_uses_this_turn(ctx.state, &card, &decl.id) > 0 {
                return;
            }
            for _ in 0..uses_allowed(&card, &decl) {
                mark_use(ctx.state, &card.id, &decl.id);
            }
        }
        let controller = ctx.controller;
        let paid = ActivationPaid {
            ability: decl.id.clone(),
            tributed: Vec::new(),
        };
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert(
            ACTIVATION_DATA_KEY.to_string(),
            serde_json::to_value(&paid).unwrap_or(Value::Null),
        );
        let mut instance = HookInstance::from(&card);
        instance.controller = controller;
        run_hook_resumable(
            ctx,
            instance,
            &activation_hook(&decl.id),
            HookResumableOptions {
                controller: Some(controller),
                targets: Some(Vec::new()),
                modes: Some(Vec::new()),
                data: Some(data),
                exits_from: None,
            },
        );
    })
}
