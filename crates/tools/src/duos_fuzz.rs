//! `cargo jackioh fuzz --duos [--seeds N] [--from S]` (#936 S5, B25): random-policy Duos games, each
//! held to the gate's assertions as a Duos game can meet them:
//!
//!   1. NO THROW: playing the game out raises nothing, and every action `duos_legal_actions` offered
//!      is one `duos_reduce` accepts.
//!   2. TERMINATION: the game ends — a pool emptied, a duel's own ending, or R2's turn cap — inside a
//!      bounded number of actions. The policy never concedes (R84), so `concede` and the server's
//!      `ceiling` cannot occur.
//!   3. REPLAY EQUALITY: `duos_fold` of `(options, log)` refuses nothing and reaches the live hash.
//!
//! A seed fixes the game: the four decks are one seeded shuffle of the gate's pool (`FUZZ_POOL`, so
//! 80 distinct cards), and the policy's picks come from streams of the seed's own, outside the state
//! (`fuzz.rs`'s header says why that is what replay equality proves). The policy is §10.7's over a
//! Duos seat's legal actions: a seat that may act is drawn uniformly, it ends its turn when nothing
//! else is on offer or with AI_END_TURN_PROBABILITY, and otherwise takes one of the rest, a pass
//! included, uniformly; it never emotes (`AI_SKIPPED_ACTIONS`).

use std::panic::{self, AssertUnwindSafe};
use std::time::Instant;

use anyhow::bail;
use indexmap::IndexMap;
use jackioh_cards::register_all;
use jackioh_engine::subsystems::AI_SKIPPED_ACTIONS;
use jackioh_engine::testkit::{
    DuosInvariantMonitor, I6_GATE_STRIDE, create_duos_invariant_monitor, preview_every_set,
};
use jackioh_engine::{
    AI_END_TURN_PROBABILITY, AI_PLAYOUT_STEP_CAP, ActionBody, DECK_SIZE, DUOS_DUELS, DUOS_REASON_DUEL_RESULT,
    DUOS_REASON_POOL_EMPTIED, DUOS_REASON_TURN_CAP, DuosAction, DuosActionBody, DuosCreateOptions, DuosDecks,
    DuosReplayInput, DuosSeat, DuosState, Rng, TURN_CAP_PLAYER_TURNS, duos_begin, duos_create, duos_fold,
    duos_hash, duos_legal_actions, duos_reduce, duos_seats_to_act,
};
use rayon::prelude::*;

use crate::agent::panic_message;
use crate::fuzz::{
    CHECKPOINT_TIME_BOUND_MS, DUOS_GROWTH_EVENTS_BOUND, DUOS_GROWTH_INSTANCES_BOUND,
    DUOS_GROWTH_NUMBER_BOUND, FUZZ_POOL, Heaviest, Heavy, MEASURES, checkpoint_step, duos_acting_card,
    duos_growth_breach, duos_heaviest_lines, state_paused, weigh_duos,
};

/// B25: the seeds `cargo jackioh fuzz --duos` plays when `--seeds` is not given.
pub const DUOS_WAVE_SEEDS: u32 = 200;

/// The seeds the tools crate's own test plays: the same games, fewer of them.
pub const DUOS_SWEEP_SEEDS: u32 = 3;

/// The endings a random-policy Duos game can reach (the policy never concedes, R84).
const DUOS_TERMINAL_REASONS: &[&str] = &[
    DUOS_REASON_POOL_EMPTIED,
    DUOS_REASON_DUEL_RESULT,
    DUOS_REASON_TURN_CAP,
];

/// Plays the Duos wave from seed `from` and prints its numbers; fails (exit 1) when a seed failed.
/// With `json`, prints the wave as one JSON object instead of the text summary.
pub fn run(from: u32, seeds: Option<u32>, json: bool) -> anyhow::Result<()> {
    let ran = duos_wave_size(seeds);
    let outcomes = play_duos_wave(from, ran);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&wave_json(from, ran, &outcomes))?
        );
    } else {
        println!("{}", summary(from, ran, &outcomes));
    }
    let failures = failure_lines(&outcomes);
    if !failures.is_empty() {
        if !json {
            println!("{}", failures.join("\n"));
        }
        bail!("fuzz-duos: {} of {ran} seed(s) failed", failures.len());
    }
    Ok(())
}

