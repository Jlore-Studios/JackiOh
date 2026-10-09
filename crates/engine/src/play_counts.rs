//! What every play leaves behind to be counted (docs/classic-sets.md B5 E4, R451).
//!
//! §10.5 step 4 counts a play the moment it places the card: `turnLog.playedIds`, `cardsPlayed`,
//! R213's `costsPaid` and the game's `played` counter (R55), and now these too —
//!   * per player, per turn (on both players' turns): plays by the type each was played as (B2.7),
//!     in the turn log, which `startTurn` rebuilds for both players (Classic+ #37 Wardrum);
//!   * per player, per game: plays by tag in the player's `gameLog`, never reset (Classic+ #64 Mulch
//!     Muncher's Fruit, AI Scaling Law's AI generated cards);
//!   * game-wide: the last Spell anyone played (Classic #57 Echo, `state.lastSpell`), and per player
//!     the last face-up card they played (AI Autocomplete, `gameLog.lastFaceUpPlay`). Each "last"
//!     record is overwritten by the next play and never cleared.
//!
//! A cast is a play and counts (R70). A countered play never reaches step 4, so it counts for nothing
//! (R448). An Echo repeat is the same play resolving again, not a play (§6.2).
//!
//! The readers are `query.rs`'s (the read half of the card-facing surface); this module only writes.
//!
//! Port of `packages/engine/src/playCounts.ts`.

use indexmap::{IndexMap, IndexSet};

use crate::catalog::fused_id_parts;
use crate::config::LAST_FACE_UP_SKIPPED_TAGS;
use crate::faces::card_type_of;
use crate::script::HookArgs;
use crate::state::{CardInstance, FaceUpRecord, GameLog, GameState, PlayRecord};
use crate::wire::{CardType, PlayerId, Tag};

/// R451: what a play records as the card played — the card itself, or what its `recordsPlayAs` hook
/// names (Classic #57 Echo records the Spell it copied; null records nothing). A fused card records
/// itself: its combined text is its ingredients' (R102), and no ingredient names another card for it.
pub fn play_record_of(state: &GameState, card: &CardInstance) -> Option<PlayRecord> {
    let own = PlayRecord {
        def_id: card.def_id.clone(),
        radiant: card.radiant,
    };
    if fused_id_parts(Some(state), &card.def_id).is_some() {
        return Some(own);
    }
    let script = crate::scripts::script_of(state, card);
    let Some(hook) = script.records_play_as.as_ref() else {
        return Some(own);
    };
    let named = hook(HookArgs {
        state,
        self_: card,
        radiant: card.radiant,
    })?;
    Some(PlayRecord {
        def_id: named.def_id,
        radiant: named.radiant,
    })
}

/// A Trap or Field Trap is set face-down (§3.2, R33), so its play is never a face-up one (R451).
fn played_face_down(type_: CardType) -> bool {
    type_ == CardType::Trap || type_ == CardType::FieldTrap
}

/// ME-ALTPLAY, MB11: Feng Shui judges no face-down play — the one test both systems agree on.
pub fn is_face_down_play(state: &GameState, card: &CardInstance) -> bool {
    played_face_down(card_type_of(state, card))
}

/// §10.5 step 4, B5 E4: count one play of `card` by `player`, read as the card is placed (its face and
/// type as they stand after step 3 made it Radiant, if it did).
pub fn record_play(state: &mut GameState, player: PlayerId, card: &CardInstance) {
    let type_ = card_type_of(state, card);
    // MD-B15, R923: a play is counted by the tags the card had as it was played, granted ones included.
    let tags: Vec<Tag> = crate::query::tags_of(state, card);

    {
        let side = &mut state.players[player];
        let mut types: IndexMap<CardType, i32> = side.turn_log.played_by_type.clone().unwrap_or_default();
        *types.entry(type_).or_insert(0) += 1;
        side.turn_log.played_by_type = Some(types);

        let mut by_tag: IndexMap<Tag, i32> = side
            .game_log
            .as_ref()
            .map(|log| log.played_by_tag.clone())
            .unwrap_or_default();
        // `new Set(tags)`: each tag once, in first-seen order.
        let distinct: IndexSet<Tag> = tags.iter().copied().collect();
        for tag in distinct {
            *by_tag.entry(tag).or_insert(0) += 1;
        }
        let log = GameLog {
            played_by_tag: by_tag,
            last_face_up_play: side
                .game_log
                .as_ref()
                .and_then(|log| log.last_face_up_play.clone()),
        };
        side.game_log = Some(log);
    }

    let Some(record) = play_record_of(state, card) else {
        return;
    };
    if type_ == CardType::Spell {
        state.last_spell = Some(record.clone());
    }
    // R451: the last face-up card, passing over Traps (set face-down, so nothing hidden is ever
    // recorded) and the tags `LAST_FACE_UP_SKIPPED_TAGS` names (the AI generated cards).
    if played_face_down(type_) || tags.iter().any(|tag| LAST_FACE_UP_SKIPPED_TAGS.contains(tag)) {
        return;
    }
    if let Some(log) = state.players[player].game_log.as_mut() {
        // R1300: a Chinese card's record is Chinese, so T-AI-5's copy of it is too; a card that
        // records itself as another card's play (`records_play_as`) records that card as printed.
        let chinese = if record.def_id == card.def_id {
            card.chinese
        } else {
            None
        };
        log.last_face_up_play = Some(FaceUpRecord {
            def_id: record.def_id,
            radiant: record.radiant,
            type_,
            chinese,
        });
    }
}
