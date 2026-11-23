//! The daily game's clock under random play (#4431; SPEC §9.13, R3727000, R3727001, R3727002,
//! R3727003, R3727006): seeded random daily games through mulligans, prompts, draw offers, seat
//! swaps and completions, checking at every step that a hold keeps its deadline under the same move
//! key, that expiry fires at the exact deadline millisecond, that `daily_legal` and `daily_allows`
//! agree, and that a finished game holds nothing. Pure like the clock: every draw is the engine's
//! `Rng` (CLAUDE.md rule 4 binds a pure crate's tests too), time a number the runner moves itself.
//!
//! No module or test here is named `fuzz`, `selfplay` or `positions`: the `rust (test tools, wasm)`
//! job skips tests with those names (`.github/ci/rust-test.toml`).

use std::fmt;

use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::{
    Action, ActionBody, CreateGameOptions, DECK_SIZE, GameState, Rng, SandboxOp, SpawnZone, hash_state,
    legal_actions, opponent_of,
};
use jackioh_shared::config::{DAILY_TIME_PER_MOVE_DAYS, MS_PER_DAY, MS_PER_HOUR};
use jackioh_shared::host::daily::{
    DailyClock, DailyHold, daily_allows, daily_expired, daily_holders, daily_legal, daily_sync, move_key,
};

/// The epoch millisecond the games start at.
const NOW: i64 = 1_700_000_000_000;

/// Meditative+ #20 KY's Competition, whose question sequence stands in for multi-step prompts.
const COMPETITION: &str = "meditativeplus-020";

/// How many random games the per-PR test plays.
const DAILY_INVARIANTS_SEEDS: u64 = 24;
/// How many random games the ignored daily sweep plays.
const DAILY_INVARIANTS_DAILY_SEEDS: u64 = 400;
/// The most steps one game is played for.
const DAILY_INVARIANTS_MAX_STEPS: usize = 150;
/// A roll below this plays a random valid action; below `TIME_ROLL` advances the clock, below
/// `CAST_ROLL` casts Competition, below `SWAP_ROLL` swaps the seats, else jumps to the earliest
/// deadline.
const PLAY_ROLL: i32 = 56;
const TIME_ROLL: i32 = 76;
const CAST_ROLL: i32 = 86;
const SWAP_ROLL: i32 = 95;
/// Set to rerun one failing seed: `repro_one_seed` plays it alone.
const REPRO: Option<u64> = None;

fn deck(salt: &str) -> Vec<String> {
    jackioh_ai::build_ai_deck(
        &mut Rng::new(&format!("daily:{salt}"), 0),
        DECK_SIZE,
        &jackioh_ai::AiDeckOptions {
            banned: Some(vec![]),
            gated_sets: Some(vec![]),
            ..jackioh_ai::AiDeckOptions::default()
        },
    )
}

/// A begun game, both mulligans open (R265). `sandbox` lets a test hand a player the cards and the
/// mana a rule needs (R923000).
fn begun(seed: &str, sandbox: bool) -> GameState {
    jackioh_cards::register_all();
    let options: CreateGameOptions = serde_json::from_value(serde_json::json!({
        "seed": seed,
        "decks": [deck(&format!("{seed}-a")), deck(&format!("{seed}-b"))],
        "sandbox": sandbox,
    }))
    .expect("create options");
    jackioh_engine::begin_game(&jackioh_engine::create_game(&options)).state
}

/// Applies `body` for `player` under a nonce no earlier action of the game used (a replayed nonce is
/// the first answer again, §9.3): one past the last applied.
fn act(state: &GameState, player: PlayerId, body: ActionBody) -> GameState {
    let last = state
        .applied
        .last()
        .and_then(|entry| entry.nonce.strip_prefix("test-"))
        .and_then(|count| count.parse::<u32>().ok())
        .unwrap_or(0);
    let nonce = format!("test-{}", last + 1);
    let result = jackioh_engine::reduce(state, &Action::new(body.clone(), player, nonce));
    assert!(result.error.is_none(), "{body:?}: {:?}", result.error);
    result.state
}

/// Like [`act`], but a refused action is `None` so a random runner can try a scripted line and skip
/// it when the position refuses it instead of failing the seed.
fn try_act(state: &GameState, player: PlayerId, body: ActionBody) -> Option<GameState> {
    let last = state
        .applied
        .last()
        .and_then(|entry| entry.nonce.strip_prefix("test-"))
        .and_then(|count| count.parse::<u32>().ok())
        .unwrap_or(0);
    let nonce = format!("test-{}", last + 1);
    let result = jackioh_engine::reduce(state, &Action::new(body, player, nonce));
    if result.error.is_some() {
        None
    } else {
        Some(result.state)
    }
}

