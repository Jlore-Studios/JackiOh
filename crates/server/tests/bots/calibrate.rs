//! R748000 (SPEC §9.12, R1454, R610, R732001): the ladder bots' seed ratings, measured. An even
//! score is no offset and a sweep stays finite; the Bradley-Terry fit recovers the offsets it is
//! given; a seed is the anchor plus its offset; an AI promotion moves the anchor by its `vsParent`;
//! the reference's live rating is read off its games against people; a calibration game is a function
//! of its seed; and `bot-calibrate`'s flags and its report are what the docs say.
//!
//! `jackioh_server::bots::calibrate` and `cli::bot_calibrate`, on the real cards.

use std::path::PathBuf;

use serde_json::{Value, json};

use jackioh_ai::SearchBudget;
use jackioh_engine::Winner;
use jackioh_server::bots::calibrate::{
    PairTally, Promotion, StrengthOffset, elo_of_score, fit_offsets, pairings, parse_vs_parent,
    performance_rating, play_game, promotion_shift, reference_observations, seed_ratings,
};
use jackioh_server::cli::bot_calibrate::{
    CalibrateOptions, CalibrationReport, parse_calibrate_args, parse_live_args, promotions_of,
    render_calibration,
};
use jackioh_server::config::{
    BOT_CALIBRATION_GAMES_PER_PAIR, BOT_SEED_RATINGS, BOT_STRENGTHS, BotStrength, GLICKO_SCALE,
};
use jackioh_server::db::store::{BotRow, RatedGameRow};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn r748000_an_even_score_is_no_offset_and_a_sweep_stays_finite() {
    assert_eq!(elo_of_score(20.0, 40.0), 0.0);
    let sweep = elo_of_score(40.0, 40.0);
    assert!(sweep.is_finite());
    assert!((sweep - GLICKO_SCALE * 81.0_f64.ln()).abs() < 1e-9, "{sweep}");
    assert!((elo_of_score(0.0, 40.0) + sweep).abs() < 1e-9);
}

#[test]
fn r748000_one_pair_fits_to_its_score() {
    let tally = PairTally {
        a: 0,
        b: 1,
        a_wins: 12,
        b_wins: 26,
        draws: 2,
        unfinished: 3,
    };
    let fit = fit_offsets(&[tally], 2, 1);
    assert!(
        (fit[0].offset - elo_of_score(13.0, 40.0)).abs() < 1e-6,
        "{:?}",
        fit[0]
    );
    assert_eq!(fit[1].offset, 0.0);
    assert_eq!(fit[1].error, 0.0, "the reference has no error against itself");
    assert!(fit[0].error > 0.0);
}

#[test]
fn r748000_the_fit_recovers_known_offsets() {
    let truth = [-400.0, -200.0, 0.0];
    let games = 10_000_usize;
    let tallies: Vec<PairTally> = pairings(truth.len())
        .into_iter()
        .map(|(a, b)| {
            let expected = 1.0 / (1.0 + ((truth[b] - truth[a]) / GLICKO_SCALE).exp());
            let a_wins = (expected * games as f64).round() as usize;
            PairTally {
                a,
                b,
                a_wins,
                b_wins: games - a_wins,
                draws: 0,
                unfinished: 0,
            }
        })
        .collect();
    let fit = fit_offsets(&tallies, truth.len(), 2);
    for (found, truth) in fit.iter().zip(truth) {
        assert!((found.offset - truth).abs() < 1.0, "{found:?} against {truth}");
    }
}

#[test]
fn r748000_seeds_are_the_anchor_plus_each_offset() {
    let offsets = [
        StrengthOffset {
            strength: 0,
            offset: -412.6,
            error: 20.0,
        },
        StrengthOffset {
            strength: 1,
            offset: 0.0,
            error: 0.0,
        },
    ];
    assert_eq!(seed_ratings(1200.0, &offsets), vec![787.0, 1200.0]);
}

