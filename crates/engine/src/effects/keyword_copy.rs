//! R4592006: gain keywords of enemy units (Infinite Assault, Scraper).
//!
//! Gains keywords from enemy units on the field: base gains keywords of one random enemy unit,
//! radiant of every enemy unit in lane order. Does not take Brittle, Temporary, spent Divine Shield
//! or spent Reborn.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::effects::buff::grant_keyword;
use crate::layers::card_keywords;
use crate::prelude::json_as;
use crate::script::Effect;
use crate::wire::{Keyword, opponent_of};
use crate::zones::{active_units_of, in_units_row_or_standing};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GainKeywordsOfArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every: Option<bool>,
}

pub fn gain_keywords_of(args: GainKeywordsOfArgs) -> Effect {
    Effect::new("gainKeywordsOf", move |ctx| {
        let Some(self_card) = ctx.live_self() else {
            return;
        };
        if !in_units_row_or_standing(ctx.state, &self_card) {
            return;
        }

        let opponent = opponent_of(ctx.controller);
        let unit_instances: Vec<crate::state::CardInstance> = active_units_of(ctx.state, opponent)
            .into_iter()
            .cloned()
            .collect();
        let sources: Vec<crate::state::CardInstance> = if args.every == Some(true) {
            unit_instances
        } else if unit_instances.is_empty() {
            Vec::new()
        } else if unit_instances.len() == 1 {
            unit_instances
        } else {
            match ctx.rng.pick(&unit_instances).cloned() {
                Some(unit) => vec![unit],
                None => Vec::new(),
            }
        };

        let mut to_grant: Vec<Keyword> = Vec::new();
        for source in &sources {
            let keywords = card_keywords(ctx.state, source);
            for kw in keywords {
                let eligible = match &kw {
                    Keyword::Brittle { .. } | Keyword::Temporary { .. } => false,
                    Keyword::DivineShield if source.divine_shield_spent == Some(true) => false,
                    Keyword::Reborn if source.reborn_spent == Some(true) => false,
                    _ => true,
                };
                if eligible {
                    to_grant.push(kw);
                }
            }
        }

        for kw in to_grant {
            let effect = grant_keyword(json_as(json!({
                "target": { "of": "self" },
                "keyword": kw,
            })));
            (effect.apply)(ctx);
        }
    })
}
