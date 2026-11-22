//! S #11.2 Arcane Missiles (SPEC §8.10 row S 11.2; §6.2, R60): (0) Spell, Other-Origins,
//! Token (printed Epic).
//!   Base:    "Echo 2 / Deal {damage} damage to a random enemy." — 1.
//!   Radiant: "Echo 5 / Deal {damage} damage to a random enemy." — 1.
//!
//! Engine: §6.2 Echo X — the Cry resolves once, then re-resolves twice (five times on the Radiant
//! face), each resolution its own run. Each run draws one enemy from the match rng (R60) among the
//! enemy hero and the acting enemy Units still standing, a Unit the last hit killed never drawn
//! again, and deals it one hit of the declared damage (§4.4: Spell Damage raises each hit). With no
//! enemy Unit the hero takes every hit.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-011-2";

fn arcane_missiles(echo: i32, slash_aware: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags { echo: Some(echo), ..StaticFlags::default() }),
        cry: Some(hook(move |ctx| {
            let amount = param(&*ctx, "damage");
            // The state and the rng are two fields of the sink: the pool reads the one while the
            // draw rolls from the other (C+ #25 Soul Shot's shape).
            let enemy = opponent_of(ctx.controller);
            let source = ctx.self_.as_ref();
            let sink = &mut ctx.sink;
            let state: &GameState = &*sink.state;
            let rng: &mut Rng = &mut *sink.rng;
            let mut pool: Vec<Value> = vec![json!({ "of": "enemyHero" })];
            pool.extend(
                active_units_of(state, enemy)
                    .iter()
                    .filter(|unit| !slash_aware || reaches(state, source, unit))
                    .map(|unit| json!({ "of": "instance", "instanceId": unit.id.clone() })),
            );
            let Some(picked) = rng.pick(&pool) else {
                return vec![];
            };
            vec![damage(json_as(json!({ "to": picked, "amount": amount })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: arcane_missiles(2, true),
        radiant: arcane_missiles(5, true),
    }
}

/// v0.6.9i and before: each missile drew among every enemy Unit, across a slash too (R3554003);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_9i() -> CardScripts {
    CardScripts {
        base: arcane_missiles(2, false),
        radiant: arcane_missiles(5, false),
    }
}

// S #11.2 Arcane Missiles — SPEC §8.10 row S 11.2, BUILD M12 row S 11.2.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MISSILES: &str = "stinkyscape-011-2";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const SOLARIUS: &str = "classicplus-038"; // Solarius: Spell Damage +2
    const FILLER: &str = "core-005";

    fn hits(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, .. } => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        fn board(radiant: bool, seed: &str, field: Value) -> Scenario {
            scenario(json!({
                "seed": seed,
                "p1": { "hand": [{ "def": MISSILES, "radiant": radiant }, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER], "field": field },
            }))
        }

        #[test]
        fn three_hits_of_1_each_on_a_random_enemy_drawn_as_it_resolves() {
            let mut heroes = 0;
            let mut units = 0;
            for seed in 1..=16 {
                let mut s = board(
                    false,
                    &format!("missiles-{seed}"),
                    json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }]),
                );
                let first = s.unit_at(P2, 1).id.clone();
                let second = s.unit_at(P2, 2).id.clone();
                let mine = s.unit_at(P1, 1).id.clone();
                s.play(MISSILES, json!({}));

                let hits = hits(&s);
                assert_eq!(hits.len(), 3, "seed missiles-{seed}");
                for target in &hits {
                    assert_ne!(target, &mine, "never your own side");
                    assert!(
                        target == "hero-p2" || target == &first || target == &second,
                        "an enemy: {target}"
                    );
                    if target == "hero-p2" {
                        heroes += 1;
                    } else {
                        units += 1;
                    }
                }
            }
            assert!(heroes > 0 && units > 0, "heroes {heroes}, units {units}");
        }

        #[test]
        fn a_unit_killed_by_one_hit_is_never_drawn_by_the_next() {
            for seed in 1..=16 {
                let mut s = board(
                    false,
                    &format!("missiles-kill-{seed}"),
                    json!([{ "def": VANILLA, "lane": 1, "damage": 3 }, { "def": VANILLA, "lane": 2 }]),
                );
                let weak = s.unit_at(P2, 1).id.clone();
                s.play(MISSILES, json!({}));

                let on_weak = hits(&s).into_iter().filter(|target| target == &weak).count();
                assert!(on_weak <= 1, "seed missiles-kill-{seed}");
            }
        }

        #[test]
        fn with_no_enemy_unit_every_hit_lands_on_the_enemy_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [MISSILES, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(MISSILES, json!({}));

            assert_eq!(hits(&s), vec!["hero-p2", "hero-p2", "hero-p2"]);
            s.expect_health(P2, HERO_HEALTH - 3);
        }

        #[test]
        fn spell_damage_raises_each_hit() {
            let mut s = scenario(json!({
                "p1": { "hand": [MISSILES, FILLER], "field": [SOLARIUS] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(MISSILES, json!({}));

            assert_eq!(hits(&s), vec!["hero-p2", "hero-p2", "hero-p2"]);
            s.expect_health(P2, HERO_HEALTH - 9);
        }

        #[test]
        fn r386_an_upgrade_makes_each_hit_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [MISSILES, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(MISSILES), "damage", 1);

            s.play(MISSILES, json!({}));

            s.expect_health(P2, HERO_HEALTH - 6);
        }
    }

    /// R3554003, R3554004: a Spell cast on a slashed board resolves on the half its cast chose, so a
    /// missile is only ever aimed at the enemy hero or a Unit on that half.
    mod slashed {
        use super::*;

        fn cast_on_the_left(seed: &str, field: Value) -> Scenario {
            let mut s = scenario(json!({
                "seed": seed,
                "p1": { "hand": [MISSILES, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER], "field": field },
            }));
            s.state_mut().slashed_lane = Some(3);
            s.play(MISSILES, json!({}));
            s.answer(json!("left"));
            s
        }

        #[test]
        fn r3554003_no_missile_is_aimed_across_the_slash() {
            let mut heroes = 0;
            let mut units = 0;
            for seed in 1..=16 {
                let s = cast_on_the_left(
                    &format!("missiles-slashed-{seed}"),
                    json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 5 }]),
                );
                let near = s.unit_at(P2, 1).id.clone();

                let hits = hits(&s);
                assert_eq!(hits.len(), 3, "seed {seed}: every missile lands");
                for target in &hits {
                    assert!(target == "hero-p2" || target == &near, "seed {seed}: {target} is across the slash");
                    if target == "hero-p2" {
                        heroes += 1;
                    } else {
                        units += 1;
                    }
                }
            }
            assert!(heroes > 0 && units > 0, "heroes {heroes}, units {units}");
        }

        #[test]
        fn r3554003_with_enemy_units_only_across_the_slash_every_missile_hits_the_hero() {
            for seed in 1..=16 {
                let s = cast_on_the_left(
                    &format!("missiles-far-{seed}"),
                    json!([{ "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 5 }]),
                );

                assert_eq!(hits(&s), vec!["hero-p2"; 3], "seed {seed}");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn six_hits_of_1_each_drawn_anew() {
            let mut s = scenario(json!({
                "seed": "missiles-radiant",
                "p1": { "hand": [{ "def": MISSILES, "radiant": true }, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "lane": 1 }] },
            }));
            let theirs = s.unit_at(P2, 1).id.clone();
            s.play(MISSILES, json!({}));

            let hits = hits(&s);
            assert_eq!(hits.len(), 6);
            assert!(hits.iter().all(|target| target == "hero-p2" || target == &theirs));
        }
    }
}
