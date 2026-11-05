//! C #62 Living Bomb (SPEC §8.6 row 62). (2) Field Spell, Rare, Plague.
//!   Base:    "Cry: Place a Plague Counter. / At the start of each player's turn: Destroy every permanent
//!            that player controls with a Plague Counter on it."
//!   Radiant: "Activate: Place a Plague Counter. / At the start of your opponent's turn: Destroy every
//!            permanent they control with a Plague Counter on it."
//!
//! Since issue #3139 it costs (2) and places Plague Counters itself: the base face with a Cry, the Radiant
//! face, in its place, with an Activate (once a turn, R384; two separate gains, no Cry on the Radiant face).
//! Both are `place_plague_tokens` of one (§6.3, R689): a single prompt names the permanent, of either
//! side and face-down ones by id alone (R177), Living Bomb itself among them. `script_v0_5_11o` keeps the
//! card with neither, cost (1), for replays (R767000).
//!
//! R400: at the start of a player's turn, every permanent that player controls with a Plague Counter
//! is destroyed (face-down cards and Living Bomb included; §6.3). All die together at the state
//! check (§4.5, R59); Indestructible stays (§6.1, R46), and R78 clears tokens on leave.
//!
//! Hooks queue at R62's start-of-turn trigger point in R68's order: `startOfTurn` on own turn (base),
//! `startOfOpponentTurn` on opponent's turn, queued after the active player's own hooks.

use jackioh_engine::effects::{destroy, place_plague_tokens};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-062";

/// R400: one destroy for each permanent `player` controls with a Plague Counter on it, in R68's order.
fn destroy_plagued(ctx: &EffectContext<'_>, player: PlayerId) -> Vec<Effect> {
    permanents_on_field(ctx.state, player)
        .iter()
        .filter(|card| card.controller == player && plague_on(card) > 0)
        .map(|card| destroy(json_as(json!({ "target": { "of": "instance", "instanceId": card.id } }))))
        .collect()
}

fn at_your_turn() -> Hook {
    hook(|ctx| {
        let player = ctx.controller;
        destroy_plagued(ctx, player)
    })
}

fn at_opponents_turn() -> Hook {
    hook(|ctx| {
        let player = opponent_of(ctx.controller);
        destroy_plagued(ctx, player)
    })
}

/// "Place a Plague Counter" (R689): one placement on the permanent a prompt names.
fn place_a_counter() -> Effect {
    place_plague_tokens(json_as(json!({ "count": 1 })))
}

/// "Activate: Place a Plague Counter" (R384): once a turn, no price.
fn place_activation() -> ActivationDecl {
    ActivationDecl {
        id: "plague".to_string(),
        label: "Place a Plague Counter".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|_ctx| vec![place_a_counter()]),
    }
}

