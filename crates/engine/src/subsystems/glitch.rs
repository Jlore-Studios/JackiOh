//! The Glitch Easter egg (issue #170; SPEC §7, R673–R679).
//!
//!   - `count_system_play` (R673): a play of a "… in the System" card (`SYSTEM_CARD_DEF_IDS`), by either
//!     player, adds one to `state.system_plays`, which `catalog::pick_generated` reads: every card
//!     generated into a hand or a deck after that is Glitch with odds n/10000.
//!   - `glitch` (R676): Glitch's own text. One draw of the match rng picks one of `GLITCH_OUTCOMES`,
//!     and a public `glitched` event names it:
//!       reset  — the match starts again from `create_game`'s decks, shuffled and dealt by the match
//!                rng, mulligans and all (`reset_match`, run by `reduce` once the action has settled,
//!                so nothing of the old game is still resolving when it goes);
//!       swap   — each account now plays the other seat (R677). The engine's game is unchanged; the
//!                hosts read `state.seat_swaps` (`seats_swapped`), and the server credits results by it;
//!       boards — both fields become the boards of two other players' games, a frozen setup input
//!                like C+ #29's last boards (`state.glitch_boards`, R678); hands, decks and life stay;
//!       void   — the game ends with no winner and reason `voided`, and the server keeps no trace of
//!                it but a log line (R679).
//!
//! All of it is plain data on the state, so `(seed, decks, …, log)` folds to the same game (§9.3).
//!
//! Port of `packages/engine/src/subsystems/glitch.ts`.

use std::borrow::Borrow;

use crate::config::{GLITCH_OUTCOMES, SETUP_TURN, SYSTEM_CARD_DEF_IDS};
use crate::script::{EngineSink, Effect};
use crate::state::{
    CardInstance, CreateGameOptions, GameState, create_game_for_reset, find_instance_mut, new_instance,
};
use crate::wire::{
    CardType, GameEvent, GameOverReason, GlitchOutcome, PLAYER_IDS, PerPlayerOpt, PlayerId, Row, Winner,
    Zone,
};

/// The cards a zone read hands back, as owned copies.
fn owned<R: Borrow<CardInstance>>(cards: impl IntoIterator<Item = R>) -> Vec<CardInstance> {
    cards.into_iter().map(|card| card.borrow().clone()).collect()
}

/// TS `catalog.selfDefIds`: the catalog ids a card stands for — its own, or, for a fused card, every
/// ingredient's, recursively. Read through `subsystems::fuse`, which knows a digest id's list from the
/// state (SURFACE §6.6: no process-wide digest table).
fn self_def_ids(state: &GameState, def_id: &str) -> Vec<String> {
    match crate::subsystems::fuse::fused_id_specs_in(state, def_id) {
        None => vec![def_id.to_string()],
        Some(specs) => specs
            .iter()
            .flat_map(|spec| self_def_ids(state, &spec.def_id))
            .collect(),
    }
}

/// R673: count one play of `card` if it is a "… in the System" card (a fused one counts once).
pub fn count_system_play(state: &mut GameState, card: &CardInstance) {
    if !self_def_ids(state, &card.def_id)
        .iter()
        .any(|id| SYSTEM_CARD_DEF_IDS.contains(&id.as_str()))
    {
        return;
    }
    state.system_plays = Some(state.system_plays.unwrap_or(0) + 1);
}

/// R677: whether the accounts now hold each other's seat — an odd number of swaps.
pub fn seats_swapped(state: &GameState) -> bool {
    state.seat_swaps.unwrap_or(0) % 2 == 1
}

/// R677: the seat the account that began the match in `seat` plays now.
pub fn seat_played_by(state: &GameState, seat: PlayerId) -> PlayerId {
    if !seats_swapped(state) {
        return seat;
    }
    if seat == PlayerId::P1 { PlayerId::P2 } else { PlayerId::P1 }
}

