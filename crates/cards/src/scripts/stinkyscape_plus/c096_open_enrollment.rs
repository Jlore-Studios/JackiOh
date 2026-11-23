//! S+ #96 Open Enrollment (SPEC §8.12 row 96). (0) embiggen (2) Spell, Common.
//!   Base:    "Fill your hand with random F tier cards (or D tier if none exist).
//!            Embiggened (2): They cost (1) less."
//!   Radiant: "Fill your hand with random Radiant F tier cards (or D tier if none exist).
//!            Embiggened (2): They cost (0)."
//!
//! Cry (R1): add F-tier cards (D-tier when no F card exists, R3538001) until the hand is full,
//! Radiant on the Radiant face. Issue #4353 gave it its Embiggened price, (2) (R1175000, the play's
//! `embiggen`, which the Cry reads as `ctx.embiggened`): played so, every card it adds costs `DISCOUNT`
//! less, a `costMod` on each (R65, never below (0)), or on the Radiant face costs `EMBIGGENED_COST`, a
//! `costOverride` (R4353023); at its base price it fills the hand as before. Through v0.6.8b it had no
//! Embiggened price, so no play of it could be embiggened and its script needs no keeping (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-096";

/// "Embiggened (2): They cost (1) less", the base face's (R1748003: a price, printed, declared nowhere).
const DISCOUNT: i32 = 1;
/// "Embiggened (2): They cost (0)", the Radiant face's (R1748003: a price, printed, declared nowhere).
const EMBIGGENED_COST: i32 = 0;

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
                let mut args = json!({
                    "query": { "tier": tier },
                    "count": room,
                    "radiant": radiant,
                });
                // R4353023: Embiggened, each card added is cheaper: (1) less, or (0) on the Radiant face.
                if ctx.embiggened {
                    if radiant {
                        args["costOverride"] = json!(EMBIGGENED_COST);
                    } else {
                        args["costMod"] = json!(-DISCOUNT);
                    }
                }
                effects.push(add_random_from_catalog(json_as(args)));
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

    /// R4353023: at its base price neither face touches the cost of the cards it adds.
    #[test]
    fn r4353023_at_its_base_price_the_cards_keep_their_cost() {
        crate::register_all();
        register_win_rates(table(vec![(FILLER, 200, 1000), ("core-008", 460, 1000)]));
        for radiant in [false, true] {
            let mut s = board(radiant, &format!("open-enrollment-price-{radiant}"));
            s.play(ID, json!({}));
            let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
            assert!(!added.is_empty());
            assert!(
                added.iter().all(|card| card.cost_mod == 0 && card.cost_override.is_none()),
                "radiant {radiant}"
            );
        }
        clear_overrides();
    }

    /// R4353023: Embiggened (2), the base face's cards each cost (1) less, a `costMod`.
    #[test]
    fn r4353023_embiggened_the_base_face_s_cards_cost_1_less() {
        crate::register_all();
        register_win_rates(table(vec![(FILLER, 200, 1000), ("core-008", 460, 1000)]));
        let mut s = board(false, "open-enrollment-embiggened");
        s.play(ID, json!({ "embiggen": true }));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::F)));
        assert!(added.iter().all(|card| card.cost_mod == -DISCOUNT && card.cost_override.is_none()));
        clear_overrides();
    }

    /// R4353023: Embiggened (2), the Radiant face's cards each cost (0), a `costOverride`.
    #[test]
    fn r4353023_embiggened_the_radiant_face_s_cards_cost_0() {
        crate::register_all();
        register_win_rates(table(vec![(FILLER, 200, 1000), ("core-008", 460, 1000)]));
        let mut s = board(true, "open-enrollment-embiggened-radiant");
        s.play(ID, json!({ "embiggen": true }));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(added.iter().all(|card| card.radiant));
        assert!(added.iter().all(|card| card.cost_override == Some(EMBIGGENED_COST)));
        clear_overrides();
    }
}
