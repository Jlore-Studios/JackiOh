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
use crate::fuzz::FUZZ_POOL;

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
pub fn run(from: u32, seeds: Option<u32>) -> anyhow::Result<()> {
    let ran = duos_wave_size(seeds);
    let outcomes = play_duos_wave(from, ran);
    let failures = failure_lines(&outcomes);
    println!("{}", summary(from, ran, &outcomes));
    if !failures.is_empty() {
        println!("{}", failures.join("\n"));
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

    let mut policy = Rng::new(&format!("jackioh-duos-fuzz-policy-{seed}"), 0);
    let mut order = Rng::new(&format!("jackioh-duos-fuzz-order-{seed}"), 0);
    let mut log: Vec<DuosAction> = Vec::new();
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
        log.push(action);
        state = result.state;

        let mut broken = monitor.after(&result.events, &state);
        if log.len().is_multiple_of(stride) || state.result.is_some() {
            broken.extend(monitor.hidden(&state));
        }
        if let Some(first) = broken.first() {
            return Err(format!("at step {}: {first}", log.len()));
        }
    }
    Ok(DuosRun { options, log, state })
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

/// One seed's verdict.
#[derive(Clone, Debug)]
struct DuosSeedOutcome {
    seed: u32,
    reason: Option<String>,
    error: Option<String>,
}

fn fuzz_duos_seed(seed: u32) -> DuosSeedOutcome {
    // R1420: the pool is the whole catalog, so this worker thread previews every set for the game.
    let _preview = preview_every_set();
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<String, String> {
        let run = play_duos_game(seed)?;
        check_duos_run(&run)
    }));
    match caught {
        Ok(Ok(reason)) => DuosSeedOutcome {
            seed,
            reason: Some(reason),
            error: None,
        },
        Ok(Err(error)) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(error),
        },
        Err(payload) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(format!("panic: {}", panic_message(&*payload))),
        },
    }
}

#[cfg(test)]
fn fuzz_duos_seed_with<F>(seed: u32, tamper: F) -> DuosSeedOutcome
where
    F: FnMut(usize, &mut DuosState, &mut DuosAction, &mut DuosInvariantMonitor),
{
    let _preview = preview_every_set();
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<String, String> {
        let run = play_duos_game_inner(seed, tamper)?;
        check_duos_run(&run)
    }));
    match caught {
        Ok(Ok(reason)) => DuosSeedOutcome {
            seed,
            reason: Some(reason),
            error: None,
        },
        Ok(Err(error)) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(error),
        },
        Err(payload) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(format!("panic: {}", panic_message(&*payload))),
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

fn summary(from: u32, ran: u32, outcomes: &[DuosSeedOutcome]) -> String {
    let failed = outcomes.iter().filter(|outcome| outcome.error.is_some()).count();
    let mut endings: IndexMap<String, usize> = IndexMap::new();
    for outcome in outcomes {
        if let Some(reason) = &outcome.reason {
            *endings.entry(reason.clone()).or_insert(0) += 1;
        }
    }
    format!(
        "[fuzz-duos] {ran} seed(s) from {from}: {failed} failed; endings {}",
        serde_json::to_string(&endings).unwrap_or_default()
    )
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
                let mut card = state.duels[0].players[PlayerId::P1].hand[0].clone();
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
