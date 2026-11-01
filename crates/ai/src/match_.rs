//! Whole games and whole AI turns, for the gates, the sweep and the puzzles (docs/polish/3-ai.md).
//!
//! Both runners are deterministic: the game rng lives in the state (`reduce` takes none, so it
//! resumes from `(seed, rngCursor)` exactly as `fold` does), every controller draws from its own
//! stream seeded from the match seed and its seat, and every nonce is derived from the log. So a
//! config is a game, and folding a record's log with `MatchConfig::fold_args` (its seed, decks,
//! handicaps and setup inputs) reproduces its hash.
//!
//! Matches and turn runners. Errors surface via `catch_unwind` on illegal states (SURFACE §4.4.9).

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::{
    Action, ActionBody, ActionType, CreateGameOptions, FoldArgs, GameResult, GameState, Handicap,
    LastBoardInput, PerPlayer, PerPlayerOpt, PlayerId, Rng, begin_game, create_game, hash_state,
    legal_actions, reduce, seat_to_act, subsystems,
};
use serde::{Deserialize, Serialize};

use crate::baselines::{greedy_action, random_action};
use crate::config::{AI_BUDGET, AI_SKILL_MAX};
use crate::decide::decide;
use crate::observe::ai_to_act;
use crate::skill::{plays_best, skill_miss_action};
use crate::types::{AiOptions, Decision, DecisionReason, SearchBudget};

/// `AI_MATCH`'s shape.
struct AiMatch {
    max_actions: usize,
}

const AI_MATCH: AiMatch = AiMatch { max_actions: 3000 };

/// play_ai_turn's ceiling on actions in one turn.
const AI_TURN_MAX_ACTIONS: usize = 60;

/// Who plays a seat: the AI (at a budget, AI_BUDGET when none is given, and at a skill, full when
/// none is given: R747000), the greedy baseline or the random policy.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SeatController {
    Ai {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        budget: Option<SearchBudget>,
        /// R747000: the chance each decision follows the AI, the rest the skill miss (R2214000)
        /// (`skill::plays_best`); `None` is `AI_SKILL_MAX`, which takes no extra draw.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill: Option<f64>,
    },
    Greedy,
    Random,
}

impl SeatController {
    /// `controller.kind`.
    pub fn kind(&self) -> &'static str {
        match self {
            SeatController::Ai { .. } => "ai",
            SeatController::Greedy => "greedy",
            SeatController::Random => "random",
        }
    }
}

/// The setup inputs a match is created with beyond its decks and handicaps (§9.3), as `create_game`
/// and `fold` take them (#1697). Each is `None` unless a caller sets it, and a config with none is
/// created, played and folded exactly as before they existed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MatchSetup {
    /// R433: the seats whose deck their player was dealt rather than built.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dealt: Option<Vec<PlayerId>>,
    /// R417, R564: each seat's last board, which C+ #29 Portal to the Past Discovers from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_boards: Option<LastBoardInput>,
    /// R678: the boards a Glitch may lay down, one per seat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glitch_boards: Option<LastBoardInput>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchConfig {
    pub seed: String,
    pub decks: (Vec<String>, Vec<String>),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handicaps: Option<PerPlayerOpt<Handicap>>,
    pub controllers: PerPlayer<SeatController>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_actions: Option<usize>,
    /// #1697: the setup inputs the game is created with beyond its decks and handicaps. Its keys
    /// (`dealt`, `lastBoards`, `glitchBoards`) are written beside the others, and it is none by default.
    #[serde(flatten)]
    pub setup: MatchSetup,
}

impl MatchConfig {
    /// What `fold` takes to replay `log` as the game `play_match` created from this config (§9.3): its
    /// seed, decks, handicaps and setup inputs.
    pub fn fold_args(&self, log: Vec<Action>) -> FoldArgs {
        FoldArgs {
            seed: self.seed.clone(),
            decks: self.decks.clone(),
            log,
            handicaps: self.handicaps.clone(),
            dealt: self.setup.dealt.clone(),
            last_boards: self.setup.last_boards.clone(),
            glitch_boards: self.setup.glitch_boards.clone(),
            ..FoldArgs::default()
        }
    }
}