#[test]
fn r748000_a_promotion_shifts_the_anchor_by_its_vs_parent() {
    assert_eq!(parse_vs_parent("65/100"), Some((65, 100)));
    assert_eq!(parse_vs_parent("x"), None);
    assert_eq!(parse_vs_parent("5/0"), None);
    assert_eq!(parse_vs_parent("7/5"), None);

    let promotions = [
        Promotion {
            generation: 9,
            wins: 65,
            games: 100,
        },
        Promotion {
            generation: 10,
            wins: 70,
            games: 100,
        },
        Promotion {
            generation: 11,
            wins: 50,
            games: 100,
        },
    ];
    let shift = promotion_shift(&promotions, 9, 11);
    assert_eq!(shift, elo_of_score(70.0, 100.0) + elo_of_score(50.0, 100.0));
    assert_eq!(promotion_shift(&promotions, 11, 11), 0.0, "none after the newest");

    let text = "{\"generation\":9,\"vsParent\":\"65/100\"}\n\n{\"generation\":1,\"vsParent\":null}\n";
    assert_eq!(promotions_of(text).expect("a history"), vec![promotions[0]]);
    assert!(promotions_of("not json").is_err());
}

#[test]
fn r748000_the_performance_rating_matches_its_score() {
    assert_eq!(performance_rating(&[]), None);
    let games = vec![(1000.0, 0.0); 9]
        .into_iter()
        .chain(vec![(1000.0, 1.0); 21])
        .collect::<Vec<_>>();
    let (rating, error) = performance_rating(&games).expect("a rating");
    assert!(
        (rating - (1000.0 + elo_of_score(21.0, 30.0))).abs() < 1e-6,
        "{rating}"
    );
    assert!(error > 0.0);
}

fn side(profile: Option<&str>, bot: Option<&str>, rating: f64) -> Value {
    let glicko = json!({ "rating": rating, "deviation": 100.0, "volatility": 0.06 });
    json!({
        "profileId": profile,
        "botId": bot,
        "pilot": if bot.is_some() { "ai" } else { "human" },
        "before": glicko,
        "after": glicko,
        "rankBefore": null,
        "rankAfter": null,
    })
}

fn game(sides: [Value; 2], winner: Option<usize>) -> RatedGameRow {
    serde_json::from_value(json!({
        "id": "game",
        "kind": "match",
        "seasonId": "v0.1",
        "patchVersion": "v0.1.1",
        "catalogVersion": "0.1.1",
        "sides": sides,
        "winnerSide": winner,
        "reason": "hero-death",
        "endedAt": 1,
    }))
    .expect("a rated game")
}

fn bot(id: &str, strength: usize) -> BotRow {
    serde_json::from_value(json!({
        "profileId": id,
        "strength": strength,
        "speed": 1.0,
        "persona": "balanced",
        "activeFromHour": 0,
        "activeHours": 24,
        "decks": [],
        "decksCatalogVersion": null,
        "createdAt": 0,
        "retiredAt": null,
        "rating": 1000.0,
        "inMatchId": null,
    }))
    .expect("a bot")
}

#[test]
fn r748000_the_live_anchor_reads_the_reference_s_games_against_people() {
    let bots = [bot("bot-ref", 4), bot("bot-other-ref", 4), bot("bot-weak", 1)];
    let games = [
        // The reference on side 1 beats a person rated 1100.
        game(
            [side(Some("p"), None, 1100.0), side(None, Some("bot-ref"), 1200.0)],
            Some(1),
        ),
        // A draw with a person rated 1250, the reference on side 0.
        game(
            [
                side(None, Some("bot-other-ref"), 1200.0),
                side(Some("p"), None, 1250.0),
            ],
            None,
        ),
        // A loss to a person rated 1300.
        game(
            [side(None, Some("bot-ref"), 1200.0), side(Some("p"), None, 1300.0)],
            Some(1),
        ),
        // A weaker bot's game, a bot id that names no bot, and two bots.
        game(
            [side(Some("p"), None, 1000.0), side(None, Some("bot-weak"), 800.0)],
            Some(0),
        ),
        game(
            [side(Some("p"), None, 1000.0), side(None, Some("bot-gone"), 800.0)],
            Some(1),
        ),
        game(
            [
                side(None, Some("bot-ref"), 1200.0),
                side(None, Some("bot-weak"), 800.0),
            ],
            Some(0),
        ),
    ];
    assert_eq!(
        reference_observations(&games, &bots, 4),
        vec![(1100.0, 1.0), (1250.0, 0.5), (1300.0, 0.0)]
    );
}

