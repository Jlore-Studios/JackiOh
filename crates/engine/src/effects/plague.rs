//! Plague Counters as verbs (SPEC §6.3 Plague Counter; docs/classic-sets.md B5 E19; R471).
//!
//! The counter, the multiplier and the `counterChanged` report are `crate::plague`'s; this file names
//! where a placement goes and asks the questions:
//!   - `place_plague`: "Place N Plague Counters on X" — ONE placement of N on the card named (Classic #39
//!     Outbreak's target, #59 Plague Doctor's "on this", Classic+ #3's end of turn, #87's "enters with
//!     X");
//!   - `place_plague_each`: one placement on each permanent a board scope names (Classic #63 Crop
//!     Dusting's "on each permanent", face-down ones included);
//!   - `place_plague_random`: one placement on each of N different random cards of a scope (Classic #42
//!     Transmutable Toxins, R60);
//!   - `place_plague_tokens`: "Place N Plague Counters" with no card named — N placements, all on the one
//!     permanent the placer chooses in a single prompt of their own over every permanent on the field,
//!     either side, face-down included (R689; Classic #61, #70, #76, #90 reward D);
//!   - `consume_plague`: take tokens off (Classic #78 Mutate Spell).
//!
//! PROMPTS (R113, R122, R689). `place_plague_tokens` opens one `target` prompt naming the single
//! permanent all of its placements go on. The prompt is opened by the effect, so the list it stands
//! in parks its rest on `state.work` as any asking effect's does; the answer places every placement
//! on the pick and drains what the prompt interrupted (`answer_placement`, which `prompts.rs` calls
//! for this module's hook, SURFACE §6.6 — TS registered it with `registerPromptAnswerer`). The resume
//! carries only how many tokens each placement puts and how many placements land, so a paused chain
//! is plain data that survives JSON and replays exactly. The state check waits for the whole effect
//! (R59), so a unit the placements shrank to 0 health (an aura reading its tokens, #42) dies after the
//! last placement, not between two.
//!
//! Port of `packages/engine/src/effects/plague.ts`.

use std::borrow::Borrow;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::def_of;
use crate::effects::targets::{BoardScope, TargetSpec, cards_in_scope, instance_of};
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

/// An owned copy of a lookup's answer, whether the lookup lent the card or handed over a copy.
fn owned<C: Borrow<CardInstance>>(card: C) -> CardInstance {
    card.borrow().clone()
}

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
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    pub amount: i32,
}

