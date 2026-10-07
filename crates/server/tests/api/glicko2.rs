//! The hidden rating's maths (SPEC §9.12, R603): Glicko-2 against reference values.
//!
//! Two sources. Glickman's own worked example ("Example of the Glicko-2 system", 2013, §"Example"),
//! at the precision the paper prints, which it rounds at every step; and the values below marked
//! REFERENCE, computed independently of this code by a transcription of the paper's steps into
//! Python's `decimal` module at 50 significant digits (ε = 1e-40), which is where draws are proved,
//! since the paper's example has none. Those match this module to 1e-6 in rating and deviation and
//! 1e-8 in volatility, the slack being only its own ε of `GLICKO_CONVERGENCE`.
//!
//! Port of `apps/server/test/ranked/glicko2.test.ts` (part 18).

use jackioh_server::config::{GLICKO_TAU, RATING_DEVIATION_START, RATING_START, RATING_VOLATILITY_START};
use jackioh_server::ranked::glicko2::{glicko2_period, rate_game, Glicko, RatedOpponent, Score, START_GLICKO};

fn glicko(rating: f64, deviation: f64) -> Glicko {
    glicko_with(rating, deviation, 0.06)
}

fn glicko_with(rating: f64, deviation: f64, volatility: f64) -> Glicko {
    Glicko { rating, deviation, volatility }
}

/// One game of a rating period: the opponent's rating before it, and the score against them.
fn game(opponent: Glicko, score: Score) -> RatedOpponent {
    RatedOpponent { opponent, score }
}

/// TS `glicko2Period(player, games)`: the period at the default τ (`GLICKO_TAU`).
fn period(player: Glicko, games: &[RatedOpponent]) -> Glicko {
    glicko2_period(&player, games, GLICKO_TAU)
}

/// TS `rateGame(a, b, scoreA)`, its `{ a, b }` read out as a pair.
fn rate(a: Glicko, b: Glicko, score_a: Score) -> (Glicko, Glicko) {
    let rated = rate_game(&a, &b, score_a);
    (rated.a, rated.b)
}

/// Asserts a computed rating against a 50-digit reference.
fn expect_reference(actual: Glicko, reference: Glicko) {
    assert!(
        (actual.rating - reference.rating).abs() < 1e-6,
        "rating {} against the reference {}",
        actual.rating,
        reference.rating
    );
    assert!(
        (actual.deviation - reference.deviation).abs() < 1e-6,
        "deviation {} against the reference {}",
        actual.deviation,
        reference.deviation
    );
    assert!(
        (actual.volatility - reference.volatility).abs() < 1e-8,
        "volatility {} against the reference {}",
        actual.volatility,
        reference.volatility
    );
}

/// A config number as `f64`, whichever numeric type `config.rs` gives it.
fn float<T: Into<f64>>(value: T) -> f64 {
    value.into()
}

/// Jest's `toBeCloseTo(expected, digits)`: within half a unit of the `digits`-th decimal.
fn close_to(actual: f64, expected: f64, digits: i32) -> bool {
    (actual - expected).abs() < 10f64.powi(-digits) / 2.0
}

mod r603_glicko_2_matches_glickmans_worked_example {
    use super::*;

    #[test]
    fn r603_rates_1500_200_0_06_after_beating_1400_30_and_losing_to_1550_100_and_1700_300_as_the_paper_does() {
        let after = period(
            glicko(1500.0, 200.0),
            &[game(glicko(1400.0, 30.0), 1.0), game(glicko(1550.0, 100.0), 0.0), game(glicko(1700.0, 300.0), 0.0)],
        );
        // The paper's printed results: r' = 1464.06, RD' = 151.52, σ' = 0.05999 (each rounded from
        // rounded intermediates, so compared at the paper's own last digit).
        assert!((after.rating - 1464.06).abs() < 0.01);
        assert!((after.deviation - 151.52).abs() < 0.01);
        assert!((after.volatility - 0.05999).abs() < 0.00001);
        // REFERENCE, unrounded.
        expect_reference(after, glicko_with(1464.050670819481, 151.516521926373, 0.05999598440084));
    }

