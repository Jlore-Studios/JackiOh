//! ME-STATS (docs/meditative-set.md M5, SPEC §8.8 row 50): the table of card win rates M #50 CN Tech
//! reads, and the pick it makes from it.
//!
//! R3049000: the table is the game's own, a snapshot of the server's cached card stats (R1367000)
//! handed to `create_game` as setup (`CreateGameOptions.win_rates`) and kept in the state, fixed for
//! the game (`GameState.win_rates`), so every pick it makes reads the same table and a fold of its
//! record, which keeps the same snapshot, reads it again. The game keeps only the rows a pick can name
//! (`kept_for_game`). No table is compiled in: a game handed none reads the frozen table of the shipped
//! catalog its fold runs under (R1266000), else none, and then CN Tech's script falls back (in the
//! card file, not here, R2521007). Per card, [[R377]]'s in-deck wins and games.
//!
//! The leader is the card ranked highest among the cards with at least `CN_TECH_MIN_GAMES` games, then
//! more games, then the catalog id — never CN Tech itself ([[R387]]), Glitch, a token or an id the
//! catalog no longer holds. Meditative+ #2 NA Tech also reads the trailer (R1183000), the card ranked
//! lowest, with the same eligibility and tie-breaks. How a table ranks (`WinRateRanking`) goes with
//! where it came from: a game's own snapshot by its 95% Wilson bounds, computed in integers to a
//! millionth (`wilson_bounds`, R3049001), and a table read with no snapshot by its exact rate in `i64`
//! ([[R1100]]), as the version that shipped it ranked it, so a record from before #3049 picks its card
//! again. A game that holds its pools (R2201001) ranks without Legendary and Mythic cards (R2386001).

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::config::{
    ADJUSTED_WIN_RATE_CENTER, CN_TECH_MIN_GAMES, NO_RECORD_WIN_RATE, TIER_A_FROM, TIER_BAND_HIGH,
    TIER_BAND_LOW, TIER_C_FROM, WIN_RATE_BOUND_SCALE, WIN_RATE_BOUND_Z_DEN, WIN_RATE_BOUND_Z_NUM,
    WIN_RATE_RECORD_MIN_GAMES, WIN_RATE_SNAPSHOT_ENDS,
};
use crate::state::GameState;
use crate::wire::stats::{CardTier, GameSource, WindowedReport};
use crate::wire::string_union;

string_union! {
    /// Whether the table's figures are live games or a placeholder until some are counted.
    #[derive(Default)]
    pub enum WinRateSource {
        Live = "live",
        #[default]
        Provisional = "provisional",
    }
}

/// One card's in-deck wins and games ([[R377]]'s `inDeck` breakdown).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct WinRateRow {
    pub id: String,
    pub wins: i32,
    pub games: i32,
}

/// The whole table: a game's snapshot (R3049000), or a shipped version's frozen
/// `crates/cards/patches/win_rates/<version>.json` (R1329000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct WinRateTable {
    #[serde(default)]
    pub patch: String,
    #[serde(default)]
    pub source: WinRateSource,
    #[serde(default)]
    pub cards: Vec<WinRateRow>,
}

/// R3049001: the 95% Wilson score interval of `wins` in `games`, in millionths
/// (`WIN_RATE_BOUND_SCALE`), each end rounded down: `wire::stats::wilson_interval` in integers, so no
/// float ever ranks a card ([[R1100]]). With `z = Z_NUM / Z_DEN`, the bounds are
/// `(2w + z² ∓ z·√(4w(n − w)/n + z²)) / 2(n + z²)`, scaled through by `Z_DEN²`. `None` for no games
/// or wins outside `0..=games`. It fits `u128` for any `i32`.
pub fn wilson_bounds(wins: i32, games: i32) -> Option<(u128, u128)> {
    if games <= 0 || wins < 0 || wins > games {
        return None;
    }
    let (w, n) = (u128::from(wins.unsigned_abs()), u128::from(games.unsigned_abs()));
    let zn = WIN_RATE_BOUND_Z_NUM * WIN_RATE_BOUND_Z_NUM;
    let zd = WIN_RATE_BOUND_Z_DEN * WIN_RATE_BOUND_Z_DEN;
    let s = WIN_RATE_BOUND_SCALE;
    let centre = 2 * zd * w + zn;
    let spread = zn * (4 * zd * w * (n - w) + n * zn);
    let denominator = 2 * (zd * n + zn);
    let root = (spread * s * s / n).isqrt();
    Some((
        (centre * s - root) / denominator,
        (centre * s + root) / denominator,
    ))
}

