//! The practice session as one pure state machine (#3099, #2956 Part B; SPEC §9.9, §9.10): a game against
//! the AI seat, with practice's own setup (`deal`) and rules.
//!
//! [`PracticeHost`] is the state a host holds: the `GameState`, the log of accepted actions and what the
//! session decides around them. [`step`] takes it, a [`PracticeEvent`] and the time, and hands back the
//! state and the [`PracticeOutput`]s the host carries out, in order: the snapshot the page is told
//! (rule 7), the save and the kept game to write, the AI's move to decide, the game's end to record.
//! Nothing here does I/O, reads a clock or draws a random number the state does not hold: a host that
//! restarts resumes from its state as JSON (`crates/shared/README.md`, the host contract).
//!
//! What the machine decides, and what stays the host's:
//!
//!  - nonces (`h<n>` the human's, `a<n>` the AI's, each counting only accepted actions, so a refused
//!    action spends nothing and the log folds exactly: R187, R508, R433), the seat each action is sent
//!    from (a Glitch swap moves both players, R677), what the AI may never take and what it falls back on
//!    (R188, R923000), the snapshot (R508, R3100000), the save and its resume (R668), the kept game and
//!    its number (R768), the game's end (R679) and the claim of a game on a server seed (R1303000);
//!  - the host holds the store the saves and the kept games go to, the AI's search (`Decide`) and the
//!    lethal check (`Lethal`) under their clocks, and the pacing of the AI's turn.
//!
//! The AI's draws come from its own stream, `"{seed}:ai"`, never the match's: the host answers `Decide`
//! with the cursor the search stopped at.
//!
//! The machine reduces under whatever catalog the host has set (R388, R767000): it calls the engine as
//! the engine is called, and a host with a pinned catalog runs `step` inside it.

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::{Deserialize, Serialize};

use jackioh_ai::{AiOptions, SearchBudget, ai_to_act};
use jackioh_engine::{
    Action, ActionBody, CreateGameOptions, GameOverReason, GameState, PlayerId, ReplayInput, Rng,
    VERSIONS_SCHEME, outcome_for,
};

use super::super::clock::{ClockExpiry, MatchClock};
use super::super::snapshot::{clock_view_for, snapshot as match_snapshot};
use super::super::timer::{Fx, TimerCommand, TimerKey};
use super::deal::deal;
use super::types::{
    AiAsk, AiMove, KeptAs, LethalAsk, PracticeClaim, PracticeConfig, PracticeDebug, PracticeRecord,
    PracticeReplay, PracticeReplaySetup, PracticeReplaySummary, PracticeSave, PracticeSnapshot, RunReport,
};
use crate::config::{PRACTICE_AI_CLOCK_MS, PRACTICE_LETHAL_HINT_CLOCK_MS, PRACTICE_REPLAYS_KEPT};

/// The first letter of a nonce the human's action is logged under, `h<n>`.
const HUMAN_NONCE: char = 'h';
/// The first letter of a nonce the AI's action is logged under, `a<n>`.
const AI_NONCE: char = 'a';

/// What the host feeds the machine.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "event", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PracticeEvent {
    /// The human's action, minted `h<n>` and sent from the seat the human plays now (R677).
    Act { body: ActionBody },
    /// The AI's turn to move, when the host's pacing says so: it owes a move (`Decide`) or nothing.
    AiTurn,
    /// The AI's move, as `Decide` asked for it (R188: a move it never takes falls back, R923000).
    Decided { moved: AiMove },
    /// An action of a game already played, applied under its own nonce and seat: a save resumed
    /// (R668), a claim checked (R2560000). Refused if its nonce is not the next of its player or its
    /// seat is not the one that player plays now.
    Fold { action: Action },
    /// R768: the host is about to list or open the games it keeps (the stateless calls of `replays.rs`).
    /// Refused while this game is in progress, whose fold would stall the host; a game that is over
    /// lets it.
    Replays,
    /// A timer the machine set came due (`Timer` output): R1563006's sequence window. A stale one, taken
    /// back or replaced since, does nothing.
    TimerFired { key: TimerKey, id: u32 },
    /// R1303000: the finished game's claim. Refused before the end (CLAUDE.md rule 7: the log names the
    /// AI's hidden cards while the game runs) and for a game not on a server seed.
    Claim,
    /// DR14, R2706000: a finished fight as the run reads it. Refused before the end and for a game that
    /// is no fight. Only the run's action crosses, never the log.
    RunReport,
    /// Dev builds only: the raw game, for the e2e replay check.
    Debug,
}

