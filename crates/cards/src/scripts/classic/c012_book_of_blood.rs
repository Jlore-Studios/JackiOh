//! C #12 Book of Blood (SPEC §8.6 row 12). (1) Spell, Book, Epic.
//!   Base:    "Lifesteal\nDeal {damage} damage to a Unit." — damage 4
//!   Radiant: "Lifesteal\nDeal {damage} damage to a Unit." — damage 8
//!   Engine:  "A Unit target, either side; the effect states its own Lifesteal (R85), so your hero
//!            heals the amount dealt. Tunes: damage 4 ↑."
//!
//! "a Unit" narrows §8's "target" to units, on either side, so no hero is offered (R90).
//!
//! R85: the damage carries its own Lifesteal (`lifesteal: true`), so §4.4 step 8 heals the caster's
//! hero the amount actually dealt — after Armor, the anti-oneshot cap and R63's zero rule — and the
//! heal is one heal however the Lifesteal is read (printed keyword and flag), never two.
//!
//! The amount is the declared number `damage` (R386), read through `param`; both faces run this script.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-012";

/// "a Unit": units only, either side.
fn targets() -> Vec<TargetDecl> {
    vec![json_as(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] } }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
                "lifesteal": true,
            })))]
        })),
        ..Script::default()
    };

    // Same script: `param` reads the Radiant face's 8 off the running face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #12 Book of Blood — SPEC §8.6 row 12, BUILD M9 Classic row C 12: "Targets a Unit only, either
// side (no hero offered); one hit of 4; its own Lifesteal heals your hero the amount dealt (R85), so
// Armor lowers the heal and a Divine Shield leaves it at 0 (R63); radiant 8; its tuned number
// (damage) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classic-012";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt; Radiant 18/18.
    const SHIELDED: &str = "core-003"; // Right-house defender 1/1 Taunt, Divine Shield, Reborn.
    const POINTMASTER: &str = "core-020"; // (2) Unit 7/1 First Strike.
    const ROCK: &str = "core-066"; // The Rock 10/10 Indestructible.
    // Classic+ #26 Tommy Tempo, a Radiant Taunt set to 18/18 (`statsOverride`; it prints 10/10 since patch
    // v0.3.10): a big body no Immutable keeps a Spell off (R781001).
    const TEMPO: &str = "classicplus-026";
    const FILLER: &str = "core-005";

    fn at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit in {player:?} lane {lane}");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    fn hits(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn hero_heals(s: &Scenario, player: PlayerId) -> Vec<i32> {
        let hero = format!("hero-{}", player.as_str());
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { target_id, amount } if *target_id == hero => Some(*amount),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn deals_4_to_an_enemy_unit_in_one_hit_and_r85_its_lifesteal_heals_your_hero_4() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 20 }, "p2": { "hand": [FILLER], "field": [MENACE] } }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(MENACE, json!({ "health": 5 }));
            assert_eq!(hits(&s), vec![4]);
            s.expect_health(P1, 24);
            assert_eq!(hero_heals(&s, P1), vec![4]);
        }

        #[test]
        fn targets_your_own_unit_too_and_still_heals_you() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "field": [MENACE], "health": 20 }, "p2": { "hand": [FILLER] } }));

            let targets = at(&s, P1, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(MENACE, json!({ "health": 5 }));
            s.expect_health(P1, 24);
        }

        #[test]
        fn offers_no_hero_a_hero_pick_is_refused() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER], "field": [MENACE] } }));

            s.expect_refused(|s| s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p2" }] })));
            s.expect_in_zone(BOOK, "hand");
        }

        #[test]
        fn with_no_unit_on_the_board_it_fizzles_nothing_is_dealt_and_nobody_is_healed() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 20 }, "p2": { "hand": [FILLER] } }));

            s.play(BOOK, json!({}));

            assert!(hits(&s).is_empty());
            s.expect_health(P1, 20).expect_in_zone(BOOK, "graveyard");
        }

        #[test]
        fn r85_armor_lowers_the_heal_a_unit_in_defense_position_armor_1_takes_3_and_you_heal_3() {
            let mut s = scenario(json!({
                "p1": { "hand": [BOOK, FILLER], "health": 20 },
                "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "position": "DEF" }] },
            }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(MENACE, json!({ "health": 6 }));
            s.expect_health(P1, 23);
        }

        #[test]
        fn r63_a_divine_shield_takes_the_hit_whole_and_the_heal_is_0() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 20 }, "p2": { "hand": [FILLER], "field": [SHIELDED] } }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_in_zone(SHIELDED, "field").expect_events(json!("divineShieldLost"));
            s.expect_health(P1, 20);
            assert!(hero_heals(&s, P1).is_empty());
        }

        #[test]
        fn sec4_4_step_5_r85_the_heal_is_the_amount_dealt_which_is_not_capped_at_the_unit_s_health_4_on_a_7_1_heals_4() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 20 }, "p2": { "hand": [FILLER], "field": [POINTMASTER] } }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_in_zone(POINTMASTER, "graveyard");
            assert_eq!(hits(&s), vec![4]);
            assert_eq!(hero_heals(&s, P1), vec![4]);
            s.expect_health(P1, 24);
        }

        #[test]
        fn sec4_4_step_4_r85_an_indestructible_unit_takes_no_damage_so_the_heal_is_0() {
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 20 }, "p2": { "hand": [FILLER], "field": [ROCK] } }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(ROCK, json!({ "health": 10 })).expect_in_zone(ROCK, "field");
            assert!(hits(&s).is_empty());
            assert!(hero_heals(&s, P1).is_empty());
            s.expect_health(P1, 20);
        }

        #[test]
        fn r386_an_upgrade_makes_it_deal_and_heal_5_a_degrade_3() {
            let mut up = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 20 }, "p2": { "hand": [FILLER], "field": [MENACE] } }));
            step_param(up.card_mut(BOOK), "damage", 1);
            let targets = at(&up, P2, 1);
            up.play(BOOK, json!({ "targets": targets }));
            up.expect_stats(MENACE, json!({ "health": 4 })).expect_health(P1, 25);

            let mut down = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 20 }, "p2": { "hand": [FILLER], "field": [MENACE] } }));
            step_param(down.card_mut(BOOK), "damage", -1);
            let targets = at(&down, P2, 1);
            down.play(BOOK, json!({ "targets": targets }));
            down.expect_stats(MENACE, json!({ "health": 6 })).expect_health(P1, 23);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn deals_8_in_one_hit_and_heals_your_hero_8() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER], "health": 20 },
                "p2": { "hand": [FILLER], "field": [{ "def": TEMPO, "radiant": true, "statsOverride": { "attack": 18, "health": 18 } }] },
            }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(TEMPO, json!({ "health": 10, "maxHealth": 18 }));
            assert_eq!(hits(&s), vec![8]);
            s.expect_health(P1, 28);
        }

        #[test]
        fn r63_a_divine_shield_still_leaves_the_radiant_heal_at_0() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER], "health": 20 },
                "p2": { "hand": [FILLER], "field": [SHIELDED] },
            }));

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_health(P1, 20);
        }

        #[test]
        fn r386_an_upgrade_steps_the_radiant_8_to_9() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER], "health": 10 },
                "p2": { "hand": [FILLER], "field": [{ "def": TEMPO, "radiant": true, "statsOverride": { "attack": 18, "health": 18 } }] },
            }));
            step_param(s.card_mut(BOOK), "damage", 1);

            let targets = at(&s, P2, 1);
            s.play(BOOK, json!({ "targets": targets }));

            s.expect_stats(TEMPO, json!({ "health": 9 })).expect_health(P1, 19);
        }
    }
}
