//! One rated game decided (`src/ranked/game.rs`, SPEC §9.12, R603–R611, R640000, R672, R1560000, R1843004,
//! R3074001, R3290000; #3105), and the JSON boundary a host makes the same calls through (`RankedCall`, `RankedAnswer`).
//! The rating maths itself is held to Glickman's worked example in `tests/glicko2.rs`; these tests hold
//! `decide` to the functions it is made of, to the bit.

use jackioh_engine::GameOverReason;
use jackioh_shared::config::{JLORIOUS_SIZE, RATING_DEVIATION_START, STAKE_DOUBLE};
use jackioh_shared::ranked::board::{SeasonStanding, record_of, standings_of};
use jackioh_shared::ranked::game::{
    PeakWrite, RankedSideInput, RateGameInput, RateSide, RatedReason, decide, promotions_since, result_of,
};
use jackioh_shared::ranked::glicko2::{Glicko, START_GLICKO, rate_game_scored, widen_for_promotions};
use jackioh_shared::ranked::ladder::{
    GameResult, Ladder, SeasonRank, VisibleRank, fresh_rank, jlorious_order, tier_bottom,
};
use jackioh_shared::ranked::season::{
    ResetPlayer, Season, SeasonPlan, plan_season, season_id_of, soft_reset,
};
use jackioh_shared::ranked::{RankedAnswer, RankedCall, apply};
use jackioh_shared::series::SeriesEnd;
use serde_json::{Value, json};

const AT: i64 = 1_700_000_000_000;

fn season() -> Season {
    Season {
        id: "v0.1".to_owned(),
        patch_version: "v0.1.0".to_owned(),
        started_at: AT - 1_000,
    }
}

fn glicko(rating: f64, deviation: f64) -> Glicko {
    Glicko {
        rating,
        deviation,
        volatility: 0.06,
    }
}

fn player(id: &str, glicko: Glicko) -> RateSide {
    RateSide {
        side: RankedSideInput::Player {
            profile_id: id.to_owned(),
        },
        glicko,
        bot_row: false,
        rated_on: None,
        bot_profile: false,
    }
}

fn standing(id: &str, ladder: Option<i32>, rating: f64) -> SeasonStanding {
    let mut rank = fresh_rank("v0.1", id, 0);
    rank.ladder = ladder;
    rank.games = if ladder.is_some() { 5 } else { 1 };
    SeasonStanding {
        rank,
        rating,
        bot: false,
        hand_set: false,
        banned: false,
    }
}

fn game(
    standings: Vec<SeasonStanding>,
    sides: (RateSide, RateSide),
    winner_side: Option<i32>,
) -> RateGameInput {
    RateGameInput {
        standings,
        sides,
        winner_side,
        reason: RatedReason::Game(GameOverReason::HeroDeath),
        stake: None,
        season: season(),
        ladder: Ladder::Constructed,
        ai_generation: None,
        strict: false,
        now_ms: AT,
    }
}

#[test]
fn r603_a_game_rates_each_side_against_the_others_rating_from_before_it_to_the_bit() {
    let (a, b) = (glicko(1500.0, 200.0), glicko(1400.0, 30.0));
    let input = game(
        vec![standing("a", Some(10), 1500.0), standing("b", Some(12), 1400.0)],
        (player("a", a), player("b", b)),
        Some(0),
    );
    let decision = decide(&input);
    let rated = rate_game_scored(&a, &b, 1.0, 0.0);
    // Exactly the update the functions make, not close to it: ratings are stored.
    assert_eq!(decision.sides.0.after, rated.a);
    assert_eq!(decision.sides.1.after, rated.b);
    assert_eq!((decision.sides.0.before, decision.sides.1.before), (a, b));
    // Both season rows moved, in side order, and the records carry the new ratings.
    assert_eq!(decision.ranks.len(), 2);
    assert_eq!(decision.ranks[0].profile_id, "a");
    assert_eq!(decision.ranks[0].wins, 1);
    assert_eq!(decision.ranks[1].losses, 1);
    assert_eq!(decision.records[0].standing.rating, rated.a.rating);
    assert_eq!(decision.records[1].standing.rating, rated.b.rating);
}

#[test]
fn r672_a_double_stake_moves_the_rating_twice_and_keeps_the_single_updates_deviation_and_volatility() {
    let (a, b) = (glicko(1500.0, 200.0), glicko(1400.0, 100.0));
    let mut input = game(
        vec![standing("a", Some(10), 1500.0), standing("b", Some(12), 1400.0)],
        (player("a", a), player("b", b)),
        Some(0),
    );
    let single = decide(&input);
    input.stake = Some(STAKE_DOUBLE);
    let double = decide(&input);
    let rated = rate_game_scored(&a, &b, 1.0, 0.0);
    assert_eq!(
        double.sides.0.after.rating,
        a.rating + 2.0 * (rated.a.rating - a.rating)
    );
    assert_eq!(
        double.sides.1.after.rating,
        b.rating + 2.0 * (rated.b.rating - b.rating)
    );
    assert_eq!(double.sides.0.after.deviation, single.sides.0.after.deviation);
    assert_eq!(double.sides.0.after.volatility, single.sides.0.after.volatility);
}

