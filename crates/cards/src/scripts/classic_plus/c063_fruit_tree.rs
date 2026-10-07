//! C+ #63 Fruit Tree (SPEC §8.7 row 63). (2) Field Spell, Fruit, Rare.
//!   Base:    "Start of Turn: Add a random Fruit to your hand. It costs (0)."
//!   Radiant: "Start of Turn: Add a random Radiant Fruit to your hand. It costs (0)."
//!   Engine:  "Its controller's start of turn (R62). The Fruit pool (R382) but Fruit Tree (R387);
//!            `costOverride` 0; the hand cap burns it (§2.4). Tunes: none."
//!
//! `startOfTurn` fires for the controller alone, on their own turn (`turn.startTurn` asks
//! `triggerOrder` for that one player, §2.2, R62), so the opponent's start of turn adds nothing. The
//! Fruit pool is the engine's: a `tags: ["Fruit"]` query holds the non-token Fruit cards of every set
//! and the five Grapes (R382), and `addRandomFromCatalog` leaves out the card running the script by
//! its def id (R387). Repeats are allowed (R60); a full hand burns what doesn't fit (§2.4, R4), and the
//! cost rider lands only on a card that reached the hand.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-063";

/// The printed "a random Fruit": one card per start of turn.
const FRUITS: i32 = 1;

/// The two faces differ only in whether the Fruit is Radiant (R276: the Radiant face's proposal).
fn fruit_tree(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            let mut args = json!({ "query": { "tags": ["Fruit"] }, "count": FRUITS, "costOverride": 0 });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fruit_tree(false),
        radiant: fruit_tree(true),
    }
}

