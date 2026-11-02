//! Meditative #40 Feng Shui: ME-ELEMENT and ME-LUCK (R980–R987).
//!
//! ME-ELEMENT: every card has the Hetu (河图) element of its index's last digit (R980). Each player's
//! last face-up play is recorded (R982, at §10.5 step 4 by `play_counts::record_play`). At step 3
//! each Feng Shui on the field judges the play from its player's last record (R981): positive when
//! the last element generates the new one, negative when it overcomes it. A positive play is made
//! Radiant there (R983); a negative one is given Brittle 2 there and takes its hit once the play has
//! resolved (R985). A Radiant judge rewards only its controller and punishes only the opponent
//! (R984). Elements are no tags (R986).
//!
//! ME-LUCK: a player's Luck, read through `query::luck_of` (R987). All of it is plain data on the
//! state, so `(seed, decks, …, log)` folds to the same game (§9.3).

use serde::{Deserialize, Serialize};

use crate::config::{
    ELEMENT_WITHOUT_DIGIT, FENG_SHUI_DAMAGE, GENERATING_CYCLE, HETU_ELEMENTS, OVERCOMING_CYCLE,
};
use crate::state::{CardInstance, GameState};
use crate::wire::{CardElement, FengShuiOutcome, PLAYER_IDS, PlayerId, PreviewValue};

/// R983: one Feng Shui's judgement of a play, fixed at §10.5 step 1 and carried on the play's run to
/// steps 3 and 7. `source_id` is the judge; `damage` is the step-7 hit for a `Negative` verdict.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FengShuiVerdict {
    pub source_id: String,
    pub outcome: FengShuiOutcome,
    pub damage: i32,
}

/// R980: the element of a catalog index: its last ASCII digit's Hetu element, or 土 with no digit.
pub fn element_of_index(index: &str) -> CardElement {
    let digit = index
        .bytes()
        .rev()
        .find(|byte| byte.is_ascii_digit())
        .map(|byte| usize::from(byte - b'0'));
    match digit {
        Some(digit) => HETU_ELEMENTS[digit],
        None => ELEMENT_WITHOUT_DIGIT,
    }
}

/// R980: the element of a definition: its first ingredient's for a fused card (R77's order).
pub fn element_of(state: &GameState, def_id: &str) -> CardElement {
    match crate::catalog::fused_id_parts(Some(state), def_id).and_then(|parts| parts.into_iter().next()) {
        Some(first) => element_of(state, &first),
        None => element_of_index(&crate::catalog::def_of(Some(state), def_id).index),
    }
}

/// R981: how `played` answers `last`, the element of the player's last face-up play: `None` for the
/// same element, the reverse relations, and a first play.
pub fn reaction(last: Option<CardElement>, played: CardElement) -> Option<FengShuiOutcome> {
    let last = last?;
    if follows_in(&GENERATING_CYCLE, last, played) {
        Some(FengShuiOutcome::Positive)
    } else if follows_in(&OVERCOMING_CYCLE, last, played) {
        Some(FengShuiOutcome::Negative)
    } else {
        None
    }
}

fn follows_in(cycle: &[CardElement; 5], last: CardElement, played: CardElement) -> bool {
    cycle
        .iter()
        .position(|element| *element == last)
        .is_some_and(|at| cycle[(at + 1) % cycle.len()] == played)
}

/// R982: the element of the player's last face-up play, as `record_element` wrote it.
pub fn last_element(state: &GameState, player: PlayerId) -> Option<CardElement> {
    state.players[player]
        .game_log
        .as_ref()
        .and_then(|log| log.last_element)
}

/// R982–R984: the Feng Shuis judging a play now, acting on either side of the field (§3.2); a
/// face-down one's text is in nobody's use until it fires (R33), so it judges nothing.
fn judges(state: &GameState) -> Vec<CardInstance> {
    let mut out = Vec::new();
    for player in PLAYER_IDS {
        for card in crate::damage::acting_texts_of(state, player) {
            if crate::scripts::flags_of(state, &card).feng_shui == Some(true) {
                out.push(card);
            }
        }
    }
    out
}

/// R982: whether a play is judged: only a face-up one.
pub fn judged(state: &GameState, card: &CardInstance) -> bool {
    !crate::play_counts::played_face_down(crate::faces::card_type_of(state, card))
}

/// R981–R984: every acting Feng Shui's verdict on `player`'s play of `card`, empty for a play none
/// judges; `damage` is the judge's face hit (R985) for a `Negative` verdict, else 0.
pub fn verdicts(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<FengShuiVerdict> {
    if !judged(state, card) {
        return Vec::new();
    }
    let Some(outcome) = reaction(last_element(state, player), element_of(state, &card.def_id)) else {
        return Vec::new();
    };
    judges(state)
        .into_iter()
        .filter(|judge| {
            if !judge.radiant {
                return true;
            }
            match outcome {
                FengShuiOutcome::Positive => judge.controller == player,
                FengShuiOutcome::Negative => judge.controller != player,
            }
        })
        .map(|judge| {
            let damage = match outcome {
                FengShuiOutcome::Positive => 0,
                FengShuiOutcome::Negative => FENG_SHUI_DAMAGE.on(judge.radiant),
            };
            FengShuiVerdict {
                source_id: judge.id.clone(),
                outcome,
                damage,
            }
        })
        .collect()
}

/// R983: whether any Feng Shui makes this play Radiant: step 1 reads it to know the face the play's
/// choices answer, as R214 fixes Gifted Program's, so step 3 applies it without asking the board again.
pub fn rewards(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    verdicts(state, player, card)
        .iter()
        .any(|verdict| verdict.outcome == FengShuiOutcome::Positive)
}

/// R982: record the element of `player`'s face-up play of `card`, beside R451's records but without
/// their AI-card skip: the hidden rule never reveals a hidden card either way, and this is its memory.
pub fn record_element(state: &mut GameState, player: PlayerId, card: &CardInstance) {
    if !judged(state, card) {
        return;
    }
    let element = element_of(state, &card.def_id);
    if let Some(log) = state.players[player].game_log.as_mut() {
        log.last_element = Some(element);
    }
}

/// R980: the element a card's view carries, `Some` only while a Feng Shui acts (D14).
pub fn shown_element(state: &GameState, card: &CardInstance) -> Option<CardElement> {
    if judges(state).is_empty() {
        return None;
    }
    Some(element_of(state, &card.def_id))
}

/// R982: what a "last element" line shows before any play.
pub const NO_ELEMENT_SHOWN: &str = "—";

/// R982: one "last element" `preview` line (R280): `value` the 1-based position in Hetu order
/// (0 for none), `display` the glyph or the dash (R372).
pub fn element_preview(label: &str, element: Option<CardElement>) -> PreviewValue {
    let (value, display) = match element {
        Some(element) => (
            CardElement::ALL
                .iter()
                .position(|known| *known == element)
                .map_or(0, |at| at as i32 + 1),
            element.as_str().to_string(),
        ),
        None => (0, NO_ELEMENT_SHOWN.to_string()),
    };
    PreviewValue {
        label: label.to_string(),
        value,
        display: Some(display),
        ids: None,
    }
}