/// R676: Glitch's text — one of its four outcomes, drawn by the match rng.
pub fn glitch() -> Effect {
    Effect::new("glitch", |ctx| {
        let at = ctx.sink.rng.int(GLITCH_OUTCOMES.len() as i32);
        let outcome = usize::try_from(at)
            .ok()
            .and_then(|at| GLITCH_OUTCOMES.get(at).copied())
            .unwrap_or(GlitchOutcome::Reset);
        ctx.sink.events.push(GameEvent::Glitched {
            player: ctx.controller,
            outcome,
        });
        match outcome {
            GlitchOutcome::Reset => ctx.sink.state.reset_owed = Some(true),
            GlitchOutcome::Swap => {
                ctx.sink.state.seat_swaps = Some(ctx.sink.state.seat_swaps.unwrap_or(0) + 1);
            }
            GlitchOutcome::Boards => place_glitch_boards(ctx.sink.state),
            // TS hands `endGame` a plain sink over the context's state, events and rng.
            GlitchOutcome::Void => {
                crate::game_over::end_game(&mut ctx.sink, Winner::Draw, GameOverReason::Voided)
            }
        }
    })
}

/// R678: every card on both fields ceases to exist (no Death, no graveyard, R11's way out), the Locks
/// go, and each side takes its frozen other game's board in order: its Units into the unit zones and
/// the rest into the backrow, left to right, until a row is full. Each card is a new one its side owns,
/// on its entry's face, as having entered this turn (R171). Nothing is summoned or played, so nothing
/// triggers. A side with no board frozen is left empty.
fn place_glitch_boards(state: &mut GameState) {
    for player in PLAYER_IDS {
        for row in [Row::Units, Row::Backrow] {
            for slot in crate::zones::slots_of(player, row) {
                for card in owned(crate::zones::zone_contents(state, &slot)) {
                    crate::zones::cease_to_exist(state, &card);
                }
                crate::zones::unlock_zone(state, &slot);
            }
        }
    }
    for player in PLAYER_IDS {
        let entries = state
            .glitch_boards
            .as_ref()
            .and_then(|boards| boards.get(player))
            .cloned()
            .unwrap_or_default();
        for entry in entries {
            if crate::subsystems::fuse::rebuild_fused_def(state, &entry.def_id, player).is_none() {
                continue;
            }
            // Created only once a zone is free, so a full row creates nothing.
            let mut card = new_instance(state, &entry.def_id, player, Zone::Resolving { player });
            card.radiant = entry.radiant;
            let card_type = crate::catalog::def_of(Some(state), &entry.def_id).type_;
            if card_type == CardType::Spell {
                continue;
            }
            let row = if card_type == CardType::Unit { Row::Units } else { Row::Backrow };
            let Some(slot) = crate::zones::first_free_zone(state, player, row) else {
                continue;
            };
            if !crate::zones::place_on_field(state, &card, &slot, Default::default()) {
                continue;
            }
            let turn = state.turn;
            if let Some(placed) = find_instance_mut(state, &card.id) {
                placed.summoned_turn = Some(turn);
                if card_type == CardType::FieldSpell {
                    placed.face_up = Some(true);
                }
            }
        }
    }
}

/// R676: the reset a Glitch owed, once its action has settled. The state becomes a new game made from
/// the decks the match began with — its seats' handicaps, last boards and Glitch boards, the seat
/// swaps and the nonce log kept — with fresh ids, numbered from where the old game stopped by a stream
/// of this reset's own (R223), and setup runs again on the match rng. A state that keeps no record of
/// its opening (one the AI redacted) does nothing.
pub fn reset_match(sink: &mut EngineSink<'_>) {
    sink.state.reset_owed = None;
    if sink.state.result.is_some() {
        return;
    }
    let Some(opening) = sink.state.opening.clone() else {
        return;
    };
    let old = &*sink.state;
    let resets = old.resets.unwrap_or(0) + 1;
    let mut handicaps = PerPlayerOpt::default();
    for player in PLAYER_IDS {
        if let Some(handicap) = old.players[player].handicap {
            *handicaps.slot(player) = Some(handicap);
        }
    }
    let options = CreateGameOptions {
        seed: old.seed.clone(),
        decks: opening.decks.clone(),
        handicaps: Some(handicaps),
        dealt: opening.dealt.clone(),
        ..CreateGameOptions::default()
    };
    let mut fresh = create_game_for_reset(&options, old.next_id, resets);
    fresh.next_seq = old.next_seq;
    fresh.applied = old.applied.clone();
    fresh.resets = Some(resets);
    if old.last_boards.is_some() {
        fresh.last_boards = old.last_boards.clone();
    }
    if old.glitch_boards.is_some() {
        fresh.glitch_boards = old.glitch_boards.clone();
    }
    if old.seat_swaps.is_some() {
        fresh.seat_swaps = old.seat_swaps;
    }
    fresh.turn = SETUP_TURN;
    *sink.state = fresh;
    crate::setup::begin_setup(sink);
}
