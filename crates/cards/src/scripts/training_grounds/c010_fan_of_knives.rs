//! TG #10 Fan of Knives (SPEC §8.11 row TG 10; §4.4, §6.1 Pierce, §6.2 Echo, R51, R59, R346): (1)
//! Spell, Common, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Pierce / Deal {damage} damage to all enemies. Draw {draw|card|cards}." — 1, 1.
//!   Radiant: "Echo 1, Pierce" and the same text — 2, 1: it resolves a second time.
//! S #13 `damage_all` before one state check (R51, R59), then `draw`. Pierce stated by effect (C+ #21, R346); Spell Damage raises hit (§4.4 step 0). Radiant prints Echo 1 (`StaticFlags.echo`).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-010";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                damage_all(json_as(json!({
                    "amount": param(&*ctx, "damage"),
                    "side": "enemy",
                    "heroes": true,
                    "ignoreArmor": true,
                }))),
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            ]
        })),
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// TG #10 Fan of Knives — SPEC §8.11 row TG 10, BUILD M13 row TG 10.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FAN: &str = "traininggrounds-010";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ARMORED: &str = "core-025"; // (4) Unit 7/7, Armor 7.
    const SHIELDED: &str = "classic-089"; // (2) Unit 5/6, Divine Shield.
    const ARCH_MAGE: &str = "meditativeplus-008"; // (2) Unit 4/8, Spell Damage +3.

    fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    /// p1 holds the Fan (Radiant when `radiant`) with `mine` on its field; p2's field holds `theirs`.
    fn fan_board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": FAN, "radiant": radiant }], "field": mine, "mana": 10, "library": filler(6) },
            "p2": { "hand": filler(1), "field": theirs, "library": filler(6) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn one_damage_to_each_enemy_unit_and_the_enemy_hero() {
            let mut s = fan_board(false, json!([]), json!([VANILLA, VANILLA]));
            let (first, second) = (s.unit_at(P2, 1), s.unit_at(P2, 2));
            s.play(FAN, json!({}));
            assert_eq!(s.card(&first).damage, 1);
            assert_eq!(s.card(&second).damage, 1);
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 1);
        }

        #[test]
        fn no_friendly_unit_or_hero_is_hit() {
            let mut s = fan_board(false, json!([VANILLA]), json!([VANILLA]));
            let mine = s.unit_at(P1, 1);
            s.play(FAN, json!({}));
            assert_eq!(s.card(&mine).damage, 0);
            assert_eq!(hero_health(&s, P1), HERO_HEALTH);
        }

        #[test]
        fn pierce_skips_a_units_armor_and_the_enemy_heros() {
            let mut s = fan_board(false, json!([]), json!([ARMORED]));
            let armored = s.unit_at(P2, 1);
            s.state_mut().players[P2].hero.armor = 3;
            s.play(FAN, json!({}));
            assert_eq!(s.card(&armored).damage, 1);
            assert_eq!(s.state().players[P2].hero.armor, 3);
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 1);
        }

        #[test]
        fn a_divine_shield_absorbs_its_hit() {
            let mut s = fan_board(false, json!([]), json!([SHIELDED]));
            let shielded = s.unit_at(P2, 1);
            s.play(FAN, json!({}));
            assert_eq!(s.card(&shielded).damage, 0);
            assert_eq!(s.card(&shielded).divine_shield_spent, Some(true));
        }

        #[test]
        fn units_it_takes_to_zero_die_together_after_the_sweep() {
            let mut s = fan_board(
                false,
                json!([]),
                json!([{ "def": VANILLA, "lane": 1, "damage": 3 }, { "def": VANILLA, "lane": 2, "damage": 3 }]),
            );
            let (first, second) = (s.unit_at(P2, 1), s.unit_at(P2, 2));
            s.play(FAN, json!({}));
            s.expect_in_zone(&first, "graveyard");
            s.expect_in_zone(&second, "graveyard");
            // R59: every hit, the hero's included, lands before the one state check destroys either.
            let events = s.last_events();
            let last_hit = events.iter().rposition(|event| matches!(event, GameEvent::Damage { .. }));
            let first_death = events.iter().position(|event| matches!(event, GameEvent::Destroyed { .. }));
            assert!(last_hit.unwrap() < first_death.unwrap());
        }

        #[test]
        fn then_its_caster_draws_one() {
            let mut s = fan_board(false, json!([]), json!([]));
            let held = s.hand(P1).len();
            s.play(FAN, json!({}));
            assert_eq!(s.hand(P1).len(), held, "the Fan left the hand and one card came in");
        }

        #[test]
        fn spell_damage_raises_each_hit() {
            let mut s = fan_board(false, json!([ARCH_MAGE]), json!([ARMORED]));
            let theirs = s.unit_at(P2, 1);
            s.play(FAN, json!({}));
            assert_eq!(s.card(&theirs).damage, 4, "1 and the Arch Mage's +3");
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 4);
        }

        #[test]
        fn damage_and_draw_read_through_param() {
            let mut s = fan_board(false, json!([]), json!([]));
            assert_eq!(upgrade_number(&mut s, FAN, "damage"), 2);
            assert_eq!(upgrade_number(&mut s, FAN, "draw"), 2);
            let held = s.hand(P1).len();
            s.play(FAN, json!({}));
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 2);
            assert_eq!(s.hand(P1).len(), held + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn two_damage_to_each_enemy_and_the_enemy_hero_piercing() {
            let mut s = fan_board(true, json!([]), json!([ARMORED]));
            let armored = s.unit_at(P2, 1);
            s.play(FAN, json!({}));
            // Echo 1: the whole resolves twice, 2 each time.
            assert_eq!(s.card(&armored).damage, 4);
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 4);
        }

        #[test]
        fn echo_one_resolves_it_twice_four_damage_and_two_cards() {
            let mut s = fan_board(true, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            let held = s.hand(P1).len();
            s.state_mut().players[P2].hero.armor = 5;
            s.play(FAN, json!({}));
            s.expect_in_zone(&theirs, "graveyard");
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 4, "the repeat pierces the hero's Armor too");
            assert_eq!(s.hand(P1).len(), held + 1, "one draw per resolution, the Fan gone");
        }

        #[test]
        fn each_resolution_draws_one() {
            let mut s = fan_board(true, json!([]), json!([]));
            assert_eq!(
                jackioh_engine::prelude::param_value(
                    s.state(),
                    Some(s.card(FAN)),
                    "draw",
                    jackioh_engine::prelude::ParamValueOptions::default()
                ),
                1
            );
            let held = s.hand(P1).len();
            s.play(FAN, json!({}));
            assert_eq!(s.hand(P1).len(), held + 1);
        }
    }
}