/// `player` keeps their opening hand.
fn mulligan(state: &GameState, player: PlayerId) -> GameState {
    let keep = legal_actions(state, player)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Mulligan { .. }))
        .expect("a mulligan is owed");
    act(state, player, keep)
}

/// A clock at the shortest time per move, synced over `state` at `now`.
fn clock_over(state: &GameState, now: i64) -> DailyClock {
    daily_sync(&DailyClock::new(DAILY_TIME_PER_MOVE_DAYS[0]), state, now)
}

fn window() -> i64 {
    i64::from(DAILY_TIME_PER_MOVE_DAYS[0]) * MS_PER_DAY
}

/// The game after both mulligans, on the first turn.
fn first_turn(seed: &str, sandbox: bool) -> GameState {
    let state = begun(seed, sandbox);
    let state = mulligan(&state, P1);
    mulligan(&state, P2)
}

/// The active player (in a sandbox game) casts KY's Competition, if the position lets them: the
/// mana, the spawn and the cast each have to go through, else `None` and the caller plays on.
fn try_competition_cast(state: &GameState) -> Option<GameState> {
    let player = state.active;
    let tools = [
        SandboxOp::SetMana {
            player,
            current: Some(10),
            max: Some(10),
        },
        SandboxOp::Spawn {
            player,
            def_id: COMPETITION.to_string(),
            radiant: None,
            to: SpawnZone::Hand,
        },
    ];
    let mut state = state.clone();
    for op in tools {
        state = try_act(&state, player, ActionBody::Sandbox { op })?;
    }
    let card = state.players[player]
        .hand
        .iter()
        .find(|card| card.def_id == COMPETITION)
        .map(|card| card.id.clone())?;
    let play = legal_actions(&state, player)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Play { instance_id, .. } if *instance_id == card))?;
    try_act(&state, player, play)
}

/// What a failed game reports: where, and which check.
#[derive(Debug)]
struct Failure {
    seed: u64,
    step: usize,
    detail: String,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "daily invariants: seed {} step {}: {}; rerun: set REPRO = Some({}) in crates/shared/tests/daily_invariants.rs",
            self.seed, self.step, self.detail, self.seed
        )
    }
}

/// The holds `state` should carry after reaching it at `now` with `prev` the clock before: each
/// holder still holding the same move keeps its hold, one the move passed to gets the whole time
/// per move from `now` (R3727001, R3727003).
fn expected_holds(state: &GameState, prev: Option<&DailyClock>, window_ms: i64, now: i64) -> Vec<DailyHold> {
    let key = move_key(state);
    daily_holders(state)
        .into_iter()
        .map(|player| {
            prev.and_then(|clock| clock.hold_of(player))
                .filter(|hold| hold.key == key)
                .cloned()
                .unwrap_or(DailyHold {
                    player,
                    key: key.clone(),
                    since: now,
                    deadline: now + window_ms,
                })
        })
        .collect()
}

/// R3727001, R3727003: the clock holds exactly the move's holders in order, and no hold's window
/// moves while its seat holds the same move.
fn check_monotonicity(
    state: &GameState,
    prev: Option<&DailyClock>,
    clock: &DailyClock,
    now: i64,
) -> Result<(), String> {
    let expected = expected_holds(state, prev, clock.window_ms(), now);
    if clock.holds == expected {
        Ok(())
    } else {
        Err(format!(
            "R3727003 hold monotonicity broken: holds {:?} are not {:?}",
            clock.holds, expected
        ))
    }
}

/// R3727000, R3727002: expiry fires at the exact deadline millisecond — nothing before it, the
/// first seat holding it at and after it — and what it would name holds the move now. The expected
/// deadline comes from `prev`, so a clock whose deadline moved early fails here even though it
/// reads consistently on its own.
fn check_timeliness(
    state: &GameState,
    prev: Option<&DailyClock>,
    clock: &DailyClock,
    now: i64,
) -> Result<(), String> {
    if state.result.is_some() {
        return Ok(());
    }
    let expected = expected_holds(state, prev, clock.window_ms(), now);
    let Some(deadline) = expected.iter().map(|hold| hold.deadline).min() else {
        return Err("R3727000 a live game holds no deadline".to_string());
    };
    let want = expected
        .iter()
        .find(|hold| hold.deadline == deadline)
        .map(|hold| hold.player)
        .expect("the earliest deadline is held");
    if daily_expired(clock, deadline - 1).is_some() {
        return Err(format!(
            "R3727000 expiry before the deadline: {want:?} runs out at {deadline}"
        ));
    }
    for at in [deadline, deadline + 1] {
        if daily_expired(clock, at) != Some(want) {
            return Err(format!(
                "R3727000 no expiry at the deadline: {at} names {:?}, not {want:?}",
                daily_expired(clock, at)
            ));
        }
    }
    let at_now = if now >= deadline { Some(want) } else { None };
    if daily_expired(clock, now) != at_now {
        return Err(format!(
            "R3727000 expiry disagrees with the clock at {now}: {:?}, deadline {deadline}",
            daily_expired(clock, now)
        ));
    }
    if !daily_holders(state).contains(&want) {
        return Err(format!("R3727002 expiry names {want:?}, who holds no move"));
    }
    Ok(())
}

