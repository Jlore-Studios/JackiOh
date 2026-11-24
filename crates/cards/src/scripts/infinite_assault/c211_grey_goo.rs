//! IA #211 Grey Goo (SPEC §8.13 row 211), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "End of turn: Transform a random adjacent Unit into a copy of this."
//!   Radiant: "End of turn: Transform two random adjacent Units into copies of this."
//!
//! Engine: an `end_of_turn` hook (its controller's, R68): the same-side Units in lanes ±1
//! (`adjacent_to`, R4593007), Immutable neighbours no candidates; the base face rewrites one of
//! them into R57's copy of the Goo as it stands (`transform_into_copy`), drawn uniformly through
//! the match rng only between two, and the Radiant face every candidate, at most two. Built by C9
//! (#4593).

use jackioh_engine::effects::{BoardScope, transform_into_copy};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-211";

/// The base face rewrites one neighbour; the Radiant face every candidate (at most two).
const BASE_PICKS: i32 = 1;
const RADIANT_PICKS: i32 = 2;

fn goo(picks: i32) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let neighbours: Vec<CardInstance> = adjacent_to(
                ctx,
                &json_as(json!({ "of": "self" })),
                &BoardScope::default(),
            )
            .into_iter()
            .filter(|card| !jackioh_engine::layers::is_immutable(ctx.state, card))
            .collect();
            // R4593007: every candidate taken draws no rng; one of two is a uniform pick.
            let ids: Vec<String> = if neighbours.len() as i32 <= picks {
                neighbours.iter().map(|card| card.id.clone()).collect()
            } else {
                match ctx.rng.pick(&neighbours).cloned() {
                    Some(picked) => vec![picked.id],
                    None => vec![],
                }
            };
            ids.into_iter()
                .map(|id| transform_into_copy(json_as(json!({ "instanceId": id }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: goo(BASE_PICKS),
        radiant: goo(RADIANT_PICKS),
    }
}

// IA #211 Grey Goo — SPEC §8.13 row 211 (R4593007).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const LEFT: &str = "stand-in (1) 4/4";
    const RIGHT: &str = "stand-in (1) 4/5";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn spreading(radiant: bool, lanes: Vec<Value>, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": lanes,
                "hand": [FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.end_turn();
        s
    }

    fn goo_count(s: &Scenario) -> usize {
        [1, 2, 3]
            .into_iter()
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == ID)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4593007_one_neighbour_becomes_a_goo() {
            let s = spreading(
                false,
                vec![
                    json!(LEFT),
                    json!({ "def": ID, "lane": 2 }),
                    json!(RIGHT),
                ],
                "goo-base",
            );
            assert_eq!(goo_count(&s), 2, "the Goo and exactly one convert");
        }

        #[test]
        fn r4593007_a_lone_goo_does_nothing() {
            let s = spreading(false, vec![json!({ "def": ID, "lane": 2 })], "goo-lone");
            assert_eq!(goo_count(&s), 1);
            assert!(
                s.events()
                    .iter()
                    .all(|event| !matches!(event, GameEvent::Transformed { .. })),
                "nothing is rewritten"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593007_both_neighbours_become_goos() {
            let s = spreading(
                true,
                vec![
                    json!(LEFT),
                    json!({ "def": ID, "lane": 2, "radiant": true }),
                    json!(RIGHT),
                ],
                "goo-radiant",
            );
            assert_eq!(goo_count(&s), 3);
        }
    }
}
