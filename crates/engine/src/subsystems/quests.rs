//! Quests (docs/classic-sets.md B5 E33): the machinery of Classic #90 In Too Deep (SPEC §8.6 row 90,
//! §10.1, §10.6, §10.8; R404). The tree is data the card's script declares (`Script.quests`); the
//! rewards are ordinary verbs its own `questCompleted` trigger returns (§10.6). This module counts.
//!
//! State (§10.1) is `memory.quest`, plain JSON so a log folds to the same hash (§9.3); R78 resets it
//! as the card leaves the field. Counting starts as a quest opens (R404); an event from before a card's
//! arrival never reaches it (R212).
//!
//! #1854 (R1854000–R1854004): staged quests (`Script.stages`), one stage open at a time. Every field it
//! adds to `QuestMemory` is skipped while empty, so In Too Deep's line hashes and replays as ever.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::def_of;
use crate::config::FIRST_QUEST_STAGE;
use crate::faces::card_type_of;
use crate::layers::unit_view;
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, GameState, active_first, find_instance, find_instance_mut};
use crate::stays::moves_in;
use crate::wire::{
    CardType, GameEvent, GameEventType, PLAYER_IDS, PlayerId, QuestItemView, QuestOpenView, QuestStageState,
    QuestStageView, QuestView, Row, Tag, ZoneName, opponent_of,
};
use crate::zones::{active_units_of, card_at, slots_of};

// The tree, as a card declares it

// Counted goals (R404) move as events dispatch (a draw a limit stopped or a fatigue draw counts
// nothing, R541); board goals are read at each state check.
pub use crate::script::{
    QuestBook, QuestDef, QuestGoal, QuestRewardDef, QuestStage, QuestStageGoal, QuestStageReward, staged_book,
};

/// The goals the state check reads off the board rather than counting.
fn is_board_goal(goal: &QuestGoal) -> bool {
    matches!(
        goal,
        QuestGoal::PermanentsControlled { .. }
            | QuestGoal::UnitTotals { .. }
            | QuestGoal::UnitsInGraveyard { .. }
    )
}

/// The number a quest's progress is shown against ("1/2"). A goal met once is 1.
pub fn quest_goal_of(quest: &QuestDef) -> i32 {
    match &quest.goal {
        QuestGoal::Draws { count }
        | QuestGoal::EnemyPermanentsDestroyed { count }
        | QuestGoal::CardsExiled { count }
        | QuestGoal::PermanentsControlled { count }
        | QuestGoal::UnitsInGraveyard { count }
        | QuestGoal::UnitsMadeRadiant { count, .. }
        | QuestGoal::UnitsDied { count, .. }
        | QuestGoal::CardsStolen { count }
        | QuestGoal::DifferentCostsInTurn { count } => *count,
        QuestGoal::DamageToEnemies { amount } => *amount,
        QuestGoal::UnitTotals { total } => *total,
        QuestGoal::UnspentManaAtTurnEnd { .. }
        | QuestGoal::DeckEmptiedByDraw
        | QuestGoal::PlayCosting { .. }
        | QuestGoal::DamageInstance { .. }
        | QuestGoal::ByEffect => 1,
    }
}

/// The quest tree a card's running face declares (none on a Vanilla card, R115; reading the script
/// needs the state, SURFACE §6.6). R1854000: stages and no tree run on their compiled book.
pub fn quest_book_of(state: &GameState, card: &CardInstance) -> Option<QuestBook> {
    let script = crate::scripts::script_of(state, card);
    if let Some(book) = script.quests.as_ref() {
        return Some(book.clone());
    }
    if script.stages.is_empty() {
        None
    } else {
        Some(staged_book(&script.stages))
    }
}

/// R1854000: a staged quest's stages, in order, or none for a tree or no quest.
pub fn quest_stages_of(state: &GameState, card: &CardInstance) -> Vec<QuestStage> {
    let script = crate::scripts::script_of(state, card);
    if script.quests.is_some() {
        return Vec::new();
    }
    script.stages.clone()
}

pub fn quest_def_of<'b>(book: &'b QuestBook, id: &str) -> Option<&'b QuestDef> {
    book.quests.iter().find(|quest| quest.id == id)
}

pub fn quest_reward_of<'b>(book: &'b QuestBook, id: &str) -> Option<&'b QuestRewardDef> {
    book.rewards.iter().find(|reward| reward.id == id)
}

/// R1854000: the stage (`FIRST_QUEST_STAGE` for the first) whose goals hold quest `id`, or `None`.
pub fn stage_of_quest(stages: &[QuestStage], id: &str) -> Option<u32> {
    stages
        .iter()
        .position(|stage| stage.goals.iter().any(|goal| goal.id == id))
        .map(|at| at as u32 + FIRST_QUEST_STAGE)
}

fn stage_def(stages: &[QuestStage], stage: u32) -> Option<&QuestStage> {
    stage
        .checked_sub(FIRST_QUEST_STAGE)
        .and_then(|at| stages.get(at as usize))
}

// The quest line on the instance (§10.1)

