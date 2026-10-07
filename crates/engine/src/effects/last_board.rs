//! The verbs over a last board (docs/classic-sets.md B5 E30, SPEC §8.7 C+ #29, R417, R564): Discover
//! a card of the caster's frozen board, add the Discovered one, add random ones. The board is
//! `subsystems::last_boards`'; these only read it and make cards from it.
//!
//! Each card made is a NEW card the caster owns, on its entry's face, through `add_to_hand`'s one
//! creation path (§2.4's hand cap burns it, R317; a unit-token card in hand is R11's). A fused entry
//! is rebuilt from its id into this match the first time it is offered or made
//! (`subsystems::fuse::rebuild_fused_def`, R179). An empty board opens no prompt and draws nothing (R129).
//!
//! Port of `packages/engine/src/effects/lastBoard.ts`. Its two numbers, `LAST_BOARD_DISCOVER_OPTIONS`
//! (§6.3 Discover: "choose 1 of 3") and `LAST_BOARD_CARD_COST` (C+ #29: "It costs (0)", "Each costs
//! (0)" — a `costOverride`, R65), live in `crate::config` (CLAUDE.md rule 9), and `effects/mod.rs`
//! re-exports them where TS's barrel did.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::catalog::self_def_ids;
use crate::config::LAST_BOARD_DISCOVER_OPTIONS;
use crate::prelude::json_as;
use crate::prompts::{OpenPromptArgs, open_prompt, resume_self};
use crate::script::{Effect, EffectContext};
use crate::state::{LastBoardEntry, PromptOption};
use crate::subsystems::fuse::rebuild_fused_def;
use crate::subsystems::last_boards::last_board_candidates;
use crate::wire::{PromptKind, Selection};

use super::add_to_hand::add_to_hand;
use super::choose::chosen_options;

/// The caster's candidates, never the running card's own definitions (R387).
fn candidates_for(ctx: &EffectContext<'_>) -> Vec<LastBoardEntry> {
    let own: Option<String> = match &ctx.self_ {
        Some(this) => Some(this.def_id.clone()),
        None => ctx.def_id.clone(),
    };
    let exclude: Vec<String> = match &own {
        None => Vec::new(),
        Some(own) => self_def_ids(own),
    };
    last_board_candidates(ctx.state, ctx.controller, &exclude)
}

/// One entry as a new card in the caster's hand, on its face, with the cost asked for.
fn add_entry(ctx: &mut EffectContext<'_>, entry: &LastBoardEntry, cost_override: Option<i32>) {
    let controller = ctx.controller;
    if rebuild_fused_def(&mut *ctx.state, &entry.def_id, controller).is_none() {
        return;
    }
    // `addToHand`'s argument as the TS literal `{ defId, radiant, ...(costOverride ? { costOverride } : {}) }`.
    let mut literal = Map::new();
    literal.insert("defId".into(), json!(entry.def_id));
    literal.insert("radiant".into(), json!(entry.radiant));
    if let Some(cost) = cost_override {
        literal.insert("costOverride".into(), json!(cost));
    }
    (add_to_hand(json_as(Value::Object(literal))).apply)(ctx);
}

/// `discoverFromLastBoard`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverFromLastBoardArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
}

/// C+ #29 base (R417): Discover among `count` different cards of the caster's last board (all of
/// them when it holds fewer), drawn without replacement and shown to the chooser only (§10.8, R81).
/// Each option is the card's definition id, on its entry's face; the answer re-enters `step`.
pub fn discover_from_last_board(args: DiscoverFromLastBoardArgs) -> Effect {
    Effect::new("discoverFromLastBoard", move |ctx| {
        let pool = candidates_for(ctx);
        if pool.is_empty() {
            return;
        }
        let count = args.count.unwrap_or(LAST_BOARD_DISCOVER_OPTIONS).max(0) as usize;
        let shuffled: Vec<LastBoardEntry> = ctx.rng.shuffle(&pool);
        let controller = ctx.controller;
        let mut options: Vec<PromptOption> = Vec::new();
        for entry in shuffled.into_iter().take(count) {
            let Some(name) = rebuild_fused_def(&mut *ctx.state, &entry.def_id, controller).map(|def| def.name.clone())
            else {
                continue;
            };
            options.push(PromptOption {
                key: format!("mode:{}", entry.def_id),
                label: name,
                selection: Selection::Mode {
                    option: entry.def_id.clone(),
                },
                cost: None,
                radiant: if entry.radiant { Some(true) } else { None },
            });
        }
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        let _ = open_prompt(
            ctx,
            OpenPromptArgs {
                player: controller,
                kind: PromptKind::Discover,
                aim: None,
                prompt: "Discover a card from the board your last game ended with".to_string(),
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

/// `addFromLastBoard`'s argument (TS default `{}`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddFromLastBoardArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
}

/// C+ #29 base's resume (R417): the Discovered card into the caster's hand, on its entry's face.
pub fn add_from_last_board(args: AddFromLastBoardArgs) -> Effect {
    Effect::new("addFromLastBoard", move |ctx| {
        let picked: Option<String> = chosen_options(ctx).into_iter().next();
        let entry = candidates_for(ctx)
            .into_iter()
            .find(|candidate| picked.as_deref() == Some(candidate.def_id.as_str()));
        if let Some(entry) = entry {
            add_entry(ctx, &entry, args.cost_override);
        }
    })
}

/// `addRandomFromLastBoard`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddRandomFromLastBoardArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
}

/// C+ #29 Radiant (R417): `count` different random cards of the caster's last board (all of them when
/// it holds fewer) straight into the caster's hand, drawn without replacement in one shuffle.
pub fn add_random_from_last_board(args: AddRandomFromLastBoardArgs) -> Effect {
    Effect::new("addRandomFromLastBoard", move |ctx| {
        let pool = candidates_for(ctx);
        if pool.is_empty() || args.count < 1 {
            return;
        }
        let picked: Vec<LastBoardEntry> = ctx.rng.shuffle(&pool).into_iter().take(args.count as usize).collect();
        for entry in picked {
            add_entry(ctx, &entry, args.cost_override);
        }
    })
}
