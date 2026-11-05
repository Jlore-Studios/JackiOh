//! M #7 Introspection (SPEC §8.8 row 7, BUILD M10 row M 7). (4) Field Spell, Legendary.
//!   Base and Radiant: "Start of turn: Heal your hero and each of your Units {heal}. Draw
//!   {draw|card|cards}. Cards in your hand cost ({discount}) less. Deal {damage} damage to each
//!   enemy." (1/2; 1/2; 1/2; 1/2)
//!
//! A `start_of_turn` hook, which runs only on its controller's turn (`turn.rs`) and before the turn's draw
//! (R62), in the written order; the Units' heal walks C+ #19.4 Support Loser's list, and every hit lands
//! before the state check (R59). The hook's own draw comes before the discount, so the card it draws is
//! cheaper too, but the turn's draw, which follows, is not (R62). The discount is a
//! `costMod` (R78: kept until a graveyard takes it off, R766), and it never reaches an X card (R65). One
//! script serves both faces: every number is declared per face and read through `param` (R386). Through
//! v0.5.11o it was an `end_of_turn` hook (`script_v0_5_11o`, R767000).

use jackioh_engine::effects::{
    ForEachCardArgs, damage_all, draw, for_each_card, heal, set_cost_mod,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-007";

/// The heal, the draw, the discount and the damage, in the order they are written.
fn introspect() -> Hook {
    hook(|ctx| {
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
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(introspect()),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared numbers, read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

/// R767000: through v0.5.11o the hook ran at the end of its controller's turn; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    let base = Script {
        end_of_turn: Some(introspect()),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #7 Introspection — SPEC §8.8 row 7, BUILD M10 row M 7 (R59, R62, R65, #2657, R386, R767000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const INTROSPECTION: &str = "meditative-007";
    // A plain 14/14 with no text: Midrange Menace heals itself to full at the same end of turn, which
    // would hide the heal.
    const GENN: &str = "classic-086";
    const FODDER: &str = "core-015"; // (1) 1/1 Unit.
    const FILLER: &str = "core-005"; // (1) Spell.
    const VANILLA: &str = "core-008"; // (1) Unit.
    const CHEAP: &str = "core-010"; // (0) Spell.
    const DIVIDEND: &str = "core-024"; // (X) Spell.

    /// The `costMod`s of p1's hand, in hand order.
    fn mods(s: &Scenario) -> Vec<i32> {
        s.hand(P1).iter().map(|card| card.cost_mod).collect()
    }

    mod meditative_007 {
        use super::*;

        fn setup(radiant: bool) -> Scenario {
            scenario(json!({
                "p1": {
                    "health": 20,
                    "hand": [FILLER, VANILLA],
                    "field": [{ "def": GENN, "damage": 4 }],
                    "backrow": [{ "def": INTROSPECTION, "radiant": radiant, "faceUp": true, "lane": 1 }],
                    "library": [CHEAP, FILLER, FILLER],
                },
                "p2": { "hand": [FILLER], "field": [FODDER], "library": [FILLER, FILLER] },
            }))
        }

        #[test]
        fn heals_draws_discounts_the_hand_then_hits_every_enemy_at_the_start_of_your_turn() {
            crate::register_all();
            let mut s = setup(false);

            s.start_turn();

            // Heal 1: the hero from 20, Genn from 10.
            s.expect_health(P1, 21);
            s.expect_stats(GENN, json!({ "health": 11 }));
            // Its draw of 1 (the Cheap) and the turn's own draw: four cards in hand.
            assert_eq!(s.hand(P1).len(), 4);
            // Deal 1 to each enemy: the 1/1 dies, the hero drops to 29.
            s.expect_in_zone(FODDER, "graveyard");
            s.expect_health(P2, HERO_HEALTH - 1);
        }

        #[test]
        fn r62_it_runs_before_the_turn_s_draw_so_the_discount_does_not_reach_that_draw() {
            crate::register_all();
            let mut s = setup(false);

            s.start_turn();

            // The hand it was in (the Stockpile and the Vanilla) and the card its own draw took (the Cheap)
            // cost (1) less; the card the turn then drew, the Stockpile under it, costs what it printed.
            assert_eq!(def_ids_of(&s), vec![FILLER, VANILLA, CHEAP, FILLER]);
            assert_eq!(mods(&s), vec![-1, -1, -1, 0]);
        }

        fn def_ids_of(s: &Scenario) -> Vec<String> {
            s.hand(P1).iter().map(|card| card.def_id.clone()).collect()
        }

        #[test]
        fn an_x_card_keeps_its_cost() {
            let mut s = scenario(json!({
                "p1": {
                    "health": 20,
                    "hand": [DIVIDEND, FILLER],
                    "backrow": [{ "def": INTROSPECTION, "faceUp": true, "lane": 1 }],
                    "library": [CHEAP, FILLER],
                },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));

            let filler = s.card(FILLER).id.clone();
            s.start_turn();
            assert_eq!(s.card(&filler).cost_mod, -1);

            // R65: an X card ignores the discount — played for 2, it costs exactly 2.
            s.state_mut().players.p1.mana.current = 4;
            s.play(
                DIVIDEND,
                json!({ "x": 2, "modes": ["damage"], "targets": [{ "pick": "hero", "player": "p2" }] }),
            );
            s.expect_mana(P1, 2);
        }

        #[test]
        fn nothing_at_the_end_of_your_turn_nor_at_the_opponents_start_or_end_of_turn() {
            crate::register_all();
            let mut s = setup(false);

            s.end_turn(); // p1's end of turn: it fires no longer; p2's start of turn follows.
            s.expect_health(P1, 20);
            s.expect_health(P2, HERO_HEALTH);
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(mods(&s), vec![0, 0]);
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Damage { .. } | GameEvent::Healed { .. }))
            );
            s.end_turn(); // p2's end of turn; then p1's own start of turn, when it fires.

            s.expect_health(P1, 21);
            s.expect_health(P2, HERO_HEALTH - 1);
            assert_eq!(s.hand(P1).len(), 4);
            assert_eq!(mods(&s), vec![-1, -1, -1, 0]);
        }

        #[test]
        fn r386_numbers_read_through_param() {
            crate::register_all();
            let mut s = setup(false);
            step_param(s.card_mut(INTROSPECTION), "heal", 1);
            step_param(s.card_mut(INTROSPECTION), "draw", 1);
            step_param(s.card_mut(INTROSPECTION), "discount", 1);
            step_param(s.card_mut(INTROSPECTION), "damage", 1);

            s.start_turn();

            // Heal 1 + 1, two draws and then the turn's, discount 2 on all but the last, damage 2.
            s.expect_health(P1, 22);
            assert_eq!(s.hand(P1).len(), 5);
            assert_eq!(mods(&s), vec![-2, -2, -2, -2, 0]);
            s.expect_health(P2, HERO_HEALTH - 2);
        }

        #[test]
        fn radiant_heals_2_discounts_2_hits_2_draws_2() {
            crate::register_all();
            let mut s = setup(true);

            s.start_turn();

            s.expect_health(P1, 22);
            // Genn's 4 damage, 2 of it healed.
            s.expect_stats(GENN, json!({ "health": 12 }));
            assert_eq!(s.hand(P1).len(), 5);
            assert_eq!(mods(&s), vec![-2, -2, -2, -2, 0]);
            s.expect_in_zone(FODDER, "graveyard");
            s.expect_health(P2, HERO_HEALTH - 2);
        }

        /// R767000: the script kept for the replays of v0.5.11o and before, which acted at the end of turn.
        #[test]
        fn r767000_a_game_on_v0_5_11o_acts_at_the_end_of_your_turn_and_not_the_start() {
            for radiant in [false, true] {
                crate::under_kept(INTROSPECTION, script_v0_5_11o(), || {
                    let mut s = setup(radiant);
                    s.start_turn();
                    s.expect_health(P1, 20);
                    assert_eq!(s.hand(P1).len(), 3, "only the turn's own draw");

                    let mut s = setup(radiant);
                    s.end_turn();
                    s.expect_health(P1, if radiant { 22 } else { 21 });
                    s.expect_health(P2, HERO_HEALTH - if radiant { 2 } else { 1 });
                });
            }
        }
    }
}
