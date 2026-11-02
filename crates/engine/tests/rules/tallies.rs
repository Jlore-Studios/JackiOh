//! R2321000 (issue #2321): a count a card on the field keeps on a Unit (`Script.tallies`, TG #31
//! Level Up!'s attacks toward its Radiant) rides on that Unit's view as `UnitView.tallies`, `value` of
//! `goal` and the keeping card by its id, on both seats when the keeping card is public and on its
//! controller's alone while it lies face-down (R33, §10.8). The client draws it and works none of it
//! out (CLAUDE.md rule 7).
//!
//! The keepers are test-only, on top of the fixture catalog: a Field Spell and a Trap whose hook reads
//! the counts a test writes into their memory, `{ "<unit id>": n }`, each toward `GOAL`.

mod r2321000_a_count_kept_on_a_unit_rides_on_its_view {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::rules::fixtures::combat::plain;
    use crate::rules::fixtures::harness::setup_catalog;

    // Indexed from 23210, colliding with no other file's catalog (BUILD §0).
    const FIELD_KEEPER: &str = "tally-field-keeper";
    const TRAP_KEEPER: &str = "tally-trap-keeper";
    const GOAL: i32 = 3;
    const COUNTS: &str = "counts";

    fn def(id: &str, index: u32, card_type: &str) -> CardDef {
        json_as(json!({
            "id": id,
            "index": index.to_string(),
            "name": format!("{id} (tallies fixture)"),
            "set": "Core",
            "type": card_type,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": id },
            "radiant": { "keywords": [], "text": id },
        }))
    }

    fn keeper() -> CardScripts {
        let script = Script {
            tallies: Some(condition_hook(|ctx: ConditionContext<'_>| {
                ctx.self_
                    .memory
                    .get(COUNTS)
                    .and_then(Value::as_object)
                    .map(|counts| {
                        counts
                            .iter()
                            .map(|(id, n)| UnitTally {
                                instance_id: id.clone(),
                                value: n.as_i64().unwrap_or(0) as i32,
                                goal: GOAL,
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })),
            ..Script::default()
        };
        CardScripts {
            base: script.clone(),
            radiant: script,
        }
    }

    /// p1 holds `p1`'s field and backrow, p2 one plain Unit in lane 1. It is p1's main phase.
    fn board(seed: &str, p1: Value) -> Scenario {
        setup_catalog();
        let mut catalog = registered_catalog().clone();
        catalog.insert(FIELD_KEEPER.to_string(), def(FIELD_KEEPER, 23210, "Field Spell"));
        catalog.insert(TRAP_KEEPER.to_string(), def(TRAP_KEEPER, 23211, "Trap"));
        register_catalog(catalog);
        let mut scripts = registered_scripts().clone();
        scripts.insert(FIELD_KEEPER.to_string(), keeper());
        scripts.insert(TRAP_KEEPER.to_string(), keeper());
        register_scripts(scripts);
        let mut p1 = p1;
        p1["library"] = json!([plain.id, plain.id]);
        scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "field": [{ "def": plain.id, "lane": 1 }], "library": [plain.id, plain.id] },
        }))
    }

    /// The keeper in p1's backrow lane `lane` counts `n` on each Unit named.
    fn count(s: &mut Scenario, lane: i32, counted: &[(&str, i64)]) {
        let keeper = s.backrow(P1, lane).expect("a keeper in that lane");
        let counts: serde_json::Map<String, Value> =
            counted.iter().map(|(id, n)| (id.to_string(), json!(n))).collect();
        s.card_mut(keeper.id.as_str())
            .memory
            .insert(COUNTS.to_string(), Value::Object(counts));
    }

    fn tallies_seen(s: &Scenario, viewer: PlayerId, id: &str) -> Option<Vec<TallyView>> {
        let view = s.view(viewer);
        view.you
            .units
            .iter()
            .chain(view.opponent.units.iter())
            .flatten()
            .find(|unit| unit.instance_id == id)
            .unwrap_or_else(|| panic!("{id} is not on the field"))
            .tallies
            .clone()
    }

    fn tally(def_id: &str, value: i32) -> TallyView {
        TallyView {
            def_id: def_id.to_string(),
            value,
            goal: GOAL,
        }
    }

    #[test]
    fn r2321000_a_public_cards_count_rides_on_the_unit_it_counts_for_both_seats() {
        let mut s = board(
            "tallies-public",
            json!({
                "field": [{ "def": plain.id, "lane": 1 }, { "def": plain.id, "lane": 2 }],
                "backrow": [{ "def": FIELD_KEEPER, "faceUp": true, "lane": 1 }],
            }),
        );
        let counted = s.unit(P1, 1).unwrap().id;
        let other = s.unit(P1, 2).unwrap().id;
        count(&mut s, 1, &[(counted.as_str(), 2)]);

        assert_eq!(tallies_seen(&s, P1, &counted), Some(vec![tally(FIELD_KEEPER, 2)]));
        assert_eq!(tallies_seen(&s, P2, &counted), Some(vec![tally(FIELD_KEEPER, 2)]));
        // A Unit no card counts carries no key, never `[]`.
        assert_eq!(tallies_seen(&s, P1, &other), None);
        let enemy = s.unit(P2, 1).unwrap().id;
        assert_eq!(tallies_seen(&s, P1, &enemy), None);
    }

    #[test]
    fn r2321000_two_keepers_count_apart_on_the_same_unit() {
        let mut s = board(
            "tallies-two",
            json!({
                "field": [{ "def": plain.id, "lane": 1 }],
                "backrow": [
                    { "def": FIELD_KEEPER, "faceUp": true, "lane": 1 },
                    { "def": FIELD_KEEPER, "faceUp": true, "lane": 2 },
                ],
            }),
        );
        let counted = s.unit(P1, 1).unwrap().id;
        count(&mut s, 1, &[(counted.as_str(), 1)]);
        count(&mut s, 2, &[(counted.as_str(), 2)]);

        assert_eq!(
            tallies_seen(&s, P2, &counted),
            Some(vec![tally(FIELD_KEEPER, 1), tally(FIELD_KEEPER, 2)])
        );
    }

    #[test]
    fn r2321000_a_face_down_cards_count_reaches_its_controller_alone() {
        let mut s = board(
            "tallies-face-down",
            json!({
                "field": [{ "def": plain.id, "lane": 1 }],
                "backrow": [{ "def": TRAP_KEEPER, "faceUp": false, "lane": 1 }],
            }),
        );
        let counted = s.unit(P1, 1).unwrap().id;
        count(&mut s, 1, &[(counted.as_str(), 1)]);

        assert_eq!(tallies_seen(&s, P1, &counted), Some(vec![tally(TRAP_KEEPER, 1)]));
        assert_eq!(
            tallies_seen(&s, P2, &counted),
            None,
            "R33: the other seat cannot read the trap"
        );
    }
}
