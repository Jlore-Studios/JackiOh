//! SPEC §8.11 Training Grounds 17 Flaming Felinor: (2) Unit, Felinor, Rare, 3/6 → 6/12, a card of the
//! held Training Grounds set (R1420, R2194000).
//! Base:    "Cry: Deal 3 damage to all other Units."
//! Radiant: "Cry: Deal 6 damage to all enemy Units."
//! damage_all over other units (MP #21 Harmony Spirit excludeSelf); radiant enemy units only. Hits snapshotted before one state check (R59).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-017";

fn flames(side: ScopeSide, exclude_self: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![damage_all(json_as(json!({
                "amount": param(&*ctx, "damage"),
                "side": side,
                "excludeSelf": exclude_self,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: flames(ScopeSide::Any, true),
        radiant: flames(ScopeSide::Enemy, false),
    }
}

// TG #17 Flaming Felinor — SPEC §8.11 row 17, BUILD M13 row TG 17.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FELINOR: &str = super::ID;
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const BIG: &str = "traininggrounds-008"; // Clobberer 14/7
    const FRAIL: &str = "traininggrounds-001"; // Spikey Steven 5/2
    const FILLER: &str = "core-005";

    /// p1: a Mr. Vanilla in lane 1 and a Clobberer in lane 2; p2: a Clobberer in lane 1 and two Spikey
    /// Stevens in lanes 2 and 3.
    fn flames(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": FELINOR, "radiant": radiant }, FILLER], "field": [BODY, BIG] },
            "p2": { "hand": [FILLER], "field": [BIG, FRAIL, FRAIL] },
        }));
        s.play(FELINOR, json!({ "zone": 5 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn s6_3_3_damage_to_each_other_unit_on_both_sides() {
            let mut s = flames(false);
            let mine = s.unit_at(P1, 1);
            s.expect_stats(&mine, json!({ "damage": 3, "health": 1 }));
            let my_big = s.unit_at(P1, 2);
            s.expect_stats(&my_big, json!({ "damage": 3 }));
            let their_big = s.unit_at(P2, 1);
            s.expect_stats(&their_big, json!({ "damage": 3 }));
        }

        #[test]
        fn s6_3_not_to_itself() {
            let mut s = flames(false);
            s.expect_stats(FELINOR, json!({ "attack": 3, "health": 6, "damage": 0 }));
        }

        #[test]
        fn s6_3_no_hero_takes_damage() {
            let mut s = flames(false);
            s.expect_health(P1, HERO_HEALTH).expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r59_the_units_it_brings_to_0_die_together_at_one_state_check() {
            let s = flames(false);
            assert!(s.unit(P2, 2).is_none() && s.unit(P2, 3).is_none());
            let deaths: Vec<usize> = s
                .last_events()
                .iter()
                .enumerate()
                .filter(|(_, event)| event.event_type() == GameEventType::Destroyed)
                .map(|(at, _)| at)
                .collect();
            assert_eq!(deaths.len(), 2);
            // Every hit lands before the first death.
            let last_hit = s
                .last_events()
                .iter()
                .rposition(|event| event.event_type() == GameEventType::Damage)
                .expect("the hits");
            assert!(deaths.iter().all(|at| *at > last_hit));
        }

        #[test]
        fn r386_an_upgrade_deals_4_a_degrade_2() {
            for (step, damage) in [(1, 4), (-1, 2)] {
                let mut s = scenario(json!({
                    "p1": { "hand": [FELINOR, FILLER], "field": [BIG] },
                    "p2": { "hand": [FILLER], "field": [BIG] },
                }));
                step_param(s.card_mut(FELINOR), "damage", step);
                s.play(FELINOR, json!({}));
                let theirs = s.unit_at(P2, 1);
                s.expect_stats(&theirs, json!({ "damage": damage }));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s6_3_a_6_12_that_deals_6_to_each_enemy_unit() {
            let mut s = flames(true);
            s.expect_stats(FELINOR, json!({ "attack": 6, "health": 12, "damage": 0 }));
            let their_big = s.unit_at(P2, 1);
            s.expect_stats(&their_big, json!({ "damage": 6 }));
            assert!(s.unit(P2, 2).is_none() && s.unit(P2, 3).is_none());
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn s6_3_your_other_units_are_untouched() {
            let mut s = flames(true);
            let mine = s.unit_at(P1, 1);
            s.expect_stats(&mine, json!({ "damage": 0, "health": 4 }));
            let my_big = s.unit_at(P1, 2);
            s.expect_stats(&my_big, json!({ "damage": 0 }));
        }
    }
}