// C+ #63 Fruit Tree — SPEC §8.7 row 63, BUILD M9 Classic+ row C+ 63: "Field Spell: at each start of
// your turn adds a random card of the Fruit pool (R382) that costs (0), never Fruit Tree (R387); nothing
// at the opponent's start of turn; a full hand burns; hidden from the opponent (R97); the count is
// fixed at 1 with no tunable (balance patch 1); radiant the Fruit is Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const TREE: &str = "classicplus-063";
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a hand card so turns do not auto-end, and deck filler.
    const GRAPES: [&str; 5] = [
        "classicplus-065-1",
        "classicplus-065-2",
        "classicplus-065-3",
        "classicplus-065-4",
        "classicplus-065-5",
    ];

    /// `_harness.ts` registers every card on import; the engine's testkit cannot, so this does.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS `stepParam(s.card(ref), key, steps)`: TS stepped the live card; here the card under its id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let live = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        step_param(live, key, steps);
    }

    fn is_grape(id: &str) -> bool {
        GRAPES.contains(&id)
    }

    /// The cards `player`'s start of turn added before its draw: `addedToHand` between `turnStarted` and
    /// `drawn`, as `(instanceId, defId)`.
    fn fruits_added(s: &Scenario, player: PlayerId) -> Vec<(String, String)> {
        let events = s.last_events();
        let Some(start) = events
            .iter()
            .position(|event| matches!(event, GameEvent::TurnStarted { player: p, .. } if *p == player))
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for event in &events[start + 1..] {
            match event {
                GameEvent::Drawn { player: p, .. } if *p == player => break,
                GameEvent::AddedToHand { player: p, instance_id, def_id } if *p == player => {
                    out.push((instance_id.clone(), def_id.clone()));
                }
                _ => {}
            }
        }
        out
    }

    /// A board with the Tree in p1's backrow and p1 about to start a turn (p2 ends theirs).
    fn grown(radiant: bool, seed: Option<&str>, hand: Option<Vec<&str>>) -> Scenario {
        scenario(json!({
            "seed": seed.unwrap_or("fruit-tree"),
            "active": "p2",
            "turn": 10,
            "p1": {
                "backrow": [{ "def": TREE, "faceUp": true, "radiant": radiant }],
                "hand": hand.unwrap_or_else(|| vec![FILLER]),
                "library": [FILLER, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    #[test]
    fn is_a_field_spell_whose_two_faces_run_one_shape_of_hook() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, TREE);
        assert_eq!(def.type_, CardType::FieldSpell);
        let scripts = super::script();
        assert!(scripts.base.start_of_turn.is_some());
        assert!(scripts.radiant.start_of_turn.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r62_at_the_start_of_your_turn_adds_one_random_fruit_to_your_hand_before_the_draw_costing_0() {
            let mut s = grown(false, None, None);
            s.end_turn();

            let added = fruits_added(&s, PlayerId::P1);
            assert_eq!(added.len(), 1);
            let fruit = s.card(&added[0].0).clone();
            assert_eq!(fruit.zone.z(), ZoneName::Hand);
            assert!(def_of(Some(s.state()), &fruit.def_id).tags.contains(&Tag::Fruit));
            assert_eq!(fruit.cost_override, Some(0));
            assert!(!fruit.radiant);
            s.expect_events(json!(["turnStarted", "addedToHand", "drawn"]));
        }

        #[test]
        fn r62_nothing_at_the_opponents_start_of_turn() {
            let mut s = scenario(json!({
                "seed": "fruit-tree",
                "active": "p1",
                "p1": { "backrow": [{ "def": TREE, "faceUp": true }], "hand": [FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            let before = s.hand(PlayerId::P1).len();
            s.end_turn();

            assert_eq!(s.state().active, PlayerId::P2);
            assert_eq!(fruits_added(&s, PlayerId::P1), Vec::<(String, String)>::new());
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
            );
            assert_eq!(s.hand(PlayerId::P1).len(), before);
        }

        #[test]
        fn r382_r387_over_many_seeds_it_hands_out_only_the_fruit_pool_grapes_included_and_never_fruit_tree() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..120 {
                let mut s = grown(false, Some(&format!("tree-{i}")), None);
                s.end_turn();
                for (_, def_id) in fruits_added(&s, PlayerId::P1) {
                    seen.insert(def_id);
                }
            }
            assert!(!seen.contains(TREE));
            let fresh = grown(false, None, None);
            for id in &seen {
                assert!(def_of(Some(fresh.state()), id).tags.contains(&Tag::Fruit));
            }
            assert!(seen.iter().any(|id| is_grape(id)));
            assert!(seen.iter().any(|id| !is_grape(id)));
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_fruit_which_keeps_no_price_in_the_graveyard() {
            let mut s = grown(false, None, Some(vec![FILLER; 10]));
            s.end_turn();

            let burned = s.last_events().iter().find_map(|event| match event {
                GameEvent::Burned { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            });
            assert!(burned.is_some());
            let fruit = s.card(&burned.unwrap_or_default()).clone();
            assert_eq!(fruit.zone.z(), ZoneName::Graveyard);
            assert_eq!(fruit.cost_override, None);
            assert_eq!(fruits_added(&s, PlayerId::P1), Vec::<(String, String)>::new());
        }

        #[test]
        fn r97_the_opponents_view_names_no_fruit_the_add_reaches_them_under_the_sentinel() {
            let mut s = grown(false, None, None);
            s.end_turn();
            let fruit = fruits_added(&s, PlayerId::P1).into_iter().next();
            assert!(fruit.is_some());
            let fruit_id = fruit.map(|(id, _)| id).unwrap_or_else(|| "?".to_string());

            let view = s.view(PlayerId::P2);
            let theirs: Vec<&GameEvent> = view
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert!(!theirs.is_empty());
            for event in theirs {
                if let GameEvent::AddedToHand { instance_id, def_id, .. } = event {
                    assert_eq!(instance_id, "hidden");
                    assert_eq!(def_id, "hidden");
                }
            }
            assert!(!serde_json::to_string(&s.view(PlayerId::P2)).unwrap().contains(&fruit_id));
            let mine = match s.view(PlayerId::P1).you.hand {
                HandView::Cards(cards) => cards.iter().any(|card| card.instance_id == fruit_id),
                HandView::Count { .. } => false,
            };
            assert!(mine);
        }

        #[test]
        fn r386_the_count_is_fixed_at_1_an_upgrade_or_a_degrade_still_adds_exactly_one_fruit() {
            let mut up = grown(false, None, None);
            step(&mut up, TREE, "fruits", 1);
            up.end_turn();
            assert_eq!(fruits_added(&up, PlayerId::P1).len(), 1);

            let mut down = grown(false, None, None);
            step(&mut down, TREE, "fruits", -1);
            down.end_turn();
            assert_eq!(fruits_added(&down, PlayerId::P1).len(), 1);
        }

        #[test]
        fn s3_2_off_the_field_it_adds_nothing_a_tree_in_the_hand_has_no_start_of_turn() {
            let mut s = scenario(json!({
                "seed": "fruit-tree",
                "active": "p2",
                "turn": 10,
                "p1": { "hand": [TREE, FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            s.end_turn();
            assert_eq!(fruits_added(&s, PlayerId::P1), Vec::<(String, String)>::new());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_fruit_it_adds_is_radiant_and_costs_0() {
            let mut s = grown(true, None, None);
            s.end_turn();

            let added = fruits_added(&s, PlayerId::P1);
            assert_eq!(added.len(), 1);
            let fruit = s.card(&added[0].0).clone();
            assert!(fruit.radiant);
            assert_eq!(fruit.cost_override, Some(0));
        }

        #[test]
        fn r387_never_fruit_tree_either_and_a_grape_comes_radiant_too() {
            let mut seen: Vec<(String, bool)> = Vec::new();
            for i in 0..80 {
                let mut s = grown(true, Some(&format!("rtree-{i}")), None);
                s.end_turn();
                for (id, def_id) in fruits_added(&s, PlayerId::P1) {
                    let radiant = s.card(&id).radiant;
                    seen.push((def_id, radiant));
                }
            }
            assert!(!seen.iter().any(|entry| entry.0 == TREE));
            assert!(seen.iter().all(|entry| entry.1));
            assert!(seen.iter().any(|entry| is_grape(&entry.0)));
        }

        #[test]
        fn r386_the_radiant_count_is_fixed_at_1_too() {
            let mut s = grown(true, None, None);
            step(&mut s, TREE, "fruits", 1);
            s.end_turn();
            let added = fruits_added(&s, PlayerId::P1);
            assert_eq!(added.len(), 1);
            assert!(added.iter().all(|(id, _)| s.card(id).radiant));
        }

        #[test]
        fn s9_3_the_same_seed_adds_the_same_fruit_and_the_state_survives_a_json_round_trip() {
            let mut a = grown(true, Some("same"), None);
            let mut b = grown(true, Some("same"), None);
            a.end_turn();
            b.end_turn();
            let defs = |s: &Scenario| -> Vec<String> {
                fruits_added(s, PlayerId::P1).into_iter().map(|(_, def_id)| def_id).collect()
            };
            assert_eq!(defs(&a), defs(&b));
            let revived: GameState = serde_json::from_value(serde_json::to_value(a.state()).unwrap()).unwrap();
            assert_eq!(&revived, a.state());
        }
    }
}
