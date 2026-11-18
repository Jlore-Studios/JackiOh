//! S+ #97 Selective Enrollment (SPEC §8.12 row 97). (5) Spell, Common.
//!   Base:    "Float reduces cost.
//!            Fill your hand with random Radiant S tier cards (or A tier if none exist)."
//!   Radiant: "Float reduces cost.
//!            Fill your hand with random Radiant S tier cards (or A tier if none exist). They cost
//!            (2) less."
//!
//! Cry (R1): add Radiant S-tier cards (Radiant A-tier when no S card exists, R3538001) until the
//! hand is full. The Radiant face's cards cost (2) less (`costMod`, a price, R1748003).
//! "Float reduces cost" as S #35 Drone Core IX's (R1563000): a hand trigger on its owner's Float,
//! never in the deck (R2657002), a `costMod` kept until a graveyard or exile (R766).
//! Kept for replays (R767000): through v0.6.6g it cost (4) and had no Float (`script_v0_6_6g`,
//! issue #4154).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-097";

/// The Radiant face's discount: what each card filled costs less (a price, R1748003).
const DISCOUNT: i32 = 2;

/// R3538001: `first` unless its pool is empty, then `then`.
fn tier_or(first: CardTier, then: CardTier) -> CardTier {
    if query(&json_as::<CatalogQueryArgs>(json!({ "tier": first }))).is_empty() {
        then
    } else {
        first
    }
}

/// One face. `float`: whether "Float reduces cost" is printed (since issue #4154), a hand trigger.
fn face(radiant: bool, float: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let tier = tier_or(CardTier::S, CardTier::A);
            let mut effects = Vec::new();
            let room = room_in(ctx.state, ctx.controller, OffFieldZone::Hand);
            if room > 0 {
                let mut args = json!({
                    "query": { "tier": tier },
                    "count": room,
                    "radiant": true,
                });
                if radiant {
                    args["costMod"] = json!(-DISCOUNT);
                }
                effects.push(add_random_from_catalog(json_as(args)));
            }
            effects
        })),
        hand_triggers: if float {
            vec![float_reduces_cost("selective-enrollment-float-hand")]
        } else {
            vec![]
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false, true),
        radiant: face(true, true),
    }
}

/// R767000: through v0.6.6g it had no Float; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    CardScripts {
        base: face(false, false),
        radiant: face(true, false),
    }
}

// S+ #97 Selective Enrollment — SPEC §8.12 row 97 (R3538001, R1563000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::stinkyscape_family as fam;
    use crate::{filler, scenario};

    /// Its printed cost since issue #4154.
    const PRINTED: i32 = 5;

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
                "mana": PRINTED,
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": "core-008", "lane": 1 }],
                "library": filler(8),
            },
        }))
    }

    #[test]
    fn r3538001_base_fills_with_radiant_s_tier_else_a_tier() {
        crate::register_all();
        // (800, 1000) is S tier and (550, 1000) A tier of record.
        register_win_rates(table(vec![(FILLER, 800, 1000), ("core-008", 550, 1000)]));
        let mut s = board(false, "selective-enrollment");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::S)),
            "every card filled is S tier"
        );
        assert!(added.iter().all(|card| card.radiant), "all of them Radiant");
        assert!(added.iter().all(|card| card.cost_mod == 0), "no discount");
        clear_overrides();

        register_win_rates(table(vec![("core-008", 550, 1000), (FILLER, 540, 1000)]));
        let mut s = board(false, "selective-enrollment-a-tier");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::A)),
            "with no S tier, every card filled is A tier"
        );
        assert!(added.iter().all(|card| card.radiant), "still Radiant");
        clear_overrides();
    }

    #[test]
    fn r3538001_radiant_cards_cost_two_less() {
        crate::register_all();
        register_win_rates(table(vec![(FILLER, 800, 1000), ("core-008", 550, 1000)]));
        let mut s = board(true, "selective-enrollment-radiant");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::S)),
            "every card filled is S tier"
        );
        assert!(added.iter().all(|card| card.radiant), "all of them Radiant");
        assert!(
            added.iter().all(|card| card.cost_mod == -DISCOUNT),
            "each costs (2) less"
        );
        clear_overrides();
    }

    /// R1563000, R2657002: since issue #4154 a Float of 2 makes it cost two less in hand, never in the
    /// deck, on both faces.
    #[test]
    fn r1563000_a_float_lowers_its_cost_in_hand_never_in_the_deck() {
        fam::drone_float_two_less(ID, PRINTED);
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        fam::drone_float_opponent(ID, PRINTED);
    }

    /// R767000: through v0.6.6g it had no Float.
    #[test]
    fn r767000_the_script_kept_for_v0_6_6g_has_no_float() {
        let kept = script_v0_6_6g();
        assert!(kept.base.hand_triggers.is_empty());
        assert!(kept.radiant.hand_triggers.is_empty());
        let current = script();
        assert_eq!(current.base.hand_triggers.len(), 1);
        assert_eq!(current.radiant.hand_triggers.len(), 1);
    }
}
