//! R195 (SPEC §10.8, §10.9): the cards with a printed condition light up yellow through
//! `conditionActive`, and each test proves the flag agrees with the branch the card's own resolution
//! then takes. The flag is read from `s.view(...)` (the viewer's `viewFor`), the branch from the
//! scenario's own assertions after a real `s.play(...)` or `s.endTurn()`.
//!
//! The key is present and `true`, or absent: `glows` below fails on a key that is present with any
//! other value.
//!
//! R196 closes the file: a fusion's hook is its ingredients' hooks or-ed, checked by crafting #53 Reno
//! with #68 Twisted Sorcerer (R77's hand path, the one #99 Craft a Card takes) and playing the result.
//!
//! These are the per-card proofs §10.9 asks of a card with `conditionMet` (crates/cards/README.md §5),
//! and the last test pins the set of cards that declare the hook to R195's list.

use jackioh_engine::testkit::*;
use serde::Serialize;

type Instance = CardInstance;

/// The harness with every card script registered: the engine's testkit cannot name the cards crate.
fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn own_hand(view: &PlayerView) -> Vec<CardView> {
    match &view.you.hand {
        HandView::Cards(cards) => cards.clone(),
        HandView::Count { .. } => panic!("the viewer's own hand must travel in full (§10.8)"),
    }
}

fn hand_card(view: &PlayerView, instance_id: &str) -> CardView {
    own_hand(view)
        .into_iter()
        .find(|card| card.instance_id == instance_id)
        .unwrap_or_else(|| panic!("{instance_id} is not in the viewer's hand"))
}

/// `true` when the view carries the key (which must then be exactly `true`), `false` when absent.
fn glows<T: Serialize>(card: Option<&T>) -> bool {
    let card = card.unwrap_or_else(|| panic!("no card at that place in the view"));
    let json = serde_json::to_value(card).unwrap();
    match json.get("conditionActive") {
        None => false,
        Some(flag) => {
            assert_eq!(flag, &json!(true));
            true
        }
    }
}

/// Every copy of a def in p1's hand, in hand order.
fn copies_in_hand(s: &Scenario, def_id: &str) -> Vec<Instance> {
    s.hand("p1")
        .into_iter()
        .filter(|card| card.def_id == def_id)
        .collect()
}

fn nth<T: Clone>(list: &[T], index: usize) -> T {
    list.get(index)
        .cloned()
        .unwrap_or_else(|| panic!("nothing at index {index}"))
}

/// Does p1's copy of `defId` in hand glow in p1's own view right now?
fn hand_glows(s: &Scenario, instance: &Instance) -> bool {
    glows(Some(&hand_card(&s.view("p1"), &instance.id)))
}

fn count_of(s: &Scenario, type_: &str) -> usize {
    s.events()
        .iter()
        .filter(|event| event.event_type().as_str() == type_)
        .count()
}

/// The unit's id, or a reference that names nothing.
fn unit_ref(s: &Scenario, player: &'static str, lane: i32) -> String {
    s.unit(player, lane).map(|card| card.id).unwrap_or_default()
}

// #10 Rapid Replenish (hand only): Combo 3

/// #10 Rapid Replenish lights up at Combo 3 (R195, B5)
mod c10_rapid_replenish_lights_up_at_combo_3_r195_b5 {
    use super::*;

    const LIBRARY: [&str; 8] = [
        "core-020", "core-020", "core-020", "core-011", "core-011", "core-011", "core-016", "core-016",
    ];

    #[test]
    fn r195_b5_after_two_plays_it_carries_no_flag_and_draws_nothing_the_copy_that_then_finds_three_plays_glows_and_draws_3()
     {
        let mut s = setup(json!({
            "seed": "r195-010-boundary",
            "p1": { "hand": ["core-011", "core-011", "core-010", "core-010"], "mana": 10, "library": LIBRARY },
            "p2": { "field": ["core-020"] },
        }));
        let copies = copies_in_hand(&s, "core-010");
        let first = nth(&copies, 0);
        let second = nth(&copies, 1);

        assert!(!hand_glows(&s, &first));
        s.play("core-011", json!({}));
        assert!(!hand_glows(&s, &first));
        s.play("core-011", json!({}));
        assert!(!hand_glows(&s, &first));
        assert!(!hand_glows(&s, &second));

        let library = s.pile("p1", "library").len();
        s.play(&first, json!({}));

        // The flag was off, and the resolution agreed: no draw.
        assert_eq!(s.pile("p1", "library").len(), library);
        let ids: Vec<String> = s.hand("p1").into_iter().map(|card| card.id).collect();
        assert_eq!(ids, std::slice::from_ref(&second.id));

        // That copy counted as a play, so the other one now finds three earlier plays.
        assert!(hand_glows(&s, &second));
        s.play(&second, json!({}));

        assert_eq!(s.pile("p1", "library").len(), library - 3);
        assert_eq!(s.hand("p1").len(), 3);
    }

    #[test]
    fn r195_b5_the_radiant_face_glows_on_the_same_count_and_then_draws_6() {
        let mut s = setup(json!({
            "seed": "r195-010-radiant",
            "p1": {
                "hand": ["core-011", "core-011", "core-011", { "def": "core-010", "radiant": true }],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "field": ["core-020"] },
        }));
        let spell = nth(&copies_in_hand(&s, "core-010"), 0);
        assert!(spell.radiant);

        s.play("core-011", json!({}));
        s.play("core-011", json!({}));
        assert!(!hand_glows(&s, &spell));
        s.play("core-011", json!({}));
        assert!(hand_glows(&s, &spell));

        let library = s.pile("p1", "library").len();
        s.play(&spell, json!({}));

        assert_eq!(s.pile("p1", "library").len(), library - 6);
        assert_eq!(s.hand("p1").len(), 6);
    }

