//! A finished game's record for the card statistics (SPEC §9.11, R376): which cards each seat started
//! with, drew and played, and how the game ended, read off a replay of `(seed, decks, log)`.
//!
//! The state keeps no such history — `turnLog` is one turn's, and `applied` keeps only the last
//! actions' events — so this folds the log exactly as `replay::fold` does and reads the raw events
//! each step emits, which no view has redacted. It runs only where the whole game is already known:
//! the server once a match has its result, and the AI's development runs.
//!
//! What "played" means needs one bookkeeping device. `cardPlayed` is emitted for a play from hand and
//! for a cast alike (§2.4's cast on draw, R70), and only the first is the player's play. So each
//! seat's hand is followed through the events: the hand the state holds as an action begins, plus
//! every `addedToHand`, minus every card that leaves it. A `cardPlayed` naming a card in that hand is
//! a play from hand; a cast names a card that never entered it. A play countered in its announce
//! window has no `cardPlayed` (§10.5, R448), so it is none; a play step 3 replaced (R449) is followed
//! by its `transformed` to the card it resolves as, the one its `cardPlayed` names.
//!
//! Port of `packages/engine/src/gameSummary.ts` (part 5).

use indexmap::IndexMap;

use crate::reduce::{begin_game, reduce};
use crate::replay::FoldArgs;
use crate::state::{CreateGameOptions, GameState, create_game};
use crate::wire::{GameEvent, GameSummary, PLAYER_IDS, PerPlayer, PlayerId, SeatSummary};

/// Each seat's hand as instance id → catalog id, in hand order.
type Hands = PerPlayer<IndexMap<String, String>>;

struct Reading {
    hands: Hands,
    /// The seat of the first `turnStarted`, and both hands at that moment; `None` until a turn has begun.
    first: Option<PlayerId>,
    opening: Option<PerPlayer<Vec<String>>>,
    drawn: PerPlayer<Vec<String>>,
    played: PerPlayer<Vec<String>>,
    played_turns: PerPlayer<Vec<i32>>,
    current_turn: i32,
}

/// Each seat's hand as an action begins: the cards the state holds there, and those of `following`
/// (the hands the last action ended with) that a play has taken to the resolving zone and not yet
/// placed. A prompt can hold a play there past its action (§10.5: the choices of the card step 3 put
/// in its place, R449, or the announce window, C #4 Palantir), and its `cardPlayed` comes in a later one.
fn hands_of(state: &GameState, following: Option<&Hands>) -> Hands {
    let hand = |player: PlayerId| -> IndexMap<String, String> {
        let mut cards: IndexMap<String, String> = state.players[player]
            .hand
            .iter()
            .map(|card| (card.id.clone(), card.def_id.clone()))
            .collect();
        for card in &state.players[player].resolving {
            if following.is_some_and(|following| following[player].contains_key(&card.id)) {
                cards.insert(card.id.clone(), card.def_id.clone());
            }
        }
        cards
    };
    PerPlayer::new(hand(PlayerId::P1), hand(PlayerId::P2))
}

fn per_seat<T>() -> PerPlayer<Vec<T>> {
    PerPlayer::new(Vec::new(), Vec::new())
}

/// A card leaves whichever hand holds it.
fn leave(hands: &mut Hands, instance_id: &str) {
    for player in PLAYER_IDS {
        hands[player].shift_remove(instance_id);
    }
}

