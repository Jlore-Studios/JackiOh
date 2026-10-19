//! After a Spell targeting an enemy resolves (Meditative+ #17 Dogpile, issue #1180, R1180005).
//!
//! "In your hand or deck: After another Spell of yours that targeted an enemy resolves, cast this at
//! it." Two halves, both here:
//!
//!  1. **The targets kept.** While a Spell resolves, the enemies it targets are noted on the card, in
//!     the order it targeted them (`TARGETED_KEY` in its memory): its declared targets and a cast's
//!     choices as its script runs with them (§10.5 step 5), each Echo repeat's fresh picks (step 6), and
//!     the picks of a `target` prompt its own script opens (§10.6). An enemy is the opponent's hero or a
//!     Unit acting on the opponent's side. Nothing is noted unless the player holds, in their hand or
//!     deck, a card that answers such Spells (`answers_targeted_spells`), so a game without one keeps
//!     its state, and its hashes, as before (D9).
//!  2. **The answer.** At §10.5 step 7 the notes are taken off the card, and `cardResolved` names the one
//!     card that answers the resolution (`answer`): the leftmost such card in the player's hand, else
//!     the topmost in their deck, cast at the first enemy the Spell targeted that is still an enemy on
//!     the field (the hero always is). With no enemy left, or no answering card, nothing is named. The
//!     answering card's hand and deck triggers (R464) read the event and cast it, aimed, at that enemy
//!     (`effects::cast` with `aimOnly`); every other copy reads that it is not the one and does nothing,
//!     so one card answers each Spell. Its own cast is a Spell that targeted an enemy, so it can call the
//!     next. `answer` is engine bookkeeping: a view never forwards it (§10.8), since it names a hidden
//!     card.
//!  3. **The chain's bound** (R1180006). The player's `answerChain` holds the card last named and how
//!     many answers the chain has named. Its resolution continues the chain, any other resolution starts
//!     a new one, and once SPELL_ANSWER_CHAIN_CAP answers are named the next names none: a deck that
//!     refills itself with answering cards, aimed at an enemy that never falls, would answer forever.

use serde_json::Value;

use crate::config::SPELL_ANSWER_CHAIN_CAP;
use crate::state::{AnswerChain, CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CardType, PlayerId, Selection, SpellAnswer, Zone, opponent_of};

/// R1180005: where a resolving Spell keeps the enemies it targeted, in its memory (§10.1), until step 7
/// takes them off.
pub const TARGETED_KEY: &str = "@targeted";

/// R1180005: whether this card answers its owner's Spells that targeted an enemy from where it lies.
fn answers(state: &GameState, card: &CardInstance) -> bool {
    crate::scripts::flags_of(state, card).answers_targeted_spells == Some(true)
}

/// R1180005: whether this player holds, in their hand or deck, a card that answers their Spells that
/// targeted an enemy — the only time a resolving Spell's targets are kept.
pub fn listens_for_targeted_spells(state: &GameState, player: PlayerId) -> bool {
    let side = &state.players[player];
    side.hand
        .iter()
        .chain(side.library.iter())
        .any(|card| answers(state, card))
}

/// R1180005: whether `pick` names an enemy of `player` now — the opponent's hero, or a Unit acting on
/// the opponent's side of the field.
pub fn is_enemy_target(state: &GameState, player: PlayerId, pick: &Selection) -> bool {
    match pick {
        Selection::Hero { player: hero } => *hero != player,
        Selection::Instance { instance_id } => crate::zones::active_units_of(state, opponent_of(player))
            .iter()
            .any(|unit| unit.id == *instance_id),
        _ => false,
    }
}

/// R1180005: note the enemies among `picks` on the Spell resolving as `card_id`, after any it noted
/// before, each once. Nothing for a card that is not a Spell in its player's resolving zone (a Field
/// Spell, a Trap, a permanent's Cry, a Spell set face-down), nor while that player holds nothing that
/// answers such Spells.
pub fn note_spell_targets(state: &mut GameState, card_id: &str, picks: &[Selection]) {
    if picks.is_empty() {
        return;
    }
    let Some(card) = find_instance(state, card_id) else {
        return;
    };
    let Zone::Resolving { player } = card.zone else {
        return;
    };
    if crate::faces::card_type_of(state, card) != CardType::Spell
        || !listens_for_targeted_spells(state, player)
    {
        return;
    }
    let mut noted: Vec<Selection> = noted_targets(card);
    for pick in picks {
        if is_enemy_target(state, player, pick) && !noted.contains(pick) {
            noted.push(pick.clone());
        }
    }
    if noted.is_empty() {
        return;
    }
    let value = serde_json::to_value(&noted).unwrap_or(Value::Null);
    if let Some(card) = find_instance_mut(state, card_id) {
        card.memory.insert(TARGETED_KEY.to_string(), value);
    }
}

/// The enemies a card has noted, in order.
fn noted_targets(card: &CardInstance) -> Vec<Selection> {
    card.memory
        .get(TARGETED_KEY)
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

/// R1180005, §10.5 step 7: take the enemies a resolution noted off its card, before the landing resets
/// it (R78). Empty when it noted none.
pub fn take_spell_targets(state: &mut GameState, card_id: &str) -> Vec<Selection> {
    let Some(card) = find_instance_mut(state, card_id) else {
        return Vec::new();
    };
    let noted = noted_targets(card);
    card.memory.shift_remove(TARGETED_KEY);
    noted
}

/// R1180005: the card that answers `player`'s resolution of `resolved_id`, which targeted `targeted`,
/// and the enemy it is cast at — the first of them that is still an enemy, and the leftmost answering
/// card in the hand, else the topmost in the deck. `None` with no enemy left or no card to answer.
pub fn spell_answer_for(
    state: &GameState,
    player: PlayerId,
    resolved_id: &str,
    targeted: &[Selection],
) -> Option<SpellAnswer> {
    let at = targeted
        .iter()
        .find(|pick| is_enemy_target(state, player, pick))?;
    let side = &state.players[player];
    let by = side
        .hand
        .iter()
        .chain(side.library.iter())
        .find(|card| card.id != resolved_id && answers(state, card))?;
    Some(SpellAnswer {
        by: by.id.clone(),
        at: at.clone(),
    })
}

/// R1180005, R1180006: name the answer to `player`'s resolution of `resolved_id` and walk the chain.
/// The resolution of the card last named continues its chain; any other starts a new one. Once the
/// chain has named SPELL_ANSWER_CHAIN_CAP answers, the next resolution in it names none, and the chain
/// ends with the first resolution that names nothing.
pub fn name_spell_answer(
    state: &mut GameState,
    player: PlayerId,
    resolved_id: &str,
    targeted: &[Selection],
) -> Option<SpellAnswer> {
    let depth = match &state.players[player].answer_chain {
        Some(chain) if chain.by == resolved_id => chain.depth,
        _ => 0,
    };
    let answer = if targeted.is_empty() || depth >= SPELL_ANSWER_CHAIN_CAP {
        None
    } else {
        spell_answer_for(state, player, resolved_id, targeted)
    };
    state.players[player].answer_chain = answer.as_ref().map(|named| AnswerChain {
        by: named.by.clone(),
        depth: depth + 1,
    });
    answer
}

/// R1180005, R946: whether `option` is the pick an aimed cast names — an instance by its id, or any
/// pick by its prompt key (`prompts::option_key`, "hero:p2" for a hero).
pub fn aims_at_pick(option: &Selection, aimed: &str) -> bool {
    matches!(option, Selection::Instance { instance_id } if instance_id == aimed)
        || crate::prompts::option_key(option) == aimed
}
