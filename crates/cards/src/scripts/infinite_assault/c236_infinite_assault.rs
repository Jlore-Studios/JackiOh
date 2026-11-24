//! SPEC §8.13 IA #236 Infinite Assault ((4) Spell, Legendary), a card of the held Infinite Assault
//! set (R4577000).
//!   Base:    "Summon a random Infinite Assault Unit in each empty unit zone of yours. They have
//!            Rush."
//!   Radiant: "Summon a random Infinite Assault Unit in each empty unit zone of yours. They're
//!            Radiant and have Rush."
//!
//! One `summonRandom` per open unit zone from the set-named pool (R1846001, R4577000), each with
//! Rush — so they attack at once — Radiant ones on the Radiant face. Occupied zones are kept; with
//! none open nothing is drawn (R129).
//!
//! Engine: `summonRandom` with `keywords` (R3570006). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-236";

/// The set the pool names (R4577000).
const SET: &str = "Infinite Assault";

fn assault(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            fill_board_zones(&*ctx.state, ctx.controller)
                .into_iter()
                .map(|slot| {
                    summon_random_with(json_as(json!({
                        "query": { "set": SET, "type": "Unit" },
                        "lane": slot.lane,
                        "radiant": radiant,
                        "keywords": [{ "kind": "Rush" }],
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: assault(false), radiant: assault(true) }
}

// IA #236 Infinite Assault — SPEC §8.13 row 236.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const KEEP: &str = "core-008"; // Mr. Vanilla: holds lane 1.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id != KEEP)
            .cloned()
            .collect()
    }

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE], "field": [KEEP], "library": crate::filler(4) },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn expect_rush_ia(s: &Scenario, radiant: bool, count: usize) {
        let made = summoned(s);
        assert_eq!(made.len(), count, "one per open zone");
        for card in nonempty(made, "the summoned Units") {
            let def = def_of(Some(s.state()), &card.def_id);
            assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
            assert_eq!(card.radiant, radiant, "{}", def.id);
            let view = unit_view(s.state(), &card);
            assert!(view.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));
        }
        // Lane 1 keeps its Unit.
        assert_eq!(s.unit_at(P1, 1).def_id, KEEP);
    }

    mod base {
        use super::*;

        #[test]
        fn the_rush_units_attack_at_once_occupied_lanes_kept() {
            let s = cast(false, "assault-base");
            expect_rush_ia(&s, false, 4);
            // A fresh summon with Rush lists attacks at once; a sick Unit lists none.
            let first = summoned(&s)[0].id.clone();
            assert!(!s.attacks_of(&first).is_empty(), "Rush attacks at once");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_copies_are_radiant_and_have_rush() {
            let s = cast(true, "assault-radiant");
            expect_rush_ia(&s, true, 4);
        }
    }
}