    #[test]
    fn r195_b5_on_the_opponents_turn_it_never_glows_its_controllers_three_plays_over_with_their_turn() {
        let mut s = setup(json!({
            "seed": "r195-010-their-turn",
            "p1": { "hand": ["core-011", "core-011", "core-011", "core-010"], "mana": 10, "library": LIBRARY },
            "p2": { "hand": ["core-005"], "library": ["core-016", "core-016", "core-016"] },
        }));
        let spell = nth(&copies_in_hand(&s, "core-010"), 0);
        s.play("core-011", json!({}));
        s.play("core-011", json!({}));
        s.play("core-011", json!({}));
        assert!(hand_glows(&s, &spell));

        s.end_turn();

        assert_eq!(s.state().active, PlayerId::P2);
        // "This turn" starts afresh for both players (§6.2), so p1's three plays no longer count on
        // p2's turn. That the hand is not even asked outside its own main phase, where a hook would
        // answer true, is R195's rule 3.
        assert_eq!(s.state().players.p1.turn_log.cards_played, 0);
        assert!(!hand_glows(&s, &spell));
    }

    #[test]
    fn r195_b5_the_opponents_view_never_carries_the_flag_its_hand_is_a_count() {
        let mut s = setup(json!({
            "seed": "r195-010-opponent-view",
            "p1": { "hand": ["core-011", "core-011", "core-011", "core-010"], "mana": 10, "library": LIBRARY },
            "p2": { "field": ["core-020"] },
        }));
        s.play("core-011", json!({}));
        s.play("core-011", json!({}));
        s.play("core-011", json!({}));

        assert!(hand_glows(&s, &nth(&copies_in_hand(&s, "core-010"), 0)));
        let theirs = s.view("p2");
        assert_eq!(
            serde_json::to_value(&theirs.opponent.hand).unwrap(),
            json!({ "count": 1 })
        );
        assert!(
            !serde_json::to_string(&theirs)
                .unwrap()
                .contains("conditionActive")
        );
    }
}

// #53 Reno (hand only): your hero below 30, radiant 60

/// #53 Reno lights up below its floor (R195, B6)
mod c53_reno_lights_up_below_its_floor_r195_b6 {
    use super::*;

    #[test]
    fn r195_b6_below_30_it_glows_in_hand_and_its_cry_then_sets_the_hero_to_30() {
        for health in [29, 12, 1] {
            let mut s = setup(
                json!({ "seed": format!("r195-053-low-{health}"), "p1": { "hand": ["core-053"], "health": health } }),
            );

            let reno = s.card("core-053").clone();
            assert!(hand_glows(&s, &reno), "hero at {health}");
            s.play("core-053", json!({})).expect_health("p1", 30);
        }
    }

    #[test]
    fn r195_b6_at_30_or_above_it_does_not_glow_and_its_cry_heals_nothing() {
        for health in [30, 35] {
            let mut s = setup(
                json!({ "seed": format!("r195-053-high-{health}"), "p1": { "hand": ["core-053"], "health": health } }),
            );

            let reno = s.card("core-053").clone();
            assert!(!hand_glows(&s, &reno), "hero at {health}");
            s.play("core-053", json!({})).expect_health("p1", health);
            let healed = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Healed)
                .count();
            assert_eq!(healed, 0);
        }
    }

    #[test]
    fn r195_b6_the_radiant_face_glows_below_60_and_raises_the_hero_to_60() {
        for health in [59, 35, 12] {
            let mut s = setup(json!({
                "seed": format!("r195-053-radiant-low-{health}"),
                "p1": { "hand": [{ "def": "core-053", "radiant": true }], "health": health },
            }));

            let reno = s.card("core-053").clone();
            assert!(hand_glows(&s, &reno), "hero at {health}");
            s.play("core-053", json!({})).expect_health("p1", 60);
        }
    }

    #[test]
    fn r195_b6_the_radiant_face_at_60_or_above_does_not_glow_and_heals_nothing() {
        for health in [60, 70] {
            let mut s = setup(json!({
                "seed": format!("r195-053-radiant-high-{health}"),
                "p1": { "hand": [{ "def": "core-053", "radiant": true }], "health": health },
            }));

            let reno = s.card("core-053").clone();
            assert!(!hand_glows(&s, &reno), "hero at {health}");
            s.play("core-053", json!({})).expect_health("p1", health);
            let healed = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Healed)
                .count();
            assert_eq!(healed, 0);
        }
    }

    #[test]
    fn r195_b6_a_low_opponent_does_not_light_it_your_hero_is_the_controllers() {
        let mut s = setup(
            json!({ "seed": "r195-053-their-hero", "p1": { "hand": ["core-053"], "health": 30 }, "p2": { "health": 5 } }),
        );

        let reno = s.card("core-053").clone();
        assert!(!hand_glows(&s, &reno));
        s.play("core-053", json!({}))
            .expect_health("p1", 30)
            .expect_health("p2", 5);
    }

    #[test]
    fn r195_b6_a_reno_on_the_field_never_carries_the_flag_even_with_its_controllers_hero_low() {
        for active in ["p1", "p2"] {
            let s = setup(json!({
                "seed": format!("r195-053-field-{active}"),
                "active": active,
                "p1": { "field": ["core-053", { "def": "core-053", "radiant": true }], "health": 12, "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            let view = s.view("p1");

            assert_eq!(
                view.you.units[0].as_ref().map(|unit| unit.def_id.as_str()),
                Some("core-053")
            );
            assert!(!glows(view.you.units[0].as_ref()), "base on {active}'s turn");
            assert!(!glows(view.you.units[1].as_ref()), "radiant on {active}'s turn");
        }
    }
}

// #68 Twisted Sorcerer (hand only): your hero below 10

/// #68 Twisted Sorcerer lights up below 10 (R195, B7)
mod c68_twisted_sorcerer_lights_up_below_10_r195_b7 {
    use super::*;

    const SORCERER: &str = "core-068"; // Unit 5/5 → 10/10.
    const SPONGE: &str = "core-019"; // Midrange Menace 9/9 → 18/18, no Armor.
    /// A free Spell that keeps the turn open (§2.5's auto-end); never played.
    const ANCHOR: &str = "core-010";

    /// The scenario with ANCHOR added at the end of p1's hand.
    fn board(mut opts: Value) -> Scenario {
        let root = opts.as_object_mut().expect("scenario options are an object");
        let p1 = root.entry("p1").or_insert_with(|| json!({}));
        let hand = p1
            .as_object_mut()
            .expect("p1 is an object")
            .entry("hand")
            .or_insert_with(|| json!([]));
        hand.as_array_mut()
            .expect("p1.hand is a list")
            .push(json!(ANCHOR));
        setup(opts)
    }

    fn at_sponge(s: &Scenario) -> Value {
        let unit = s
            .unit("p2", 1)
            .unwrap_or_else(|| panic!("no sponge in p2's lane 1"));
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    fn sorcerer(s: &Scenario) -> Instance {
        nth(&copies_in_hand(s, SORCERER), 0)
    }

    #[test]
    fn r195_b7_below_10_it_glows_in_hand_and_its_cry_then_deals_8() {
        let mut s = board(
            json!({ "seed": "r195-068-low", "p1": { "hand": [SORCERER], "health": 9 }, "p2": { "field": [SPONGE] } }),
        );

        assert!(hand_glows(&s, &sorcerer(&s)));
        let targets = at_sponge(&s);
        s.play(SORCERER, json!({ "targets": targets }));
        s.expect_stats(SPONGE, json!({ "health": 1, "maxHealth": 9 }));
    }

    #[test]
    fn r195_b7_at_exactly_10_it_does_not_glow_and_its_cry_deals_4() {
        let mut s = board(
            json!({ "seed": "r195-068-ten", "p1": { "hand": [SORCERER], "health": 10 }, "p2": { "field": [SPONGE] } }),
        );

        assert!(!hand_glows(&s, &sorcerer(&s)));
        let targets = at_sponge(&s);
        s.play(SORCERER, json!({ "targets": targets }));
        s.expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }));
    }

    #[test]
    fn r195_b7_the_radiant_face_glows_below_10_and_deals_16() {
        let mut s = board(json!({
            "seed": "r195-068-radiant-low",
            "p1": { "hand": [{ "def": SORCERER, "radiant": true }], "health": 9 },
            "p2": { "field": [{ "def": SPONGE, "radiant": true }] },
        }));

        assert!(hand_glows(&s, &sorcerer(&s)));
        let targets = at_sponge(&s);
        s.play(SORCERER, json!({ "targets": targets }));
        s.expect_stats(SPONGE, json!({ "health": 2, "maxHealth": 18 }));
    }

    #[test]
    fn r195_b7_the_radiant_face_at_exactly_10_does_not_glow_and_deals_8() {
        let mut s = board(json!({
            "seed": "r195-068-radiant-ten",
            "p1": { "hand": [{ "def": SORCERER, "radiant": true }], "health": 10 },
            "p2": { "field": [SPONGE] },
        }));

        assert!(!hand_glows(&s, &sorcerer(&s)));
        let targets = at_sponge(&s);
        s.play(SORCERER, json!({ "targets": targets }));
        s.expect_stats(SPONGE, json!({ "health": 1, "maxHealth": 9 }));
    }

    #[test]
    fn r195_b7_a_low_opponent_does_not_light_it_and_the_cry_deals_4() {
        let mut s = board(
            json!({ "seed": "r195-068-their-hero", "p1": { "hand": [SORCERER], "health": 30 }, "p2": { "field": [SPONGE], "health": 3 } }),
        );

        assert!(!hand_glows(&s, &sorcerer(&s)));
        let targets = at_sponge(&s);
        s.play(SORCERER, json!({ "targets": targets }));
        s.expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }));
    }

    #[test]
    fn r195_b7_a_twisted_sorcerer_on_the_field_never_carries_the_flag() {
        let s = board(json!({ "seed": "r195-068-field", "p1": { "field": [SORCERER], "health": 3 } }));

        assert!(!glows(s.view("p1").you.units[0].as_ref()));
    }
}