/// "Place N Plague Counters on X": one placement of `amount` on the card `target` names (default the
/// running card), multiplied by that card's multiplier (R471). Nothing happens for a card that is not
/// a permanent on the field, or for an amount below 1.
pub fn place_plague(args: PlacePlagueArgs) -> Effect {
    Effect::new("placePlague", move |ctx| {
        let spec = args.target.clone().unwrap_or_else(self_spec);
        let card = instance_of(ctx, &spec).map(owned);
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
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueEachArgs {
    pub scope: BoardScope,
    pub amount: i32,
}

/// One placement of `amount` on each card a board scope names, in R68's order (`cards_in_scope`). Crop
/// Dusting's "each permanent" is `{ side: "any", rows: ["units", "backrow"] }`, which reaches
/// face-down cards, as every backrow card is in a scope's backrow row.
pub fn place_plague_each(args: PlacePlagueEachArgs) -> Effect {
    Effect::new("placePlagueEach", move |ctx| {
        let cards: Vec<CardInstance> = cards_in_scope(ctx, &args.scope).into_iter().map(owned).collect();
        for card in &cards {
            place_plague_on(ctx, card, args.amount);
        }
    })
}

/// `placePlagueRandom`'s arguments: the scope defaults to every unit on both sides.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueRandomArgs {
    pub count: i32,
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<BoardScope>,
}

/// One placement of `amount` on each of `count` different random cards of a scope (Classic #42's "a
/// Plague Counter on each of 2 random Units"): R60's random pick of N existing cards picks N different
/// ones, or all of them if fewer exist. The cards are drawn with `ctx.rng` and placed in R68's order.
/// An empty scope draws nothing (R129).
pub fn place_plague_random(args: PlacePlagueRandomArgs) -> Effect {
    Effect::new("placePlagueRandom", move |ctx| {
        let scope: BoardScope = match &args.scope {
            Some(scope) => scope.clone(),
            None => json_as(json!({ "side": "any", "rows": ["units"] })),
        };
        let pool: Vec<CardInstance> = cards_in_scope(ctx, &scope).into_iter().map(owned).collect();
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
#[serde(rename_all = "camelCase")]
pub struct ConsumePlagueArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
}

/// Take up to `amount` Plague Counters (default 1) off the card `target` names: "remove a Plague Counter
/// from a permanent" (Classic #78). The count floors at 0; a card with none changes nothing.
pub fn consume_plague(args: ConsumePlagueArgs) -> Effect {
    Effect::new("consumePlague", move |ctx| {
        let spec = args.target.clone().unwrap_or_else(self_spec);
        let Some(card) = instance_of(ctx, &spec).map(owned) else {
            return;
        };
        remove_plague(ctx, &card, args.amount.unwrap_or(1));
    })
}

// ---------------------------------------------------------------------------
// "Place N Plague Counters": N placements, each a prompt (R471, R113, R122).
// ---------------------------------------------------------------------------

/// The hook every placement prompt names, answered by `answer_placement` below (R122).
pub const PLAGUE_PLACEMENT_HOOK: &str = "plague:placement";

/// What the one placement prompt carries to its answer: each placement's tokens and how many land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlacementData {
    amount: i32,
    count: i32,
}

/// TS `typeof x === "number"`: a JSON number, as the integer the data bag was written with.
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
    def_of(state, &card.def_id).name.clone()
}

/// Open the one placement prompt for `resume`'s placer over every permanent on the field (R68's
/// order, the placer's side first), or return false when there is none — the placements fizzle
/// then, and draw nothing (R129).
fn ask_placement(sink: &mut EngineSink<'_>, player: PlayerId, resume: Resume) -> bool {
    let cards: Vec<CardInstance> = permanents_on_field(sink.state, player).into_iter().map(owned).collect();
    if cards.is_empty() {
        return false;
    }
    let data = placement_data(&resume.data);
    let tokens = data.map_or(1, |data| data.amount);
    let count = data.map_or(1, |data| data.count).max(1);
    let total = tokens * count;
    // B5 E12, R452: a random cast's caster is never asked, so under one every placement goes on one
    // random permanent (R60; an enemy one when the cast targets enemies) and nothing pauses —
    // this hook answers its own prompts (`prompts.rs` calls `answer_placement` for it), so
    // `open_prompt` would ask instead.
    let mode = cast_mode_for_prompt(sink.state, player, resume.instance_id.as_deref());
    if let Some(mode) = mode
        && mode.random
    {
        let now: Vec<CardInstance> = permanents_on_field(sink.state, player).into_iter().map(owned).collect();
        let pool: Vec<CardInstance> = if mode.target_enemies {
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
#[serde(rename_all = "camelCase")]
pub struct PlacePlagueTokensArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
}

/// "Place N Plague Counters" (B5 E19, R471, R689): `count` placements of `amount` (default 1), all on
/// the one permanent the running card's controller chooses — either side, face-down cards included —
/// in a single prompt. The prompt opens as this effect applies; its answer places every placement on
/// the pick, then what the prompt interrupted resumes. With no permanent on the field the placements
/// do nothing. The other player sees only that a prompt is open (§10.6, R81).
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

/// R122, R689: the answer to the one placement prompt — validated as any prompt's, closed, every
/// placement made on the pick (a card that is no longer a permanent on the field takes nothing),
/// then what the prompt interrupted resumes (R113).
///
/// TS registered this with `registerPromptAnswerer(PLAGUE_PLACEMENT_HOOK, …)`; here `prompts.rs`
/// calls it by name for that hook (SURFACE §6.6). A refusal is TS's text verbatim.
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