/// What the host carries out, in the order given.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "output", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PracticeOutput {
    /// The game is on (made, or resumed): the seat the AI plays; it began there.
    Began { ai_seat: PlayerId },
    /// R668: write this save, or clear it (`None`). R3476000: a run fight's save goes in its own
    /// slot, filed by the driver, never touching the free game's.
    Save { save: Option<Box<PracticeSave>> },
    /// R768: append this game to the ones kept and keep only the newest `keep`.
    Keep { replay: Box<PracticeReplay>, keep: u32 },
    /// The game is over: record it (SPEC §9.5).
    Finished { record: Box<PracticeRecord> },
    /// R679: a Glitch voided the game: no result, no replay, no claim.
    Voided {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// R1563006: set or take back a timer; its firing comes back as `timerFired` with the same key and id.
    /// Only the human's question sequence has one (practice keeps no turn clock, SPEC §9.9).
    Timer { command: TimerCommand },
    /// What the page is told (rule 7).
    Snapshot { snapshot: Box<PracticeSnapshot> },
    /// R2102000: ask the lethal check for `seat` under a clock; when it says yes, the snapshot above
    /// says `lethal`. A host that has no such check leaves it out.
    Lethal { ask: LethalAsk },
    /// The AI owes a move: decide it under the clock, and feed `Decided`.
    Decide { ask: AiAsk },
    /// R1303000: the finished game's claim.
    Claim { claim: Box<PracticeClaim> },
    /// DR14: the fight as the run reads it.
    RunReport { report: Box<RunReport> },
    /// Dev builds only.
    Debug { debug: Box<PracticeDebug> },
    /// The event was refused and nothing changed: the page is told this reason.
    Failed { message: String },
}

/// The practice session's state: the checkpoint. Serializable, with the `GameState` inside it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeHost {
    /// The config the game is played under (a run fight's seed replaces the asked one, R2706000).
    config: PracticeConfig,
    /// The game as `create_game` had it: decks, handicaps, last boards, dealt seats, deck marks.
    options: CreateGameOptions,
    /// The catalog version the game is played on (R767000), named in its save and its kept game.
    catalog: String,
    /// The seat the AI began in; after a Glitch swap it plays the other one (R677).
    ai_seat: PlayerId,
    game: GameState,
    log: Vec<Action>,
    ai_cursor: u32,
    human_count: u32,
    ai_count: u32,
    /// The engine's refusal of the human's last action, until the board moves on.
    error: Option<String>,
    /// R768: its end has been dealt with (kept, or passed over), so it is dealt with once.
    ended: bool,
    /// R768: the numbers of the games kept on the device, or `None` when the host keeps none.
    kept_games: Option<Vec<u32>>,
    /// R768000: the number and step count it was kept under, once kept.
    kept_as: Option<KeptAs>,
    /// R1563006: the match clock, of which only the question sequence's window runs here, and its timers.
    clock: MatchClock,
    fx: Fx,
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    out: Vec<PracticeOutput>,
}

/// The machine's one function: `host` and `event` in, the next `host` and its outputs out.
pub fn step(mut host: PracticeHost, event: PracticeEvent, now: i64) -> (PracticeHost, Vec<PracticeOutput>) {
    let outputs = host.step(event, now);
    (host, outputs)
}

