//! The random-legal-action policy of SPEC §10.7 (BUILD M3-T7, R44): My Pawn's AI turn and every
//! "Targets chosen randomly" pick use it. Uniform over `legal_actions`, ending the turn when that is
//! the only option left or with AI_END_TURN_PROBABILITY otherwise, and answering prompts uniformly.
//!
//! Every choice comes from the seeded rng (§10.7): `play_out_turn` draws from the sink's rng, whose
//! cursor `reduce` stores back in the state, so a My Pawn turn replays exactly (R44).
//!
//! `AI_PLAYOUT_STEP_CAP` (`crate::config`, CLAUDE.md rule 9, SURFACE §6.4) is far above any reachable
//! turn, only so a card that keeps refilling the hand cannot spin forever. `work.rs` dispatches
//! `"@aiTurn"` to `run_owed_ai_turn` (SURFACE §6.6).

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::config::{AI_END_TURN_PROBABILITY, AI_PLAYOUT_STEP_CAP};
use crate::reduce::ReduceResult;
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{GameState, Resume, WorkItem};
use crate::wire::{Action, ActionBody, ActionType, Phase, PlayerId};

/// Actions the policy does not take. `legal_actions` always offers `concede`, so §10.7's "ending the
/// turn when it is the only option" reads on the set without these, and R44's AI plays the turn out
/// rather than resigning it (R84). The M1-gate `play_random_game` filters the first three.
pub const AI_SKIPPED_ACTIONS: &[ActionType] = &[
    ActionType::Concede,
    ActionType::OfferDraw,
    ActionType::AnswerDraw,
    // MD-D29, R1127: the random policy never emotes.
    ActionType::Emote,
];

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PolicyOptions {
    /// Replaces AI_SKIPPED_ACTIONS; pass `Some(vec![])` for a literal uniform draw over `legal_actions`.
    pub skip: Option<Vec<ActionType>>,
}

/// The set the policy draws from: `legal_actions` minus the action types it never takes.
pub fn policy_actions(state: &GameState, player: PlayerId, options: PolicyOptions) -> Vec<ActionBody> {
    let skip: &[ActionType] = options.skip.as_deref().unwrap_or(AI_SKIPPED_ACTIONS);
    crate::reduce::legal_actions(state, player)
        .into_iter()
        .filter(|action| !skip.contains(&action.action_type()))
        .collect()
}

/// One step of the policy (§10.7) with the default options, for the fuzz, the golden recorder and the
/// training lanes' random opponent (SURFACE §9, §10.1).
pub fn choose_action(state: &GameState, player: PlayerId, rng: &mut Rng) -> Option<ActionBody> {
    choose_action_with(state, player, rng, PolicyOptions::default())
}

/// One step of the policy (§10.7): end the turn when nothing else is on offer, else with
/// AI_END_TURN_PROBABILITY, else pick uniformly. With a prompt open `legal_actions` offers its answers
/// and R211's concede, which the skipped set removes, so this draw answers prompts uniformly (R44).
pub fn choose_action_with(
    state: &GameState,
    player: PlayerId,
    rng: &mut Rng,
    options: PolicyOptions,
) -> Option<ActionBody> {
    let actions = policy_actions(state, player, options);
    let end_turn = actions
        .iter()
        .find(|action| action.action_type() == ActionType::EndTurn)
        .cloned();
    let others: Vec<ActionBody> = actions
        .into_iter()
        .filter(|action| action.action_type() != ActionType::EndTurn)
        .collect();

    // No draw is taken when there is no choice to make, so the rng cursor tracks decisions only.
    if others.is_empty() {
        return end_turn;
    }
    if end_turn.is_some() && rng.chance(AI_END_TURN_PROBABILITY) {
        return end_turn;
    }
    let at = rng.int(others.len() as i32);
    usize::try_from(at).ok().and_then(|at| others.get(at).cloned())
}

/// Why a playout stopped. Everything but `StepCap` and `Rejected` is an ordinary finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayoutStop {
    TurnEnded,
    PromptElsewhere,
    GameOver,
    NoAction,
    Rejected,
    StepCap,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayoutResult {
    /// The actions taken, in order, so a caller can log or animate them.
    pub actions: Vec<Action>,
    pub stopped: PlayoutStop,
    /// Set only with `Rejected`: `legal_actions` offered an action the reducer refused.
    pub error: Option<String>,
}

impl PlayoutResult {
    fn stopped(actions: Vec<Action>, stopped: PlayoutStop) -> PlayoutResult {
        PlayoutResult {
            actions,
            stopped,
            error: None,
        }
    }
}

/// A nonce no earlier action used (the reducer dedupes by nonce, §9.3), numbered from the state
/// itself so replay stays exact.
fn nonce_for(state: &GameState, player: PlayerId, step: usize) -> String {
    let base = format!("ai:{player}:t{}:s{step}", state.turn);
    let mut nonce = base.clone();
    let mut n = 1;
    while state.applied.iter().any(|entry| entry.nonce == nonce) {
        nonce = format!("{base}#{n}");
        n += 1;
    }
    nonce
}

/// `reduce` hands back a fresh state: copy it into the sink so the caller (and any enclosing `reduce`)
/// holds it; every instance is a new one, so re-read from `sink.state`. Except `applied` (§9.3's nonce
/// dedupe, §10.8's event window): the playout's events reach the enclosing action's entry through the
/// sink (R168), so adopting its entries would show each event twice (§10.10).
fn adopt_state(sink: &mut EngineSink<'_>, next: GameState) {
    let applied = std::mem::take(&mut sink.state.applied);
    *sink.state = next;
    sink.state.applied = applied;
}