pub const QUEST_MEMORY_KEY: &str = "quest";

/// R1854004: a staged line waiting to hear back: `stage` opens once `turns` more of its controller's
/// turns have started.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HearBack {
    pub turns: i32,
    pub stage: u32,
}

/// R1854002: the costs a `differentCostsInTurn` quest has counted, and their turn; a record of an
/// earlier turn counts nothing.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TurnCosts {
    pub turn: i32,
    pub costs: Vec<i32>,
}

/// `memory.quest`. All JSON.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuestMemory {
    /// The open quests, in the order they opened.
    pub active: Vec<String>,
    /// Each counted quest's count so far, capped at its goal.
    pub progress: IndexMap<String, i32>,
    /// The completed quests, in completion order.
    pub done: Vec<String>,
    /// The auras held (rewards L and M), in grant order.
    pub auras: Vec<String>,
    /// Quests a reward opened whose opening report is not dispatched yet: not counting.
    pub waiting: Vec<String>,
    /// R1854000: the stage a staged line stands at (1 for the first); absent on a tree's line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<u32>,
    /// R1854004: the stage the line hears back with, and the turns until it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hear_back: Option<HearBack>,
    /// R1854001: open quests an effect has completed, which the next state check notices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub met: Vec<String>,
    /// R1854002: each open `differentCostsInTurn` quest's costs this turn.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub costs: IndexMap<String, TurnCosts>,
}