// #71 Intern Stimmy (hand and field): your library strictly larger

/// #71 Intern Stimmy lights up while its controller's library is larger (R195, B8)
mod c71_intern_stimmy_lights_up_while_its_controllers_library_is_larger_r195_b8 {
    use super::*;

    #[test]
    fn r195_b8_in_the_backrow_it_glows_at_3_cards_to_1_the_opponent_still_sees_a_face_down_card_and_it_fires_at_the_turns_end()
     {
        let mut s = setup(json!({
            "seed": "r195-071-ahead",
            "p1": { "backrow": ["core-071"], "library": ["core-001", "core-003", "core-005"], "hand": ["core-005"] },
            "p2": { "library": ["core-005"], "hand": ["core-005"] },
        }));

        assert!(glows(s.view("p1").you.backrow[0].as_ref()));
        assert_eq!(
            serde_json::to_value(&s.view("p2").opponent.backrow[0]).unwrap(),
            json!({ "faceDown": true, "cost": 1 })
        );

        s.end_turn();

        assert_eq!(count_of(&s, "trapFired"), 1);
        assert_eq!(s.unit("p1", 1).map(|c| c.def_id), Some("core-003".to_string()));
        // Fired, so public now (R33), and still never flagged on the opponent's side.
        let theirs = s.view("p2").opponent.backrow[0].clone();
        let theirs_json = serde_json::to_value(&theirs).unwrap();
        assert_eq!(theirs_json["faceDown"], json!(false), "{theirs_json}");
        assert_eq!(theirs_json["defId"], json!("core-071"), "{theirs_json}");
        assert!(!glows(theirs.as_ref()));
    }

    #[test]
    fn r195_b8_with_equal_libraries_it_does_not_glow_and_the_trap_does_not_fire() {
        let mut s = setup(json!({
            "seed": "r195-071-equal",
            "p1": { "backrow": ["core-071"], "library": ["core-001", "core-003"], "hand": ["core-005"] },
            "p2": { "library": ["core-005", "core-023"], "hand": ["core-005"] },
        }));

        assert!(!glows(s.view("p1").you.backrow[0].as_ref()));
        s.end_turn();
        assert_eq!(count_of(&s, "trapFired"), 0);
        assert!(s.unit("p1", 1).is_none());
    }

    #[test]
    fn r195_b8_with_a_smaller_library_it_does_not_glow_and_the_trap_does_not_fire() {
        let mut s = setup(json!({
            "seed": "r195-071-behind",
            "p1": { "backrow": ["core-071"], "library": ["core-003"], "hand": ["core-005"] },
            "p2": { "library": ["core-005", "core-023", "core-035"], "hand": ["core-005"] },
        }));

        assert!(!glows(s.view("p1").you.backrow[0].as_ref()));
        s.end_turn();
        assert_eq!(count_of(&s, "trapFired"), 0);
    }

