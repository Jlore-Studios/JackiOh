//! B32 (#936, #1760): the Duos gate at smoke size. The Easy AI team (both seats on `duos_decide`)
//! against a random-policy team has to win at least DUOS_GATE_WIN_FLOOR of DUOS_GATE_GAMES_SMOKE games,
//! and against a greedy team (each seat on the 1v1 greedy baseline) at least DUOS_GATE_GREEDY_WIN_FLOOR.
//! No action of either team may be refused, no AI action may come off a fallback path (#1385) and no AI
//! seat's search may catch a panic (#1429); a failure names each such game's seed, which
//! `duos_gate_game(n, "gate", opponent)` replays. A seat that passes its first card first, before
//! anything else, still clears the random floor and fails the greedy one: the reason the greedy matchup
//! exists. It plays whole Duos games, so it runs in `ai_slow.rs` rather than the `ai` binary, and daily
//! (`#[ignore]`).

use jackioh_ai::{
    AI_GATE_BUDGET, AiOptions, DUOS_GATE_GAMES_SMOKE, DUOS_GATE_GREEDY_WIN_FLOOR, DuosDecision, DuosGateGame,
    DuosGateReport, DuosOpponent, SearchBudget, duos_decision, duos_gate_game, duos_gate_game_at,
    duos_gate_game_with, duos_gate_report, duos_gate_subject,
};
use jackioh_engine::duos::{DuosState, duos_legal_actions};
use jackioh_engine::wire::{DuosActionBody, DuosSeat};

use super::support::register_cards;

#[test]
#[ignore = "daily: twenty whole Duos games; duos.rs's tests hold the AI's Duos seats per pull request"]
fn b32_the_easy_ai_team_beats_a_random_policy_team_in_at_least_the_floor_of_the_smoke_games() {
    assert_the_gate_passes(DuosOpponent::Random);
}

#[test]
#[ignore = "daily: twenty whole Duos games against the greedy team; duos.rs's greedy-seat tests and this file's report tests hold it per pull request"]
fn b32_the_easy_ai_team_beats_a_greedy_team_in_at_least_its_floor_of_the_smoke_games() {
    assert_the_gate_passes(DuosOpponent::Greedy);
}

/// The gate's clean run against `opponent`: its floor of wins, nothing refused, no fallback, no panic.
fn assert_the_gate_passes(opponent: DuosOpponent) {
    register_cards();
    let played: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE)
        .map(|n| duos_gate_game(n, "gate", opponent))
        .collect();
    let report = duos_gate_report(opponent, &played);
    assert_eq!(report.games, DUOS_GATE_GAMES_SMOKE);
    assert!(
        report.wins + report.draws <= report.games,
        "{} wins, {} draws",
        report.wins,
        report.draws
    );
    let floor = opponent.win_floor() * DUOS_GATE_GAMES_SMOKE as f64;
    assert!(
        report.wins as f64 >= floor,
        "{} wins of {} games against the {opponent} team, under the floor of {floor}",
        report.wins,
        report.games
    );
    let refused: Vec<String> = played
        .iter()
        .filter(|game| game.rejected > 0)
        .map(|game| {
            format!(
                "{} (AI team {}, {} refused, {} by the AI team)",
                game.seed, game.subject, game.rejected, game.ai_rejected
            )
        })
        .collect();
    assert_eq!(report.rejected, 0, "refused: {}", refused.join(", "));
    let fell_back: Vec<String> = played
        .iter()
        .filter(|game| game.fallbacks > 0)
        .map(|game| format!("{} ({} fallbacks)", game.seed, game.fallbacks))
        .collect();
    assert_eq!(report.fallbacks, 0, "fell back: {}", fell_back.join(", "));
    let panicked: Vec<String> = played
        .iter()
        .filter(|game| game.sim_panics > 0)
        .map(|game| {
            format!(
                "{} ({} search panics, the first {:?})",
                game.seed, game.sim_panics, game.first_sim_panic
            )
        })
        .collect();
    assert_eq!(report.sim_panics, 0, "panicked: {}", panicked.join(", "));
    assert!(report.passes());
}