/// `--seeds N`, else DUOS_WAVE_SEEDS (DUOS_SWEEP_SEEDS under `cargo test`).
fn duos_wave_size(raw: Option<u32>) -> u32 {
    match raw {
        Some(parsed) if parsed >= 1 => parsed,
        _ => {
            if cfg!(test) {
                DUOS_SWEEP_SEEDS
            } else {
                DUOS_WAVE_SEEDS
            }
        }
    }
}

/// One seeded shuffle of the pool, a deck of DECK_SIZE cards to each seat in seat order (A1, A2, B1,
/// B2), so no card is in two decks.
pub fn duos_decks_for_seed(seed: u32) -> DuosDecks {
    let mut rng = Rng::new(&format!("jackioh-duos-fuzz-decks-{seed}"), 0);
    let shuffled = rng.shuffle(FUZZ_POOL.as_slice());
    let size = usize::try_from(DECK_SIZE).unwrap_or(0);
    let deck = |at: usize| shuffled[at * size..(at + 1) * size].to_vec();
    DuosDecks {
        a1: deck(0),
        a2: deck(1),
        b1: deck(2),
        b2: deck(3),
    }
}

/// Actions one Duos game may take before it is declared non-terminating: the 1v1 gate's bound for
/// each duel (R2's cap times a turn's playout cap), so it only ever fires on a genuine loop.
fn max_duos_actions() -> usize {
    usize::try_from(TURN_CAP_PLAYER_TURNS).unwrap_or(0) * AI_PLAYOUT_STEP_CAP * DUOS_DUELS
}

/// One played game: the setup, the log and where it ended.
struct DuosRun {
    options: DuosCreateOptions,
    log: Vec<DuosAction>,
    state: DuosState,
    /// This game's heaviest action by each measure (#1364).
    heaviest: Heaviest,
    /// The prefix lengths whose end state held paused work in some duel (#1387): step 0 is the
    /// setup state, otherwise the state after that many actions.
    paused: Vec<usize>,
}

/// Whether some duel holds paused work (SPEC §10.1): the Duos form of the gate's `state_paused`.
fn duos_paused(state: &DuosState) -> bool {
    state.duels.iter().any(state_paused)
}

/// A `DuosActionBody`'s wire name: a `Duel` action's inner `ActionType`, or the pass/concede itself.
fn duos_action_name(body: &DuosActionBody) -> String {
    match body {
        DuosActionBody::Duel { body } => body.action_type().as_str().to_string(),
        DuosActionBody::Pass { .. } => "pass".to_string(),
        DuosActionBody::Concede => "concede".to_string(),
    }
}

/// §10.7's policy over one seat's Duos actions: never a concede or an emote; End turn when nothing
/// else is on offer or with AI_END_TURN_PROBABILITY; otherwise one of the rest, uniformly.
fn choose(state: &DuosState, seat: DuosSeat, rng: &mut Rng) -> Option<DuosActionBody> {
    let is_end = |action: &DuosActionBody| {
        matches!(
            action,
            DuosActionBody::Duel {
                body: ActionBody::EndTurn
            }
        )
    };
    let actions: Vec<DuosActionBody> = duos_legal_actions(state, seat)
        .into_iter()
        .filter(|action| match action {
            DuosActionBody::Concede => false,
            DuosActionBody::Duel { body } => !AI_SKIPPED_ACTIONS.contains(&body.action_type()),
            DuosActionBody::Pass { .. } => true,
        })
        .collect();
    let end = actions.iter().find(|action| is_end(action)).cloned();
    let others: Vec<DuosActionBody> = actions.into_iter().filter(|action| !is_end(action)).collect();
    if others.is_empty() {
        return end;
    }
    if end.is_some() && rng.chance(AI_END_TURN_PROBABILITY) {
        return end;
    }
    rng.pick(&others).cloned()
}

/// Assertion 1: one game played out, every offered action accepted.
fn play_duos_game(seed: u32) -> Result<DuosRun, String> {
    play_duos_game_inner(seed, |_, _, _, _| {})
}