#[test]
fn r748000_a_calibration_game_is_a_function_of_its_seed() {
    let strength = BotStrength {
        budget: SearchBudget {
            nodes: 20,
            lethal_nodes: 10,
            determinizations: 1,
            beam_width: 1,
            root_branching: 4,
            branching: 2,
            max_depth: 2,
            finalists: 1,
        },
        temperature: 1.0,
        missed_lethal_chance: 0.5,
        seed_rating: 1000.0,
    };
    let first: Option<Winner> = play_game("r748000-test", &strength, &strength);
    let second = play_game("r748000-test", &strength, &strength);
    assert_eq!(first, second);
    assert!(first.is_some(), "the game reached a result");
}

#[test]
fn r748000_the_strengths_seed_from_bot_seed_ratings() {
    assert_eq!(BOT_STRENGTHS.len(), BOT_SEED_RATINGS.len());
    for (index, strength) in BOT_STRENGTHS.iter().enumerate() {
        assert_eq!(strength.seed_rating, BOT_SEED_RATINGS[index], "strength {index}");
    }
}

#[test]
fn r748000_bot_calibrate_parses_its_flags() {
    assert_eq!(
        parse_calibrate_args(&[]).expect("the defaults"),
        CalibrateOptions {
            games: BOT_CALIBRATION_GAMES_PER_PAIR,
            seed: None,
            anchor: None,
            history: PathBuf::from("training/history"),
            threads: None,
        }
    );
    assert_eq!(
        parse_calibrate_args(&args(&[
            "--games=50",
            "--seed=abc",
            "--anchor=1234.5",
            "--history=some/dir",
            "--threads=3",
        ]))
        .expect("every flag"),
        CalibrateOptions {
            games: 50,
            seed: Some("abc".to_string()),
            anchor: Some(1234.5),
            history: PathBuf::from("some/dir"),
            threads: Some(3),
        }
    );
    let few = parse_calibrate_args(&args(&["--games=29"])).expect_err("too few games");
    assert!(few.to_string().contains("30"), "{few}");
    assert!(parse_calibrate_args(&args(&["--anchor=abc"])).is_err());
    assert!(parse_calibrate_args(&args(&["--bogus=1"])).is_err());

    assert_eq!(parse_live_args(&[]).expect("the default"), 0);
    assert_eq!(parse_live_args(&args(&["--since=123"])).expect("a since"), 123);
    assert!(parse_live_args(&args(&["--games=40"])).is_err());
}

#[test]
fn r748000_the_report_prints_the_rows_to_paste() {
    let report = CalibrationReport {
        generation: 9,
        seed_base: "bot-calibration:gen9".to_string(),
        games_per_pair: 40,
        anchor: 1200.0,
        anchor_source: "from --anchor".to_string(),
        tallies: vec![PairTally {
            a: 0,
            b: 1,
            a_wins: 5,
            b_wins: 33,
            draws: 2,
            unfinished: 0,
        }],
        offsets: vec![
            StrengthOffset {
                strength: 0,
                offset: -412.6,
                error: 20.0,
            },
            StrengthOffset {
                strength: 1,
                offset: 0.0,
                error: 0.0,
            },
        ],
        seeds: vec![787.0, 1200.0],
    };
    let text = render_calibration(&report);
    assert!(
        text.contains("pub const BOT_SEED_RATINGS: [f64; 2] = [787.0, 1200.0];"),
        "{text}"
    );
    assert!(
        text.contains("pub const BOT_CALIBRATION_GENERATION: i64 = 9;"),
        "{text}"
    );
    assert!(text.contains("| 0 v 1 | 5 | 2 | 33 | 0 |"), "{text}");
}
