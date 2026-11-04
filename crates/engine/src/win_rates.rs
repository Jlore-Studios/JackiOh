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
//! R3049001: the leader is the card whose 95% Wilson lower bound is highest, computed in integers to a
//! millionth (`wilson_bounds`), among the cards with at least `CN_TECH_MIN_GAMES` games, then more
//! games, then the catalog id — never CN Tech itself ([[R387]]), Glitch, a token or an id the catalog no
//! longer holds. Meditative+ #2 NA Tech also reads the trailer (R1183000), the lowest upper bound, with
//! the same eligibility and tie-breaks. A game that holds its pools (R2201001) ranks without Legendary
//! and Mythic cards (R2386001).

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::config::{
    CN_TECH_MIN_GAMES, WIN_RATE_BOUND_SCALE, WIN_RATE_BOUND_Z_DEN, WIN_RATE_BOUND_Z_NUM,
    WIN_RATE_SNAPSHOT_ENDS,
};
use crate::state::GameState;
use crate::wire::stats::{GameSource, WindowedReport};
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

/// MD-D1, R3049001: the card `table` ranks highest, leaving `exclude_def_ids` (CN Tech itself,
/// [[R387]]) out: the highest Wilson lower bound among the rows with at least `CN_TECH_MIN_GAMES`
/// games whose ids the catalog still holds as deckable cards (no token, Glitch being one). Ties break
/// by more games, then by the catalog id, ascending.
pub fn leader_of(table: &WinRateTable, exclude_def_ids: &[&str]) -> Option<String> {
    ranked_end(table, exclude_def_ids, RateEnd::Highest)
}

/// R1183000, R3049001 (Meditative+ #2 NA Tech): the card `table` ranks lowest, [[R1100]]'s leader read
/// from the other end — the lowest Wilson upper bound — with the same eligibility and tie-breaks;
/// `exclude_def_ids` holds NA Tech itself ([[R387]]) and the leader, so the two ends are two cards.
pub fn trailer_of(table: &WinRateTable, exclude_def_ids: &[&str]) -> Option<String> {
    ranked_end(table, exclude_def_ids, RateEnd::Lowest)
}

/// R3049000: the table `state`'s picks read: the snapshot it was handed, else the one a state with
/// none reads (`catalog::registered_win_rates`: its shipped catalog's frozen table, R1266000).
pub fn win_rates_of(state: &GameState) -> &WinRateTable {
    match &state.win_rates {
        Some(table) => table,
        None => crate::catalog::registered_win_rates(),
    }
}

/// MD-D1: `leader_of` the table `state` reads (`win_rates_of`), leaving `exclude_def_id` out.
pub fn win_rate_leader(state: &GameState, exclude_def_id: &str) -> Option<String> {
    leader_of(win_rates_of(state), &[exclude_def_id])
}

/// R1183000: `trailer_of` the table `state` reads (`win_rates_of`), leaving `exclude_def_ids` out.
pub fn win_rate_trailer(state: &GameState, exclude_def_ids: &[&str]) -> Option<String> {
    trailer_of(win_rates_of(state), exclude_def_ids)
}

/// R3049000: what a game keeps of the snapshot it is handed: the first `WIN_RATE_SNAPSHOT_ENDS`
/// rows each end ranks, with the catalog and the pool hold in force, each row once, sorted by id,
/// under `table`'s `patch` and `source`. A pick passes over at most its card and the leader, so every
/// pick reads the same card off what is kept as off the whole snapshot, and a state, a clone the AI
/// searches and a match row carry a few rows rather than the catalog's. Keeping it again keeps it
/// all, so a fold of the kept rows keeps the same rows.
pub fn kept_for_game(table: &WinRateTable) -> WinRateTable {
    let mut cards: Vec<WinRateRow> = Vec::new();
    for end in [RateEnd::Highest, RateEnd::Lowest] {
        let mut rows: Vec<Ranked<'_>> = table.cards.iter().filter_map(|row| eligible(row, &[])).collect();
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

/// A row a pick may name, with its Wilson bounds.
struct Ranked<'a> {
    row: &'a WinRateRow,
    lower: u128,
    upper: u128,
}

/// A row with at least `CN_TECH_MIN_GAMES` games, not in `exclude_def_ids`, whose id the catalog
/// holds as a deckable card a held pool keeps, and whose figures have bounds.
fn eligible<'a>(row: &'a WinRateRow, exclude_def_ids: &[&str]) -> Option<Ranked<'a>> {
    if row.games < CN_TECH_MIN_GAMES || exclude_def_ids.contains(&row.id.as_str()) {
        return None;
    }
    let def = crate::catalog::find_def(None, &row.id)?;
    // R2386001: a game that holds its pools ranks without the cards a held pool leaves out.
    if !crate::catalog::deckable(def) || crate::catalog::held_out(def) {
        return None;
    }
    let (lower, upper) = wilson_bounds(row.wins, row.games)?;
    Some(Ranked { row, lower, upper })
}

/// R3049001: `a` before `b` at `end`: the higher lower bound first toward the highest, the lower
/// upper bound first toward the lowest, then more games, then the smaller id.
fn rank(a: &Ranked<'_>, b: &Ranked<'_>, end: RateEnd) -> Ordering {
    match end {
        RateEnd::Highest => b.lower.cmp(&a.lower),
        RateEnd::Lowest => a.upper.cmp(&b.upper),
    }
    .then(b.row.games.cmp(&a.row.games))
    .then_with(|| a.row.id.cmp(&b.row.id))
}

/// The one walk both ends share: the row that ranks first at `end`.
fn ranked_end(table: &WinRateTable, exclude_def_ids: &[&str], end: RateEnd) -> Option<String> {
    table
        .cards
        .iter()
        .filter_map(|row| eligible(row, exclude_def_ids))
        .min_by(|a, b| rank(a, b, end))
        .map(|ranked| ranked.row.id.clone())
}
