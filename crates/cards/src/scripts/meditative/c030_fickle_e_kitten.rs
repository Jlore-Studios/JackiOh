//! M #30 Fickle E-Kitten (SPEC §8.8 row 30): (1) Unit, Felinor, Epic, 3/4 → 6/8.
//!
//! Base:    "End of turn: If your opponent has a pricier permanent, they gain control of this.
//! Otherwise, shuffle a Love Bomb into your deck."
//! Radiant: "End of turn: If your opponent has a pricier permanent and a bigger deck, they gain
//! control of this. Otherwise, shuffle a Radiant Love Bomb into your deck."
//! Engine: at the end of its controller's turn (§2.2, R62's queue).
//! - **The condition** is `opponent_outbids`, this card counting for its controller.
//! - **When it holds**, `give_control({ instanceId: self })` (R1423): R15's placement, R171's entry.
//!   With no free unit zone the card stays, and the "Otherwise" clause does not run. Given at your
//!   end of turn, it has been theirs since their turn began, so it may attack on it.
//!
//! Its new controller asks again from their side, so it settles with the cheaper board.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-030";

/// M #30.1 Love Bomb, the token the "Otherwise" clause shuffles (§7).
const LOVE_BOMB: &str = "meditative-030-1";

/// Whether the opponent's most expensive permanent beats the controller's (R901, MD-B5), on the
/// Radiant face only when their deck is also larger.
fn opponent_outbids(state: &GameState, controller: PlayerId, needs_larger_deck: bool) -> bool {
    let foe = opponent_of(controller);
    let (Some(theirs), Some(ours)) =
        (highest_permanent_cost(state, foe), highest_permanent_cost(state, controller))
    else {
        return false;
    };
    if theirs <= ours {
        return false;
    }
    if needs_larger_deck
        && zone_count(state, foe, OffFieldZone::Library)
            <= zone_count(state, controller, OffFieldZone::Library)
    {
        return false;
    }
    true
}

/// The card's question, asked at its controller's end of turn, or at their start of turn through
/// v0.4.0f.
fn kitten(radiant: bool, at_end: bool) -> Script {
    let ask = hook(move |ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let (id, controller) = (me.id.clone(), me.controller);
            if opponent_outbids(ctx.state, controller, radiant) {
                return vec![give_control(json_as(json!({ "instanceId": id })))];
            }
            vec![shuffle_into(json_as(json!({
                "defId": LOVE_BOMB,
                "count": 1,
                "player": "self",
                "radiant": radiant,
            })))]
    });
    if at_end {
        Script {
            end_of_turn: Some(ask),
            ..Script::default()
        }
    } else {
        Script {
            start_of_turn: Some(ask),
            ..Script::default()
        }
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: kitten(false, true),
        radiant: kitten(true, true),
    }
}

/// R767000: the scripts v0.4.0f and every version before it ran, asking at the start of the turn;
/// `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_4_0f() -> CardScripts {
    CardScripts {
        base: kitten(false, false),
        radiant: kitten(true, false),
    }
}