fn play_duos_game_inner<F>(seed: u32, mut tamper: F) -> Result<DuosRun, String>
where
    F: FnMut(usize, &mut DuosState, &mut DuosAction, &mut DuosInvariantMonitor),
{
    register_all();
    let options = DuosCreateOptions {
        seed: format!("jackioh-duos-fuzz-{seed}"),
        decks: duos_decks_for_seed(seed),
        ..DuosCreateOptions::default()
    };
    let begun = duos_begin(&duos_create(&options));
    let mut state = begun.state;
    let mut monitor = create_duos_invariant_monitor(&state);
    let stride = if cfg!(test) { 1 } else { I6_GATE_STRIDE };
    let mut setup = monitor.after(&begun.events, &state);
    setup.extend(monitor.hidden(&state));
    if let Some(first) = setup.first() {
        return Err(format!("at step 0: {first}"));
    }
    // #1387: the checkpoint prefers a state with paused work; step 0 is the setup state.
    let mut paused_steps: Vec<usize> = Vec::new();
    if duos_paused(&state) {
        paused_steps.push(0);
    }

    let mut policy = Rng::new(&format!("jackioh-duos-fuzz-policy-{seed}"), 0);
    let mut order = Rng::new(&format!("jackioh-duos-fuzz-order-{seed}"), 0);
    let mut log: Vec<DuosAction> = Vec::new();
    let mut heaviest = Heaviest::default();
    let max_actions = max_duos_actions();

    while state.result.is_none() {
        if log.len() >= max_actions {
            return Err(format!(
                "no ending within {max_actions} actions (round {})",
                state.round
            ));
        }
        let seats = duos_seats_to_act(&state);
        let Some(&seat) = order.pick(&seats) else {
            return Err(format!(
                "no seat may act in round {} while the game is live: a stall",
                state.round
            ));
        };
        let Some(body) = choose(&state, seat, &mut policy) else {
            return Err(format!(
                "{seat} was waited on but chose nothing in round {}",
                state.round
            ));
        };
        let mut action = DuosAction {
            seat,
            nonce: format!("duos-fuzz-{}", log.len()),
            body,
        };

        tamper(log.len(), &mut state, &mut action, &mut monitor);

        let unsound = monitor.before(&state, action.seat, &action.body);
        if let Some(first) = unsound.first() {
            return Err(format!("at step {}: {first}", log.len()));
        }

        let result = duos_reduce(&state, &action);
        if let Some(error) = result.error {
            return Err(format!(
                "duos_legal_actions offered {} {:?} but duos_reduce refused it: {error}",
                seat, action.body
            ));
        }
        let action_type = duos_action_name(&action.body);
        let card = duos_acting_card(&state, action.seat, &action.body);
        log.push(action);
        state = result.state;
        if duos_paused(&state) {
            paused_steps.push(log.len());
        }

        let mut broken = monitor.after(&result.events, &state);
        if log.len().is_multiple_of(stride) || state.result.is_some() {
            broken.extend(monitor.hidden(&state));
        }
        if let Some(first) = broken.first() {
            return Err(format!("at step {}: {first}", log.len()));
        }

        // #1364: weigh the action; one past a growth bound fails the seed here.
        let step = log.len();
        let weights = weigh_duos(&result.events, &state);
        for (measure, value) in MEASURES.into_iter().zip(weights) {
            if heaviest.list(measure).first().is_none_or(|top| value > top.value) {
                let heavy = Heavy {
                    seed,
                    step,
                    action: action_type.clone(),
                    card: card.clone(),
                    value,
                };
                heaviest.keep(measure, heavy, 1);
            }
        }
        if let Some(message) = duos_growth_breach(&weights, &state, step, &action_type, card.as_deref()) {
            return Err(format!("growth: {message}"));
        }
    }
    Ok(DuosRun {
        options,
        log,
        state,
        heaviest,
        paused: paused_steps,
    })
}

