//! R4592000: face-up play history (Infinite Assault, Scraper).
//!
//! A player's ordered face-up plays this game, written only while the Infinite Assault set is open
//! (R982 gate). Casts count (R70). A countered play (R448) or a face-down set (R451) never does.
//! AI cards are kept, unlike R451's skip.

use serde::{Deserialize, Serialize};

use crate::catalog::set_is_open;
use crate::state::GameState;
use crate::wire::{CardType, PlayerId, SetName};

/// R4592000: a face-up play's record in the play history.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceUpPlay {
    pub def_id: String,
    pub radiant: bool,
    #[serde(rename = "type")]
    pub type_: CardType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub chinese: Option<bool>,
    pub turn: i32,
}

pub fn scraper_records_open() -> bool {
    set_is_open(SetName::InfiniteAssault)
}

pub(crate) fn record_face_up_play(state: &mut GameState, player: PlayerId, play: FaceUpPlay) {
    if !scraper_records_open() {
        return;
    }
    let log = state.players[player]
        .game_log
        .get_or_insert_with(Default::default);
    let history = log.face_up_plays.get_or_insert_with(Vec::new);
    history.push(play);
}

pub fn face_up_plays(state: &GameState, player: PlayerId) -> Vec<FaceUpPlay> {
    state.players[player]
        .game_log
        .as_ref()
        .and_then(|log| log.face_up_plays.as_ref())
        .cloned()
        .unwrap_or_default()
}

pub fn last_plays(state: &GameState, player: PlayerId, count: usize) -> Vec<FaceUpPlay> {
    let plays = face_up_plays(state, player);
    if count == 0 || plays.is_empty() {
        return Vec::new();
    }
    let take_count = count.min(plays.len());
    plays[plays.len() - take_count..].to_vec()
}

pub fn last_unit_played(state: &GameState, player: PlayerId) -> Option<FaceUpPlay> {
    face_up_plays(state, player)
        .into_iter()
        .rev()
        .find(|p| p.type_ == CardType::Unit)
}

pub fn plays_last_turn(state: &GameState, player: PlayerId) -> Vec<FaceUpPlay> {
    let last_turn = state.turn - 1;
    face_up_plays(state, player)
        .into_iter()
        .filter(|p| p.turn == last_turn)
        .collect()
}
