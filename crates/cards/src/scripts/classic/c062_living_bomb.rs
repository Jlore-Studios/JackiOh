//! C #62 Living Bomb (SPEC §8.6 row 62). (1) Field Spell, Rare.
//!   Base:    "At the start of each player's turn: Destroy every permanent that player controls with a
//!            Plague Counter on it."
//!   Radiant: "At the start of your opponent's turn: Destroy every permanent they control with a Plague
//!            Token on it."
//!   Engine:  "A start-of-turn trigger on both players' turns (Radiant: the opponent's only), in R68's
//!            order; the designer's "Plague Counter" is the Plague Counter. "They destroy all cards" is the
//!            turn player's own permanents, face-down ones included (R400): the only reading under which
//!            the Radiant face is the stronger one. Indestructible permanents stay (§6.1). Tunes: none."
//!
//! R400: at the start of a player's turn, every permanent THAT player controls with at least one Plague
//! Token on it is destroyed — the top of each unit pile and every backrow card, face-down ones included,
//! Living Bomb itself when it is the turn player's and carries a token — and the other player's are left
//! alone. Each is an ordinary §6.3 destroy, all in one effect, so they die together at the one state
//! check after it (§4.5, R59): an Indestructible one is knocked into Attack Position and stays (R46), a
//! Reborn unit comes back, Death hooks run. A card's tokens are counters, which R78 clears when it leaves
//! the field, so a card bounced and played again carries none. The set is read as the trigger resolves.
//!
//! Each turn's half is a start-of-turn hook queued at R62's start-of-turn trigger point in R68's order:
//! its controller's own turn is its `startOfTurn` (the base face only), the opponent's its
//! `startOfOpponentTurn`, which the engine queues right after the active player's own hooks — so a
//! plagued Fed Fauci of the turn player's still gains its mana before it is destroyed.
//!
//! No declared numbers: the text has none.

use jackioh_engine::effects::destroy;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-062";

/// R400: one destroy for each permanent `player` controls with a Plague Counter on it, in R68's order.
fn destroy_plagued(ctx: &EffectContext<'_>, player: PlayerId) -> Vec<Effect> {
    permanents_on_field(&ctx.state, player)
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

pub fn script() -> CardScripts {
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

// C #62 Living Bomb — SPEC §8.6 row 62, BUILD M9 Classic row C 62: "At the start of each player's turn,
// in R68's order: destroy every permanent that player controls with a Plague Counter on it, face-down ones
// and Living Bomb itself included, the other player's untouched (R400); Indestructible ones survive
// (R46); a card's tokens are gone once it leaves (R78); radiant: only at the start of your opponent's
// turn, and only their permanents; no tuned numbers".
//
// Both halves are start-of-turn hooks queued at R62's start-of-turn trigger point in R68's order: its own
// turn's `startOfTurn`, and the opponent's turn's `startOfOpponentTurn`, after the turn player's hooks.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;
    use std::sync::Arc;

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

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

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

    /// p1's Living Bomb, and a plagued and a clean board on each side.
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

    mod c_62_living_bomb {
        use super::*;

        #[test]
        fn has_no_declared_numbers_the_base_face_holds_both_turns_the_radiant_face_the_opponents_only() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["id"], BOMB);
            assert!(def.get("params").is_none());
            let scripts = script();
            assert!(scripts.base.start_of_turn.is_some());
            assert!(scripts.base.start_of_opponent_turn.is_some());
            assert!(scripts.radiant.start_of_turn.is_none());
            // TS `expect(radiant.startOfOpponentTurn).toBe(base.startOfOpponentTurn)`: the same shared hook.
            assert!(match (&scripts.radiant.start_of_opponent_turn, &scripts.base.start_of_opponent_turn) {
                (Some(radiant_hook), Some(base_hook)) => Arc::ptr_eq(radiant_hook, base_hook),
                _ => false,
            });
        }

        mod base {
            use super::*;

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
                // The clean ones stay, the clean face-down trap still unnamed to p1.
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
                // p2's survivors are untouched on p1's turn.
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
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
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
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": BOMB, "radiant": true }], "library": lib(2) },
                    "p2": { "hand": [FILLER], "field": [{ "def": ROCK, "counters": { "plague": 1 } }], "library": lib(2) },
                }));

                s.end_turn();

                s.expect_in_zone(ROCK, "field");
            }
        }
    }
}
