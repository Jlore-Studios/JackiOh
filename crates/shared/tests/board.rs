//! The leaderboard's sort key (`src/ranked/board.rs`, SPEC §9.12, R608, R612, R1436, R3074001; #3105): the
//! integer on each rating record orders a ladder as the public leaderboard lists it, so a store that sorts
//! integers lists the same players in the same order. The oracle below is the listing the server's
//! `leaderboard` makes (`crates/server/src/api/ranked.rs`), written out: Jlorious first by hidden rating,
//! then each Grape tier from Mythic down by ladder position, a tie by profile id. The server's own tests
//! hold the same order against its real leaderboard (`tests/api/ranked.rs`).

use jackioh_shared::config::{
    JLORIOUS_SIZE, SORT_KEY_GRAPE_BASE, SORT_KEY_JLORIOUS_BASE, SORT_KEY_RATING_BITS,
};
use jackioh_shared::ranked::board::{SeasonStanding, counts, record_of, sort_key, standings_of};
use jackioh_shared::ranked::ladder::{
    GRAPE_TIERS, LADDER_TOP, PIPS_PER_TIER, fresh_rank, jlorious_order, tier_bottom, tier_index_of,
};
use std::collections::BTreeSet;

fn standing(id: &str, ladder: Option<i32>, rating: f64, bot: bool, hand_set: bool) -> SeasonStanding {
    let mut rank = fresh_rank("v0.1", id, 0);
    rank.ladder = ladder;
    SeasonStanding {
        rank,
        rating,
        bot,
        hand_set,
    }
}

/// A small deterministic generator: the tests draw ladders and ratings from it.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// The ids the public leaderboard lists, in the order it lists them (R612): Jlorious #1 first, then each Grape
/// tier from Mythic down, a tier's players by ladder position and a tie by profile id (R1436). A player still
/// in their placements is counted, not listed, and on a strict server so is a hand-set rating (R3074001).
fn board_order(standings: &[SeasonStanding], strict: bool) -> Vec<String> {
    let jlorious = jlorious_order(&standings_of(standings, strict));
    let mut order: Vec<String> = jlorious.clone();
    for tier in (0..GRAPE_TIERS.len()).rev() {
        let mut rows: Vec<(i32, &str)> = standings
            .iter()
            .filter(|standing| !(strict && standing.hand_set))
            .filter(|standing| !jlorious.contains(&standing.rank.profile_id))
            .filter_map(|standing| {
                let ladder = standing.rank.ladder?;
                (tier_index_of(ladder) as usize == tier)
                    .then_some((ladder, standing.rank.profile_id.as_str()))
            })
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        order.extend(rows.into_iter().map(|(_, id)| id.to_string()));
    }
    order
}

/// The ids a store that sorts keys descending, and equal keys by profile id, lists; key 0 is off the board.
fn key_order(standings: &[SeasonStanding], strict: bool) -> Vec<String> {
    let jlorious: BTreeSet<String> = jlorious_order(&standings_of(standings, strict))
        .into_iter()
        .collect();
    let mut keyed: Vec<(i64, &str)> = standings
        .iter()
        .map(|standing| {
            let key = record_of(standing, jlorious.contains(&standing.rank.profile_id), strict).sort_key;
            (key, standing.rank.profile_id.as_str())
        })
        .filter(|(key, _)| *key > 0)
        .collect();
    keyed.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    keyed.into_iter().map(|(_, id)| id.to_string()).collect()
}

/// A ladder of `count` players: most placed anywhere, a good share in Mythic Grape, some with equal ratings
/// and equal positions (ties), some bots, some hand-set, some still in their placements.
fn ladder_of(seed: u64, count: usize) -> Vec<SeasonStanding> {
    let mut draw = Draw(seed);
    (0..count)
        .map(|index| {
            let id = format!("p{:04}", draw.below(100_000) * 1_000 + index as u64);
            let mythic = draw.below(3) == 0;
            let ladder = match draw.below(8) {
                0 => None,
                _ if mythic => Some(tier_bottom(4) + draw.below(PIPS_PER_TIER as u64) as i32),
                _ => Some(draw.below((LADDER_TOP + 1) as u64) as i32),
            };
            // Ratings on a coarse grid, so some are exactly equal: the board breaks that tie by id.
            let rating = 800.0 + draw.below(1_600) as f64 * 0.25;
            standing(&id, ladder, rating, draw.below(9) == 0, draw.below(11) == 0)
        })
        .collect()
}

#[test]
fn r612_the_sort_keys_order_is_the_leaderboards_on_a_ladder_of_hundreds() {
    for seed in 1..=12_u64 {
        for strict in [false, true] {
            // 400 players, a third of them in Mythic Grape: past `JLORIOUS_SIZE`, so Jlorious is a cut.
            let standings = ladder_of(seed * 7_919, 400);
            let board = board_order(&standings, strict);
            assert!(
                standings_of(&standings, strict)
                    .iter()
                    .filter(|standing| standing.ladder.is_some_and(|ladder| tier_index_of(ladder) == 4))
                    .count()
                    > JLORIOUS_SIZE,
                "the ladder must hold more Mythic players than Jlorious does"
            );
            assert_eq!(
                key_order(&standings, strict),
                board,
                "seed {seed}, strict {strict}"
            );
        }
    }
}