/// R3727000: a finished game holds nothing, so nothing of it can run out.
fn check_completion(state: &GameState, clock: &DailyClock, now: i64) -> Result<(), String> {
    if state.result.is_none() {
        return Ok(());
    }
    if !daily_holders(state).is_empty() {
        return Err(format!(
            "R3727000 a finished game still owes the move: {:?}",
            daily_holders(state)
        ));
    }
    if !clock.holds.is_empty() {
        return Err(format!(
            "R3727000 a finished game still holds the move: {:?}",
            clock.holds
        ));
    }
    for at in [now, now + clock.window_ms()] {
        if daily_expired(clock, at).is_some() {
            return Err(format!("R3727000 a finished game expires at {at}"));
        }
    }
    Ok(())
}

/// R3727006: `offered` is what `daily_legal` gives `player`: the engine's legal actions filtered by
/// `daily_allows`, so a seat holding no move may only concede, and a concession is never filtered
/// out.
fn check_filtering(state: &GameState, player: PlayerId, offered: &[ActionBody]) -> Result<(), String> {
    let legal = legal_actions(state, player);
    let manual: Vec<ActionBody> = legal
        .iter()
        .filter(|body| daily_allows(state, player, body))
        .cloned()
        .collect();
    if offered != manual {
        return Err(format!(
            "R3727006 filter parity broken for {player:?}: {offered:?} is not the allowed {manual:?}"
        ));
    }
    if !daily_holders(state).contains(&player)
        && offered.iter().any(|body| !matches!(body, ActionBody::Concede))
    {
        return Err(format!(
            "R3727006 {player:?} holds no move but may send {offered:?}"
        ));
    }
    if legal.contains(&ActionBody::Concede) && !offered.contains(&ActionBody::Concede) {
        return Err(format!("R3727006 {player:?} may concede but the filter drops it"));
    }
    Ok(())
}

fn check_all(
    state: &GameState,
    prev: Option<&DailyClock>,
    clock: &DailyClock,
    now: i64,
) -> Result<(), String> {
    check_completion(state, clock, now)?;
    if state.result.is_none() {
        check_monotonicity(state, prev, clock, now)?;
        check_timeliness(state, prev, clock, now)?;
    }
    for seat in [P1, P2] {
        check_filtering(state, seat, &daily_legal(state, seat, legal_actions(state, seat)))?;
    }
    Ok(())
}

/// Plays one seeded random daily game, checking the invariants after every step: random valid
/// actions, clock advances, scripted Competition casts for multi-step prompts, seat swaps, and
/// jumps to the exact deadline. Ends on a finished or an expired game, or after
/// `DAILY_INVARIANTS_MAX_STEPS` steps; the fingerprint is the final state's hash and step count.
fn play_daily_game(seed: u64) -> Result<String, Failure> {
    let fail = |step: usize, detail: String| Failure { seed, step, detail };
    let mut rng = Rng::new(&format!("daily-invariants:{seed}"), 0);
    let per_move_days = *rng.pick(&DAILY_TIME_PER_MOVE_DAYS).expect("the times per move");
    let mut state = first_turn(&format!("daily-invariants-{seed}"), true);
    let mut now = NOW;
    let mut clock = daily_sync(&DailyClock::new(per_move_days), &state, now);
    check_all(&state, None, &clock, now).map_err(|detail| fail(0, detail))?;
    let mut steps = 0;
    for step in 1..=DAILY_INVARIANTS_MAX_STEPS {
        steps = step;
        let prev = clock.clone();
        let roll = rng.int(100);
        if roll < PLAY_ROLL {
            let holders = daily_holders(&state);
            let holder = *rng.pick(&holders).expect("a live game holds a move");
            let playable: Vec<ActionBody> = daily_legal(&state, holder, legal_actions(&state, holder))
                .into_iter()
                .filter(|body| !matches!(body, ActionBody::Sandbox { .. }))
                .collect();
            if let Some(body) = rng.pick(&playable) {
                state = act(&state, holder, body.clone());
            }
            clock = daily_sync(&prev, &state, now);
        } else if roll < TIME_ROLL {
            now += i64::from(rng.int(MS_PER_DAY as i32));
            clock = daily_sync(&prev, &state, now);
        } else if roll < CAST_ROLL {
            if daily_holders(&state) == vec![state.active]
                && state.pending.is_none()
                && let Some(cast) = try_competition_cast(&state)
            {
                state = cast;
            }
            clock = daily_sync(&prev, &state, now);
        } else if roll < SWAP_ROLL {
            let mut swapped = state.clone();
            swapped.seat_swaps = Some(swapped.seat_swaps.unwrap_or(0) + 1);
            state = swapped;
            clock = daily_sync(&prev, &state, now);
        } else if let Some(deadline) = clock.next_deadline() {
            now = deadline;
            clock = daily_sync(&prev, &state, now);
        }
        check_all(&state, Some(&prev), &clock, now).map_err(|detail| fail(step, detail))?;
        if state.result.is_some() {
            break;
        }
        if daily_expired(&clock, now).is_some() {
            break;
        }
    }
    Ok(format!("{}:{steps}", hash_state(&state)))
}

