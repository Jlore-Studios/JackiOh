//! C+ #65.2 Normal Grape (SPEC §8.7 row 65.2). (1) Spell, Fruit, Token (printed Common).
//!   Base:    "Deal {amount} damage to an enemy or heal an ally {amount}. Draw {draw}. Reduce its cost
//!            by (1)."
//!   Radiant: the same text, amount 4 and draw 4: "… Draw {draw}. Reduce their cost by (1)."
//!   Engine:  "The target, any unit or hero, is declared at play (R81); an enemy is one the opponent
//!            controls, the heroes included. Each drawn card gets `costMod` −1 (never an X-cost card,
//!            R65) — one card, "its cost", on the base face; four cards, "their cost", on the Radiant;
//!            a card cast on draw never reaches the hand (R58) and a burned one isn't there, so
//!            neither gets it. Tunes: amount 2 ↑; draw 1 ↑."
//!
//! The target travels in the play (R81): any Unit or hero on either side. Whether it is an enemy is read
//! as the Spell resolves (`damageEnemyOrHealFriend`, effects/fruit.ts): an enemy takes one §4.4 hit from
//! this Spell (Spell Damage raises it), a friend is healed (§6.3 Heal, R19).
//!
//! "Draw N" is N separate draws (§2.4), one `drawPriced` each, so a cast-on-draw card that asks pauses
//! the rest of the list and the answer makes the rest (R113, R117). Each draw prices only the card IT
//! put in the hand: `costMod` −1, which R65 never lets reach an X-cost card; a card cast on draw, a
//! burned card, a fatigue draw and a limited draw get nothing.

use jackioh_engine::effects::{damage_enemy_or_heal_friend, draw_priced};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-065-2";

