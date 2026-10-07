//! C+ #36.1 Bone Storm (SPEC §8.7 row 36.1): (1) Spell, Token (printed Rare), from C+ #36.
//!   Base:    "Cast on draw: Deal {damage} damage to each enemy."
//!   Radiant: "Echo 1" plus the same: it resolves a second time.
//! Cast on draw is §6.2's static flag (R58, R70); "each enemy" is one hit on every enemy unit and
//! the enemy hero, all landing before the state check (R59). Echo is the `echo` flag (R51's repeat).

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-036-1";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            vec![damage_all(json_as(json!({
                "amount": param(&*ctx, "damage"),
                "side": "enemy",
                "heroes": true,
            })))]
        })),
        ..Script::default()
    };
    let radiant = Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };
    CardScripts { base, radiant }
}

// C+ #36.1 Bone Storm — SPEC §8.7 row 36.1, BUILD M9 Classic+ row C+ 36.1: "Cast on draw (R70: free,
// counts as played): 1 damage to the enemy hero and to each enemy Unit, separate instances all
// landing before the state check, then you draw again; one draw casts at most 20 in a row
// (`CAST_ON_DRAW_CHAIN_CAP`, R58), the next going to hand uncast (burned when the hand is full);
// played from a hand it does the same; Spell Damage raises each hit; the damage reads through
// `param()`; radiant Echo: it resolves a second time".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BONE_STORM: &str = "classicplus-036-1";
    const SOLARIUS: &str = "classicplus-038";
    const MENACE: &str = "core-019"; // 9/9 Taunt.
    const TIMMY: &str = "core-011"; // a 3/3 Unit that keeps p1's turns alive.
    const FILLER: &str = "core-005";

    /// The harness's `scenario`, with the shipped cards registered first (the TS harness did it at import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn storm(radiant: bool) -> Value {
        json!({ "def": BONE_STORM, "radiant": radiant })
    }

    /// p1 with the given library, an enemy Menace on the board; run to p1's next draw.
    fn draw_storm(library: Vec<Value>, field: Option<Vec<&str>>, hand: Option<Vec<&str>>) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": hand.unwrap_or_else(|| vec![FILLER]), "field": field.unwrap_or_else(|| vec![TIMMY]), "library": library },
            "p2": { "hand": [FILLER], "field": [MENACE], "library": [FILLER, FILLER, FILLER] },
        }));
        s.end_turn(); // p2's turn.
        s.end_turn(); // p1's turn: the draw.
        s
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    fn damage_events(s: &Scenario) -> Vec<Value> {
        events_json(s).into_iter().filter(|event| event["type"] == "damage").collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r70_drawn_it_casts_itself_for_free_1_to_the_enemy_hero_and_1_to_each_enemy_unit_then_you_draw_again() {
            let mut s = draw_storm(vec![storm(false), json!(FILLER), json!(FILLER)], None, None);
            s.expect_health(P2, HERO_HEALTH - 1).expect_health(P1, HERO_HEALTH);
            s.expect_stats(MENACE, json!({ "health": 8 }));
            s.expect_stats(TIMMY, json!({ "health": 3 })); // your own side untouched
            assert_eq!(s.card(BONE_STORM).zone.z(), ZoneName::Graveyard);
            // Then you draw again: the next card arrived in hand.
            assert_eq!(def_ids(&s.hand(P1)), vec![FILLER, FILLER]);
            assert_eq!(s.pile(P1, "library").len(), 1);
            // It counts as played and paid nothing (R70).
            let id = s.card(BONE_STORM).id.clone();
            assert!(s.state().players.p1.turn_log.played_ids.contains(&id));
            s.expect_mana(P1, 4);
        }

        #[test]
        fn r59_each_enemy_takes_its_own_hit_all_landing_before_the_state_check() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [TIMMY], "library": [storm(false), FILLER] },
                "p2": { "hand": [FILLER], "field": ["core-t-sheep", "core-t-sheep", MENACE], "library": [FILLER, FILLER] },
            }));
            s.end_turn();
            let from = s.events().len();
            s.end_turn();
            let after: Vec<Value> = events_json(&s)[from..].to_vec();
            let hits: Vec<usize> =
                after.iter().enumerate().filter(|(_, event)| event["type"] == "damage").map(|(index, _)| index).collect();
            let deaths: Vec<usize> =
                after.iter().enumerate().filter(|(_, event)| event["type"] == "destroyed").map(|(index, _)| index).collect();
            assert_eq!(hits.len(), 4); // two Sheep, Menace, the hero
            assert_eq!(deaths.len(), 2);
            assert!(hits.iter().max() < deaths.iter().min());
        }

        #[test]
        fn r58_one_draw_casts_at_most_20_in_a_row_the_next_goes_to_hand_uncast() {
            let library: Vec<Value> = (0..CAST_ON_DRAW_CHAIN_CAP + 3).map(|_| storm(false)).collect();
            let mut s = draw_storm(library, None, None);
            s.expect_health(P2, HERO_HEALTH - CAST_ON_DRAW_CHAIN_CAP);
            let in_hand = s.hand(P1).into_iter().filter(|card| card.def_id == BONE_STORM).count();
            assert_eq!(in_hand, 1);
            assert_eq!(s.pile(P1, "library").len(), 2);
        }

        #[test]
        fn r58_it_is_cast_even_with_a_full_hand_and_the_card_after_the_cap_is_burned_when_the_hand_is_full() {
            let full: Vec<&str> = (0..HAND_CAP).map(|_| FILLER).collect();
            let library: Vec<Value> = (0..CAST_ON_DRAW_CHAIN_CAP + 1).map(|_| storm(false)).collect();
            let mut s = draw_storm(library, None, Some(full));
            s.expect_health(P2, HERO_HEALTH - CAST_ON_DRAW_CHAIN_CAP);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert!(events_json(&s).iter().any(|event| event["type"] == "burned" && event["defId"] == BONE_STORM));
        }

        #[test]
        fn played_from_a_hand_it_does_the_same_for_its_cost_1() {
            let mut s = scenario(json!({ "p1": { "hand": [BONE_STORM, FILLER] }, "p2": { "hand": [FILLER], "field": [MENACE] } }));
            s.play(BONE_STORM, json!({}));
            s.expect_health(P2, HERO_HEALTH - 1).expect_stats(MENACE, json!({ "health": 8 })).expect_mana(P1, 3);
            assert_eq!(s.card(BONE_STORM).zone.z(), ZoneName::Graveyard);
        }

        #[test]
        fn spell_damage_raises_each_hit_with_solarius_2_every_enemy_takes_3() {
            let mut s = draw_storm(vec![storm(false), json!(FILLER)], Some(vec![SOLARIUS]), None);
            s.expect_health(P2, HERO_HEALTH - 3).expect_stats(MENACE, json!({ "health": 6 }));
        }

        #[test]
        fn the_opponents_bone_storm_hits_you_never_its_own_side() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [TIMMY], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE], "library": [storm(false), FILLER] },
            }));
            s.end_turn(); // p2 draws the storm.
            s.expect_health(P1, HERO_HEALTH - 1).expect_health(P2, HERO_HEALTH);
            s.expect_stats(TIMMY, json!({ "health": 2 })).expect_stats(MENACE, json!({ "health": 9 }));
        }

        #[test]
        fn r386_the_damage_reads_through_param_an_upgrade_makes_each_hit_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [TIMMY], "library": [storm(false), FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE], "library": [FILLER, FILLER] },
            }));
            step_param(s.card_mut(BONE_STORM), "damage", 1);
            s.end_turn();
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH - 2).expect_stats(MENACE, json!({ "health": 7 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn echo_it_resolves_a_second_time_hitting_every_enemy_twice() {
            let mut s = draw_storm(vec![storm(true), json!(FILLER), json!(FILLER)], None, None);
            s.expect_health(P2, HERO_HEALTH - 2).expect_stats(MENACE, json!({ "health": 7 }));
            assert_eq!(damage_events(&s).into_iter().filter(|event| event["amount"] == 1).count(), 4);
        }

        #[test]
        fn played_from_hand_echo_repeats_it_too() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BONE_STORM, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            s.play(BONE_STORM, json!({}));
            s.expect_health(P2, HERO_HEALTH - 2).expect_stats(MENACE, json!({ "health": 7 }));
        }
    }
}