/// Called with the true state before and after every accepted action.
pub type AfterActionHook<'a> = Box<dyn FnMut(&GameState, &GameState, PlayerId, &ActionBody) + 'a>;

/// Wraps controller calls for timing (e.g. sweep re-decision, R977000). Must invoke callback once.
pub type TimeDecisionHook<'a> = Box<dyn FnMut(&GameState, PlayerId, &Rng, &mut dyn FnMut()) + 'a>;

/// Handed (the true state, the seat, the controller's answer); answers the action the controller plays.
pub type OverrideChoiceHook<'a> =
    Box<dyn FnMut(&GameState, PlayerId, Option<ActionBody>) -> Option<ActionBody> + 'a>;

#[derive(Default)]
pub struct MatchHooks<'a> {
    /// Called with the true state before and after every accepted action.
    pub after_action: Option<AfterActionHook<'a>>,
    /// Wraps each controller call for timing (the sweep); default none.
    pub time_decision: Option<TimeDecisionHook<'a>>,
    /// Overrides controller choice before fallback; panics count as controller throws.
    pub override_choice: Option<OverrideChoiceHook<'a>>,
}

/// One refused action (`MatchRecord.rejected`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RejectedAction {
    pub seat: PlayerId,
    pub action: ActionBody,
    pub error: String,
}

/// One controller or reducer failure that ended the match (`MatchRecord.thrown`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThrownError {
    pub seat: PlayerId,
    pub message: String,
}

