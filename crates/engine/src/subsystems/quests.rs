//! Quests (docs/classic-sets.md B5 E33): the machinery of Classic #90 In Too Deep (SPEC §8.6 row 90,
//! §10.1, §10.6, §10.8; R404). The tree (which quests there are, what completes each, which rewards
//! each offers and where each leads) is data the card's script declares (`Script.quests`, a
//! `QuestBook`); the rewards are ordinary verbs in the card file, and the card's own trigger answers
//! `questCompleted` with them: a `reward` prompt on the base face (§10.6), every reward on the Radiant
//! face. This module is what no card file may do: keep the count.
//!
//! State (§10.1) is the card's instance, `memory.quest` (`QuestMemory`): plain JSON, so a paused state
//! survives a round trip and a log folds to the same hash (§9.3). R78 resets memory as a card leaves
//! the field, which is the whole of "leaving the field resets the quest line".
//!
//! Counting starts "from the moment it opens" (R404): events dispatch in emit order, so a quest a reward
//! opens waits for its own opening report, and an event from before a card's arrival never reaches it (R212).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::def_of;
use crate::faces::card_type_of;
use crate::layers::unit_view;
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::stays::moves_in;
use crate::wire::{
    CardType, GameEvent, GameEventType, PLAYER_IDS, PlayerId, QuestItemView, QuestOpenView, QuestView, Row,
    ZoneName, opponent_of,
};
use crate::zones::{active_units_of, card_at, slots_of};

// The tree, as a card declares it

// What completes a quest (R404's countable readings): `QuestGoal` (`script.rs`, which describes each
// goal). Counted goals move as events are dispatched, and a draw a limit stopped or a fatigue draw
// counts for nothing (R541); board goals are read off the board at each state check.
pub use crate::script::{QuestBook, QuestDef, QuestGoal, QuestRewardDef};

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
        | QuestGoal::UnitsInGraveyard { count } => *count,
        QuestGoal::DamageToEnemies { amount } => *amount,
        QuestGoal::UnitTotals { total } => *total,
        QuestGoal::UnspentManaAtTurnEnd { .. } | QuestGoal::DeckEmptiedByDraw => 1,
    }
}

/// The quest tree a card's running face declares, or `None` (a Vanilla card has none, R115). It reads
/// the card's script, which needs the state (SURFACE §6.6).
pub fn quest_book_of(state: &GameState, card: &CardInstance) -> Option<QuestBook> {
    crate::scripts::script_of(state, card).quests.clone()
}

pub fn quest_def_of<'b>(book: &'b QuestBook, id: &str) -> Option<&'b QuestDef> {
    book.quests.iter().find(|quest| quest.id == id)
}

pub fn quest_reward_of<'b>(book: &'b QuestBook, id: &str) -> Option<&'b QuestRewardDef> {
    book.rewards.iter().find(|reward| reward.id == id)
}

// The quest line on the instance (§10.1)

/// Where the quest line lives on the instance (`memory.quest`, §10.1).
pub const QUEST_MEMORY_KEY: &str = "quest";

/// `memory.quest`. All JSON.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuestMemory {
    /// The open quests, in the order they opened.
    pub active: Vec<String>,
    /// Each counted quest's count so far, capped at its goal.
    pub progress: IndexMap<String, i32>,
    /// The completed quests, in the order they were completed.
    pub done: Vec<String>,
    /// The auras held (rewards L and M), in the order they were granted.
    pub auras: Vec<String>,
    /// Quests a reward opened whose opening report the loop has not dispatched yet: not counting.
    pub waiting: Vec<String>,
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

/// A finite number in the bag.
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
        // `Object.entries` of an array: its indices as keys.
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
    })
}

fn write_memory(card: &mut CardInstance, memory: &QuestMemory) {
    card.memory.insert(
        QUEST_MEMORY_KEY.to_string(),
        serde_json::to_value(memory).unwrap_or(Value::Null),
    );
}

/// `write_memory` on the card as it stands in the state.
fn store_memory(state: &mut GameState, card_id: &str, memory: &QuestMemory) {
    if let Some(card) = find_instance_mut(state, card_id) {
        write_memory(card, memory);
    }
}

/// The open quests of a card, in the order they opened.
pub fn open_quests_of(card: &CardInstance) -> Vec<String> {
    quest_memory_of(card)
        .map(|memory| memory.active)
        .unwrap_or_default()
}

/// The quests a card has completed, in order.
pub fn completed_quests_of(card: &CardInstance) -> Vec<String> {
    quest_memory_of(card)
        .map(|memory| memory.done)
        .unwrap_or_default()
}

/// The auras a card's quest line holds (rewards L and M): a pure read for its `aura` and `graveyard_play`.
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

