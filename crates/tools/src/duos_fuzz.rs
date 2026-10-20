//! `cargo jackioh fuzz --duos [--seeds N] [--from S]` (#936 S5, B25): random-policy Duos games, each
//! held to the gate's assertions as a Duos game can meet them:
//!
//!   1. NO THROW: playing the game out raises nothing, and every action `duos_legal_actions` offered
//!      is one `duos_reduce` accepts.
//!   2. TERMINATION: the game ends — a pool emptied, a duel's own ending, or R2's turn cap — inside a
//!      bounded number of actions. The policy never concedes (R84), so `concede` and the server's
//!      `ceiling` cannot occur.
//!   3. REPLAY EQUALITY: `duos_fold` of `(options, log)` refuses nothing and reaches the live hash.
//!   4. PROBES (#1386): every `probe_stride()`-th state, two forged actions that are never logged, each
//!      owed a refusal with no events and the state exactly as it was. A reused-nonce probe sends the
//!      action under a nonce of the round's own (`DUOS_NONCE_PREFIX`) that a duel has applied (§13.3),
//!      so it can only come once a round has closed; a wrong-seat probe sends the body from a seat the
//!      game is not waiting on. A seat's own nonces are the server's to dedupe in Duos (§9.3, §13.9), so
//!      no probe reuses one. A failure is an error beginning `wrong-seat probe` or `reused-nonce probe`
//!      and names the step.
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
    AI_END_TURN_PROBABILITY, AI_PLAYOUT_STEP_CAP, ActionBody, DECK_SIZE, DUOS_DUELS, DUOS_NONCE_PREFIX,
    DUOS_REASON_DUEL_RESULT, DUOS_REASON_POOL_EMPTIED, DUOS_REASON_TURN_CAP, DUOS_SEATS, DuosAction,
    DuosActionBody, DuosCreateOptions, DuosDecks, DuosReduceResult, DuosReplayInput, DuosSeat, DuosState,
    Rng, TURN_CAP_PLAYER_TURNS, duos_begin, duos_create, duos_fold, duos_hash, duos_legal_actions,
    duos_reduce, duos_seats_to_act,
};
use rayon::prelude::*;