/// R188: the actions the AI never takes on its own, even as a fallback. MD-D29, R1127: the persona's
/// emote is no AI decision; it arrives on its own request. R923000: the sandbox's tools are the human's
/// alone (`legal_actions` never lists one; this gate holds it twice).
pub fn ai_never_takes(body: &ActionBody) -> bool {
    super::super::bot::r188_forbidden(body) || matches!(body, ActionBody::Sandbox { .. })
}

/// R1371000: the catalog a kept game or save was played on, as it is named now: one kept before the
/// version retcon carries no `versions` marker, and one of an older marker names a v0.5 version as it
/// was (R2532000).
pub fn catalog_of(catalog: &str, versions: Option<u32>) -> String {
    jackioh_cards::versions::versions_now_in(versions.unwrap_or(1), catalog)
}

/// `body` and `action_type` for the engine's own panic text.
fn panic_text(body: &ActionBody, payload: &(dyn std::any::Any + Send)) -> String {
    format!(
        "the engine failed on {}: {}",
        body.action_type(),
        jackioh_ai::panic_message(payload)
    )
}

impl PracticeHost {
    /// A game for `config` on `catalog`, dealt and begun, and what the host does first. `kept` is the
    /// numbers of the games the host keeps on its device (`None`: it keeps none). `Err` is why there is
    /// no game: the engine's reason for a deck or a setup it refuses.
    pub fn start(
        config: &PracticeConfig,
        catalog: &str,
        kept: Option<Vec<u32>>,
        now: i64,
    ) -> Result<(PracticeHost, Vec<PracticeOutput>), String> {
        let mut host = PracticeHost::open(config, catalog, kept)?;
        host.began(now);
        let outputs = host.take_out();
        Ok((host, outputs))
    }

    /// R668: the game `save` left, folded again through the same nonces to the same hash. `asked` is
    /// the game the page remembers: a save of any other game is not the one it asked for. `build` is the
    /// catalog version this build plays on: a save from another catalog is refused.
    pub fn resume(
        asked: &PracticeConfig,
        save: Option<&PracticeSave>,
        build: &str,
        kept: Option<Vec<u32>>,
        now: i64,
    ) -> Result<(PracticeHost, Vec<PracticeOutput>), String> {
        let Some(save) = save else {
            return Err("no practice game is saved on this device".to_string());
        };
        // R923000: a sandbox game is the page's only when it remembers a sandbox, whose panel it then shows.
        let kept_config = &save.config;
        if kept_config.seed != asked.seed
            || kept_config.human_seat != asked.human_seat
            || kept_config.difficulty != asked.difficulty
            || kept_config.sandbox != asked.sandbox
        {
            return Err("the saved practice game is not the one the page remembers".to_string());
        }
        // R3476000: a free game never resumes a run fight's save, nor a run fight a free game's.
        // Two run fights then rest on the seed, which names the fight (the run's seed, row and
        // column).
        if kept_config.is_run() != asked.is_run() {
            return Err("the saved practice game is not the one the page remembers".to_string());
        }
        let saved_on = catalog_of(&save.catalog, save.versions);
        if saved_on != build {
            return Err(format!(
                "the saved game was played on catalog {saved_on}, not {build}"
            ));
        }
        let mut host = PracticeHost::open(&save.config, build, kept)?;
        for action in &save.log {
            host.fold(action)
                .map_err(|refusal| format!("the saved game does not fold: {refusal}"))?;
        }
        if jackioh_engine::hash_state(&host.game) != save.hash {
            return Err("the saved game does not fold to the state it was saved at".to_string());
        }
        host.ai_cursor = save.ai_cursor;
        host.began(now);
        let outputs = host.take_out();
        Ok((host, outputs))
    }

