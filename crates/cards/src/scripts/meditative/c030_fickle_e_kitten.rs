//! M #30 Fickle E-Kitten (SPEC §8.8 row 30): (1) Unit, Felinor, Epic, 3/4 → 6/8.
//!
//! Base:    "Start of turn: If your opponent has a more expensive permanent than you, they gain
//! control of this. Otherwise, shuffle a Love Bomb into your deck."
//! Radiant: "Start of turn: If your opponent has a more expensive permanent than you and a larger
//! deck, they gain control of this. Otherwise, shuffle a Radiant Love Bomb into your deck."
//! Engine: at the start of its controller's turn (§2.2, R62's queue).
//! - **The condition** compares each side's most expensive permanent with NEW read helper
//!   `highest_permanent_cost(state, player)` (R901, MD-B5), this card counting for its controller.
//!   The Radiant face also needs the opponent's library to be larger than its controller's.
//! - **When it holds**, `give_control({ instanceId: self })` (R1423): R15's placement, R171's entry.
//!   With no free unit zone the card stays, and the "Otherwise" clause does not run.
//! - **Otherwise**, `shuffle_into({ defId: "meditative-030-1", count: 1, radiant: face })` into its
//!   controller's deck.
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

fn kitten(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
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
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: kitten(false),
        radiant: kitten(true),
    }
}

// M #30 Fickle E-Kitten — SPEC §8.8 row 30, BUILD M10 row M 30: "At its controller's start of
// turn: with the opponent's highest field cost strictly above yours (tops of piles and backrow, X
// cards at their X, face-down cards at their public cost; MD-B5) it moves to their side by R15,
// summoning sick (R171), `controlChanged` public; equal or lower, or with no enemy permanent, a
// base Love Bomb is shuffled into its controller's deck; with their side full it stays and nothing
// is shuffled; under its new controller the comparison runs from their side; radiant 6/8, it moves
// only when their deck is also larger, and the Love Bomb is Radiant".
#[cfg(test)]
mod tests {
    use super::*;
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

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

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

    /// Past p2's turn and back to p1's start of turn, where the Kitten asks its question.
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

                to_p1_start(&mut s);

                // R15: the same lane on the receiver's side, which is free.
                assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(id.clone()));
                assert!(s.unit(P1, 1).is_none());
                assert_eq!(s.card(&id).controller, P2);
                assert_eq!(s.card(&id).owner, P1);
                // R171: it entered its new side this turn, summoning sick with a fresh exertion.
                let now = s.card(&id).clone();
                assert_eq!(now.summoned_turn, Some(s.state().turn));
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
            fn r901_equal_or_lower_shuffles_a_base_love_bomb() {
                let mut s = kittened("kitten-equal", false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));

                to_p1_start(&mut s);

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

                to_p1_start(&mut s);

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

                to_p1_start(&mut s);

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

                to_p1_start(&mut s);

                assert_eq!(s.card(KITTEN).controller, P2);
                assert_eq!(control_changed(s.events()).len(), 1);
            }

            #[test]
            fn r901_with_their_side_full_it_stays_and_nothing_is_shuffled() {
                let full: Vec<Value> = (1..=5).map(|lane| json!({ "def": MENACE, "lane": lane })).collect();
                let mut s = kittened("kitten-full", false, json!([]), json!(full));

                to_p1_start(&mut s);

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

                to_p1_start(&mut s);
                assert_eq!(s.card(KITTEN).controller, P2);

                // p2's next start of turn: p2 holds 4 (the 7/7) against p1's 3, so no move —
                // instead p2, the new controller, is fed the bomb.
                s.end_turn();
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
                to_p1_start(&mut smaller);
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
                to_p1_start(&mut larger);
                assert_eq!(larger.card(KITTEN).controller, P2);
            }

            #[test]
            fn shuffles_a_radiant_love_bomb_and_is_6_8() {
                let mut s = kittened("kitten-radiant", true, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));

                to_p1_start(&mut s);

                let bombs = bombs_in(&s, P1);
                assert_eq!(bombs.len(), 1);
                assert!(bombs[0].radiant);
                let me = s.unit(P1, 1).expect("the Kitten on the field");
                s.expect_stats(&me, json!({ "attack": 6, "health": 8, "maxHealth": 8 }));
            }
        }
    }
}