#[test]
fn r1843004_a_fled_game_is_the_fleers_win_and_the_others_draw_and_a_series_never_is() {
    let fled = RatedReason::Game(GameOverReason::Fled);
    assert_eq!(result_of(Some(0), &fled, 0), GameResult::Win);
    assert_eq!(result_of(Some(0), &fled, 1), GameResult::Draw);
    let death = RatedReason::Game(GameOverReason::HeroDeath);
    assert_eq!(result_of(Some(1), &death, 0), GameResult::Loss);
    assert_eq!(
        result_of(Some(1), &RatedReason::Series(SeriesEnd::Decided), 0),
        GameResult::Loss
    );
    assert_eq!(result_of(None, &death, 0), GameResult::Draw);
    assert_eq!(result_of(None, &fled, 1), GameResult::Draw);
    // The draw is the score a fled game gives the other side: Glicko-2 reads 0.5 for it.
    let (a, b) = (glicko(1500.0, 200.0), glicko(1500.0, 200.0));
    let mut input = game(vec![], (player("a", a), player("b", b)), Some(0));
    input.reason = fled;
    let decision = decide(&input);
    let rated = rate_game_scored(&a, &b, 1.0, 0.5);
    assert_eq!(
        (decision.sides.0.after, decision.sides.1.after),
        (rated.a, rated.b)
    );
}

#[test]
fn r1560000_a_bots_deviation_is_widened_by_the_promotions_since_its_row_was_rated() {
    assert_eq!(promotions_since(None, Some(3)), 0);
    assert_eq!(promotions_since(Some(14), None), 1);
    assert_eq!(promotions_since(Some(14), Some(11)), 3);
    let bot = RateSide {
        side: RankedSideInput::Bot {
            bot_id: "bot".to_owned(),
        },
        glicko: glicko(1300.0, 60.0),
        bot_row: true,
        rated_on: Some(11),
        bot_profile: true,
    };
    let mut input = game(vec![], (player("a", START_GLICKO), bot.clone()), Some(0));
    input.ai_generation = Some(14);
    let widened = widen_for_promotions(&bot.glicko, 3);
    assert!(widened.deviation > bot.glicko.deviation && widened.deviation <= RATING_DEVIATION_START);
    let decision = decide(&input);
    assert_eq!(decision.sides.1.before, widened);
    assert_eq!(
        decision.sides.1.after,
        rate_game_scored(&START_GLICKO, &widened, 1.0, 0.0).b
    );
    // A bot with no row is not widened, whatever the build's generation.
    let fresh = RateSide {
        bot_row: false,
        glicko: START_GLICKO,
        ..bot
    };
    input.sides.1 = fresh;
    assert_eq!(decide(&input).sides.1.before, START_GLICKO);
}

#[test]
fn r640000_a_bot_with_a_profile_holds_a_row_and_never_a_jlorious_place_and_a_bare_rating_holds_none() {
    let bot = |profile: bool| RateSide {
        side: RankedSideInput::Bot {
            bot_id: "bot".to_owned(),
        },
        glicko: glicko(2400.0, 50.0),
        bot_row: true,
        rated_on: None,
        bot_profile: profile,
    };
    let mut standings = vec![
        standing("bot", Some(tier_bottom(4)), 2400.0),
        standing("a", Some(tier_bottom(4)), 1500.0),
    ];
    standings[0].bot = true;
    let held = decide(&game(
        standings.clone(),
        (player("a", glicko(1500.0, 50.0)), bot(true)),
        Some(1),
    ));
    assert_eq!(held.ranks.len(), 2);
    assert_eq!(
        held.sides.1.rank_after,
        Some(VisibleRank::Grape {
            tier: jackioh_shared::ranked::ladder::GrapeTier::Mythic,
            division: 3,
            pips: 1,
            pips_per_division: 3,
            floor: jackioh_shared::ranked::ladder::GrapeTier::Mythic,
        })
    );
    let bare = decide(&game(
        standings,
        (player("a", glicko(1500.0, 50.0)), bot(false)),
        Some(1),
    ));
    assert_eq!(bare.ranks.len(), 1);
    assert_eq!((bare.sides.1.rank_before, bare.sides.1.rank_after), (None, None));
    // Its rating is still rated: a bare rating moves like any other.
    assert_ne!(bare.sides.1.after, bare.sides.1.before);
}