/// The first panic an AI's search caught in a match (`MatchRecord.first_sim_panic`, #1429): the seat
/// whose decision caught it and its message.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SimPanic {
    pub seat: PlayerId,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchRecord {
    /// None when maxActions was hit or a controller threw.
    pub result: Option<GameResult>,
    pub log: Vec<Action>,
    /// hash_state of the final state.
    pub hash: String,
    pub turns: i32,
    pub rejected: Vec<RejectedAction>,
    pub thrown: Vec<ThrownError>,
    /// AI decisions with reason "fallback", plus controller nulls replaced by random_action.
    pub fallbacks: i32,
    pub decisions: i32,
    pub nodes: usize,
    /// The AI decisions' `stats.sim_panics`, summed (#1429): panics their searches caught, each a
    /// trap that loses the decision in the browser; the gates fail on any (B31).
    pub sim_panics: usize,
    /// The first of them, by the seat that caught it; None when there is none.
    pub first_sim_panic: Option<SimPanic>,
    /// defIds each seat played, in order.
    pub played: PerPlayer<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct AiTurnResult {
    pub state: GameState,
    pub actions: Vec<Action>,
    pub decisions: Vec<Decision>,
}

/// What one controller call produced: the action, and the AI's decision when it was an AI.
struct ControllerChoice {
    action: Option<ActionBody>,
    decision: Option<Decision>,
}

fn choose_for(
    controller: &SeatController,
    state: &GameState,
    seat: PlayerId,
    rng: &mut Rng,
) -> ControllerChoice {
    match controller {
        SeatController::Ai { budget, skill } => {
            if !plays_best(skill.unwrap_or(AI_SKILL_MAX), rng) {
                return ControllerChoice {
                    action: skill_miss_action(state, seat, rng),
                    decision: None,
                };
            }
            let mut options = AiOptions {
                rng: rng.clone(),
                budget: budget.unwrap_or(AI_BUDGET),
                should_stop: None,
            };
            let decision = decide(state, seat, &mut options);
            *rng = options.rng;
            ControllerChoice {
                action: decision.as_ref().map(|decision| decision.action.clone()),
                decision,
            }
        }
        SeatController::Greedy => ControllerChoice {
            action: greedy_action(state, seat, rng),
            decision: None,
        },
        SeatController::Random => ControllerChoice {
            action: random_action(state, seat, rng),
            decision: None,
        },
    }
}

/// What stands in for a refused action: endTurn when it is legal, else the first legal answer (or
/// mulligan), else any other legal action the random policy would take. Tried in this order.
/// `decide` reads the same list (`submitted`): an action the reducer would refuse on the true
/// state is replaced before it is ever submitted, so a refusal costs no recorded rejection.
pub(crate) fn replacements_for(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    let legal: Vec<ActionBody> = legal_actions(state, seat)
        .into_iter()
        .filter(|action| !subsystems::AI_SKIPPED_ACTIONS.contains(&action.action_type()))
        .collect();
    let is_answer =
        |action: &ActionBody| matches!(action.action_type(), ActionType::Answer | ActionType::Mulligan);
    let mut out: Vec<ActionBody> = Vec::new();
    out.extend(
        legal
            .iter()
            .filter(|action| matches!(action, ActionBody::EndTurn))
            .cloned(),
    );
    out.extend(legal.iter().filter(|action| is_answer(action)).cloned());
    out.extend(
        legal
            .iter()
            .filter(|action| !matches!(action, ActionBody::EndTurn) && !is_answer(action))
            .cloned(),
    );
    out
}

/// The defId of the hand card a `play` names, read before the play moves it.
fn played_def_id(state: &GameState, seat: PlayerId, action: &ActionBody) -> Option<String> {
    let ActionBody::Play { instance_id, .. } = action else {
        return None;
    };
    state.players[seat]
        .hand
        .iter()
        .find(|instance| instance.id == *instance_id)
        .map(|card| card.def_id.clone())
}

/// An accepted action: what was chosen, as sent, and the state after it.
struct Accepted {
    body: ActionBody,
    action: Action,
    next: GameState,
}

/// Plays deterministic match: creates and begins game, querying `seat_to_act` (R265).
/// Records rejected actions and controller thrown errors.
pub fn play_match(config: &MatchConfig, hooks: &mut MatchHooks) -> MatchRecord {
    let max_actions = config.max_actions.unwrap_or(AI_MATCH.max_actions);
    let created = create_game(&CreateGameOptions {
        seed: config.seed.clone(),
        decks: config.decks.clone(),
        handicaps: config.handicaps.clone(),
        dealt: config.setup.dealt.clone(),
        last_boards: config.setup.last_boards.clone(),
        glitch_boards: config.setup.glitch_boards.clone(),
        ..CreateGameOptions::default()
    });
    let mut state = begin_game(&created).state;

    let mut rngs: PerPlayer<Rng> = PerPlayer {
        p1: Rng::new(&format!("{}:ctl:p1", config.seed), 0),
        p2: Rng::new(&format!("{}:ctl:p2", config.seed), 0),
    };

    let mut log: Vec<Action> = Vec::new();
    let mut rejected: Vec<RejectedAction> = Vec::new();
    let mut thrown: Vec<ThrownError> = Vec::new();
    let mut played: PerPlayer<Vec<String>> = PerPlayer {
        p1: Vec::new(),
        p2: Vec::new(),
    };
    let mut fallbacks: i32 = 0;
    let mut decisions: i32 = 0;
    let mut nodes: usize = 0;
    let mut sim_panics: usize = 0;
    let mut first_sim_panic: Option<SimPanic> = None;

    while state.result.is_none() && log.len() < max_actions {
        let seat: PlayerId = seat_to_act(&state).unwrap_or(state.active);
        let controller = config.controllers[seat];
        let current = &state;

        let mut choice: Option<ControllerChoice> = None;
        let called = {
            let rng = &mut rngs[seat];
            let time_decision = &mut hooks.time_decision;
            let override_choice = &mut hooks.override_choice;
            catch_unwind(AssertUnwindSafe(|| {
                // The controller's stream before the call, for a hook that decides the state again (R977000).
                let before = time_decision.as_ref().map(|_| rng.clone());
                let mut run = || {
                    let mut made = choose_for(&controller, current, seat, rng);
                    if let Some(over) = override_choice.as_mut() {
                        made.action = over(current, seat, made.action.take());
                    }
                    choice = Some(made);
                };
                match (time_decision.as_mut(), before.as_ref()) {
                    (Some(time), Some(before)) => time(current, seat, before, &mut run),
                    _ => run(),
                }
            }))
        };
        if let Err(error) = called {
            thrown.push(ThrownError {
                seat,
                message: format!(
                    "controller {} threw: {}",
                    controller.kind(),
                    crate::simulate::panic_message(error.as_ref())
                ),
            });
            break;
        }
        let choice = choice.unwrap_or(ControllerChoice {
            action: None,
            decision: None,
        });

        if let Some(decision) = &choice.decision {
            decisions += 1;
            nodes += decision.stats.nodes;
            sim_panics += decision.stats.sim_panics;
            if first_sim_panic.is_none()
                && let Some(message) = &decision.first_sim_panic
            {
                first_sim_panic = Some(SimPanic {
                    seat,
                    message: message.clone(),
                });
            }
            if decision.reason == DecisionReason::Fallback {
                fallbacks += 1;
            }
        }

        let chosen = match choice.action {
            Some(chosen) => chosen,
            None => {
                fallbacks += 1;
                match random_action(current, seat, &mut rngs[seat]) {
                    Some(chosen) => chosen,
                    None => {
                        thrown.push(ThrownError {
                            seat,
                            message: format!(
                                "no action for {} while the game is live (turn {})",
                                seat.as_str(),
                                current.turn
                            ),
                        });
                        break;
                    }
                }
            }
        };

        let nonce = format!("m{}", log.len());
        let reduced = catch_unwind(AssertUnwindSafe(|| {
            let action = Action::new(chosen.clone(), seat, nonce.clone());
            let result = reduce(current, &action);
            match result.error {
                None => Some(Accepted {
                    body: chosen.clone(),
                    action,
                    next: result.state,
                }),
                Some(error) => {
                    rejected.push(RejectedAction {
                        seat,
                        action: chosen.clone(),
                        error,
                    });
                    for replacement in replacements_for(current, seat) {
                        let alternative = Action::new(replacement.clone(), seat, nonce.clone());
                        let retry = reduce(current, &alternative);
                        if retry.error.is_none() {
                            return Some(Accepted {
                                body: replacement,
                                action: alternative,
                                next: retry.state,
                            });
                        }
                    }
                    None
                }
            }
        }));
        let accepted = match reduced {
            Ok(accepted) => accepted,
            Err(error) => {
                thrown.push(ThrownError {
                    seat,
                    message: format!(
                        "reduce threw on \"{}\": {}",
                        chosen.action_type().as_str(),
                        crate::simulate::panic_message(error.as_ref())
                    ),
                });
                break;
            }
        };

        let Some(accepted) = accepted else {
            thrown.push(ThrownError {
                seat,
                message: format!(
                    "no legal replacement for a refused \"{}\" (turn {})",
                    chosen.action_type().as_str(),
                    current.turn
                ),
            });
            break;
        };

        if let Some(def_id) = played_def_id(current, seat, &accepted.body) {
            played[seat].push(def_id);
        }

        log.push(accepted.action);
        if let Some(after_action) = hooks.after_action.as_mut() {
            after_action(&state, &accepted.next, seat, &accepted.body);
        }
        state = accepted.next;
    }

    MatchRecord {
        result: state.result,
        log,
        hash: hash_state(&state),
        turns: state.turn,
        rejected,
        thrown,
        fallbacks,
        decisions,
        nodes,
        sim_panics,
        first_sim_panic,
        played,
    }
}

/// decide → reduce until !ai_to_act(state, seat) or AI_TURN_MAX_ACTIONS; nonces `t${n}`.
pub fn play_ai_turn(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> AiTurnResult {
    let mut actions: Vec<Action> = Vec::new();
    let mut decisions: Vec<Decision> = Vec::new();
    let mut current = state.clone();

    let mut n = 0usize;
    while n < AI_TURN_MAX_ACTIONS && ai_to_act(&current, seat) {
        let Some(decision) = decide(&current, seat, options) else {
            break;
        };
        let action = Action::new(decision.action.clone(), seat, format!("t{n}"));
        decisions.push(decision);
        let result = reduce(&current, &action);
        // A refusal would be offered again on the unchanged state, so the turn stops here instead.
        if result.error.is_some() {
            break;
        }
        actions.push(action);
        current = result.state;
        n += 1;
    }

    AiTurnResult {
        state: current,
        actions,
        decisions,
    }
}
