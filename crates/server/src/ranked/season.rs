//! Seasons (SPEC §9.12, R609): which season a build plays in, and the soft reset that opens one.
//!
//! A season is named by the minor version of the game: every patch of v0.2 (v0.2.0, v0.2.6,
//! v0.2.6-r1) plays in season `v0.2`, and the first patch of v0.3 opens the next. The version is the
//! newest patch in `crates/cards/patches/patches.json` (R375), the one the site footer names.
//!
//! The soft reset pulls every rated player's hidden rating `SEASON_RESET_STRENGTH` of the way toward
//! the mean of those players' ratings and widens each deviation by `SEASON_RESET_DEVIATION_BOOST` in
//! quadrature, capped at a new player's deviation; volatility is kept. The season's ladder starts
//! empty, so everyone is a Raisin again. Bots are not reset: their ratings measure a fixed program,
//! not a player coming back from a break (R610).
//!
//! Pure: `src/api/ranked.rs` reads the players, calls `soft_reset`, and writes the result and the
//! season row in one transaction; `src/cli/season_start.rs` runs the same path against a database
//! (a copy first, `--dry-run`, which reports and rolls back).

use serde::{Deserialize, Serialize};

use super::glicko2::Glicko;
use crate::config::{RATING_DEVIATION_START, SEASON_RESET_DEVIATION_BOOST, SEASON_RESET_STRENGTH};

/// The leading `v<major>.<minor>` of a patch version, read by hand (no regex crate).
fn minor_version(patch_version: &str) -> Option<(&str, &str)> {
    let rest = patch_version.strip_prefix('v')?;
    let major_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if major_end == 0 {
        return None;
    }
    let (major, rest) = rest.split_at(major_end);
    let rest = rest.strip_prefix('.')?;
    let minor_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if minor_end == 0 {
        return None;
    }
    let (minor, rest) = rest.split_at(minor_end);
    match rest.chars().next() {
        None | Some('.') | Some('-') => Some((major, minor)),
        Some(_) => None,
    }
}

/// A run of ASCII digits without its leading zeros.
fn number_text(digits: &str) -> String {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// R609: the season a patch version plays in, `v<major>.<minor>`. Panics on a version it cannot
/// read: it is compiled in from `patches.json`, so an unreadable one is a broken build.
pub fn season_id_of(patch_version: &str) -> String {
    match minor_version(patch_version) {
        Some((major, minor)) => format!("v{}.{}", number_text(major), number_text(minor)),
        None => panic!("\"{patch_version}\" is not a patch version (v<major>.<minor>…)"),
    }
}

/// One rated player going into a reset.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResetPlayer {
    pub profile_id: String,
    pub glicko: Glicko,
}

/// One player's reset: the rating before and after.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResetChange {
    pub profile_id: String,
    pub before: Glicko,
    pub after: Glicko,
}

/// What a reset did, for the operator who runs it and the log line the server writes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResetReport {
    pub players: usize,
    /// The mean rating the reset pulls toward, which it also leaves unchanged.
    pub mean: f64,
    /// Population standard deviation of the ratings, before and after.
    pub spread_before: f64,
    pub spread_after: f64,
    /// Mean rating deviation, before and after.
    pub deviation_before: f64,
    pub deviation_after: f64,
    /// The lowest and highest rating, before and after.
    pub lowest_before: f64,
    pub highest_before: f64,
    pub lowest_after: f64,
    pub highest_after: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SoftReset {
    pub changes: Vec<ResetChange>,
    pub report: ResetReport,
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().fold(0.0, |sum, value| sum + value) / values.len() as f64
    }
}

fn spread(values: &[f64], centre: f64) -> f64 {
    let squares: Vec<f64> = values
        .iter()
        .map(|value| (value - centre) * (value - centre))
        .collect();
    mean(&squares).sqrt()
}

fn lowest(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn highest(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

pub fn reset_glicko(glicko: &Glicko, centre: f64) -> Glicko {
    Glicko {
        rating: centre + (glicko.rating - centre) * (1.0 - SEASON_RESET_STRENGTH),
        deviation: (glicko.deviation * glicko.deviation
            + SEASON_RESET_DEVIATION_BOOST * SEASON_RESET_DEVIATION_BOOST)
            .sqrt()
            .min(RATING_DEVIATION_START),
        volatility: glicko.volatility,
    }
}

/// R609: the soft reset of every rated player. Players are taken in profile-id order, so the mean is
/// summed the same way whatever order the store returned them in, and the same players always reset
/// to the same ratings.
pub fn soft_reset(players: &[ResetPlayer]) -> SoftReset {
    let mut ordered: Vec<ResetPlayer> = players.to_vec();
    // Stable sort (SURFACE §4.4.1).
    ordered.sort_by(|a, b| a.profile_id.cmp(&b.profile_id));
    let before: Vec<f64> = ordered.iter().map(|player| player.glicko.rating).collect();
    let centre = mean(&before);
    let changes: Vec<ResetChange> = ordered
        .iter()
        .map(|player| ResetChange {
            profile_id: player.profile_id.clone(),
            before: player.glicko,
            after: reset_glicko(&player.glicko, centre),
        })
        .collect();
    let after: Vec<f64> = changes.iter().map(|change| change.after.rating).collect();
    let deviations_before: Vec<f64> = changes.iter().map(|change| change.before.deviation).collect();
    let deviations_after: Vec<f64> = changes.iter().map(|change| change.after.deviation).collect();
    let report = ResetReport {
        players: ordered.len(),
        mean: centre,
        spread_before: spread(&before, centre),
        spread_after: spread(&after, mean(&after)),
        deviation_before: mean(&deviations_before),
        deviation_after: mean(&deviations_after),
        lowest_before: if before.is_empty() { 0.0 } else { lowest(&before) },
        highest_before: if before.is_empty() { 0.0 } else { highest(&before) },
        lowest_after: if after.is_empty() { 0.0 } else { lowest(&after) },
        highest_after: if after.is_empty() { 0.0 } else { highest(&after) },
    };
    SoftReset { changes, report }
}