#[test]
fn daily_games_keep_clock_invariants() {
    for seed in 0..DAILY_INVARIANTS_SEEDS {
        if let Err(failure) = play_daily_game(seed) {
            panic!("{failure}");
        }
    }
}

#[test]
fn the_same_seed_plays_the_same_daily_game() {
    let first = play_daily_game(0).expect("seed 0 plays");
    let second = play_daily_game(0).expect("seed 0 plays");
    assert_eq!(first, second);
}

#[test]
fn repro_one_seed() {
    if let Some(seed) = REPRO {
        play_daily_game(seed).expect("the repro seed plays");
    }
}

#[test]
#[ignore = "daily: DAILY_INVARIANTS_DAILY_SEEDS games; the per-PR test plays DAILY_INVARIANTS_SEEDS of them"]
fn daily_clock_sweep() {
    for seed in 0..DAILY_INVARIANTS_DAILY_SEEDS {
        if let Err(failure) = play_daily_game(seed) {
            panic!("{failure}");
        }
    }
}

#[test]
fn the_hold_checker_fails_a_shortened_deadline() {
    let state = first_turn("daily-invariants-hold", true);
    let prev = clock_over(&state, NOW);
    let kept = daily_sync(&prev, &state, NOW + MS_PER_HOUR);
    check_monotonicity(&state, Some(&prev), &kept, NOW + MS_PER_HOUR).expect("a resync keeps the window");
    let mut broken = kept.clone();
    broken.holds[0].deadline -= 1;
    let err = check_monotonicity(&state, Some(&prev), &broken, NOW + MS_PER_HOUR)
        .expect_err("a shortened deadline");
    assert!(err.contains("R3727003"), "{err}");
}

#[test]
fn the_expiry_checker_fails_before_the_deadline() {
    let state = first_turn("daily-invariants-expiry", true);
    let prev = clock_over(&state, NOW);
    let deadline = NOW + window();
    let synced = daily_sync(&prev, &state, deadline - 1);
    check_timeliness(&state, Some(&prev), &synced, deadline - 1)
        .expect("a sound clock holds to its deadline");
    assert_eq!(daily_expired(&synced, deadline - 1), None);
    let mut broken = synced.clone();
    broken.holds[0].deadline = NOW + 1;
    assert_eq!(daily_expired(&broken, deadline - 1), Some(state.active));
    let err = check_timeliness(&state, Some(&prev), &broken, deadline - 1).expect_err("an early expiry");
    assert!(err.contains("before the deadline"), "{err}");
}

#[test]
fn the_completion_checker_fails_a_hold_after_game_over() {
    let state = first_turn("daily-invariants-completion", true);
    let live = clock_over(&state, NOW);
    let over = act(&state, state.active, ActionBody::Concede);
    assert!(over.result.is_some(), "a concession ends the game");
    let err = check_completion(&over, &live, NOW + 1).expect_err("a hold past the end of the game");
    assert!(err.contains("R3727000"), "{err}");
    let synced = daily_sync(&live, &over, NOW + 1);
    check_completion(&over, &synced, NOW + 1).expect("an emptied clock ends clean");
}

#[test]
fn the_filter_checker_fails_a_non_holder_move() {
    let state = first_turn("daily-invariants-filter", true);
    let other = opponent_of(state.active);
    check_filtering(
        &state,
        other,
        &daily_legal(&state, other, legal_actions(&state, other)),
    )
    .expect("the recomputed filter holds");
    let err = check_filtering(&state, other, &[ActionBody::EndTurn]).expect_err("a non-holder's EndTurn");
    assert!(err.contains("R3727006"), "{err}");
}
