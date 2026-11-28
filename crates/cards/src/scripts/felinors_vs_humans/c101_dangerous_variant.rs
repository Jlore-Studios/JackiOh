//! SPEC §8.14 FvH #101 Dangerous Variant ((2) Unit, Human, Common, 1/1 → 2/2), a card of the Felinors
//! vs Humans set (R5235000).
//!   Base:    "Start of turn: Transform this into a random (4) Cost Unit."
//!   Radiant: "Divine Shield / Start of turn: Transform this into a random Radiant (4) Cost Unit."
//!
//! At the start of its controller's turn, on the field (R62): `transform_random` of itself (§6.3, C+ #73.1
//! Classic Golem's verb) into a random non-token Unit of the sets that ship whose printed cost is (4)
//! (R60, R65), on its Radiant face for the Radiant Variant. The new body is a new arrival, summoning sick
//! that turn (R659), with no Cry (R1). Immutable refuses it (R23), and a pool with nothing that could take
//! its place changes nothing (R129). The (4) is the pool's price, no number the card tunes. Divine Shield
//! is catalog data. Built by S11 (#5261).

use jackioh_engine::effects::transform_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-101";

/// The printed cost of the Unit it becomes: the pool's filter.
const NEW_COST: i32 = 4;

/// "Start of turn: Transform this": `radiant` is the new body's face.
fn variant(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![transform_random(json_as(json!({
                "target": { "of": "self" },
                "query": { "type": "Unit", "cost": NEW_COST },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: variant(false),
        radiant: variant(true),
    }
}

// FvH #101 Dangerous Variant — SPEC §8.14 row 101.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// Over a few seeds, the Variant is itself through the opponent's start of turn and a random
    /// non-token (4) Unit on `radiant`'s face after its own.
    fn becomes_a_4_cost_unit(radiant: bool) {
        for seed in 0..6 {
            let seed = format!("dangerous-variant-{radiant}-{seed}");
            let mut s = board(radiant, &seed);
            s.end_turn();
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "{seed}: the opponent's start of turn leaves it be");
            s.end_turn();
            let ours = s.unit_at(P1, 1);
            assert_ne!(ours.def_id, ID, "{seed}: it transformed at the start of its controller's turn");
            let def = def_of(Some(s.state()), &ours.def_id);
            assert_eq!(def.type_, CardType::Unit, "{}", def.id);
            assert_eq!(query_cost(def), NEW_COST, "{}", def.id);
            assert!(!def.is_token(), "{}", def.id);
            assert_eq!(ours.radiant, radiant, "{}", def.id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn at_the_start_of_your_turn_it_transforms_into_a_random_4_cost_unit() {
            becomes_a_4_cost_unit(false);
        }

        #[test]
        fn it_has_no_divine_shield() {
            let s = board(false, "dangerous-variant-shield");
            assert!(!has_keyword(&s.stats(&s.unit_at(P1, 1)).keywords, KeywordKind::DivineShield));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn at_the_start_of_your_turn_it_transforms_into_a_random_radiant_4_cost_unit() {
            becomes_a_4_cost_unit(true);
        }

        #[test]
        fn it_has_divine_shield_on_the_field() {
            let s = board(true, "dangerous-variant-radiant-shield");
            assert!(has_keyword(&s.stats(&s.unit_at(P1, 1)).keywords, KeywordKind::DivineShield));
        }
    }
}