/// One step's events, in the order they happened.
fn read(reading: &mut Reading, events: &[GameEvent]) {
    for (at, event) in events.iter().enumerate() {
        match event {
            GameEvent::TurnStarted { player, turn } => {
                reading.current_turn = *turn;
                // §2.1: the mulligans have resolved and The Coin is dealt; the first turn's draw comes next.
                if reading.opening.is_none() {
                    reading.first = Some(*player);
                    reading.opening = Some(PerPlayer::new(
                        reading.hands.p1.values().cloned().collect(),
                        reading.hands.p2.values().cloned().collect(),
                    ));
                }
            }
            GameEvent::Drawn {
                player,
                instance_id,
                def_id,
                ..
            } => {
                // §2.4: a draw that reached the hand is followed at once by its `addedToHand`; a burned
                // card's `burned` or a cast's `cardPlayed` follows instead. The opening hand's draws are the
                // opening hand's, not draws of the game.
                let reached_hand = matches!(
                    events.get(at + 1),
                    Some(GameEvent::AddedToHand { instance_id: next_id, .. }) if next_id == instance_id
                );
                if reading.opening.is_some() && reached_hand {
                    reading.drawn[*player].push(def_id.clone());
                }
            }
            GameEvent::AddedToHand {
                player,
                instance_id,
                def_id,
            } => {
                reading.hands[*player].insert(instance_id.clone(), def_id.clone());
            }
            GameEvent::CardPlayed {
                player,
                instance_id,
                def_id,
                former_id,
                ..
            } => {
                // R227: a Trap set face-down took a fresh id; the hand knew it by the old one.
                let id = former_id.as_ref().unwrap_or(instance_id);
                if !reading.hands[*player].contains_key(id) {
                    continue;
                }
                reading.played[*player].push(def_id.clone());
                let turn = reading.current_turn;
                reading.played_turns[*player].push(turn);
                reading.hands[*player].shift_remove(id);
            }
            GameEvent::Transformed {
                instance_id,
                to_def_id,
                new_instance_id,
                ..
            } => {
                for player in PLAYER_IDS {
                    let hand = &mut reading.hands[player];
                    if !hand.contains_key(instance_id) {
                        continue;
                    }
                    hand.shift_remove(instance_id);
                    hand.insert(new_instance_id.clone(), to_def_id.clone());
                }
            }
            GameEvent::Fused { instance_ids, .. } => {
                for id in instance_ids {
                    leave(&mut reading.hands, id);
                }
            }
            GameEvent::ShuffledIn { instance_id, .. }
            | GameEvent::Discarded { instance_id, .. }
            | GameEvent::Exiled { instance_id, .. }
            | GameEvent::Bounced { instance_id, .. }
            | GameEvent::EnteredGraveyard { instance_id, .. } => {
                leave(&mut reading.hands, instance_id);
            }
            _ => {}
        }
    }
}

fn seat_summary(
    reading: &Reading,
    decks: &(Vec<String>, Vec<String>),
    player: PlayerId,
    seat: usize,
) -> SeatSummary {
    let deck = match seat {
        0 => decks.0.clone(),
        1 => decks.1.clone(),
        _ => Vec::new(),
    };
    let played_turns = &reading.played_turns[player];
    SeatSummary {
        deck,
        opening: reading
            .opening
            .as_ref()
            .map(|opening| opening[player].clone())
            .unwrap_or_default(),
        drawn: reading.drawn[player].clone(),
        played: reading.played[player].clone(),
        played_turns: if played_turns.is_empty() {
            None
        } else {
            Some(played_turns.clone())
        },
    }
}

/// R376: what each seat's cards did in a finished game, and how it ended. The fold is
/// `replay::fold`'s, so an action the log holds and the engine refuses is skipped as it is there.
/// `None` when the log leaves the game without a result: a record is only ever made of a finished
/// game.
pub fn summarize_game(input: &FoldArgs) -> Option<GameSummary> {
    // TS hands `createGame` no `dealt` here (only `fold` does); kept so, since it changes no event.
    let start = create_game(&CreateGameOptions {
        seed: input.seed.clone(),
        decks: input.decks.clone(),
        catalog: input.catalog.clone(),
        handicaps: input.handicaps.clone(),
        dealt: None,
        // R417, R678: setup like the decks, so the fold is the game that was played.
        last_boards: input.last_boards.clone(),
        glitch_boards: input.glitch_boards.clone(),
    });
    let mut reading = Reading {
        hands: hands_of(&start, None),
        first: None,
        opening: None,
        drawn: per_seat(),
        played: per_seat(),
        played_turns: per_seat(),
        current_turn: 1,
    };

    let begun = begin_game(&start);
    read(&mut reading, &begun.events);
    let mut state = begun.state;

    for action in &input.log {
        // The state's own hands as the action begins, so nothing the events do not spell out (a card
        // whose control changed, say) carries from one action into the next, but a play still in flight.
        reading.hands = hands_of(&state, Some(&reading.hands));
        let result = reduce(&state, action);
        if result.error.is_some() {
            continue;
        }
        read(&mut reading, &result.events);
        state = result.state;
    }

    let outcome = state.result?;
    Some(GameSummary {
        // §2.1: Player 1 takes the first turn. A game that ended before any turn began names it too.
        first: reading.first.unwrap_or(PLAYER_IDS[0]),
        winner: outcome.winner,
        reason: outcome.reason,
        turns: state.turn,
        seats: PerPlayer::new(
            seat_summary(&reading, &input.decks, PlayerId::P1, 0),
            seat_summary(&reading, &input.decks, PlayerId::P2, 1),
        ),
    })
}