/// How a pick ranks a table's rows, which goes with where the table came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WinRateRanking {
    /// R3049001: a game's own snapshot (R3049000): the highest Wilson lower bound leads and the
    /// lowest upper bound trails.
    Bound,
    /// [[R1100]], R1183000: a table a game handed no snapshot reads, a shipped version's frozen table
    /// (R1266000) or the testkit's: the exact rate, as the version that shipped it ranked it.
    Rate,
}

/// MD-D1: the card `table` ranks highest by `ranking`, leaving `exclude_def_ids` (CN Tech itself,
/// [[R387]]) out, among the rows with at least `CN_TECH_MIN_GAMES` games whose ids the catalog still
/// holds as deckable cards (no token, Glitch being one). Ties break by more games, then by the catalog
/// id, ascending.
pub fn leader_of(table: &WinRateTable, ranking: WinRateRanking, exclude_def_ids: &[&str]) -> Option<String> {
    ranked_end(table, ranking, exclude_def_ids, RateEnd::Highest)
}

/// R1183000 (Meditative+ #2 NA Tech): the card `table` ranks lowest by `ranking`, [[R1100]]'s leader
/// read from the other end with the same eligibility and tie-breaks; `exclude_def_ids` holds NA Tech
/// itself ([[R387]]) and the leader, so the two ends are two cards.
pub fn trailer_of(table: &WinRateTable, ranking: WinRateRanking, exclude_def_ids: &[&str]) -> Option<String> {
    ranked_end(table, ranking, exclude_def_ids, RateEnd::Lowest)
}

/// R3049000: the table `state`'s picks read and how they rank it: the snapshot it was handed, by its
/// Wilson bounds (R3049001), else the one a state with none reads (`catalog::registered_win_rates`:
/// its shipped catalog's frozen table, R1266000), by its exact rate, as that version ranked it.
pub fn win_rates_of(state: &GameState) -> (&WinRateTable, WinRateRanking) {
    match &state.win_rates {
        Some(table) => (table, WinRateRanking::Bound),
        None => (crate::catalog::registered_win_rates(), WinRateRanking::Rate),
    }
}

/// MD-D1: `leader_of` the table `state` reads (`win_rates_of`), leaving `exclude_def_id` out.
pub fn win_rate_leader(state: &GameState, exclude_def_id: &str) -> Option<String> {
    let (table, ranking) = win_rates_of(state);
    leader_of(table, ranking, &[exclude_def_id])
}

/// R1183000: `trailer_of` the table `state` reads (`win_rates_of`), leaving `exclude_def_ids` out.
pub fn win_rate_trailer(state: &GameState, exclude_def_ids: &[&str]) -> Option<String> {
    let (table, ranking) = win_rates_of(state);
    trailer_of(table, ranking, exclude_def_ids)
}

/// R3049000: what a game keeps of the snapshot it is handed: the first `WIN_RATE_SNAPSHOT_ENDS`
/// rows each end ranks by its bounds (`WinRateRanking::Bound`), with the catalog and the pool hold in
/// force, each row once, sorted by id, under `table`'s `patch` and `source`. A pick passes over at
/// most its card and the leader, so every pick reads the same card off what is kept as off the whole
/// snapshot, and a state, a clone the AI searches and a match row carry a few rows rather than the
/// catalog's. Keeping it again keeps it all, so a fold of the kept rows keeps the same rows.
pub fn kept_for_game(table: &WinRateTable) -> WinRateTable {
    let mut cards: Vec<WinRateRow> = Vec::new();
    for end in [RateEnd::Highest, RateEnd::Lowest] {
        let mut rows: Vec<Ranked<'_>> = table
            .cards
            .iter()
            .filter_map(|row| eligible(row, WinRateRanking::Bound, &[]))
            .collect();
        rows.sort_by(|a, b| rank(a, b, end));
        cards.extend(
            rows.into_iter()
                .take(WIN_RATE_SNAPSHOT_ENDS)
                .map(|ranked| ranked.row.clone()),
        );
    }
    cards.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then(a.games.cmp(&b.games))
            .then(a.wins.cmp(&b.wins))
    });
    cards.dedup();
    WinRateTable {
        patch: table.patch.clone(),
        source: table.source,
        cards,
    }
}

