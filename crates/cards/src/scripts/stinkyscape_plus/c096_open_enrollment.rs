//! S+ #96 Open Enrollment (SPEC §8.12 row 96). (2) Spell, Common.
//!   Base:    "Fill your hand with random F tier cards (or D tier if none exist)."
//!   Radiant: "Fill your hand with random Radiant F tier cards (or D tier if none exist)."
//!
//! Cry (R1): add F-tier cards (D-tier when no F card exists, R3538001) until the hand is full,
//! Radiant on the Radiant face.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-096";

/// R3538001: `first` unless its pool is empty, then `then`.
fn tier_or(first: CardTier, then: CardTier) -> CardTier {
    if query(&json_as::<CatalogQueryArgs>(json!({ "tier": first }))).is_empty() {
        then
    } else {
        first
    }
}

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let tier = tier_or(CardTier::F, CardTier::D);
            let mut effects = Vec::new();
            let room = room_in(ctx.state, ctx.controller, OffFieldZone::Hand);
            if room > 0 {
                effects.push(add_random_from_catalog(json_as(json!({
                    "query": { "tier": tier },
                    "count": room,
                    "radiant": radiant,
                }))));
            }
            effects
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

// S+ #96 Open Enrollment — SPEC §8.12 row 96 (R3538001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a Spell

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

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [{ "def": "core-008", "lane": 1 }],
                "library": filler(8),
                "mana": 2,
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": "core-008", "lane": 1 }],
                "library": filler(8),
            },
        }))
    }

    #[test]
    fn r3538001_base_fills_with_f_tier_else_d_tier() {
        crate::register_all();
        // (200, 1000) is F tier and (460, 1000) D tier of record.
        register_win_rates(table(vec![(FILLER, 200, 1000), ("core-008", 460, 1000)]));
        let mut s = board(false, "open-enrollment");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::F)),
            "every card filled is F tier"
        );
        assert!(added.iter().all(|card| !card.radiant));
        clear_overrides();

        register_win_rates(table(vec![("core-008", 460, 1000), (FILLER, 465, 1000)]));
        let mut s = board(false, "open-enrollment-d-tier");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::D)),
            "with no F tier, every card filled is D tier"
        );
        clear_overrides();
    }

    #[test]
    fn r3538001_radiant_fills_with_radiant_f_tier() {
        crate::register_all();
        register_win_rates(table(vec![(FILLER, 200, 1000), ("core-008", 460, 1000)]));
        let mut s = board(true, "open-enrollment-radiant");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::F)),
            "every card filled is F tier"
        );
        assert!(added.iter().all(|card| card.radiant), "all of them Radiant");
        clear_overrides();
    }
}
