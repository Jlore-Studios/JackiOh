//! SPEC §8.11 Training Grounds 33 Guerilla Felinor: (3) Unit, Felinor, Epic, 4/6 → 8/12, a card of the
//! held Training Grounds set (R1420, R2194000).
//!
//! Base:    "Cry: Deal 7 damage randomly split among enemy Units."
//! Radiant: "Pierce / Cry: Deal 14 damage randomly split among enemy Units."
//! Engine: `damage_split`: {damage} hits of 1, each drawn from the match rng among the enemy Units still
//! standing as it lands (R60), no state check between hits (R59), no hero; with none standing the rest
//! are lost. Each hit's source is this Unit, so on the Radiant face each pierces Armor (R346), as S #19
//! Imprisoned Cosmality's do. The designer's "repeat 7 times" is 7 hits in all.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-033";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![damage_split(json_as(json!({ "amount": param(&*ctx, "damage"), "among": "enemyUnits" })))]
        })),
        ..Script::default()
    };
    // The same script: Pierce and the 14 are the Radiant face's catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #33 Guerilla Felinor — SPEC §8.11 row 33, BUILD M13 row TG 33 (R129).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GUERILLA: &str = super::ID;
    const BIG: &str = "traininggrounds-008"; // Clobberer 14/7
    const FRAIL: &str = "traininggrounds-001"; // Spikey Steven 5/2
    const SUN: &str = "traininggrounds-006"; // Chosen by the Sun 3/3, Divine Shield
    const MOON: &str = "traininggrounds-007"; // Chosen by the Moon 3/3, Taunt, Armor 3
    const FILLER: &str = "core-005";

    fn guerilla(radiant: bool, theirs: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": GUERILLA, "radiant": radiant }, FILLER], "field": [BIG] },
            "p2": { "hand": [FILLER], "field": theirs },
        }));
        s.play(GUERILLA, json!({ "zone": 3 }));
        s
    }

    /// The Cry's hits: (target, amount) for each `damage` event this Unit dealt.
    fn hits(s: &Scenario) -> Vec<(String, i32)> {
        let me = s.card(GUERILLA).id.clone();
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { source_id: Some(source), target_id, amount, .. } if *source == me => {
                    Some((target_id.clone(), *amount))
                }
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r60_7_hits_of_1_each_on_an_enemy_unit() {
            let s = guerilla(false, json!([BIG, BIG]));
            let hits = hits(&s);
            assert_eq!(hits.len(), 7);
            assert!(hits.iter().all(|(_, amount)| *amount == 1));
            let theirs: Vec<String> = [1, 2].map(|lane| s.unit_at(P2, lane).id).to_vec();
            assert!(hits.iter().all(|(target, _)| theirs.contains(target)));
            let mine = s.unit_at(P1, 1);
            assert_eq!(s.card(&mine).damage, 0);
            let total: i32 = [1, 2].map(|lane| s.unit_at(P2, lane).damage).iter().sum();
            assert_eq!(total, 7);
        }

        #[test]
        fn r60_a_unit_killed_by_an_earlier_hit_is_never_drawn_again() {
            let mut s = scenario(json!({
                "p1": { "hand": [GUERILLA, FILLER] },
                "p2": { "hand": [FILLER], "field": [FRAIL, BIG] },
            }));
            let frail = s.unit_at(P2, 1).id;
            s.play(GUERILLA, json!({}));
            let hits = hits(&s);
            assert_eq!(hits.len(), 7);
            let on_frail = hits.iter().filter(|(target, _)| *target == frail).count();
            assert!(on_frail <= 2, "{on_frail} hits on a 2-Health Unit");
            let big = s.unit_at(P2, 2);
            assert_eq!(big.damage as usize, 7 - on_frail);
        }

        #[test]
        fn s6_3_the_enemy_hero_is_never_hit() {
            let mut s = guerilla(false, json!([FRAIL]));
            s.expect_health(P2, HERO_HEALTH);
            assert!(hits(&s).iter().all(|(target, _)| !target.starts_with("hero")));
        }

        #[test]
        fn r129_no_enemy_unit_nothing_and_no_random_number_drawn() {
            let mut s = scenario(json!({
                "p1": { "hand": [GUERILLA, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let cursor = s.state().rng_cursor;
            s.play(GUERILLA, json!({}));
            assert!(hits(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn s4_4_a_divine_shield_eats_one_hit() {
            let s = guerilla(false, json!([SUN]));
            // The shield takes the first, three more kill the 3/3, and the last three are lost.
            assert!(s.unit(P2, 1).is_none());
            let landed: Vec<i32> = hits(&s).iter().map(|(_, amount)| *amount).collect();
            assert_eq!(landed.iter().sum::<i32>(), 3);
        }

        #[test]
        fn r386_an_upgrade_deals_9_a_degrade_5() {
            for (step, total) in [(1, 9), (-1, 5)] {
                let mut s = scenario(json!({
                    "p1": { "hand": [GUERILLA, FILLER] },
                    "p2": { "hand": [FILLER], "field": [BIG, BIG] },
                }));
                step_param(s.card_mut(GUERILLA), "damage", step);
                s.play(GUERILLA, json!({}));
                assert_eq!(hits(&s).len(), total as usize, "step {step}");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r60_14_hits() {
            let mut s = guerilla(true, json!([BIG, BIG]));
            s.expect_stats(GUERILLA, json!({ "attack": 8, "health": 12 }));
            assert_eq!(hits(&s).len(), 14);
        }

        #[test]
        fn r346_pierce_on_the_unit_and_on_the_cry_s_hits() {
            let s = guerilla(true, json!([MOON]));
            assert!(s.stats(GUERILLA).keywords.iter().any(|keyword| matches!(keyword, Keyword::Pierce)));
            // Armor 3 would turn every hit of 1 to nothing; each pierces, so three kill the Moon.
            assert!(s.unit(P2, 1).is_none());
        }
    }
}