/// Assertions 2 and 3: a real ending, and a fold that refuses nothing and reaches the live hash.
fn check_duos_run(run: &DuosRun) -> Result<String, String> {
    let Some(result) = &run.state.result else {
        return Err("the game loop exited with no result".to_string());
    };
    if !DUOS_TERMINAL_REASONS.contains(&result.reason.as_str()) {
        return Err(format!(
            "ended with reason \"{}\"; the policy never concedes (R84), so only {} can occur",
            result.reason,
            DUOS_TERMINAL_REASONS.join(", ")
        ));
    }
    if let Some(game) = run
        .state
        .duels
        .iter()
        .find(|game| game.turn > TURN_CAP_PLAYER_TURNS)
    {
        return Err(format!(
            "a duel reached turn {}, past the R2 cap of {TURN_CAP_PLAYER_TURNS}",
            game.turn
        ));
    }
    let replayed = duos_fold(&DuosReplayInput {
        options: run.options.clone(),
        log: run.log.clone(),
    });
    if let Some(at) = replayed.refused_at {
        return Err(format!(
            "the fold refused action {at} of {} ({:?})",
            run.log.len(),
            run.log.get(at).map(|action| &action.body)
        ));
    }
    let live = duos_hash(&run.state);
    if replayed.hash != live {
        return Err(format!(
            "fold hash {} != live hash {live} after {} actions",
            replayed.hash,
            run.log.len()
        ));
    }
    if replayed.state.result != run.state.result {
        return Err("the fold reached a different ending".to_string());
    }
    Ok(result.reason.clone())
}

// ---------------------------------------------------------------------------------------------
// The serialization checkpoint (#1387)
// ---------------------------------------------------------------------------------------------

/// The Duos game survives a serialized restart: folds the log's prefix to the seed-drawn
/// checkpoint step (paused work preferred, the gate's `checkpoint_step`), round-trips that state
/// through JSON requiring the identical hash, then applies the rest of the log from the restored
/// state and requires the live hash and ending. Failures carry the `checkpoint:` prefix the way
/// growth failures carry `growth:`. Returns the step, whether it held paused work, and the cost
/// in microseconds.
fn check_duos_serial_restart(
    options: &DuosCreateOptions,
    log: &[DuosAction],
    live: &DuosState,
    stream: &str,
    paused: &[usize],
) -> Result<(usize, bool, u128), String> {
    let started_at = Instant::now();
    let (step, was_paused) = checkpoint_step(stream, paused, log.len());
    let prefix = duos_fold(&DuosReplayInput {
        options: options.clone(),
        log: log[..step].to_vec(),
    });
    if let Some(at) = prefix.refused_at {
        return Err(format!(
            "checkpoint: the prefix fold refused action {at} of {} ({:?})",
            log.len(),
            log.get(at).map(|action| &action.body)
        ));
    }
    let text = serde_json::to_string(&prefix.state)
        .map_err(|error| format!("checkpoint: the state did not serialise: {error}"))?;
    let restored: DuosState = serde_json::from_str(&text)
        .map_err(|error| format!("checkpoint: the state did not parse back: {error}"))?;
    if duos_hash(&restored) != prefix.hash {
        return Err(format!(
            "checkpoint: the round trip changed the state: hash {} != {}",
            duos_hash(&restored),
            prefix.hash
        ));
    }
    let mut state = restored;
    for action in &log[step..] {
        let result = duos_reduce(&state, action);
        if let Some(error) = result.error {
            return Err(format!(
                "checkpoint: the continuation refused action {:?} of {}: {error}",
                action.body,
                log.len()
            ));
        }
        state = result.state;
    }
    let live_hash = duos_hash(live);
    if duos_hash(&state) != live_hash {
        return Err(format!(
            "checkpoint: fold hash {} != live hash {live_hash} after {} actions",
            duos_hash(&state),
            log.len()
        ));
    }
    if state.result != live.result {
        return Err("checkpoint: the restored game reached a different ending".to_string());
    }
    let micros = started_at.elapsed().as_micros();
    if micros > CHECKPOINT_TIME_BOUND_MS * 1000 {
        return Err(format!(
            "checkpoint: the restart took {}ms, past CHECKPOINT_TIME_BOUND_MS ({CHECKPOINT_TIME_BOUND_MS}ms)",
            micros / 1000
        ));
    }
    Ok((step, was_paused, micros))
}