#[test]
fn r612_the_sort_keys_order_is_the_leaderboards_on_small_ladders_and_ties() {
    for seed in 1..=60_u64 {
        let standings = ladder_of(seed, 2 + (seed % 17) as usize);
        for strict in [false, true] {
            assert_eq!(
                key_order(&standings, strict),
                board_order(&standings, strict),
                "seed {seed}, strict {strict}"
            );
        }
    }
    // Equal ratings and equal positions are told apart by profile id, in Jlorious and out of it.
    let tied = vec![
        standing("b", Some(tier_bottom(4)), 1500.0, false, false),
        standing("a", Some(tier_bottom(4)), 1500.0, false, false),
        standing("d", Some(tier_bottom(2)), 1200.0, false, false),
        standing("c", Some(tier_bottom(2)), 900.0, false, false),
    ];
    assert_eq!(key_order(&tied, false), ["a", "b", "c", "d"]);
    assert_eq!(board_order(&tied, false), ["a", "b", "c", "d"]);
}

#[test]
fn r608_a_bot_is_listed_in_its_tier_and_never_in_jlorious() {
    let standings = vec![
        standing("bot", Some(tier_bottom(4) + 8), 3000.0, true, false),
        standing("person", Some(tier_bottom(4)), 1000.0, false, false),
    ];
    let jlorious = jlorious_order(&standings_of(&standings, false));
    assert_eq!(jlorious, ["person"]);
    let bot = record_of(&standings[0], false, false);
    let person = record_of(&standings[1], true, false);
    assert_eq!(bot.sort_key, SORT_KEY_GRAPE_BASE + i64::from(tier_bottom(4) + 8));
    assert!(
        person.sort_key > bot.sort_key,
        "Jlorious is above every Grape position"
    );
    assert!(!counts(&standings[0], false), "a bot counts toward no percentile");
}

#[test]
fn r3074001_a_hand_set_rating_is_off_a_strict_servers_board_and_on_a_lax_ones() {
    let hand_set = standing("h", Some(tier_bottom(3)), 2900.0, false, true);
    assert_eq!(sort_key(&hand_set, false, true), 0);
    assert_eq!(sort_key(&hand_set, true, true), 0);
    assert!(sort_key(&hand_set, false, false) > 0);
    // And a player still in their placements is counted, not listed.
    assert_eq!(
        sort_key(&standing("r", None, 1000.0, false, false), false, false),
        0
    );
}

#[test]
fn r608_a_key_fits_53_bits_rises_with_the_rating_and_the_ladder_and_clamps_the_extremes() {
    // The key is exact in a Luau number: under 2^53.
    let top = standing("t", Some(tier_bottom(4)), f64::MAX, false, false);
    assert!(sort_key(&top, true, false) < 1 << 51);
    let bottom = standing("t", Some(tier_bottom(4)), f64::NEG_INFINITY, false, false);
    assert_eq!(sort_key(&bottom, true, false), SORT_KEY_JLORIOUS_BASE);
    let not_a_number = standing("t", Some(tier_bottom(4)), f64::NAN, false, false);
    assert_eq!(sort_key(&not_a_number, true, false), SORT_KEY_JLORIOUS_BASE);
    // Every Grape key is under every Jlorious key, and rises with the ladder.
    let grape: Vec<i64> = (0..=LADDER_TOP)
        .map(|ladder| sort_key(&standing("g", Some(ladder), 5000.0, false, false), false, false))
        .collect();
    assert!(grape.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(grape[0], SORT_KEY_GRAPE_BASE);
    assert!(grape[grape.len() - 1] < SORT_KEY_JLORIOUS_BASE);
    // Ratings further apart than the key's resolution are told apart, in order.
    let resolution = 1.0 / (1_u64 << SORT_KEY_RATING_BITS) as f64;
    let low = standing("l", Some(tier_bottom(4)), 1500.0, false, false);
    let high = standing("h", Some(tier_bottom(4)), 1500.0 + 4.0 * resolution, false, false);
    assert!(sort_key(&low, true, false) < sort_key(&high, true, false));
    // The key of a standing without a position is not on the board, whatever Jlorious would say.
    assert_eq!(
        sort_key(&standing("u", None, 1500.0, false, false), true, false),
        0
    );
}

#[test]
fn r608_a_record_serializes_as_its_standing_with_the_key_beside_it() {
    let record = record_of(&standing("p", Some(7), 1234.5, false, false), false, false);
    let json = serde_json::to_value(&record).expect("a record serializes");
    assert_eq!(json["profileId"], "p");
    assert_eq!(json["rating"], 1234.5);
    assert_eq!(json["sortKey"], SORT_KEY_GRAPE_BASE + 7);
    assert!(json.get("bot").is_none() && json.get("handSet").is_none());
    let back: jackioh_shared::ranked::board::RatingRecord =
        serde_json::from_value(json).expect("a record reads");
    assert_eq!(back, record);
}
