//! FvH #72.2 Swipe (SPEC §8.14 row 72.2; §4.4, §4.5, R59, R81, R346, R1748001): (1) Spell, Other-Origins,
//! Token (printed Epic), a token FvH #72 Otherworldly Naturalism adds.
//!   Base:    "Pierce / Deal {damage} damage to an enemy and {splash} damage to all other enemies." — 4
//!            and 1.
//!   Radiant: "Pierce / Deal {damage} damage to an enemy and {splash} damage to all other enemies." — 8
//!            and 2.
//!
//! Engine: one declared enemy target, a Unit or the hero (R81); one hit of the declared `damage` on
//! it, then one of `splash` on each other enemy Unit (read once as the clause begins, `forEachCard`) and
//! on the enemy hero unless it was the one chosen, all in one list before one state check (R59).
//! Hearthstone's Swipe. The printed Pierce skips Armor for every hit, stated on each as True Strike's
//! is (`ignoreArmor`, R346); Spell Damage adds to each as ever. The faces differ only in the declared
//! numbers. Built by S05 (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-072-2";

/// The enemy Unit the play chose, if it chose one.
fn chosen_unit(ctx: &EffectContext<'_>) -> Option<String> {
    match ctx.targets.first() {
        Some(Selection::Instance { instance_id }) => Some(instance_id.clone()),
        _ => None,
    }
}

/// Every enemy Unit on the field but the one chosen (§3.2, lane order).
fn other_enemy_units(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let chosen = chosen_unit(ctx);
    active_units_of(ctx.state, opponent_of(ctx.controller))
        .into_iter()
        .filter(|unit| chosen.as_deref() != Some(unit.id.as_str()))
        .map(|unit| unit.id.clone())
        .collect()
}

fn swipe() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))],
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            let splash = param(&*ctx, "splash");
            let hero_chosen = matches!(ctx.targets.first(), Some(Selection::Hero { .. }));
            let mut effects = vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount, "ignoreArmor": true }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(other_enemy_units),
                    each: Arc::new(move |instance_id: &str| {
                        damage(json_as(json!({
                            "to": { "of": "instance", "instanceId": instance_id },
                            "amount": splash,
                            "ignoreArmor": true,
                        })))
                    }),
                }),
            ];
            if !hero_chosen {
                effects.push(damage(json_as(json!({
                    "to": { "of": "enemyHero" },
                    "amount": splash,
                    "ignoreArmor": true,
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = swipe();
    // The same script: the Radiant face's 8 and 2 are its declared `damage` and `splash`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #72.2 Swipe — SPEC §8.14 row 72.2 (R59, R81, R346).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BODY: &str = "stand-in (1) 1/9";
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 holds Swipe and a 1/9; p2 three 1/9s and `armor` on its hero.
    fn board(radiant: bool, armor: i32) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [BODY] },
            "p2": { "hand": [FILLER], "field": [BODY, BODY, BODY], "armor": armor },
        }))
    }

    /// Swipe aimed at p2's lane 2: the chosen takes `amount`, every other enemy `splash`, p1 nothing.
    fn swiped_at_a_unit(radiant: bool, amount: i32, splash: i32) {
        let mut s = board(radiant, 0);
        let (left, chosen, right, mine) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3), s.unit_at(P1, 1));

        s.play(ID, pick::targets(pick::instance(&chosen)));

        s.expect_stats(&chosen, json!({ "health": 9 - amount }));
        s.expect_stats(&left, json!({ "health": 9 - splash }));
        s.expect_stats(&right, json!({ "health": 9 - splash }));
        s.expect_health(P2, HERO_HEALTH - splash);
        s.expect_stats(&mine, json!({ "health": 9 }));
        s.expect_health(P1, HERO_HEALTH);
    }

    /// Swipe aimed at p2's hero: it takes `amount` once, every enemy Unit `splash`.
    fn swiped_at_the_hero(radiant: bool, amount: i32, splash: i32) {
        let mut s = board(radiant, 0);
        let units = nonempty((1..=3).map(|lane| s.unit_at(P2, lane)).collect(), "p2's units");

        s.play(ID, pick::targets(pick::hero("p2")));

        s.expect_health(P2, HERO_HEALTH - amount);
        for unit in units {
            s.expect_stats(&unit, json!({ "health": 9 - splash }));
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r59_deals_4_to_the_chosen_enemy_unit_and_1_to_every_other_enemy() {
            swiped_at_a_unit(false, 4, 1);
        }

        #[test]
        fn r81_aimed_at_the_enemy_hero_it_hits_it_once_for_4() {
            swiped_at_the_hero(false, 4, 1);
        }

        #[test]
        fn r346_its_pierce_skips_the_enemy_heros_armor() {
            let mut s = board(false, 5);

            s.play(ID, pick::targets(pick::hero("p2")));

            s.expect_health(P2, HERO_HEALTH - 4);
            assert_eq!(hero_of(s.state(), P2).armor, 5, "the Armor is untouched");
        }

        #[test]
        fn r346_its_pierce_skips_a_units_armor_on_the_splash() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID, FILLER] },
                "p2": { "hand": [FILLER], "field": [BODY, ARMORED] },
            }));
            let (chosen, armored) = (s.unit_at(P2, 1), s.unit_at(P2, 2));

            s.play(ID, pick::targets(pick::instance(&chosen)));

            s.expect_stats(&armored, json!({ "health": 7 - 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r59_deals_8_to_the_chosen_enemy_unit_and_2_to_every_other_enemy() {
            swiped_at_a_unit(true, 8, 2);
        }

        #[test]
        fn r81_aimed_at_the_enemy_hero_it_hits_it_once_for_8() {
            swiped_at_the_hero(true, 8, 2);
        }
    }
}
