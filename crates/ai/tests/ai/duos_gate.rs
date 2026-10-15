//! B32 (#936): the Duos gate at smoke size. The Easy AI team (both seats on `duos_decide`) against a
//! random-policy team has to win at least DUOS_GATE_WIN_FLOOR of DUOS_GATE_GAMES_SMOKE games. It plays
//! whole Duos games, so it runs in `ai_slow_b.rs` rather than the `ai` binary.

use jackioh_ai::{DUOS_GATE_GAMES_SMOKE, DUOS_GATE_WIN_FLOOR, duos_gate};

use super::support::register_cards;

#[test]
fn b32_the_easy_ai_team_beats_a_random_policy_team_in_at_least_the_floor_of_the_smoke_games() {
    register_cards();
    let report = duos_gate(DUOS_GATE_GAMES_SMOKE, "gate");
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
}
