//! Hidden rating maths (SPEC §9.12, R603): Glicko-2 against reference values.

use jackioh_shared::config::{GLICKO_TAU, RATING_DEVIATION_START, RATING_START, RATING_VOLATILITY_START};
use jackioh_shared::ranked::glicko2::{
    Glicko, RatedOpponent, START_GLICKO, Score, glicko2_period, rate_game,
};

fn glicko(rating: f64, deviation: f64) -> Glicko {
    glicko_with(rating, deviation, 0.06)
}

fn glicko_with(rating: f64, deviation: f64, volatility: f64) -> Glicko {
    Glicko {
        rating,
        deviation,
        volatility,
    }
}

/// One game of a rating period: the opponent's rating before it, and the score against them.
fn game(opponent: Glicko, score: Score) -> RatedOpponent {
    RatedOpponent { opponent, score }
}

/// Glicko-2 period at default tau.
fn period(player: Glicko, games: &[RatedOpponent]) -> Glicko {
    glicko2_period(&player, games, GLICKO_TAU)
}

/// Rates game between two players.
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
    fn r603_rates_1500_200_0_06_after_beating_1400_30_and_losing_to_1550_100_and_1700_300_as_the_paper_does()
    {
        let after = period(
            glicko(1500.0, 200.0),
            &[
                game(glicko(1400.0, 30.0), 1.0),
                game(glicko(1550.0, 100.0), 0.0),
                game(glicko(1700.0, 300.0), 0.0),
            ],
        );
        // The paper's printed results: r' = 1464.06, RD' = 151.52, σ' = 0.05999 (each rounded from
        // rounded intermediates, so compared at the paper's own last digit).
        assert!((after.rating - 1464.06).abs() < 0.01);
        assert!((after.deviation - 151.52).abs() < 0.01);
        assert!((after.volatility - 0.05999).abs() < 0.00001);
        // REFERENCE, unrounded.
        expect_reference(
            after,
            glicko_with(1464.050670819481, 151.516521926373, 0.05999598440084),
        );
    }

    #[test]
    fn r603_the_update_depends_only_on_rating_differences_so_the_scales_center_is_any_number() {
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
            &[
                game(glicko(1400.0, 30.0), 0.5),
                game(glicko(1550.0, 100.0), 0.5),
                game(glicko(1700.0, 300.0), 0.5),
            ],
        );
        expect_reference(
            after,
            glicko_with(1509.107200047628, 151.516520727744, 0.05999567822515),
        );
    }

    #[test]
    fn r603_a_draw_between_two_new_players_moves_neither_rating_and_narrows_both_deviations_reference() {
        let (a, b) = rate(START_GLICKO, START_GLICKO, 0.5);
        for side in [a, b] {
            assert_eq!(side.rating, float(RATING_START));
            expect_reference(
                Glicko {
                    rating: 1500.0,
                    ..side
                },
                glicko_with(1500.0, 290.318959913803, 0.05999896145086),
            );
        }
    }

    #[test]
    fn r603_a_draw_pulls_an_underdog_up_and_a_favorite_down_reference() {
        let (under, over) = rate(glicko(1400.0, 80.0), glicko(1600.0, 120.0), 0.5);
        expect_reference(
            under,
            glicko_with(1408.306768783764, 79.272855907714, 0.05999850515082),
        );
        expect_reference(
            over,
            glicko_with(1581.089049161912, 115.693290645246, 0.05999851743814),
        );
    }
}

mod r603_one_rated_game_is_one_rating_period {
    use super::*;

    #[test]
    fn r603_a_win_and_a_loss_between_new_players_are_mirror_images_reference() {
        let (winner, loser) = rate(START_GLICKO, START_GLICKO, 1.0);
        expect_reference(
            Glicko {
                rating: winner.rating + 500.0,
                ..winner
            },
            glicko_with(1662.310895033019, 290.318962017920, 0.0599996753731),
        );
        expect_reference(
            Glicko {
                rating: loser.rating + 500.0,
                ..loser
            },
            glicko_with(1337.689104966981, 290.318962017920, 0.0599996753731),
        );
    }

    #[test]
    fn r603_an_upset_moves_both_sides_by_their_own_deviations_each_from_the_others_rating_before_reference() {
        let (underdog, favorite) = rate(glicko(1350.0, 60.0), glicko(1720.0, 45.0), 1.0);
        expect_reference(
            underdog,
            glicko_with(1368.629463000698, 60.547858911524, 0.06000902052519),
        );
        expect_reference(
            favorite,
            glicko_with(1709.330958947476, 46.03837643654, 0.06000892422778),
        );
    }

