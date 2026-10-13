//! C #49 Anti-Greed Machine (SPEC §8.6 row 49, BUILD M9 Classic row C 49). (3) Unit 9/9 → 18/18, Common.
//!   Base:    "Rush / Aura: Players can't draw more than 1 card each turn."
//!   Radiant: "Rush / Aura: Your opponent can't draw more than {limit|card|cards} each turn." (1)
//!   Engine:  "A draw limit (§2.4) of 1 on both players (Radiant: on the opponent only) while it is on
//!            the field, on every turn, the start-of-turn draw included, read against the per-turn draw
//!            count (§10.1), so draws made earlier in the turn count: a draw beyond it does not happen at
//!            all (no card moves, no fatigue, no cast on draw), and with several limits the lowest holds.
//!            Tunes: Radiant limit 1 ↓ (never below 1)."
//!
//! The aura is B5 E3's draw limit (`Script.drawLimit`), asked of every card acting on the field before
//! each draw, so it lifts the moment the Machine leaves; Rush is printed on both faces (§10.4 layer 1).
//! The base face's 1 is printed; the Radiant limit reads `param(…, "limit")` (R386, `min` 1).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-049";

/// "Players can't draw more than 1 card each turn": the base face's printed limit, on both players.
const PLAYERS_LIMIT: i32 = 1;

fn base_limit(_args: HookArgs<'_>) -> Vec<DrawLimit> {
    vec![DrawLimit {
        player: DrawLimitPlayer::Both,
        count: PLAYERS_LIMIT,
    }]
}

/// "Your opponent can't draw more than {limit} …": the declared, tunable number.
fn radiant_limit(args: HookArgs<'_>) -> Vec<DrawLimit> {
    vec![DrawLimit {
        player: DrawLimitPlayer::Enemy,
        count: param(&args, "limit"),
    }]
}

