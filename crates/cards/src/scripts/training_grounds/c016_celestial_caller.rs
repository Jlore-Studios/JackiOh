//! SPEC §8.11 Training Grounds 16 Celestial Caller: (4) Unit, Rare, 3/3 → 6/6, Divine Shield, Taunt,
//! Armor 3 (Radiant 6), a card of the held Training Grounds set (R1420, R2194000).
//! Base:    "Cry: Summon a Chosen by the Sun to the left of this and a Chosen by the Moon to the right of this."
//! Radiant: "Cry: Summon a Radiant Chosen by the Sun to the left of this and a Radiant Chosen by the Moon to the right of this."
//! M #49 aimed summon: TG #6 left, TG #7 right (mirrored R1060, R2196001). Edges make no summon (R129); occupied/reserved fail (R47, R64); Locked takes it (R688); no Cry (R1).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-016";

/// TG #6 Chosen by the Sun, summoned to the left.
const SUN: &str = "traininggrounds-006";
/// TG #7 Chosen by the Moon, summoned to the right.
const MOON: &str = "traininggrounds-007";

fn caller(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let Some(at) = ctx.live_self().and_then(|me| slot_of(&*ctx.state, me)) else {
                return vec![];
            };
            [(SUN, at.lane - 1), (MOON, at.lane + 1)]
                .into_iter()
                .filter(|(_, lane)| (1..=UNIT_ZONES).contains(lane))
                .map(|(def_id, lane)| {
                    jackioh_engine::effects::summon(json_as(json!({ "defId": def_id, "player": "self", "lane": lane, "radiant": radiant })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: caller(false),
        radiant: caller(true),
    }
}

// TG #16 Celestial Caller — SPEC §8.11 row 16, BUILD M13 row TG 16.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const CALLER: &str = super::ID;
    const SUN: &str = super::SUN;
    const MOON: &str = super::MOON;
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const FILLER: &str = "core-005";

    fn caller(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": CALLER, "radiant": radiant }, FILLER], "field": field },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// p1's unit row, lane by lane, as def ids.
    fn row(s: &Scenario) -> Vec<Option<String>> {
        (1..=UNIT_ZONES).map(|lane| s.unit(P1, lane).map(|card| card.def_id)).collect()
    }

    fn some(id: &str) -> Option<String> {
        Some(id.to_string())
    }

    fn keywords(s: &Scenario, card: &CardInstance) -> Vec<Keyword> {
        s.stats(card).keywords
    }

    mod base {
        use super::*;

        #[test]
        fn s6_1_it_prints_divine_shield_taunt_and_armor_3() {
            let mut s = caller(false, json!([]));
            s.play(CALLER, json!({ "zone": 3 }));
            let me = s.card(CALLER).clone();
            s.expect_stats(&me, json!({ "attack": 3, "health": 3 }));
            let keywords = keywords(&s, &me);
            assert!(keywords.iter().any(|keyword| matches!(keyword, Keyword::DivineShield)));
            assert!(keywords.iter().any(|keyword| matches!(keyword, Keyword::Taunt)));
            assert!(keywords.iter().any(|keyword| matches!(keyword, Keyword::Armor { n: 3 })));
        }

        #[test]
        fn r2196001_in_lane_3_the_sun_lands_in_lane_2_and_the_moon_in_lane_4() {
            let mut s = caller(false, json!([]));
            s.play(CALLER, json!({ "zone": 3 }));
            assert_eq!(row(&s), vec![None, some(SUN), some(CALLER), some(MOON), None]);
            let sun = s.unit_at(P1, 2);
            s.expect_stats(&sun, json!({ "attack": 3, "health": 3 }));
            assert!(keywords(&s, &sun).iter().any(|keyword| matches!(keyword, Keyword::DivineShield)));
            let moon = s.unit_at(P1, 4);
            s.expect_stats(&moon, json!({ "attack": 3, "health": 3 }));
            assert!(keywords(&s, &moon).iter().any(|keyword| matches!(keyword, Keyword::Armor { n: 3 })));
            assert!(!s.card(&sun).radiant && !s.card(&moon).radiant);
        }

        #[test]
        fn r2196001_in_lane_1_only_the_moon_lands_in_lane_2() {
            let mut s = caller(false, json!([]));
            s.play(CALLER, json!({ "zone": 1 }));
            assert_eq!(row(&s), vec![some(CALLER), some(MOON), None, None, None]);
        }

        #[test]
        fn r2196001_in_lane_5_only_the_sun_lands_in_lane_4() {
            let mut s = caller(false, json!([]));
            s.play(CALLER, json!({ "zone": 5 }));
            assert_eq!(row(&s), vec![None, None, None, some(SUN), some(CALLER)]);
        }

        #[test]
        fn r2196001_r47_an_occupied_lane_2_takes_no_sun_and_the_moon_still_lands() {
            let mut s = caller(false, json!([{ "def": BODY, "lane": 2 }]));
            s.play(CALLER, json!({ "zone": 3 }));
            assert_eq!(row(&s), vec![None, some(BODY), some(CALLER), some(MOON), None]);
            assert!(!def_ids_on_field(&s).contains(&SUN.to_string()));
        }

        #[test]
        fn r688_a_locked_neighbour_zone_takes_its_summon() {
            let mut s = caller(false, json!([]));
            lock_zone(s.state_mut(), ZoneRef { player: P1, row: Row::Units, lane: 4 });
            s.play(CALLER, json!({ "zone": 3 }));
            assert_eq!(row(&s), vec![None, some(SUN), some(CALLER), some(MOON), None]);
        }

        #[test]
        fn r1_summoned_rather_than_played_it_summons_nothing() {
            let mut s = caller(false, json!([]));
            crate::run_as(
                &mut s,
                jackioh_engine::effects::summon(json_as(json!({ "defId": CALLER, "player": "self", "lane": 3 }))),
                P1,
            );
            assert_eq!(row(&s), vec![None, None, some(CALLER), None, None]);
        }

        fn def_ids_on_field(s: &Scenario) -> Vec<String> {
            row(s).into_iter().flatten().collect()
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s6_1_a_6_6_with_armor_6() {
            let mut s = caller(true, json!([]));
            s.play(CALLER, json!({ "zone": 3 }));
            let me = s.card(CALLER).clone();
            s.expect_stats(&me, json!({ "attack": 6, "health": 6 }));
            assert!(keywords(&s, &me).iter().any(|keyword| matches!(keyword, Keyword::Armor { n: 6 })));
        }

        #[test]
        fn r2196001_it_summons_a_radiant_sun_9_9_and_a_radiant_moon_6_6_with_armor_6() {
            let mut s = caller(true, json!([]));
            s.play(CALLER, json!({ "zone": 3 }));
            let sun = s.unit_at(P1, 2);
            let moon = s.unit_at(P1, 4);
            assert_eq!((sun.def_id.as_str(), moon.def_id.as_str()), (SUN, MOON));
            assert!(s.card(&sun).radiant && s.card(&moon).radiant);
            s.expect_stats(&sun, json!({ "attack": 9, "health": 9 }));
            s.expect_stats(&moon, json!({ "attack": 6, "health": 6 }));
            assert!(keywords(&s, &moon).iter().any(|keyword| matches!(keyword, Keyword::Armor { n: 6 })));
        }
    }
}