    #[test]
    fn r603_a_result_far_from_the_expected_one_takes_the_other_branch_of_the_volatility_bracket_reference() {
        // Δ² > φ² + v here, so step 5 starts B at ln(Δ² − φ² − v) rather than stepping down from a.
        let after = period(glicko(1500.0, 50.0), &[game(glicko(2400.0, 30.0), 1.0)]);
        expect_reference(
            after,
            glicko_with(1514.856410184712, 51.062871777297, 0.06001314423775),
        );
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
            [
                float(RATING_START),
                float(RATING_DEVIATION_START),
                RATING_VOLATILITY_START
            ],
            [1000.0, 350.0, 0.06]
        );
    }
}

mod r603_step_5_to_the_bit {
    use super::*;
    use jackioh_shared::config::{BOT_PROMOTION_DEVIATION_BOOST, GLICKO_SCALE};
    use jackioh_shared::ranked::glicko2::widen_for_promotions;

    /// R603: step 5 solves for the new volatility only to GLICKO_CONVERGENCE, about 3e-8 of a volatility, so a
    /// reference held to 1e-8 cannot see the step taken another way. These hold it exactly as Glickman's §5 sets
    /// it out (f, the bracket's start on either side of Δ² against φ² + v, the Illinois update and its halving),
    /// to the bit: on the paper's period, and on a result far from the expected one, each way.
    #[test]
    fn r603_the_volatility_step_is_glickmans_section_5_to_the_bit() {
        let paper = period(
            glicko(1500.0, 200.0),
            &[
                game(glicko(1400.0, 30.0), 1.0),
                game(glicko(1550.0, 100.0), 0.0),
                game(glicko(1700.0, 300.0), 0.0),
            ],
        );
        assert_eq!(
            paper,
            glicko_with(1464.050670819693, 151.5165219259256, 0.05999598428664987),
            "{paper:?}"
        );
        let win = period(glicko(1500.0, 50.0), &[game(glicko(2400.0, 30.0), 1.0)]);
        assert_eq!(
            win,
            glicko_with(1514.8564101846532, 51.062871777196214, 0.06001314423490439),
            "{win:?}"
        );
        // Δ is negative here: Δ² > φ² + v all the same.
        let loss = period(glicko(1500.0, 50.0), &[game(glicko(600.0, 30.0), 0.0)]);
        assert_eq!(
            loss,
            glicko_with(1485.1435898153468, 51.062871777196214, 0.06001314423490439),
            "{loss:?}"
        );
    }

    /// R603: a period with no games keeps the rating and the volatility and widens the deviation by the volatility
    /// in quadrature (Glickman's step 6 alone).
    #[test]
    fn r603_a_period_with_no_games_widens_only_the_deviation() {
        let after = period(glicko(1500.0, 200.0), &[]);
        let phi = 200.0 / GLICKO_SCALE;
        let widened = (phi * phi + 0.06 * 0.06).sqrt() * GLICKO_SCALE;
        assert_eq!((after.rating, after.volatility), (1500.0, 0.06));
        assert!(
            (after.deviation - widened).abs() < 1e-9,
            "{after:?} against {widened}"
        );
    }

    /// R1560000: each promotion widens a bot's deviation by BOT_PROMOTION_DEVIATION_BOOST in quadrature, under the
    /// cap, and leaves its rating and volatility.
    #[test]
    fn r1560000_each_promotion_widens_the_deviation_in_quadrature_under_the_cap() {
        let settled = glicko(1500.0, 50.0);
        for promotions in [1_i64, 2] {
            let widened = widen_for_promotions(&settled, promotions);
            let expected = (50.0_f64 * 50.0
                + promotions as f64 * BOT_PROMOTION_DEVIATION_BOOST * BOT_PROMOTION_DEVIATION_BOOST)
                .sqrt();
            assert!(expected < RATING_DEVIATION_START, "PREMISE: under the cap");
            assert!(
                (widened.deviation - expected).abs() < 1e-9,
                "{promotions}: {widened:?} against {expected}"
            );
            assert_eq!((widened.rating, widened.volatility), (1500.0, 0.06));
        }
    }
}