    #[test]
    fn r603_the_update_depends_only_on_rating_differences_so_the_scales_centre_is_any_number() {
        // RATING_START is R79's 1000, not Glickman's 1500: the same example moved by 500 moves by 500.
        let games = |shift: f64| {
            [
                game(glicko(1400.0 + shift, 30.0), 1.0),
                game(glicko(1550.0 + shift, 100.0), 0.0),
                game(glicko(1700.0 + shift, 300.0), 0.0),
            ]
        };
        let at1500 = period(glicko(1500.0, 200.0), &games(0.0));
        let at1000 = period(glicko(1000.0, 200.0), &games(-500.0));
        assert!(close_to(at1000.rating + 500.0, at1500.rating, 9));
        assert!(close_to(at1000.deviation, at1500.deviation, 9));
        assert!(close_to(at1000.volatility, at1500.volatility, 12));
    }
}

mod r603_draws_count_as_half_a_win {
    use super::*;

    #[test]
    fn r603_the_papers_player_drawing_all_three_games_reference() {
        let after = period(
            glicko(1500.0, 200.0),
            &[game(glicko(1400.0, 30.0), 0.5), game(glicko(1550.0, 100.0), 0.5), game(glicko(1700.0, 300.0), 0.5)],
        );
        expect_reference(after, glicko_with(1509.107200047628, 151.516520727744, 0.05999567822515));
    }

    #[test]
    fn r603_a_draw_between_two_new_players_moves_neither_rating_and_narrows_both_deviations_reference() {
        let (a, b) = rate(START_GLICKO, START_GLICKO, 0.5);
        for side in [a, b] {
            assert_eq!(side.rating, float(RATING_START));
            expect_reference(
                Glicko { rating: 1500.0, ..side },
                glicko_with(1500.0, 290.318959913803, 0.05999896145086),
            );
        }
    }

    #[test]
    fn r603_a_draw_pulls_an_underdog_up_and_a_favourite_down_reference() {
        let (under, over) = rate(glicko(1400.0, 80.0), glicko(1600.0, 120.0), 0.5);
        expect_reference(under, glicko_with(1408.306768783764, 79.272855907714, 0.05999850515082));
        expect_reference(over, glicko_with(1581.089049161912, 115.693290645246, 0.05999851743814));
    }
}

mod r603_one_rated_game_is_one_rating_period {
    use super::*;

    #[test]
    fn r603_a_win_and_a_loss_between_new_players_are_mirror_images_reference() {
        let (winner, loser) = rate(START_GLICKO, START_GLICKO, 1.0);
        expect_reference(
            Glicko { rating: winner.rating + 500.0, ..winner },
            glicko_with(1662.310895033019, 290.318962017920, 0.0599996753731),
        );
        expect_reference(
            Glicko { rating: loser.rating + 500.0, ..loser },
            glicko_with(1337.689104966981, 290.318962017920, 0.0599996753731),
        );
    }

    #[test]
    fn r603_an_upset_moves_both_sides_by_their_own_deviations_each_from_the_others_rating_before_reference() {
        let (underdog, favourite) = rate(glicko(1350.0, 60.0), glicko(1720.0, 45.0), 1.0);
        expect_reference(underdog, glicko_with(1368.629463000698, 60.547858911524, 0.06000902052519));
        expect_reference(favourite, glicko_with(1709.330958947476, 46.03837643654, 0.06000892422778));
    }

    #[test]
    fn r603_a_result_far_from_the_expected_one_takes_the_other_branch_of_the_volatility_bracket_reference() {
        // Δ² > φ² + v here, so step 5 starts B at ln(Δ² − φ² − v) rather than stepping down from a.
        let after = period(glicko(1500.0, 50.0), &[game(glicko(2400.0, 30.0), 1.0)]);
        expect_reference(after, glicko_with(1514.856410184712, 51.062871777297, 0.06001314423775));
    }

    #[test]
    fn r603_is_deterministic_and_independent_of_which_side_is_computed_first() {
        let a = || glicko_with(1123.4, 77.7, 0.061);
        let b = || glicko_with(1088.8, 140.2, 0.059);
        let first = rate(a(), b(), 0.0);
        let swapped = rate(b(), a(), 1.0);
        assert_eq!(first.0, swapped.1);
        assert_eq!(first.1, swapped.0);
        assert_eq!(rate(a(), b(), 0.0), first);
    }

    #[test]
    fn r603_a_new_profile_starts_at_r79s_1000_with_glickmans_starting_deviation_and_volatility() {
        assert_eq!(
            START_GLICKO,
            Glicko {
                rating: float(RATING_START),
                deviation: float(RATING_DEVIATION_START),
                volatility: RATING_VOLATILITY_START,
            }
        );
        assert_eq!(
            [float(RATING_START), float(RATING_DEVIATION_START), RATING_VOLATILITY_START],
            [1000.0, 350.0, 0.06]
        );
    }
}