/// The action reduced on the sink's own rng. `reduce` takes no rng (SURFACE §6.1) and rebuilds it from
/// `(state.seed, state.rng_cursor)`, so the sink's cursor goes in with the state and the sink's rng is
/// rebuilt from the result's. Exact when the sink's rng is the match's own stream; a side stream (a
/// scorer dry run's) cannot be handed through and is left where it stood.
fn reduce_on_sink_rng(sink: &mut EngineSink<'_>, action: &Action) -> ReduceResult {
    let own_stream = Rng::new(&sink.state.seed, sink.rng.cursor()) == *sink.rng;
    if own_stream {
        sink.state.rng_cursor = sink.rng.cursor();
    }
    let result = crate::reduce::reduce(sink.state, action);
    if own_stream && result.error.is_none() {
        *sink.rng = Rng::new(&result.state.seed, result.state.rng_cursor);
    }
    result
}

/// Play this player's turn out with the policy (R44: "it plays out the turn while the opponent is
/// locked out"). Stops when the turn it started on is over, when the open prompt is somebody else's,
/// when the game is over, or at AI_PLAYOUT_STEP_CAP.
pub fn play_out_turn(sink: &mut EngineSink<'_>, player: PlayerId, options: PolicyOptions) -> PlayoutResult {
    let mut actions: Vec<Action> = Vec::new();
    // R44, R152: only THIS turn. R82 can hand the next turn straight back to the same player inside
    // one reduction, and that later turn is the player's own.
    let turn = sink.state.turn;

    for step in 0..AI_PLAYOUT_STEP_CAP {
        if sink.state.result.is_some() {
            return PlayoutResult::stopped(actions, PlayoutStop::GameOver);
        }
        if sink.state.turn != turn {
            return PlayoutResult::stopped(actions, PlayoutStop::TurnEnded);
        }
        if let Some(pending) = &sink.state.pending {
            // Someone else's prompt blocks every action of ours (§9.3); a turn handed to the AI is
            // owed, so the answer brings it back (R113, R44).
            if pending.player_id != player {
                if sink.state.players[player].ai_turn {
                    owe_ai_turn(sink, player, turn);
                }
                return PlayoutResult::stopped(actions, PlayoutStop::PromptElsewhere);
            }
        } else if sink.state.active != player || sink.state.phase != Phase::Main {
            return PlayoutResult::stopped(actions, PlayoutStop::TurnEnded);
        }

        let Some(chosen) = choose_action_with(sink.state, player, sink.rng, options.clone()) else {
            return PlayoutResult::stopped(actions, PlayoutStop::NoAction);
        };

        let nonce = nonce_for(sink.state, player, step);
        let action = Action::new(chosen, player, nonce);
        let result = reduce_on_sink_rng(sink, &action);
        if let Some(error) = result.error {
            return PlayoutResult {
                actions,
                stopped: PlayoutStop::Rejected,
                error: Some(error),
            };
        }

        adopt_state(sink, result.state);
        // §10.3: `reduce`'s own loop has run these, so they are reported (R168's window), never
        // offered to the traps and triggers again.
        let delivered = result.events.clone();
        sink.events.extend(result.events);
        crate::triggers::mark_dispatched(sink, &delivered);
        actions.push(action);
    }

    PlayoutResult::stopped(actions, PlayoutStop::StepCap)
}

/// R113: the `resume.hook` of an AI turn another player's prompt stopped. One of #96 My Pawn's AI
/// actions can ask the other player a question (a trap, a Death hook); the turn it interrupted is
/// still the AI's to finish (R44, R152's lockout). The item is parked as the playout stops (R117),
/// behind what the AI's own action owes.
pub const AI_TURN_WORK: &str = "@aiTurn";

fn owe_ai_turn(sink: &mut EngineSink<'_>, player: PlayerId, turn: i32) {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("player".to_string(), json!(player));
    data.insert("turn".to_string(), json!(turn));
    let _ = crate::work::owe(
        sink,
        Resume {
            def_id: String::new(),
            hook: AI_TURN_WORK.to_string(),
            step: "turn".to_string(),
            radiant: false,
            instance_id: None,
            data,
        },
    );
}

/// `work.rs`'s handler for `AI_TURN_WORK`: the same AI turn, going on once the answer's events and
/// the triggers they wake have resolved (§10.3). What is owed after this item is the enclosing
/// sequences' (My Pawn's firing, the window, the combat it canceled), so it waits aside while the
/// playout's `reduce`s settle (R117) and comes back behind whatever the playout leaves owed.
pub fn run_owed_ai_turn(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let player = match item.resume.data.get("player").and_then(Value::as_str) {
        Some("p1") => PlayerId::P1,
        Some("p2") => PlayerId::P2,
        _ => return,
    };
    let turn = item.resume.data.get("turn").and_then(Value::as_i64);
    if sink.state.result.is_some() || turn != Some(i64::from(sink.state.turn)) {
        return;
    }
    let turn = sink.state.turn;
    if !sink.state.players[player].ai_turn {
        return;
    }

    let after = std::mem::take(&mut sink.state.work);
    sink.state.work_cursor = 0;
    continue_owed_ai_turn(sink, player, turn);
    sink.state.work.extend(after);
}

fn continue_owed_ai_turn(sink: &mut EngineSink<'_>, player: PlayerId, turn: i32) {
    crate::triggers::settle(sink, Default::default());
    if sink.state.result.is_some() {
        return;
    }
    // A question for the other player waits for its answer; the playout answers the AI's (R44).
    if sink
        .state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.player_id != player)
    {
        owe_ai_turn(sink, player, turn);
        return;
    }
    play_out_turn(sink, player, PolicyOptions::default());
}
