//! SPEC §8.12 S+ #21 Jlockeed Shredder-12, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    12/16, Tribute 3, Pierce; "Float reduces Tribute requirement by 1.
//!            End of turn: Deal {damage} damage to all enemies, twice."
//!   Radiant: 24/32, Tribute 3, Pierce; the same.
//!
//! Tribute 3 is the play validator's (§6.3, R101), read off `staticFlags.tribute`, the keyword line's
//! printed Tribute (R366), lowered 1 by each of its owner's Floats while it waits in hand, never in the
//! deck (R2657002), never below 0 (`float_lowers_tribute`, R1841003, as S #86 Prototype-X2's). The end of
//! turn is #13's shape: one `damage_all` per pass (R51, R59), and "twice" is two passes, so every enemy
//! takes two damage instances, the hero last in each (R3553000).
//! Issue #4154 raised the Tribute from 2 and gave it the Float line; `script_v0_6_6g` keeps Tribute 2 with
//! no Float for the replays of v0.6.6g and before, and issue #3996 had raised it from 1, which
//! `script_v0_6_5b` keeps for the replays of v0.6.5b and before (R767000).

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-021";

/// "Tribute 3" (R101, R366: printed, declared nowhere), since issue #4154.
const TRIBUTE: i32 = 3;
/// "Tribute 2", after v0.6.5b through v0.6.6g (issue #3996).
const TRIBUTE_V0_6_6G: i32 = 2;
/// "Tribute 1", through v0.6.5b.
const TRIBUTE_V0_6_5B: i32 = 1;
/// "twice": two end-of-turn passes over every enemy (R3553000, R1748003: a printed word).
const TWICE: i32 = 2;