// M #30 Fickle E-Kitten — SPEC §8.8 row 30, BUILD M10 row M 30.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const KITTEN: &str = "meditative-030";
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const SEVEN_SEVEN: &str = "core-025"; // (4) Unit 7/7.
    const MOON: &str = "classicplus-022"; // (1) Trap, set face-down.
    const CHALICE: &str = "classic-087"; // (X) Field Spell.
    const FILLER: &str = "core-005";

    /// p1 holds the Kitten (base unless `radiant_face`); `p1_field`, `p2_field` and `p2_backrow`
    /// are the setup's other permanents. Both libraries hold four filler.
    fn kittened(seed: &str, radiant_face: bool, p1_field: Value, p2_field: Value) -> Scenario {
        kittened_with(seed, radiant_face, p1_field, p2_field, json!([]), 4, 4)
    }

    fn kittened_with(
        seed: &str,
        radiant_face: bool,
        p1_field: Value,
        p2_field: Value,
        p2_backrow: Value,
        p1_library: usize,
        p2_library: usize,
    ) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": KITTEN, "radiant": radiant_face }, FILLER, FILLER],
                "field": p1_field,
                "library": filler(p1_library),
            },
            "p2": {
                "hand": [FILLER, FILLER],
                "field": p2_field,
                "backrow": p2_backrow,
                "library": filler(p2_library),
            },
        }));
        s.play(KITTEN, json!({}));
        s
    }

    /// The Love Bombs in `player`'s library.
    fn bombs_in(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.state().players[player]
            .library
            .iter()
            .filter(|card| card.def_id == LOVE_BOMB)
            .cloned()
            .collect()
    }

    fn control_changed(events: &[GameEvent]) -> Vec<Value> {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::ControlChanged { .. }))
            .map(crate::js)
            .collect()
    }

    /// Past p2's turn and back to p1's start of turn, where the kept Kitten asks its question.
    fn to_p1_start(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
    }

    mod m30_fickle_e_kitten {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r901_a_strictly_higher_enemy_permanent_takes_it_by_r15_summoning_sick() {
                let mut s = kittened("kitten-taken", false, json!([]), json!([{ "def": MENACE, "lane": 2 }]));
                let id = s.card(KITTEN).id.clone();
                let turn = s.state().turn;

                s.end_turn();

                // R15: the same lane on the receiver's side, which is free.
                assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(id.clone()));
                assert!(s.unit(P1, 1).is_none());
                assert_eq!(s.card(&id).controller, P2);
                assert_eq!(s.card(&id).owner, P1);
                // R171: it entered its new side on p1's turn, with a fresh exertion.
                let now = s.card(&id).clone();
                assert_eq!(now.summoned_turn, Some(turn));
                assert_eq!(
                    now.exertion,
                    Exertion {
                        attacked: false,
                        switched: false,
                        attacks: None
                    }
                );
                assert_eq!(control_changed(s.events()).len(), 1);
                // The "Otherwise" clause did not run.
                assert!(bombs_in(&s, P1).is_empty());
                assert!(bombs_in(&s, P2).is_empty());
            }

            #[test]
            fn r171_given_at_your_end_of_turn_it_attacks_on_theirs() {
                let mut s = kittened("kitten-attacks", false, json!([]), json!([{ "def": MENACE, "lane": 2 }]));
                let id = s.card(KITTEN).id.clone();

                s.end_turn();

                assert_eq!(s.state().active, P2);
                assert_eq!(s.card(&id).controller, P2);
                s.attack(&id, "hero");
                s.expect_health(P1, 30 - 3);
            }

            #[test]
            fn r901_equal_or_lower_shuffles_a_base_love_bomb() {
                let mut s = kittened("kitten-equal", false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));

                s.end_turn();

                assert_eq!(s.card(KITTEN).controller, P1);
                assert!(control_changed(s.events()).is_empty());
                let bombs = bombs_in(&s, P1);
                assert_eq!(bombs.len(), 1);
                assert_eq!(bombs[0].owner, P1);
                assert!(!bombs[0].radiant);
            }

            #[test]
            fn r901_with_no_enemy_permanent_it_shuffles() {
                let mut s = kittened("kitten-none", false, json!([]), json!([]));

                s.end_turn();

                assert_eq!(s.card(KITTEN).controller, P1);
                assert_eq!(bombs_in(&s, P1).len(), 1);
            }

            #[test]
            fn r901_a_face_down_trap_counts_at_its_shown_cost() {
                let mut s = kittened_with(
                    "kitten-trap",
                    false,
                    json!([]),
                    json!([]),
                    json!([{ "def": MOON, "lane": 1, "faceUp": false }]),
                    4,
                    4,
                );
                s.card_mut(MOON).cost_override = Some(5);

                s.end_turn();

                assert_eq!(s.card(KITTEN).controller, P2);
                assert_eq!(control_changed(s.events()).len(), 1);
            }

            #[test]
            fn r901_an_x_card_counts_at_its_played_x() {
                let mut s = kittened_with(
                    "kitten-x",
                    false,
                    json!([]),
                    json!([]),
                    json!([{ "def": CHALICE, "lane": 1 }]),
                    4,
                    4,
                );
                s.card_mut(CHALICE).x = Some(4);

                s.end_turn();

                assert_eq!(s.card(KITTEN).controller, P2);
                assert_eq!(control_changed(s.events()).len(), 1);
            }

            #[test]
            fn r901_with_their_side_full_it_stays_and_nothing_is_shuffled() {
                let full: Vec<Value> = (1..=5).map(|lane| json!({ "def": MENACE, "lane": lane })).collect();
                let mut s = kittened("kitten-full", false, json!([]), json!(full));

                s.end_turn();

                // R1423's no-room no-op: still p1's, in its lane, and no bomb for anyone.
                assert_eq!(s.card(KITTEN).controller, P1);
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(KITTEN.to_string()));
                assert!(control_changed(s.events()).is_empty());
                assert!(bombs_in(&s, P1).is_empty());
                assert!(bombs_in(&s, P2).is_empty());
            }

            #[test]
            fn under_its_new_controller_it_asks_from_their_side() {
                let mut s = kittened(
                    "kitten-flip",
                    false,
                    json!([{ "def": MENACE, "lane": 2 }]),
                    json!([{ "def": SEVEN_SEVEN, "lane": 2 }]),
                );

                s.end_turn();
                assert_eq!(s.card(KITTEN).controller, P2);

                // p2's end of turn: p2 holds 4 (the 7/7) against p1's 3, so no move — instead p2,
                // the new controller, is fed the bomb.
                s.end_turn();

                assert_eq!(s.card(KITTEN).controller, P2);
                assert_eq!(control_changed(s.events()).len(), 1);
                assert!(bombs_in(&s, P1).is_empty());
                // One Love Bomb went to p2's deck. p2's own draw may land on it and cast it on
                // the same turn, so the total spans the library and the graveyards.
                let shuffled = s
                    .events()
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::ShuffledIn { def_id, .. } if def_id == LOVE_BOMB)
                    })
                    .count();
                assert_eq!(shuffled, 1);
                let bomb = bombs_in(&s, P2).into_iter().next().or_else(|| {
                    s.pile(P2, "graveyard").into_iter().find(|card| card.def_id == LOVE_BOMB)
                });
                assert_eq!(bomb.as_ref().map(|card| card.owner), Some(P2));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r901_it_moves_only_when_their_deck_is_also_larger() {
                // Their permanent is higher but their deck is not larger: no move, a bomb.
                let mut smaller = kittened_with(
                    "kitten-deck-small",
                    true,
                    json!([]),
                    json!([{ "def": MENACE, "lane": 2 }]),
                    json!([]),
                    4,
                    2,
                );
                smaller.end_turn();
                assert_eq!(smaller.card(KITTEN).controller, P1);
                assert_eq!(bombs_in(&smaller, P1).len(), 1);

                // Their permanent is higher and their deck is larger: it moves.
                let mut larger = kittened_with(
                    "kitten-deck-large",
                    true,
                    json!([]),
                    json!([{ "def": MENACE, "lane": 2 }]),
                    json!([]),
                    2,
                    4,
                );
                larger.end_turn();
                assert_eq!(larger.card(KITTEN).controller, P2);
            }

            #[test]
            fn shuffles_a_radiant_love_bomb_and_is_6_8() {
                let mut s = kittened("kitten-radiant", true, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));

                s.end_turn();

                let bombs = bombs_in(&s, P1);
                assert_eq!(bombs.len(), 1);
                assert!(bombs[0].radiant);
                let me = s.unit(P1, 1).expect("the Kitten on the field");
                s.expect_stats(&me, json!({ "attack": 6, "health": 8, "maxHealth": 8 }));
            }
        }

        /// R767000: the scripts kept for the replays of v0.4.0f and before, which asked at the start
        /// of the turn.
        mod v0_4_0f {
            use super::*;

            #[test]
            fn r901_r767000_it_asks_at_its_controllers_start_of_turn_not_their_end() {
                crate::under_kept(KITTEN, super::super::super::script_v0_4_0f(), || {
                    let mut s = kittened("kitten-kept", false, json!([]), json!([{ "def": MENACE, "lane": 2 }]));
                    let id = s.card(KITTEN).id.clone();
                    s.end_turn();
                    assert_eq!(s.card(&id).controller, P1);
                    s.end_turn();
                    assert_eq!(s.card(&id).controller, P2);

                    let mut radiant = kittened("kitten-kept-radiant", true, json!([]), json!([]));
                    to_p1_start(&mut radiant);
                    let bombs = bombs_in(&radiant, P1);
                    assert_eq!(bombs.len(), 1);
                    assert!(bombs[0].radiant);
                });
            }
        }
    }
}
