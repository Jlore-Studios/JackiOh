//! Copy a Unit's text (R1850000; Stinkyscape #20 Tnack Trav): the running card has the text of the
//! Unit `target` names from now on, while it stays on the field (`subsystems::unit_text`).

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::effects::targets::{TargetSpec, instance_by_id_or_spec};
use crate::script::Effect;
use crate::state::{find_instance, find_instance_mut};
use crate::subsystems::unit_text::{UNIT_TEXT_KEY, text_def_of};
use crate::wire::{CardType, GameEvent, KeywordKind};

/// `copyUnitText`'s argument: the Unit whose text is copied (the first chosen one by default), and
/// `radiant: true` to copy its Radiant text whatever face it wears.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CopyUnitTextArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// R1850000: the running card copies the text of a Unit on the field — the text that Unit runs now
/// (a copier's copy, a fused card's fused text), on the face it wears or, with `radiant`, its Radiant
/// face: keywords, aura, triggers, Death, abilities and declared numbers; its name, cost, type, tags
/// and stats stay. Reported by an in-place `transformed`. Nothing happens when the running card is off
/// the field or Immutable (§6.1, R23), or the target is no Unit on the field, is the running card, or
/// is Vanilla, with no text to copy (§6.3).
pub fn copy_unit_text(args: CopyUnitTextArgs) -> Effect {
    Effect::new("copyUnitText", move |ctx| {
        let Some(copier) = ctx
            .self_
            .as_ref()
            .and_then(|me| find_instance(ctx.state, &me.id))
            .cloned()
        else {
            return;
        };
        if !crate::zones::acts_on_field(ctx.state, &copier)
            || crate::layers::unit_has(ctx.state, &copier, KeywordKind::Immutable)
        {
            return;
        }
        let Some(source) = instance_by_id_or_spec(ctx, None, args.target.as_ref()) else {
            return;
        };
        let Some(source) = find_instance(ctx.state, &source.id).cloned() else {
            return;
        };
        if source.id == copier.id
            || source.vanilla
            || !crate::zones::acts_on_field(ctx.state, &source)
            || crate::faces::card_type_of(ctx.state, &source) != CardType::Unit
        {
            return;
        }
        let (def_id, radiant) = text_def_of(&source);
        let record = json!({ "defId": def_id, "radiant": radiant || args.radiant == Some(true) });
        if let Some(live) = find_instance_mut(ctx.state, &copier.id) {
            live.memory.insert(UNIT_TEXT_KEY.to_string(), record);
        }
        ctx.events.push(GameEvent::Transformed {
            instance_id: copier.id.clone(),
            from_def_id: copier.def_id.clone(),
            to_def_id: copier.def_id.clone(),
            new_instance_id: copier.id.clone(),
            hidden_from: None,
            creator: None,
        });
    })
}
