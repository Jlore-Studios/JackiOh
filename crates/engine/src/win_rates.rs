//! ME-STATS (docs/meditative-set.md M5, SPEC §8.8 row 50): the compiled-in table of card win
//! rates M #50 CN Tech reads, and the pick it makes from it.
//!
//! The table is `crates/cards/data/win_rates.json`, validated by `crates/cards/build.rs` and
//! compiled in beside the catalog, so the pure crates keep their two statics (CLAUDE.md rule 4):
//! per card, [[R377]]'s in-deck wins and games. The leader is the best exact rate among the cards
//! with at least `CN_TECH_MIN_GAMES` games, then more games, then the catalog id — never CN Tech
//! itself ([[R387]]), Glitch, a token or an id the catalog no longer holds. With none qualifying,
//! CN Tech's script falls back to a random non-token card (that fallback lives in the card file,
//! not here). The table changes only with a card patch, so a game's seed and log replay on the
//! build that played it (MD-D1, MD-D2).

use serde::{Deserialize, Serialize};

use crate::config::CN_TECH_MIN_GAMES;
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
#[serde(rename_all = "camelCase")]
pub struct WinRateRow {
    pub id: String,
    pub wins: i32,
    pub games: i32,
}

/// The whole table, as `crates/cards/data/win_rates.json` writes it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct WinRateTable {
    #[serde(default)]
    pub patch: String,
    #[serde(default)]
    pub source: WinRateSource,
    #[serde(default)]
    pub cards: Vec<WinRateRow>,
}

/// MD-D1: the card with the highest win rate, leaving `exclude_def_id` (CN Tech itself, [[R387]])
/// out. Only rows with at least `CN_TECH_MIN_GAMES` games qualify, and only ids the catalog still
/// holds as deckable cards — which rules out tokens (Glitch is one) and sets that do not ship.
/// Ties break by more games, then by the catalog id, ascending. The rate is compared exactly, as
/// `wins_a * games_b` against `wins_b * games_a` in `i64`, so no float ever decides it. `None` when
/// no row qualifies (a new patch, an empty table).
pub fn win_rate_leader(exclude_def_id: &str) -> Option<String> {
    let table = crate::catalog::registered_win_rates();
    let mut best: Option<&WinRateRow> = None;
    for row in &table.cards {
        if row.games < CN_TECH_MIN_GAMES || row.id == exclude_def_id {
            continue;
        }
        let Some(def) = crate::catalog::find_def(None, &row.id) else {
            continue;
        };
        if !crate::catalog::deckable(def) {
            continue;
        }
        let takes = match best {
            None => true,
            Some(held) => {
                let left = i64::from(row.wins) * i64::from(held.games);
                let right = i64::from(held.wins) * i64::from(row.games);
                left > right
                    || left == right
                        && (row.games > held.games || row.games == held.games && row.id < held.id)
            }
        };
        if takes {
            best = Some(row);
        }
    }
    best.map(|row| row.id.clone())
}
