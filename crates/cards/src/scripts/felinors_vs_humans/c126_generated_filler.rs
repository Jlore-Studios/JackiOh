//! SPEC §8.14 FvH #126 Plagiarized: Generated Filler, a frozen copy of IA #5 Generated Filler (R2195003,
//! FV4), a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Generated Filler`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #5: its pool is the Plagiarized Units printed at (1), not the Infinite Assault ones. The IA
//! file's notes, as they stood:
//!
//! SPEC §8.13 IA #5 Generated Filler ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Add two random (1) Cost Infinite Assault Units to your hand. They're Temporary."
//!   Radiant: "Add two random (1) Cost Infinite Assault Units to your hand. They cost ({setCost}) and are
//!             Temporary." (setCost 0)
//!
//! C+ #54 Book of Books' adds over a set-named pool of (1) Cost Units (R1846001, R4577000; R387 and §5.1 leave
//! this card and the tokens out), Temporary (R637); the Radiant face's price is its declared `setCost` (R386). The
//! (1) is a price the pool's filter names (R1748003).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-126";

/// FV4: the Plagiarized cards' set, which the pool names while it is held (a pool that names no set
/// holds only the sets that ship, R380, R1420); every Plagiarized card is of it.
const SET: &str = "Felinors vs Humans";

/// FV4: "Plagiarized Units", the tag the pool keeps to (IA #5's pool is the Infinite Assault set's).
const TAG: &str = "Plagiarized";

/// "two random … Units".
const UNITS: i32 = 2;

/// "(1) Cost": the printed price the pool keeps to.
const PRICE: i32 = 1;

fn filler(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "query": { "set": SET, "tags": [TAG], "type": "Unit", "cost": PRICE },
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

// FvH #126 Plagiarized: Generated Filler — SPEC §8.14 row 126; IA #5's tests (SPEC §8.13 row 5), on this
// card.
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
                assert_eq!(def.set, SetName::FelinorsVsHumans, "{}", def.id);
                assert!(def.tags.contains(&Tag::Plagiarized), "{}", def.id);
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
        fn adds_two_temporary_one_cost_plagiarized_units() {
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