    /// The game dealt and begun, before the host is told anything.
    fn open(config: &PracticeConfig, catalog: &str, kept: Option<Vec<u32>>) -> Result<PracticeHost, String> {
        let opening = deal(config)?;
        let created = catch_unwind(AssertUnwindSafe(|| jackioh_engine::create_game(&opening.options)))
            .map_err(|payload| jackioh_ai::panic_message(&*payload))?;
        // `create_game` throws on an illegal deck and `begin_game` reports a refusal: no game.
        let begun = jackioh_engine::begin_game(&created);
        if let Some(error) = begun.error {
            return Err(format!("the engine refused to begin the game: {error}"));
        }
        Ok(PracticeHost {
            ai_seat: opening.config.human_seat.opponent(),
            config: opening.config,
            options: opening.options,
            catalog: catalog.to_string(),
            game: begun.state,
            log: Vec::new(),
            ai_cursor: 0,
            human_count: 0,
            ai_count: 0,
            error: None,
            ended: false,
            kept_games: kept,
            kept_as: None,
            clock: MatchClock::new(0),
            fx: Fx::default(),
            out: Vec::new(),
        })
    }

    /// One event in, its outputs out.
    pub fn step(&mut self, event: PracticeEvent, now: i64) -> Vec<PracticeOutput> {
        match event {
            PracticeEvent::Act { body } => {
                self.error = self.apply(body, true).err();
                self.answered(now);
            }
            PracticeEvent::AiTurn => self.on_ai_turn(now),
            PracticeEvent::Decided { moved } => self.on_decided(moved, now),
            PracticeEvent::TimerFired { key, id } => self.on_timer(key, id, now),
            PracticeEvent::Fold { action } => match self.fold(&action) {
                Ok(()) => self.end_of_game(now, false),
                Err(message) => self.emit(PracticeOutput::Failed { message }),
            },
            PracticeEvent::Replays => {
                if !self.is_over() {
                    self.emit(PracticeOutput::Failed {
                        message: "replays are not available while a practice game is in progress".to_string(),
                    });
                }
            }
            PracticeEvent::Claim => self.on_claim(),
            PracticeEvent::RunReport => self.on_run_report(),
            PracticeEvent::Debug => {
                let debug = Box::new(self.debug());
                self.emit(PracticeOutput::Debug { debug });
            }
        }
        self.take_out()
    }

    // Reads

    /// The config the game is played under.
    pub fn config(&self) -> &PracticeConfig {
        &self.config
    }

    /// The game as it stands (never sent to the page: its window onto it is [`PracticeSnapshot`]).
    pub fn game(&self) -> &GameState {
        &self.game
    }

    /// Every accepted action, in order, under its `h<n>` or `a<n>` nonce.
    pub fn log(&self) -> &[Action] {
        &self.log
    }

    /// The setup `create_game` had, plus the log: what the game folds from.
    pub fn fold_args(&self) -> ReplayInput {
        replay_input(&self.options, self.log.clone())
    }

    /// R677: the seat the human plays now: the one it began in, or the AI's after an odd number of
    /// Glitch swaps.
    pub fn human_seat(&self) -> PlayerId {
        jackioh_engine::seat_played_by(&self.game, self.config.human_seat)
    }

    /// R677: the seat the AI plays now.
    pub fn ai_seat(&self) -> PlayerId {
        jackioh_engine::seat_played_by(&self.game, self.ai_seat)
    }

    /// Whether the game has a result (voided or not).
    pub fn is_over(&self) -> bool {
        self.game.result.is_some()
    }

    /// The AI's move for `ask` on this game, for a host in the same process with no clock (a test, the
    /// server): the search ends at `budget`. One that panics gives no move, as `Decide` allows.
    pub fn decide(&self, ask: &AiAsk, budget: SearchBudget) -> AiMove {
        let mut options = AiOptions {
            rng: Rng::new(&ask.rng_seed, ask.rng_cursor),
            budget,
            should_stop: None,
            wrap_up: None,
        };
        let decision = catch_unwind(AssertUnwindSafe(|| {
            jackioh_ai::decide(&self.game, ask.seat, &mut options)
        }));
        match decision {
            Ok(decision) => AiMove {
                body: decision.map(|decision| decision.action),
                rng_cursor: options.rng.cursor(),
            },
            Err(_) => AiMove {
                body: None,
                rng_cursor: ask.rng_cursor,
            },
        }
    }

