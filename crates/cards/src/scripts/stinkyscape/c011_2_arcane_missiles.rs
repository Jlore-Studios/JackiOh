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

fn arcane_missiles(echo: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags { echo: Some(echo), ..StaticFlags::default() }),
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            // The state and the rng are two fields of the sink: the pool reads the one while the
            // draw rolls from the other (C+ #25 Soul Shot's shape).
            let enemy = opponent_of(ctx.controller);
            let sink = &mut ctx.sink;
            let state: &GameState = &*sink.state;
            let rng: &mut Rng = &mut *sink.rng;
            let mut pool: Vec<Value> = vec![json!({ "of": "enemyHero" })];
            pool.extend(active_units_of(state, enemy).iter().map(|unit| {
                json!({ "of": "instance", "instanceId": unit.id.clone() })
            }));
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
        base: arcane_missiles(2),
        radiant: arcane_missiles(5),
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
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2.
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
            crate::register_all();
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
            crate::register_all();
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
            crate::register_all();
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
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [MISSILES, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(MISSILES), "damage", 1);

            s.play(MISSILES, json!({}));

            s.expect_health(P2, HERO_HEALTH - 6);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn six_hits_of_1_each_drawn_anew() {
            crate::register_all();
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