/// R3538000: a card's winrate of record: the newest per-version table row at or before the version
/// the id names, past patches included. Under the testkit, while a table override is set, it is that
/// table's row for the id itself. A row under `WIN_RATE_RECORD_MIN_GAMES` games is no record, and
/// neither is a missing row: both read as the 50% default (`rate_of`), never here.
pub fn win_rate_of_record(def_id: &str) -> Option<(i32, i32)> {
    #[cfg(feature = "testkit")]
    if let Some(table) = crate::testkit::scenario::win_rates_override() {
        return table
            .cards
            .iter()
            .find(|row| row.id == def_id)
            .filter(|row| row.games >= WIN_RATE_RECORD_MIN_GAMES)
            .map(|row| (row.wins, row.games));
    }
    let (card, version) = match crate::catalog::split_past_id(def_id) {
        Some((card, version)) => (card, version),
        None => (def_id, crate::catalog::catalog_version()),
    };
    crate::catalog::past_record(card, version).filter(|(_, games)| *games >= WIN_RATE_RECORD_MIN_GAMES)
}

/// R3538000: a threshold in millionths (`WIN_RATE_BOUND_SCALE`), rounded, so every comparison below
/// stays in integers as R3049001's do.
fn scaled(threshold: f64) -> u128 {
    (threshold * WIN_RATE_BOUND_SCALE as f64).round() as u128
}

/// R3538000: a card's rate as an exact `(numerator, denominator)`: its record, else the 50%
/// default in millionths (`NO_RECORD_WIN_RATE`), which KPIs itself reads when it has none. Both
/// sides stay unscaled records or the scaled default, so a crosswise comparison reads the true
/// order either way.
fn rate_of(def_id: &str) -> (u128, u128) {
    match win_rate_of_record(def_id) {
        Some((wins, games)) => (
            u128::from(wins.unsigned_abs()),
            u128::from(games.unsigned_abs()),
        ),
        None => (scaled(NO_RECORD_WIN_RATE), WIN_RATE_BOUND_SCALE),
    }
}

/// R3538007: whether `a`'s rate is strictly lower than `b`'s, compared crosswise in integers; a card
/// with no record counts as 50% (R3538000), and so does `b` with none.
pub fn lower_win_rate(a: &str, b: &str) -> bool {
    let (a_num, a_den) = rate_of(a);
    let (b_num, b_den) = rate_of(b);
    a_num * b_den < b_num * a_den
}

/// R3538001: a card's tier of record: R3022000's tier (`wire::stats::tier_of`) read off the record —
/// the exact rate against the band and the quarter floors, the S/F certainty against the Wilson
/// bounds, all in integers. `None` with no record.
pub fn tier_of_record(def_id: &str) -> Option<CardTier> {
    let (wins, games) = win_rate_of_record(def_id)?;
    let (w, g) = (u128::from(wins.unsigned_abs()), u128::from(games.unsigned_abs()));
    let (low, high) = wilson_bounds(wins, games)?;
    let rate = w * WIN_RATE_BOUND_SCALE;
    let center = scaled(ADJUSTED_WIN_RATE_CENTER);
    if rate > scaled(TIER_BAND_HIGH) * g && low > center {
        Some(CardTier::S)
    } else if rate < scaled(TIER_BAND_LOW) * g && high < center {
        Some(CardTier::F)
    } else if rate >= scaled(TIER_A_FROM) * g {
        Some(CardTier::A)
    } else if rate >= center * g {
        Some(CardTier::B)
    } else if rate >= scaled(TIER_C_FROM) * g {
        Some(CardTier::C)
    } else {
        Some(CardTier::D)
    }
}

