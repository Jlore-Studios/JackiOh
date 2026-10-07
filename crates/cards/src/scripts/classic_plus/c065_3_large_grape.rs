//! C+ #65.3 Large Grape (SPEC §8.7 row 65.3). (3) Spell, Fruit, Token (printed Rare).
//!   Base:    "Choose a Unit or hero. If it's an enemy, deal {amount} damage to it; if it's yours, heal
//!            it {amount}. Draw {draw}. Each costs (0)."
//!   Radiant: the same text, amount 10 and draw 2.
//!   Engine:  "As C+ #65.2, the drawn card taking `costOverride` 0. Tunes: amount 5 ↑; draw 1 ↑."
//!
//! C+ #65.2 Normal Grape with a set price in place of the discount: each card a draw put in the hand
//! costs (0) (`costOverride`, R65), which an X-cost card's X never minds. A card cast on draw, a burned
//! card, a fatigue draw and a limited draw get nothing (`drawPriced`, effects/fruit.ts).

use jackioh_engine::effects::{damage_enemy_or_heal_friend, draw_priced};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-065-3";

/// §8.7: "Choose a Unit or hero" — either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// §8.7: "It costs (0)".
const SET_COST: i32 = 0;

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let mut effects = vec![damage_enemy_or_heal_friend(json_as(json!({ "amount": param(ctx, "amount") })))];
            effects.extend((0..param(ctx, "draw")).map(|_| draw_priced(json_as(json!({ "costOverride": SET_COST })))));
            effects
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 10 and 2 are its declared `amount` and `draw`, which `param` reads.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #65.3 Large Grape — SPEC §8.7 row 65.3, BUILD M9 Classic+ row C+ 65.3: "As Normal Grape with 5 and
// 5, and the drawn card costs (0) (`costOverride` 0); amount and draw read through `param()`; radiant 10
// and 10, draw 2, and each drawn card costs (0)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GRAPE: &str = "classicplus-065-3";
    const FILLER: &str = "core-005";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const HINDER: &str = "core-021"; // Cast on draw; base face makes its caster discard 1 at random (R682)
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const DECK_A: &str = "core-043"; // (4) Unit, Big Felinor
    const DECK_B: &str = "core-025"; // (4) Unit
    const BILLY: &str = "classicplus-069"; // Buff Billy, an (X) Unit

    /// An engine value as the JSON TS compares it with.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn unit_at(s: &Scenario, player: PlayerId) -> Value {
        let Some(unit) = s.unit(player, 1) else {
            panic!("no unit in {player} lane 1");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    fn radiant_grape() -> Value {
        json!({ "def": GRAPE, "radiant": true })
    }

    fn fillers(n: usize) -> Vec<Value> {
        (0..n).map(|_| json!(FILLER)).collect()
    }

    fn has_event(events: &[GameEvent], kind: &str) -> bool {
        events.iter().any(|event| event.event_type().as_str() == kind)
    }

    /// TS `stepParam(s.card(ref), key, steps)`: the live card in the state, tuned in place.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let instance = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        step_param(instance, key, steps);
    }

    fn zone_of(s: &Scenario, card: &str) -> ZoneName {
        s.card(card).zone.z()
    }

    #[test]
    fn is_a_3_fruit_spell_token_printed_rare_that_targets_any_unit_or_hero_one_script_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, GRAPE);
        assert_eq!(js(&def.cost), json!(3));
        assert_eq!(js(&def.tags), json!(["Fruit", "Token"]));
        assert_eq!(js(&def.printed_rarity), json!("Rare"));
        let CardScripts { base, radiant } = script();
        assert_eq!(
            js(&base.targets),
            json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }])
        );
        assert!(Arc::ptr_eq(base.cry.as_ref().unwrap(), radiant.cry.as_ref().unwrap()));
    }

    mod base {
        use super::*;

        #[test]
        fn an_enemy_unit_takes_5_then_you_draw_1_that_costs_0() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_B] }, "p2": { "hand": [FILLER], "field": [MENACE] } }));
            let targets = unit_at(&s, P2);
            s.play(GRAPE, json!({ "targets": targets }));
            s.expect_stats(MENACE, json!({ "health": 4 }));
            let drawn = s.card(DECK_B).clone();
            assert_eq!(drawn.zone.z(), ZoneName::Hand);
            assert_eq!(drawn.cost_override, Some(0));
            let hand = js(&s.view(P1).you.hand);
            assert!(
                hand.as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instanceId"] == json!(drawn.id) && card["cost"] == json!(0))
            );
        }

        #[test]
        fn the_enemy_hero_takes_5_spell_damage_2_makes_it_7() {
            crate::register_all();
            let mut plain = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_B] }, "p2": { "hand": [FILLER] } }));
            plain.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            plain.expect_health(P2, 25);

            let mut raised = scenario(json!({
                "p1": { "hand": [GRAPE, FILLER], "field": [SOLARIUS], "library": [DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            raised.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            raised.expect_health(P2, 23);
        }

        #[test]
        fn r19_a_friendly_unit_is_healed_5_and_your_hero_5_past_30() {
            crate::register_all();
            let mut unit = scenario(json!({
                "p1": { "hand": [GRAPE, FILLER], "field": [{ "def": MENACE, "damage": 6 }], "library": [DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            let targets = unit_at(&unit, P1);
            unit.play(GRAPE, json!({ "targets": targets }));
            unit.expect_stats(MENACE, json!({ "health": 8 }));

            let mut hero = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_B] }, "p2": { "hand": [FILLER] } }));
            hero.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            hero.expect_health(P1, 35);
        }

        #[test]
        fn r596_a_burned_card_a_fatigue_draw_and_a_card_cast_on_draw_nor_the_card_its_draw_then_brings_get_no_price() {
            crate::register_all();
            let mut hand = vec![json!(GRAPE)];
            hand.extend(fillers(10));
            let mut burned = scenario(json!({ "p1": { "hand": hand, "library": [DECK_B] }, "p2": { "hand": [FILLER] } }));
            burned.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert_eq!(zone_of(&burned, DECK_B), ZoneName::Graveyard);
            assert_eq!(burned.card(DECK_B).cost_override, None);

            let mut fatigue = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [] }, "p2": { "hand": [FILLER] } }));
            fatigue.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert!(!has_event(fatigue.last_events(), "costChanged"));

            let mut cast = scenario(json!({
                "p1": { "hand": [GRAPE, FILLER], "library": [{ "def": HINDER, "radiant": true }, DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            cast.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert_eq!(zone_of(&cast, HINDER), ZoneName::Graveyard);
            assert_eq!(cast.card(DECK_B).cost_override, None);
        }

        #[test]
        fn r65_an_x_cost_card_it_draws_is_free_to_play_its_x_still_chosen_buff_billy_played_for_1_with_1_mana_left_costs_nothing() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [BILLY] }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert_eq!(s.card(BILLY).cost_override, Some(0));
            s.expect_mana(P1, 1);
            s.play(BILLY, json!({ "zone": 1, "x": 1 }));
            s.expect_mana(P1, 1);
            assert_eq!(s.card(BILLY).x, Some(1));
        }

        #[test]
        fn r386_an_upgrade_makes_it_6_and_2_draws_a_degrade_4() {
            crate::register_all();
            let mut up = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_A, DECK_B] }, "p2": { "hand": [FILLER] } }));
            step(&mut up, GRAPE, "amount", 1);
            step(&mut up, GRAPE, "draw", 1);
            up.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            up.expect_health(P2, 24);
            assert_eq!([up.card(DECK_A).cost_override, up.card(DECK_B).cost_override], [Some(0), Some(0)]);

            let mut down = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_A, DECK_B] }, "p2": { "hand": [FILLER] } }));
            step(&mut down, GRAPE, "amount", -1);
            down.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            down.expect_health(P2, 26);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn an_enemy_takes_10_you_draw_2_and_each_costs_0() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [radiant_grape(), FILLER], "library": [DECK_A, DECK_B] },
                "p2": { "hand": [FILLER], "field": [{ "def": MENACE }] }
            }));
            let targets = unit_at(&s, P2);
            s.play(GRAPE, json!({ "targets": targets }));
            assert!(s.unit(P2, 1).is_none());
            assert_eq!(s.card(DECK_A).cost_override, Some(0));
            assert_eq!(s.card(DECK_B).cost_override, Some(0));
        }

        #[test]
        fn r19_a_friend_is_healed_10() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [radiant_grape(), FILLER], "library": [DECK_A, DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, 40);
        }

        #[test]
        fn r58_a_cast_on_draw_card_casts_free_with_no_prompt_r682_its_chain_s_repeat_brings_no_price_the_grape_s_own_draw_does() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [radiant_grape(), FILLER], "library": [HINDER, DECK_A, DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            // Base Hinder's discard is random (R682): no prompt opens mid-list.
            assert!(s.state().pending.is_none());
            let revived: GameState = serde_json::from_value(js(s.state())).expect("the state round-trips");
            assert_eq!(&revived, s.state());
            assert_eq!(hash_state(&revived), hash_state(s.state()));
            // Hinder was cast (no price) and discarded the one card held; its chain's repeat brought
            // DECK_A (no price, R596); the grape's own second draw brought DECK_B, priced (0).
            assert_eq!(zone_of(&s, HINDER), ZoneName::Graveyard);
            assert_eq!(zone_of(&s, FILLER), ZoneName::Graveyard);
            assert_eq!(zone_of(&s, DECK_A), ZoneName::Hand);
            assert_eq!(s.card(DECK_A).cost_override, None);
            assert_eq!(zone_of(&s, DECK_B), ZoneName::Hand);
            assert_eq!(s.card(DECK_B).cost_override, Some(0));
        }
    }
}
