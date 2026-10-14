//! #34 Collateral Damage (SPEC §8.2 row 34): "Exile target permanent and a random card from the
//! opponent's library", radiant "Also the permanents adjacent to the target in its row".
//!
//! "Also …" adds a clause and keeps every base clause (§8 Conventions), so the radiant form exiles
//! the target, its two neighbours and a library card. Exile (§6.3) bypasses Indestructible and bumps
//! the exile counter in the engine's verb (R55); a unit token ceases to exist instead (R11).
//!
//! R81: "target permanent" is a unit or a backrow card on either side, declared here and carried in
//! the `play` action; resolution never pauses.
//!
//! §3.1: adjacent is lane N−1 and N+1 on the SAME side and row, no wrap (`zones.adjacent`). Exile
//! moves the card at once, so the neighbours go FIRST, while the target still stands in its lane.

use jackioh_engine::effects::{exile, exile_adjacent_to, exile_random_from_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-034";

/// R81: "target permanent" — a unit or a backrow card, either side (§8 Conventions, §5.1).
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| {
                vec![
                    exile(json_as(json!({ "target": { "of": "chosen" } }))),
                    exile_random_from_library(json_as(json!({ "count": 1, "player": "enemy" }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| {
                vec![
                    // First, while the target is still in its lane for `adjacent` to read (see the header).
                    exile_adjacent_to(json_as(json!({ "target": { "of": "chosen" } }))),
                    exile(json_as(json!({ "target": { "of": "chosen" } }))),
                    exile_random_from_library(json_as(json!({ "count": 1, "player": "enemy" }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// #34 Collateral Damage — SPEC §8.2 row 34, BUILD M4-T4 must-pass row 34:
// "Exiles an Indestructible permanent and a random opponent library card; radiant same-row
//  neighbours too".
// #66 The Rock is a 10/10 with printed Indestructible, so it is the must-pass's Indestructible
// permanent (placed by the setup, so its Tribute cost is not in the way).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// Registers the catalog first: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn must(card: Option<CardInstance>, what: &str) -> CardInstance {
        card.unwrap_or_else(|| panic!("the scenario has no {what}"))
    }

    fn at(instance: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": instance.id }])
    }

    mod base {
        use super::*;

        #[test]
        fn sec6_3_exile_bypasses_indestructible_the_rock_goes_to_exile() {
            let mut s = scn(json!({
                "seed": "collateral",
                "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                "p2": { "field": ["66"], "library": ["15", "43", "13"] },
            }));
            let rock = must(s.unit("p2", 1), "p2 lane 1");

            s.play("34", json!({ "targets": at(&rock) }));

            s.expect_in_zone(&rock, "exile");
            s.expect_events(json!(["cardPlayed", "exiled"]));
        }

        #[test]
        fn r55_the_exile_counter_counts_the_permanent() {
            let mut s = scn(json!({
                "seed": "collateral-counter",
                "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                "p2": { "field": ["66"], "library": ["15", "43", "13"] },
            }));
            assert_eq!(s.state().counters.exiled, 0);

            let rock = must(s.unit("p2", 1), "p2 lane 1");
            s.play("34", json!({ "targets": at(&rock) }));

            // One for the permanent, one for the library card.
            assert_eq!(s.state().counters.exiled, 2);
        }

        #[test]
        fn a_backrow_permanent_is_a_legal_target_too() {
            let mut s = scn(json!({
                "seed": "collateral-backrow",
                "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                "p2": { "backrow": ["18"], "field": ["15"], "library": ["15", "43", "13"] },
            }));
            let trap = must(s.backrow("p2", 1), "p2 backrow 1");

            s.play("34", json!({ "targets": at(&trap) }));

            s.expect_in_zone(&trap, "exile");
        }

        #[test]
        fn it_also_exiles_one_random_card_from_the_opponents_library() {
            let library = json!(["15", "43", "13"]);
            let run = || -> Scenario {
                let mut s = scn(json!({
                    "seed": "collateral-library",
                    "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                    "p2": { "field": ["66"], "library": library },
                }));
                let rock = must(s.unit("p2", 1), "p2 lane 1");
                s.play("34", json!({ "targets": at(&rock) }));
                s
            };

            let s = run();
            assert_eq!(s.pile("p2", "library").len(), 2);
            // The permanent and exactly one library card.
            assert_eq!(s.pile("p2", "exile").len(), 2);

            // §9.3: the pick comes from the seeded rng, so the same seed exiles the same card every time.
            let exiled = |scene: &Scenario| -> Vec<String> {
                scene.pile("p2", "library").into_iter().map(|card| card.def_id).collect()
            };
            assert_eq!(exiled(&run()), exiled(&s));
        }

        #[test]
        fn an_empty_opponent_library_costs_it_nothing() {
            let mut s = scn(json!({
                "seed": "collateral-empty-library",
                "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                "p2": { "field": ["66"], "library": [] },
            }));
            let rock = must(s.unit("p2", 1), "p2 lane 1");

            s.play("34", json!({ "targets": at(&rock) }));

            s.expect_in_zone(&rock, "exile");
            assert_eq!(s.pile("p2", "exile").len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn sec3_1_also_the_permanents_adjacent_to_the_target_in_its_row() {
            let mut s = scn(json!({
                "seed": "collateral-radiant",
                "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                "p2": { "field": ["15", "66", "43"], "library": ["15", "43", "13"] },
            }));
            s.card_mut("34").radiant = true;
            let left = must(s.unit("p2", 1), "p2 lane 1");
            let middle = must(s.unit("p2", 2), "p2 lane 2");
            let right = must(s.unit("p2", 3), "p2 lane 3");

            s.play("34", json!({ "targets": at(&middle) }));

            s.expect_in_zone(&left, "exile");
            s.expect_in_zone(&middle, "exile");
            s.expect_in_zone(&right, "exile");
        }

        #[test]
        fn sec3_1_adjacency_does_not_wrap_and_never_crosses_sides() {
            let mut s = scn(json!({
                "seed": "collateral-radiant-edge",
                "p1": { "hand": ["34"], "field": [{ "def": "15", "lane": 1 }], "library": ["15"] },
                "p2": {
                    "field": [{ "def": "66", "lane": 1 }, { "def": "15", "lane": 2 }, { "def": "43", "lane": 5 }],
                    "library": ["15", "43", "13"],
                },
            }));
            s.card_mut("34").radiant = true;
            let edge = must(s.unit("p2", 1), "p2 lane 1");
            let neighbour = must(s.unit("p2", 2), "p2 lane 2");
            let faraway = must(s.unit("p2", 5), "p2 lane 5");
            let ally = must(s.unit("p1", 1), "p1 lane 1");

            s.play("34", json!({ "targets": at(&edge) }));

            s.expect_in_zone(&edge, "exile");
            s.expect_in_zone(&neighbour, "exile");
            // Lane 1 has no lane 0, so nothing wraps round to lane 5, and p1's own lane 1 is another side.
            s.expect_in_zone(&faraway, "field");
            s.expect_in_zone(&ally, "field");
        }

        #[test]
        fn the_radiant_form_keeps_the_base_library_clause_also() {
            let mut s = scn(json!({
                "seed": "collateral-radiant-library",
                "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                "p2": { "field": ["66"], "library": ["15", "43", "13"] },
            }));
            s.card_mut("34").radiant = true;

            let rock = must(s.unit("p2", 1), "p2 lane 1");
            s.play("34", json!({ "targets": at(&rock) }));

            assert_eq!(s.pile("p2", "library").len(), 2);
        }

        #[test]
        fn the_radiant_form_still_exiles_the_target_itself() {
            let mut s = scn(json!({
                "seed": "collateral-radiant-target",
                "p1": { "hand": ["34"], "field": ["15"], "library": ["15"] },
                "p2": { "field": ["66"], "library": ["15", "43", "13"] },
            }));
            s.card_mut("34").radiant = true;
            let rock = must(s.unit("p2", 1), "p2 lane 1");

            s.play("34", json!({ "targets": at(&rock) }));

            s.expect_in_zone(&rock, "exile");
        }
    }
}