/// R3290000: a ban that lands while a game is running leaves the game rated, and its banned side's row moves
/// like any other's, but the banned side takes no Jlorious place and its record is off the board.
#[test]
fn r3290000_a_banned_accounts_running_game_is_rated_but_takes_no_jlorious_place() {
    let mut standings = vec![
        standing("banned", Some(tier_bottom(4) + 8), 2500.0),
        standing("a", Some(tier_bottom(4) + 1), 1500.0),
    ];
    standings[0].banned = true;
    let decision = decide(&game(
        standings,
        (
            player("a", glicko(1500.0, 50.0)),
            player("banned", glicko(2500.0, 50.0)),
        ),
        Some(0),
    ));
    assert_eq!(decision.ranks.len(), 2);
    let banned = decision
        .ranks
        .iter()
        .find(|rank| rank.profile_id == "banned")
        .expect("the banned side's row is written");
    assert!(
        banned.ladder.is_some_and(|ladder| ladder >= tier_bottom(4)),
        "the row stays in Mythic Grape, so only the ban keeps it out of Jlorious"
    );
    assert_eq!(banned.peak_jlorious, None);
    assert_eq!(decision.jlorious, ["a"]);
    assert!(decision.peaks.is_empty());
    // The banned side is rated as ever, and never reads a Jlorious place.
    assert_ne!(decision.sides.1.after, decision.sides.1.before);
    assert!(!matches!(
        decision.sides.1.rank_after,
        Some(VisibleRank::Jlorious { .. })
    ));
    let record = decision
        .records
        .iter()
        .find(|record| record.standing.rank.profile_id == "banned")
        .expect("the banned side's record is written");
    assert!(record.standing.banned);
    assert_eq!(record.sort_key, 0);
}

/// 100 Jlorious places held by people at 2000 and over; a 101st Mythic player at 1500 beats one of them and
/// takes its place: the game brings them in and pushes the one with the lowest rating out, and the records
/// carry the keys that moved.
#[test]
fn r608_a_game_that_moves_jlorious_writes_the_record_of_each_player_it_brings_in_or_pushes_out() {
    let mythic = tier_bottom(4);
    let mut standings: Vec<SeasonStanding> = (0..JLORIOUS_SIZE)
        .map(|index| standing(&format!("j{index:03}"), Some(mythic + 4), 2000.0 + index as f64))
        .collect();
    standings.push(standing("climber", Some(mythic), 1999.5));
    let lowest = "j000"; // 2000.0: the Jlorious player with the lowest rating, before the game
    let before = jlorious_order(&standings_of(&standings, false));
    assert_eq!(before.len(), JLORIOUS_SIZE);
    assert!(!before.contains(&"climber".to_owned()) && before.contains(&lowest.to_owned()));
    // The climber (1999.5) beats a high one by a lot: their rating passes j000's 2000.
    let high = "j099";
    let input = game(
        standings,
        (
            player("climber", glicko(1999.5, 60.0)),
            player(high, glicko(2099.0, 60.0)),
        ),
        Some(0),
    );
    let decision = decide(&input);
    let climber = &decision.records[0];
    assert_eq!(climber.standing.rank.profile_id, "climber");
    assert!(climber.standing.rating > 2000.0);
    assert!(
        climber.sort_key >= jackioh_shared::config::SORT_KEY_JLORIOUS_BASE,
        "now Jlorious"
    );
    // The one pushed out is a bystander whose key fell to a Grape key.
    let pushed = decision
        .records
        .iter()
        .find(|record| record.standing.rank.profile_id == lowest)
        .expect("the player pushed out is written");
    assert!(pushed.sort_key < jackioh_shared::config::SORT_KEY_JLORIOUS_BASE && pushed.sort_key > 0);
    // Nobody else's key moved, so nobody else is written: the sides, and the one pushed out.
    assert_eq!(decision.records.len(), 3);
    // The climber's new place is a peak to write for the climber itself, and a bystander's peak is a PeakWrite.
    assert!(
        decision
            .ranks
            .iter()
            .any(|rank| rank.profile_id == "climber" && rank.peak_jlorious.is_some())
    );
    assert!(decision.jlorious.contains(&"climber".to_owned()));
    let _: &[PeakWrite] = &decision.peaks;
    // Each record is its standing and the key `record_of` gives it.
    for record in &decision.records {
        let jlorious = record.sort_key >= jackioh_shared::config::SORT_KEY_JLORIOUS_BASE;
        assert_eq!(*record, record_of(&record.standing, jlorious, false));
    }
}

