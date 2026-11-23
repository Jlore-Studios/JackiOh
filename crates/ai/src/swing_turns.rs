//! R4552000, R4552002 (SPEC §9.11): a finished game's turn rows, the data Decisiveness is read off.
//!
//! The engine's record (`summarize_game`) cannot call the AI, so this fold replays the same log from the
//! same start (`recorded_start`, `read_steps`) and, at the start of every turn, writes one row for the
//! seat whose turn it is (`SwingTurn`): the turn counter, the AI's evaluation of the seat less its
//! opponent's (`evaluate`, the true, whole state: the game is over and nothing is hidden from a
//! statistic), and the cards acting on each side's field (`acting_on`, a face-down Trap left out).
//!
//! R4552002: a Created card (R943) on the field names its maker, the card whose play or trigger was
//! resolving when it appeared (a summon's `sourceId` when the event names one), unless a hand held it
//! first: then it was played, and its play was the seat's decision (The Coin, Rosemary Triscuit). The
//! fold follows every instance through the whole log, so no instance carries its maker.
//!
//! Pure (CLAUDE.md rule 4): the server calls it under the record's catalog pin, beside `summarize_game`
//! (`crates/server/src/api/game_records.rs`), and the development runs call it as they file a record.

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::config::SWING_EVAL_SCALE;
use jackioh_engine::{
    ActingCard, CardInstance, CreatedActing, FoldArgs, GameEvent, GameState, GameSummary, PLAYER_IDS,
    PerPlayer, PlayerId, StandingAt, SwingTurn, acting_on, find_instance, is_face_down, read_steps,
    recorded_start,
};

use crate::config::AI_EVAL;
use crate::evaluate::{NextSwing, evaluate};

/// R4552000: reads a game's turn rows off its steps, one `(before, events, after)` at a time, as
/// `swing_turns` folds a log; a test reads a `scenario()`'s verbs through it the same way.
#[derive(Default)]
pub struct SwingReader {
    rows: PerPlayer<Vec<SwingTurn>>,
    /// R4552002: each Created instance met on the field, by instance id, with its maker (`None` when a
    /// hand held it first).
    makers: IndexMap<String, Option<String>>,
    /// R4552002: every Created instance a hand has held.
    in_hand: IndexSet<String>,
}

fn is_created(card: &CardInstance) -> bool {
    card.created == Some(true)
}

impl SwingReader {
    /// A reader with no rows yet.
    pub fn new() -> SwingReader {
        SwingReader::default()
    }

    /// One step: the state as it began, the events it emitted, and the state it left.
    pub fn step(&mut self, before: &GameState, events: &[GameEvent], after: &GameState) {
        for state in [before, after] {
            for player in PLAYER_IDS {
                for card in state.players[player].hand.iter().filter(|card| is_created(card)) {
                    self.in_hand.insert(card.id.clone());
                }
            }
        }
        let mut resolving: Option<String> = None;
        let mut turns_started: Vec<(PlayerId, i32)> = Vec::new();
        for event in events {
            match event {
                GameEvent::CardPlayed { def_id, .. } | GameEvent::HookResolving { def_id, .. } => {
                    resolving = Some(def_id.clone());
                }
                GameEvent::Summoned {
                    instance_id,
                    source_id,
                    ..
                } => {
                    let made = find_instance(after, instance_id).is_some_and(is_created);
                    if made && !self.makers.contains_key(instance_id) && !self.in_hand.contains(instance_id) {
                        let source = source_id
                            .as_deref()
                            .and_then(|source| {
                                find_instance(after, source).or_else(|| find_instance(before, source))
                            })
                            .map(|source| source.def_id.clone());
                        self.makers
                            .insert(instance_id.clone(), source.or_else(|| resolving.clone()));
                    }
                }
                GameEvent::TurnStarted { player, turn, .. } => turns_started.push((*player, *turn)),
                _ => {}
            }
        }
        // A Created card no summon named (a Field Spell an effect cast: Jayaad's Shawarma Law) is the
        // last resolving card's, unless a hand held it.
        for player in PLAYER_IDS {
            for card in acting_on(after, player, StandingAt::AmongUnits) {
                if is_created(card) && !self.makers.contains_key(&card.id) {
                    let maker = if self.in_hand.contains(&card.id) {
                        None
                    } else {
                        resolving.clone()
                    };
                    self.makers.insert(card.id.clone(), maker);
                }
            }
        }
        if after.result.is_some() {
            return;
        }
        // Every turn the step began has its row. A turn can begin and end inside one step (one with
        // nothing but ending it left to do ends by itself, R345; the autopilot plays one, R1183002), and
        // the step's own states are all there is to read: each such turn is read off the state the step
        // left, as the last one is.
        for (seat, turn) in turns_started {
            let opponent = seat.opponent();
            let difference = evaluate(after, seat, NextSwing::Seat, &AI_EVAL)
                - evaluate(after, opponent, NextSwing::Enemy, &AI_EVAL);
            let row = SwingTurn {
                turn,
                eval: (difference * SWING_EVAL_SCALE).round() as i64,
                acting: self.acting(after, seat),
                facing: self.acting(after, opponent),
            };
            self.rows[seat].push(row);
        }
    }

    /// `player`'s acting cards, each a row's entry: a face-down Trap has no text acting, so it is left
    /// out (and it is hidden).
    fn acting(&self, state: &GameState, player: PlayerId) -> Vec<ActingCard> {
        acting_on(state, player, StandingAt::AmongUnits)
            .into_iter()
            .filter(|card| !is_face_down(state, card))
            .map(|card| {
                if is_created(card) {
                    ActingCard::Created(CreatedActing {
                        card: card.def_id.clone(),
                        maker: self.makers.get(&card.id).cloned().flatten(),
                    })
                } else {
                    ActingCard::Dealt(card.def_id.clone())
                }
            })
            .collect()
    }

    /// Each seat's rows so far.
    pub fn turns(&self) -> &PerPlayer<Vec<SwingTurn>> {
        &self.rows
    }
}

/// R4552000: each seat's turn rows of a finished game, `None` with no result. It replays the log as
/// `summarize_game` does (the same start, an action the engine refuses skipped), so the rows are the
/// game the record summarizes.
pub fn swing_turns(input: &FoldArgs) -> Option<PerPlayer<Vec<SwingTurn>>> {
    let start = recorded_start(input);
    let mut reader = SwingReader::new();
    let end = read_steps(&start, &input.log, |before, events, after| {
        reader.step(before, events, after)
    });
    end.result?;
    Some(reader.rows)
}

/// R4552000: files each seat's rows in its summary (`SeatSummary.swing_turns`).
pub fn attach_swing_turns(game: &mut GameSummary, turns: PerPlayer<Vec<SwingTurn>>) {
    let PerPlayer { p1, p2 } = turns;
    game.seats.p1.swing_turns = Some(p1);
    game.seats.p2.swing_turns = Some(p2);
}