/// §8.7: "Choose a Unit or hero" — either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// §8.7: "It costs (1) less" — a discount that stacks with every other modifier (R65).
const DISCOUNT: i32 = -1;

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let mut effects = vec![damage_enemy_or_heal_friend(json_as(json!({ "amount": param(ctx, "amount") })))];
            effects.extend((0..param(ctx, "draw")).map(|_| draw_priced(json_as(json!({ "costMod": DISCOUNT })))));
            effects
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 4 and 2 are its declared `amount` and `draw`, which `param` reads.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #65.2 Normal Grape — SPEC §8.7 row 65.2, BUILD M9 Classic+ row C+ 65.2: "A target Unit or hero
// chosen with the play: an enemy takes 2 damage (Spell Damage raises it), one of yours is healed 2;
// then you draw 1 and that card gets `costMod` −1; a card cast on draw, a burned card or a fatigue draw
// gets no discount; amount and draw read through `param()`; radiant 4 and 4, draw 4, each drawn card
// costs (1) less".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GRAPE: &str = "classicplus-065-2";
    const FILLER: &str = "core-005"; // (1) Spell
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const HINDER: &str = "core-021"; // Cast on draw; base face makes its caster discard 1 at random (R682)
    const SOLARIUS: &str = "classicplus-038"; // Unit printing Spell Damage +2
    const DECK_A: &str = "core-011"; // Tempo Timmy, (1) Unit
    const DECK_B: &str = "core-001"; // Big D-fender, (2) Unit
    const DECK_C: &str = "core-002"; // Bigot, (2) Unit
    const DECK_D: &str = "core-003"; // Right-house defender, (1) Unit
    const BILLY: &str = "classicplus-069"; // Buff Billy, an (X) Unit

    /// An engine value as the JSON TS compares it with.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit in {player} lane {lane}");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    fn grape(radiant: bool) -> Value {
        if radiant { json!({ "def": GRAPE, "radiant": true }) } else { json!({ "def": GRAPE }) }
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
    fn is_a_1_fruit_spell_token_printed_common_that_targets_any_unit_or_hero_one_script_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, GRAPE);
        assert_eq!(js(&def.cost), json!(1));
        assert_eq!(js(&def.tags), json!(["Fruit", "Token"]));
        assert_eq!(js(&def.printed_rarity), json!("Common"));
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
        fn an_enemy_unit_takes_2_damage_then_you_draw_1_that_costs_1_less() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_B] }, "p2": { "hand": [FILLER], "field": [MENACE] } }));
            let targets = unit_at(&s, P2, 1);
            s.play(GRAPE, json!({ "targets": targets }));

            s.expect_stats(MENACE, json!({ "health": 7 }));
            let drawn = s.card(DECK_B).clone();
            assert_eq!(drawn.zone.z(), ZoneName::Hand);
            assert_eq!(drawn.cost_mod, -1);
            let hand = js(&s.view(P1).you.hand);
            assert!(
                hand.as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instanceId"] == json!(drawn.id) && card["cost"] == json!(1))
            );
            s.expect_events(json!(["damage", "drawn", "costChanged"]));
        }

        #[test]
        fn the_enemy_hero_takes_2_damage() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_B] }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, 28).expect_health(P1, 30);
        }

        #[test]
        fn r19_one_of_your_units_is_healed_2_never_hit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GRAPE, FILLER], "field": [{ "def": MENACE, "damage": 5 }], "library": [DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            let targets = unit_at(&s, P1, 1);
            s.play(GRAPE, json!({ "targets": targets }));
            s.expect_stats(MENACE, json!({ "health": 6 }));
            assert!(!has_event(s.last_events(), "damage"));
        }

        #[test]
        fn r19_your_own_hero_is_healed_2_past_30() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_B] }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, 32).expect_health(P2, 30);
        }

        #[test]
        fn e6_spell_damage_raises_the_hit_on_an_enemy_spell_damage_2_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GRAPE, FILLER], "field": [SOLARIUS], "library": [DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn e6_spell_damage_never_raises_the_heal_on_a_friend() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GRAPE, FILLER], "field": [SOLARIUS], "library": [DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, 32);
        }

        #[test]
        fn s2_4_r4_a_burned_card_gets_no_discount() {
            crate::register_all();
            let mut hand = vec![json!(GRAPE)];
            hand.extend(fillers(10));
            let mut s = scenario(json!({ "p1": { "hand": hand, "library": [DECK_B] }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            let drawn = s.card(DECK_B);
            assert_eq!(drawn.zone.z(), ZoneName::Graveyard);
            assert_eq!(drawn.cost_mod, 0);
        }

        #[test]
        fn r65_an_x_cost_card_it_draws_gets_no_discount_buff_billy_played_for_2_still_costs_2() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [BILLY] }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_mana(P1, 3);
            s.play(BILLY, json!({ "zone": 1, "x": 2 }));
            s.expect_mana(P1, 1);
        }

        #[test]
        fn s2_4_a_fatigue_draw_brings_no_card_so_nothing_is_discounted() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [] }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert!(has_event(s.last_events(), "fatigue"));
            assert!(!has_event(s.last_events(), "costChanged"));
            assert!(s.hand(P1).iter().all(|card| card.cost_mod == 0));
        }

        #[test]
        fn r596_a_card_cast_on_draw_never_reaches_the_hand_neither_it_nor_the_card_its_draw_then_brings_is_discounted() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GRAPE, FILLER], "library": [{ "def": HINDER, "radiant": true }, DECK_B] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_in_zone(HINDER, "graveyard");
            assert_eq!(s.card(HINDER).cost_mod, 0);
            assert_eq!(zone_of(&s, DECK_B), ZoneName::Hand);
            assert_eq!(s.card(DECK_B).cost_mod, 0);
        }

        #[test]
        fn r97_the_opponent_sees_the_draw_under_the_sentinel_and_no_price_on_a_card_they_can_t_read() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_B] }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            let theirs = s.view(P2);
            let drawn_id = s.card(DECK_B).id.clone();
            assert!(!serde_json::to_string(&theirs).unwrap().contains(&drawn_id));
            assert_eq!(js(&theirs.opponent.hand), json!({ "count": 2 }));
        }

        #[test]
        fn r386_an_upgrade_makes_it_3_and_2_draws_a_degrade_never_takes_either_below_1() {
            crate::register_all();
            let mut up = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_A, DECK_B] }, "p2": { "hand": [FILLER] } }));
            step(&mut up, GRAPE, "amount", 1);
            step(&mut up, GRAPE, "draw", 1);
            up.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            up.expect_health(P2, 27);
            assert_eq!([up.card(DECK_A).cost_mod, up.card(DECK_B).cost_mod], [-1, -1]);

            let mut down = scenario(json!({ "p1": { "hand": [GRAPE, FILLER], "library": [DECK_A, DECK_B] }, "p2": { "hand": [FILLER] } }));
            step(&mut down, GRAPE, "amount", -3);
            step(&mut down, GRAPE, "draw", -3);
            down.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            down.expect_health(P2, 29);
            assert_eq!(zone_of(&down, DECK_A), ZoneName::Hand);
            assert_eq!(zone_of(&down, DECK_B), ZoneName::Library);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn an_enemy_takes_4_you_draw_4_and_each_costs_1_less() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [grape(true), FILLER], "library": [DECK_A, DECK_B, DECK_C, DECK_D] },
                "p2": { "hand": [FILLER], "field": [MENACE] }
            }));
            let targets = unit_at(&s, P2, 1);
            s.play(GRAPE, json!({ "targets": targets }));
            s.expect_stats(MENACE, json!({ "health": 5 }));
            assert_eq!(
                [s.card(DECK_A).cost_mod, s.card(DECK_B).cost_mod, s.card(DECK_C).cost_mod, s.card(DECK_D).cost_mod],
                [-1, -1, -1, -1]
            );
        }

        #[test]
        fn r19_a_friend_is_healed_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [grape(true), FILLER], "library": [DECK_A, DECK_B, DECK_C, DECK_D] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, 34);
            assert_eq!(
                [zone_of(&s, DECK_A), zone_of(&s, DECK_B), zone_of(&s, DECK_C), zone_of(&s, DECK_D)],
                [ZoneName::Hand, ZoneName::Hand, ZoneName::Hand, ZoneName::Hand]
            );
        }

        #[test]
        fn s2_4_each_draw_prices_its_own_card_the_first_keeps_its_discount_the_rest_burn() {
            crate::register_all();
            let mut hand = vec![grape(true)];
            hand.extend(fillers(9));
            let mut s = scenario(json!({
                "p1": { "hand": hand, "library": [DECK_A, DECK_B, DECK_C, DECK_D] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert_eq!(zone_of(&s, DECK_A), ZoneName::Hand);
            assert_eq!(s.card(DECK_A).cost_mod, -1);
            for id in [DECK_B, DECK_C, DECK_D] {
                assert_eq!(zone_of(&s, id), ZoneName::Graveyard);
                assert_eq!(s.card(id).cost_mod, 0);
            }
        }

        #[test]
        fn r58_a_cast_on_draw_card_casts_free_with_no_prompt_r682_its_chain_s_repeat_brings_no_discount_the_grape_s_own_draws_do() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [grape(true), FILLER], "library": [HINDER, DECK_A, DECK_B, DECK_C, DECK_D] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(GRAPE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            // Base Hinder's discard is random (R682): no prompt opens mid-list.
            assert!(s.state().pending.is_none());

            let revived: GameState = serde_json::from_value(js(s.state())).expect("the state round-trips");
            assert_eq!(&revived, s.state());
            assert_eq!(hash_state(&revived), hash_state(s.state()));

            // Hinder was cast (no discount) and discarded the one card held; its chain's repeat brought
            // DECK_A (no discount, R596); the grape's own remaining draws brought DECK_B/C/D, discounted.
            assert_eq!(zone_of(&s, HINDER), ZoneName::Graveyard);
            assert_eq!(s.card(HINDER).cost_mod, 0);
            assert_eq!(zone_of(&s, FILLER), ZoneName::Graveyard);
            assert_eq!(zone_of(&s, DECK_A), ZoneName::Hand);
            assert_eq!(s.card(DECK_A).cost_mod, 0);
            for id in [DECK_B, DECK_C, DECK_D] {
                assert_eq!(zone_of(&s, id), ZoneName::Hand);
                assert_eq!(s.card(id).cost_mod, -1);
            }
            s.expect_health(P2, 26);
        }
    }
}