/// One seed's verdict.
#[derive(Clone, Debug)]
struct DuosSeedOutcome {
    seed: u32,
    reason: Option<String>,
    error: Option<String>,
    /// The passing game's heaviest action by each measure (#1364); a failed seed keeps none.
    heaviest: Heaviest,
    /// Whether the game's checkpoint restarted from paused work (#1387); false when it failed.
    checkpoint_paused: bool,
    /// What the game's checkpoint cost, in microseconds (#1387); zero when it failed.
    checkpoint_micros: u128,
}

/// The checkpoint a seed crossed, after its run passed: the step is unused here, only whether it
/// held paused work and what it cost.
fn check_seed_checkpoint(seed: u32, run: &DuosRun) -> Result<(bool, u128), String> {
    check_duos_serial_restart(
        &run.options,
        &run.log,
        &run.state,
        &format!("jackioh-duos-fuzz-checkpoint-{seed}"),
        &run.paused,
    )
    .map(|(_, paused, micros)| (paused, micros))
}

fn fuzz_duos_seed(seed: u32) -> DuosSeedOutcome {
    // R1420: the pool is the whole catalog, so this worker thread previews every set for the game.
    let _preview = preview_every_set();
    let caught = panic::catch_unwind(AssertUnwindSafe(
        || -> Result<(String, Heaviest, bool, u128), String> {
            let run = play_duos_game(seed)?;
            let reason = check_duos_run(&run)?;
            let (paused, micros) = check_seed_checkpoint(seed, &run)?;
            Ok((reason, run.heaviest, paused, micros))
        },
    ));
    match caught {
        Ok(Ok((reason, heaviest, checkpoint_paused, checkpoint_micros))) => DuosSeedOutcome {
            seed,
            reason: Some(reason),
            error: None,
            heaviest,
            checkpoint_paused,
            checkpoint_micros,
        },
        Ok(Err(error)) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(error),
            heaviest: Heaviest::default(),
            checkpoint_paused: false,
            checkpoint_micros: 0,
        },
        Err(payload) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(format!("panic: {}", panic_message(&*payload))),
            heaviest: Heaviest::default(),
            checkpoint_paused: false,
            checkpoint_micros: 0,
        },
    }
}

#[cfg(test)]
fn fuzz_duos_seed_with<F>(seed: u32, tamper: F) -> DuosSeedOutcome
where
    F: FnMut(usize, &mut DuosState, &mut DuosAction, &mut DuosInvariantMonitor),
{
    let _preview = preview_every_set();
    let caught = panic::catch_unwind(AssertUnwindSafe(
        || -> Result<(String, Heaviest, bool, u128), String> {
            let run = play_duos_game_inner(seed, tamper)?;
            let reason = check_duos_run(&run)?;
            let (paused, micros) = check_seed_checkpoint(seed, &run)?;
            Ok((reason, run.heaviest, paused, micros))
        },
    ));
    match caught {
        Ok(Ok((reason, heaviest, checkpoint_paused, checkpoint_micros))) => DuosSeedOutcome {
            seed,
            reason: Some(reason),
            error: None,
            heaviest,
            checkpoint_paused,
            checkpoint_micros,
        },
        Ok(Err(error)) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(error),
            heaviest: Heaviest::default(),
            checkpoint_paused: false,
            checkpoint_micros: 0,
        },
        Err(payload) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(format!("panic: {}", panic_message(&*payload))),
            heaviest: Heaviest::default(),
            checkpoint_paused: false,
            checkpoint_micros: 0,
        },
    }
}

/// Seeds `from..from + ran`, in parallel, read back in seed order.
fn play_duos_wave(from: u32, ran: u32) -> Vec<DuosSeedOutcome> {
    register_all();
    (from..from + ran).into_par_iter().map(fuzz_duos_seed).collect()
}

fn failure_lines(outcomes: &[DuosSeedOutcome]) -> Vec<String> {
    outcomes
        .iter()
        .filter_map(|outcome| {
            outcome
                .error
                .as_ref()
                .map(|error| format!("seed {}: {error}", outcome.seed))
        })
        .collect()
}

