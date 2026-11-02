//! Plague Counters as verbs (SPEC §6.3 Plague Counter; docs/classic-sets.md B5 E19; R471). The counter,
//! the multiplier and the `counterChanged` report are `crate::plague`'s; this file names where a
//! placement goes and asks the questions.
//!
//! PROMPTS (R113, R122, R689). `place_plague_tokens` opens one `target` prompt naming the permanent all
//! its placements go on; `answer_placement` (which `prompts.rs` calls for this hook, SURFACE §6.6)
//! places them and drains what the prompt interrupted. The resume carries only plain numbers, so a
//! paused chain survives JSON and replays exactly. The state check waits for the whole effect (R59), so
//! a unit the placements shrank to 0 health (#42) dies after the last placement, not between two.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::def_of;
use crate::effects::targets::{BoardScope, TargetSpec, cards_in_scope_aimed, instance_of};
use crate::plague::{permanents_on_field, place_plague_on, remove_plague};
use crate::prelude::json_as;
use crate::prompts::{
    AnswerInput, OpenPromptArgs, close_prompt, in_offered_order, open_prompt, resume_at, resume_of,
    why_answer_refused,
};
use crate::random_cast::{cast_mode_for_prompt, prefer_enemies};
use crate::script::{Effect, EngineSink};
use crate::state::{CardInstance, EngineError, GameState, PromptOption, Resume, find_instance};
use crate::wire::{PlayerId, PromptKind, Selection, ZoneName};
use crate::work::{begin_work_cascade, drain_work};
use crate::zones::is_buried;

/// `{ of: "self" }`, built from its JSON so this file names no variant of `TargetSpec`'s own.
fn self_spec() -> TargetSpec {
    json_as(json!({ "of": "self" }))
}

/// A card the verbs below may put tokens on: a permanent on the field, not dormant (R13).
fn on_field(state: &GameState, card: Option<&CardInstance>) -> bool {
    match card {
        Some(card) => card.zone.z() == ZoneName::Field && !is_buried(state, card),
        None => false,
    }
}

/// `placePlague`'s arguments: the target defaults to the running card.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    pub amount: i32,
}

/// "Place N Plague Counters on X" (Classic #39 Outbreak, #59 Plague Doctor, Classic+ #3, #87): one
/// placement of `amount` on `target`, times its multiplier (R471); nothing for a card off the field.
pub fn place_plague(args: PlacePlagueArgs) -> Effect {
    Effect::new("placePlague", move |ctx| {
        let spec = args.target.clone().unwrap_or_else(self_spec);
        let card = instance_of(ctx, &spec);
        if !on_field(ctx.state, card.as_ref()) {
            return;
        }
        if let Some(card) = card {
            place_plague_on(ctx, &card, args.amount);
        }
    })
}

/// `placePlagueEach`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueEachArgs {
    pub scope: BoardScope,
    pub amount: i32,
}

/// One placement on each card of a board scope, in R68's order (Classic #63 Crop Dusting: every
/// backrow card is in a scope's backrow row, so face-down ones too).
pub fn place_plague_each(args: PlacePlagueEachArgs) -> Effect {
    Effect::new("placePlagueEach", move |ctx| {
        // MD-B1, R940: a harmful walk — an immune card in a tribal scope is passed by.
        let cards: Vec<CardInstance> = cards_in_scope_aimed(ctx, &args.scope, crate::wire::TargetAim::Harm)
            .into_iter()
            .collect();
        for card in &cards {
            place_plague_on(ctx, card, args.amount);
        }
    })
}

/// `placePlagueRandom`'s arguments: the scope defaults to every unit on both sides.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueRandomArgs {
    pub count: i32,
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<BoardScope>,
}

/// One placement on each of `count` different random cards of a scope (Classic #42), all of them if
/// fewer exist (R60), placed in R68's order. An empty scope draws nothing (R129).
pub fn place_plague_random(args: PlacePlagueRandomArgs) -> Effect {
    Effect::new("placePlagueRandom", move |ctx| {
        let scope: BoardScope = match &args.scope {
            Some(scope) => scope.clone(),
            None => json_as(json!({ "side": "any", "rows": ["units"] })),
        };
        // MD-B1, R940: a harmful walk.
        let pool: Vec<CardInstance> = cards_in_scope_aimed(ctx, &scope, crate::wire::TargetAim::Harm)
            .into_iter()
            .collect();
        let count = args.count;
        if pool.is_empty() || count <= 0 {
            return;
        }
        let picked: IndexSet<String> = ctx
            .sink
            .rng
            .shuffle(&pool)
            .into_iter()
            .take(count as usize)
            .map(|card| card.id)
            .collect();
        for card in &pool {
            if picked.contains(&card.id) {
                place_plague_on(ctx, card, args.amount);
            }
        }
    })
}

/// `consumePlague`'s arguments: the target defaults to the running card, the amount to 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ConsumePlagueArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
}

/// Take up to `amount` Plague Counters off the card `target` names (Classic #78), flooring at 0.
pub fn consume_plague(args: ConsumePlagueArgs) -> Effect {
    Effect::new("consumePlague", move |ctx| {
        let spec = args.target.clone().unwrap_or_else(self_spec);
        let Some(card) = instance_of(ctx, &spec) else {
            return;
        };
        remove_plague(ctx, &card, args.amount.unwrap_or(1));
    })
}

// "Place N Plague Counters": N placements, each a prompt (R471, R113, R122).

/// The hook every placement prompt names, answered by `answer_placement` below (R122).
pub const PLAGUE_PLACEMENT_HOOK: &str = "plague:placement";

/// What the one placement prompt carries to its answer: each placement's tokens and how many land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlacementData {
    amount: i32,
    count: i32,
}