/// R3049000, R1367000: the table a windowed card report holds, as `/api/stats/cards` shows it: each
/// card's in-deck wins and games over its own window, a card with no games left out, sorted by id and
/// filed under `report.at`. It is `live` only when the report holds a card and no card's window
/// counts a development game.
pub fn win_rates_from_report(report: &WindowedReport) -> WinRateTable {
    let live = !report.cards.is_empty()
        && report
            .cards
            .iter()
            .all(|card| !card.window.sources().contains(&GameSource::Dev));
    let mut cards: Vec<WinRateRow> = report
        .cards
        .iter()
        .filter(|card| card.stats.in_deck.games > 0)
        .map(|card| WinRateRow {
            id: card.stats.card.clone(),
            wins: card.stats.in_deck.wins,
            games: card.stats.in_deck.games,
        })
        .collect();
    cards.sort_by(|a, b| a.id.cmp(&b.id));
    WinRateTable {
        patch: report.at.clone(),
        source: if live {
            WinRateSource::Live
        } else {
            WinRateSource::Provisional
        },
        cards,
    }
}

/// Which end of the table a pick reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RateEnd {
    Highest,
    Lowest,
}

/// A row a pick may name, with its Wilson bounds when its table ranks by them.
struct Ranked<'a> {
    row: &'a WinRateRow,
    /// R3049001: `(lower, upper)` for `WinRateRanking::Bound`; `None` for `WinRateRanking::Rate`.
    bounds: Option<(u128, u128)>,
}

/// A row with at least `CN_TECH_MIN_GAMES` games, not in `exclude_def_ids`, whose id the catalog
/// holds as a deckable card a held pool keeps, and, ranked by `WinRateRanking::Bound`, whose figures
/// have bounds.
fn eligible<'a>(
    row: &'a WinRateRow,
    ranking: WinRateRanking,
    exclude_def_ids: &[&str],
) -> Option<Ranked<'a>> {
    if row.games < CN_TECH_MIN_GAMES || exclude_def_ids.contains(&row.id.as_str()) {
        return None;
    }
    let def = crate::catalog::find_def(None, &row.id)?;
    // R2386001: a game that holds its pools ranks without the cards a held pool leaves out.
    if !crate::catalog::deckable(def) || crate::catalog::held_out(def) {
        return None;
    }
    let bounds = match ranking {
        WinRateRanking::Bound => Some(wilson_bounds(row.wins, row.games)?),
        WinRateRanking::Rate => None,
    };
    Some(Ranked { row, bounds })
}

/// `a` before `b` at `end`, two rows of one table ranked alike: by their bounds (R3049001), the
/// higher lower bound first toward the highest and the lower upper bound first toward the lowest, or
/// by the exact rate, compared crosswise in `i64` so no float decides it ([[R1100]]); then more games,
/// then the smaller id.
fn rank(a: &Ranked<'_>, b: &Ranked<'_>, end: RateEnd) -> Ordering {
    match (a.bounds, b.bounds) {
        (Some((a_lower, a_upper)), Some((b_lower, b_upper))) => match end {
            RateEnd::Highest => b_lower.cmp(&a_lower),
            RateEnd::Lowest => a_upper.cmp(&b_upper),
        },
        _ => {
            let a_rate = i64::from(a.row.wins) * i64::from(b.row.games);
            let b_rate = i64::from(b.row.wins) * i64::from(a.row.games);
            match end {
                RateEnd::Highest => b_rate.cmp(&a_rate),
                RateEnd::Lowest => a_rate.cmp(&b_rate),
            }
        }
    }
    .then(b.row.games.cmp(&a.row.games))
    .then_with(|| a.row.id.cmp(&b.row.id))
}

/// The one walk both ends share: the row that ranks first at `end`.
fn ranked_end(
    table: &WinRateTable,
    ranking: WinRateRanking,
    exclude_def_ids: &[&str],
    end: RateEnd,
) -> Option<String> {
    table
        .cards
        .iter()
        .filter_map(|row| eligible(row, ranking, exclude_def_ids))
        .min_by(|a, b| rank(a, b, end))
        .map(|ranked| ranked.row.id.clone())
}