    #[test]
    fn r195_b8_on_the_opponents_turn_the_backrow_copy_still_glows_for_its_controller_and_fires_at_that_turns_end()
     {
        let mut s = setup(json!({
            "seed": "r195-071-their-turn",
            "active": "p2",
            "p1": { "backrow": ["core-071"], "library": ["core-001", "core-003", "core-005"], "hand": ["core-005"] },
            "p2": { "library": ["core-005", "core-023"], "hand": ["core-005"] },
        }));

        assert!(glows(s.view("p1").you.backrow[0].as_ref()));
        assert_eq!(
            serde_json::to_value(&s.view("p2").opponent.backrow[0]).unwrap(),
            json!({ "faceDown": true, "cost": 1 })
        );

        s.end_turn();

        assert_eq!(count_of(&s, "trapFired"), 1);
        assert_eq!(s.unit("p1", 1).map(|c| c.def_id), Some("core-003".to_string()));
    }

    #[test]
    fn r195_b8_the_radiant_face_glows_on_the_same_condition_and_recruits_up_to_cost_2() {
        let mut s = setup(json!({
            "seed": "r195-071-radiant",
            "p1": {
                "backrow": [{ "def": "core-071", "radiant": true }],
                "library": ["core-001", "core-003", "core-005"],
                "hand": ["core-005"],
            },
            "p2": { "library": ["core-005"], "hand": ["core-005"] },
        }));

        assert!(glows(s.view("p1").you.backrow[0].as_ref()));
        s.end_turn();
        assert_eq!(count_of(&s, "trapFired"), 1);
        assert_eq!(s.unit("p1", 1).map(|c| c.def_id), Some("core-001".to_string()));
    }

    #[test]
    fn r195_b8_in_hand_it_glows_while_the_library_is_larger_and_keeps_glowing_once_played_to_the_backrow() {
        let mut s = setup(json!({
            "seed": "r195-071-hand-ahead",
            "p1": { "hand": ["core-071", "core-005"], "library": ["core-001", "core-003", "core-005"] },
            "p2": { "library": ["core-005"], "hand": ["core-005"] },
        }));
        let trap = s.card("core-071").clone();

        assert!(hand_glows(&s, &trap));
        s.play(&trap, json!({}));

        // R227: set face-down, the card took a fresh id; it is the same Intern Stimmy.
        assert_eq!(s.backrow("p1", 1).map(|c| c.def_id), Some("core-071".to_string()));
        assert!(glows(s.view("p1").you.backrow[0].as_ref()));
        assert_eq!(
            serde_json::to_value(&s.view("p2").opponent.backrow[0]).unwrap(),
            json!({ "faceDown": true, "cost": 1 })
        );
    }

    #[test]
    fn r195_b8_in_hand_with_equal_libraries_it_does_not_glow() {
        let s = setup(json!({
            "seed": "r195-071-hand-equal",
            "p1": { "hand": ["core-071", "core-005"], "library": ["core-001", "core-003"] },
            "p2": { "library": ["core-005", "core-023"], "hand": ["core-005"] },
        }));

        let trap = s.card("core-071").clone();
        assert!(!hand_glows(&s, &trap));
    }
}

// #93 Combo-Index (field only): your turn, cards played reach the grade, not at S

/// #93 Combo-Index lights up when its grade will rise (R195, B9)
mod c93_combo_index_lights_up_when_its_grade_will_rise_r195_b9 {
    use super::*;

    const COMBO_INDEX: &str = "core-093";
    /// Keyword-only units: a play is only a play.
    const FODDER: [&str; 3] = ["core-003", "core-008", "core-011"];
    /// Cards that sit in hand as material for the cascade's steps.
    const HELD: [&str; 3] = ["core-005", "core-010", "core-056"];

    fn fodder_and_held() -> Vec<&'static str> {
        FODDER.iter().chain(HELD.iter()).copied().collect()
    }

    fn grade(s: &Scenario) -> i32 {
        s.card(COMBO_INDEX).counters.grade.unwrap_or(1)
    }

    fn index_glows(s: &Scenario) -> bool {
        glows(s.view("p1").you.backrow[0].as_ref())
    }

    /// `entry` is a def id or `{ def, counters?, radiant? }`.
    fn board(seed: &str, entry: Value) -> Scenario {
        setup(json!({
            "seed": seed,
            "p1": { "backrow": [entry], "hand": fodder_and_held(), "library": ["core-016", "core-016"], "mana": 10 },
            "p2": { "hand": ["core-005"], "library": ["core-016", "core-016"] },
        }))
    }

    #[test]
    fn r195_b9_at_e_it_glows_once_a_card_has_been_played_this_turn_and_the_grade_then_rises_to_d() {
        let mut s = board("r195-093-e", json!(COMBO_INDEX));

        assert!(!index_glows(&s));
        s.play(FODDER[0], json!({}));
        assert!(index_glows(&s));
        // A Field Spell is public, but the flag is still the controller's alone.
        assert!(!glows(s.view("p2").opponent.backrow[0].as_ref()));

        s.end_turn();
        assert_eq!(grade(&s), 2);
    }

    #[test]
    fn r195_b9_below_its_grade_it_does_not_glow_and_the_grade_does_not_rise() {
        let mut s = board(
            "r195-093-d-short",
            json!({ "def": COMBO_INDEX, "counters": { "grade": 2 } }),
        );

        s.play(FODDER[0], json!({}));
        assert!(!index_glows(&s));

        s.end_turn();
        assert_eq!(grade(&s), 2);
    }

    #[test]
    fn r195_b9_reaching_its_grade_lights_it_and_the_grade_then_rises() {
        let mut s = board(
            "r195-093-d-reached",
            json!({ "def": COMBO_INDEX, "counters": { "grade": 2 } }),
        );

        s.play(FODDER[0], json!({}));
        assert!(!index_glows(&s));
        s.play(FODDER[1], json!({}));
        assert!(index_glows(&s));

        s.end_turn();
        assert_eq!(grade(&s), 3);
    }

    #[test]
    fn r195_b9_at_s_it_never_glows_however_many_cards_were_played_and_the_grade_stays_s() {
        let mut s = board(
            "r195-093-s",
            json!({ "def": COMBO_INDEX, "counters": { "grade": 6 } }),
        );

        s.play(FODDER[0], json!({}));
        s.play(FODDER[1], json!({}));
        s.play(FODDER[2], json!({}));
        assert!(!index_glows(&s));

        s.end_turn();
        assert_eq!(grade(&s), 6);
    }

    #[test]
    fn r195_b9_on_the_opponents_turn_it_never_glows_its_controllers_plays_over_with_their_turn() {
        let mut s = board("r195-093-their-turn", json!(COMBO_INDEX));

        s.play(FODDER[0], json!({}));
        s.play(FODDER[1], json!({}));
        assert!(index_glows(&s));

        s.end_turn();

        assert_eq!(s.state().active, PlayerId::P2);
        assert_eq!(grade(&s), 2);
        // "This turn" starts afresh for both players (§6.2), so p1's two plays no longer count on
        // p2's turn, and the hook's `yourTurn` would keep it dark even if they did.
        assert_eq!(s.state().players.p1.turn_log.cards_played, 0);
        assert!(!index_glows(&s));
    }

    #[test]
    fn r195_b9_the_radiant_face_glows_on_the_same_condition() {
        let mut s = board("r195-093-radiant", json!({ "def": COMBO_INDEX, "radiant": true }));

        assert!(!index_glows(&s));
        s.play(FODDER[0], json!({}));
        assert!(index_glows(&s));
    }

    #[test]
    fn r195_b9_in_hand_it_never_glows_even_once_the_cards_played_would_reach_its_grade() {
        let mut hand = vec![COMBO_INDEX];
        hand.extend(fodder_and_held());
        let mut s = setup(json!({
            "seed": "r195-093-hand",
            "p1": { "hand": hand, "library": ["core-016"], "mana": 10 },
            "p2": { "hand": ["core-005"], "library": ["core-016"] },
        }));
        let index = s.card(COMBO_INDEX).clone();

        s.play(FODDER[0], json!({}));
        s.play(FODDER[1], json!({}));
        s.play(FODDER[2], json!({}));

        assert!(!hand_glows(&s, &index));
    }
}