    // Output

    fn emit(&mut self, output: PracticeOutput) {
        self.out.push(output);
    }

    fn take_out(&mut self) -> Vec<PracticeOutput> {
        std::mem::take(&mut self.out)
    }

    // Applying an action

    /// R677: in the seat the player plays now; `Ok` when accepted, else the engine's reason. The nonce
    /// counts by player, not by seat, so a swap leaves both streams unbroken.
    fn apply(&mut self, body: ActionBody, human: bool) -> Result<(), String> {
        let seat = if human { self.human_seat() } else { self.ai_seat() };
        let nonce = if human {
            format!("{HUMAN_NONCE}{}", self.human_count)
        } else {
            format!("{AI_NONCE}{}", self.ai_count)
        };
        let action = Action::new(body, seat, nonce);
        let result = catch_unwind(AssertUnwindSafe(|| jackioh_engine::reduce(&self.game, &action)))
            .map_err(|payload| panic_text(&action.body, &*payload))?;
        if let Some(error) = result.error {
            return Err(error);
        }
        self.game = result.state;
        self.log.push(action);
        if human {
            self.human_count += 1;
        } else {
            self.ai_count += 1;
        }
        Ok(())
    }

    /// An action already played, applied under its own nonce and seat (R2560000: an AI action is held to
    /// its nonce and seat and reduced like any other, never re-decided). `Err` is why it does not fold.
    fn fold(&mut self, action: &Action) -> Result<(), String> {
        if self.is_over() {
            return Err("an action after the end of the game".to_string());
        }
        let human = if action.nonce == format!("{AI_NONCE}{}", self.ai_count) {
            false
        } else if action.nonce == format!("{HUMAN_NONCE}{}", self.human_count) {
            true
        } else {
            return Err("a nonce differs".to_string());
        };
        let seat = if human { self.human_seat() } else { self.ai_seat() };
        if action.player_id != seat {
            return Err(format!("{} is not the seat its player plays now", action.nonce));
        }
        self.apply(action.body.clone(), human)
    }

    // What follows a change

    /// The answer to a human action or an AI move: the save, the end of the game if it came, the snapshot.
    fn answered(&mut self, now: i64) {
        self.save();
        self.end_of_game(now, true);
        self.sync_clock(now);
        self.snapshot();
    }

    /// The game is on: its first answer.
    fn began(&mut self, now: i64) {
        let ai_seat = self.ai_seat;
        self.emit(PracticeOutput::Began { ai_seat });
        self.save();
        self.sync_clock(now);
        self.snapshot();
    }

    /// R668: the save a game in progress leaves after each answer (R3476000: a run fight's too,
    /// filed by the driver in its own slot, never touching the free game's); none for a lesson (its
    /// coach reads the game from its first snapshot on, so a lesson restarts instead) or a finished
    /// game, which clears the slot.
    fn save(&mut self) {
        let save = (!self.config.is_lesson() && !self.is_over()).then(|| {
            Box::new(PracticeSave {
                catalog: self.catalog.clone(),
                versions: Some(VERSIONS_SCHEME),
                config: self.config.clone(),
                log: self.log.clone(),
                ai_cursor: self.ai_cursor,
                hash: jackioh_engine::hash_state(&self.game),
            })
        });
        self.emit(PracticeOutput::Save { save });
    }