/// #1760's weakened pass heuristic: once per round in its duel, before anything else, the seat passes
/// the first card the wrapper lets it pass, whatever it is worth; otherwise it decides as the AI does.
fn passes_its_first_card_first(
    state: &DuosState,
    seat: DuosSeat,
    options: &mut AiOptions<'_>,
) -> Option<DuosDecision> {
    let first = if state.passes.get(seat.duel()) == Some(&0) {
        duos_legal_actions(state, seat)
            .into_iter()
            .find(|body| matches!(body, DuosActionBody::Pass { .. }))
    } else {
        None
    };
    match first {
        Some(body) => Some(DuosDecision {
            body,
            fallback: false,
            sim_panics: 0,
            first_sim_panic: None,
        }),
        None => duos_decision(state, seat, options),
    }
}

/// The smoke games' gate run against `opponent` with the AI team on `passes_its_first_card_first`.
fn weakened_run(opponent: DuosOpponent) -> DuosGateReport {
    register_cards();
    let played: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE)
        .map(|n| duos_gate_game_with(n, "gate", opponent, AI_GATE_BUDGET, passes_its_first_card_first))
        .collect();
    duos_gate_report(opponent, &played)
}

#[test]
#[ignore = "daily: twenty whole Duos games with a weakened pass; this file's report tests hold the floor per pull request"]
fn b32_a_seat_that_passes_its_first_card_first_still_clears_the_random_floor() {
    let report = weakened_run(DuosOpponent::Random);
    println!("{report:?}");
    assert!(report.passes(), "{report:?}");
}

#[test]
#[ignore = "daily: twenty whole Duos games with a weakened pass; this file's report tests hold the floor per pull request"]
fn b32_a_seat_that_passes_its_first_card_first_fails_the_greedy_floor() {
    let report = weakened_run(DuosOpponent::Greedy);
    println!("{report:?}");
    assert_eq!(
        (report.rejected, report.fallbacks, report.sim_panics),
        (0, 0, 0),
        "it fails on wins alone: {report:?}"
    );
    assert!(!report.passes(), "{report:?}");
}

/// A run of DUOS_GATE_GAMES_SMOKE games against `opponent`, every one won, with nothing refused and no
/// fallback.
fn every_game_won(opponent: DuosOpponent) -> Vec<DuosGateGame> {
    (1..=DUOS_GATE_GAMES_SMOKE)
        .map(|n| DuosGateGame {
            seed: opponent.seed("gate", n),
            subject: duos_gate_subject(n),
            opponent,
            won: true,
            drawn: false,
            reason: Some("poolEmptied".to_string()),
            actions: 1,
            rejected: 0,
            ai_rejected: 0,
            fallbacks: 0,
            sim_panics: 0,
            first_sim_panic: None,
        })
        .collect()
}

#[test]
fn b32_a_refused_action_fails_the_duos_gate_report_even_with_every_game_won() {
    let clean = every_game_won(DuosOpponent::Random);
    assert_eq!(duos_gate_report(DuosOpponent::Random, &clean).rejected, 0);
    assert!(duos_gate_report(DuosOpponent::Random, &clean).passes());

    let mut refused = clean.clone();
    refused[0].rejected = 1;
    refused[0].ai_rejected = 1;
    refused[1].rejected = 2;
    let report = duos_gate_report(DuosOpponent::Random, &refused);
    assert_eq!(report.wins, DUOS_GATE_GAMES_SMOKE);
    assert_eq!(report.rejected, 3);
    assert!(!report.passes());
}

#[test]
fn b32_a_fallback_action_fails_the_duos_gate_report_even_with_every_game_won() {
    let clean = every_game_won(DuosOpponent::Random);
    assert_eq!(duos_gate_report(DuosOpponent::Random, &clean).fallbacks, 0);
    assert!(duos_gate_report(DuosOpponent::Random, &clean).passes());

    let mut fell_back = clean.clone();
    fell_back[0].fallbacks = 1;
    fell_back[1].fallbacks = 2;
    let report = duos_gate_report(DuosOpponent::Random, &fell_back);
    assert_eq!(report.wins, DUOS_GATE_GAMES_SMOKE);
    assert_eq!(report.rejected, 0);
    assert_eq!(report.fallbacks, 3);
    assert!(!report.passes());
}

