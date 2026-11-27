//! R4681000: the Hall of Fame's four lists, read off every play version's figures
//! (`PlayVersionTable`, R4675001): Most broken, Most nerfed, Most buffed and Most changed.
//!
//! Every list is ranked by the bound of the range, not the rate, so a lucky small sample never tops
//! one; only a version with enough games (R654) ranks on its own figures, while a short version may
//! still stand as a pair's companion. A pair neither side of which has figures ranks nowhere.

use std::cmp::Ordering;

use indexmap::IndexMap;
use serde::Serialize;

use super::play_versions::{PlayVersionFigures, PlayVersionTable};

/// R4681000: one Hall of Fame entry's two play-version ids: the ranked version and the version beside
/// it (the card's live version, the pair's other side, or itself where before equals after).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HallPair {
    pub before: String,
    pub after: String,
}

/// R4681000: the four lists, each entry its version ids; the changed list also carries how many
/// changes lie between its two versions (`card_change_totals`, R4681000).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HallOfFame {
    pub broken: Vec<HallPair>,
    pub nerfed: Vec<HallPair>,
    pub buffed: Vec<HallPair>,
    pub changed: Vec<(HallPair, usize)>,
}

/// A rate with its range, as a pair's ranking reads them; `None` where the version has no figures.
fn rated(row: &PlayVersionFigures) -> Option<(f64, f64, f64)> {
    match (row.adjusted_win_rate, row.adjusted_low, row.adjusted_high) {
        (Some(rate), Some(low), Some(high)) => Some((rate, low, high)),
        _ => None,
    }
}

/// R4681000: the four lists over `table`, each kept to `top`; `changes_of` names each card's change
/// lines and summed `fields` lengths (`change_totals_since`), which the changed list ranks ties by
/// and counts between its two versions. The table's order is each card's oldest first, so consecutive
/// rows of one card are consecutive play versions.
pub fn hall_of_fame(
    table: &PlayVersionTable,
    top: usize,
    changes_of: &IndexMap<String, (usize, usize)>,
) -> HallOfFame {
    let mut by_card: IndexMap<&str, Vec<&PlayVersionFigures>> = IndexMap::new();
    for row in &table.versions {
        by_card.entry(row.card.as_str()).or_default().push(row);
    }
    /// A card's live play version, or its newest where it has none live.
    fn live_of<'a>(versions: &[&'a PlayVersionFigures]) -> &'a PlayVersionFigures {
        versions
            .iter()
            .rev()
            .find(|row| row.live)
            .or(versions.last())
            .expect("a card's versions are never empty")
    }

    // Most broken: the highest Adjusted of all time, by the range's lower bound.
    let mut broken: Vec<&PlayVersionFigures> = table
        .versions
        .iter()
        .filter(|row| row.has_enough_games && rated(row).is_some())
        .collect();
    broken.sort_by(|a, b| {
        let (a_rate, a_low, _) = rated(a).unwrap_or((0.0, 0.0, 0.0));
        let (b_rate, b_low, _) = rated(b).unwrap_or((0.0, 0.0, 0.0));
        b_low
            .partial_cmp(&a_low)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b_rate.partial_cmp(&a_rate).unwrap_or(Ordering::Equal))
            .then_with(|| a.id.cmp(&b.id))
    });
    let broken = broken
        .into_iter()
        .take(top)
        .map(|row| {
            let live = live_of(&by_card[row.card.as_str()]);
            HallPair {
                before: row.id.clone(),
                after: live.id.clone(),
            }
        })
        .collect();

    // Consecutive play versions of one card, oldest first.
    let mut pairs: Vec<(&PlayVersionFigures, &PlayVersionFigures)> = Vec::new();
    for versions in by_card.values() {
        for pair in versions.windows(2) {
            pairs.push((pair[0], pair[1]));
        }
    }
    /// The bound gap of a fall or a rise, where the higher side ranks: `None` where either side has
    /// no figures or the order runs the other way.
    fn gap(high: &PlayVersionFigures, low: &PlayVersionFigures) -> Option<f64> {
        let (high_rate, _, high_low) = rated(high)?;
        let (low_rate, _, low_high) = rated(low)?;
        (high_rate > low_rate).then_some(high_low - low_high)
    }
    // Most nerfed: the biggest falls, ranked by what the figures still show once both sides' noise
    // is out; the before version ranks, the after one may be short of games.
    let mut nerfed: Vec<(&PlayVersionFigures, &PlayVersionFigures)> = pairs
        .iter()
        .filter(|(before, _)| before.has_enough_games)
        .filter_map(|(before, after)| gap(before, after).map(|_| (*before, *after)))
        .collect();
    nerfed.sort_by(|a, b| {
        gap(b.0, b.1)
            .partial_cmp(&gap(a.0, a.1))
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });
    // Most buffed, the same way mirrored: the after version ranks, the before one may be short.
    let mut buffed: Vec<(&PlayVersionFigures, &PlayVersionFigures)> = pairs
        .iter()
        .filter(|(_, after)| after.has_enough_games)
        .filter_map(|(before, after)| gap(after, before).map(|_| (*before, *after)))
        .collect();
    buffed.sort_by(|a, b| {
        gap(b.1, b.0)
            .partial_cmp(&gap(a.1, a.0))
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });
    let pair_of = |(before, after): (&PlayVersionFigures, &PlayVersionFigures)| HallPair {
        before: before.id.clone(),
        after: after.id.clone(),
    };

    // Most changed: the most versions, ties by fields changed, then the card id; one version is none.
    let mut cards: Vec<(&str, usize, usize)> = by_card
        .iter()
        .map(|(card, versions)| {
            let fields = changes_of.get(*card).map(|(_, fields)| *fields).unwrap_or(0);
            (*card, versions.len(), fields)
        })
        .filter(|(_, count, _)| *count >= 2)
        .collect();
    cards.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.0.cmp(b.0))
    });
    let changed = cards
        .into_iter()
        .take(top)
        .map(|(card, _, _)| {
            let versions = &by_card[card];
            let live = live_of(versions);
            (
                HallPair {
                    before: versions[0].id.clone(),
                    after: live.id.clone(),
                },
                changes_of.get(card).map(|(lines, _)| *lines).unwrap_or(0),
            )
        })
        .collect();

    HallOfFame {
        broken,
        nerfed: nerfed.into_iter().take(top).map(pair_of).collect(),
        buffed: buffed.into_iter().take(top).map(pair_of).collect(),
        changed,
    }
}

