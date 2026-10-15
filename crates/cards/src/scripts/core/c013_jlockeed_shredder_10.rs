//! #13 Jlockeed Shredder-10 (SPEC §8.1): 8/10 → 16/20 Unit, cost 3. Base "End of turn: deal 2 damage
//! to each enemy unit and the enemy hero", radiant "5 damage" — a radiant cell that changes only a
//! number changes only that number (§8 Conventions). §8's Engine cell: "One damage instance per
//! target, controller's end of turn".
//!
//! R51: every enemy unit plus the enemy hero, ONE damage instance each, so each hit goes through
//! §4.4's pipeline alone and meets that target's Armor, Divine Shield and Anti-oneshot cap.
//!
//! R59: the state check never runs between the hits of one effect, so a unit the first hit kills is
//! still on the field while the rest land. `damage_all` (engine/src/effects/damage.rs) is one effect
//! and `end_turn` runs `state_check` after each end-of-turn hook. The scan for `endOfTurn` hooks is
//! narrowed to the player whose turn is ending, so this card needs no check of its own.

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-013";

/// R51: every enemy unit and the enemy hero, one instance each, in one effect (R59).
fn shred(amount: i32) -> Effect {
    damage_all(json_as(json!({ "side": "enemy", "amount": amount, "heroes": true })))
}

/// The amount is the declared number `damage` (R386): 2, 5 on the Radiant face.
pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| vec![shred(param(&*ctx, "damage"))])),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #13 Jlockeed Shredder-10 (SPEC §8.1, BUILD M4-T4 row 13): "Controller's end of turn: 2 to every
// enemy unit and hero as separate instances; units it kills die after all hits land (R59); not on
// the opponent's end; radiant 5 (R51)".
//
// The board: p2 lane 1 is Pointmaster (7/1), which 2 damage kills, and lane 2 is Mr. Vanilla (4/4),
// which survives at 2. A lane-1 death makes R59 observable: the later hits must still land.
//
// Both sides get a library, so nobody takes §2.4 fatigue damage when the opponent draws, and a spare
// hand card, so the fresh turn has an action and cannot auto-end onwards (§2.5).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

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
                "field": [{ "def": "core-013", "radiant": radiant }],
                "hand": ["core-005"],
                "library": ["core-005"]
            },
            "p2": {
                "field": ["core-020", "core-008"],
                "hand": ["core-005"],
                "library": ["core-005"]
            }
        }))
    }

    mod n13_jlockeed_shredder_10 {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r51_deals_2_to_every_enemy_unit_and_the_enemy_hero_at_its_controller_s_end_of_turn() {
                crate::register_all();
                let mut s = shredder_board(false);
                let pointmaster = s.card("core-020").clone();

                s.end_turn();

                s.expect_in_zone(&pointmaster, "graveyard"); // 7/2 took 2
                s.expect_stats("core-008", json!({ "health": 2, "maxHealth": 4 })); // 4/4 took 2
                s.expect_health(PlayerId::P2, 28);
                s.expect_health(PlayerId::P1, 30); // "each ENEMY unit and the enemy hero": nothing of its own
            }

            #[test]
            fn r51_lands_one_separate_damage_instance_per_target_the_hero_last() {
                crate::register_all();
                let mut s = shredder_board(false);

                s.end_turn();

                // Three instances, not one area effect: two units in lane order, then the hero.
                assert_eq!(hits(&s).iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![2, 2, 2]);
                assert_eq!(hits(&s).get(2).map(|hit| hit.0.as_str()), Some("hero-p2"));
            }

            #[test]
            fn r59_kills_land_before_anything_dies_the_state_check_runs_after_the_whole_hook() {
                crate::register_all();
                let mut s = shredder_board(false);

                s.end_turn();

                // Lane 1 was dead on the first hit, yet the lane-2 unit and the hero were still hit, and
                // the `destroyed` event comes after the last `damage` — the state check never ran
                // between them.
                s.expect_events(json!(["damage", "damage", "damage", "destroyed"]));
                let types: Vec<GameEventType> = s.last_events().iter().map(|event| event.event_type()).collect();
                let first_destroyed = types.iter().position(|kind| *kind == GameEventType::Destroyed);
                let last_damage = types.iter().rposition(|kind| *kind == GameEventType::Damage);
                assert!(first_destroyed.is_some());
                assert!(last_damage.is_some());
                assert!(first_destroyed > last_damage);
            }

            #[test]
            fn does_nothing_on_the_opponent_s_end_of_turn_s6_2_its_controller_s_turn_alone() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                    // The Shredder belongs to the player who is NOT ending a turn here.
                    "p2": { "field": ["core-013"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                s.end_turn();

                s.expect_health(PlayerId::P1, 30);
                s.expect_stats("core-008", json!({ "health": 4, "maxHealth": 4 }));
                assert_eq!(
                    s.events()
                        .iter()
                        .filter(|event| event.event_type() == GameEventType::Damage)
                        .count(),
                    0
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r51_deals_5_to_every_enemy_unit_and_the_enemy_hero() {
                crate::register_all();
                let mut s = shredder_board(true);
                let pointmaster = s.card("core-020").clone();
                let vanilla = s.card("core-008").clone();

                s.end_turn();

                s.expect_in_zone(&pointmaster, "graveyard"); // 7/2
                s.expect_in_zone(&vanilla, "graveyard"); // 4/4, which 2 would have survived
                s.expect_health(PlayerId::P2, 25);
                assert_eq!(hits(&s).iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![5, 5, 5]);
            }
        }

        #[test]
        fn r386_an_upgrade_deals_3_and_a_degrade_1() {
            for (upgrade, amount) in [(true, 3), (false, 1)] {
                crate::register_all();
                let mut s = shredder_board(false);
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-013", "damage")
                } else {
                    crate::degrade_number(&mut s, "core-013", "damage")
                };
                assert_eq!(moved, amount);
                s.end_turn();
                s.expect_health(PlayerId::P2, 30 - amount);
            }
        }
    }
}
