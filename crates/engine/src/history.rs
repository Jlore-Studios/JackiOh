//! The game history (R4594005, R4594009): every Unit play and every Unit death this game, so
//! Infinite Assault #224, #229 and #230 can read them back.
//!
//! `History` hangs off the game, ungated: it is written in every game, so the cards read deaths
//! and plays from before they arrived. It is presentation-free and hash-free: `replay::hash_state`
//! leaves it out, so no recorded game's hash moves (the hash test guards that). Only public facts
//! are logged — definitions, faces and turns — so no view redacts it.
//!
//! A death is one record per death under the card's controller as it died (R172), tokens included;
//! a discard is no death. A play is every resolved Unit play as `record_play` saw it (casts count,
//! R70; countered plays never get here, R448); a Cloaked set is a Trap, not a Unit (R451).

use serde::{Deserialize, Serialize};

use crate::state::{CardInstance, GameState};
use crate::wire::{CardType, PlayerId};

/// R4594009: one Unit play this game: who played it and what it was.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PlayedUnit {
    pub player: PlayerId,
    pub def_id: String,
}

/// R4594005: one Unit death this game: what died, under whose control, on which face (R4577002),
/// and on which player-turn (`GameState.turn`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DeathRecord {
    pub player: PlayerId,
    pub def_id: String,
    pub radiant: bool,
    pub turn: i32,
}

/// R4594005, R4594009: the game's Units played and Units died, oldest first.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct History {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub units_played: Vec<PlayedUnit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deaths: Vec<DeathRecord>,
}

impl History {
    /// Empty, and absent from the JSON, until a Unit is played or dies.
    pub fn is_empty(&self) -> bool {
        self.units_played.is_empty() && self.deaths.is_empty()
    }
}

/// R4594009: log one Unit play of `player`'s, as `record_play` saw it.
pub(crate) fn record_unit_played(state: &mut GameState, player: PlayerId, def_id: &str) {
    state.history.units_played.push(PlayedUnit { player, def_id: def_id.to_string() });
}

/// R4594005: log one Unit's death from its snapshot as it died: its controller, its face and the
/// player-turn. `died_as` decides the type when the printed one does not (a set Cloaked card died
/// as a Trap, R3777000; an Animated one as a Unit, R383); otherwise the snapshot's printed type.
pub(crate) fn record_death(
    state: &mut GameState,
    snapshot: &CardInstance,
    died_as: Option<CardType>,
) {
    let type_ = died_as.unwrap_or_else(|| {
        crate::faces::card_type_of_face(state, &snapshot.def_id, snapshot.radiant)
    });
    if type_ != CardType::Unit {
        return;
    }
    let turn = state.turn;
    state.history.deaths.push(DeathRecord {
        player: snapshot.controller,
        def_id: snapshot.def_id.clone(),
        radiant: snapshot.radiant,
        turn,
    });
}

/// R4594009: every Unit `player` played this game, distinct definitions in first-play order.
pub fn units_played_by(state: &GameState, player: PlayerId) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for played in &state.history.units_played {
        if played.player == player && !seen.contains(&played.def_id) {
            seen.push(played.def_id.clone());
        }
    }
    seen
}

/// R4594005: every Unit that died under `player` this game, oldest first.
pub fn deaths_of(state: &GameState, player: PlayerId) -> Vec<DeathRecord> {
    state.history.deaths.iter().filter(|death| death.player == player).cloned().collect()
}
