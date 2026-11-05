//! R2214000 (SPEC §9.12): a strength-zero bot's skill miss makes far fewer beginner's mistakes than the
//! random policy does, counted over random games played side by side on threads, which the pure crates'
//! tests (`crates/shared/tests/bots_decide.rs`, where the rest of the decision's tests are) may not start
//! (CLAUDE.md rule 4).

use jackioh_ai::{
    DeterminizeOptions, GREEDY_EVAL, NextSwing, candidate_set, determinize, evaluate, one_ply_scores,
    random_action, redact, skill_miss_action,
};
use jackioh_engine::{Action, ActionBody, GameState, Rng, seat_to_act};
use jackioh_server::actor::engine::{
    Pinned, begin_game, create_game, create_game_args, deal_random_deck, legal_actions, reduce,
};

/// States at each turn start of game with random legal moves.
fn random_turn_starts(seed: &str) -> Vec<GameState> {
    jackioh_cards::register_all();
    let decks = (
        deal_random_deck(&format!("{seed}-a"), None),
        deal_random_deck(&format!("{seed}-b"), None),
    );
    let mut state = begin_game(
        Pinned::default(),
        &create_game(
            Pinned::default(),
            &create_game_args(seed, &decks, None, None, None),
        ),
    )
    .state;
    let mut rng = Rng::new(seed, 0);
    let mut starts: Vec<GameState> = Vec::new();
    for step in 0..2_000 {
        let Some(seat) = seat_to_act(&state).filter(|_| state.result.is_none()) else {
            break;
        };
        if state.mulligan.is_none()
            && state.pending.is_none()
            && starts.last().is_none_or(|last| last.turn != state.turn)
        {
            starts.push(state.clone());
        }
        let legal: Vec<ActionBody> = legal_actions(Pinned::default(), &state, seat)
            .into_iter()
            .filter(|body| !matches!(body, ActionBody::Concede | ActionBody::OfferDraw))
            .collect();
        let at = (rng.next() * legal.len() as f64) as usize;
        let Some(body) = legal.get(at) else { break };
        let result = reduce(
            Pinned::default(),
            &state,
            &Action::new(body.clone(), seat, format!("random-{step}")),
        );
        if result.error.is_none() {
            state = result.state;
        }
    }
    starts
}

/// Tallies premature turn ends and suboptimal attacks across random vs skill miss pickers.
#[derive(Default)]
struct MissTally {
    idle: [usize; 2],
    idle_of: usize,
    bad_attacks: [usize; 2],
    bad_of: usize,
}

/// The `MissTally` of the random game `r2214000-rates-{game}`, two picks of each picker at each of its
/// turn starts.
fn miss_tally(game: usize) -> MissTally {
    const DRAWS: usize = 2;
    let mut tally = MissTally::default();
    for (at, state) in random_turn_starts(&format!("r2214000-rates-{game}"))
        .iter()
        .enumerate()
    {
        let seat = state.active;
        let position = format!("r2214000-rates-{game}-{at}");
        let has_play = legal_actions(Pinned::default(), state, seat)
            .iter()
            .any(|body| matches!(body, ActionBody::Play { .. }));
        let candidates = candidate_set(state, seat);
        let det = determinize(
            &redact(state, seat),
            seat,
            &mut Rng::new(&format!("{position}:det"), 0),
            DeterminizeOptions {
                match_shown_cost: Some(false),
            },
        );
        let standing = evaluate(&det, seat, NextSwing::Enemy, &GREEDY_EVAL);
        let bad: Vec<&ActionBody> = candidates
            .iter()
            .zip(one_ply_scores(&det, seat, &candidates))
            .filter(|(body, score)| matches!(body, ActionBody::Attack { .. }) && *score < standing)
            .map(|(body, _)| body)
            .collect();
        if has_play {
            tally.idle_of += DRAWS;
        }
        if !bad.is_empty() {
            tally.bad_of += DRAWS;
        }
        for draw in 0..DRAWS {
            let picks = [
                random_action(state, seat, &mut Rng::new(&format!("{position}:{draw}"), 0)),
                skill_miss_action(state, seat, &mut Rng::new(&format!("{position}:{draw}"), 0)),
            ];
            for (picker, pick) in picks.iter().enumerate() {
                if has_play && *pick == Some(ActionBody::EndTurn) {
                    tally.idle[picker] += 1;
                }
                if pick.as_ref().is_some_and(|pick| bad.contains(&pick)) {
                    tally.bad_attacks[picker] += 1;
                }
            }
        }
    }
    tally
}

#[test]
fn r2214000_a_strength_zero_miss_makes_far_fewer_beginners_mistakes_than_random() {
    const GAMES: usize = 20;
    // The games are played side by side, each on a thread of its own: one after another they took
    // most of a minute, the last test `rust (test server)` waited on (#2323). Each count is a sum over
    // the games, the same in any order.
    let tallies: Vec<MissTally> = std::thread::scope(|scope| {
        let games: Vec<_> = (0..GAMES)
            .map(|game| scope.spawn(move || miss_tally(game)))
            .collect();
        games
            .into_iter()
            .map(|game| {
                game.join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });
    let mut total = MissTally::default();
    for tally in tallies {
        for (sum, count) in total.idle.iter_mut().zip(tally.idle) {
            *sum += count;
        }
        for (sum, count) in total.bad_attacks.iter_mut().zip(tally.bad_attacks) {
            *sum += count;
        }
        total.idle_of += tally.idle_of;
        total.bad_of += tally.bad_of;
    }
    let MissTally {
        idle,
        idle_of,
        bad_attacks,
        bad_of,
    } = total;
    assert!(idle_of >= 500, "PREMISE: {idle_of} picks with a card to play");
    assert!(bad_of >= 150, "PREMISE: {bad_of} picks with a bad attack to make");
    let rate = |count: usize, of: usize| count as f64 / of as f64;
    let (idle_random, idle_miss) = (rate(idle[0], idle_of), rate(idle[1], idle_of));
    let (bad_random, bad_miss) = (rate(bad_attacks[0], bad_of), rate(bad_attacks[1], bad_of));
    let report = format!(
        "ended the turn with a card to play: random {idle_random:.3}, miss {idle_miss:.3} ({} and {} of \
         {idle_of}); attacked below standing still: random {bad_random:.3}, miss {bad_miss:.3} ({} and {} of \
         {bad_of})",
        idle[0], idle[1], bad_attacks[0], bad_attacks[1]
    );
    println!("{report}");
    assert!(idle_miss * 2.0 < idle_random, "{report}");
    assert!(bad_miss * 2.0 < bad_random, "{report}");
}
