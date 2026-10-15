//! `cargo jackioh gate --duos [--games N]`: the Duos gate (#936 B32, S5). `jackioh_ai::duos` plays and
//! measures the games; this file runs them, prints the report and fails under the floor.
//!
//! Game n of a run is `duos_gate_game(n, DUOS_GATE_SERIES)`: the Easy AI team (`duos_decide` at
//! AI_GATE_BUDGET) against a team on §10.7's random policy over `duos_legal_actions`, the AI on team A
//! in odd games and on team B in even ones, every seat dealt by the AI's deck rule. A game is a pure
//! function of its number, so the games run in parallel (rayon) and the report is the one
//! `duos_gate(games, DUOS_GATE_SERIES)` gives. Without `--games` it plays DUOS_GATE_GAMES_SMOKE. The
//! gate passes when the AI team wins at least DUOS_GATE_WIN_FLOOR of the games
//! (`DuosGateReport::passes`); a draw counts for nothing. Output goes to stdout; a failed gate
//! returns `Err` naming the games not won (exit 1).

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

/// Pass, or an `Err` naming the floor, the count and every game the AI team did not win.
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
    Err(anyhow!(
        "the Duos gate failed: {} wins and {} draws in {} games ({:.3}), {DUOS_GATE_WIN_FLOOR} needed; \
         not won: {}",
        report.wins,
        report.draws,
        report.games,
        report.rate(),
        not_won.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    //! The verdict alone: playing the games is the AI crate's test (B32).

    use super::*;
    use jackioh_engine::wire::Team;

    fn game(n: usize, won: bool) -> DuosGateGame {
        DuosGateGame {
            seed: format!("{DUOS_GATE_SERIES}:{n}"),
            subject: jackioh_ai::duos_gate_subject(n),
            won,
            drawn: false,
            reason: Some("poolEmptied".to_string()),
            actions: 1,
            rejected: 0,
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
    fn b32_no_games_is_no_pass() {
        assert!(verdict(&duos_gate_report(&[]), &[]).is_err());
    }
}
