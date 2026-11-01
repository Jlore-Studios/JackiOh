//! S #19 Imprisoned Cosmality (SPEC §8.10 row 19): (2) Unit, Common, 10/6 → 20/12, of the
//! Stinkyscape set.
//!   Base:    "Pierce / Dormant for 2 turns. When this awakens, deal {damage} damage randomly split
//!            among all enemies." ({damage} 10)
//!   Radiant: the same text, {damage} 20.
//! Engine: Pierce (R346) is catalog data. Dormant (R1839000), `StaticFlags.slumber { turns: 2 }`,
//! awake at the start of its controller's second turn after it arrived, summoning sick that turn
//! (R1839001, R2259000). `when_this_awakens` (R1839002): C+ #3's `damage_split`, {damage} hits of 1, each on an
//! enemy drawn from the match rng among the enemy hero and the enemy Units still standing (R60), with
//! no state check between the hits (R59); each hit's source is this Unit, so each pierces Armor
//! (R346). Both faces run this script; the number is the face's declared `damage` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-019";

/// "Dormant for 2 turns": the status's printed count, no param (docs/stinkyscape-set.md, #19).
const DORMANT_TURNS: i32 = 2;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            ..StaticFlags::default()
        }),
        triggers: vec![when_this_awakens("awakens", |ctx, _event| {
            let damage = param(&*ctx, "damage");
            vec![damage_split(json_as(json!({ "amount": damage, "among": "enemies" })))]
        })],
        ..Script::default()
    };
    // The Radiant face differs only in its stats and its `damage` value (20), both catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #19 Imprisoned Cosmality — SPEC §8.10 row 19, BUILD M12 row S 19.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{DORMANT_TURNS, ID};
    use crate::merged;

    const FILLER: &str = "core-010"; // (0) Spell: a play always open, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1.
    const ENEMY_HERO: &str = "hero-p2";

    fn played(radiant: bool, p2: Value, seed: &str) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(4) },
            "p2": merged(json!({ "hand": [FILLER], "library": crate::filler(4) }), p2),
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn to_its_awakening(s: &mut Scenario) {
        s.end_turn().end_turn().end_turn().end_turn();
        assert_eq!(s.state().active, P1);
    }

    fn dormant_for(turns: i32) -> Option<Slumber> {
        Some(Slumber { turns: Some(turns) })
    }

    struct Hit {
        index: usize,
        target_id: String,
        amount: i32,
    }

    fn hits(s: &Scenario) -> Vec<Hit> {
        let id = s.card(ID).id.clone();
        s.events()
            .iter()
            .enumerate()
            .filter_map(|(index, event)| match event {
                GameEvent::Damage { source_id: Some(source), target_id, amount, .. } if *source == id => {
                    Some(Hit { index, target_id: target_id.clone(), amount: *amount })
                }
                _ => None,
            })
            .collect()
    }

    fn hits_of_the_awake_one(radiant: bool) -> usize {
        let mut s = crate::scenario(json!({
            "seed": "cosmality-beside",
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        let awake = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "zone": 2 }));
        to_its_awakening(&mut s);
        assert!(s.events().iter().any(|event| matches!(event, GameEvent::Awakened { .. })));
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { source_id: Some(source), .. } if *source == awake))
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r2271000_another_cards_awakening_deals_nothing() {
            // "When THIS awakens": the awake Cosmality answered every awakening with its 10 hits too,
            // until its condition was read (R2271000).
            assert_eq!(hits_of_the_awake_one(false), 0);
        }

        #[test]
        fn r1839001_played_it_lies_dormant_and_awakens_at_the_start_of_its_controllers_second_turn_after() {
            let mut s = played(false, json!({}), "cosmality");
            let cosmality = s.card(ID).id.clone();
            assert_eq!(s.card(&cosmality).slumber, dormant_for(DORMANT_TURNS));
            s.expect_refused(|s| s.attack(&cosmality, "hero"));

            s.end_turn().end_turn();
            assert_eq!(s.card(&cosmality).slumber, dormant_for(DORMANT_TURNS - 1));
            assert!(hits(&s).is_empty());

            s.end_turn().end_turn();
            assert_eq!(s.card(&cosmality).slumber, None);
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Awakened { .. })));
        }

        #[test]
        fn r60_when_it_awakens_10_hits_of_1_each_on_the_enemy_hero_or_an_enemy_unit() {
            let mut s = played(false, json!({ "field": [VANILLA, VANILLA] }), "cosmality");
            let vanillas: Vec<String> = [1, 2].iter().map(|&lane| s.unit_at(P2, lane).id).collect();
            to_its_awakening(&mut s);

            let hits = hits(&s);
            assert_eq!(hits.len(), 10);
            assert!(hits.iter().all(|hit| hit.amount == 1));
            assert!(hits.iter().all(|hit| hit.target_id == ENEMY_HERO || vanillas.contains(&hit.target_id)));
            let on_hero = hits.iter().filter(|hit| hit.target_id == ENEMY_HERO).count() as i32;
            s.expect_health(P2, HERO_HEALTH - on_hero);
            s.expect_health(P1, HERO_HEALTH);
        }

        #[test]
        fn r59_a_unit_an_earlier_hit_brought_to_0_is_not_hit_again_and_dies_after_the_last_hit() {
            let mut deaths = 0;
            for seed in 1..=8 {
                let mut s = played(false, json!({ "field": [MR_TOKEN] }), &format!("cosmality-{seed}"));
                let token = s.unit_at(P2, 1).id;
                to_its_awakening(&mut s);

                let hits = hits(&s);
                assert_eq!(hits.len(), 10);
                assert!(hits.iter().filter(|hit| hit.target_id == token).count() <= 1);
                let died = s.events().iter().position(|event| {
                    matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == token)
                });
                if let Some(died) = died {
                    deaths += 1;
                    assert!(died > hits[9].index);
                }
            }
            assert!(deaths > 0);
        }

        #[test]
        fn r346_each_hit_pierces_the_enemy_heros_armor() {
            let mut s = played(false, json!({ "armor": 20 }), "cosmality");
            to_its_awakening(&mut s);

            assert_eq!(hits(&s).len(), 10);
            s.expect_health(P2, HERO_HEALTH - 10);
            assert_eq!(s.state().players.p2.hero.armor, 20);
        }

        #[test]
        fn r1839001_r2259000_awake_it_is_summoning_sick_that_turn_and_attacks_the_next() {
            let mut s = played(false, json!({}), "cosmality");
            to_its_awakening(&mut s);
            s.expect_health(P2, HERO_HEALTH - 10);

            s.expect_refused(|s| s.attack(ID, "hero"));
            s.end_turn().end_turn();
            s.attack(ID, "hero");
            s.expect_health(P2, HERO_HEALTH - 10 - 10);
        }

        #[test]
        fn r386_its_damage_reads_through_param_an_upgrade_adds_a_hit() {
            let mut s = played(false, json!({}), "cosmality");
            assert_eq!(crate::upgrade_number(&mut s, ID, "damage"), 11);
            to_its_awakening(&mut s);

            assert_eq!(hits(&s).len(), 11);
            s.expect_health(P2, HERO_HEALTH - 11);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2271000_another_cards_awakening_deals_nothing_on_the_radiant_face() {
            assert_eq!(hits_of_the_awake_one(true), 0);
        }

        #[test]
        fn r275_a_20_12_whose_awakening_deals_20_hits_of_1() {
            let mut s = played(true, json!({}), "cosmality");
            s.expect_stats(ID, json!({ "attack": 20, "health": 12 }));
            to_its_awakening(&mut s);

            let hits = hits(&s);
            assert_eq!(hits.len(), 20);
            assert!(hits.iter().all(|hit| hit.amount == 1 && hit.target_id == ENEMY_HERO));
            s.expect_health(P2, HERO_HEALTH - 20);
        }
    }
}