/// The heaviest actions of the games that passed, one per game (#1364): a failed seed adds
/// nothing, as the gate's wave does.
fn passed_heaviest(outcomes: &[DuosSeedOutcome]) -> Heaviest {
    let mut heaviest = Heaviest::default();
    for outcome in outcomes.iter().filter(|outcome| outcome.error.is_none()) {
        heaviest.merge(&outcome.heaviest);
    }
    heaviest
}

fn summary(from: u32, ran: u32, outcomes: &[DuosSeedOutcome]) -> String {
    let failed = outcomes.iter().filter(|outcome| outcome.error.is_some()).count();
    let mut endings: IndexMap<String, usize> = IndexMap::new();
    for outcome in outcomes {
        if let Some(reason) = &outcome.reason {
            *endings.entry(reason.clone()).or_insert(0) += 1;
        }
    }
    let passed = outcomes.iter().filter(|outcome| outcome.error.is_none());
    let paused = passed.clone().filter(|outcome| outcome.checkpoint_paused).count();
    let slowest_ms = outcomes
        .iter()
        .filter(|outcome| outcome.error.is_none())
        .map(|outcome| outcome.checkpoint_micros / 1000)
        .max()
        .unwrap_or(0);
    format!(
        "[fuzz-duos] {ran} seed(s) from {from}: {failed} failed; endings {}\n  checkpoints: {paused} paused, {} mid-game fallback; slowest {slowest_ms}ms of {CHECKPOINT_TIME_BOUND_MS}ms\n{}",
        serde_json::to_string(&endings).unwrap_or_default(),
        outcomes.len() - failed - paused,
        duos_heaviest_lines(&passed_heaviest(outcomes)),
    )
}

/// `--duos --json`: the wave's failures, each measure's heaviest actions and the growth bounds
/// (#1364). Failures carry only seed and message: Duos has no stage taxonomy, so the `growth:`
/// prefix in the message is the stage name.
fn wave_json(from: u32, ran: u32, outcomes: &[DuosSeedOutcome]) -> serde_json::Value {
    let passed: Vec<&DuosSeedOutcome> = outcomes
        .iter()
        .filter(|outcome| outcome.error.is_none())
        .collect();
    let paused = passed.iter().filter(|outcome| outcome.checkpoint_paused).count();
    let slowest_ms = passed
        .iter()
        .map(|outcome| outcome.checkpoint_micros / 1000)
        .max()
        .unwrap_or(0);
    serde_json::json!({
        "from": from,
        "ran": ran,
        "passed": passed.len(),
        "failures": outcomes.iter().filter_map(|outcome| outcome.error.as_ref().map(|error| serde_json::json!({
            "seed": outcome.seed,
            "message": error,
        }))).collect::<Vec<_>>(),
        "heaviest": passed_heaviest(outcomes),
        "bounds": {
            "events": DUOS_GROWTH_EVENTS_BOUND,
            "largest": DUOS_GROWTH_NUMBER_BOUND,
            "instances": DUOS_GROWTH_INSTANCES_BOUND,
        },
        "checkpoint": {
            "paused": paused,
            "midGame": passed.len() - paused,
            "slowestMs": u64::try_from(slowest_ms).unwrap_or(u64::MAX),
            "boundMs": u64::try_from(CHECKPOINT_TIME_BOUND_MS).unwrap_or(u64::MAX),
        },
    })
}

#[cfg(test)]
mod tests {
    use jackioh_engine::DUOS_SEATS;

    use super::*;

    #[test]
    fn the_four_decks_come_from_the_seed_alone_and_share_no_card() {
        let decks = duos_decks_for_seed(7);
        assert_eq!(decks, duos_decks_for_seed(7));
        let size = usize::try_from(DECK_SIZE).unwrap_or(0);
        let mut all: Vec<&String> = Vec::new();
        for seat in DUOS_SEATS {
            assert_eq!(decks.of(seat).len(), size);
            all.extend(decks.of(seat).iter());
        }
        let distinct: indexmap::IndexSet<&String> = all.iter().copied().collect();
        assert_eq!(distinct.len(), size * DUOS_SEATS.len());
        assert_ne!(duos_decks_for_seed(1), duos_decks_for_seed(2));
    }

