//! SPEC §8.11 Training Grounds 5 Midlane Menace: (2) Unit, Human, Common, 3/3 → 6/6, a card of the held
//! Training Grounds set (R1420, R2194000).
//! Base:    "Cry: If this is in midlane, it gains +5/+5 and Rush."
//! Radiant: "Cry: If this is in midlane, it gains +10/+10 and Rush."
//! Midlane test (C #22, R685; backrow counts, R1081000). Permanent buff and granted Rush on itself (§6.1; on play only, R1).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-005";

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return vec![];
    };
    let lane = slot_of(&*ctx.state, &self_).map_or(-1, |slot| slot.lane);
    let in_midlane = midlane_lanes_of(&*ctx.state, ctx.controller).contains(&lane)
        && in_units_row_or_standing(&*ctx.state, &self_);
    if !in_midlane {
        return vec![];
    }
    let amount = param(&*ctx, "buff");
    vec![
        buff(json_as(json!({ "target": { "of": "self" }, "attack": amount, "health": amount }))),
        grant_keyword(json_as(json!({ "target": { "of": "self" }, "keyword": { "kind": "Rush" } }))),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };
    // The same script: the Radiant face's 10 is its declared `buff`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #5 Midlane Menace — SPEC §8.11 row 5, BUILD M13 row TG 5 (R1).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = super::ID;
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const FILLER: &str = "core-005";

    fn menace(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": MENACE, "radiant": radiant }, FILLER, FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER, FILLER], "library": [FILLER, FILLER], "field": [{ "def": BODY, "lane": 1 }] },
        }))
    }

    fn played_into(radiant: bool, lane: i32) -> Scenario {
        let mut s = menace(radiant);
        s.play(MENACE, json!({ "zone": lane }));
        s
    }

    fn has_rush(s: &Scenario) -> bool {
        s.stats(MENACE).keywords.iter().any(|keyword| matches!(keyword, Keyword::Rush))
    }

    fn count(s: &Scenario, kind: GameEventType) -> usize {
        s.events().iter().filter(|event| event.event_type() == kind).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r685_played_into_lane_3_it_is_an_8_8_with_rush() {
            let mut s = played_into(false, 3);
            s.expect_stats(MENACE, json!({ "attack": 8, "health": 8, "maxHealth": 8 }));
            assert!(has_rush(&s));
            assert_eq!(count(&s, GameEventType::Buffed), 1);
            assert_eq!(count(&s, GameEventType::KeywordGranted), 1);
        }

        #[test]
        fn s6_1_the_granted_rush_attacks_an_enemy_unit_that_turn() {
            let mut s = played_into(false, 3);
            let body = s.unit_at(P2, 1);
            s.attack(MENACE, &body);
            s.expect_in_zone(&body, "graveyard");
        }

        #[test]
        fn s6_1_the_granted_rush_cannot_hit_the_hero_that_turn() {
            let mut s = played_into(false, 3);
            s.expect_refused(|s| s.attack(MENACE, "hero"));
        }

        #[test]
        fn r685_played_into_lanes_1_2_4_or_5_it_is_a_summoning_sick_3_3() {
            for lane in [1, 2, 4, 5] {
                let mut s = played_into(false, lane);
                s.expect_stats(MENACE, json!({ "attack": 3, "health": 3 }));
                assert!(!has_rush(&s), "lane {lane}");
                let body = s.unit_at(P2, 1);
                s.expect_refused(|s| s.attack(MENACE, &body));
            }
        }

        #[test]
        fn s10_4_the_buff_and_the_rush_stay_after_the_turn() {
            let mut s = played_into(false, 3);
            s.end_turn().end_turn();
            s.expect_stats(MENACE, json!({ "attack": 8, "health": 8 }));
            assert!(has_rush(&s));
            assert!(s.card(MENACE).granted_keywords.iter().any(|keyword| matches!(keyword, Keyword::Rush)));
        }

        #[test]
        fn r1_summoned_into_lane_3_rather_than_played_it_gains_nothing() {
            let mut s = menace(false);
            let events = crate::run_as(
                &mut s,
                jackioh_engine::effects::summon(json_as(json!({ "defId": MENACE, "player": "self", "lane": 3 }))),
                P1,
            );
            let summoned = s.unit_at(P1, 3);
            s.expect_stats(&summoned, json!({ "attack": 3, "health": 3 }));
            assert!(events.iter().all(|event| event.event_type() != GameEventType::Buffed));
            assert!(s.stats(&summoned).keywords.is_empty());
        }

        #[test]
        fn r1_a_copy_of_it_summoned_into_lane_3_gains_nothing() {
            // The source stands unbuffed in lane 1 (a copy keeps buffs and granted keywords, R57), so an
            // 8/8 or a Rush on the copy could only come from a Cry it must not have.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "library": [FILLER, FILLER],
                    "field": [{ "def": MENACE, "lane": 1 }],
                },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            let source = s.unit_at(P1, 1);
            let events = crate::run_as(
                &mut s,
                jackioh_engine::effects::summon_copy(json_as(json!({
                    "of": { "of": "instance", "instanceId": source.id },
                    "lane": 3,
                }))),
                P1,
            );
            let copy = s.unit_at(P1, 3);
            assert_eq!(copy.def_id, MENACE);
            assert_ne!(copy.id, source.id);
            s.expect_stats(&copy, json!({ "attack": 3, "health": 3 }));
            assert!(s.stats(&copy).keywords.is_empty());
            assert!(events.iter().all(|event| event.event_type() != GameEventType::Buffed));
            assert!(events.iter().all(|event| event.event_type() != GameEventType::KeywordGranted));
        }

        #[test]
        fn r386_an_upgrade_gives_6_6_a_degrade_4_4() {
            for (step, gain) in [(1, 6), (-1, 4)] {
                let mut s = menace(false);
                step_param(s.card_mut(MENACE), "buff", step);
                s.play(MENACE, json!({ "zone": 3 }));
                s.expect_stats(MENACE, json!({ "attack": 3 + gain, "health": 3 + gain }));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r685_in_lane_3_a_16_16_with_rush() {
            let mut s = played_into(true, 3);
            s.expect_stats(MENACE, json!({ "attack": 16, "health": 16 }));
            assert!(has_rush(&s));
        }

        #[test]
        fn r685_elsewhere_a_6_6_and_nothing() {
            for lane in [1, 2, 4, 5] {
                let mut s = played_into(true, lane);
                s.expect_stats(MENACE, json!({ "attack": 6, "health": 6 }));
                assert!(!has_rush(&s), "lane {lane}");
            }
        }
    }
}