use crate::agent::panic_message;
use crate::fuzz::{
    DUOS_GROWTH_EVENTS_BOUND, DUOS_GROWTH_INSTANCES_BOUND, DUOS_GROWTH_NUMBER_BOUND, FUZZ_POOL, Heaviest,
    Heavy, MEASURES, REUSED_NONCE_PROBE, WRONG_SEAT_PROBE, duos_acting_card, duos_growth_breach,
    duos_heaviest_lines, probe_stride, weigh_duos,
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

/// What a probe's forged action goes through: `duos_reduce`, or in a test a broken one (#1386).
type DuosReducer = fn(&DuosState, &DuosAction) -> DuosReduceResult;

/// What a probe's `result` did wrong: it owed a refusal with no events and `state` exactly.
fn judge_duos_refusal(state: &DuosState, result: &DuosReduceResult) -> Option<String> {
    let mut wrong: Vec<String> = Vec::new();
    if result.error.is_none() {
        wrong.push("was accepted".to_string());
    }
    if !result.events.is_empty() {
        wrong.push(format!("emitted {} event(s)", result.events.len()));
    }
    if result.state != *state {
        wrong.push(format!(
            "moved the state (hash {} became {})",
            duos_hash(state),
            duos_hash(&result.state)
        ));
    }
    (!wrong.is_empty()).then(|| wrong.join(" and "))
}

/// The probes on one Duos state (#1386), never logged: `action` under a nonce of the round's own that
/// a duel has applied (§13.3), then its body sent by a seat not in `seats` (who the game waits on),
/// rotating by probe. Both are owed a refusal.
fn probe_duos_state(
    state: &DuosState,
    action: &DuosAction,
    seats: &[DuosSeat],
    step: usize,
    probe: DuosReducer,
) -> Option<String> {
    let used = state
        .duels
        .iter()
        .flat_map(|game| game.applied.iter())
        .find(|entry| entry.nonce.starts_with(DUOS_NONCE_PREFIX));
    if let Some(entry) = used {
        let forged = DuosAction {
            nonce: entry.nonce.clone(),
            ..action.clone()
        };
        if let Some(wrong) = judge_duos_refusal(state, &probe(state, &forged)) {
            return Some(format!(
                "{REUSED_NONCE_PROBE} probe after step {step}: {} sent {} under the round's nonce \"{}\", and it {wrong}",
                action.seat,
                duos_action_name(&action.body),
                entry.nonce
            ));
        }
    }
    let idle: Vec<DuosSeat> = DUOS_SEATS
        .into_iter()
        .filter(|seat| !seats.contains(seat))
        .collect();
    let other = *idle.get((step / probe_stride()) % idle.len().max(1))?;
    let forged = DuosAction {
        seat: other,
        nonce: format!("duos-fuzz-probe-{step}"),
        body: action.body.clone(),
    };
    judge_duos_refusal(state, &probe(state, &forged)).map(|wrong| {
        format!(
            "{WRONG_SEAT_PROBE} probe after step {step}: {other} sent {} while it may not act, and it {wrong}",
            duos_action_name(&action.body)
        )
    })
}

/// Assertion 1: one game played out, every offered action accepted.
fn play_duos_game(seed: u32) -> Result<DuosRun, String> {
    play_duos_game_inner(seed, duos_reduce, |_, _, _, _| {})
}

fn play_duos_game_inner<F>(seed: u32, probe: DuosReducer, mut tamper: F) -> Result<DuosRun, String>
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
    let probe_stride = probe_stride();
    let mut setup = monitor.after(&begun.events, &state);
    setup.extend(monitor.hidden(&state));
    if let Some(first) = setup.first() {
        return Err(format!("at step 0: {first}"));
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

        // #1386: forged actions against this state, never logged.
        if log.len().is_multiple_of(probe_stride)
            && let Some(message) = probe_duos_state(&state, &action, &seats, log.len(), probe)
        {
            return Err(message);
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

/// One seed's verdict.
#[derive(Clone, Debug)]
struct DuosSeedOutcome {
    seed: u32,
    reason: Option<String>,
    error: Option<String>,
    /// The passing game's heaviest action by each measure (#1364); a failed seed keeps none.
    heaviest: Heaviest,
}

fn fuzz_duos_seed(seed: u32) -> DuosSeedOutcome {
    // R1420: the pool is the whole catalog, so this worker thread previews every set for the game.
    let _preview = preview_every_set();
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<(String, Heaviest), String> {
        let run = play_duos_game(seed)?;
        let reason = check_duos_run(&run)?;
        Ok((reason, run.heaviest))
    }));
    match caught {
        Ok(Ok((reason, heaviest))) => DuosSeedOutcome {
            seed,
            reason: Some(reason),
            error: None,
            heaviest,
        },
        Ok(Err(error)) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(error),
            heaviest: Heaviest::default(),
        },
        Err(payload) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(format!("panic: {}", panic_message(&*payload))),
            heaviest: Heaviest::default(),
        },
    }
}

#[cfg(test)]
fn fuzz_duos_seed_with<F>(seed: u32, probe: DuosReducer, tamper: F) -> DuosSeedOutcome
where
    F: FnMut(usize, &mut DuosState, &mut DuosAction, &mut DuosInvariantMonitor),
{
    let _preview = preview_every_set();
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<(String, Heaviest), String> {
        let run = play_duos_game_inner(seed, probe, tamper)?;
        let reason = check_duos_run(&run)?;
        Ok((reason, run.heaviest))
    }));
    match caught {
        Ok(Ok((reason, heaviest))) => DuosSeedOutcome {
            seed,
            reason: Some(reason),
            error: None,
            heaviest,
        },
        Ok(Err(error)) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(error),
            heaviest: Heaviest::default(),
        },
        Err(payload) => DuosSeedOutcome {
            seed,
            reason: None,
            error: Some(format!("panic: {}", panic_message(&*payload))),
            heaviest: Heaviest::default(),
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
    format!(
        "[fuzz-duos] {ran} seed(s) from {from}: {failed} failed; endings {}\n{}",
        serde_json::to_string(&endings).unwrap_or_default(),
        duos_heaviest_lines(&passed_heaviest(outcomes)),
    )
}