#[test]
fn r609_the_build_plays_in_one_season_which_the_first_opens_without_a_reset() {
    let none: Vec<Season> = Vec::new();
    let SeasonPlan::New { season, first } = plan_season(&none, "v0.3.1", AT) else {
        panic!("a build with no season opens one");
    };
    assert!(first);
    assert_eq!(
        (
            season.id.as_str(),
            season.patch_version.as_str(),
            season.started_at
        ),
        ("v0.3", "v0.3.1", AT)
    );
    // The next minor version opens another, now resetting.
    let SeasonPlan::New { first, .. } = plan_season(std::slice::from_ref(&season), "v0.4.0", AT + 1) else {
        panic!("a new minor version opens a season");
    };
    assert!(!first);
    // A patch of the same minor version plays in the season that exists, started when it was.
    assert_eq!(
        plan_season(std::slice::from_ref(&season), "v0.3.9", AT + 2),
        SeasonPlan::Exists { season }
    );
    assert_eq!(season_id_of("v0.3.9"), "v0.3");
}

/// A call as a host sends it and the answer as it reads it: both are JSON, nothing else crosses.
fn call(request: Value) -> Value {
    let request: RankedCall = serde_json::from_value(request).expect("a RankedCall");
    serde_json::to_value(apply(&request)).expect("a RankedAnswer")
}

#[test]
fn r603_the_json_calls_answer_what_the_functions_answer() {
    let (a, b) = (glicko(1500.0, 200.0), glicko(1400.0, 30.0));
    let input = game(
        vec![standing("a", Some(10), 1500.0), standing("b", Some(12), 1400.0)],
        (player("a", a), player("b", b)),
        Some(0),
    );
    let answer = call(json!({ "call": "rateGame", "game": input }));
    assert_eq!(answer, json!({ "game": decide(&input) }));
    // A game as a host writes it: only what it knows.
    let sparse = call(json!({
        "call": "rateGame",
        "game": {
            "standings": [],
            "sides": [
                { "side": { "kind": "player", "profileId": "a" }, "glicko": { "rating": 1000.0, "deviation": 350.0, "volatility": 0.06 } },
                { "side": { "kind": "bot", "botId": "b" }, "botProfile": false,
                  "glicko": { "rating": 1000.0, "deviation": 350.0, "volatility": 0.06 } },
            ],
            "reason": "hero-death", "season": season(), "ladder": "constructed", "nowMs": AT, "winnerSide": 1,
        },
    }));
    assert_eq!(sparse["game"]["ranks"].as_array().map(Vec::len), Some(1));
    assert_eq!(sparse["game"]["sides"][0]["rankBefore"]["tier"], "raisin");

    let seasons = vec![season()];
    assert_eq!(
        call(json!({ "call": "planSeason", "patchVersion": "v0.1.7", "seasons": seasons, "nowMs": AT })),
        json!({ "season": plan_season(&seasons, "v0.1.7", AT) })
    );
    let players = vec![
        ResetPlayer {
            profile_id: "x".into(),
            glicko: glicko(1800.0, 60.0),
        },
        ResetPlayer {
            profile_id: "y".into(),
            glicko: glicko(1200.0, 80.0),
        },
    ];
    assert_eq!(
        call(json!({ "call": "softReset", "players": players })),
        json!({ "reset": soft_reset(&players) })
    );
    let rank: SeasonRank = SeasonRank {
        ladder: Some(tier_bottom(2) + 4),
        peak_ladder: Some(tier_bottom(2) + 4),
        ..fresh_rank("v0.1", "x", 0)
    };
    assert_eq!(
        call(json!({ "call": "visibleRank", "rank": rank, "jloriousPosition": null })),
        json!({ "rank": serde_json::to_value(jackioh_shared::ranked::ladder::visible_rank(Some(&rank), None)).unwrap() })
    );
    assert_eq!(call(json!({ "call": "visibleRank" }))["rank"]["tier"], "raisin");
    assert_eq!(
        call(json!({ "call": "peakBadge", "rank": rank }))["badge"]["tier"],
        "large"
    );
    assert_eq!(
        call(json!({ "call": "jloriousOrder", "standings": [
            { "profileId": "m", "ladder": tier_bottom(4), "rating": 1500.0 },
            { "profileId": "g", "ladder": tier_bottom(3), "rating": 3000.0 },
        ] })),
        json!({ "jlorious": ["m"] })
    );
    let _: RankedAnswer = serde_json::from_value(call(json!({ "call": "softReset", "players": [] })))
        .expect("an answer reads back");
}

#[test]
fn r603_a_call_that_does_not_parse_is_an_error() {
    for request in [
        json!({ "call": "nope" }),
        json!({ "call": "rateGame" }),
        json!({ "call": "planSeason", "patchVersion": "v0.1", "nowMs": 1 }),
        json!({ "standings": [] }),
    ] {
        assert!(
            serde_json::from_value::<RankedCall>(request.clone()).is_err(),
            "{request}"
        );
    }
}