/// A JSON number, as the integer the data bag was written with.
fn number_of(value: Option<&Value>) -> Option<i32> {
    let value = value?;
    if let Some(n) = value.as_i64() {
        return Some(n as i32);
    }
    value.as_f64().map(|n| n as i32)
}

fn placement_data(data: &IndexMap<String, Value>) -> Option<PlacementData> {
    let amount = number_of(data.get("amount"))?;
    let count = number_of(data.get("count"))?;
    Some(PlacementData { amount, count })
}

/// The label a placement option shows its chooser; `view_for` hides a card the chooser may not read (R177).
fn label_of(state: &GameState, card: &CardInstance) -> String {
    def_of(Some(state), &card.def_id).name.clone()
}

/// Open the one placement prompt over every permanent on the field (R68's order), or return false
/// when there is none: the placements fizzle and draw nothing (R129).
fn ask_placement(sink: &mut EngineSink<'_>, player: PlayerId, resume: Resume) -> bool {
    let cards: Vec<CardInstance> = permanents_on_field(sink.state, player)
        .into_iter()
        .cloned()
        .collect();
    if cards.is_empty() {
        return false;
    }
    let data = placement_data(&resume.data);
    let tokens = data.map_or(1, |data| data.amount);
    let count = data.map_or(1, |data| data.count).max(1);
    let total = tokens * count;
    // B5 E12, R452: a random cast's caster is never asked, so every placement goes on one random
    // permanent (R60). This hook answers its own prompts, so `open_prompt` would ask instead.
    let mode = cast_mode_for_prompt(sink.state, player, resume.instance_id.as_deref());
    // R1200: while a Mayor acts the placement is drawn at random too, and nobody is asked.
    let random = mode.is_some_and(|mode| mode.random) || crate::random_targets::targets_random(sink.state);
    if random {
        let now: Vec<CardInstance> = permanents_on_field(sink.state, player)
            .into_iter()
            .cloned()
            .collect();
        let pool: Vec<CardInstance> = if mode.is_some_and(|mode| mode.friends_only) {
            // R1850004: a cast for its caster's own side places there alone.
            crate::random_cast::friends_only(sink.state, player, &now, |card: &CardInstance| {
                Selection::Instance {
                    instance_id: card.id.clone(),
                }
            })
        } else if mode.is_some_and(|mode| mode.target_enemies) {
            prefer_enemies(
                sink.state,
                player,
                &now,
                |card: &CardInstance| Selection::Instance {
                    instance_id: card.id.clone(),
                },
                1,
            )
        } else {
            now
        };
        if pool.is_empty() {
            return false;
        }
        let at = sink.rng.int(pool.len() as i32);
        let Some(card) = pool.get(at as usize).cloned() else {
            return false;
        };
        for _ in 0..count {
            place_plague_on(sink, &card, tokens);
        }
        return false;
    }
    let options: Vec<PromptOption> = cards
        .iter()
        .map(|card| PromptOption {
            key: format!("instance:{}", card.id),
            label: label_of(sink.state, card),
            selection: Selection::Instance {
                instance_id: card.id.clone(),
            },
            cost: None,
            radiant: None,
        })
        .collect();
    let prompt = if total == 1 {
        "Place a Plague Counter on a permanent".to_string()
    } else {
        format!("Place {total} Plague Counters on a permanent")
    };
    let asked = open_prompt(
        sink,
        OpenPromptArgs {
            player,
            kind: PromptKind::Target,
            aim: None,
            prompt,
            options,
            min: None,
            max: None,
            budget: None,
            owner: None,
            resume,
        },
    );
    asked.is_some()
}

/// `placePlagueTokens`'s arguments: `amount` (each placement's tokens) defaults to 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueTokensArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
}

/// "Place N Plague Counters" (B5 E19, R689; Classic #61, #70, #76, #90 reward D): `count` placements,
/// all on the one permanent of either side, face-down included, that the controller picks in a single
/// prompt. The other player sees only that a prompt is open (§10.6, R81).
pub fn place_plague_tokens(args: PlacePlagueTokensArgs) -> Effect {
    Effect::new("placePlagueTokens", move |ctx| {
        let count = args.count;
        if count <= 0 {
            return;
        }
        let def_id = ctx
            .self_
            .as_ref()
            .map(|me| me.def_id.clone())
            .or_else(|| ctx.def_id.clone())
            .unwrap_or_default();
        let mut request = json!({
            "defId": def_id,
            "hook": PLAGUE_PLACEMENT_HOOK,
            "step": "place",
            "radiant": ctx.radiant,
            "data": { "amount": args.amount.unwrap_or(1).max(1), "count": count },
        });
        if let Some(me) = &ctx.self_ {
            request["instanceId"] = json!(me.id);
        }
        let resume: Resume = resume_at(json_as(request));
        let controller = ctx.controller;
        ask_placement(ctx, controller, resume);
    })
}

/// R122, R689: the answer to the placement prompt: validated, closed, every placement made on the
/// pick, then what the prompt interrupted resumes (R113).
pub fn answer_placement(sink: &mut EngineSink<'_>, answer: &AnswerInput) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };
    why_answer_refused(&pending, answer)?;
    let resume = resume_of(&pending.resume);
    let Some(data) = placement_data(&resume.data) else {
        return Err(EngineError::new("that prompt carries no placement"));
    };
    let picked = in_offered_order(&pending, &answer.selection).into_iter().next();

    close_prompt(sink);
    begin_work_cascade(sink);
    let card = match &picked {
        Some(Selection::Instance { instance_id }) => find_instance(sink.state, instance_id).cloned(),
        _ => None,
    };
    if let Some(card) = card {
        let times = data.count.max(1);
        for _ in 0..times {
            place_plague_on(sink, &card, data.amount);
        }
    }

    drain_work(sink);
    Ok(())
}