fn string_list(raw: Option<&Value>) -> Vec<String> {
    match raw {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn finite_count(value: &Value) -> Option<i32> {
    value
        .as_f64()
        .filter(|number| number.is_finite())
        .map(|number| number as i32)
}

/// The card's quest line, read back defensively (it may have come through JSON), or `None`.
pub fn quest_memory_of(card: &CardInstance) -> Option<QuestMemory> {
    let raw = card.memory.get(QUEST_MEMORY_KEY)?;
    // An object, or an array (whose named fields are all absent).
    if !raw.is_object() && !raw.is_array() {
        return None;
    }
    let mut progress: IndexMap<String, i32> = IndexMap::new();
    match raw.get("progress") {
        Some(Value::Object(entries)) => {
            for (id, value) in entries {
                if let Some(count) = finite_count(value) {
                    progress.insert(id.clone(), count);
                }
            }
        }
        // An array: its indices as keys.
        Some(Value::Array(items)) => {
            for (at, value) in items.iter().enumerate() {
                if let Some(count) = finite_count(value) {
                    progress.insert(at.to_string(), count);
                }
            }
        }
        _ => {}
    }
    Some(QuestMemory {
        active: string_list(raw.get("active")),
        progress,
        done: string_list(raw.get("done")),
        auras: string_list(raw.get("auras")),
        waiting: string_list(raw.get("waiting")),
        stage: raw
            .get("stage")
            .and_then(finite_count)
            .filter(|stage| *stage > 0)
            .map(|stage| stage as u32),
        hear_back: raw
            .get("hearBack")
            .and_then(|value| serde_json::from_value::<HearBack>(value.clone()).ok()),
        met: string_list(raw.get("met")),
        costs: raw
            .get("costs")
            .and_then(|value| serde_json::from_value::<IndexMap<String, TurnCosts>>(value.clone()).ok())
            .unwrap_or_default(),
    })
}

fn write_memory(card: &mut CardInstance, memory: &QuestMemory) {
    card.memory.insert(
        QUEST_MEMORY_KEY.to_string(),
        serde_json::to_value(memory).unwrap_or(Value::Null),
    );
}

fn store_memory(state: &mut GameState, card_id: &str, memory: &QuestMemory) {
    if let Some(card) = find_instance_mut(state, card_id) {
        write_memory(card, memory);
    }
}

/// The auras the quest line holds (rewards L and M), for the card's `aura` and `graveyard_play`.
pub fn held_quest_auras(card: &CardInstance) -> Vec<String> {
    quest_memory_of(card)
        .map(|memory| memory.auras)
        .unwrap_or_default()
}

// The board goals, read now

fn backrow_cards_of(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    slots_of(player, Row::Backrow)
        .iter()
        .filter_map(|slot| card_at(state, slot))
        .collect()
}

/// Quest 3: the permanents `player` controls now: their unit piles' tops and backrow cards.
fn permanents_controlled(state: &GameState, player: PlayerId) -> i32 {
    // R1081000: a Unit in the backrow is among the active units already; count it once.
    let units = active_units_of(state, player);
    let backrow = backrow_cards_of(state, player)
        .into_iter()
        .filter(|card| !units.iter().any(|unit| unit.id == card.id))
        .count();
    (units.len() + backrow) as i32
}

/// Quest 6: the lower of `player`'s Units' total attack and total health, through the layers (§10.4).
fn unit_totals(state: &GameState, player: PlayerId) -> i32 {
    let mut attack = 0;
    let mut health = 0;
    for unit in active_units_of(state, player).iter() {
        let view = unit_view(state, unit);
        attack += view.attack;
        health += view.health.max(0);
    }
    attack.min(health)
}

/// Quest 10: the Units in `player`'s graveyard, each by the type it has there (B2.7).
fn units_in_graveyard(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .graveyard
        .iter()
        .filter(|card| card_type_of(state, card) == CardType::Unit)
        .count() as i32
}

fn board_value(state: &GameState, player: PlayerId, goal: &QuestGoal) -> i32 {
    match goal {
        QuestGoal::PermanentsControlled { .. } => permanents_controlled(state, player),
        QuestGoal::UnitTotals { .. } => unit_totals(state, player),
        QuestGoal::UnitsInGraveyard { .. } => units_in_graveyard(state, player),
        _ => 0,
    }
}

fn costs_this_turn(state: &GameState, memory: &QuestMemory, id: &str) -> i32 {
    memory
        .costs
        .get(id)
        .filter(|record| record.turn == state.turn)
        .map_or(0, |record| record.costs.len() as i32)
}

/// A quest's progress now: its count, or what the board comes to, capped at its goal.
pub fn quest_progress_of(state: &GameState, card: &CardInstance, quest: &QuestDef) -> i32 {
    let goal = quest_goal_of(quest);
    let value = if is_board_goal(&quest.goal) {
        board_value(state, card.controller, &quest.goal)
    } else if matches!(quest.goal, QuestGoal::DifferentCostsInTurn { .. }) {
        quest_memory_of(card).map_or(0, |memory| costs_this_turn(state, &memory, &quest.id))
    } else {
        quest_memory_of(card)
            .and_then(|memory| memory.progress.get(&quest.id).copied())
            .unwrap_or(0)
    };
    goal.min(value).max(0)
}

/// R1854002: the cost a `playCosting` quest's play must pay: the card's memory under `remembered`
/// while it holds one, else the goal's `cost`.
pub fn quest_cost_of(card: &CardInstance, goal: &QuestGoal) -> Option<i32> {
    let QuestGoal::PlayCosting { cost, remembered } = goal else {
        return None;
    };
    let held = remembered
        .as_ref()
        .and_then(|key| card.memory.get(key))
        .and_then(finite_count);
    Some(held.unwrap_or(*cost))
}

// Opening a quest

fn report(sink: &mut EngineSink<'_>, card: &CardInstance, quest: &QuestDef, progress: i32) {
    sink.events.push(GameEvent::QuestProgressed {
        player: card.controller,
        instance_id: card.id.clone(),
        quest: quest.id.clone(),
        progress,
        goal: quest_goal_of(quest),
    });
}

/// Open `id` on `card_id` unless it was ever opened (a quest reached by two paths opens once, R404).
/// `live`: counting at once (the first quest); otherwise it waits for its own opening report. Quest
/// 9's "a deck already empty when it opens" is its count met at once.
fn open_on(sink: &mut EngineSink<'_>, card_id: &str, id: &str, live: bool) {
    let Some(mut card) = find_instance(sink.state, card_id).cloned() else {
        return;
    };
    let Some(book) = quest_book_of(sink.state, &card) else {
        return;
    };
    let Some(quest) = quest_def_of(&book, id).cloned() else {
        return;
    };
    let mut memory = quest_memory_of(&card).unwrap_or_default();
    if memory.active.iter().any(|open| open == id) || memory.done.iter().any(|done| done == id) {
        return;
    }
    memory.active.push(id.to_string());
    if !is_board_goal(&quest.goal) {
        let empty_deck = matches!(quest.goal, QuestGoal::DeckEmptiedByDraw)
            && sink.state.players[card.controller].library.is_empty();
        memory
            .progress
            .insert(id.to_string(), if empty_deck { quest_goal_of(&quest) } else { 0 });
    }
    if !live {
        memory.waiting.push(id.to_string());
    }
    write_memory(&mut card, &memory);
    store_memory(sink.state, card_id, &memory);
    let progress = quest_progress_of(sink.state, &card, &quest);
    report(sink, &card, &quest, progress);
}

/// R1854000: open stage `stage` on `card_id`. One stage is open at a time, so the open quests close
/// unfinished and a wait to hear back ends; its goals are forgotten (a return opens it afresh) and one
/// opens, drawn from the match rng if several. `live` as `open_on`'s.
fn open_stage_on(sink: &mut EngineSink<'_>, card_id: &str, stage: u32, live: bool) {
    let Some(card) = find_instance(sink.state, card_id).cloned() else {
        return;
    };
    let stages = quest_stages_of(sink.state, &card);
    let Some(def) = stage_def(&stages, stage).cloned() else {
        return;
    };
    let mut memory = quest_memory_of(&card).unwrap_or_default();
    let closing: Vec<String> = memory.active.drain(..).collect();
    let goals: Vec<String> = def.goals.iter().map(|goal| goal.id.clone()).collect();
    for id in closing.iter().chain(goals.iter()) {
        memory.progress.shift_remove(id);
        memory.costs.shift_remove(id);
    }
    memory.waiting.clear();
    memory.met.clear();
    memory.done.retain(|id| !goals.contains(id));
    memory.hear_back = None;
    memory.stage = Some(stage);
    store_memory(sink.state, card_id, &memory);
    let pick = match goals.len() {
        0 => None,
        1 => goals.first().cloned(),
        _ => sink.rng.pick(&goals).cloned(),
    };
    if let Some(id) = pick {
        open_on(sink, card_id, &id, live);
    }
}

/// A card on the field with no quest line yet opens its first quest (or stage, R1854000).
fn open_first_if_new(sink: &mut EngineSink<'_>, card_id: &str) {
    let Some(card) = find_instance(sink.state, card_id).cloned() else {
        return;
    };
    if quest_memory_of(&card).is_some() {
        return;
    }
    let Some(book) = quest_book_of(sink.state, &card) else {
        return;
    };
    store_memory(sink.state, card_id, &QuestMemory::default());
    if quest_stages_of(sink.state, &card).is_empty() {
        open_on(sink, card_id, &book.first, true);
    } else {
        open_stage_on(sink, card_id, FIRST_QUEST_STAGE, true);
    }
}

/// Open the running card's quest `id`, a reward's next quest, counting from the moment it opens (R404,
/// `QuestMemory.waiting`). Nothing for a card that has left the field since its run began (R174, R78).
pub fn open_quest(id: impl Into<String>) -> Effect {
    let id: String = id.into();
    Effect::new("openQuest", move |ctx| {
        let Some(self_) = quest_card_of(ctx) else {
            return;
        };
        open_on(ctx, &self_.id, &id, false);
    })
}

/// Hold a reward's aura on the running card (In Too Deep's L and M) until it leaves the field (R78); its
/// `aura` and `graveyard_play` hooks read it back (`held_quest_auras`).
pub fn hold_quest_aura(reward_id: impl Into<String>) -> Effect {
    let reward_id: String = reward_id.into();
    Effect::new("holdQuestAura", move |ctx| {
        let Some(self_) = quest_card_of(ctx) else {
            return;
        };
        let Some(mut memory) = quest_memory_of(&self_) else {
            return;
        };
        if memory.auras.contains(&reward_id) {
            return;
        }
        memory.auras.push(reward_id.clone());
        store_memory(ctx.sink.state, &self_.id, &memory);
    })
}

/// The running card as it stands now, if on the field on its run's stay (R174) with a quest tree.
pub(crate) fn quest_card_of(ctx: &EffectContext<'_>) -> Option<CardInstance> {
    let on_stay = crate::effects::targets::self_on_its_stay(ctx)?;
    let self_ = find_instance(ctx.sink.state, &on_stay.id)?.clone();
    if !crate::zones::acts_on_field(ctx.sink.state, &self_) || quest_book_of(ctx.sink.state, &self_).is_none()
    {
        return None;
    }
    Some(self_)
}

// Moving a staged line on (R1854000, R1854004): the card's own reward effects call these
// (`effects::open_stage`, `next_stage`, `hear_back`).

pub(crate) fn current_stage_of(card: &CardInstance) -> Option<u32> {
    quest_memory_of(card).and_then(|memory| memory.stage)
}

/// R1854000: open stage `stage` of the running card's staged line (`effects::open_stage`).
pub(crate) fn open_stage_now(ctx: &mut EffectContext<'_>, stage: u32) {
    let Some(card) = quest_card_of(ctx) else {
        return;
    };
    open_stage_on(ctx, &card.id, stage, false);
}

/// R1854004: the running card's staged line hears back with `stage` once `turns` of its controller's
/// turns have started (at once for 0 or less); a later wait replaces an earlier one.
pub(crate) fn hear_back_on(ctx: &mut EffectContext<'_>, turns: i32, stage: u32) {
    let Some(card) = quest_card_of(ctx) else {
        return;
    };
    if stage_def(&quest_stages_of(ctx.sink.state, &card), stage).is_none() {
        return;
    }
    if turns <= 0 {
        open_stage_on(ctx, &card.id, stage, false);
        return;
    }
    let mut memory = quest_memory_of(&card).unwrap_or_default();
    memory.hear_back = Some(HearBack { turns, stage });
    store_memory(ctx.sink.state, &card.id, &memory);
}

/// R1854004: at the start of `player`'s turn (R62's Brittle stage, after the Dormant countdowns), each
/// of their lines waiting to hear back counts a turn down, in R68's order; one whose wait ends opens.
pub fn hear_back_at_turn_start(sink: &mut EngineSink<'_>, player: PlayerId) {
    for card in quest_cards_in_order(sink.state) {
        if card.controller != player || !crate::zones::acts_on_field(sink.state, &card) {
            continue;
        }
        let Some(mut memory) = quest_memory_of(&card) else {
            continue;
        };
        let Some(wait) = memory.hear_back else {
            continue;
        };
        if wait.turns > 1 {
            memory.hear_back = Some(HearBack {
                turns: wait.turns - 1,
                stage: wait.stage,
            });
            store_memory(sink.state, &card.id, &memory);
        } else {
            open_stage_on(sink, &card.id, wait.stage, false);
        }
    }
}

/// R1854000: the stage the running card's own `questCompleted` names (`FIRST_QUEST_STAGE` for the
/// first), whose reward its trigger returns; `None` for another card's report or an unstaged quest.
pub fn completed_stage_of(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<u32> {
    let GameEvent::QuestCompleted {
        instance_id, quest, ..
    } = event
    else {
        return None;
    };
    let card = ctx.self_.as_ref()?;
    if card.id != *instance_id {
        return None;
    }
    stage_of_quest(&quest_stages_of(ctx.sink.state, card), quest)
}

// Completing by an effect (R1854001)

/// R1854001: mark every quest open on `card_id` met, for the next state check (`notice_quests`). A
/// card with no line yet (summoned this moment) opens its first quest or stage first.
fn complete_open_on(sink: &mut EngineSink<'_>, card_id: &str) {
    open_first_if_new(sink, card_id);
    let Some(card) = find_instance(sink.state, card_id).cloned() else {
        return;
    };
    let Some(mut memory) = quest_memory_of(&card) else {
        return;
    };
    let mut changed = false;
    for id in memory.active.clone() {
        if !memory.met.contains(&id) {
            memory.met.push(id);
            changed = true;
        }
    }
    if changed {
        store_memory(sink.state, card_id, &memory);
    }
}

/// R1854001: "Complete the current stage on all your Quests" (Stinkyscape #6 Tylannt): each face-up
/// Quest-tagged card with a quest (a tree or staged) acting on `player`'s side, in lane order.
pub(crate) fn complete_quests_of(sink: &mut EngineSink<'_>, player: PlayerId) {
    let cards: Vec<String> = quest_cards_in_order(sink.state)
        .into_iter()
        .filter(|card| {
            card.controller == player
                && crate::zones::acts_on_field(sink.state, card)
                && !crate::preview::is_face_down(sink.state, card)
                && crate::query::tags_of(sink.state, card).contains(&Tag::Quest)
        })
        .map(|card| card.id)
        .collect();
    for id in cards {
        complete_open_on(sink, &id);
    }
}

/// R1854001: the running card's own open quests completed, whatever its tags (a `byEffect` goal's
/// "Activate: …", Job Application's resume).
pub(crate) fn complete_this_quest_now(ctx: &mut EffectContext<'_>) {
    let Some(card) = quest_card_of(ctx) else {
        return;
    };
    complete_open_on(ctx, &card.id);
}

// Counting, as the loop dispatches each event

/// Every card with a quest tree acting on the field, in R68's order: the active side first.
fn quest_cards_in_order(state: &GameState) -> Vec<CardInstance> {
    let sides = active_first(state);
    let mut out: Vec<CardInstance> = Vec::new();
    for player in sides {
        let units: Vec<CardInstance> = active_units_of(state, player).into_iter().cloned().collect();
        // R1081000: visit a backrow Unit once.
        let backrow: Vec<CardInstance> = backrow_cards_of(state, player)
            .into_iter()
            .filter(|card| !units.iter().any(|unit| unit.id == card.id))
            .cloned()
            .collect();
        for card in units.into_iter().chain(backrow) {
            if quest_book_of(state, &card).is_some() {
                out.push(card);
            }
        }
    }
    out
}

fn hero_side(id: &str) -> Option<PlayerId> {
    PLAYER_IDS
        .into_iter()
        .find(|player| id == format!("hero-{}", player.as_str()))
}

/// R212: the side a card or hero stood on when an event happened: off a later change of control or
/// `destroyed`, else off the card where it is now.
fn side_when(state: &GameState, id: &str, after: &[GameEvent]) -> Option<PlayerId> {
    if let Some(hero) = hero_side(id) {
        return Some(hero);
    }
    for event in after {
        match event {
            GameEvent::ControlChanged {
                instance_id,
                controller,
                ..
            } if instance_id == id => return Some(opponent_of(*controller)),
            GameEvent::Destroyed {
                instance_id,
                controller,
                ..
            } if instance_id == id => return Some(*controller),
            _ => {}
        }
    }
    if let Some(card) = find_instance(state, id) {
        return Some(match card.zone.z() {
            ZoneName::Field => card.controller,
            ZoneName::Resolving => card.zone.player(),
            _ => card.owner,
        });
    }
    // A unit token that has ceased to exist is in no pile (R11): its leaving named its owner.
    for event in after {
        match event {
            GameEvent::Bounced {
                instance_id, owner, ..
            }
            | GameEvent::Exiled {
                instance_id, owner, ..
            } if instance_id == id => return Some(*owner),
            _ => {}
        }
    }
    None
}

/// A unit token ceases to exist rather than entering an exile pile (R11), so it is not an exiled card.
fn entered_exile(state: &GameState, def_id: &str) -> bool {
    let def = def_of(Some(state), def_id);
    !(def.token && def.type_ == CardType::Unit)
}

/// R1854002: whether the card an event names is a Unit with `tag` (any Unit for none), by its type and
/// tags now, or its definition's once it has ceased to exist (a unit token, R11).
fn is_unit_tagged(state: &GameState, instance_id: &str, def_id: &str, tag: Option<Tag>) -> bool {
    let (card_type, tags) = match find_instance(state, instance_id) {
        Some(card) => (card_type_of(state, card), crate::query::tags_of(state, card)),
        None => {
            let def = def_of(Some(state), def_id);
            (def.type_, def.tags.clone())
        }
    };
    card_type == CardType::Unit && tag.is_none_or(|tag| tags.contains(&tag))
}

/// What one event adds to a quest of `goal` on a card `me` controls; `after` is the events since.
fn credit_of(
    state: &GameState,
    card: &CardInstance,
    goal: &QuestGoal,
    event: &GameEvent,
    me: PlayerId,
    after: &[GameEvent],
) -> i32 {
    match (goal, event) {
        (QuestGoal::Draws { .. }, GameEvent::Drawn { player, .. }) => i32::from(*player == me),
        (QuestGoal::DeckEmptiedByDraw, GameEvent::Drawn { player, emptied, .. }) => {
            i32::from(*player == me && *emptied == Some(true))
        }
        (QuestGoal::EnemyPermanentsDestroyed { .. }, GameEvent::Destroyed { controller, .. }) => {
            i32::from(*controller == opponent_of(me))
        }
        (
            QuestGoal::UnspentManaAtTurnEnd { mana },
            GameEvent::TurnEnded {
                player, unspent_mana, ..
            },
        ) => i32::from(*player == me && *unspent_mana >= *mana),
        (QuestGoal::CardsExiled { .. }, GameEvent::Exiled { def_id, .. }) => {
            i32::from(entered_exile(state, def_id))
        }
        (
            QuestGoal::DamageToEnemies { .. },
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                ..
            },
        ) => {
            let Some(source_id) = source_id else {
                return 0;
            };
            if *amount <= 0 {
                return 0;
            }
            if side_when(state, source_id, after) != Some(me) {
                return 0;
            }
            if side_when(state, target_id, after) == Some(opponent_of(me)) {
                *amount
            } else {
                0
            }
        }
        // R1854002: a Unit of yours with the tag made Radiant, in any zone of yours.
        (
            QuestGoal::UnitsMadeRadiant { tag, .. },
            GameEvent::RadiantSet {
                instance_id,
                def_id,
                zone,
            },
        ) => i32::from(zone.player() == me && is_unit_tagged(state, instance_id, def_id, *tag)),
        // R1854002, R172: a Unit with the tag that died under your control.
        (
            QuestGoal::UnitsDied { tag, .. },
            GameEvent::Destroyed {
                instance_id,
                def_id,
                controller,
                ..
            },
        ) => i32::from(*controller == me && is_unit_tagged(state, instance_id, def_id, *tag)),
        // R1854002, R56, R70: a card of yours played (a cast included) at the cost the goal names.
        (
            QuestGoal::PlayCosting { .. },
            GameEvent::CardPlayed {
                player, cost_paid, ..
            },
        ) => i32::from(*player == me && quest_cost_of(card, goal) == Some(*cost_paid)),
        // R1854002: one instance of at least `amount` your hero took, or a card or hero of yours dealt.
        (
            QuestGoal::DamageInstance { amount: least },
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                ..
            },
        ) => {
            if *amount < *least || *amount <= 0 {
                return 0;
            }
            let dealt = source_id
                .as_ref()
                .is_some_and(|source| side_when(state, source, after) == Some(me));
            let taken = hero_side(target_id) == Some(me);
            i32::from(dealt || taken)
        }
        _ => 0,
    }
}

/// The event types a counted goal reads, so an event no quest counts costs no board read.
const COUNTED_EVENTS: [GameEventType; 7] = [
    GameEventType::Drawn,
    GameEventType::Destroyed,
    GameEventType::TurnEnded,
    GameEventType::Exiled,
    GameEventType::Damage,
    GameEventType::RadiantSet,
    GameEventType::CardPlayed,
];

/// R1854002: a play of yours joins a `differentCostsInTurn` quest's costs this turn; `Some(progress)`
/// when it moved.
fn count_play_cost(
    state: &GameState,
    memory: &mut QuestMemory,
    id: &str,
    event: &GameEvent,
    me: PlayerId,
) -> Option<i32> {
    let GameEvent::CardPlayed {
        player, cost_paid, ..
    } = event
    else {
        return None;
    };
    if *player != me {
        return None;
    }
    let record = memory.costs.entry(id.to_string()).or_default();
    if record.turn != state.turn {
        *record = TurnCosts {
            turn: state.turn,
            costs: Vec::new(),
        };
    }
    if record.costs.contains(cost_paid) {
        return None;
    }
    record.costs.push(*cost_paid);
    Some(record.costs.len() as i32)
}

/// §10.3, R404: one event reaching the resolution loop, before traps and triggers, moves every open
/// quest it counts for (`questProgressed`). `after` (`triggers::events_after_dispatched`) is read
/// before anything here writes: a card that has moved since did not see the event (R212).
pub fn observe_quest_event(
    sink: &mut EngineSink<'_>,
    event: &GameEvent,
    after: &dyn Fn(&EngineSink<'_>) -> Vec<GameEvent>,
) {
    let cards = quest_cards_in_order(sink.state);
    if cards.is_empty() {
        return;
    }

    if let GameEvent::QuestProgressed {
        instance_id, quest, ..
    } = event
    {
        let card = cards.iter().find(|candidate| candidate.id == *instance_id);
        if let Some(card) = card
            && let Some(mut memory) = quest_memory_of(card)
            && memory.waiting.contains(quest)
        {
            memory.waiting.retain(|id| id != quest);
            store_memory(sink.state, &card.id, &memory);
        }
        return;
    }

    let later: Vec<GameEvent> = after(sink);
    let moves = moves_in(&later, Some(&*sink.state));
    for card in &cards {
        if moves.moved.contains(&card.id) {
            continue;
        }
        open_first_if_new(sink, &card.id);
        if !COUNTED_EVENTS.contains(&event.event_type()) {
            continue;
        }
        // R1854002: the quest card's own play came before its quest opened (R404).
        if matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == card.id) {
            continue;
        }
        let Some(live) = find_instance(sink.state, &card.id).cloned() else {
            continue;
        };
        let Some(book) = quest_book_of(sink.state, &live) else {
            continue;
        };
        let Some(mut memory) = quest_memory_of(&live) else {
            continue;
        };
        let me = moves
            .controller_before
            .get(&card.id)
            .copied()
            .unwrap_or(live.controller);
        let mut changed = false;
        for id in memory.active.clone() {
            if memory.waiting.contains(&id) {
                continue;
            }
            let Some(quest) = quest_def_of(&book, &id) else {
                continue;
            };
            if is_board_goal(&quest.goal) {
                continue;
            }
            let goal = quest_goal_of(quest);
            if matches!(quest.goal, QuestGoal::DifferentCostsInTurn { .. }) {
                if costs_this_turn(sink.state, &memory, &id) >= goal {
                    continue;
                }
                if let Some(progress) = count_play_cost(sink.state, &mut memory, &id, event, me) {
                    changed = true;
                    report(sink, &live, quest, goal.min(progress));
                }
                continue;
            }
            let before = memory.progress.get(&id).copied().unwrap_or(0);
            if before >= goal {
                continue;
            }
            let credit = credit_of(sink.state, &live, &quest.goal, event, me, &later);
            if credit <= 0 {
                continue;
            }
            let progress = goal.min(before + credit);
            memory.progress.insert(id.clone(), progress);
            changed = true;
            report(sink, &live, quest, progress);
        }
        if changed {
            store_memory(sink.state, &live.id, &memory);
        }
    }
}

/// R1854002: a steal (`stolen`, or `controlChanged` with `how: steal`) moves `thief`'s `cardsStolen`
/// quests here: the loop reads events without their `how` (`GameEvent::as_rules_read`, R1366).
pub fn note_steal(sink: &mut EngineSink<'_>, thief: PlayerId) {
    for card in quest_cards_in_order(sink.state) {
        if card.controller != thief || !crate::zones::acts_on_field(sink.state, &card) {
            continue;
        }
        let Some(book) = quest_book_of(sink.state, &card) else {
            continue;
        };
        let Some(mut memory) = quest_memory_of(&card) else {
            continue;
        };
        let mut changed = false;
        for id in memory.active.clone() {
            let Some(quest) = quest_def_of(&book, &id) else {
                continue;
            };
            if !matches!(quest.goal, QuestGoal::CardsStolen { .. }) {
                continue;
            }
            let goal = quest_goal_of(quest);
            let before = memory.progress.get(&id).copied().unwrap_or(0);
            if before >= goal {
                continue;
            }
            memory.progress.insert(id.clone(), before + 1);
            changed = true;
            report(sink, &card, quest, before + 1);
        }
        if changed {
            store_memory(sink.state, &card.id, &memory);
        }
    }
}

// Completion, at the state check

fn complete(state: &GameState, card: &CardInstance, quest: &QuestDef, memory: &QuestMemory) -> bool {
    // R1854001: an effect completed it.
    if memory.met.contains(&quest.id) {
        return true;
    }
    if is_board_goal(&quest.goal) {
        return board_value(state, card.controller, &quest.goal) >= quest_goal_of(quest);
    }
    if matches!(quest.goal, QuestGoal::DifferentCostsInTurn { .. }) {
        return costs_this_turn(state, memory, &quest.id) >= quest_goal_of(quest);
    }
    memory.progress.get(&quest.id).copied().unwrap_or(0) >= quest_goal_of(quest)
}

/// §4.5, R404: once the state check has settled the board, each open quest whose goal is met moves to
/// `done` and is reported (`questCompleted`), in R68's order, each card's in opening order. Each
/// completion runs its own rewards, so a reward two completed quests offer is granted by each.
pub fn notice_quests(sink: &mut EngineSink<'_>) {
    for card in quest_cards_in_order(sink.state) {
        open_first_if_new(sink, &card.id);
        let Some(live) = find_instance(sink.state, &card.id).cloned() else {
            continue;
        };
        let Some(book) = quest_book_of(sink.state, &live) else {
            continue;
        };
        let Some(memory) = quest_memory_of(&live) else {
            continue;
        };
        let completed: Vec<String> = memory
            .active
            .iter()
            .filter(|id| {
                quest_def_of(&book, id).is_some_and(|quest| complete(sink.state, &live, quest, &memory))
            })
            .cloned()
            .collect();
        if completed.is_empty() {
            continue;
        }
        let mut done = memory.done.clone();
        done.extend(completed.iter().cloned());
        let next = QuestMemory {
            active: memory
                .active
                .iter()
                .filter(|id| !completed.contains(id))
                .cloned()
                .collect(),
            done,
            waiting: memory
                .waiting
                .iter()
                .filter(|id| !completed.contains(id))
                .cloned()
                .collect(),
            met: memory
                .met
                .iter()
                .filter(|id| !completed.contains(id))
                .cloned()
                .collect(),
            costs: memory
                .costs
                .iter()
                .filter(|(id, _)| !completed.contains(id))
                .map(|(id, record)| (id.clone(), record.clone()))
                .collect(),
            ..memory.clone()
        };
        store_memory(sink.state, &live.id, &next);
        for quest in completed {
            sink.events.push(GameEvent::QuestCompleted {
                player: live.controller,
                instance_id: live.id.clone(),
                quest,
            });
        }
    }
}

// The view (§10.8)

/// R1854000: each stage of a staged line: done, open now, or to come (the one it hears back with
/// carrying the turns left, R1854004).
fn stages_view(stages: &[QuestStage], memory: &QuestMemory) -> Vec<QuestStageView> {
    let current = memory.stage.unwrap_or(FIRST_QUEST_STAGE);
    stages
        .iter()
        .enumerate()
        .map(|(at, stage)| {
            let number = at as u32 + FIRST_QUEST_STAGE;
            let ids: Vec<&str> = stage.goals.iter().map(|goal| goal.id.as_str()).collect();
            let open: Vec<&str> = ids
                .iter()
                .copied()
                .filter(|id| memory.active.iter().any(|active| active == id))
                .collect();
            let finished: Vec<&str> = ids
                .iter()
                .copied()
                .filter(|id| memory.done.iter().any(|done| done == id))
                .collect();
            let hear_back = memory
                .hear_back
                .filter(|wait| wait.stage == number)
                .map(|wait| wait.turns);
            let state = if hear_back.is_some() || number > current {
                QuestStageState::Upcoming
            } else if !open.is_empty() {
                QuestStageState::Current
            } else {
                QuestStageState::Done
            };
            // The goal drawn for the stage once it has opened; every goal it may draw before.
            let shown: Vec<&str> = match state {
                QuestStageState::Current => open,
                QuestStageState::Done if !finished.is_empty() => finished,
                _ => ids,
            };
            QuestStageView {
                name: stage.name.clone(),
                state,
                goals: stage
                    .goals
                    .iter()
                    .filter(|goal| shown.contains(&goal.id.as_str()))
                    .map(|goal| goal.text.clone())
                    .collect(),
                reward: stage.reward.text.clone(),
                hear_back,
            }
        })
        .collect()
}

/// R404, §10.8: the open quests with their progress and rewards, and the auras held. All of it is the
/// tree's text and public counts, so it rides on every view of the card. R1854000 adds a staged line's
/// stages, R1854002 a `playCosting` quest's cost.
pub fn quest_view_of(state: &GameState, card: &CardInstance) -> Option<QuestView> {
    let book = quest_book_of(state, card)?;
    let memory = quest_memory_of(card)?;
    if !crate::zones::acts_on_field(state, card) {
        return None;
    }
    let reward_text = |id: &str| -> String {
        quest_reward_of(&book, id)
            .map(|reward| reward.text.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let stages = quest_stages_of(state, card);
    Some(QuestView {
        open: memory
            .active
            .iter()
            .filter_map(|id| {
                let quest = quest_def_of(&book, id)?;
                Some(QuestOpenView {
                    id: id.clone(),
                    text: quest.text.clone(),
                    progress: quest_progress_of(state, card, quest),
                    goal: quest_goal_of(quest),
                    rewards: quest
                        .rewards
                        .iter()
                        .map(|reward| QuestItemView {
                            id: reward.clone(),
                            text: reward_text(reward),
                        })
                        .collect(),
                    cost: quest_cost_of(card, &quest.goal),
                })
            })
            .collect(),
        auras: memory
            .auras
            .iter()
            .map(|id| QuestItemView {
                id: id.clone(),
                text: reward_text(id),
            })
            .collect(),
        stages: if stages.is_empty() {
            None
        } else {
            Some(stages_view(&stages, &memory))
        },
    })
}
