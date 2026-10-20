//! `cargo jackioh gate --duos [--games N]`: the Duos gate (#936 B32, S5). `jackioh_ai::duos` plays and
//! measures the games; this file runs them, prints the report and fails under the floor.
//!
//! Game n of a run is `duos_gate_game(n, DUOS_GATE_SERIES)`: the Easy AI team (`duos_decide` at
//! AI_GATE_BUDGET) against a team on §10.7's random policy over `duos_legal_actions`, the AI on team A
//! in odd games and on team B in even ones, every seat dealt by the AI's deck rule. A game is a pure
//! function of its number, so the games run in parallel (rayon) and the report is the one
//! `duos_gate(games, DUOS_GATE_SERIES)` gives. Without `--games` it plays DUOS_GATE_GAMES_SMOKE. The
//! gate passes when the AI team wins at least DUOS_GATE_WIN_FLOOR of the games and no game had an
//! action refused, on either team, nor an AI action off a fallback path, nor a panic an AI seat's
//! search caught (#1429, `DuosGateReport::passes`); a draw counts for nothing. Output goes to stdout;
//! a failed gate returns `Err` naming the games not won, the games with a refusal, the games with a
//! fallback (seed and count only) and the games with a search panic (seed, count, and the first
//! one's seat and message) (exit 1).

use anyhow::{Result, anyhow};
use rayon::prelude::*;

use jackioh_ai::{
    DUOS_GATE_GAMES_SMOKE, DUOS_GATE_SERIES, DUOS_GATE_WIN_FLOOR, DuosGateGame, DuosGateReport,
    duos_gate_game, duos_gate_report,
};

/// `cargo jackioh gate --duos [--games N]`.
pub fn run(games: Option<usize>) -> Result<()> {
    jackioh_cards::register_all();
    let games = games.unwrap_or(DUOS_GATE_GAMES_SMOKE);
    let numbers: Vec<usize> = (1..=games).collect();
    let played: Vec<DuosGateGame> = numbers
        .par_iter()
        .map(|&n| duos_gate_game(n, DUOS_GATE_SERIES))
        .collect();
    for game in &played {
        println!(
            "[gate duos] {}",
            serde_json::to_string(game).unwrap_or_else(|error| format!("<{error}>"))
        );
    }
    let report = duos_gate_report(&played);
    println!(
        "[gate duos] {}",
        serde_json::to_string(&report).unwrap_or_else(|error| format!("<{error}>"))
    );
    verdict(&report, &played)
}

/// Pass, or an `Err` naming the floor, the counts, every game the AI team did not win, every game
/// with a refused action (its seed, the AI's team and how many of its refusals the AI team sent),
/// every game with a fallback (its seed and count, nothing else) and every game whose AI searches
/// caught a panic (its seed, the count, and the first one's seat and message, #1429).
fn verdict(report: &DuosGateReport, played: &[DuosGateGame]) -> Result<()> {
    if report.passes() {
        return Ok(());
    }
    let not_won: Vec<String> = played
        .iter()
        .filter(|game| !game.won)
        .map(|game| {
            format!(
                "{} (AI team {}, {})",
                game.seed,
                game.subject,
                game.reason.as_deref().unwrap_or("unfinished")
            )
        })
        .collect();
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
    let fell_back: Vec<String> = played
        .iter()
        .filter(|game| game.fallbacks > 0)
        .map(|game| format!("{} ({} fallbacks)", game.seed, game.fallbacks))
        .collect();
    let panicked: Vec<String> = played
        .iter()
        .filter(|game| game.sim_panics > 0)
        .map(|game| match &game.first_sim_panic {
            Some(first) => format!(
                "{} ({} search panics, the first in {}'s search: {})",
                game.seed, game.sim_panics, first.seat, first.message
            ),
            None => format!("{} ({} search panics)", game.seed, game.sim_panics),
        })
        .collect();
    Err(anyhow!(
        "the Duos gate failed: {} wins and {} draws in {} games ({:.3}), {DUOS_GATE_WIN_FLOOR} needed; \
         {} actions refused, none allowed; {} fallback actions, none allowed; {} search panics, none \
         allowed; not won: {}; refused: {}; fell back: {}; panicked: {}",
        report.wins,
        report.draws,
        report.games,
        report.rate(),
        report.rejected,
        report.fallbacks,
        report.sim_panics,
        listed(&not_won),
        listed(&refused),
        listed(&fell_back),
        listed(&panicked)
    ))
}

