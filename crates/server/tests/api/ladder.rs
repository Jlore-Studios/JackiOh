//! The visible ladder (SPEC §9.12, R605–R608), as pure functions: where a position sits, what the
//! hidden rating calls for, how one game moves it, the floor, and Jlorious.
//!
//! Port of `apps/server/test/ranked/ladder.test.ts` (part 18). The ladder's answers that travel to
//! the client (`VisibleRank`, `PeakBadge`, the Grape tier names) are compared as the JSON they
//! serialise to, which is what TS's `toEqual` on those objects compared.

use std::collections::BTreeMap;

use jackioh_server::config::{
    JLORIOUS_SIZE, RANK_CONVERGENCE_GAP_PIPS, RANK_DIVISIONS_PER_TIER, RANK_PIPS_PER_DIVISION,
    RANK_PLACEMENT_GAMES, RANK_STREAK_LENGTH, RANK_TIER_PERCENTS,
};
use jackioh_server::ranked::ladder::{
    ApplyRankedGameInput, GRAPE_TIERS, GameResult, LADDER_TOP, PIPS_PER_TIER, Percentile, PipDeltaInput,
    SeasonRank, Standing, apply_ranked_game, fresh_rank, jlorious_order, peak_badge, percentile_of,
    pip_delta, place_of, target_ladder, tier_bottom, tier_index_of, visible_rank, with_jlorious_peak,
};
use serde_json::{Value, json};

const AT: i64 = 1_700_000_000_000;

/// A value as the JSON it serialises to.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("a ladder value serialises")
}