#[test]
fn b32_a_search_panic_fails_the_duos_gate_report_even_with_every_game_won() {
    let clean = every_game_won(DuosOpponent::Random);
    assert_eq!(duos_gate_report(DuosOpponent::Random, &clean).sim_panics, 0);
    assert!(duos_gate_report(DuosOpponent::Random, &clean).passes());

    let mut panicked = clean.clone();
    panicked[0].sim_panics = 1;
    panicked[1].sim_panics = 2;
    let report = duos_gate_report(DuosOpponent::Random, &panicked);
    assert_eq!(report.wins, DUOS_GATE_GAMES_SMOKE);
    assert_eq!(report.rejected, 0);
    assert_eq!(report.fallbacks, 0);
    assert_eq!(report.sim_panics, 3);
    assert!(!report.passes());
}

#[test]
fn b32_a_starved_ai_team_counts_its_fallbacks_and_fails_the_duos_gate() {
    register_cards();
    // No node to search with: `decide`'s search scores nothing, so it falls back (decide.rs step 7).
    let starved = SearchBudget {
        nodes: 0,
        lethal_nodes: 0,
        ..AI_GATE_BUDGET
    };
    let game = duos_gate_game_at(1, "gate", DuosOpponent::Random, starved);
    assert!(game.fallbacks > 0, "{game:?}");
    let report = duos_gate_report(DuosOpponent::Random, std::slice::from_ref(&game));
    assert_eq!(report.fallbacks, game.fallbacks);
    assert!(!report.passes(), "{report:?}");
}

#[test]
fn b32_a_greedy_run_is_held_to_its_own_floor() {
    let needed = (DUOS_GATE_GREEDY_WIN_FLOOR * DUOS_GATE_GAMES_SMOKE as f64).ceil() as usize;
    let run = |wins: usize| -> DuosGateReport {
        let mut played = every_game_won(DuosOpponent::Greedy);
        for (n, game) in (1..).zip(played.iter_mut()) {
            game.won = n <= wins;
        }
        duos_gate_report(DuosOpponent::Greedy, &played)
    };

    let at_floor = run(needed);
    assert_eq!((at_floor.opponent, at_floor.wins), (DuosOpponent::Greedy, needed));
    assert!(at_floor.passes(), "{at_floor:?}");

    let under = run(needed - 1);
    assert_eq!(under.wins, needed - 1);
    assert!(!under.passes(), "{under:?}");
}

#[test]
fn b32_a_report_counts_only_its_own_opponents_games() {
    let mut played = every_game_won(DuosOpponent::Random);
    played.extend(
        every_game_won(DuosOpponent::Greedy)
            .into_iter()
            .map(|game| DuosGateGame { won: false, ..game }),
    );

    let random = duos_gate_report(DuosOpponent::Random, &played);
    assert_eq!(
        (random.opponent, random.games, random.wins),
        (DuosOpponent::Random, DUOS_GATE_GAMES_SMOKE, DUOS_GATE_GAMES_SMOKE)
    );
    assert!(random.passes(), "{random:?}");

    let greedy = duos_gate_report(DuosOpponent::Greedy, &played);
    assert_eq!(
        (greedy.opponent, greedy.games, greedy.wins),
        (DuosOpponent::Greedy, DUOS_GATE_GAMES_SMOKE, 0)
    );
    assert!(!greedy.passes(), "{greedy:?}");
}

#[test]
fn b32_each_opponent_deals_its_own_seeds() {
    assert_eq!(DuosOpponent::Random.seed("gate", 3), "gate:3");
    assert_eq!(DuosOpponent::Greedy.seed("gate", 3), "gate:greedy:3");
    assert_eq!(DuosOpponent::Random.to_string(), "random");
    assert_eq!(DuosOpponent::Greedy.to_string(), "greedy");
    assert_eq!(DuosOpponent::ALL, [DuosOpponent::Random, DuosOpponent::Greedy]);
}
