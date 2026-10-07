//! C+ #12.1 Devour (SPEC §8.7 row 12.1): (0) Spell, Pancake, Token (printed Legendary).
//!   Base:    "Destroy a Unit. Your hero takes damage equal to its health."
//!   Radiant: "Destroy a Unit. Heal your hero by its health."
//! A target Unit on either side; its current health is read before the destroy. The self-hit is one
//! instance from Devour through §4.4 (your Armor, caps and Spell Damage apply); the gain is a Heal. An
//! Indestructible target survives (R46) and the damage or heal still happens (Hearthstone's Obliterate).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012-1";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// The chosen Unit's health now, before the destroy marks it; 0 when it is gone (nothing then).
fn health_of_chosen(ctx: &EffectContext<'_>) -> i32 {
    match instance_of(ctx, &json_as::<TargetSpec>(json!({ "of": "chosen" }))) {
        None => 0,
        Some(unit) => unit_view(&ctx.state, &unit).health.max(0),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|ctx| {
                vec![
                    destroy(json_as(json!({ "target": { "of": "chosen" } }))),
                    damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": health_of_chosen(ctx) }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            targets: targets(),
            cry: Some(hook(|ctx| {
                let health = health_of_chosen(ctx);
                let mut effects = vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))];
                if health > 0 {
                    effects.push(heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": health }))));
                }
                effects
            })),
            ..Script::default()
        },
    }
}

// C+ #12.1 Devour — SPEC §8.7 row 12.1, BUILD M9 Classic+ row C+ 12.1: "Destroys a target Unit and your
// hero takes damage equal to that Unit's current health read before the destroy, one instance from
// Devour through your Armor and caps (Anime Armor clamps it to 1), raised by your own Spell Damage
// (§4.4 step 0); an Indestructible target survives and the damage still happens; radiant heals your
// hero by that health instead, a heal that an opposing Blood Moon in force turns into Pierce damage
// (R413)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const DEVOUR: &str = "classicplus-012-1";
    const MENACE: &str = "core-019"; // 9/9
    const ROCK: &str = "core-066"; // 10/10 Indestructible
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2
    const ANIME_ARMOR: &str = "classicplus-011"; // Aura: your hero takes at most 1 damage at a time
    const BLOOD_MOON: &str = "classicplus-022"; // Trap: enemy heals become Pierce damage
    const FILLER: &str = "core-005";

    fn at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit in {player} lane {lane}");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    use crate::merged;

    fn devour(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": merged(json!({ "hand": [{ "def": DEVOUR, "radiant": radiant }, FILLER] }), p1),
            "p2": merged(json!({ "hand": [FILLER], "field": [MENACE] }), p2),
        }))
    }

    /// The `damage` events on `target_id`, as their sources.
    fn hits_on(s: &Scenario, target_id: &str) -> Vec<Option<String>> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { source_id, target_id: hit, .. } if hit == target_id => Some(source_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn destroys_a_target_unit_and_your_hero_takes_damage_equal_to_its_health_one_instance_from_devour() {
            crate::register_all();
            let mut s = devour(false, json!({}), json!({}));
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_in_zone(MENACE, "graveyard").expect_health(P1, HERO_HEALTH - 9);
            let hits = hits_on(&s, "hero-p1");
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0], Some(s.card(DEVOUR).id.clone()));
        }

        #[test]
        fn reads_its_current_health_damage_included_before_the_destroy_either_side_is_legal() {
            crate::register_all();
            let mut s = devour(false, json!({ "field": [{ "def": MENACE, "damage": 5 }] }), json!({ "field": [] }));
            let targets = at(&s, P1, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_health(P1, HERO_HEALTH - 4);
        }

        #[test]
        fn s4_4_your_hero_s_armor_takes_its_share() {
            crate::register_all();
            let mut s = devour(false, json!({ "armor": 5 }), json!({}));
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_health(P1, HERO_HEALTH - 4);
        }

        #[test]
        fn s4_4_step_0_your_own_spell_damage_raises_it() {
            crate::register_all();
            let mut s = devour(false, json!({ "field": [SOLARIUS] }), json!({}));
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_health(P1, HERO_HEALTH - 11);
        }

        #[test]
        fn anime_armor_clamps_the_hit_to_1() {
            crate::register_all();
            let mut s = devour(false, json!({ "field": [ANIME_ARMOR] }), json!({}));
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_health(P1, HERO_HEALTH - 1);
        }

        #[test]
        fn r46_an_indestructible_target_survives_and_the_damage_still_happens() {
            crate::register_all();
            let mut s = devour(false, json!({}), json!({ "field": [ROCK] }));
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_in_zone(ROCK, "field").expect_health(P1, HERO_HEALTH - 10);
        }

        #[test]
        fn targets_a_unit_only_never_a_hero() {
            crate::register_all();
            let mut s = devour(false, json!({}), json!({}));
            s.expect_refused(|s| s.play(DEVOUR, json!({ "targets": [{ "pick": "hero", "player": "p2" }] })));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn destroys_the_unit_and_heals_your_hero_by_its_health_instead() {
            crate::register_all();
            let mut s = devour(true, json!({ "health": 10 }), json!({}));
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_in_zone(MENACE, "graveyard").expect_health(P1, 19);
            assert_eq!(hits_on(&s, "hero-p1").len(), 0);
        }

        #[test]
        fn r46_an_indestructible_target_survives_and_the_heal_still_happens() {
            crate::register_all();
            let mut s = devour(true, json!({ "health": 10 }), json!({ "field": [ROCK] }));
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_in_zone(ROCK, "field").expect_health(P1, 20);
        }

        #[test]
        fn r413_an_opposing_blood_moon_in_force_turns_the_heal_into_pierce_damage() {
            crate::register_all();
            let mut s = devour(
                true,
                json!({ "health": 20 }),
                json!({ "backrow": [{ "def": BLOOD_MOON, "faceUp": false }] }),
            );
            let targets = at(&s, P2, 1);
            s.play(DEVOUR, json!({ "targets": targets }));
            s.expect_in_zone(MENACE, "graveyard").expect_health(P1, 11);
        }
    }
}
