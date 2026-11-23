//! SPEC §8.13 IA #20 Synthetic Data ((2) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Fill your hand with random Infinite Assault cards. They're Temporary."
//!   Radiant: "Fill your hand with random Infinite Assault cards. They're Temporary and cost ({discount})
//!             less." (discount 1)
//!
//! M #77 Bulk Booster's fill (the room its hand has as it resolves, R4) from the set-named pool
//! (R1846001, R4577000; this card and the tokens left out, R387, §5.1), each Temporary (R637); the Radiant face's adds carry
//! `costMod` −`discount` (a price never below (0), R65).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-020";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

fn synthetic_data(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let room = room_in(ctx.state, ctx.controller, OffFieldZone::Hand);
            if room == 0 {
                return vec![];
            }
            let mut args = json!({ "query": { "set": SET }, "count": room, "temporary": true });
            if radiant {
                args["costMod"] = json!(-param(&*ctx, "discount"));
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: synthetic_data(false),
        radiant: synthetic_data(true),
    }
}

// IA #20 Synthetic Data — SPEC §8.13 row 20.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const KEPT: &str = "stand-in (0) Spell";

    /// Plays the Data beside two cards kept in hand, and returns the cards it added.
    fn filled(radiant: bool) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "seed": "synthetic-data",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, KEPT, KEPT] },
        }));
        s.play(ID, json!({}));
        let added = s.hand(P1).into_iter().filter(|card| card.def_id != KEPT).collect();
        (s, added)
    }

    fn expect_filled(radiant: bool) {
        let (s, added) = filled(radiant);
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        assert_eq!(added.len(), HAND_CAP as usize - 2);
        for card in nonempty(added, "the added cards") {
            let def = def_of(Some(s.state()), &card.def_id);
            assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
            assert!(!def.is_token(), "{}", def.id);
            assert_eq!(card.granted_keywords, vec![Keyword::TEMPORARY]);
            assert_eq!(card.cost_mod, if radiant { -1 } else { 0 });
        }
    }

    mod base {
        use super::*;

        #[test]
        fn fills_the_hand_with_temporary_infinite_assault_cards() {
            expect_filled(false);
        }

        #[test]
        fn r637_they_leave_the_hand_at_the_end_of_the_turn() {
            let (mut s, added) = filled(false);
            s.end_turn();
            for card in nonempty(added, "the added cards") {
                assert_eq!(s.card(&card.id).zone.z(), ZoneName::Graveyard);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn fills_the_hand_with_temporary_cards_that_cost_1_less() {
            expect_filled(true);
        }
    }
}