// R196: a fusion glows when any ingredient's condition holds

/// R196 a crafted #53 Reno + #68 Twisted Sorcerer glows when either printed condition holds
mod r196_a_crafted_c53_reno_c68_twisted_sorcerer_glows_when_either_printed_condition_holds {
    use super::*;

    const RENO: &str = "core-053";
    const SORCERER: &str = "core-068";
    const SPONGE: &str = "core-019"; // Midrange Menace 9/9, no Armor.
    /// A free Spell that keeps the turn open (§2.5's auto-end); never played.
    const ANCHOR: &str = "core-010";

    /// Fuse p1's Reno and Sorcerer into p1's hand through R77's hand path, as #99 Craft a Card does.
    fn crafted(seed: &str, health: i32) -> (Scenario, CardInstance) {
        let mut s = setup(json!({
            "seed": seed,
            "p1": { "hand": [RENO, SORCERER, ANCHOR], "health": health, "mana": 10 },
            "p2": { "field": [SPONGE] },
        }));
        let ingredients = vec![s.card(RENO).clone(), s.card(SORCERER).clone()];
        // A sink of its own, whose events and rng cursor are dropped afterwards.
        let fused = {
            let state = s.state_mut();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut events: Vec<GameEvent> = Vec::new();
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            subsystems::fuse::fuse(
                &mut sink,
                subsystems::fuse::FuseArgs {
                    ingredients,
                    to_hand: Some(PlayerId::P1),
                    ..Default::default()
                },
            )
        };
        let fused = fused.unwrap_or_else(|| panic!("the fusion did not happen"));
        (s, fused)
    }

    fn play_at_sponge(s: &mut Scenario, fused: &CardInstance) {
        let sponge = s
            .unit("p2", 1)
            .unwrap_or_else(|| panic!("no sponge in p2's lane 1"));
        s.play(
            fused,
            json!({ "targets": [{ "pick": "instance", "instanceId": sponge.id }] }),
        );
    }

    #[test]
    fn r196_with_the_hero_at_5_both_conditions_hold_it_glows_heals_to_30_and_the_sorcerer_half_then_reads_30_and_deals_4()
     {
        let (mut s, fused) = crafted("r196-both", 5);

        assert!(hand_glows(&s, &fused));
        play_at_sponge(&mut s, &fused);
        // R102: each ingredient's list is built as the fused Cry reaches it, so the Sorcerer's
        // "8 if your hero is below 10" reads the hero Reno has just set to 30.
        s.expect_health("p1", 30)
            .expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }));
    }

    #[test]
    fn r196_with_the_hero_at_20_only_renos_holds_it_still_glows_heals_to_30_and_the_sorcerer_half_deals_4() {
        let (mut s, fused) = crafted("r196-reno-only", 20);

        assert!(hand_glows(&s, &fused));
        play_at_sponge(&mut s, &fused);
        s.expect_health("p1", 30)
            .expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }));
    }

    #[test]
    fn r196_with_the_hero_at_30_neither_holds_no_glow_no_heal_and_4_damage() {
        let (mut s, fused) = crafted("r196-neither", 30);

        assert!(!hand_glows(&s, &fused));
        play_at_sponge(&mut s, &fused);
        s.expect_health("p1", 30)
            .expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }));
        let healed = s
            .last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Healed)
            .count();
        assert_eq!(healed, 0);
    }
}

// Classic #22 Mid Runner (hand only): your mana is {threshold} or more now

/// C #22 Mid Runner lights up in hand while your mana reaches its threshold (R195)
mod classic_c22_mid_runner_lights_up_in_hand_while_your_mana_reaches_its_threshold_r195 {
    use super::*;

    const RUNNER: &str = "classic-022";
    const ANCHOR: &str = "core-010";
    const TARGETS: [&str; 3] = ["core-008", "core-019", "core-011"];

    fn bounces(s: &Scenario) -> usize {
        count_of(s, "bounced")
    }

