//! SPEC §8.12 S+ #21 Jlockeed Shredder-12, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    12/16, Tribute 1, Pierce; "End of turn: Deal {damage} damage to all enemies, twice."
//!   Radiant: 24/32, Tribute 1, Pierce; the same.
//!
//! Tribute 1 is the play validator's (§6.3, R101), read off `staticFlags.tribute`, the keyword line's
//! printed Tribute (R366). The end of turn is #13's shape: one `damage_all` per pass (R51, R59), and
//! "twice" is two passes, so every enemy takes two damage instances, the hero last in each (R3553000).

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-021";

/// "Tribute 1" (R101, R366: printed, declared nowhere).
const TRIBUTE: i32 = 1;
/// "twice": two end-of-turn passes over every enemy (R3553000, R1748003: a printed word).
const TWICE: i32 = 2;

/// R3553000: two `damage_all` passes of the declared number `damage`, each one instance per enemy
/// unit and the enemy hero, the hero last (R51, R59).
fn shredder() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
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
    let base = shredder();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #21 Jlockeed Shredder-12 — SPEC §8.12 row S+ 21 (R3553000; R51, R59, R101, R366).
// End-of-turn double AoE over enemy units and hero; Tribute 1 to play.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    use crate::scenario;

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

        #[test]
        fn r101_it_needs_tribute_1() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID], "field": [VANILLA], "library": [FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER] },
            }));
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
            // No tribute is refused; one Unit pays it.
            s.expect_refused(|s| s.play(ID, json!({ "zone": 2 })));
            let first = s.unit_at(PlayerId::P1, 1).id;
            s.play(ID, json!({ "zone": 2, "tributes": [first.as_str()] }));
            s.expect_in_zone(ID, "field");
            s.expect_in_zone(first.as_str(), "graveyard");
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