/// The verdict's list: its items joined, or "none".
fn listed(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    //! The verdict alone: playing the games is the AI crate's test (B32).

    use super::*;
    use jackioh_ai::DuosSimPanic;
    use jackioh_engine::wire::{DuosSeat, Team};

    fn game(n: usize, won: bool) -> DuosGateGame {
        DuosGateGame {
            seed: format!("{DUOS_GATE_SERIES}:{n}"),
            subject: jackioh_ai::duos_gate_subject(n),
            won,
            drawn: false,
            reason: Some("poolEmptied".to_string()),
            actions: 1,
            rejected: 0,
            ai_rejected: 0,
            fallbacks: 0,
            sim_panics: 0,
            first_sim_panic: None,
        }
    }

    #[test]
    fn b32_the_duos_gate_passes_at_the_floor_and_fails_one_win_under_it() {
        let needed = (DUOS_GATE_WIN_FLOOR * DUOS_GATE_GAMES_SMOKE as f64).ceil() as usize;
        let at_floor: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE)
            .map(|n| game(n, n <= needed))
            .collect();
        assert!(verdict(&duos_gate_report(&at_floor), &at_floor).is_ok());

        let under: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE).map(|n| game(n, n < needed)).collect();
        let error = verdict(&duos_gate_report(&under), &under)
            .unwrap_err()
            .to_string();
        assert!(error.contains(&format!("{DUOS_GATE_SERIES}:{needed}")), "{error}");
    }

    #[test]
    fn b32_the_ai_plays_team_a_in_odd_games_and_team_b_in_even_ones() {
        assert_eq!(jackioh_ai::duos_gate_subject(1), Team::A);
        assert_eq!(jackioh_ai::duos_gate_subject(2), Team::B);
    }

    #[test]
    fn b32_a_refused_action_fails_the_duos_gate_with_every_game_won_and_names_its_seed_and_team() {
        let mut played: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE).map(|n| game(n, true)).collect();
        assert!(verdict(&duos_gate_report(&played), &played).is_ok());

        played[2].rejected = 2;
        played[2].ai_rejected = 1;
        played[3].rejected = 1;
        let error = verdict(&duos_gate_report(&played), &played)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!(
                "{DUOS_GATE_SERIES}:3 (AI team a, 2 refused, 1 by the AI team)"
            )),
            "{error}"
        );
        assert!(
            error.contains(&format!(
                "{DUOS_GATE_SERIES}:4 (AI team b, 1 refused, 0 by the AI team)"
            )),
            "{error}"
        );
        assert!(error.contains("3 actions refused"), "{error}");
    }

    #[test]
    fn b32_a_fallback_fails_the_duos_gate_with_every_game_won_and_names_its_seed_and_count() {
        let mut played: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE).map(|n| game(n, true)).collect();
        assert!(verdict(&duos_gate_report(&played), &played).is_ok());

        played[2].fallbacks = 2;
        played[3].fallbacks = 1;
        let error = verdict(&duos_gate_report(&played), &played)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!(
                "fell back: {DUOS_GATE_SERIES}:3 (2 fallbacks), {DUOS_GATE_SERIES}:4 (1 fallbacks)"
            )),
            "{error}"
        );
        assert!(error.contains("3 fallback actions, none allowed"), "{error}");
        assert!(error.contains("refused: none"), "{error}");
    }

    #[test]
    fn b32_a_search_panic_fails_the_duos_gate_with_every_game_won_and_names_its_seed_seat_and_message() {
        let mut played: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE).map(|n| game(n, true)).collect();
        assert!(verdict(&duos_gate_report(&played), &played).is_ok());

        played[2].sim_panics = 2;
        played[2].first_sim_panic = Some(DuosSimPanic {
            seat: DuosSeat::A2,
            message: "a guessed world broke".into(),
        });
        let error = verdict(&duos_gate_report(&played), &played)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!(
                "panicked: {DUOS_GATE_SERIES}:3 (2 search panics, the first in a2's search: a guessed world \
                 broke)"
            )),
            "{error}"
        );
        assert!(error.contains("2 search panics, none allowed"), "{error}");
        assert!(error.contains("fell back: none"), "{error}");
    }

    #[test]
    fn b32_no_games_is_no_pass() {
        assert!(verdict(&duos_gate_report(&[]), &[]).is_err());
    }
}