    #[test]
    fn a_duos_smoke_wave_ends_every_game_and_folds_it_back() {
        let outcomes = play_duos_wave(1, duos_wave_size(None));
        assert_eq!(failure_lines(&outcomes), Vec::<String>::new());
    }

    #[test]
    fn a_planted_sick_attack_fails_duos_fuzz_naming_seed_duel_and_step() {
        use jackioh_engine::{Phase, PlayerId, Row, Zone};
        let outcome = fuzz_duos_seed_with(7, |step, state, action, monitor| {
            if state.round >= 1 && state.duels[0].phase == Phase::Main && step == 4 {
                // Moved, not copied: a copy left in hand shares the id, and both the monitor and
                // the reducer would find that copy (whatever the seed's deck put there) first.
                let mut card = state.duels[0].players[PlayerId::P1].hand.remove(0);
                card.def_id = "core-001".to_string();
                card.granted_keywords.clear();
                card.zone = Zone::Field {
                    player: PlayerId::P1,
                    row: Row::Units,
                    lane: 1,
                };
                card.summoned_turn = Some(state.duels[0].turn);
                let attacker_id = card.id.clone();
                state.duels[0].players[PlayerId::P1].units[0] = Some(vec![card]);
                monitor.duel_monitor_mut(0).enter(&attacker_id);
                action.seat = DuosSeat::A1;
                action.body = DuosActionBody::Duel {
                    body: ActionBody::Attack {
                        attacker_id,
                        target_id: "hero-p2".to_string(),
                    },
                };
            }
        });
        assert!(outcome.error.is_some(), "outcome should have failed");
        let line = format!("seed {}: {}", outcome.seed, outcome.error.unwrap());
        assert!(line.contains("seed 7"), "should name seed 7: {line}");
        assert!(line.contains("duel 0"), "should name duel 0: {line}");
        assert!(line.contains("step 4"), "should name step 4: {line}");
        assert!(
            line.contains("I1 sick attack"),
            "should cite I1 sick attack: {line}"
        );
    }

    mod duos_growth_stage {
        use std::cell::RefCell;
        use std::rc::Rc;

        use jackioh_engine::PlayerId;

        use super::*;
        use crate::fuzz::DUOS_GROWTH_NUMBER_BOUND;

        #[test]
        fn a_planted_growing_number_fails_duos_fuzz_in_the_growth_stage() {
            let seen: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
            let seen_in = Rc::clone(&seen);
            let outcome = fuzz_duos_seed_with(1, move |step, state, action, _monitor| {
                if step == 0 {
                    // Library, never hand: legal actions and monitors are undisturbed, but every
                    // weighed action afterwards sees the planted number.
                    state.duels[0].players[PlayerId::P1].library[0].buffs.attack =
                        i32::try_from(DUOS_GROWTH_NUMBER_BOUND + 1).unwrap_or(i32::MAX);
                    *seen_in.borrow_mut() = Some(duos_action_name(&action.body));
                }
            });
            let error = outcome.error.expect("the planted growth must fail the seed");
            let action = seen.borrow().clone().expect("step 0 must record its action");
            assert!(error.contains("growth"), "{error}");
            assert!(error.contains(&action), "{error}");
            assert!(error.contains("largest"), "{error}");
            assert!(error.contains("DUOS_GROWTH_NUMBER_BOUND"), "{error}");
        }

        #[test]
        fn the_same_seed_passes_once_the_growth_is_removed() {
            let outcome = fuzz_duos_seed_with(1, |_, _, _, _| {});
            assert_eq!(outcome.error, None);
        }

