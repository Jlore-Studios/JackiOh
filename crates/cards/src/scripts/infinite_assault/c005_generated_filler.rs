//! SPEC §8.13 IA #5 Generated Filler ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Add two random (1)-Cost Infinite Assault Units to your hand. They're Temporary."
//!   Radiant: "Add two random (1)-Cost Infinite Assault Units to your hand. They cost ({setCost}) and are
//!             Temporary." (setCost 0)
//!
//! C+ #54 Book of Books' adds over a set-named pool of (1)-Cost Units (R1846001, R4577000; R387 and §5.1 leave
//! this card and the tokens out), Temporary (R637); the Radiant face's price is its declared `setCost` (R386). The
//! (1) is a price the pool's filter names (R1748003).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-005";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// "two random … Units".
const UNITS: i32 = 2;

/// "(1)-Cost": the printed price the pool keeps to.
const PRICE: i32 = 1;

fn filler(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "query": { "set": SET, "type": "Unit", "cost": PRICE },
                "count": UNITS,
                "temporary": true,
            });
            if radiant {
                args["costOverride"] = json!(param(&*ctx, "setCost"));
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: filler(false),
        radiant: filler(true),
    }
}

// IA #5 Generated Filler — SPEC §8.13 row 5.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn added(radiant: bool, seed: &str) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({ "seed": seed, "p1": { "hand": [{ "def": ID, "radiant": radiant }] } }));
        s.play(ID, json!({}));
        let hand = s.hand(P1);
        (s, hand)
    }

    fn holds_two_temporary_one_cost_units(radiant: bool) {
        for seed in 0..8 {
            let (s, hand) = added(radiant, &format!("generated-filler-{seed}"));
            assert_eq!(hand.len(), 2, "seed {seed}");
            for card in nonempty(hand, "the added Units") {
                let def = def_of(Some(s.state()), &card.def_id);
                assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
                assert_eq!(def.type_, CardType::Unit, "{}", def.id);
                assert_eq!(query_cost(def), 1, "{}", def.id);
                assert!(!def.is_token(), "{}", def.id);
                assert_eq!(card.granted_keywords, vec![Keyword::TEMPORARY]);
                assert!(!card.radiant);
                let expected = if radiant { 0 } else { 1 };
                assert_eq!(cost_now(s.state(), &card), expected, "{}", def.id);
            }
        }
    }

    mod base {
        use super::*;

        #[test]
        fn adds_two_temporary_one_cost_infinite_assault_units() {
            holds_two_temporary_one_cost_units(false);
        }

        #[test]
        fn r637_they_leave_the_hand_at_the_end_of_the_turn() {
            let (mut s, hand) = added(false, "generated-filler");
            s.end_turn();
            for card in nonempty(hand, "the added Units") {
                assert_eq!(s.card(&card.id).zone.z(), ZoneName::Graveyard);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_two_temporary_one_cost_units_that_cost_0() {
            holds_two_temporary_one_cost_units(true);
        }
    }
}