    /// R768: a free game that has just ended is kept; a lesson, or a game Glitch voided (R679), is not,
    /// nor is R2706000's run fight, which the run records itself. `keep` is false for a game being folded
    /// (a claim, a resume), which keeps nothing.
    fn end_of_game(&mut self, now: i64, keep: bool) {
        let Some(result) = self.game.result else {
            return;
        };
        if self.ended {
            return;
        }
        self.ended = true;
        self.clock.stop(&mut self.fx);
        self.flush_timers();
        // R679: the one ending that records nothing.
        if result.reason == GameOverReason::Voided {
            self.emit(PracticeOutput::Voided { at: now });
            return;
        }
        if keep && !self.config.is_lesson() && !self.config.is_run() {
            self.keep(now);
        }
        let human = self.human_seat();
        let record = PracticeRecord {
            outcome: result,
            human_seat: human,
            // R1843004: the AI's `fled` is the human's draw.
            result: outcome_for(&result, human).into(),
            turns: self.game.turn,
            at: now,
            human_actions: self
                .log
                .iter()
                .filter(|action| action.nonce.starts_with(HUMAN_NONCE))
                .map(|action| action.body.clone())
                .collect(),
            ai_moves: self.ai_count,
            last_board: jackioh_engine::last_board_for(&self.game, human),
            final_hash: jackioh_engine::hash_state(&self.game),
        };
        self.emit(PracticeOutput::Finished {
            record: Box::new(record),
        });
    }

    /// R768: the finished game as the host keeps it, numbered after the newest it holds.
    fn keep(&mut self, now: i64) {
        let Some(kept) = &mut self.kept_games else {
            return;
        };
        let Some(result) = self.game.result else {
            return;
        };
        let game = kept.iter().copied().max().unwrap_or(0) + 1;
        kept.push(game);
        let surplus = kept.len().saturating_sub(PRACTICE_REPLAYS_KEPT as usize);
        kept.drain(..surplus);
        let end_seat = self.human_seat();
        let steps = u32::try_from(self.log.len() + 1).unwrap_or(u32::MAX);
        let replay = PracticeReplay {
            catalog: self.catalog.clone(),
            versions: Some(VERSIONS_SCHEME),
            config: self.config.clone(),
            setup: Some(PracticeReplaySetup {
                decks: self.options.decks.clone(),
                handicaps: self.options.handicaps.clone().unwrap_or_default(),
                last_boards: self.options.last_boards.clone(),
                dealt: self.options.dealt.clone(),
            }),
            log: self.log.clone(),
            hash: jackioh_engine::hash_state(&self.game),
            end_seat,
            summary: PracticeReplaySummary {
                game,
                ended_at: now,
                // R1843004: the AI's `fled` is the human's draw.
                result: outcome_for(&result, end_seat).into(),
                turns: self.game.turn,
                steps,
                portraits: self.config.portraits.clone().unwrap_or_else(default_portraits),
            },
        };
        self.kept_as = Some(KeptAs { game, steps });
        self.emit(PracticeOutput::Keep {
            replay: Box::new(replay),
            keep: PRACTICE_REPLAYS_KEPT,
        });
    }

    /// What the page is told, and the lethal check to run for it (R2102000, R2291000).
    fn snapshot(&mut self) {
        let snapshot = Box::new(self.snapshot_now());
        let seat = self.human_seat();
        self.emit(PracticeOutput::Snapshot { snapshot });
        self.emit(PracticeOutput::Lethal {
            ask: LethalAsk {
                seat,
                clock_ms: PRACTICE_LETHAL_HINT_CLOCK_MS,
            },
        });
    }

    /// The page's window onto the game as it stands now (rule 7): what a snapshot output holds, for a
    /// host that is asked for it again (a page that reloads).
    pub fn snapshot_now(&self) -> PracticeSnapshot {
        // R677: the human sees, and acts from, the seat it plays now; the result reads from it too.
        let human = self.human_seat();
        // R508: the board the human takes away, once a free game is over; R679: a voided game leaves
        // none, and R923000: a sandbox game, whose board its tools built, leaves none either, nor does
        // R2706000's run fight.
        let keeps_board = self
            .game
            .result
            .is_some_and(|result| result.reason != GameOverReason::Voided)
            && !self.config.is_lesson()
            && !self.config.is_sandbox()
            && !self.config.is_run();
        PracticeSnapshot {
            view: jackioh_engine::view_for(&self.game, human),
            legal: jackioh_engine::legal_actions(&self.game, human),
            ai_to_act: ai_to_act(&self.game, self.ai_seat()),
            error: self.error.clone(),
            last_board: keeps_board.then(|| jackioh_engine::last_board_for(&self.game, human)),
            replay: self.kept_as,
            lethal: None,
            // R3100000, R3477000: every snapshot; a lesson's coach names its attacks by it, and the
            // page decides where to draw it.
            forecast: Some(jackioh_engine::combat_forecast(&self.game, human)),
        }
    }