// R4681000: the lists over tables built by hand, so each ranking reads off the test.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::stats::{CardTier, DecisivenessField, StatsField};

    const FLOOR: i64 = 20;

    /// One play version's row: `games` below the floor leaves its figures but takes it out of every
    /// ranking but a pair's companion.
    fn fig(card: &str, first: &str, live: bool, games: i64, rate: f64, tier: CardTier) -> PlayVersionFigures {
        PlayVersionFigures {
            id: format!("{card}~{first}"),
            card: card.to_string(),
            name: format!("{card} name"),
            first: first.to_string(),
            last: first.to_string(),
            live,
            iteration: "v1".to_string(),
            patches: 1,
            first_shipped: None,
            games,
            adjusted_win_rate: Some(rate),
            adjusted_low: Some(rate - 0.02),
            adjusted_high: Some(rate + 0.02),
            decisiveness: None,
            tier: Some(tier),
            has_enough_games: games >= FLOOR,
            win_rate: None,
            play_rate: None,
        }
    }

    fn table(versions: Vec<PlayVersionFigures>) -> PlayVersionTable {
        PlayVersionTable {
            field: StatsField::PRIOR,
            band_low: 0.45,
            band_high: 0.55,
            decisiveness: DecisivenessField::PRIOR.typical,
            min_sample: FLOOR,
            versions,
        }
    }

    fn changes(lines: &[(&str, usize, usize)]) -> IndexMap<String, (usize, usize)> {
        lines
            .iter()
            .map(|(card, lines, fields)| (card.to_string(), (*lines, *fields)))
            .collect()
    }

    #[test]
    fn r4681000_nerf_from_s_to_c_tops_most_nerfed() {
        let table = table(vec![
            fig("core-001", "v1", false, 500, 0.61, CardTier::S),
            fig("core-001", "v2", true, 500, 0.50, CardTier::C),
            fig("core-002", "v1", true, 500, 0.52, CardTier::B),
        ]);
        let hall = hall_of_fame(&table, 10, &changes(&[]));
        assert_eq!(
            hall.nerfed[0],
            HallPair {
                before: "core-001~v1".to_string(),
                after: "core-001~v2".to_string(),
            }
        );
        assert!(hall.buffed.is_empty());
        // The S version is the most broken too, shown beside its live one.
        assert_eq!(
            hall.broken[0],
            HallPair {
                before: "core-001~v1".to_string(),
                after: "core-001~v2".to_string(),
            }
        );
        // Two versions are a changed card, with no counted changes.
        assert_eq!(hall.changed, [(hall.nerfed[0].clone(), 0)],);
    }

    #[test]
    fn r4681000_short_after_still_shows_as_companion() {
        let table = table(vec![
            fig("core-001", "v1", false, 500, 0.61, CardTier::S),
            fig("core-001", "v2", true, 3, 0.50, CardTier::C),
        ]);
        let hall = hall_of_fame(&table, 10, &changes(&[]));
        // The fall still ranks on its before version's figures; the companion stays out of every
        // ranking of its own.
        assert_eq!(hall.nerfed.len(), 1);
        assert_eq!(hall.nerfed[0].after, "core-001~v2");
        assert_eq!(hall.broken.len(), 1);
        assert_eq!(hall.broken[0].before, "core-001~v1");
        assert!(hall.buffed.is_empty(), "a short after version ranks no rise");
    }

    #[test]
    fn r4681000_small_sample_spike_ranks_nowhere() {
        let table = table(vec![
            fig("core-001", "v1", true, 500, 0.52, CardTier::B),
            fig("core-002", "v1", true, 3, 0.99, CardTier::S),
        ]);
        let hall = hall_of_fame(&table, 10, &changes(&[]));
        for list in [&hall.broken, &hall.nerfed, &hall.buffed] {
            assert!(
                list.iter()
                    .all(|pair| pair.before != "core-002~v1" && pair.after != "core-002~v1"),
                "the spike ranks nowhere: {list:?}"
            );
        }
        assert!(hall.changed.is_empty());
        assert_eq!(hall.broken[0].before, "core-001~v1");
    }

    #[test]
    fn r4681000_four_versions_top_most_changed() {
        let table = table(vec![
            fig("core-004", "v1", false, 100, 0.52, CardTier::B),
            fig("core-004", "v2", false, 100, 0.51, CardTier::B),
            fig("core-004", "v3", false, 100, 0.53, CardTier::B),
            fig("core-004", "v4", true, 100, 0.50, CardTier::C),
            fig("core-001", "v1", false, 500, 0.61, CardTier::S),
            fig("core-001", "v2", true, 500, 0.50, CardTier::C),
            fig("core-005", "v1", true, 500, 0.55, CardTier::A),
        ]);
        let hall = hall_of_fame(&table, 10, &changes(&[("core-004", 7, 9), ("core-001", 2, 2)]));
        assert_eq!(
            hall.changed[0],
            (
                HallPair {
                    before: "core-004~v1".to_string(),
                    after: "core-004~v4".to_string(),
                },
                7,
            )
        );
        // The unchanged card contends Most broken alone: no pair, no changed entry.
        assert!(hall.broken.iter().any(|pair| pair.before == "core-005~v1"));
        for list in [&hall.nerfed, &hall.buffed] {
            assert!(
                list.iter()
                    .all(|pair| !pair.before.starts_with("core-005") && !pair.after.starts_with("core-005")),
                "{list:?}"
            );
        }
        assert!(
            hall.changed
                .iter()
                .all(|(pair, _)| !pair.before.starts_with("core-005"))
        );
    }

    #[test]
    fn r4681000_rise_tops_most_buffed() {
        let table = table(vec![
            fig("core-003", "v1", false, 500, 0.40, CardTier::F),
            fig("core-003", "v2", true, 500, 0.60, CardTier::S),
            fig("core-002", "v1", true, 500, 0.52, CardTier::B),
        ]);
        let hall = hall_of_fame(&table, 10, &changes(&[]));
        assert_eq!(
            hall.buffed[0],
            HallPair {
                before: "core-003~v1".to_string(),
                after: "core-003~v2".to_string(),
            }
        );
        assert!(hall.nerfed.is_empty());
        assert_eq!(hall.broken[0].before, "core-003~v2");
    }
}