pub fn script() -> CardScripts {
    let at_opponents_turn = at_opponents_turn();

    let base = Script {
        cry: Some(hook(|_ctx| vec![place_a_counter()])),
        start_of_turn: Some(at_your_turn()),
        start_of_opponent_turn: Some(at_opponents_turn.clone()),
        ..Script::default()
    };

    let radiant = Script {
        activations: vec![place_activation()],
        start_of_opponent_turn: Some(at_opponents_turn),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

/// R767000: the script v0.5.11p and the versions before it ran, with no Cry and no Activate; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    let at_opponents_turn = at_opponents_turn();

    let base = Script {
        start_of_turn: Some(at_your_turn()),
        start_of_opponent_turn: Some(at_opponents_turn.clone()),
        ..Script::default()
    };

    let radiant = Script {
        start_of_opponent_turn: Some(at_opponents_turn),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #62 Living Bomb (SPEC §8.6 row 62): at the start of each player's turn, in R68's order,
// destroy every permanent that player controls with a Plague Counter (R400; Indestructible stays,
// R46; tokens cleared on leave, R78). Radiant: only at the start of opponent's turn. Since issue #3139 the
// base face places a Plague Counter as its Cry and the Radiant face as an Activate, once a turn (R689, R384).
#[cfg(test)]
mod tests {
    use super::script_v0_5_11o;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use std::collections::BTreeSet;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOMB: &str = "classic-062";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const ROCK: &str = "core-066"; // (4) Unit 10/10 Indestructible, Tribute 1.
    const FAUCI: &str = "core-091"; // (2) Unit 1/6 Rush; Start of turn: +1 mana per Plague Counter.
    const FIENDER: &str = "core-092"; // (2) Unit 5/7 Stack.
    const REBORN: &str = "core-003"; // (1) Unit 1/1 Taunt, Divine Shield, Reborn.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const FILLER: &str = "core-005"; // (1) Spell, a card to keep a turn from auto-ending (§2.5).
    const X: &str = "core-020"; // library filler.

    use crate::js;

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    fn destroyed_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "destroyed")
            .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
            .collect()
    }

    fn board(radiant_face: bool, bomb_tokens: i32) -> Scenario {
        let bomb_counters = if bomb_tokens > 0 { json!({ "plague": bomb_tokens }) } else { json!({}) };
        scenario(json!({
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "counters": { "plague": 1 } }, { "def": MENACE, "lane": 2 }],
                "backrow": [
                    { "def": BOMB, "radiant": radiant_face, "counters": bomb_counters },
                    { "def": PAWN, "faceUp": false, "lane": 2, "counters": { "plague": 2 } },
                ],
                "library": lib(4),
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "counters": { "plague": 2 } }, { "def": MENACE, "lane": 2 }],
                "backrow": [
                    { "def": MANA_WELL, "counters": { "plague": 1 } },
                    { "def": PAWN, "faceUp": false, "lane": 2, "counters": { "plague": 1 } },
                    { "def": PAWN, "faceUp": false, "lane": 3 },
                ],
                "library": lib(4),
            },
        }))
    }

    fn set_of(ids: Vec<String>) -> BTreeSet<String> {
        ids.into_iter().collect()
    }

    /// The instance ids the open placement prompt offers (R689).
    fn option_ids(s: &Scenario) -> BTreeSet<String> {
        must(s.state().pending.as_ref(), "an open prompt")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn placements(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterChanged { placed: Some(placed), .. } => Some(*placed),
                _ => None,
            })
            .collect()
    }

    /// p1 holds a Living Bomb (`radiant`'s face) with the mana to play it (2), and `board` is p2's side.
    fn in_hand(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": BOMB, "radiant": radiant }, FILLER], "field": [VANILLA], "library": lib(4) },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": MENACE, "lane": 1 }],
                "backrow": [{ "def": MANA_WELL, "lane": 1 }, { "def": PAWN, "faceUp": false, "lane": 2 }],
                "library": lib(4),
            },
        }))
    }

    mod c_62_living_bomb {
        use super::*;

        #[test]
        fn declares_no_tuned_numbers() {
            crate::register_all();
            assert!(crate::card_def(BOMB).params.is_none());
        }

        mod base {
            use super::*;

            /// Issue #3139: "Cry: Place a Plague Counter." One prompt names the permanent, of either side,
            /// face-down ones by id alone, Living Bomb itself among them (R689, R177).
            #[test]
            fn r689_its_cry_places_one_counter_on_the_permanent_a_single_prompt_names() {
                let mut s = in_hand(false);
                let mine = s.card(VANILLA).clone();
                let menace = s.card(MENACE).clone();
                let well = s.card(MANA_WELL).clone();
                let pawn = s.card(PAWN).clone();

                s.play(BOMB, json!({ "zone": 1 }));

                let bomb = s.card(BOMB).clone();
                assert_eq!(must(s.state().pending.as_ref(), "the placement prompt").player_id, P1);
                assert_eq!(
                    option_ids(&s),
                    set_of(vec![mine.id.clone(), menace.id.clone(), well.id.clone(), pawn.id.clone(), bomb.id.clone()])
                );
                s.answer(json!(menace.id));

                assert!(s.state().pending.is_none());
                assert_eq!(placements(&s), vec![1]);
                assert_eq!(s.card(&menace).counters.plague, Some(1));
                for other in [&mine, &well, &pawn, &bomb] {
                    assert_eq!(s.card(other).counters.plague.unwrap_or(0), 0);
                }
                s.expect_mana(P1, 2);
            }

            /// The counter the Cry placed is the Bomb's own weapon: the enemy Unit it names is destroyed at the
            /// start of the enemy's turn (R400).
            #[test]
            fn r400_the_unit_its_cry_marks_is_destroyed_at_the_start_of_that_players_turn() {
                let mut s = in_hand(false);
                let menace = s.card(MENACE).clone();
                s.play(BOMB, json!({ "zone": 1 }));
                s.answer(json!(menace.id));

                s.end_turn();

                assert_eq!(s.state().active, P2);
                s.expect_in_zone(&menace, "graveyard");
            }

            #[test]
            fn r689_it_costs_2() {
                crate::register_all();
                assert_eq!(serde_json::to_value(crate::card_def(BOMB).cost).unwrap(), json!(2));
                let mut s = in_hand(false);
                s.state_mut().players.p1.mana.current = 1;
                s.expect_refused(|s| s.play(BOMB, json!({ "zone": 1 })));
            }

            #[test]
            fn r689_it_has_no_activate() {
                crate::register_all();
                assert!(super::super::super::script().base.activations.is_empty());
                let mut s = scenario(json!({ "p1": { "backrow": [BOMB], "hand": [FILLER], "field": [VANILLA] }, "p2": { "hand": [FILLER] } }));
                s.expect_refused(|s| s.activate(BOMB, json!({})));
            }

            #[test]
            fn r400_at_the_start_of_the_opponents_turn_every_permanent_they_control_with_a_token_is_destroyed_face_down_ones_included() {
                crate::register_all();
                let mut s = board(false, 0);
                let (Some(their_vanilla), Some(their_well), Some(their_trap), Some(their_clean_trap)) =
                    (s.unit(P2, 1), s.backrow(P2, 1), s.backrow(P2, 2), s.backrow(P2, 3))
                else {
                    panic!("board");
                };

                s.end_turn();

                assert_eq!(s.state().active, P2);
                assert_eq!(
                    set_of(destroyed_ids(&s)),
                    set_of(vec![their_vanilla.id.clone(), their_well.id.clone(), their_trap.id.clone()]),
                );
                s.expect_in_zone(&their_vanilla, "graveyard");
                s.expect_in_zone(&their_well, "graveyard");
                s.expect_in_zone(&their_trap, "graveyard");
                assert_eq!(s.unit(P2, 2).map(|unit| unit.def_id), Some(MENACE.to_string()));
                assert_eq!(s.backrow(P2, 3).map(|card| card.id), Some(their_clean_trap.id.clone()));
            }

            #[test]
            fn r400_the_other_players_permanents_are_untouched_p1s_plagued_ones_survive_p2s_turn_start() {
                crate::register_all();
                let mut s = board(false, 1);

                s.end_turn();

                assert_eq!(s.unit(P1, 1).and_then(|unit| unit.counters.plague), Some(1));
                assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(BOMB.to_string()));
                assert_eq!(s.backrow(P1, 2).map(|card| card.def_id), Some(PAWN.to_string()));
            }

            #[test]
            fn r400_at_the_start_of_your_own_turn_your_plagued_permanents_are_destroyed_face_down_ones_and_living_bomb_itself_included() {
                crate::register_all();
                let mut s = board(false, 1);
                let (Some(my_vanilla), Some(bomb), Some(my_trap)) = (s.unit(P1, 1), s.backrow(P1, 1), s.backrow(P1, 2))
                else {
                    panic!("board");
                };
                s.end_turn(); // p2's turn: their plagued permanents go

                let before = destroyed_ids(&s).len();
                s.end_turn(); // p1's turn

                assert_eq!(s.state().active, P1);
                assert_eq!(
                    set_of(destroyed_ids(&s).split_off(before)),
                    set_of(vec![my_vanilla.id.clone(), bomb.id.clone(), my_trap.id.clone()]),
                );
                s.expect_in_zone(&bomb, "graveyard");
                assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(MENACE.to_string()));
                assert_eq!(s.unit(P2, 2).map(|unit| unit.def_id), Some(MENACE.to_string()));
            }

            #[test]
            fn s4_5_they_die_together_at_one_state_check_after_the_trigger() {
                crate::register_all();
                let mut s = board(false, 0);

                s.end_turn();

                let kinds: Vec<Value> = s
                    .last_events()
                    .iter()
                    .map(js)
                    .filter(|event| event["type"] == "destroyed")
                    .map(|event| event["type"].clone())
                    .collect();
                assert_eq!(kinds.len(), 3);
            }

            #[test]
            fn r46_an_indestructible_permanent_with_a_token_survives_knocked_into_attack_position() {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [BOMB], "library": lib(2) },
                    "p2": { "hand": [FILLER], "field": [{ "def": ROCK, "position": "DEF", "counters": { "plague": 2 } }], "library": lib(2) },
                }));
                let rock = s.card(ROCK).clone();

                s.end_turn();

                s.expect_in_zone(&rock, "field");
                assert_eq!(js(&s.stats(&rock).position), "ATK");
                assert_eq!(s.card(&rock).counters.plague, Some(2));
            }

            #[test]
            fn r78_a_cards_tokens_are_gone_once_it_leaves_bounced_and_played_again_it_survives_your_next_turn_start() {
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FLOOD, FILLER],
                        "field": [{ "def": VANILLA, "counters": { "plague": 3 } }],
                        "backrow": [BOMB],
                        "library": lib(4),
                        "mana": 10,
                    },
                    "p2": { "hand": [FILLER], "library": lib(4) },
                }));
                let vanilla = s.card(VANILLA).clone();
                s.play(FLOOD, json!({}));
                assert!(s.card(&vanilla).counters.plague.is_none());
                s.play(&vanilla, json!({}));

                s.end_turn();
                s.end_turn();

                assert_eq!(s.state().active, P1);
                s.expect_in_zone(&vanilla, "field");
                assert_eq!(destroyed_ids(&s), Vec::<String>::new());
            }

            #[test]
            fn r68_on_your_own_turn_it_is_a_start_of_turn_trigger_in_queue_order_a_plagued_fed_fauci_in_lane_1_gains_its_mana_first_then_dies() {
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [{ "def": FAUCI, "counters": { "plague": 2 } }], "backrow": [BOMB], "library": lib(2) },
                    "p2": { "hand": [FILLER], "library": lib(2) },
                }));
                let fauci = s.card(FAUCI).clone();

                s.end_turn();

                assert_eq!(s.state().active, P1);
                s.expect_mana(P1, 4 + 2);
                s.expect_in_zone(&fauci, "graveyard");
            }

            #[test]
            fn r68_on_the_opponents_turn_it_comes_after_their_own_start_of_turn_triggers_their_plagued_fed_fauci_gains_its_mana_then_dies() {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [BOMB], "library": lib(2) },
                    "p2": { "hand": [FILLER], "field": [{ "def": FAUCI, "counters": { "plague": 2 } }], "library": lib(2) },
                }));
                let fauci = s.card(FAUCI).clone();

                s.end_turn();

                assert_eq!(s.state().active, P2);
                s.expect_mana(P2, 4 + 2);
                s.expect_in_zone(&fauci, "graveyard");
            }

            #[test]
            fn s3_2_r13_a_card_dormant_under_a_stack_pile_is_not_on_the_field_the_plagued_top_goes_the_plagued_card_beneath_resumes() {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [BOMB], "library": lib(2) },
                    "p2": {
                        "hand": [FILLER],
                        "field": [{ "def": VANILLA, "counters": { "plague": 1 } }, { "def": FIENDER, "stack": true, "counters": { "plague": 1 } }],
                        "library": lib(2),
                    },
                }));
                let top = s.card(FIENDER).clone();
                let beneath = s.card(VANILLA).clone();

                s.end_turn();

                assert_eq!(destroyed_ids(&s), vec![top.id.clone()]);
                assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(beneath.id.clone()));
                assert_eq!(s.card(&beneath).counters.plague, Some(1));
            }

            #[test]
            fn s6_1_r78_a_plagued_reborn_unit_comes_back_without_its_token_and_the_next_turn_start_leaves_it() {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [BOMB], "library": lib(3) },
                    "p2": { "hand": [FILLER], "field": [{ "def": REBORN, "counters": { "plague": 1 } }], "library": lib(3) },
                }));
                let unit = s.card(REBORN).clone();

                s.end_turn();

                assert_eq!(destroyed_ids(&s), vec![unit.id.clone()]);
                assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(REBORN.to_string()));
                assert!(s.unit(P2, 1).and_then(|card| card.counters.plague).is_none());
                s.end_turn();
                s.end_turn();
                assert_eq!(destroyed_ids(&s), vec![unit.id.clone()]);
            }

            #[test]
            fn with_no_plagued_permanent_it_destroys_nothing() {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [VANILLA], "backrow": [BOMB], "library": lib(2) },
                    "p2": { "hand": [FILLER], "field": [MENACE], "library": lib(2) },
                }));

                s.end_turn();
                s.end_turn();

                assert_eq!(destroyed_ids(&s), Vec::<String>::new());
            }

            #[test]
            fn leaving_the_field_ends_it_once_it_is_gone_no_turn_start_destroys_anything() {
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": BOMB, "counters": { "plague": 1 } }],
                        "field": [{ "def": VANILLA, "counters": { "plague": 1 } }],
                        "library": lib(3),
                    },
                    "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "counters": { "plague": 1 } }], "library": lib(3) },
                }));
                s.end_turn(); // p2's Vanilla goes
                s.end_turn(); // p1's Vanilla and the Bomb go together
                let after = destroyed_ids(&s).len();
                assert_eq!(after, 3);

                s.end_turn();

                assert_eq!(destroyed_ids(&s).len(), after);
            }
        }

        mod radiant {
            use super::*;

            /// Issue #3139: the Radiant face has the Activate in place of the Cry: playing it places nothing.
            #[test]
            fn r384_playing_it_places_no_counter_and_asks_nothing_it_has_no_cry() {
                let mut s = in_hand(true);

                s.play(BOMB, json!({ "zone": 1 }));

                assert!(s.state().pending.is_none());
                assert!(placements(&s).is_empty());
                s.expect_in_zone(BOMB, "field");
            }

            #[test]
            fn r384_r689_its_activate_places_one_counter_on_the_permanent_a_prompt_names_once_a_turn() {
                let mut s = in_hand(true);
                let menace = s.card(MENACE).clone();
                s.play(BOMB, json!({ "zone": 1 }));

                s.activate(BOMB, json!({}));

                assert_eq!(must(s.state().pending.as_ref(), "the placement prompt").player_id, P1);
                s.answer(json!(menace.id));
                assert!(s.state().pending.is_none());
                assert_eq!(placements(&s), vec![1]);
                assert_eq!(s.card(&menace).counters.plague, Some(1));
                // Once a turn (R384): a second activation is refused...
                s.expect_refused(|s| s.activate(BOMB, json!({})));
                // ...and it serves again on the next turn of its controller.
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, P1);
                s.activate(BOMB, json!({}));
                s.answer(json!(s.card(MANA_WELL).id));
                assert_eq!(s.card(MANA_WELL).counters.plague, Some(1));
            }

            /// The Radiant face's counter is destroyed on the enemy's turn only (R400).
            #[test]
            fn r400_the_permanent_its_activate_marks_is_destroyed_at_the_opponents_turn_start() {
                let mut s = in_hand(true);
                let menace = s.card(MENACE).clone();
                s.play(BOMB, json!({ "zone": 1 }));
                s.activate(BOMB, json!({}));
                s.answer(json!(menace.id));

                s.end_turn();

                s.expect_in_zone(&menace, "graveyard");
            }

            #[test]
            fn r384_only_its_controller_activates_it() {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [VANILLA], "library": lib(2) },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": BOMB, "radiant": true }], "library": lib(2) },
                }));
                s.expect_refused(|s| s.activate(BOMB, json!({})));
                assert!(placements(&s).is_empty());
            }

            #[test]
            fn r400_at_the_start_of_the_opponents_turn_it_destroys_their_plagued_permanents_face_down_ones_included() {
                crate::register_all();
                let mut s = board(true, 0);
                let (Some(their_vanilla), Some(their_well), Some(their_trap)) =
                    (s.unit(P2, 1), s.backrow(P2, 1), s.backrow(P2, 2))
                else {
                    panic!("board");
                };

                s.end_turn();

                assert_eq!(
                    set_of(destroyed_ids(&s)),
                    set_of(vec![their_vanilla.id.clone(), their_well.id.clone(), their_trap.id.clone()]),
                );
            }

            #[test]
            fn r400_at_the_start_of_your_own_turn_it_does_nothing_your_plagued_permanents_and_the_bomb_itself_stay() {
                crate::register_all();
                let mut s = board(true, 2);
                s.end_turn();
                let before = destroyed_ids(&s).len();

                s.end_turn();

                assert_eq!(s.state().active, P1);
                assert_eq!(destroyed_ids(&s).len(), before);
                assert_eq!(s.unit(P1, 1).and_then(|unit| unit.counters.plague), Some(1));
                assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(BOMB.to_string()));
                assert_eq!(s.backrow(P1, 2).map(|card| card.def_id), Some(PAWN.to_string()));
            }

            #[test]
            fn r46_an_indestructible_enemy_permanent_survives_it_too() {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": BOMB, "radiant": true }], "library": lib(2) },
                    "p2": { "hand": [FILLER], "field": [{ "def": ROCK, "counters": { "plague": 1 } }], "library": lib(2) },
                }));

                s.end_turn();

                s.expect_in_zone(ROCK, "field");
            }
        }

        /// R767000: through v0.5.11p it had no Cry and no Activate, and cost (1) in the catalog it folded under.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11o_places_nothing_on_either_face() {
            let kept = script_v0_5_11o();
            assert!(kept.base.cry.is_none() && kept.radiant.cry.is_none());
            assert!(kept.base.activations.is_empty() && kept.radiant.activations.is_empty());
            for radiant in [false, true] {
                crate::under_kept(BOMB, script_v0_5_11o(), || {
                    let mut s = in_hand(radiant);
                    s.play(BOMB, json!({ "zone": 1 }));
                    assert!(s.state().pending.is_none(), "radiant {radiant}");
                    assert!(placements(&s).is_empty(), "radiant {radiant}");
                    s.expect_refused(|s| s.activate(BOMB, json!({})));
                });
            }
        }
    }
}