        #[test]
        fn the_summary_and_json_list_each_measure_s_heaviest_actions() {
            let heavy = |seed: u32, value: i64| Heavy {
                seed,
                step: 3,
                action: "pass".to_string(),
                card: None,
                value,
            };
            let mut game = Heaviest::default();
            for (measure, value) in MEASURES.into_iter().zip([10, 20, 30]) {
                game.keep(measure, heavy(1, value), 1);
            }
            let outcomes = vec![DuosSeedOutcome {
                seed: 1,
                reason: Some(DUOS_REASON_POOL_EMPTIED.to_string()),
                error: None,
                heaviest: game,
                checkpoint_paused: true,
                checkpoint_micros: 2_000,
            }];
            let summary = summary(1, 1, &outcomes);
            assert!(summary.contains("1/3 pass -: 10"), "{summary}");
            assert!(summary.contains("1/3 pass -: 20"), "{summary}");
            assert!(summary.contains("1/3 pass -: 30"), "{summary}");
            assert!(summary.contains("1 paused, 0 mid-game fallback"), "{summary}");
            let json = wave_json(1, 1, &outcomes);
            assert_eq!(json["heaviest"]["events"][0]["value"], serde_json::json!(10));
            assert_eq!(json["checkpoint"]["paused"], serde_json::json!(1));
            assert_eq!(json["checkpoint"]["midGame"], serde_json::json!(0));
            assert_eq!(
                json["bounds"]["largest"],
                serde_json::json!(DUOS_GROWTH_NUMBER_BOUND)
            );
            assert_eq!(json["passed"], serde_json::json!(1));
        }
    }

    #[test]
    fn a_mid_game_state_survives_a_serialized_restart() {
        let run = play_duos_game_inner(1, |_, _, _, _| {}).expect("seed 1 plays out");
        let (step, paused, micros) = check_duos_serial_restart(
            &run.options,
            &run.log,
            &run.state,
            "jackioh-duos-fuzz-checkpoint-1",
            &run.paused,
        )
        .expect("seed 1 crosses its checkpoint");
        assert!(step <= run.log.len(), "step {step}");
        assert_eq!(paused, !run.paused.is_empty());
        assert!(micros <= CHECKPOINT_TIME_BOUND_MS * 1000);
        // The wave records the same checkpoint for the seed.
        let outcome = fuzz_duos_seed(1);
        assert_eq!(outcome.error, None);
        assert_eq!(outcome.checkpoint_paused, paused);
        assert!(outcome.checkpoint_micros <= CHECKPOINT_TIME_BOUND_MS * 1000);
    }

    #[test]
    fn a_tampered_duos_round_trip_is_detected() {
        let run = play_duos_game_inner(1, |_, _, _, _| {}).expect("seed 1 plays out");
        let live = duos_hash(&run.state);
        let mut value = serde_json::to_value(&run.state).expect("the state serialises");
        let health = value
            .get_mut("duels")
            .and_then(|duels| duels.get_mut(0))
            .and_then(|duel| duel.get_mut("players"))
            .and_then(|players| players.get_mut("p1"))
            .and_then(|p1| p1.get_mut("hero"))
            .and_then(|hero| hero.get_mut("health"));
        let Some(health) = health.and_then(|health| health.as_i64()) else {
            panic!("duels[0].players.p1.hero.health is a number");
        };
        value["duels"][0]["players"]["p1"]["hero"]["health"] = serde_json::json!(health + 1);
        let tampered: DuosState = serde_json::from_value(value).expect("the tampered state parses back");
        assert_ne!(
            duos_hash(&tampered),
            live,
            "adding one health must move the hash, or the comparison proves nothing"
        );
    }

    #[test]
    fn a_planted_view_leak_fails_duos_fuzz_naming_seed_duel_and_step() {
        let outcome = fuzz_duos_seed_with(7, |step, _state, _action, monitor| {
            if step == 1 {
                monitor.view_tamper = Some(std::sync::Arc::new(|view| {
                    if view.seat == DuosSeat::A1 {
                        view.enemy.ended = vec![true, false];
                    }
                }));
            }
        });
        assert!(outcome.error.is_some(), "outcome should have failed");
        let line = format!("seed {}: {}", outcome.seed, outcome.error.unwrap());
        assert!(line.contains("seed 7"), "should name seed 7: {line}");
        assert!(line.contains("duel 0"), "should name duel 0: {line}");
        assert!(line.contains("step 2"), "should name step 2: {line}");
        assert!(line.contains("I6 duos leak"), "should cite I6 duos leak: {line}");
    }
}
