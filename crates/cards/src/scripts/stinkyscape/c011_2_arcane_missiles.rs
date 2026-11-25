//! S #11.2 Arcane Missiles (SPEC §8.10 row S 11.2; §6.2, R60): (0) Spell, Other-Origins,
//! Token (printed Epic).
//!   Base:    "Echo 2 / Deal {damage} damage to a random enemy." — 1.
//!   Radiant: "Echo 5 / Deal {damage} damage to a random enemy." — 1.
//!
//! Engine: §6.2 Echo X — the Cry resolves once, then re-resolves twice (five times on the Radiant
//! face), each resolution its own run. Each run draws one enemy from the match rng (R60) among the
//! enemy hero and the enemy Units a Spell may affect (C+ #25 Soul Shot's pool, `cards_in_scope`):
//! a Unit immune to Spells is never drawn, and a Dormant card never is (R4424000); a Unit the last
//! hit killed is never drawn again, and each draw deals its hit of the declared damage (§4.4: Spell
//! Damage raises each hit). With no enemy Unit the hero takes every hit.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-011-2";

/// The enemy Units a missile may be drawn onto, which each shipped version of the card read differently.
#[derive(Clone, Copy)]
enum Pool {
    /// R4424000: the enemy Units a Spell may affect (C+ #25 Soul Shot's pool): `cards_in_scope`, which
    /// leaves out a Unit immune to Spells, a Dormant card and, on a slashed board (R3554003), a card
    /// across the slash.
    InScope,
    /// v0.6.9l: every acting enemy Unit on the half the cast chose (R3554003).
    Reaching,
    /// v0.6.9k and before: every acting enemy Unit, across a slash too.
    Acting,
}

fn arcane_missiles(echo: i32, pool: Pool) -> Script {
    Script {
        static_flags: Some(StaticFlags { echo: Some(echo), ..StaticFlags::default() }),
        cry: Some(hook(move |ctx| {
            let amount = param(&*ctx, "damage");
            let enemy = opponent_of(ctx.controller);
            let ids: Vec<String> = match pool {
                Pool::InScope => cards_in_scope(ctx, &json_as(json!({ "side": "enemy" })))
                    .into_iter()
                    .map(|unit| unit.id)
                    .collect(),
                Pool::Reaching => {
                    let state: &GameState = &*ctx.sink.state;
                    let source = ctx.self_.as_ref();
                    active_units_of(state, enemy)
                        .iter()
                        .filter(|unit| reaches(state, source, unit))
                        .map(|unit| unit.id.clone())
                        .collect()
                }
                Pool::Acting => active_units_of(&*ctx.sink.state, enemy)
                    .iter()
                    .map(|unit| unit.id.clone())
                    .collect(),
            };
            // The state and the rng are two fields of the sink: the pool reads the one while the
            // draw rolls from the other (C+ #25 Soul Shot's shape).
            let rng: &mut Rng = &mut *ctx.sink.rng;
            let mut targets: Vec<Value> = vec![json!({ "of": "enemyHero" })];
            targets.extend(ids.iter().map(|id| {
                json!({ "of": "instance", "instanceId": id.clone() })
            }));
            let Some(picked) = rng.pick(&targets) else {
                return vec![];
            };
            vec![damage(json_as(json!({ "to": picked, "amount": amount })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: arcane_missiles(2, Pool::InScope),
        radiant: arcane_missiles(5, Pool::InScope),
    }
}

/// R767000: v0.6.9l drew each missile among the acting enemy Units on the half the cast chose
/// (R3554003), a Unit immune to Spells among them (issue #4424); `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_6_9l() -> CardScripts {
    CardScripts {
        base: arcane_missiles(2, Pool::Reaching),
        radiant: arcane_missiles(5, Pool::Reaching),
    }
}

/// v0.6.9k and before: each missile drew among every enemy Unit, across a slash too (R3554003);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_9j() -> CardScripts {
    CardScripts {
        base: arcane_missiles(2, Pool::Acting),
        radiant: arcane_missiles(5, Pool::Acting),
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
    const IMMUNE: &str = "classicplus-019-1"; // Top Loser: Radiant: Immune to Spells
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

        #[test]
        fn r4424000_with_only_spell_immune_enemies_every_missile_hits_the_enemy_hero() {
            let mut s = scenario(json!({
                "seed": "missiles-immune-only",
                "p1": { "hand": [MISSILES, FILLER], "field": [VANILLA] },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": IMMUNE, "lane": 1, "radiant": true },
                        { "def": IMMUNE, "lane": 2, "radiant": true },
                    ],
                },
            }));

            s.play(MISSILES, json!({}));

            assert_eq!(hits(&s), vec!["hero-p2", "hero-p2", "hero-p2"]);
            s.expect_health(P2, HERO_HEALTH - 3);
        }

        #[test]
        fn r4424000_a_spell_immune_enemy_is_never_drawn() {
            for seed in 1..=16 {
                let mut s = scenario(json!({
                    "seed": format!("missiles-immune-{seed}"),
                    "p1": { "hand": [MISSILES, FILLER], "field": [VANILLA] },
                    "p2": {
                        "hand": [FILLER],
                        "field": [
                            { "def": IMMUNE, "lane": 1, "radiant": true },
                            { "def": VANILLA, "lane": 2 },
                        ],
                    },
                }));
                let immune = s.unit_at(P2, 1).id.clone();
                s.play(MISSILES, json!({}));

                // Every missile lands: none is drawn onto the immune Unit and dropped there.
                let landed = hits(&s);
                assert_eq!(landed.len(), 3, "seed missiles-immune-{seed}");
                assert!(!landed.contains(&immune), "seed missiles-immune-{seed}");
            }
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
