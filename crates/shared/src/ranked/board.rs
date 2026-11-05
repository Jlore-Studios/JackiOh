//! The leaderboard's sort key (SPEC §9.12, R608, R612; #3105): one integer on each rating record, so a
//! store that sorts integers (an OrderedDataStore on Roblox, #2898) lists the ladder as it is, with no
//! logic of its own. The server's leaderboard queries keep sorting the way they always did; the key is
//! derived from the same fields, and `tests/board.rs` and the server's leaderboard tests hold the two orders
//! together.
//!
//! The leaderboard lists Jlorious first, by hidden rating, then each Grape tier from Mythic down, by ladder
//! position, and states a tie by profile id (R1436). The key is that order, descending:
//!
//! | Record | Key |
//! | --- | --- |
//! | not on the board: unplaced (a Raisin, counted but not listed), or a hand-set rating on a strict server (R3074001) | `0` |
//! | Grape, not Jlorious | `SORT_KEY_GRAPE_BASE` + the ladder position |
//! | Jlorious (R608: a placed person in Mythic Grape, among the top `JLORIOUS_SIZE` by rating) | `SORT_KEY_JLORIOUS_BASE` + the rating in fixed point |
//!
//! The ladder position rises through the tiers, so the key's order across Grape tiers is the tier's, then the
//! position's. A tie, in the key or in the board, goes to the lower profile id. A Jlorious key resolves
//! ratings to `2^-SORT_KEY_RATING_BITS`; two ratings closer than that share a key, where the board tells them
//! apart, and that is the only place the two orders can part.
//!
//! Whether a record is Jlorious depends on the others' ratings, so a game that moves the Jlorious set also
//! moves the keys of the people it brings in or pushes out (`RatedDecision::records`).

use serde::{Deserialize, Serialize};

use super::ladder::{LADDER_TOP, SeasonRank, Standing};
use crate::config::{SORT_KEY_GRAPE_BASE, SORT_KEY_JLORIOUS_BASE, SORT_KEY_RATING_BITS};

/// One player's season row with their current hidden rating: what percentiles and Jlorious read.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStanding {
    #[serde(flatten)]
    pub rank: SeasonRank,
    pub rating: f64,
    /// R640000: a ladder bot's row; never serialized when false. Jlorious and the percentiles read
    /// the rows without it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bot: bool,
    /// R3074001: the player holds a hand-set rating on this ladder (migration 0080's
    /// `hand_set_ratings`); never serialized when false. On a strict server (`DEV_STRICT`) the public
    /// ladder, the percentiles and Jlorious read the rows without it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hand_set: bool,
}

/// A standing and the integer a leaderboard sorts it by (see this module).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RatingRecord {
    #[serde(flatten)]
    pub standing: SeasonStanding,
    /// Higher is higher on the board; `0` is off it. Under `2^51`.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub sort_key: i64,
}

/// Whether the public board lists a standing at all: a hand-set rating is left off a strict server's (R3074001).
pub fn listed(standing: &SeasonStanding, strict: bool) -> bool {
    !(strict && standing.hand_set)
}

/// A rating in fixed point, `SORT_KEY_RATING_BITS` fractional bits, under `2^50`.
fn fixed_rating(rating: f64) -> i64 {
    let scale = (1_i64 << SORT_KEY_RATING_BITS) as f64;
    let top = SORT_KEY_JLORIOUS_BASE - 1;
    // A rating below 0 (or NaN) is 0, and `as` saturates above.
    ((rating.max(0.0) * scale).floor() as i64).min(top)
}

/// The key `standing` sorts by: `jlorious` is whether R608 puts it in Jlorious, which the whole ladder
/// decides (`ladder::jlorious_order`); `strict` is the server's `DEV_STRICT` (R3074001).
pub fn sort_key(standing: &SeasonStanding, jlorious: bool, strict: bool) -> i64 {
    let Some(position) = standing.rank.ladder else {
        return 0;
    };
    if !listed(standing, strict) {
        return 0;
    }
    if jlorious {
        return SORT_KEY_JLORIOUS_BASE + fixed_rating(standing.rating);
    }
    SORT_KEY_GRAPE_BASE + position.clamp(0, LADDER_TOP) as i64
}

/// `standing` as a record: its key, given whether it is Jlorious.
pub fn record_of(standing: &SeasonStanding, jlorious: bool, strict: bool) -> RatingRecord {
    RatingRecord {
        sort_key: sort_key(standing, jlorious, strict),
        standing: standing.clone(),
    }
}

/// Whether a season row counts toward the percentiles, Jlorious and the public leaderboard. R640000:
/// a person's, never a ladder bot's. R3074001: on a strict server (`DEV_STRICT`), not a hand-set
/// rating's either, until it is reset to the one its games earned.
pub fn counts(standing: &SeasonStanding, strict: bool) -> bool {
    !standing.bot && !(strict && standing.hand_set)
}

/// The season as the ladder's percentiles and Jlorious read it: the rows that [`counts`] keeps.
/// R640000: people only, so a ladder bot never takes a Jlorious place from a person; R3074001: nor,
/// on a strict server, a hand-set rating.
pub fn standings_of(standings: &[SeasonStanding], strict: bool) -> Vec<Standing> {
    standings
        .iter()
        .filter(|standing| counts(standing, strict))
        .map(|standing| Standing {
            profile_id: standing.rank.profile_id.clone(),
            ladder: standing.rank.ladder,
            rating: standing.rating,
        })
        .collect()
}