/// R3553000: two `damage_all` passes of the declared number `damage`, each one instance per enemy
/// unit and the enemy hero, the hero last (R51, R59); `tribute` to play it, and `float`: whether
/// "Float reduces Tribute requirement by 1" is printed (since issue #4154), a hand trigger (R1841003).
fn shredder(tribute: i32, float: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(tribute),
            ..StaticFlags::default()
        }),
        hand_triggers: if float {
            vec![float_lowers_tribute("stinkyscapeplus-021-float-hand")]
        } else {
            vec![]
        },
        end_of_turn: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            (0..TWICE)
                .map(|_| damage_all(json_as(json!({ "side": "enemy", "amount": amount, "heroes": true }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = shredder(TRIBUTE, true);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: after v0.6.5b through v0.6.6g it was Tribute 2 with no Float (issue #4154); `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    let base = shredder(TRIBUTE_V0_6_6G, false);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.6.5b it was Tribute 1 (issue #3996); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    let base = shredder(TRIBUTE_V0_6_5B, false);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #21 Jlockeed Shredder-12 — SPEC §8.12 row S+ 21 (R3553000; R51, R59, R101, R366, R1841003).
// End-of-turn double AoE over enemy units and hero; Tribute 3 to play, each Float lowering it by 1 (2
// with no Float through v0.6.6g, 1 through v0.6.5b).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    use crate::{next_turn_of, scenario};

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a Spell, p1's spare card (§2.5).

    /// Every `damage` event of the most recent step, as `(targetId, amount)`.
    fn hits(s: &Scenario) -> Vec<(String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    fn shredder_board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "hand": [FILLER],
                "library": [FILLER]
            },
            "p2": {
                "field": [
                    { "def": VANILLA, "radiant": true, "lane": 1 },
                    { "def": VANILLA, "radiant": true, "lane": 2 },
                ],
                "hand": [FILLER],
                "library": [FILLER]
            }
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r3553000_twice_is_two_damage_instances_per_enemy_the_hero_last_each_time() {
            let mut s = shredder_board(false);

            s.end_turn();

            // Two passes of one instance per target: two 12/12s then the hero, twice.
            let hits = hits(&s);
            assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![2, 2, 2, 2, 2, 2]);
            let heroes: Vec<usize> = hits
                .iter()
                .enumerate()
                .filter(|(_, hit)| hit.0 == "hero-p2")
                .map(|(at, _)| at)
                .collect();
            assert_eq!(heroes, vec![2, 5]);
            // Both passes landed on the units too: 12 minus 2 twice.
            for lane in [1, 2] {
                let id = s.unit(PlayerId::P2, lane).expect("still standing").id;
                s.expect_stats(id.as_str(), json!({ "health": 8, "maxHealth": 12 }));
            }
            s.expect_health(PlayerId::P2, 26);
            // "all ENEMIES": nothing of its own.
            s.expect_health(PlayerId::P1, 30);
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn r3553000_a_unit_the_first_pass_kills_still_takes_the_second_r59() {
            let mut s = scenario(json!({
                "p1": { "field": [ID], "hand": [FILLER], "library": [FILLER] },
                "p2": { "field": ["core-020"], "hand": [FILLER], "library": [FILLER] },
            }));
            s.end_turn();
            // 6/1: dead after the first 2, hit by the second all the same, then destroyed once.
            let unit = s.card("core-020").id.clone();
            assert_eq!(hits(&s).iter().filter(|hit| hit.0 == unit).count(), 2);
            s.expect_in_zone("core-020", "graveyard");
            s.expect_health(PlayerId::P2, 26);
        }

        #[test]
        fn opponents_end_of_turn_is_silent() {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": VANILLA, "radiant": true }], "hand": [FILLER], "library": [FILLER] },
                // The Shredder belongs to the player who is NOT ending a turn here.
                "p2": { "field": [ID], "hand": [FILLER], "library": [FILLER] }
            }));

            s.end_turn();

            s.expect_health(PlayerId::P1, 30);
            assert_eq!(
                s.events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::Damage)
                    .count(),
                0
            );
        }

        /// p1 holds the Shredder (`radiant` its face) beside three Units to tribute.
        fn to_play(radiant: bool) -> Scenario {
            scenario(json!({
                "p1": {
                    "hand": [{ "def": ID, "radiant": radiant }],
                    "field": [VANILLA, VANILLA, VANILLA],
                    "library": [FILLER],
                },
                "p2": { "hand": [FILLER], "library": [FILLER] },
            }))
        }

        #[test]
        fn r101_it_needs_tribute_3_on_both_faces() {
            for radiant in [false, true] {
                let mut s = to_play(radiant);
                assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
                let [first, second, third] = [1, 2, 3].map(|lane| s.unit_at(PlayerId::P1, lane).id);
                // No tribute, or two, is refused; three Units pay it.
                s.expect_refused(|s| s.play(ID, json!({ "zone": 4 })));
                s.expect_refused(|s| {
                    s.play(ID, json!({ "zone": 4, "tributes": [first.as_str(), second.as_str()] }))
                });
                s.play(
                    ID,
                    json!({ "zone": 4, "tributes": [first.as_str(), second.as_str(), third.as_str()] }),
                );
                s.expect_in_zone(ID, "field");
                for id in [&first, &second, &third] {
                    s.expect_in_zone(id.as_str(), "graveyard");
                }
            }
        }

        /// R1841003, R2657002: each of its owner's Floats lowers its Tribute by 1 while it waits in hand,
        /// never in the deck, never below 0; a lowered Tribute 2 is paid by two Units.
        #[test]
        fn r1841003_each_float_lowers_its_tribute_by_1_in_hand_never_in_the_deck() {
            for radiant in [false, true] {
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                        "field": [VANILLA, VANILLA],
                        "library": [FILLER, FILLER, FILLER, FILLER, { "def": ID, "radiant": radiant }],
                        "mana": 2,
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
                }));
                let in_hand = s
                    .hand(PlayerId::P1)
                    .into_iter()
                    .find(|card| card.def_id == ID)
                    .expect("the Shredder in hand")
                    .id;
                let in_deck = s
                    .pile(PlayerId::P1, "library")
                    .into_iter()
                    .find(|card| card.def_id == ID)
                    .expect("the Shredder in the deck")
                    .id;
                let tribute = |s: &Scenario, id: &str| tribute_cost_of(s.state(), s.card(id));
                s.end_turn();
                assert_eq!(
                    (tribute(&s, &in_hand), tribute(&s, &in_deck)),
                    (TRIBUTE - 1, TRIBUTE),
                    "radiant {radiant}"
                );
                next_turn_of(&mut s, PlayerId::P1);
                assert_eq!(tribute(&s, &in_hand), TRIBUTE - 1, "radiant {radiant}");
                let [first, second] = [1, 2].map(|lane| s.unit_at(PlayerId::P1, lane).id);
                s.play(&in_hand, json!({ "zone": 3, "tributes": [first.as_str(), second.as_str()] }));
                s.expect_in_zone(&in_hand, "field");
            }
        }

        /// R767000: after v0.6.5b through v0.6.6g two Units paid it and a Float lowered nothing (issue #4154).
        #[test]
        fn r767000_the_script_kept_for_v0_6_6g_needs_tribute_2_and_no_float_lowers_it() {
            crate::under_kept(ID, script_v0_6_6g(), || {
                let mut s = to_play(false);
                assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE_V0_6_6G);
                assert!(script_v0_6_6g().base.hand_triggers.is_empty(), "no Float");
                let [first, second] = [1, 2].map(|lane| s.unit_at(PlayerId::P1, lane).id);
                s.expect_refused(|s| s.play(ID, json!({ "zone": 4, "tributes": [first.as_str()] })));
                s.play(ID, json!({ "zone": 4, "tributes": [first.as_str(), second.as_str()] }));
                s.expect_in_zone(ID, "field");
                assert!(s.unit(PlayerId::P1, 3).is_some(), "the third Unit stays");
            });
        }

        /// R767000: through v0.6.5b one Unit paid it (issue #3996).
        #[test]
        fn r767000_the_script_kept_for_v0_6_5b_needs_tribute_1() {
            crate::under_kept(ID, script_v0_6_5b(), || {
                let mut s = to_play(false);
                assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE_V0_6_5B);
                let first = s.unit_at(PlayerId::P1, 1).id;
                s.play(ID, json!({ "zone": 4, "tributes": [first.as_str()] }));
                s.expect_in_zone(ID, "field");
                s.expect_in_zone(first.as_str(), "graveyard");
                assert!(s.unit(PlayerId::P1, 2).is_some(), "the second Unit stays");
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3553000_the_radiant_face_deals_5_twice() {
            let mut s = shredder_board(true);

            s.end_turn();

            let hits = hits(&s);
            assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![5, 5, 5, 5, 5, 5]);
            for lane in [1, 2] {
                let id = s.unit(PlayerId::P2, lane).expect("still standing").id;
                s.expect_stats(id.as_str(), json!({ "health": 2, "maxHealth": 12 }));
            }
            s.expect_health(PlayerId::P2, 20);
            s.expect_health(PlayerId::P1, 30);
        }
    }
}