    // The sequence clock (R1563006)

    /// R1563006: a sequence's first question for the human (Meditative+ #20 KY's Competition) arms the
    /// window `KY_COMPETITION_SECONDS` long, a later question keeps it, none open clears it. What the 1v1
    /// match clock decides (`host::clock`, `sync_sequence`) is read here as it is there; only the human's
    /// own questions run it, as the AI answers its own without a clock. The page is told as a `timer` output.
    fn sync_clock(&mut self, now: i64) {
        if self.clock.is_stopped() {
            return;
        }
        let mut view = clock_view_for(&match_snapshot(&self.game));
        let human = self.human_seat();
        view.sequence = view.sequence.filter(|question| question.player == human);
        self.clock.sync_sequence(&view, now, &mut self.fx);
        self.flush_timers();
    }

    fn flush_timers(&mut self) {
        for command in self.fx.take() {
            self.emit(PracticeOutput::Timer { command });
        }
    }

    /// R1563006: the window lapsed with a question of its sequence still open for the human: it is
    /// answered Stop (R1184001's answer, as a timeout gives), like the 1v1 machine's clock action, and
    /// logged as the human's action, `h<n>`, so a claim's log still folds under the nonces it knows.
    fn on_timer(&mut self, key: TimerKey, id: u32, now: i64) {
        if let Some(ClockExpiry::Sequence { player }) = self.clock.fire(key, id) {
            let open = match_snapshot(&self.game)
                .sequence
                .is_some_and(|question| question.player == player && player == self.human_seat());
            if open && let Some(stop) = jackioh_engine::prompts::stop_answer(&self.game) {
                self.error = self.apply(stop, true).err();
            }
        }
        self.answered(now);
    }

    // The AI's turn

    fn on_ai_turn(&mut self, now: i64) {
        let seat = self.ai_seat();
        if !ai_to_act(&self.game, seat) {
            self.answered(now);
            return;
        }
        let ask = AiAsk {
            seat,
            rng_seed: format!("{}:ai", self.config.seed),
            rng_cursor: self.ai_cursor,
            clock_ms: PRACTICE_AI_CLOCK_MS,
        };
        self.emit(PracticeOutput::Decide { ask });
    }

    /// One AI action. R188: a move it never takes, or one the engine refuses, falls back on `endTurn`
    /// first and then the rest of what is legal, never one R188 forbids; it fails only when no action is
    /// possible.
    fn on_decided(&mut self, moved: AiMove, now: i64) {
        if !ai_to_act(&self.game, self.ai_seat()) {
            self.answered(now);
            return;
        }
        self.ai_cursor = moved.rng_cursor;
        if let Some(body) = moved.body.filter(|body| !ai_never_takes(body))
            && self.apply(body, false).is_ok()
        {
            self.error = None;
            self.answered(now);
            return;
        }
        let mut last_refusal = "no legal action".to_string();
        for candidate in self.fallbacks() {
            match self.apply(candidate, false) {
                Ok(()) => {
                    self.error = None;
                    self.answered(now);
                    return;
                }
                Err(refusal) => last_refusal = refusal,
            }
        }
        self.emit(PracticeOutput::Failed {
            message: format!(
                "the AI owes an action and the engine accepts none of its legal ones ({last_refusal})"
            ),
        });
    }

