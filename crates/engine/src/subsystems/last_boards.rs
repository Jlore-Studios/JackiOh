//! Last boards: a match setup input from outside the match (docs/classic-sets.md B5 E30; SPEC §8.7
//! C+ #29 Portal to the Past, §9.3, §10.1, R417, R564).
//!
//! "Your last game" is a game this match never saw, so its board is an INPUT: one list per seat,
//! handed to `create_game` beside the decks and handicaps and frozen into the match
//! (`GameState.last_boards`). Nothing writes it again, so the match folds exactly from
//! `(seed, decks, handicaps, lastBoards, log)` (§9.3, `replay::ReplayInput.last_boards`).
//!
//!  - `freeze_last_boards`: what `create_game` keeps — `{ defId, radiant }` per entry, nothing else, and
//!    only entries this match can rebuild from the id alone (R564). Pure, and it reads the catalog
//!    and the id's text only, so every process freezes the same input the same way.
//!  - `last_board_for`: the reader the server calls for each seat as a game ends.
//!  - `last_board_candidates`: what C+ #29 picks among (R564).
//!
//! `view_for` never sends a last board (it copies what a view may hold and names no such field), and
//! the AI's redaction keeps only its own seat's (`crates/ai/src/observe.rs`, R185).
//!
//! Port of `packages/engine/src/subsystems/lastBoards.ts`. TS's `isEntry` guarded an `unknown[]`
//! input; the frozen `LastBoardInput` is typed (`state.rs`), so every entry already is one and the
//! guard has nothing left to check.

use crate::catalog::{fused_id_specs, is_digest_id};
// R77's smallest fusion, as `subsystems::fuse`'s `FUSE_MIN_INGREDIENTS` (TS kept its own copy, since
// fuse imports state; part 1 moved it to `config.rs`).
use crate::config::FUSED_MIN_PARTS;
use crate::preview::is_face_down;
use crate::state::{CardInstance, GameState, LastBoardEntry, LastBoardInput};
use crate::wire::{CardDefs, PLAYER_IDS, PerPlayerOpt, PlayerId, Row};
use crate::zones::{card_at, carried_at, slots_of};

/// R564: whether this match can have the definition `def_id` names, from the id alone: a catalog card,
/// or a fused id (R179) whose every ingredient, all the way down, is one. A digest id (R468) names its
/// list only to the process that minted it, so it never can.
pub fn rebuildable_from_id(def_id: &str, catalog: &CardDefs) -> bool {
    if catalog.contains_key(def_id) {
        return true;
    }
    if is_digest_id(def_id) {
        return false;
    }
    // No state here (`create_game` freezes before the match exists): a digest id was refused above,
    // so the readable id's own text is all there is to read (part 2's `fused_id_specs(None, …)`).
    let Some(specs) = fused_id_specs(None, def_id) else {
        return false;
    };
    if specs.len() < FUSED_MIN_PARTS {
        return false;
    }
    specs.iter().all(|spec| rebuildable_from_id(&spec.def_id, catalog))
}

/// R417, R564: what `create_game` keeps of its `lastBoards` input — per seat, in order, each entry as
/// `{ defId, radiant }`, every entry this match cannot rebuild dropped. `None` when no seat keeps one.
pub fn freeze_last_boards(
    input: Option<&LastBoardInput>,
    catalog: &CardDefs,
) -> Option<PerPlayerOpt<Vec<LastBoardEntry>>> {
    let input = input?;
    let mut frozen: PerPlayerOpt<Vec<LastBoardEntry>> = PerPlayerOpt::default();
    for (seat, player) in PLAYER_IDS.into_iter().enumerate() {
        let entries: &Vec<LastBoardEntry> = if seat == 0 { &input.0 } else { &input.1 };
        let kept: Vec<LastBoardEntry> = entries
            .iter()
            .filter(|entry| rebuildable_from_id(&entry.def_id, catalog))
            .map(|entry| LastBoardEntry {
                def_id: entry.def_id.clone(),
                radiant: entry.radiant,
            })
            .collect();
        if !kept.is_empty() {
            *frozen.slot(player) = Some(kept);
        }
    }
    if frozen.is_empty() { None } else { Some(frozen) }
}

fn entry_of(card: &CardInstance) -> LastBoardEntry {
    LastBoardEntry {
        def_id: card.def_id.clone(),
        radiant: card.radiant,
    }
}

/// R417: the board `seat` takes away from this game — every card on the field, both sides, in board
/// order (p1's unit lanes, then p1's backrow lanes, a carried Unit before its carrier, R446, then
/// p2's), each as its card and face, except a backrow card `seat` may not read (R33: a face-down Trap
/// or Field Trap it does not control). Dormant cards are not on the field (R13).
pub fn last_board_for(state: &GameState, seat: PlayerId) -> Vec<LastBoardEntry> {
    let mut board: Vec<LastBoardEntry> = Vec::new();
    for player in PLAYER_IDS {
        for slot in slots_of(player, Row::Units) {
            if let Some(top) = card_at(state, &slot) {
                board.push(entry_of(top));
            }
        }
        for slot in slots_of(player, Row::Backrow) {
            if let Some(carried) = carried_at(state, &slot) {
                board.push(entry_of(carried));
            }
            if let Some(card) = card_at(state, &slot)
                && !(is_face_down(state, card) && card.controller != seat)
            {
                board.push(entry_of(card));
            }
        }
    }
    board
}

/// R564: the different cards C+ #29 picks among on `player`'s frozen board — each definition once,
/// where it first appears, on its Radiant face if any of its entries was Radiant — minus `exclude`
/// (R387: the generating card's own definitions). TS's default `exclude = []` is an empty slice.
pub fn last_board_candidates(state: &GameState, player: PlayerId, exclude: &[String]) -> Vec<LastBoardEntry> {
    let mut candidates: Vec<LastBoardEntry> = Vec::new();
    let entries: &[LastBoardEntry] = state
        .last_boards
        .as_ref()
        .and_then(|boards| boards.get(player))
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for entry in entries {
        if exclude.contains(&entry.def_id) {
            continue;
        }
        match candidates.iter_mut().find(|candidate| candidate.def_id == entry.def_id) {
            None => candidates.push(entry.clone()),
            Some(held) => held.radiant = held.radiant || entry.radiant,
        }
    }
    candidates
}
