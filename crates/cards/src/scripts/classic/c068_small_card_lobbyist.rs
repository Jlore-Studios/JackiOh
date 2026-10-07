//! C #68 Small Card Lobbyist (SPEC §8.6 row 68, BUILD M9 Classic row C 68). (4) Unit 11/13 → 22/26, Common.
//!   Base:    "Aura: ({threshold})+ Cost cards cost ({surcharge}) more." (3, 1)
//!   Radiant: "Aura: Your opponent can't play ({threshold})+ Cost cards." (3)
//!   Engine:  "Cost (§6.3, R65) on both players' cards where a play takes them from (a hand, or a
//!            graveyard a permission lets its owner play from, §6.3 Play), as a price for a play; '(3)+'
//!            is read where R363 reads Professor Curvature's '(4)+', on the cost before this aura adds
//!            its (1). Radiant: `legalActions` offers the opponent no play of a card that costs (3) or
//!            more at that moment (R65); casts (R70) are not plays from hand and are unaffected. Tunes:
//!            surcharge 1 ↑; threshold 3 ↓."
//!
//! The aura is B5 E15's price rule (`Script.costAura`), laid while the Lobbyist acts on the field
//! (the top of its pile) and read by R65's `effectiveCost` on every card a play would take:
//!   - Base: a threshold rung on every player's cards, `minCost` the declared threshold and `amount` the
//!     declared surcharge; the ladder tests the threshold against the price the flat rungs left (R363),
//!     before this rule adds its own, so a (2) Cost card is never lifted into range. An X-cost card
//!     costs exactly its X and takes no rule (R65); a cast pays nothing (R70).
//!   - Radiant: a ban (`ban: true`) on the opponent's cards whose finished price is the threshold or
//!     more (an X card at its chosen X included): `legalActions` never offers such a play and §10.5
//!     step 1 refuses it, read last (R65); its controller's own plays and every cast are untouched.
//!
//! Both numbers are declared and read through `param` (R386); "threshold ↓" moves toward harder for a
//! Degrade — up, so fewer cards are caught.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-068";