    /// The AI's fallback when its decision gave nothing usable: `endTurn` first, never one R188 forbids.
    fn fallbacks(&self) -> Vec<ActionBody> {
        let legal: Vec<ActionBody> = jackioh_engine::legal_actions(&self.game, self.ai_seat())
            .into_iter()
            .filter(|body| !ai_never_takes(body))
            .collect();
        let (end_turn, rest): (Vec<ActionBody>, Vec<ActionBody>) = legal
            .into_iter()
            .partition(|body| matches!(body, ActionBody::EndTurn));
        end_turn.into_iter().chain(rest).collect()
    }

    // Claims, reports and the debug answer

    /// R1303000: a finished game on a server seed, as the server checks it.
    fn on_claim(&mut self) {
        if !self.config.is_earning() {
            return self.emit(PracticeOutput::Failed {
                message: "this practice game is not on a server seed, so it earns nothing".to_string(),
            });
        }
        let Some(result) = self.game.result else {
            return self.emit(PracticeOutput::Failed {
                message: "a practice game is claimed only once it is over".to_string(),
            });
        };
        let end_seat = self.human_seat();
        let claim = PracticeClaim {
            seed: self.config.seed.clone(),
            // R677, R1843004: from the seat the human ended in, as `keep` and the server's fold read it.
            result: outcome_for(&result, end_seat).into(),
            log: self.log.clone(),
        };
        self.emit(PracticeOutput::Claim {
            claim: Box::new(claim),
        });
    }

    /// DR14, R2706000: `run_fight_report` over the game's setup and log for the seat the human ended in
    /// (R677). The log never crosses: the page gets the run's action, or its refusal.
    fn on_run_report(&mut self) {
        if !self.config.is_run() {
            return self.emit(PracticeOutput::Failed {
                message: "this practice game is not a dungeon run's fight".to_string(),
            });
        }
        if !self.is_over() {
            return self.emit(PracticeOutput::Failed {
                message: "a fight is reported only once it is over".to_string(),
            });
        }
        let report = match jackioh_run::run_fight_report(&self.fold_args(), self.human_seat()) {
            Ok(action) => RunReport::Action {
                action: Box::new(action),
            },
            Err(error) => RunReport::Refused { error },
        };
        self.emit(PracticeOutput::RunReport {
            report: Box::new(report),
        });
    }

    fn debug(&self) -> PracticeDebug {
        PracticeDebug {
            seed: self.config.seed.clone(),
            decks: self.options.decks.clone(),
            handicaps: self.options.handicaps.clone().unwrap_or_default(),
            log: self.log.clone(),
            state: self.game.clone(),
            hash: jackioh_engine::hash_state(&self.game),
            difficulty: self.config.difficulty,
            human_seat: self.config.human_seat,
            lesson: self.config.lesson.clone(),
            last_boards: self.options.last_boards.clone(),
            dealt: self.options.dealt.clone(),
            deck_marks: self.options.deck_marks.clone(),
            sandbox: self.config.is_sandbox().then_some(true),
        }
    }
}

/// R768: both seats' portraits when the page dealt none.
fn default_portraits() -> jackioh_engine::PerPlayer<jackioh_engine::PortraitId> {
    jackioh_engine::PerPlayer::new(jackioh_engine::DEFAULT_PORTRAIT, jackioh_engine::DEFAULT_PORTRAIT)
}

/// The fold input of a setup and a log: `create_game`'s options and the log.
pub(super) fn replay_input(options: &CreateGameOptions, log: Vec<Action>) -> ReplayInput {
    ReplayInput {
        seed: options.seed.clone(),
        decks: options.decks.clone(),
        log,
        catalog: None,
        handicaps: options.handicaps.clone(),
        dealt: options.dealt.clone(),
        last_boards: options.last_boards.clone(),
        glitch_boards: None,
        sandbox: options.sandbox,
        deck_marks: options.deck_marks.clone(),
        win_rates: options.win_rates.clone(),
    }
}
