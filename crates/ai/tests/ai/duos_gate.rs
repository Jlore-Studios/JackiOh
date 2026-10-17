//! B32 (#936): the Duos gate at smoke size. The Easy AI team (both seats on `duos_decide`) against a
//! random-policy team has to win at least DUOS_GATE_WIN_FLOOR of DUOS_GATE_GAMES_SMOKE games. No action
//! of either team may be refused; a failure names each refused game's seed, which
//! `duos_gate_game(n, "gate")` replays. It plays whole Duos games, so it runs in `ai_slow_b.rs` rather
//! than the `ai` binary.

use jackioh_ai::{
    DUOS_GATE_GAMES_SMOKE, DUOS_GATE_WIN_FLOOR, DuosGateGame, duos_gate_game, duos_gate_report,
    duos_gate_subject,
};

use super::support::register_cards;

#[test]
fn b32_the_easy_ai_team_beats_a_random_policy_team_in_at_least_the_floor_of_the_smoke_games() {
    register_cards();
    let played: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE)
        .map(|n| duos_gate_game(n, "gate"))
        .collect();
    let report = duos_gate_report(&played);
    assert_eq!(report.games, DUOS_GATE_GAMES_SMOKE);
    assert!(
        report.wins + report.draws <= report.games,
        "{} wins, {} draws",
        report.wins,
        report.draws
    );
    let floor = DUOS_GATE_WIN_FLOOR * DUOS_GATE_GAMES_SMOKE as f64;
    assert!(
        report.wins as f64 >= floor,
        "{} wins of {} games, under the floor of {floor}",
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
    assert!(report.passes());
}

#[test]
fn b32_a_refused_action_fails_the_duos_gate_report_even_with_every_game_won() {
    let clean: Vec<DuosGateGame> = (1..=DUOS_GATE_GAMES_SMOKE)
        .map(|n| DuosGateGame {
            seed: format!("gate:{n}"),
            subject: duos_gate_subject(n),
            won: true,
            drawn: false,
            reason: Some("poolEmptied".to_string()),
            actions: 1,
            rejected: 0,
            ai_rejected: 0,
        })
        .collect();
    assert_eq!(duos_gate_report(&clean).rejected, 0);
    assert!(duos_gate_report(&clean).passes());

    let mut refused = clean.clone();
    refused[0].rejected = 1;
    refused[0].ai_rejected = 1;
    refused[1].rejected = 2;
    let report = duos_gate_report(&refused);
    assert_eq!(report.wins, DUOS_GATE_GAMES_SMOKE);
    assert_eq!(report.rejected, 3);
    assert!(!report.passes());
}
