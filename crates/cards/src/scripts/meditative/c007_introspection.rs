//! M #7 Introspection (SPEC §8.8 row 7, BUILD M10 row M 7). (4) Field Spell, Legendary.
//!   Base and Radiant: "End of turn: Heal your hero and each of your Units {heal}. Draw
//!   {draw|card|cards}. Cards in your hand cost ({discount}) less. Deal {damage} damage to each
//!   enemy." (3/8 step 2; 1; 1/2; 2/4)
//!
//! An `end_of_turn` hook, which runs only on its controller's turn (`turn.rs`), returning in the
//! written order: `heal` on `selfHero`, then each of the controller's Units (C+ #19.4 Support Loser's
//! list); `draw`; a `for_each_card` over the hand with `set_cost_mod { amount: −discount }`; and
//! `damage_all { side: enemy, heroes: true }`, where every hit lands before the state check (R59).
//! The draw comes before the discount, so the drawn card is cheaper too. The discount is a `costMod`
//! (R78: kept until a graveyard takes it off, R766), and it never reaches an X card (R65). One script
//! serves both faces: every number but the draw is declared per face and read through `param` (R386).

use jackioh_engine::effects::{
    ForEachCardArgs, damage_all, draw, for_each_card, heal, set_cost_mod,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-007";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            let heal_amount = param(&*ctx, "heal");
            let draws = param(&*ctx, "draw");
            let discount = param(&*ctx, "discount");
            let damage = param(&*ctx, "damage");
            vec![
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": heal_amount }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|each: &mut EffectContext<'_>| {
                        cards_in_scope(each, &json_as(json!({ "side": "self" })))
                            .into_iter()
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(move |instance_id: &str| {
                        heal(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "amount": heal_amount,
                        })))
                    }),
                }),
                draw(json_as(json!({ "count": draws }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|each: &mut EffectContext<'_>| {
                        zone_cards(&*each.state, each.controller, OffFieldZone::Hand)
                            .iter()
                            .map(|card| card.id.clone())
                            .collect()
                    }),
                    each: Arc::new(move |instance_id: &str| {
                        set_cost_mod(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "amount": -discount,
                        })))
                    }),
                }),
                damage_all(json_as(json!({ "amount": damage, "side": "enemy", "heroes": true }))),
            ]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared numbers, read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// M #7 Introspection — SPEC §8.8 row 7, BUILD M10 row M 7: "At your end of turn: heal your hero and
// each of your Units, draw, discount the whole hand (the drawn card included), then hit every enemy
// (R59); an X card keeps its cost (R65); nothing fires at the opponent's end of turn; radiant heals
// 8, discounts 2, hits 4 and still draws 1; its tuned numbers read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const INTROSPECTION: &str = "meditative-007";
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const FODDER: &str = "core-015"; // (1) 1/1 Unit.
    const FILLER: &str = "core-005"; // (1) Spell.
    const VANILLA: &str = "core-008"; // (1) Unit.
    const CHEAP: &str = "core-010"; // (0) Spell.
    const DIVIDEND: &str = "core-024"; // (X) Spell.

    mod meditative_007 {
        use super::*;

        fn setup(radiant: bool) -> Scenario {
            scenario(json!({
                "p1": {
                    "health": 20,
                    "hand": [FILLER, VANILLA],
                    "field": [{ "def": MENACE, "damage": 4 }],
                    "backrow": [{ "def": INTROSPECTION, "radiant": radiant, "faceUp": true, "lane": 1 }],
                    "library": [CHEAP, FILLER, FILLER],
                },
                "p2": { "hand": [FILLER], "field": [FODDER], "library": [FILLER, FILLER] },
            }))
        }

        #[test]
        fn heals_draws_discounts_the_whole_hand_then_hits_every_enemy() {
            crate::register_all();
            let mut s = setup(false);

            s.end_turn();

            // Heal 3: the hero from 20, the Menace from 5.
            s.expect_health(P1, 23);
            s.expect_stats(MENACE, json!({ "health": 8 }));
            // Draw 1, then discount 1 on the whole hand, the drawn card included.
            assert_eq!(s.hand(P1).len(), 3);
            for card in s.hand(P1) {
                assert_eq!(card.cost_mod, -1);
            }
            // Deal 2 to each enemy: the 1/1 dies, the hero drops to 28.
            s.expect_in_zone(FODDER, "graveyard");
            s.expect_health(P2, HERO_HEALTH - 2);
        }

        #[test]
        fn an_x_card_keeps_its_cost() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "health": 20,
                    "hand": [DIVIDEND, FILLER],
                    "backrow": [{ "def": INTROSPECTION, "faceUp": true, "lane": 1 }],
                    "library": [CHEAP, FILLER],
                },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));

            s.end_turn();

            assert_eq!(s.card(DIVIDEND).cost_mod, 0);
            assert_eq!(s.card(FILLER).cost_mod, -1);
        }

        #[test]
        fn nothing_at_the_opponents_end_of_turn() {
            crate::register_all();
            let mut s = setup(false);

            s.end_turn(); // p1's end: Introspection fires.
            s.expect_health(P1, 23);
            let hand = s.hand(P1).len();
            s.end_turn(); // p2's end: nothing of p1's fires; p1 draws for its new turn.

            assert_eq!(s.hand(P1).len(), hand + 1);
            s.expect_health(P1, 23);
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Damage { .. } | GameEvent::Healed { .. }))
            );
        }

        #[test]
        fn r386_numbers_read_through_param() {
            crate::register_all();
            let mut s = setup(false);
            step_param(s.card_mut(INTROSPECTION), "heal", 1);
            step_param(s.card_mut(INTROSPECTION), "draw", 1);
            step_param(s.card_mut(INTROSPECTION), "discount", 1);
            step_param(s.card_mut(INTROSPECTION), "damage", 1);

            s.end_turn();

            // Heal 3 + 2, two draws, discount 2, damage 3.
            s.expect_health(P1, 25);
            assert_eq!(s.hand(P1).len(), 4);
            for card in s.hand(P1) {
                assert_eq!(card.cost_mod, -2);
            }
            s.expect_health(P2, HERO_HEALTH - 3);
        }

        #[test]
        fn radiant_heals_8_discounts_2_hits_4_draws_1() {
            crate::register_all();
            let mut s = setup(true);

            s.end_turn();

            s.expect_health(P1, 28);
            // The Menace's 4 damage is healed past full, capped at 9.
            s.expect_stats(MENACE, json!({ "health": 9 }));
            assert_eq!(s.hand(P1).len(), 3);
            for card in s.hand(P1) {
                assert_eq!(card.cost_mod, -2);
            }
            s.expect_in_zone(FODDER, "graveyard");
            s.expect_health(P2, HERO_HEALTH - 4);
        }
    }
}
