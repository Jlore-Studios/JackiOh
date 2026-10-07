//! Seasons (SPEC §9.12, R609): the season a patch plays in, and the soft reset that opens one.
//!
//! Port of `apps/server/test/ranked/season.test.ts` (part 18).

use indexmap::IndexMap;
use jackioh_server::config::{RATING_DEVIATION_START, SEASON_RESET_DEVIATION_BOOST, SEASON_RESET_STRENGTH};
use jackioh_server::ranked::glicko2::Glicko;
use jackioh_server::ranked::season::{
    ResetChange, ResetPlayer, ResetReport, reset_glicko, season_id_of, soft_reset,
};

/// A config number as `f64`, whichever numeric type `config.rs` gives it.
fn float<T: Into<f64>>(value: T) -> f64 {
    value.into()
}

/// Jest's `toBeCloseTo(expected, digits)`: within half a unit of the `digits`-th decimal.
fn close_to(actual: f64, expected: f64, digits: i32) -> bool {
    (actual - expected).abs() < 10f64.powi(-digits) / 2.0
}

/// TS `softReset(players)`, its `{ changes, report }` read out as a pair.
fn reset(players: &[ResetPlayer]) -> (Vec<ResetChange>, ResetReport) {
    let result = soft_reset(players);
    (result.changes, result.report)
}

/// What `seasonIdOf` refused a version with: TS threw, the port panics (SURFACE §4.4.9).
fn refusal_of(version: &str) -> String {
    match std::panic::catch_unwind(|| season_id_of(version)) {
        Ok(season) => panic!("{version:?} was read as season {season:?} instead of being refused"),
        Err(payload) => payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_string()))
            .unwrap_or_default(),
    }
}

mod r609_a_season_per_minor_version {
    use super::*;

    #[test]
    fn r609_names_the_season_by_the_patch_versions_major_and_minor() {
        assert_eq!(season_id_of("v0.1.1"), "v0.1");
        assert_eq!(season_id_of("v0.1.0-r3"), "v0.1");
        assert_eq!(season_id_of("v0.2.6"), "v0.2");
        assert_eq!(season_id_of("v0.2"), "v0.2");
        assert_eq!(season_id_of("v1.10.0"), "v1.10");
        assert_ne!(season_id_of("v0.3.0"), season_id_of("v0.2.9"));
    }

    #[test]
    fn r609_refuses_a_version_it_cannot_read_rather_than_guessing_a_season() {
        for bad in ["core-1", "0.2.6", "v0", "", "v0.x.1"] {
            let refusal = refusal_of(bad);
            assert!(
                refusal.contains("patch version"),
                "{bad:?} was refused with {refusal:?}"
            );
        }
    }
}

mod r609_the_soft_reset {
    use super::*;

    fn players() -> Vec<ResetPlayer> {
        vec![
            ResetPlayer {
                profile_id: "c".to_string(),
                glicko: Glicko {
                    rating: 1400.0,
                    deviation: 60.0,
                    volatility: 0.059,
                },
            },
            ResetPlayer {
                profile_id: "a".to_string(),
                glicko: Glicko {
                    rating: 800.0,
                    deviation: 200.0,
                    volatility: 0.06,
                },
            },
            ResetPlayer {
                profile_id: "b".to_string(),
                glicko: Glicko {
                    rating: 1100.0,
                    deviation: 340.0,
                    volatility: 0.061,
                },
            },
        ]
    }

    #[test]
    fn r609_pulls_each_rating_season_reset_strength_of_the_way_to_the_players_mean_which_it_keeps() {
        assert_eq!(float(SEASON_RESET_STRENGTH), 0.5);
        let (changes, report) = reset(&players());
        assert!(close_to(report.mean, 1100.0, 9));
        assert_eq!(
            changes
                .iter()
                .map(|change| (change.profile_id.as_str(), change.after.rating))
                .collect::<Vec<_>>(),
            vec![("a", 950.0), ("b", 1100.0), ("c", 1250.0)]
        );
        assert!(close_to(
            report.spread_after,
            report.spread_before * (1.0 - float(SEASON_RESET_STRENGTH)),
            9
        ));
        assert_eq!(
            [
                report.lowest_before,
                report.highest_before,
                report.lowest_after,
                report.highest_after
            ],
            [800.0, 1400.0, 950.0, 1250.0]
        );
    }

    #[test]
    fn r609_widens_every_deviation_in_quadrature_never_past_a_new_players_and_keeps_volatility() {
        let (changes, report) = reset(&players());
        let by_id: IndexMap<&str, &Glicko> = changes
            .iter()
            .map(|change| (change.profile_id.as_str(), &change.after))
            .collect();
        let boost = float(SEASON_RESET_DEVIATION_BOOST);
        assert!(close_to(
            by_id["c"].deviation,
            (60.0f64 * 60.0 + boost * boost).sqrt(),
            9
        ));
        assert!(close_to(
            by_id["a"].deviation,
            (200.0f64 * 200.0 + boost * boost).sqrt(),
            9
        ));
        assert_eq!(by_id["b"].deviation, float(RATING_DEVIATION_START));
        for change in &changes {
            assert!(change.after.deviation >= change.before.deviation);
            assert_eq!(change.after.volatility, change.before.volatility);
        }
        assert!(report.deviation_after > report.deviation_before);
    }

    #[test]
    fn r609_gives_the_same_reset_whatever_order_the_players_arrive_in() {
        let mut reversed = players();
        reversed.reverse();
        assert_eq!(reset(&reversed), reset(&players()));
        let players = players();
        assert_eq!(reset_glicko(&players[0].glicko, 1400.0).rating, 1400.0);
    }

    #[test]
    fn r609_resets_nobody_when_nobody_has_played() {
        let (changes, report) = reset(&[]);
        assert!(changes.is_empty());
        assert_eq!(report.players, 0);
    }
}