/// A config count as a `usize`, whichever integer type `config.rs` gives it.
fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` and matches, recursively.
fn is_match(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|held| is_match(held, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(held, value)| is_match(held, value))
        }
        _ => actual == expected,
    }
}

fn assert_match_object(actual: Value, expected: Value) {
    assert!(is_match(&actual, &expected), "{actual} does not match {expected}");
}

/// `placeOf(ladder)` as the client reads it: `{ tier, division, pips }`.
fn place(ladder: i32) -> Value {
    let place = place_of(ladder);
    json!({ "tier": place.tier, "division": place.division, "pips": place.pips })
}

/// The Grape tier a ladder position is in, by its name.
fn tier_name(ladder: i32) -> String {
    js(place_of(ladder).tier)
        .as_str()
        .expect("a tier serialises as its name")
        .to_string()
}

/// TS `targetLadder(percentile)`.
fn target(percentile: Percentile) -> i32 {
    target_ladder(&percentile)
}

/// TS `pipDelta({ result, ladder, target, streak })`.
fn delta(result: GameResult, ladder: i32, target: i32, streak: i32) -> i32 {
    pip_delta(&PipDeltaInput {
        result,
        ladder,
        target,
        streak,
    })
}

/// TS `applyRankedGame(rank, { result, target, at: AT })`.
fn apply(rank: &SeasonRank, result: GameResult, target: i32) -> SeasonRank {
    apply_ranked_game(
        rank,
        &ApplyRankedGameInput {
            result,
            target,
            at: AT,
        },
    )
}

/// A placed player's row at `ladder`.
fn placed(ladder: i32) -> SeasonRank {
    SeasonRank {
        games: RANK_PLACEMENT_GAMES,
        ladder: Some(ladder),
        floor: tier_index_of(ladder),
        peak_ladder: Some(ladder),
        ..fresh_rank("v0.1", "p", AT)
    }
}

/// Plays `results` on `rank`, each with the target `target`.
fn play(rank: &SeasonRank, results: &[GameResult], target: i32) -> SeasonRank {
    results
        .iter()
        .fold(rank.clone(), |row, &result| apply(&row, result, target))
}

/// A win on even games and a loss on odd ones.
fn alternating(game: i32) -> GameResult {
    if game % 2 == 0 {
        GameResult::Win
    } else {
        GameResult::Loss
    }
}

mod r605_the_ladders_shape {
    use super::*;

    #[test]
    fn r605_has_five_grape_tiers_of_three_divisions_iii_up_to_i_each_of_rank_pips_per_division_pips() {
        assert_eq!(
            js(GRAPE_TIERS),
            json!(["rotten", "normal", "large", "golden", "mythic"])
        );
        assert_eq!(RANK_DIVISIONS_PER_TIER, 3);
        assert_eq!(place(0), json!({ "tier": "rotten", "division": 3, "pips": 0 }));
        assert_eq!(
            place(RANK_PIPS_PER_DIVISION - 1),
            json!({ "tier": "rotten", "division": 3, "pips": RANK_PIPS_PER_DIVISION - 1 })
        );
        // A full division is the bottom of the next one.
        assert_eq!(
            place(RANK_PIPS_PER_DIVISION),
            json!({ "tier": "rotten", "division": 2, "pips": 0 })
        );
        assert_eq!(
            place(PIPS_PER_TIER - 1),
            json!({ "tier": "rotten", "division": 1, "pips": RANK_PIPS_PER_DIVISION - 1 })
        );
        assert_eq!(
            place(PIPS_PER_TIER),
            json!({ "tier": "normal", "division": 3, "pips": 0 })
        );
        assert_eq!(
            place(LADDER_TOP),
            json!({ "tier": "mythic", "division": 1, "pips": RANK_PIPS_PER_DIVISION - 1 })
        );
    }

    #[test]
    fn r605_a_player_is_a_raisin_until_the_seasons_placements_are_played_then_stands_where_the_rating_calls_for()
     {
        let target = tier_bottom(2) + 4; // Large Grape II, 1 pip
        let mut rank = fresh_rank("v0.1", "p", AT);
        for game in 1..RANK_PLACEMENT_GAMES {
            rank = apply(&rank, alternating(game), target);
            assert_eq!(rank.ladder, None);
            assert_eq!(
                js(visible_rank(Some(&rank), None)),
                json!({ "tier": "raisin", "placementsPlayed": game, "placementGames": RANK_PLACEMENT_GAMES })
            );
        }
        rank = apply(&rank, GameResult::Loss, target);
        // The game that completes them places the player at the target, whatever its own result.
        assert_eq!(rank.ladder, Some(target));
        assert_eq!(rank.floor, 2);
        assert_eq!(rank.peak_ladder, Some(target));
        assert_eq!(
            js(visible_rank(Some(&rank), None)),
            json!({
                "tier": "large",
                "division": 2,
                "pips": 1,
                "pipsPerDivision": RANK_PIPS_PER_DIVISION,
                "floor": "large"
            })
        );
        assert_eq!(
            js(visible_rank(None, None)),
            json!({ "tier": "raisin", "placementsPlayed": 0, "placementGames": RANK_PLACEMENT_GAMES })
        );
    }

    #[test]
    fn r605_counts_every_rated_game_of_the_season_placements_included() {
        use jackioh_server::ranked::ladder::GameResult::{Draw, Loss, Win};
        let rank = play(
            &fresh_rank("v0.1", "p", AT),
            &[Win, Loss, Draw, Win, Win, Win],
            10,
        );
        assert_eq!((rank.games, rank.wins, rank.losses, rank.draws), (6, 4, 1, 1));
    }
}

mod r606_the_rank_the_hidden_rating_calls_for {
    use super::*;

    #[test]
    fn r606_is_the_ratings_mid_rank_percentile_among_the_seasons_placed_players() {
        let alone = percentile_of(1000.0, &[]);
        assert_eq!((alone.numerator, alone.denominator), (1, 2));
        let among = percentile_of(1000.0, &[900.0, 1000.0, 1100.0]);
        // 2·below + level + 1 over 2·(others + 1): one below, one level.
        assert_eq!((among.numerator, among.denominator), (4, 8));
    }

    #[test]
    fn r606_spreads_a_population_over_the_tiers_in_rank_tier_percents_rotten_12_normal_60_large_20_golden_7_mythic_1()
     {
        assert_eq!(
            (
                RANK_TIER_PERCENTS.rotten,
                RANK_TIER_PERCENTS.normal,
                RANK_TIER_PERCENTS.large,
                RANK_TIER_PERCENTS.golden,
                RANK_TIER_PERCENTS.mythic,
            ),
            (12, 60, 20, 7, 1)
        );
        let ratings: Vec<f64> = (0..1000_i32).map(|i| 700.0 + f64::from(i)).collect();
        let mut counts: BTreeMap<String, i32> = BTreeMap::new();
        for &rating in &ratings {
            let others: Vec<f64> = ratings.iter().copied().filter(|&other| other != rating).collect();
            let tier = tier_name(target(percentile_of(rating, &others)));
            *counts.entry(tier).or_insert(0) += 1;
        }
        let expected: BTreeMap<String, i32> = [
            ("rotten", 120),
            ("normal", 600),
            ("large", 200),
            ("golden", 70),
            ("mythic", 10),
        ]
        .into_iter()
        .map(|(tier, players)| (tier.to_string(), players))
        .collect();
        assert_eq!(counts, expected);
    }

    #[test]
    fn r606_compares_a_boundary_exactly_a_percentile_on_one_belonging_to_the_tier_above() {
        // 50 players: the best stands at exactly the 99th percentile, Mythic's lower edge.
        assert_eq!(
            tier_name(target(Percentile {
                numerator: 99,
                denominator: 100
            })),
            "mythic"
        );
        assert_eq!(
            tier_name(target(Percentile {
                numerator: 12,
                denominator: 100
            })),
            "normal"
        );
        assert_eq!(
            place(target(Percentile {
                numerator: 11,
                denominator: 100
            })),
            json!({ "tier": "rotten", "division": 1, "pips": 2 })
        );
        // The very bottom and the very top.
        assert_eq!(
            target(Percentile {
                numerator: 1,
                denominator: 10_000
            }),
            0
        );
        assert_eq!(
            target(Percentile {
                numerator: 9_999,
                denominator: 10_000
            }),
            LADDER_TOP
        );
        // Alone in the season: the middle of Normal Grape.
        assert_eq!(
            place(target(percentile_of(1000.0, &[]))),
            json!({ "tier": "normal", "division": 2, "pips": 2 })
        );
    }
}

mod r606_how_one_game_moves_a_placed_player {
    use super::*;
    use jackioh_server::ranked::ladder::GameResult::{Draw, Loss, Win};

    /// Normal Grape II, 1 pip.
    fn mid() -> i32 {
        tier_bottom(1) + 4
    }

    #[test]
    fn r606_a_win_gives_a_pip_a_loss_takes_one_and_a_draw_moves_none_near_the_target() {
        let mid = mid();
        assert_eq!(delta(Win, mid, mid, 1), 1);
        assert_eq!(delta(Loss, mid, mid, 0), -1);
        assert_eq!(delta(Draw, mid, mid, 0), 0);
        // Inside one division of the target is still near.
        assert_eq!(delta(Win, mid, mid + RANK_CONVERGENCE_GAP_PIPS - 1, 1), 1);
        assert_eq!(delta(Loss, mid, mid - RANK_CONVERGENCE_GAP_PIPS + 1, 0), -1);
    }

    #[test]
    fn r606_leans_one_pip_toward_a_target_a_division_or_more_away_and_no_more_however_far() {
        let mid = mid();
        let above = mid + RANK_CONVERGENCE_GAP_PIPS;
        let below = mid - RANK_CONVERGENCE_GAP_PIPS;
        // Rated above the visible rank: wins count double, losses as ever.
        assert_eq!(delta(Win, mid, above, 1), 2);
        assert_eq!(delta(Win, mid, LADDER_TOP, 1), 2);
        assert_eq!(delta(Loss, mid, above, 0), -1);
        // Rated below it: losses count double, wins as ever.
        assert_eq!(delta(Loss, mid, below, 0), -2);
        assert_eq!(delta(Loss, mid, 0, 0), -2);
        assert_eq!(delta(Win, mid, below, 1), 1);
    }

    #[test]
    fn r606_converges_a_player_who_wins_half_their_games_drifts_to_the_rank_the_rating_calls_for() {
        // Up across tiers, and down inside one (the floor, R607, stops a fall across a tier boundary).
        for (start, target) in [
            (tier_bottom(0), tier_bottom(3) + 4),
            (tier_bottom(1) + PIPS_PER_TIER - 2, tier_bottom(1)),
        ] {
            let mut rank = placed(start);
            for game in 0..400 {
                rank = apply(&rank, alternating(game), target);
            }
            assert!((rank.ladder.unwrap_or(0) - target).abs() < RANK_CONVERGENCE_GAP_PIPS + 1);
        }
    }

    #[test]
    fn r606_a_win_streak_earns_a_bonus_pip_from_its_rank_streak_length_th_win_below_mythic_grape_only() {
        let mid = mid();
        let wins = vec![Win; count(RANK_STREAK_LENGTH)];
        // Below Mythic: the streak's third win gives two pips.
        let rank = play(&placed(mid), &wins, mid);
        assert_eq!(rank.streak, RANK_STREAK_LENGTH);
        assert_eq!(rank.ladder, Some(mid + RANK_STREAK_LENGTH + 1));
        // A loss ends the streak; a draw neither ends nor extends it.
        assert_eq!(play(&rank, &[Loss], mid).streak, 0);
        assert_eq!(
            play(
                &SeasonRank {
                    streak: 2,
                    ..placed(mid)
                },
                &[Draw],
                mid
            )
            .streak,
            2
        );
        assert_eq!(
            play(
                &SeasonRank {
                    streak: 2,
                    ..placed(mid)
                },
                &[Draw, Win],
                mid
            )
            .ladder,
            Some(mid + 2)
        );
        // In Mythic Grape a streak earns nothing extra.
        let mythic = tier_bottom(4);
        assert_eq!(
            play(&placed(mythic), &wins, mythic).ladder,
            Some(mythic + RANK_STREAK_LENGTH)
        );
    }

    #[test]
    fn r606_holds_a_player_at_the_top_of_mythic_grape_i() {
        assert_eq!(
            play(&placed(LADDER_TOP), &[Win, Win], LADDER_TOP).ladder,
            Some(LADDER_TOP)
        );
    }
}

mod r607_the_tier_floor_and_the_seasons_peak {
    use super::*;
    use jackioh_server::ranked::ladder::GameResult::{Loss, Win};

    #[test]
    fn r607_a_player_cannot_drop_below_the_grape_tier_they_have_reached_this_season_but_divisions_inside_it_can_drop()
     {
        let large = tier_bottom(2) + 4; // Large Grape II, 1 pip
        let fallen = play(&placed(large), &[Loss; 7], 0);
        assert_eq!(fallen.ladder, Some(tier_bottom(2)));
        assert_match_object(
            js(visible_rank(Some(&fallen), None)),
            json!({ "tier": "large", "division": 3, "pips": 0, "floor": "large" }),
        );
        // The floor is the highest tier reached, not where the player started.
        let climbed = play(&placed(tier_bottom(3) - 1), &[Win], tier_bottom(3) - 1);
        assert_eq!(climbed.floor, 3);
        assert_eq!(play(&climbed, &[Loss; 3], 0).ladder, Some(tier_bottom(3)));
        assert_eq!(play(&climbed, &[Loss; 3], 0).peak_ladder, Some(tier_bottom(3)));
    }

    #[test]
    fn r607_keeps_the_seasons_best_position_as_the_profiles_badge() {
        // +1, +1, then +2 for the streak's third win; then two losses, the first leaning toward the
        // target four pips below.
        let rank = play(
            &placed(tier_bottom(1)),
            &[Win, Win, Win, Loss, Loss],
            tier_bottom(1),
        );
        assert_eq!(rank.ladder, Some(tier_bottom(1) + 1));
        assert_eq!(rank.peak_ladder, Some(tier_bottom(1) + 4));
        assert_eq!(
            js(peak_badge(&rank)),
            json!({ "seasonId": "v0.1", "tier": "normal", "division": 2 })
        );
        assert!(peak_badge(&fresh_rank("v0.1", "p", AT)).is_none());
        let jlorious = with_jlorious_peak(&with_jlorious_peak(&placed(tier_bottom(4)), 12), 40);
        assert_eq!(jlorious.peak_jlorious, Some(12));
        assert_eq!(
            js(peak_badge(&jlorious)),
            json!({ "seasonId": "v0.1", "tier": "jlorious", "position": 12 })
        );
    }
}

mod r608_jlorious {
    use super::*;

    fn mythic() -> i32 {
        tier_bottom(4)
    }

    fn standing(profile_id: &str, ladder: Option<i32>, rating: f64) -> Standing {
        Standing {
            profile_id: profile_id.to_string(),
            ladder,
            rating,
        }
    }

    #[test]
    fn r608_is_the_top_jlorious_size_mythic_grape_players_by_hidden_rating_ties_on_profile_id() {
        assert_eq!(JLORIOUS_SIZE, 100);
        let mut standings: Vec<Standing> = (0..130_i32)
            .map(|i| {
                standing(
                    &format!("m-{i:03}"),
                    Some(mythic() + (i % 9)),
                    2000.0 - f64::from(i),
                )
            })
            .collect();
        standings.push(standing("tie-b", Some(mythic()), 1950.5));
        standings.push(standing("tie-a", Some(mythic()), 1950.5));
        let order = jlorious_order(&standings);
        assert_eq!(order.len(), count(JLORIOUS_SIZE));
        assert_eq!(order[0], "m-000");
        let index_of = |id: &str| {
            order
                .iter()
                .position(|entry| entry == id)
                .unwrap_or_else(|| panic!("{id} is not Jlorious"))
        };
        assert_eq!(index_of("tie-a"), index_of("tie-b") - 1);
        // The 101st by rating is out, and back in Mythic Grape.
        assert!(!order.iter().any(|entry| entry == "m-129"));
    }

    #[test]
    fn r608_is_every_mythic_grape_player_when_fewer_than_jlorious_size_qualify_and_nobody_below_mythic_or_still_placing()
     {
        let order = jlorious_order(&[
            standing("golden-but-rated-highest", Some(tier_bottom(3) + 8), 2600.0),
            standing("raisin-rated-high", None, 2500.0),
            standing("mythic-low", Some(mythic()), 1500.0),
            standing("mythic-high", Some(mythic() + 1), 1700.0),
        ]);
        assert_eq!(order, ["mythic-high", "mythic-low"]);
    }

    #[test]
    fn r608_is_shown_as_a_numbered_position_instead_of_a_division_and_a_player_who_falls_out_is_in_mythic_grape_again()
     {
        let rank = placed(mythic() + 2);
        assert_eq!(
            js(visible_rank(Some(&rank), Some(7))),
            json!({ "tier": "jlorious", "position": 7 })
        );
        assert_match_object(
            js(visible_rank(Some(&rank), None)),
            json!({ "tier": "mythic", "division": 3, "pips": 2 }),
        );
    }
}