pub fn script() -> CardScripts {
    let base = Script {
        draw_limit: Some(read_hook(base_limit)),
        ..Script::default()
    };

    let radiant = Script {
        draw_limit: Some(read_hook(radiant_limit)),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// BUILD M9 Classic row C 49 (SPEC §8.6 row 49): draws beyond 1 a turn do not happen at all, no card
// moves, no fatigue, nothing cast on draw (§2.4); the lowest limit holds, shown with C #4 Palantir and
// two Machines; lifted when it leaves; radiant: the opponent only, its limit reads `param()` (R386).
// The aura is B5 E3's draw limit. Stockpile (core-005, "Draw 2. Heal your hero 2.") makes the draws.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MACHINE: &str = "classic-049";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const VANILLA: &str = "core-008"; // 4/4, no text
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const FIENDER: &str = "core-092"; // Stack Unit
    const CN_VIRUS: &str = "core-090-1"; // Cast on draw: take 1 damage
    const FILLER: &str = "core-010"; // (0) Spell, Combo 3 — a card to keep in hand
    const PALANTIR: &str = "classic-004"; // Aura: your opponent can't draw more than 1 card each turn.

    use crate::js;

    fn of_type(s: &Scenario, kind: &str, player: PlayerId) -> Vec<Value> {
        s.last_events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == kind && event["player"] == js(&player))
            .collect()
    }

    fn drawn(s: &Scenario, player: PlayerId) -> Vec<Value> {
        of_type(s, "drawn", player)
    }

    fn limited(s: &Scenario, player: PlayerId) -> Vec<Value> {
        of_type(s, "drawLimited", player)
    }

    mod c49_anti_greed_machine {
        use super::*;

        #[test]
        fn is_a_3_9_9_rush_unit_18_18_radiant_rush_its_radiant_limit_a_declared_number() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["cost"], 3);
            assert_eq!(
                json!([def["base"]["attack"], def["base"]["health"], def["radiant"]["attack"], def["radiant"]["health"]]),
                json!([9, 9, 18, 18]),
            );
            assert_eq!(def["base"]["keywords"], json!([{ "kind": "Rush" }]));
            assert_eq!(def["radiant"]["keywords"], json!([{ "kind": "Rush" }]));
            assert_eq!(def["params"], json!([{ "key": "limit", "base": 1, "radiant": 1, "better": "down", "step": 1, "min": 1 }]));
            let scripts = script();
            assert!(scripts.base.draw_limit.is_some());
            assert!(scripts.radiant.draw_limit.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_9_9_with_rush_on_the_field_it_may_attack_a_unit_the_turn_it_arrives() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [MACHINE, FILLER] }, "p2": { "field": [VANILLA], "hand": [FILLER] } }));
                s.play(MACHINE, json!({}));
                s.expect_stats(MACHINE, json!({ "attack": 9, "health": 9 }));
                s.attack(MACHINE, VANILLA).expect_in_zone(VANILLA, "graveyard");
            }

            #[test]
            fn s2_4_your_second_draw_in_a_turn_does_not_happen_at_all_no_card_moves_draw_limited_says_so() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, FILLER], "field": [MACHINE], "library": [VANILLA, VANILLA, VANILLA] },
                }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P1).len(), 1);
                assert_eq!(limited(&s, P1).len(), 1);
                assert_eq!(s.pile(P1, "library").len(), 2);
                assert_eq!(s.hand(P1).len(), 2);
            }

            #[test]
            fn s2_4_a_draw_made_earlier_that_turn_counts_after_one_draw_the_next_stockpile_draws_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
                    "p2": { "hand": [MACHINE, FILLER] },
                }));
                // p1 draws once on its own turn before any Machine is there.
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P1).len(), 2);
                s.end_turn();
                s.play(MACHINE, json!({}));
                s.end_turn();
                // p1's start-of-turn draw is its first this turn; the Stockpile's two are past the limit.
                assert_eq!(drawn(&s, P1).len(), 1);
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P1).len(), 0);
                assert_eq!(limited(&s, P1).len(), 2);
            }

            #[test]
            fn binds_every_player_on_their_own_turn_the_opponents_start_of_turn_draw_is_their_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [MACHINE] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                }));
                s.end_turn();
                assert_eq!(drawn(&s, P2).len(), 1);
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P2).len(), 0);
                assert_eq!(limited(&s, P2).len(), 2);
                assert_eq!(s.pile(P2, "library").len(), 2);
            }

            #[test]
            fn s2_4_a_stopped_draw_takes_no_fatigue_from_an_empty_deck() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, FILLER], "field": [MACHINE], "library": [] } }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(s.state().players.p1.fatigue_count, 1);
                assert_eq!(s.last_events().iter().filter(|event| matches!(event, GameEvent::Fatigue { .. })).count(), 1);
                assert_eq!(limited(&s, P1).len(), 1);
            }

            #[test]
            fn s2_4_r58_nothing_is_cast_on_a_stopped_draw_a_cast_on_draw_card_stays_in_the_deck() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, FILLER], "field": [MACHINE], "library": [VANILLA, CN_VIRUS] },
                }));
                s.play(STOCKPILE, json!({}));
                let library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.def_id).collect();
                assert_eq!(library, vec![CN_VIRUS]);
                assert_eq!(
                    s.last_events()
                        .iter()
                        .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == CN_VIRUS))
                        .count(),
                    0,
                );
            }

            #[test]
            fn b5_e3_lifted_when_it_leaves_the_field_destroyed_the_next_draw_happens() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, STOCKPILE, HIT_JOB, FILLER],
                        "field": [MACHINE],
                        "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                        "mana": 9,
                    },
                }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P1).len(), 1);
                let machine = s.card(MACHINE).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": machine }] }));
                s.expect_in_zone(MACHINE, "graveyard");
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P1).len(), 2);
            }

            #[test]
            fn s3_2_r13_dormant_under_a_stack_pile_it_is_not_on_the_field_and_limits_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, FILLER],
                        "field": [MACHINE, { "def": FIENDER, "stack": true }],
                        "library": [VANILLA, VANILLA],
                    },
                }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P1).len(), 2);
                assert_eq!(limited(&s, P1).len(), 0);
            }

            #[test]
            fn b5_e3_with_several_limits_the_lowest_holds_a_radiant_machine_tuned_to_2_beside_a_base_one_still_stops_the_2nd_draw() {
                crate::register_all();
                let mut both = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MACHINE, "radiant": true }, MACHINE] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                    "active": "p2",
                }));
                let Some(tuned) = both.unit(P1, 1) else {
                    panic!("no Radiant Machine");
                };
                step_param(both.card_mut(&tuned.id), "limit", 1);
                both.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&both, P2).len(), 1);
                assert_eq!(limited(&both, P2).len(), 1);
                // Without the base one, the tuned Radiant limit of 2 lets both draws through.
                let mut alone = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MACHINE, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                    "active": "p2",
                }));
                step_param(alone.card_mut(MACHINE), "limit", 1);
                alone.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&alone, P2).len(), 2);
            }

            #[test]
            fn b5_e3_with_c_4_palantirs_limit_the_lowest_holds_whichever_card_sets_it() {
                crate::register_all();
                // Palantir (p1's) limits p2 to 1; a Radiant Machine tuned to 2 beside it still lets only 1 through.
                let mut palantir_lower = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MACHINE, "radiant": true }], "backrow": [PALANTIR] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                    "active": "p2",
                }));
                step_param(palantir_lower.card_mut(MACHINE), "limit", 1);
                palantir_lower.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&palantir_lower, P2).len(), 1);
                assert_eq!(limited(&palantir_lower, P2).len(), 1);
                // A Palantir tuned to 2 beside a base Machine: the Machine's 1 holds.
                let mut machine_lower = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [MACHINE], "backrow": [PALANTIR] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                    "active": "p2",
                }));
                step_param(machine_lower.card_mut(PALANTIR), "drawLimit", 1);
                machine_lower.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&machine_lower, P2).len(), 1);
                assert_eq!(limited(&machine_lower, P2).len(), 1);
            }

            #[test]
            fn r97_the_stopped_draw_names_no_card_in_either_view() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, FILLER], "field": [MACHINE], "library": [VANILLA, VANILLA] } }));
                s.play(STOCKPILE, json!({}));
                for viewer in [P1, P2] {
                    let events: Vec<Value> = js(&s.view(viewer))["events"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|event| event["type"] == "drawLimited")
                        .collect();
                    assert_eq!(json!(events), json!([{ "type": "drawLimited", "player": "p1" }]));
                }
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_an_18_18_with_rush() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": MACHINE, "radiant": true }, FILLER] } }));
                s.play(MACHINE, json!({})).expect_stats(MACHINE, json!({ "attack": 18, "health": 18 }));
                let kinds: Vec<Value> = js(&s.stats(MACHINE).keywords)
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|keyword| keyword["kind"].clone())
                    .collect();
                assert_eq!(json!(kinds), json!(["Rush"]));
            }

            #[test]
            fn your_draws_are_free_its_controllers_stockpile_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, FILLER], "field": [{ "def": MACHINE, "radiant": true }], "library": [VANILLA, VANILLA] },
                }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P1).len(), 2);
                assert_eq!(limited(&s, P1).len(), 0);
            }

            #[test]
            fn binds_the_opponent_their_start_of_turn_draw_is_their_only_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MACHINE, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                }));
                s.end_turn();
                assert_eq!(drawn(&s, P2).len(), 1);
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P2).len(), 0);
                assert_eq!(limited(&s, P2).len(), 2);
            }

            #[test]
            fn r386_its_limit_is_the_declared_number_a_degrades_step_less_is_better_lets_the_opponent_draw_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MACHINE, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
                }));
                step_param(s.card_mut(MACHINE), "limit", 1);
                s.end_turn();
                assert_eq!(drawn(&s, P2).len(), 1);
                s.play(STOCKPILE, json!({}));
                assert_eq!(drawn(&s, P2).len(), 1);
                assert_eq!(limited(&s, P2).len(), 1);
            }

            #[test]
            fn r386_its_limit_never_goes_below_1_a_step_down_from_1_holds_at_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MACHINE, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                }));
                step_param(s.card_mut(MACHINE), "limit", -1);
                assert_eq!(js(&s.view(P1))["you"]["units"][0]["params"], json!({ "limit": 1 }));
                s.end_turn();
                assert_eq!(drawn(&s, P2).len(), 1);
            }

            #[test]
            fn the_opponents_legal_plays_are_not_changed_by_it_a_limited_draw_is_not_a_refusal() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MACHINE, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA] },
                    "active": "p2",
                }));
                let stockpile = s.card(STOCKPILE).id.clone();
                assert!(
                    legal_actions(s.state(), P2)
                        .iter()
                        .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == stockpile))
                );
            }
        }
    }
}
