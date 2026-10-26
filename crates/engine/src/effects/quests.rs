//! Quest verbs (issue #1854, R1854000, R1854001, R1854004): the effects a staged quest's own text and
//! Stinkyscape #6 Tylannt write with. The line itself — its stages, its counts, its completion at the
//! state check and its view — is `subsystems::quests`'; these only move it, as `subsystems::open_quest`
//! and `hold_quest_aura` move In Too Deep's.

use serde::{Deserialize, Serialize};

use crate::script::Effect;
use crate::subsystems::quests::{
    complete_quests_of, complete_this_quest_now, current_stage_of, hear_back_on, open_stage_now,
    quest_card_of,
};

/// R1854001: "Complete the current stage on all your Quests" (Stinkyscape #6 Tylannt): every card with
/// the Quest tag acting on its controller's side of the field has the quests open on it completed —
/// a staged quest's current stage, In Too Deep's open quests — each noticed at the next state check as
/// any completion is, its reward given then. A line hearing back, or done, has none open.
pub fn complete_quests() -> Effect {
    Effect::new("completeQuests", move |ctx| {
        let player = ctx.controller;
        complete_quests_of(ctx, player);
    })
}

/// R1854001: the running card completes its own current stage (a `byEffect` goal, such as Job
/// Application's "send a card from your hand in as your resume", its Activate's last effect).
pub fn complete_this_quest() -> Effect {
    Effect::new("completeThisQuest", complete_this_quest_now)
}

/// R1854000: the running card's staged line opens stage `stage` (1 for the first): its open quests
/// close, and the stage opens afresh even when it was done before ("return to Stage 1").
pub fn open_stage(stage: u32) -> Effect {
    Effect::new("openStage", move |ctx| open_stage_now(ctx, stage))
}

/// R1854000: the running card's staged line opens the stage after the one it stands at — a stage's
/// reward's last effect. Nothing after the last stage: the line is done.
pub fn next_stage() -> Effect {
    Effect::new("nextStage", move |ctx| {
        let Some(card) = quest_card_of(ctx) else {
            return;
        };
        let Some(stage) = current_stage_of(&card) else {
            return;
        };
        open_stage_now(ctx, stage + 1);
    })
}

/// `hear_back`'s arguments: the turns to wait, and the stage the line hears back with (the one after
/// the stage it stands at when absent).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HearBackArgs {
    pub turns: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<u32>,
}

/// R1854004: "Hear back in N turns" (Stinkyscape #112 Job Application's first reward): the running
/// card's staged line waits, and its next stage opens at the start of its controller's Nth turn from
/// now, before that turn's start-of-turn triggers and draw; at once for N of 0 or less. The view shows
/// the turns left on the stage it waits for.
pub fn hear_back(args: HearBackArgs) -> Effect {
    Effect::new("hearBack", move |ctx| {
        let Some(card) = quest_card_of(ctx) else {
            return;
        };
        let Some(stage) = args
            .stage
            .or_else(|| current_stage_of(&card).map(|stage| stage + 1))
        else {
            return;
        };
        hear_back_on(ctx, args.turns, stage);
    })
}