/// Quest 3: the permanents `player` controls now — the tops of their unit piles and their backrow cards.
fn permanents_controlled(state: &GameState, player: PlayerId) -> i32 {
    (active_units_of(state, player).len() + backrow_cards_of(state, player).len()) as i32
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

/// A quest's progress now: its count, or what the board comes to, capped at its goal.
pub fn quest_progress_of(state: &GameState, card: &CardInstance, quest: &QuestDef) -> i32 {
    let goal = quest_goal_of(quest);
    let value = if is_board_goal(&quest.goal) {
        board_value(state, card.controller, &quest.goal)
    } else {
        quest_memory_of(card)
            .and_then(|memory| memory.progress.get(&quest.id).copied())
            .unwrap_or(0)
    };
    goal.min(value).max(0)
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

/// Open `id` on the card `card_id` names unless it was ever opened (a quest reached by two paths opens
/// once, R404). `live`: counting at once (the first quest, as the card enters); otherwise it waits for
/// its own opening report to be dispatched. Quest 9's "a deck already empty when the quest opens
/// completes it at once" is its count met as it opens.
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

/// The first quest of a card on the field that has no quest line yet: it opens as the card enters.
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
    open_on(sink, card_id, &book.first, true);
}

/// Open the running card's quest `id` — a reward's next quest (R404). Counting from the moment it
/// opens: the events emitted before it in this action do not count (`QuestMemory.waiting`). Nothing
/// for a card that has left the field since its run began (R174: its quest line went with it, R78), for
/// a quest the tree does not have, or for one already opened.
pub fn open_quest(id: impl Into<String>) -> Effect {
    let id: String = id.into();
    Effect::new("openQuest", move |ctx| {
        let Some(self_) = quest_card_of(ctx) else {
            return;
        };
        open_on(ctx, &self_.id, &id, false);
    })
}

/// Hold a reward's aura on the running card (In Too Deep's L and M): it holds while the card is on the
/// field, and leaving the field ends it (R78). The card's own `aura` and `graveyard_play` hooks read it
/// back (`held_quest_auras`).
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

/// The running card, on the field on the stay its run began with (R174), with a quest tree — as it
/// stands in the state now.
fn quest_card_of(ctx: &EffectContext<'_>) -> Option<CardInstance> {
    let on_stay = crate::effects::targets::self_on_its_stay(ctx)?;
    let self_ = find_instance(ctx.sink.state, &on_stay.id)?.clone();
    if !crate::zones::acts_on_field(ctx.sink.state, &self_) || quest_book_of(ctx.sink.state, &self_).is_none()
    {
        return None;
    }
    Some(self_)
}

// Counting, as the loop dispatches each event

/// Every card with a quest tree acting on the field, in R68's order: the active side first.
fn quest_cards_in_order(state: &GameState) -> Vec<CardInstance> {
    let sides: [PlayerId; 2] = if state.active == PlayerId::P1 {
        [PlayerId::P1, PlayerId::P2]
    } else {
        [PlayerId::P2, PlayerId::P1]
    };
    let mut out: Vec<CardInstance> = Vec::new();
    for player in sides {
        let units: Vec<CardInstance> = active_units_of(state, player).into_iter().cloned().collect();
        let backrow: Vec<CardInstance> = backrow_cards_of(state, player).into_iter().cloned().collect();
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

/// R212: the side a card or hero stood on when an event happened, read off the events that followed
/// it — a change of control since hands back the controller before it, and a card destroyed since died
/// under the controller its `destroyed` names — and otherwise off the card where it is now. `None` for
/// a card gone with nothing to say (a replaced card).
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

/// What one event adds to a quest of `goal` for a card controlled by `me`, or 0. `after` is the events
/// that followed this one.
fn credit_of(
    state: &GameState,
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
        _ => 0,
    }
}

/// The event types a counted goal reads, so an event no quest counts costs no board read.
const COUNTED_EVENTS: [GameEventType; 5] = [
    GameEventType::Drawn,
    GameEventType::Destroyed,
    GameEventType::TurnEnded,
    GameEventType::Exiled,
    GameEventType::Damage,
];

/// §10.3, R404: one event reaching the resolution loop, before the traps and the triggers see it: every
/// open, counting quest of every card on the field that the event counts for moves, and says so
/// (`questProgressed`). `after` is the events that followed this one (`triggers::events_after_dispatched`),
/// read once before anything here writes: a card that has moved since did not see the event (R212).
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
        // R212: a card that arrived after the event, or came back since, is on a stay that did not see it.
        if moves.moved.contains(&card.id) {
            continue;
        }
        open_first_if_new(sink, &card.id);
        if !COUNTED_EVENTS.contains(&event.event_type()) {
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
            let before = memory.progress.get(&id).copied().unwrap_or(0);
            if before >= goal {
                continue;
            }
            let credit = credit_of(sink.state, &quest.goal, event, me, &later);
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

// Completion, at the state check

fn complete(state: &GameState, card: &CardInstance, quest: &QuestDef, memory: &QuestMemory) -> bool {
    if is_board_goal(&quest.goal) {
        return board_value(state, card.controller, &quest.goal) >= quest_goal_of(quest);
    }
    memory.progress.get(&quest.id).copied().unwrap_or(0) >= quest_goal_of(quest)
}

/// §4.5, R404: the state check has settled the board, so every card with a quest tree on the field opens
/// its first quest if it has none yet, and every open quest whose count reached its goal or whose board
/// condition holds now is completed — moved to `done` and reported by `questCompleted`, in R68's order
/// and each card's quests in the order they opened. Each completion runs its own rewards, even one
/// another completed quest offers.
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

/// R404, §10.8: the open quests with their progress and the rewards on offer, and the auras held, or
/// `None` for a card with no quest line. Everything here is the tree's own text and public counts, so
/// it rides on every view of the card (a face-up Field Spell both players read).
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
    })
}
