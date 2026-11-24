//! The recursion log (R4589000–R4589001, R4589004): what an Infinite Assault Recursion game
//! remembers of its plays and deaths, so IA #99 to #120 can read them back.
//!
//! `RecursionLog` is per player, beside `game_log` (`play_counts.rs`): `played`, every face-up play
//! this game (casts count, R70; countered plays never do, R448; face-down plays never do, R451),
//! each as the play's `FaceUpRecord` (definition, face, type), and `deaths`, the Units that died
//! under that player (R172's controller, tokens included), each with the face it died on
//! (R4577002) and the player-turn it died on.
//!
//! It is kept only while the game holds an Infinite Assault card (`kept`): either side's log is
//! already `Some`, or `every_instance` holds a card whose definition is of the set. Games of
//! shipped sets never hold one, so no recorded game's hash moves (D14, R768). It is not gated on
//! `set_is_open`, which is never true for a single-player set, so the log would stay off in the
//! real Assault. Only public facts are logged, so no view redacts it.

use serde::{Deserialize, Serialize};

use crate::state::{CardInstance, FaceUpRecord, GameState, PlayRecord};
use crate::wire::{CardType, GameEvent, PlayerId, SetName};

/// R4589001: "since your last turn" reaches back this many player-turns: this player-turn and the
/// one before it. In `config` (CLAUDE.md rule 9).
pub use crate::config::SINCE_LAST_TURN;

/// R4589000: one death a player suffered: what died, on which face (R4577002), and on which
/// player-turn (`GameState.turn`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DeathRecord {
    pub def_id: String,
    pub radiant: bool,
    pub turn: i32,
}

/// R4589000: a player's per-game recursion record (`PlayerState.recursion`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RecursionLog {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub played: Vec<FaceUpRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deaths: Vec<DeathRecord>,
}

/// R4589000: whether the game keeps the log: once either side's is `Some`, or the game holds an
/// Infinite Assault card anywhere (hands, libraries and piles included, D14's hashes among them).
fn kept(state: &GameState) -> bool {
    if state.players.p1.recursion.is_some() || state.players.p2.recursion.is_some() {
        return true;
    }
    crate::state::every_instance(state).any(|card| {
        crate::catalog::find_def(Some(state), &card.def_id)
            .is_some_and(|def| def.set == SetName::InfiniteAssault)
    })
}

/// The first write opens the log on both sides at once, so the two reads agree.
fn open(state: &mut GameState) {
    if state.players.p1.recursion.is_none() {
        state.players.p1.recursion = Some(RecursionLog::default());
    }
    if state.players.p2.recursion.is_none() {
        state.players.p2.recursion = Some(RecursionLog::default());
    }
}

/// R4589000: log one face-up play of `player`'s, as `play_record_of` read it. A rejected
/// `set_is_open` gate left out: the Assault's set never opens.
pub fn note_play(state: &mut GameState, player: PlayerId, record: &PlayRecord, type_: CardType) {
    if !kept(state) {
        return;
    }
    open(state);
    if let Some(log) = state.players[player].recursion.as_mut() {
        log.played.push(FaceUpRecord {
            def_id: record.def_id.clone(),
            radiant: record.radiant,
            type_,
            chinese: None,
        });
    }
}

/// R4589000: log one Unit's death under its controller (R172): a `Destroyed` the state check
/// reports as a Unit's death (`query::is_unit_death`), on the face it died on.
pub fn note_death(state: &mut GameState, died: &GameEvent) {
    let GameEvent::Destroyed {
        def_id,
        controller,
        radiant,
        ..
    } = died
    else {
        return;
    };
    if !kept(state) {
        return;
    }
    open(state);
    if let Some(log) = state.players[*controller].recursion.as_mut() {
        log.deaths.push(DeathRecord {
            def_id: def_id.clone(),
            radiant: *radiant == Some(true),
            turn: state.turn,
        });
    }
}

/// R4589000: every face-up play `player` made this game, oldest first; none without a log.
pub fn played_this_game(state: &GameState, player: PlayerId) -> Vec<FaceUpRecord> {
    state.players[player]
        .recursion
        .as_ref()
        .map(|log| log.played.clone())
        .unwrap_or_default()
}

/// R4589000: every Unit that died under `player` this game, oldest first; none without a log.
pub fn deaths_this_game(state: &GameState, player: PlayerId) -> Vec<DeathRecord> {
    state.players[player]
        .recursion
        .as_ref()
        .map(|log| log.deaths.clone())
        .unwrap_or_default()
}

/// R4589001: the Units that died under `player` on this player-turn.
pub fn deaths_this_turn(state: &GameState, player: PlayerId) -> Vec<DeathRecord> {
    deaths_this_game(state, player)
        .into_iter()
        .filter(|death| death.turn == state.turn)
        .collect()
}

/// R4589001: the Units that died under `player` on this player-turn or the one before.
pub fn deaths_since_last_turn(state: &GameState, player: PlayerId) -> Vec<DeathRecord> {
    deaths_this_game(state, player)
        .into_iter()
        .filter(|death| death.turn >= state.turn - SINCE_LAST_TURN)
        .collect()
}

/// R4589004: whether the card came back through Reborn: its return set `reborn_spent`, or a kept
/// Reborn left its marker (R4589003). `Summoned.via` never reaches a trigger (R2783001), so this
/// is how a card knows.
pub fn returned_by_reborn(card: &CardInstance) -> bool {
    card.reborn_spent == Some(true) || card.memory.contains_key(crate::state_check::REBORN_KEPT_KEY)
}
