//! IA6: the attempt's state machine, the day's record and the fold ([[R4579002]]). A refused
//! action changes nothing, as a dungeon run's does.

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::config::{HUMAN_HANDICAP, MAX_COPIES};
use jackioh_engine::replay::ReplayInput;
use jackioh_engine::state::validate_deck_with;
use jackioh_engine::wire::PlayerId;

use crate::assault::{
    AssaultAction, AssaultDay, AssaultFightSetup, AssaultFile, AssaultPhase, AssaultRecord, AssaultSetup,
    AssaultState, checkpoints, commander_rank,
};
use crate::config::ASSAULT_LEVELS;
use crate::{RunAction, run_fight_report};

/// IA6: an attempt at its start: the setup checked against the day and the record. The deck is
/// checked at create, as a dungeon run's starting deck is: an attempt lasts one day, so
/// [[R3496000]]'s filtering is refused here instead ([[R4579002]]).
pub fn assault_create(
    setup: &AssaultSetup,
    record: &AssaultRecord,
    day: &AssaultDay,
) -> Result<AssaultState, String> {
    if setup.date != day.date {
        return Err(format!("the attempt is for {}, not for {}", setup.date, day.date));
    }
    if setup.date != record.date {
        return Err(format!(
            "the attempt is for {}, not for {}",
            setup.date, record.date
        ));
    }
    if setup.attempt != record.attempts + 1 {
        return Err(format!(
            "attempt {} follows {} attempts, not {}",
            setup.attempt, record.attempts, record.attempts
        ));
    }
    if setup.start_level != 1
        && !(checkpoints().contains(&setup.start_level) && record.checkpoints.contains(&setup.start_level))
    {
        return Err(format!(
            "level {} is no checkpoint reached today",
            setup.start_level
        ));
    }
    validate_deck_with(
        &setup.deck,
        registered_catalog(),
        "Your deck",
        HUMAN_HANDICAP.deck_size,
        MAX_COPIES,
    )
    .map_err(|error| error.to_string())?;
    Ok(AssaultState {
        setup: setup.clone(),
        level: setup.start_level,
        phase: AssaultPhase::Briefing,
        beaten: Vec::new(),
        turns: 0,
    })
}

/// IA6: the attempt after one action, or why it is refused, the state unchanged. A win moves to
/// the next level's briefing, or repels the Assault at the top; a loss ends the attempt lost. An
/// ended attempt refuses every action.
pub fn assault_reduce(
    state: &AssaultState,
    day: &AssaultDay,
    action: &AssaultAction,
) -> Result<AssaultState, String> {
    if state.setup.date != day.date {
        return Err(format!(
            "the attempt is for {}, not for {}",
            state.setup.date, day.date
        ));
    }
    if matches!(
        state.phase,
        AssaultPhase::Lost | AssaultPhase::Repelled | AssaultPhase::Abandoned
    ) {
        return Err("the attempt is over".into());
    }
    let mut next = state.clone();
    match (&state.phase, action) {
        (AssaultPhase::Briefing, AssaultAction::Fight) => {
            next.phase = AssaultPhase::Fight;
        }
        (AssaultPhase::Fight, AssaultAction::FightOver { won, turns }) => {
            if *turns < 0 {
                return Err(format!("a fight lasts no fewer than 0 turns, not {turns}"));
            }
            next.turns += turns;
            if *won {
                next.beaten.push(state.level);
                if state.level == ASSAULT_LEVELS {
                    next.phase = AssaultPhase::Repelled;
                } else {
                    next.level += 1;
                    next.phase = AssaultPhase::Briefing;
                }
            } else {
                next.phase = AssaultPhase::Lost;
            }
        }
        (AssaultPhase::Briefing | AssaultPhase::Fight, AssaultAction::Abandon) => {
            next.phase = AssaultPhase::Abandoned;
        }
        _ => {
            return Err(format!("no {:?} while {:?}", action_kind(action), state.phase));
        }
    }
    Ok(next)
}

/// The action's kind, for a refusal that names it.
fn action_kind(action: &AssaultAction) -> &'static str {
    match action {
        AssaultAction::Fight => "fight",
        AssaultAction::FightOver { .. } => "fightOver",
        AssaultAction::Abandon => "abandon",
    }
}

/// IA6: the day's record folded with a finished or running attempt: the larger attempt number, the
/// highest level beaten, each beaten Commander level below the top as its next level's
/// checkpoint, and the smaller repelling attempt. Idempotent: folding the same state twice reads
/// the same record.
pub fn assault_record(record: &AssaultRecord, state: &AssaultState) -> AssaultRecord {
    if state.setup.date != record.date {
        return record.clone();
    }
    let mut next = record.clone();
    next.attempts = next.attempts.max(state.setup.attempt);
    if let Some(best) = state.beaten.iter().max() {
        next.best = next.best.max(*best);
    }
    for beaten in &state.beaten {
        if commander_rank(*beaten).is_some() && *beaten < ASSAULT_LEVELS {
            let checkpoint = beaten + 1;
            if !next.checkpoints.contains(&checkpoint) {
                next.checkpoints.push(checkpoint);
            }
        }
    }
    next.checkpoints.sort_unstable();
    if state.phase == AssaultPhase::Repelled {
        next.repelled_on = Some(
            next.repelled_on
                .map_or(state.setup.attempt, |first| first.min(state.setup.attempt)),
        );
    }
    next
}

/// IA6: an attempt file replayed from its start, as `run_fold` replays a run's: create, then each
/// action, the error naming the first refused step.
pub fn assault_fold(file: &AssaultFile, day: &AssaultDay) -> Result<AssaultState, String> {
    let mut state = assault_create(&file.setup, &file.record, day)?;
    for (step, action) in file.actions.iter().enumerate() {
        state = assault_reduce(&state, day, action).map_err(|error| format!("step {step}: {error}"))?;
    }
    Ok(state)
}

/// IA6: the setup of the fight being played, `None` in any other phase, on another day, or at a
/// level the day does not hold. The human plays its deck at `HUMAN_HANDICAP` in P1, as a dungeon
/// fight's human does; the enemy plays the level's deck, handicap and marks.
pub fn assault_fight_setup(state: &AssaultState, day: &AssaultDay) -> Option<AssaultFightSetup> {
    if state.phase != AssaultPhase::Fight || state.setup.date != day.date {
        return None;
    }
    let level = day.levels.iter().find(|row| row.level == state.level)?.clone();
    Some(AssaultFightSetup {
        seed: format!("{}-L{}-A{}", day.seed, state.level, state.setup.attempt),
        human_handicap: jackioh_engine::config::Handicap {
            deck_size: state.setup.deck.len() as i32,
            ..HUMAN_HANDICAP
        },
        human_deck: state.setup.deck.clone(),
        enemy_deck: level.deck.clone(),
        enemy_handicap: level.handicap,
        enemy_marks: level.marks.clone(),
        level,
    })
}

/// IA6, IA15: the `AssaultAction::FightOver` a finished fight sends, folded from its record as a
/// dungeon fight's is. A draw at the turn cap is a loss: no winner is the human's win.
pub fn assault_fight_report(game: &ReplayInput, human: PlayerId) -> Result<AssaultAction, String> {
    let RunAction::FightOver { won, turns, .. } = run_fight_report(game, human)? else {
        return Err("the fight is not over: only a finished fight reports".into());
    };
    Ok(AssaultAction::FightOver { won, turns })
}
