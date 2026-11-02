//! C+ #11 Anime Armor (SPEC §8.7 row 11). (2) Unit, Rare, 3/3 → 6/6 (Radiant: Reborn).
//! Aura: while it acts on the field its controller's hero and this Unit itself each take at most {cap}
//! from each damage instance (§4.4 step 3, E6's per-hit cap: the lowest cap wins beside Anti-oneshot
//! Armor's; on this Unit the cap is its guard's `guardsSelf`, R2179005). Lose health (R18) and Set
//! health are not damage and are not capped.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-011";

/// One script for both faces; `guards_self` is #2179's "and this".
fn anime_armor(guards_self: bool) -> Script {
    Script {
        hero_guard: Some(read_hook(move |args| {
            vec![HeroGuard {
                cap: Some(param(&args, "cap")),
                divisor: None,
                guards_self: guards_self.then_some(true),
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = anime_armor(true);
    // The Radiant face adds Reborn and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.5.2 the cap guarded its controller's hero alone; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_5_2() -> CardScripts {
    let base = anime_armor(false);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #11 Anime Armor — SPEC §8.7 row 11, BUILD M9 Classic+ row C+ 11.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const ANIME: &str = "classicplus-011";
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const LUNAR: &str = "core-035"; // (1) Deal 3 damage to a target.
    const TRUE_STRIKE: &str = "core-044"; // (1) Pierce. Deal 4 damage.
    const STOCKPILE: &str = "core-005"; // (1) Draw 2. Heal your hero 2.
    const JELLY_BEAN: &str = "core-027"; // Cast on draw: make a random hand card Radiant. Lose 7 health.
    const ANTI_ONESHOT: &str = "core-073"; // Field Spell: hits on your hero are capped at 5 (Radiant 3).
    const BIG_MAX: &str = "classic-080"; // 13/8 Rush, Trample, Indestructible.
    const SNAKE: &str = "classicplus-003"; // Death: a hit of {damage} on a random enemy per Plague Counter.
    const VITAL_KILL: &str = "classic-029"; // (1) Set a hero's health to 13.
    const HIT_JOB: &str = "core-016"; // (3) Destroy target Unit.
    const SILENCE: &str = "classicplus-009"; // (0) Vanilla a Unit.
    const FILLER: &str = "core-010";

    fn hero_hits(s: &Scenario, player: PlayerId) -> Vec<i32> {
        let hero = format!("hero-{player}");
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if *target_id == hero => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn hero_p1() -> Value {
        json!([{ "pick": "hero", "player": "p1" }])
    }

    use crate::merged;

    /// p2 active, attacking or casting into p1, who holds Anime Armor.
    fn under_fire(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "active": "p2",
            "p1": merged(
                json!({
                    "hand": [FILLER],
                    "library": [STOCKPILE, STOCKPILE],
                    "field": [{ "def": ANIME, "radiant": radiant_face }],
                    "health": 20,
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE], "mana": 10 }), p2),
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn s4_4_step_3_a_combat_hit_on_your_hero_takes_1() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "field": [MENACE] }), false);

            s.attack(MENACE, "hero");

            assert_eq!(hero_hits(&s, P1), vec![1]);
            s.expect_health(P1, 19);
        }

        #[test]
        fn a_spell_s_hit_takes_1_and_so_does_a_pierce_hit() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "hand": [LUNAR, TRUE_STRIKE, FILLER] }), false);

            s.play(LUNAR, json!({ "targets": hero_p1() }));
            s.play(TRUE_STRIKE, json!({ "targets": hero_p1() }));

            assert_eq!(hero_hits(&s, P1), vec![1, 1]);
            s.expect_health(P1, 18);
        }

        #[test]
        fn s2_4_r125_each_fatigue_hit_is_its_own_capped_instance() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [STOCKPILE, FILLER], "library": [], "field": [ANIME], "health": 20 },
                "p2": { "hand": [FILLER] },
            }));

            s.play(STOCKPILE, json!({})); // draws 2 from an empty deck: fatigue 1, then 2; then heals 2

            assert_eq!(hero_hits(&s, P1), vec![1, 1]);
            s.expect_health(P1, 20);
        }

        #[test]
        fn s4_4_step_9_a_trample_excess_is_its_own_instance_capped_at_1() {
            crate::register_all();
            let mut s = under_fire(
                json!({ "field": [{ "def": ANIME, "lane": 1 }, { "def": VANILLA, "lane": 2 }] }),
                json!({ "field": [BIG_MAX] }),
                false,
            );

            let target = s.unit(P1, 2).unwrap();
            s.attack(BIG_MAX, &target);

            assert_eq!(hero_hits(&s, P1), vec![1]);
            s.expect_health(P1, 19);
        }

        #[test]
        fn e37_each_hit_of_a_split_is_its_own_capped_instance() {
            crate::register_all();
            // p2's Second Amendment Snake, tuned to hits of 2, dies to p1's Hit Job and splits its hits over p1.
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, FILLER], "field": [ANIME], "health": 20, "mana": 8 },
                "p2": { "hand": [FILLER], "field": [{ "def": SNAKE, "counters": { "plague": 6 } }] },
            }));
            step_param(s.card_mut(SNAKE), "damage", 1);

            let snake = s.card(SNAKE).id.clone();
            s.play(HIT_JOB, json!({ "targets": pick::id(&snake) }));

            let on_hero = hero_hits(&s, P1);
            assert!(!on_hero.is_empty());
            assert!(on_hero.iter().all(|&amount| amount == 1));
            s.expect_health(P1, 20 - on_hero.len() as i32);
        }

        #[test]
        fn e6_the_lowest_cap_wins_beside_anti_oneshot_armor_s_4_or_its_radiant_3() {
            crate::register_all();
            let mut s = under_fire(json!({ "backrow": [ANTI_ONESHOT] }), json!({ "field": [MENACE] }), false);
            s.attack(MENACE, "hero");
            assert_eq!(hero_hits(&s, P1), vec![1]);

            let mut loose = under_fire(json!({ "backrow": [ANTI_ONESHOT] }), json!({ "field": [MENACE] }), false);
            step_param(loose.card_mut(ANIME), "cap", 6); // Degraded six times: cap 7, so Anti-oneshot's 4 is lower
            loose.attack(MENACE, "hero");
            assert_eq!(hero_hits(&loose, P1), vec![4]);

            let mut tight = under_fire(
                json!({ "backrow": [{ "def": ANTI_ONESHOT, "radiant": true }] }),
                json!({ "field": [MENACE] }),
                false,
            );
            step_param(tight.card_mut(ANIME), "cap", 6);
            tight.attack(MENACE, "hero");
            assert_eq!(hero_hits(&tight, P1), vec![3]);
        }

        #[test]
        fn r18_lose_health_is_not_damage_and_is_not_capped() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [STOCKPILE, FILLER],
                    "library": [JELLY_BEAN, FILLER, FILLER],
                    "field": [ANIME],
                    "health": 20,
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(STOCKPILE, json!({})); // draws the Jelly Bean, which casts itself: lose 7; then heal 2

            s.expect_health(P1, 15);
            assert_eq!(hero_hits(&s, P1), Vec::<i32>::new());
        }

        #[test]
        fn e7_set_health_is_not_damage_and_is_not_capped() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "hand": [VITAL_KILL, FILLER] }), false);

            s.play(VITAL_KILL, json!({ "targets": hero_p1() }));

            s.expect_health(P1, 13);
        }

        #[test]
        fn the_opponent_s_hero_is_unaffected() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [ANIME, MENACE] },
                "p2": { "hand": [FILLER] },
            }));

            s.attack(MENACE, "hero");

            s.expect_health(P2, 21);
        }

        #[test]
        fn the_cap_lifts_when_it_leaves_the_field() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "hand": [HIT_JOB, LUNAR, FILLER] }), false);

            let anime = s.card(ANIME).id.clone();
            s.play(HIT_JOB, json!({ "targets": pick::id(&anime) }));
            s.play(LUNAR, json!({ "targets": hero_p1() }));

            s.expect_in_zone(ANIME, "graveyard");
            assert_eq!(hero_hits(&s, P1), vec![3]);
        }

        #[test]
        fn s6_3_a_vanilla_anime_armor_caps_nothing_its_text_is_gone() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "hand": [SILENCE, LUNAR, FILLER] }), false);

            let anime = s.card(ANIME).id.clone();
            s.play(SILENCE, json!({ "targets": pick::id(&anime) }));
            s.play(LUNAR, json!({ "targets": hero_p1() }));

            assert_eq!(hero_hits(&s, P1), vec![3]);
        }

        fn unit_hits(s: &Scenario, id: &str) -> Vec<i32> {
            s.events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Damage { target_id, amount, .. } if target_id == id => Some(*amount),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn s8_7_it_is_3_3() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({}), false);
            let anime = s.card(ANIME).clone();
            s.expect_stats(&anime, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r2179005_a_spell_s_hit_on_this_takes_1_a_pierce_hit_too() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "hand": [LUNAR, TRUE_STRIKE, FILLER] }), false);
            let anime = s.card(ANIME).id.clone();

            s.play(LUNAR, json!({ "targets": pick::id(&anime) }));
            s.play(TRUE_STRIKE, json!({ "targets": pick::id(&anime) }));

            assert_eq!(unit_hits(&s, &anime), vec![1, 1]);
            s.expect_stats(&anime, json!({ "health": 1, "maxHealth": 3 }));
            s.expect_in_zone(ANIME, "field");
        }

        #[test]
        fn r2179005_a_combat_hit_on_this_takes_1_and_a_trample_has_no_excess_past_it() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "field": [MENACE, BIG_MAX] }), false);
            let anime = s.card(ANIME).id.clone();

            s.attack(MENACE, &anime);
            assert_eq!(unit_hits(&s, &anime), vec![1]);
            // Big Max's 13 meets the cap at step 3, before step 5 splits off an excess: none reaches the hero.
            s.attack(BIG_MAX, &anime);
            assert_eq!(unit_hits(&s, &anime), vec![1, 1]);
            assert_eq!(hero_hits(&s, P1), Vec::<i32>::new());
            s.expect_stats(&anime, json!({ "health": 1 }));
        }

        #[test]
        fn r2179005_another_unit_of_yours_is_not_capped() {
            crate::register_all();
            let mut s = under_fire(
                json!({ "field": [{ "def": ANIME, "lane": 1 }, { "def": VANILLA, "lane": 2 }] }),
                json!({ "hand": [LUNAR, FILLER] }),
                false,
            );
            let vanilla = s.unit(P1, 2).expect("the Vanilla").id;
            s.play(LUNAR, json!({ "targets": pick::id(&vanilla) }));
            assert_eq!(unit_hits(&s, &vanilla), vec![3]);
        }

        /// R767000: the script kept for the replays of v0.5.2 and before, whose cap guarded the hero alone.
        #[test]
        fn r767000_a_game_on_v0_5_2_caps_the_hero_and_not_the_unit() {
            crate::under_kept(ANIME, crate::scripts::classic_plus::c011_anime_armor::script_v0_5_2(), || {
                let mut s = under_fire(json!({}), json!({ "hand": [LUNAR, LUNAR, FILLER] }), false);
                let anime = s.card(ANIME).id.clone();
                s.play(LUNAR, json!({ "targets": hero_p1() }));
                assert_eq!(hero_hits(&s, P1), vec![1]);
                // Uncapped, the 3 lands whole (and kills the 3/3).
                s.play(LUNAR, json!({ "targets": pick::id(&anime) }));
                assert_eq!(unit_hits(&s, &anime), vec![3]);
            });
        }

        #[test]
        fn r386_the_cap_reads_through_param_a_degrade_lets_2_through_and_an_upgrade_never_takes_it_below_1() {
            crate::register_all();
            let mut degraded = under_fire(json!({}), json!({ "hand": [LUNAR, FILLER] }), false);
            step_param(degraded.card_mut(ANIME), "cap", 1);
            degraded.play(LUNAR, json!({ "targets": hero_p1() }));
            assert_eq!(hero_hits(&degraded, P1), vec![2]);

            let mut upgraded = under_fire(json!({}), json!({ "hand": [LUNAR, FILLER] }), false);
            step_param(upgraded.card_mut(ANIME), "cap", -1);
            upgraded.play(LUNAR, json!({ "targets": hero_p1() }));
            assert_eq!(hero_hits(&upgraded, P1), vec![1]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2179005_radiant_a_hit_on_this_takes_1() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "hand": [LUNAR, FILLER] }), true);
            let anime = s.card(ANIME).id.clone();
            s.play(LUNAR, json!({ "targets": pick::id(&anime) }));
            s.expect_stats(&anime, json!({ "health": 5, "maxHealth": 6 }));
        }

        #[test]
        fn is_6_6_with_reborn_and_the_cap_comes_back_with_the_reborn_body() {
            crate::register_all();
            let mut s = under_fire(json!({}), json!({ "hand": [HIT_JOB, LUNAR, FILLER] }), true);
            let anime = s.card(ANIME).clone();
            s.expect_stats(&anime, json!({ "attack": 6, "health": 6 }));

            s.play(HIT_JOB, json!({ "targets": pick::id(&anime.id) }));
            let back = s.unit(P1, 1).expect("the Reborn body");
            assert_eq!(back.def_id, ANIME);
            s.expect_stats(&back, json!({ "health": 1 }));

            s.play(LUNAR, json!({ "targets": hero_p1() }));
            assert_eq!(hero_hits(&s, P1), vec![1]);
        }
    }
}
