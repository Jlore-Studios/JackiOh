//! S+ #164 KPIs (SPEC §8.12 row 164). (2) Spell, Epic.
//!   Base:    "Destroy every card with a lower winrate than this."
//!   Radiant: "Destroy every enemy card with a lower winrate than this."
//!
//! Cry (R1): destroy every permanent on both rows whose rate is strictly lower than this card's
//! (`destroy_all` with `winRateBelow`, R3538007); the Radiant face spares its controller's own. A
//! card with no recorded winrate counts as 50%, and so does KPIs itself when it has none
//! (R3538000).
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-164";

fn face(enemy_only: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![destroy_all(json_as(json!({
                "side": if enemy_only { "enemy" } else { "any" },
                "rows": ["units", "backrow"],
                "winRateBelow": ID,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S+ #164 KPIs — SPEC §8.12 row 164 (R3538007, R3538000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A Unit with a high winrate of record.
    const HIGH: &str = "core-008"; // Mr. Vanilla
    /// A Unit with a low winrate of record.
    const LOW: &str = "core-013"; // Jlockeed Shredder-10
    /// A backrow permanent with a low winrate of record.
    const LOW_BACKROW: &str = "core-014"; // Jlockeed's Weapons
    /// A backrow permanent with a high winrate of record.
    const HIGH_BACKROW: &str = "classic-004"; // Palantir
    /// A stand-in Unit, which counts as 50%.
    const EVEN: &str = "stand-in (3) 9/9";

    fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
        WinRateTable {
            patch: "v0.3.X".to_string(),
            source: WinRateSource::Provisional,
            cards: cards
                .into_iter()
                .map(|(id, wins, games)| WinRateRow {
                    id: id.to_string(),
                    wins,
                    games,
                })
                .collect(),
        }
    }

    /// KPIs holds a row at 60%: the low cards go, the high and even ones stay.
    fn records() -> Vec<(&'static str, i32, i32)> {
        vec![
            (ID, 600, 1000),
            (HIGH, 800, 1000),
            (HIGH_BACKROW, 800, 1000),
            (LOW, 200, 1000),
            (LOW_BACKROW, 200, 1000),
        ]
    }

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, "core-005"],
                "field": [
                    { "def": HIGH, "lane": 1 },
                    { "def": LOW, "lane": 2 },
                    { "def": EVEN, "lane": 3 },
                ],
                "backrow": [
                    { "def": HIGH_BACKROW, "lane": 1 },
                    { "def": LOW_BACKROW, "lane": 2 },
                ],
                "library": filler(8),
                "mana": 4,
            },
            "p2": {
                "hand": ["core-005"],
                "field": [
                    { "def": HIGH, "lane": 1 },
                    { "def": LOW, "lane": 2 },
                    { "def": EVEN, "lane": 3 },
                ],
                "backrow": [
                    { "def": HIGH_BACKROW, "lane": 1 },
                    { "def": LOW_BACKROW, "lane": 2 },
                ],
                "library": filler(8),
            },
        }))
    }

    fn unit_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        let mut defs: Vec<String> = (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .map(|unit| unit.def_id.clone())
            .collect();
        defs.sort();
        defs
    }

    fn backrow_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        let mut defs: Vec<String> = (1..=5)
            .filter_map(|lane| s.backrow(player, lane))
            .map(|card| card.def_id.clone())
            .collect();
        defs.sort();
        defs
    }

    #[test]
    fn r3538007_base_destroys_every_lower_card_on_both_sides_and_rows() {
        crate::register_all();
        register_win_rates(table(records()));
        let mut s = board(false, "kpis");
        s.play(ID, json!({}));
        for player in [P1, P2] {
            // The record-less stand-in counts as half, below KPIs' 60%, so it goes with the low.
            assert_eq!(unit_defs(&s, player), vec![HIGH.to_string()], "{player:?}");
            assert_eq!(backrow_defs(&s, player), vec![HIGH_BACKROW.to_string()], "{player:?}");
        }
        clear_overrides();
    }

    #[test]
    fn r3538007_radiant_spares_your_own() {
        crate::register_all();
        register_win_rates(table(records()));
        let mut s = board(true, "kpis-radiant");
        s.play(ID, json!({}));
        assert_eq!(unit_defs(&s, P1), vec![HIGH.to_string(), LOW.to_string(), EVEN.to_string()]);
        assert_eq!(backrow_defs(&s, P1), vec![HIGH_BACKROW.to_string(), LOW_BACKROW.to_string()]);
        assert_eq!(unit_defs(&s, P2), vec![HIGH.to_string()]);
        assert_eq!(backrow_defs(&s, P2), vec![HIGH_BACKROW.to_string()]);
        clear_overrides();
    }

    #[test]
    fn r3538000_a_card_with_no_record_counts_as_half_and_so_does_kpis() {
        crate::register_all();
        // KPIs holds no row, so it counts as 50%: 49% goes, 51% and the record-less stay.
        register_win_rates(table(vec![(LOW, 490, 1000), (HIGH, 510, 1000)]));
        let mut s = board(false, "kpis-half");
        s.play(ID, json!({}));
        for player in [P1, P2] {
            assert_eq!(unit_defs(&s, player), vec![HIGH.to_string(), EVEN.to_string()], "{player:?}");
        }
        clear_overrides();
    }
}