    fn glows_at_4_mana_and_bounces(radiant: bool) {
        let count = if radiant { 3 } else { 2 };
        let mut s = setup(
            json!({ "p1": { "hand": [{ "def": RUNNER, "radiant": radiant }, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": TARGETS } }),
        );

        assert!(hand_glows(&s, &nth(&copies_in_hand(&s, RUNNER), 0)));
        s.play(RUNNER, json!({ "zone": 1 }));
        assert_eq!(bounces(&s), count);
    }

    fn dark_at_3_mana_and_bounces_nothing(radiant: bool) {
        let mut s = setup(
            json!({ "p1": { "hand": [{ "def": RUNNER, "radiant": radiant }, ANCHOR], "mana": 3 }, "p2": { "hand": [ANCHOR], "field": TARGETS } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, RUNNER), 0)));
        s.play(RUNNER, json!({ "zone": 1 }));
        assert_eq!(bounces(&s), 0);
    }

    #[test]
    fn r195_base_with_4_mana_it_glows_in_hand_and_played_now_it_bounces_two() {
        glows_at_4_mana_and_bounces(false);
    }

    #[test]
    fn r195_base_with_3_mana_it_does_not_glow_and_played_now_it_bounces_nothing() {
        dark_at_3_mana_and_bounces_nothing(false);
    }

    #[test]
    fn r195_radiant_with_4_mana_it_glows_in_hand_and_played_now_it_bounces_three() {
        glows_at_4_mana_and_bounces(true);
    }

    #[test]
    fn r195_radiant_with_3_mana_it_does_not_glow_and_played_now_it_bounces_nothing() {
        dark_at_3_mana_and_bounces_nothing(true);
    }

    #[test]
    fn r195_r386_a_degrade_of_the_threshold_to_5_4_mana_neither_glows_nor_bounces() {
        let mut s = setup(
            json!({ "p1": { "hand": [RUNNER, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": TARGETS } }),
        );
        let runner_id = nth(&copies_in_hand(&s, RUNNER), 0).id;
        let live = find_instance_mut(s.state_mut(), &runner_id).expect("the Runner in p1's hand");
        params::step_param(live, "threshold", 1);

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, RUNNER), 0)));
        s.play(RUNNER, json!({ "zone": 1 }));
        assert_eq!(bounces(&s), 0);
    }

    #[test]
    fn r195_on_the_field_it_never_glows_and_the_opponents_view_never_carries_the_flag() {
        let s = setup(
            json!({ "p1": { "hand": [RUNNER, ANCHOR], "field": [RUNNER] }, "p2": { "hand": [ANCHOR] } }),
        );

        assert!(!glows(s.view("p1").you.units[0].as_ref()));
        assert!(
            !serde_json::to_string(&s.view("p2"))
                .unwrap()
                .contains("conditionActive")
        );
    }

    #[test]
    fn r195_on_the_opponents_turn_it_never_glows_whatever_its_owners_mana() {
        let s = setup(
            json!({ "active": "p2", "p1": { "hand": [RUNNER, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": TARGETS } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, RUNNER), 0)));
    }
}

// Classic #36 Burn (hand only): mana left after paying now (base), max mana (Radiant)

/// C #36 Burn lights up in hand when it would draw if played now (R195)
mod classic_c36_burn_lights_up_in_hand_when_it_would_draw_if_played_now_r195 {
    use super::*;

    const BURN: &str = "classic-036";
    const MONKEY: &str = "classic-077"; // Anti-Magic Monkey: Aura: Spells cost (1) more.
    const ANCHOR: &str = "core-010";
    const LIBRARY: [&str; 2] = ["core-008", "core-011"];

    fn at_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    fn drew(s: &Scenario) -> usize {
        count_of(s, "drawn")
    }

    #[test]
    fn r195_base_a_0_burn_with_4_mana_glows_and_played_now_it_draws() {
        let mut s = setup(
            json!({ "p1": { "hand": [BURN, ANCHOR], "library": LIBRARY }, "p2": { "hand": [ANCHOR] } }),
        );

        assert!(hand_glows(&s, &nth(&copies_in_hand(&s, BURN), 0)));
        s.play(BURN, json!({ "targets": at_hero() }));
        assert_eq!(drew(&s), 1);
    }

    #[test]
    fn r195_base_with_3_mana_it_does_not_glow_and_played_now_it_draws_nothing() {
        let mut s = setup(
            json!({ "p1": { "hand": [BURN, ANCHOR], "library": LIBRARY, "mana": 3 }, "p2": { "hand": [ANCHOR] } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, BURN), 0)));
        s.play(BURN, json!({ "targets": at_hero() }));
        assert_eq!(drew(&s), 0);
    }

    #[test]
    fn r195_r65_base_it_reads_the_mana_left_after_paying_its_price_now_a_burn_made_to_cost_1_with_4_mana_does_not_glow_and_draws_nothing()
     {
        let mut s = setup(
            json!({ "p1": { "hand": [{ "def": BURN, "costMod": 1 }, ANCHOR], "library": LIBRARY }, "p2": { "hand": [ANCHOR] } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, BURN), 0)));
        s.play(BURN, json!({ "targets": at_hero() }));
        assert_eq!(drew(&s), 0);
    }

    #[test]
    fn r195_r65_base_a_surcharge_moves_it_under_c_c77_anti_magic_monkey_a_burn_costs_1_and_4_mana_neither_glows_nor_draws()
     {
        let mut s = setup(
            json!({ "p1": { "hand": [BURN, ANCHOR], "library": LIBRARY }, "p2": { "hand": [ANCHOR], "field": [MONKEY] } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, BURN), 0)));
        s.play(BURN, json!({ "targets": at_hero() }));
        s.expect_mana("p1", 3);
        assert_eq!(drew(&s), 0);
    }

    #[test]
    fn r195_radiant_max_mana_4_glows_with_no_mana_left_and_played_now_it_draws() {
        let mut s = setup(
            json!({ "p1": { "hand": [{ "def": BURN, "radiant": true }, ANCHOR], "library": LIBRARY, "mana": 0 }, "p2": { "hand": [ANCHOR] } }),
        );

        assert!(hand_glows(&s, &nth(&copies_in_hand(&s, BURN), 0)));
        s.play(BURN, json!({ "targets": at_hero() }));
        assert_eq!(drew(&s), 1);
    }

    #[test]
    fn r195_radiant_max_mana_3_does_not_glow_however_much_mana_is_left_and_draws_nothing() {
        let mut s = setup(
            json!({ "turn": 5, "p1": { "hand": [{ "def": BURN, "radiant": true }, ANCHOR], "library": LIBRARY, "mana": 9 }, "p2": { "hand": [ANCHOR] } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, BURN), 0)));
        s.play(BURN, json!({ "targets": at_hero() }));
        assert_eq!(drew(&s), 0);
    }

    #[test]
    fn r195_on_the_opponents_turn_it_never_glows() {
        let s = setup(
            json!({ "active": "p2", "p1": { "hand": [BURN, ANCHOR], "library": LIBRARY }, "p2": { "hand": [ANCHOR] } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, BURN), 0)));
    }
}

// Classic #40 MC Tech (hand only): your opponent controls {threshold} or more permanents

/// C #40 MC Tech lights up in hand while the opponent controls enough permanents (R195)
mod classic_c40_mc_tech_lights_up_in_hand_while_the_opponent_controls_enough_permanents_r195 {
    use super::*;

    const TECH: &str = "classic-040";
    const ANCHOR: &str = "core-010";

    fn steals(s: &Scenario) -> usize {
        count_of(s, "controlChanged")
    }

    #[test]
    fn r195_base_4_enemy_permanents_glow_and_played_now_it_steals_one() {
        let mut s = setup(
            json!({ "p1": { "hand": [TECH, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": ["core-008", "core-019", "core-011"], "backrow": ["core-073"] } }),
        );

        assert!(hand_glows(&s, &nth(&copies_in_hand(&s, TECH), 0)));
        s.play(TECH, json!({ "zone": 5 }));
        assert_eq!(steals(&s), 1);
    }

    #[test]
    fn r195_base_3_enemy_permanents_do_not_glow_and_played_now_it_steals_nothing() {
        let mut s = setup(
            json!({ "p1": { "hand": [TECH, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": ["core-008", "core-019", "core-011"] } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, TECH), 0)));
        s.play(TECH, json!({ "zone": 5 }));
        assert_eq!(steals(&s), 0);
    }

    #[test]
    fn r195_r13_a_face_down_trap_counts_a_card_dormant_under_a_stack_does_not() {
        let s = setup(json!({
            "p1": { "hand": [TECH, ANCHOR] },
            "p2": { "hand": [ANCHOR], "field": ["core-008", { "def": "core-092", "stack": true }, "core-019"], "backrow": [{ "def": "core-096", "faceUp": false }] },
        }));

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, TECH), 0)));
    }

    #[test]
    fn r195_radiant_4_enemy_permanents_glow_and_played_now_it_asks_which_to_steal() {
        let mut s = setup(json!({
            "p1": { "hand": [{ "def": TECH, "radiant": true }, ANCHOR] },
            "p2": { "hand": [ANCHOR], "field": ["core-008", "core-019", "core-011"], "backrow": [{ "def": "core-096", "faceUp": false }] },
        }));

        assert!(hand_glows(&s, &nth(&copies_in_hand(&s, TECH), 0)));
        s.play(TECH, json!({ "zone": 5 }));
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.options.len()),
            Some(4)
        );
    }

    #[test]
    fn r195_on_the_field_it_never_glows() {
        let s = setup(
            json!({ "p1": { "hand": [ANCHOR], "field": [TECH] }, "p2": { "hand": [ANCHOR], "field": ["core-008", "core-019", "core-011", "core-001"] } }),
        );

        assert!(!glows(s.view("p1").you.units[0].as_ref()));
    }

    #[test]
    fn r195_on_the_opponents_turn_it_never_glows_however_many_permanents_they_control() {
        let s = setup(
            json!({ "active": "p2", "p1": { "hand": [TECH, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": ["core-008", "core-019", "core-011", "core-001"] } }),
        );

        assert!(!hand_glows(&s, &nth(&copies_in_hand(&s, TECH), 0)));
    }
}

// C+ #50 Adaptive Growth (hand only): you control fewer Units than your opponent

/// C+ #50 Adaptive Growth lights up while you control fewer Units (R195)
mod classic_plus_c50_adaptive_growth_lights_up_while_you_control_fewer_units_r195 {
    use super::*;

    const GROWTH: &str = "classicplus-050";
    const VANILLA: &str = "core-008";

    fn growth(mine: usize, theirs: usize, radiant: bool) -> Scenario {
        let card = if radiant {
            json!({ "def": GROWTH, "radiant": true })
        } else {
            json!(GROWTH)
        };
        setup(json!({
            "seed": format!("r195-cp050-{mine}-{theirs}-{radiant}"),
            "p1": { "hand": [card, "core-005"], "field": vec![VANILLA; mine] },
            "p2": { "hand": ["core-005"], "field": vec![VANILLA; theirs] },
        }))
    }

    #[test]
    fn r195_with_fewer_units_it_glows_in_hand_and_the_spell_then_gives_every_unit_2_2() {
        let mut s = growth(1, 2, false);
        let card = s.card(GROWTH).clone();
        assert!(hand_glows(&s, &card));
        s.play(GROWTH, json!({}));
        let mine = unit_ref(&s, "p1", 1);
        let theirs = unit_ref(&s, "p2", 1);
        s.expect_stats(mine, json!({ "attack": 2, "health": 2 }));
        s.expect_stats(theirs, json!({ "attack": 2, "health": 2 }));
    }

    #[test]
    fn r195_with_equal_counts_it_does_not_glow_and_the_spell_then_gives_every_unit_plus_2_plus_2() {
        let mut s = growth(1, 1, false);
        let card = s.card(GROWTH).clone();
        assert!(!hand_glows(&s, &card));
        s.play(GROWTH, json!({}));
        let mine = unit_ref(&s, "p1", 1);
        let theirs = unit_ref(&s, "p2", 1);
        s.expect_stats(mine, json!({ "attack": 6, "health": 6 }));
        s.expect_stats(theirs, json!({ "attack": 6, "health": 6 }));
    }

    #[test]
    fn r195_with_more_units_it_does_not_glow_either() {
        let s = growth(2, 1, false);
        let card = s.card(GROWTH).clone();
        assert!(!hand_glows(&s, &card));
    }

    #[test]
    fn r195_the_radiant_face_glows_on_the_same_count_fewer_and_only_the_enemy_units_get_3_3() {
        let mut s = growth(1, 2, true);
        let card = s.card(GROWTH).clone();
        assert!(hand_glows(&s, &card));
        s.play(GROWTH, json!({}));
        let mine = unit_ref(&s, "p1", 1);
        let theirs = unit_ref(&s, "p2", 1);
        s.expect_stats(mine, json!({ "attack": 4, "health": 4 }));
        s.expect_stats(theirs, json!({ "attack": 1, "health": 1 }));
    }

    #[test]
    fn r195_the_radiant_face_with_equal_counts_does_not_glow_and_gives_your_units_plus_3_plus_3() {
        let mut s = growth(1, 1, true);
        let card = s.card(GROWTH).clone();
        assert!(!hand_glows(&s, &card));
        s.play(GROWTH, json!({}));
        let mine = unit_ref(&s, "p1", 1);
        let theirs = unit_ref(&s, "p2", 1);
        s.expect_stats(mine, json!({ "attack": 7, "health": 7 }));
        s.expect_stats(theirs, json!({ "attack": 4, "health": 4 }));
    }

    #[test]
    fn r195_the_opponents_view_never_carries_the_flag_its_hand_is_a_count() {
        let s = growth(0, 2, false);
        assert_eq!(
            serde_json::to_value(&s.view("p2").opponent.hand).unwrap(),
            json!({ "count": 2 })
        );
    }
}

// The set of cards that declare the hook is R195's list

/// R195 the cards that declare conditionMet
mod r195_the_cards_that_declare_condition_met {
    // Classic+ #18 Gullible Treatler, #19.5 Bot Loser and #37 Wardrum prove theirs in their own card
    // files, and so do R662's Core #18, #60, #70, #85, #96 and #100.
    #[test]
    fn r195_r662_are_exactly_c10_c53_c68_c71_and_c93_classic_c22_c36_c40_and_c69_classic_plus_c18_c19_5_c37_and_c50_and_r662s_c18_c60_c70_c85_c96_and_c100_on_both_faces_so_a_new_hook_cannot_land_untested()
     {
        // One entry per script file present: the registry `scripts_of` builds.
        let cards = jackioh_cards::scripts_of();
        let mut hooked: Vec<String> = cards
            .iter()
            .filter(|(_, card)| card.base.condition_met.is_some() || card.radiant.condition_met.is_some())
            .map(|(id, _)| id.clone())
            .collect();
        hooked.sort();
        assert_eq!(
            hooked,
            [
                "classic-022",
                "classic-036",
                "classic-040",
                "classic-069",
                "classicplus-018",
                "classicplus-019-5",
                "classicplus-037",
                "classicplus-050",
                "core-010",
                "core-018",
                "core-053",
                "core-060",
                "core-068",
                "core-070",
                "core-071",
                "core-085",
                "core-093",
                "core-096",
                "core-100",
            ]
        );

        for id in &hooked {
            let card = cards.get(id);
            assert!(
                card.is_some_and(|card| card.base.condition_met.is_some()),
                "{id} base"
            );
            assert!(
                card.is_some_and(|card| card.radiant.condition_met.is_some()),
                "{id} radiant"
            );
        }
    }
}

// C #69 Plague Charger (field only): while it has a Plague Counter

/// C #69 Plague Charger lights up while it has a Plague Counter (R195)
mod classic_c69_plague_charger_lights_up_while_it_has_a_plague_counter_r195 {
    use super::*;

    const CHARGER: &str = "classic-069";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";

    fn hits_on(s: &Scenario, instance_id: &str) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == instance_id))
            .count()
    }

    fn with_a_token_it_glows_and_strikes_first(radiant: bool) {
        let face = if radiant { "radiant" } else { "base" };
        let mut s = setup(json!({
            "seed": format!("r195-c069-{face}-on"),
            "p1": { "hand": [FILLER], "field": [{ "def": CHARGER, "radiant": radiant, "counters": { "plague": 1 } }] },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }));

        assert!(glows(s.view("p1").you.units[0].as_ref()));
        let vanilla = s.card(VANILLA).clone();
        s.attack(CHARGER, &vanilla);
        s.expect_in_zone(VANILLA, "graveyard");
        let charger_id = s.card(CHARGER).id.clone();
        assert_eq!(hits_on(&s, &charger_id), 0);
    }

    fn with_no_token_it_is_dark_and_is_struck_back(radiant: bool) {
        let face = if radiant { "radiant" } else { "base" };
        let mut s = setup(json!({
            "seed": format!("r195-c069-{face}-off"),
            "p1": { "hand": [FILLER], "field": [{ "def": CHARGER, "radiant": radiant }] },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }));

        assert!(!glows(s.view("p1").you.units[0].as_ref()));
        let vanilla = s.card(VANILLA).clone();
        s.attack(CHARGER, &vanilla);
        let charger_id = s.card(CHARGER).id.clone();
        assert_eq!(hits_on(&s, &charger_id), 1);
    }

    #[test]
    fn r195_base_with_a_token_it_glows_on_the_field_and_it_then_strikes_first_a_4_4_dies_before_striking_back()
     {
        with_a_token_it_glows_and_strikes_first(false);
    }

    #[test]
    fn r195_base_with_no_token_it_does_not_glow_and_it_then_has_no_first_strike_the_4_4_strikes_back() {
        with_no_token_it_is_dark_and_is_struck_back(false);
    }

    #[test]
    fn r195_radiant_with_a_token_it_glows_on_the_field_and_it_then_strikes_first_a_4_4_dies_before_striking_back()
     {
        with_a_token_it_glows_and_strikes_first(true);
    }

    #[test]
    fn r195_radiant_with_no_token_it_does_not_glow_and_it_then_has_no_first_strike_the_4_4_strikes_back() {
        with_no_token_it_is_dark_and_is_struck_back(true);
    }

    #[test]
    fn r195_r78_in_hand_it_never_glows_a_card_there_holds_no_tokens() {
        let s = setup(json!({ "seed": "r195-c069-hand", "p1": { "hand": [CHARGER, FILLER] } }));
        let charger = s.card(CHARGER).clone();
        assert!(!hand_glows(&s, &charger));
    }

    #[test]
    fn r195_the_opponents_plagued_charger_carries_no_flag_in_your_view() {
        let s = setup(
            json!({ "seed": "r195-c069-theirs", "p1": { "hand": [FILLER] }, "p2": { "hand": [FILLER], "field": [{ "def": CHARGER, "counters": { "plague": 2 } }] } }),
        );
        assert!(!glows(s.view("p1").opponent.units[0].as_ref()));
        assert!(glows(s.view("p2").you.units[0].as_ref()));
    }
}