pub fn script() -> CardScripts {
    let base = Script {
        cost_aura: Some(read_hook(|args: CostAuraArgs<'_>| -> Vec<CostAura> {
            vec![CostAura {
                rule: CostRule {
                    min_cost: Some(param(&args, "threshold")),
                    amount: Some(param(&args, "surcharge")),
                    ..CostRule::default()
                },
                whose: CostAuraWhose::All,
                ban: None,
            }]
        })),
        ..Script::default()
    };

    let radiant = Script {
        cost_aura: Some(read_hook(|args: CostAuraArgs<'_>| -> Vec<CostAura> {
            vec![CostAura {
                rule: CostRule {
                    min_cost: Some(param(&args, "threshold")),
                    ..CostRule::default()
                },
                whose: CostAuraWhose::Opponents,
                ban: Some(true),
            }]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #68 Small Card Lobbyist — SPEC §8.6 row 68, BUILD M9 Classic row C 68: "Aura: every (3)+ Cost card
// either player could play (in a hand, or in a graveyard a permission lets its owner play from, R65)
// costs (1) more, the threshold read before this aura adds its 1, as R363 reads Curvature's, so a (2)
// Cost card is not lifted into range; X-cost cards are untouched (R65); casts pay nothing (R70); gone
// when it leaves; a hidden hand card's `costChanged` shows −1 to the other player (R177); radiant
// 22/26: the opponent can't play a (3)+ Cost card at all, checked last (R65) and absent from their
// `legalActions`, an X card for X of 3 or more included; casts still happen; your plays are free of
// it; its tuned numbers (surcharge, threshold) read through `param()` (R386)".
//
// A graveyard play is shown under C #28 Second Wind's permission (its Radiant face, so a played card
// lands in the graveyard as usual), sent to `reduce` since the harness's `play` takes a hand card. A
// cast of a (3)+ Cost card is shown with a Cast-on-draw card a `costMod` has priced at (3).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LOBBYIST: &str = "classic-068";
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const FIG: &str = "core-047"; // (3) Spell: Heal a target 20.
    const NETHER: &str = "core-088"; // (4) Spell: Destroy all permanents.
    const ARMOR: &str = "core-073"; // (2) Field Spell
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const ECLIPSE: &str = "core-035"; // (1) Spell: Deal 3 damage; your next Spell this turn costs (1) less.
    const DIVIDEND: &str = "core-024"; // (X) Spell
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw
    const REMINISCE: &str = "core-072"; // (1) Spell: Discover a card from your GY. It costs (1) less. Exile this.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const FILLER: &str = "core-010"; // (0) Spell
    const SECOND_WIND: &str = "classic-028"; // Radiant Aura: you may play cards from your graveyard that cost (1) or more.

    use crate::js;

    use crate::matches_object;

    fn hand_cost(s: &Scenario, player: PlayerId, def_id: &str) -> Option<i64> {
        let view = js(&s.view(player));
        let hand = &view["you"]["hand"];
        hand.as_array()
            .and_then(|cards| cards.iter().find(|card| card["defId"] == def_id))
            .and_then(|card| card["cost"].as_i64())
    }

    fn offered(s: &Scenario, player: PlayerId, def_id: &str) -> bool {
        let id = s.card(def_id).id.clone();
        legal_actions(s.state(), player)
            .iter()
            .map(js)
            .any(|action| action["type"] == "play" && action["instanceId"] == id.as_str())
    }

    /// A play action sent straight to `reduce` (the harness's `play` takes a hand card).
    fn play_action(player: &str, instance_id: &str, nonce: &str) -> Action {
        json_as(json!({ "type": "play", "playerId": player, "instanceId": instance_id, "nonce": nonce }))
    }

    mod c_68_small_card_lobbyist {
        use super::*;

        #[test]
        fn is_a_4_11_13_unit_22_26_radiant_with_no_keywords_its_two_numbers_declared() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["cost"], 4);
            assert_eq!(
                json!([def["base"]["attack"], def["base"]["health"], def["radiant"]["attack"], def["radiant"]["health"]]),
                json!([11, 13, 22, 26]),
            );
            assert_eq!(
                def["params"],
                json!([
                    { "key": "surcharge", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 },
                    { "key": "threshold", "base": 3, "radiant": 3, "better": "down", "step": 1, "min": 1 },
                ]),
            );
            let scripts = script();
            assert!(scripts.base.cost_aura.is_some());
            assert!(scripts.radiant.cost_aura.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn is_an_11_13_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LOBBYIST, FILLER] } }));
                s.play(LOBBYIST, json!({})).expect_stats(LOBBYIST, json!({ "attack": 11, "health": 13 }));
            }

            #[test]
            fn every_3_cost_card_in_either_players_hand_costs_1_more_spells_and_units_alike() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, NETHER], "field": [LOBBYIST] },
                    "p2": { "hand": [FIG, MENACE] },
                }));
                assert_eq!(hand_cost(&s, P1, HIT_JOB), Some(4));
                assert_eq!(hand_cost(&s, P1, NETHER), Some(5));
                assert_eq!(hand_cost(&s, P2, FIG), Some(4));
                assert_eq!(hand_cost(&s, P2, MENACE), Some(4));
            }

            #[test]
            fn a_play_pays_the_surcharge() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [LOBBYIST] },
                    "p2": { "field": [VANILLA] },
                }));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] })).expect_mana(P1, 0);
            }

            #[test]
            fn r65_legal_actions_agrees_with_the_refusal_at_3_mana_a_3_cost_card_now_4_is_not_offered_and_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [LOBBYIST], "mana": 3 },
                    "p2": { "field": [VANILLA] },
                }));
                assert!(!offered(&s, P1, HIT_JOB));
                let vanilla = s.card(VANILLA).id.clone();
                s.expect_refused_with(
                    |s| s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] })),
                    "costs 4",
                );
            }

            #[test]
            fn r65_a_play_from_a_graveyard_c_28_second_wind_permits_pays_the_surcharge_too() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "field": [LOBBYIST],
                        "backrow": [{ "def": SECOND_WIND, "radiant": true }],
                        "graveyard": [MENACE],
                    },
                }));
                let menace = s.card(MENACE).clone();
                assert!(offered(&s, P1, MENACE));
                let result = reduce(s.state(), &play_action("p1", &menace.id, "c68-graveyard"));
                assert!(result.error.is_none());
                let played = result
                    .events
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "cardPlayed")
                    .expect("a cardPlayed event");
                assert!(matches_object(
                    &played,
                    &json!({ "instanceId": menace.id, "from": "graveyard", "costPaid": 4 }),
                ));
                assert_eq!(result.state.players.p1.mana.current, 0);
            }

            #[test]
            fn r363_a_2_cost_card_is_not_lifted_into_range_the_threshold_reads_the_cost_before_its_own_1() {
                crate::register_all();
                let s = scenario(json!({ "p1": { "hand": [ARMOR, VANILLA], "field": [LOBBYIST] } }));
                assert_eq!(hand_cost(&s, P1, ARMOR), Some(2));
                assert_eq!(hand_cost(&s, P1, VANILLA), Some(1));
            }

            #[test]
            fn r363_r65_a_3_cost_card_a_discount_has_brought_to_2_is_out_of_range_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ECLIPSE, HIT_JOB, FILLER], "field": [LOBBYIST], "mana": 9 },
                }));
                s.play(ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
                assert_eq!(hand_cost(&s, P1, HIT_JOB), Some(2));
            }

            #[test]
            fn r65_an_x_cost_card_is_untouched_x_of_3_costs_3() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [DIVIDEND, FILLER], "field": [LOBBYIST] } }));
                s.play(DIVIDEND, json!({ "x": 3, "modes": ["damage"], "targets": [{ "pick": "hero", "player": "p2" }] }))
                    .expect_mana(P1, 1);
            }

            #[test]
            fn r70_a_cast_pays_nothing_a_cast_on_draw_card_priced_at_3_is_cast_for_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [LOBBYIST] },
                    "p2": { "hand": [FILLER], "library": [{ "def": CN_VIRUS, "costMod": 2 }, VANILLA] },
                }));
                s.end_turn();
                let cast = s
                    .last_events()
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "cardPlayed" && event["defId"] == CN_VIRUS)
                    .expect("the Cast on draw card's cardPlayed");
                assert!(matches_object(&cast, &json!({ "player": "p2", "costPaid": 0 })));
            }

            #[test]
            fn gone_when_it_leaves_destroyed_a_3_cost_card_costs_3_again() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, FIG], "field": [LOBBYIST], "mana": 9 } }));
                let lobbyist = s.card(LOBBYIST).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": lobbyist }] }));
                s.expect_in_zone(LOBBYIST, "graveyard");
                assert_eq!(hand_cost(&s, P1, FIG), Some(3));
            }

            #[test]
            fn r177_a_hidden_hand_cards_cost_changed_shows_1_and_the_sentinel_to_the_other_player() {
                crate::register_all();
                // Reminisce returns a (4) Spell that costs (1) less: (3), lifted to (4) by the aura.
                let mut s = scenario(json!({
                    "p1": { "hand": [REMINISCE, FILLER], "field": [LOBBYIST], "graveyard": [NETHER] },
                }));
                s.play(REMINISCE, json!({})).answer(json!(NETHER));
                let nether = s.card(NETHER).clone();
                s.expect_in_zone(&nether, "hand");
                assert_eq!(hand_cost(&s, P1, NETHER), Some(4));
                let own: Vec<Value> = js(&s.view(P1).events)
                    .as_array()
                    .expect("the view's events")
                    .iter()
                    .filter(|event| event["type"] == "costChanged" && event["instanceId"] == nether.id.as_str())
                    .cloned()
                    .collect();
                assert_eq!(own, vec![json!({ "type": "costChanged", "instanceId": nether.id, "cost": 4 })]);
                let theirs: Vec<Value> = js(&s.view(P2).events)
                    .as_array()
                    .expect("the view's events")
                    .iter()
                    .filter(|event| event["type"] == "costChanged")
                    .cloned()
                    .collect();
                assert_eq!(theirs, vec![json!({ "type": "costChanged", "instanceId": "hidden", "cost": -1 })]);
            }

            #[test]
            fn r386_its_surcharge_is_declared_an_upgrades_step_makes_3_cost_cards_cost_2_more() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB], "field": [LOBBYIST] } }));
                step_param(s.card_mut(LOBBYIST), "surcharge", 1);
                assert_eq!(hand_cost(&s, P1, HIT_JOB), Some(5));
            }

            #[test]
            fn r386_its_threshold_is_declared_and_a_degrade_moves_it_toward_harder_4_so_a_3_card_is_spared() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, NETHER], "field": [LOBBYIST] } }));
                step_param(s.card_mut(LOBBYIST), "threshold", 1);
                assert_eq!(hand_cost(&s, P1, HIT_JOB), Some(3));
                assert_eq!(hand_cost(&s, P1, NETHER), Some(5));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_22_26_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": LOBBYIST, "radiant": true }, FILLER] } }));
                s.play(LOBBYIST, json!({})).expect_stats(LOBBYIST, json!({ "attack": 22, "health": 26 }));
            }

            #[test]
            fn r65_the_opponent_cant_play_a_3_cost_card_absent_from_their_legal_actions_and_refused() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": LOBBYIST, "radiant": true }] },
                    "p2": { "hand": [HIT_JOB, FIG, ARMOR, FILLER], "field": [VANILLA] },
                    "active": "p2",
                }));
                assert!(!offered(&s, P2, HIT_JOB));
                assert!(!offered(&s, P2, FIG));
                assert!(offered(&s, P2, ARMOR));
                let vanilla = s.card(VANILLA).id.clone();
                s.expect_refused_with(
                    |s| s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] })),
                    "can't play (3)+ Cost cards",
                );
                // The price itself is not raised: a ban prices nothing.
                assert_eq!(hand_cost(&s, P2, HIT_JOB), Some(3));
            }

            #[test]
            fn r65_an_x_card_is_banned_for_an_x_of_3_or_more_only_x_of_1_and_2_are_offered() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": LOBBYIST, "radiant": true }] },
                    "p2": { "hand": [DIVIDEND, FILLER] },
                    "active": "p2",
                }));
                let id = s.card(DIVIDEND).id.clone();
                // TS `new Set(…)`, then `[...xs].sort()`: the distinct X values offered, in order.
                let mut xs: Vec<Option<i64>> = legal_actions(s.state(), P2)
                    .iter()
                    .map(js)
                    .filter(|action| action["type"] == "play" && action["instanceId"] == id.as_str())
                    .map(|action| action["x"].as_i64())
                    .collect::<IndexSet<Option<i64>>>()
                    .into_iter()
                    .collect();
                xs.sort();
                assert_eq!(xs, vec![Some(1), Some(2)]);
                s.expect_refused_with(|s| s.play(DIVIDEND, json!({ "x": 3, "modes": ["mana"] })), "can't play");
            }

            #[test]
            fn r65_checked_last_on_the_finished_price_a_4_cost_card_discounted_to_2_may_be_played() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": LOBBYIST, "radiant": true }] },
                    "p2": { "hand": [{ "def": NETHER, "costMod": -2 }, FILLER] },
                    "active": "p2",
                }));
                assert!(offered(&s, P2, NETHER));
                s.play(NETHER, json!({}));
                s.expect_in_zone(LOBBYIST, "graveyard");
            }

            #[test]
            fn r65_the_ban_reaches_a_graveyard_c_28_second_wind_lets_the_opponent_play_from_a_3_there_is_not_offered() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": LOBBYIST, "radiant": true }] },
                    "p2": {
                        "hand": [FILLER],
                        "backrow": [{ "def": SECOND_WIND, "radiant": true }],
                        "graveyard": [MENACE, VANILLA],
                    },
                    "active": "p2",
                }));
                assert!(offered(&s, P2, VANILLA));
                assert!(!offered(&s, P2, MENACE));
                let menace = s.card(MENACE).id.clone();
                let refused = reduce(s.state(), &play_action("p2", &menace, "c68-banned"));
                assert!(refused.error.as_deref().is_some_and(|error| error.contains("can't play (3)+ Cost cards")));
            }

            #[test]
            fn your_plays_are_free_of_it_its_controller_plays_a_3_cost_card_at_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [{ "def": LOBBYIST, "radiant": true }] },
                    "p2": { "field": [VANILLA] },
                }));
                assert!(offered(&s, P1, HIT_JOB));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] })).expect_mana(P1, 1);
            }

            #[test]
            fn r70_casts_still_happen_the_opponents_cast_on_draw_card_priced_at_3_is_cast() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": LOBBYIST, "radiant": true }] },
                    "p2": { "hand": [FILLER], "library": [{ "def": CN_VIRUS, "costMod": 2 }, VANILLA] },
                }));
                s.end_turn();
                let cast = s
                    .last_events()
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "cardPlayed" && event["defId"] == CN_VIRUS)
                    .expect("the Cast on draw card's cardPlayed");
                assert!(matches_object(&cast, &json!({ "player": "p2" })));
            }

            #[test]
            fn r386_its_declared_threshold_moves_toward_harder_on_a_degrade_4_so_the_opponent_may_play_a_3() {
                crate::register_all();
                let mut sp = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": LOBBYIST, "radiant": true }, VANILLA] },
                    "p2": { "hand": [HIT_JOB, NETHER, FILLER] },
                    "active": "p2",
                }));
                assert!(!offered(&sp, P2, HIT_JOB));
                step_param(sp.card_mut(LOBBYIST), "threshold", 1);
                assert!(offered(&sp, P2, HIT_JOB));
                assert!(!offered(&sp, P2, NETHER));
            }
        }
    }
}