/// `--duos --json`: the wave's failures, each measure's heaviest actions and the growth bounds
/// (#1364). Failures carry only seed and message: Duos has no stage taxonomy, so the `growth:`
/// prefix in the message is the stage name.
fn wave_json(from: u32, ran: u32, outcomes: &[DuosSeedOutcome]) -> serde_json::Value {
    let passed = outcomes.iter().filter(|outcome| outcome.error.is_none()).count();
    serde_json::json!({
        "from": from,
        "ran": ran,
        "passed": passed,
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
    })
}

#[cfg(test)]
mod tests {
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
        let outcome = fuzz_duos_seed_with(7, duos_reduce, |step, state, action, monitor| {
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
            let outcome = fuzz_duos_seed_with(1, duos_reduce, move |step, state, action, _monitor| {
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
            let outcome = fuzz_duos_seed_with(1, duos_reduce, |_, _, _, _| {});
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
            }];
            let summary = summary(1, 1, &outcomes);
            assert!(summary.contains("1/3 pass -: 10"), "{summary}");
            assert!(summary.contains("1/3 pass -: 20"), "{summary}");
            assert!(summary.contains("1/3 pass -: 30"), "{summary}");
            let json = wave_json(1, 1, &outcomes);
            assert_eq!(json["heaviest"]["events"][0]["value"], serde_json::json!(10));
            assert_eq!(
                json["bounds"]["largest"],
                serde_json::json!(DUOS_GROWTH_NUMBER_BOUND)
            );
            assert_eq!(json["passed"], serde_json::json!(1));
        }
    }

    #[test]
    fn a_planted_view_leak_fails_duos_fuzz_naming_seed_duel_and_step() {
        let outcome = fuzz_duos_seed_with(7, duos_reduce, |step, _state, _action, monitor| {
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

    mod duos_probe_stage {
        use super::*;

        /// A reducer that takes the action from whichever seat the game waits on, whoever sent it.
        fn any_seat(state: &DuosState, action: &DuosAction) -> DuosReduceResult {
            for seat in duos_seats_to_act(state) {
                let result = duos_reduce(
                    state,
                    &DuosAction {
                        seat,
                        ..action.clone()
                    },
                );
                if result.error.is_none() {
                    return result;
                }
            }
            duos_reduce(state, action)
        }

        /// A reducer that lets the round's own nonces through as a seat's.
        fn unguarded(state: &DuosState, action: &DuosAction) -> DuosReduceResult {
            let nonce = action.nonce.replacen(DUOS_NONCE_PREFIX, "seat:", 1);
            duos_reduce(
                state,
                &DuosAction {
                    nonce,
                    ..action.clone()
                },
            )
        }

        /// `probe` fails seed 7 naming the probe called `name`, the seed and a step.
        fn assert_caught(probe: DuosReducer, name: &str) {
            let outcome = fuzz_duos_seed_with(7, probe, |_, _, _, _| {});
            let lines = failure_lines(&[outcome]);
            assert_eq!(lines.len(), 1, "{lines:?}");
            assert!(
                lines[0].starts_with(&format!("seed 7: {name} probe after step ")),
                "{}",
                lines[0]
            );
        }

        #[test]
        fn a_reducer_that_takes_any_seat_s_action_fails_the_wrong_seat_probe_naming_seed_and_step() {
            assert_caught(any_seat, WRONG_SEAT_PROBE);
        }

        #[test]
        fn a_reducer_that_lets_the_round_s_nonces_through_fails_the_reused_nonce_probe_naming_seed_and_step()
        {
            assert_caught(unguarded, REUSED_NONCE_PROBE);
        }
    }
}
