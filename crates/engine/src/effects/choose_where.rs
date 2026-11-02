//! A target prompt narrowed by a condition no `TargetScope` field can say (§10.6): Classic #32 Felinor
//! Feelings' Radiant face, asked after its Felinor Token lands so the token's lane counts (the Classic
//! #1–#45 workstream), the prompt half of `TargetFilter.check` / `Script.target_checks`.
//!
//! `where` is read once, as the effect applies; the open prompt holds only data, so a pause survives a
//! JSON round trip (§9.3). `view_for` redacts an option the chooser may not read (R177).

use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::choose::{TargetScope, targets_in_scope};
use crate::catalog::def_of;
use crate::prompts::{OpenPromptArgs, hero_option_label, open_prompt, resume_self};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, PromptOption, find_instance};
use crate::wire::{PromptKind, Selection};

fn key_of(selection: &Selection) -> String {
    match selection {
        Selection::Instance { instance_id } => format!("instance:{instance_id}"),
        Selection::Hero { player } => format!("hero:{player}"),
        _ => "none".to_string(),
    }
}

/// `choose_target_where`'s `where`: whether a card (or a hero, `None`) may be offered.
pub type TargetWhere = Arc<dyn Fn(&EffectContext<'_>, Option<&CardInstance>) -> bool + Send + Sync>;

/// `choose_target_where`'s arguments (TS's inline object). `where_` is a callback, so serde skips it and
/// a card sets it in Rust; left unset it admits everything the scope offers.
#[derive(Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ChooseTargetWhereArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<TargetScope>,
    #[serde(skip)]
    pub where_: Option<TargetWhere>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
}

impl std::fmt::Debug for ChooseTargetWhereArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChooseTargetWhereArgs")
            .field("step", &self.step)
            .field("scope", &self.scope)
            .field("where", &self.where_.is_some())
            .field("prompt", &self.prompt)
            .field("data", &self.data)
            .finish()
    }
}

/// A target prompt for `ctx.controller` over the scope's cards `where` admits (a hero only when it
/// admits `None`). None admitted fizzles, as in `choose_target`.
pub fn choose_target_where(args: ChooseTargetWhereArgs) -> Effect {
    Effect::new("chooseTargetWhere", move |ctx| {
        let reader: &EffectContext<'_> = &*ctx;
        let admits =
            |card: Option<&CardInstance>| args.where_.as_ref().is_none_or(|admits| admits(reader, card));
        let selections: Vec<Selection> = targets_in_scope(reader, args.scope.as_ref())
            .into_iter()
            .filter(|selection| match selection {
                Selection::Instance { instance_id } => {
                    find_instance(reader.state, instance_id).is_some_and(|card| admits(Some(card)))
                }
                _ => admits(None),
            })
            .collect();
        if selections.is_empty() {
            return;
        }
        let options: Vec<PromptOption> = selections
            .into_iter()
            .map(|selection| {
                let card = match &selection {
                    Selection::Instance { instance_id } => find_instance(reader.state, instance_id),
                    _ => None,
                };
                let label = match (card, &selection) {
                    (Some(card), _) => def_of(Some(&*reader.state), &card.def_id).name.clone(),
                    (None, Selection::Hero { player }) => hero_option_label(*player, reader.controller),
                    (None, _) => "nothing".to_string(),
                };
                PromptOption {
                    key: key_of(&selection),
                    label,
                    selection,
                    cost: None,
                    radiant: None,
                }
            })
            .collect();
        let player = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: args
                    .prompt
                    .clone()
                    .unwrap_or_else(|| "Choose a target".to_string()),
                options,
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume,
            },
        );
    })
}
