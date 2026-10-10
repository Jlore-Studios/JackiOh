//! #84 Going Long (SPEC §8.4 row 84): Field Spell, Quickdraw, cost 2 embiggen 4, Rare.
//!   Base:    "Your hero has Armor 2 (paid 4: 5)"
//!   Radiant: "Armor 4 (paid 4: 10)" — the cell changes only the two numbers (§8 Conventions).
//!   Engine:  "Hero armor in pipeline step 2".
//!
//! Four numbers: paid 2 → 2, paid 4 → 5; radiant paid 2 → 4, radiant paid 4 → 10. The embiggen choice
//! lands on the instance (R81) and Radiant is its flag, so the engine reads both off the card in the
//! backrow and no card-side code is needed. Quickdraw is §6.2's static flag: setup moves the card into
//! the opening hand on either face. The Armor is the `heroArmor` static flag, #73's shape: the pipeline
//! sums every backrow grant (R124: sources add, unlike step 3's cap), and §4.4 step 2, lethal, the
//! scorer and §10.8's hero block all read that one function. Nothing is stored, so the Armor stops when
//! the Field Spell leaves, and "Ignores armor" skips step 2 whole.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-084";

/// Both faces are the same script: §6.2's Quickdraw flag and the `heroArmor` flag. The four values
/// live in `config.HERO_ARMOR`, selected by the instance's `radiant` and `embiggened`. The card
/// declares them as `armor` and `paidArmor` (R386), which `damage::hero_armor_of` reads through
/// `params::declared_or`, so a Degrade or an Upgrade moves the Armor the price selects.
fn going_long() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            hero_armor: Some(FlagOrCount::Flag(true)),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = going_long();
    // "Armor 4 (paid 4: 10)": two numbers, both of them the pipeline's to read (`HERO_ARMOR.radiant`).
    let radiant = going_long();
    CardScripts { base, radiant }
}

// #84 Going Long (SPEC §8.4 row 84). BUILD M4-T4: "In opening hand; embiggen 2 → Armor 2, 4 → Armor 4
// on the hero; radiant 4 / 8" (patch v0.1.1: the paid-4 numbers were 5 and 10). The Armor is asserted through a real damage instance on the protected
// hero (§4.4 step 2); attackers are placed by `field`, so no Cry fires (R1). A card placed by setup is
// "paid 2"; "paid 4" is PLAYED with `embiggen: true`. R63: a hit that is 0 after Armor emits no `damage`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GOING_LONG: &str = "core-084"; // Field Spell, Quickdraw, cost 2 embiggen 4
    const BIGOT: &str = "core-002"; // 6/1
    const POINTMASTER: &str = "core-020"; // 7/2
    const POSTDOC: &str = "core-061"; // 2/4
    const TRUE_STRIKE: &str = "core-044"; // Spell, 1 — "Deal 4 damage to a target, ignoring Armor"
    const MAGIC_JAMMED: &str = "core-036"; // Spell, 1 — "Destroy target backrow card; Lock its zone"
    const STOCKPILE: &str = "core-005"; // the spare card that keeps a turn meaningful (§2.5)
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";
    const SCARAB: &str = "core-007"; // 1/1, no Taunt — a body of p1's so §2.5 never auto-ends the turn

    /// `extra` with the spare hand `[STOCKPILE, TIMMY]` and library `[MENACE, TIMMY]` set.
    fn spare(extra: Value) -> Value {
        let mut side = extra;
        if let Some(fields) = side.as_object_mut() {
            fields.insert("hand".into(), json!([STOCKPILE, TIMMY]));
            fields.insert("library".into(), json!([MENACE, TIMMY]));
        }
        side
    }

    fn damage_to(s: &Scenario, target_id: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id: to, amount, .. } if to == target_id => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn healed_count(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Healed { .. }))
            .count()
    }

    /// What the client is told this hero's Armor is (§10.8).
    fn shown_armor(s: &Scenario, player: PlayerId) -> i32 {
        s.view(Some(player)).you.hero.armor
    }

    fn targeting(instance_id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": instance_id }])
    }

    #[test]
    fn r386_an_upgrade_gives_armor_3_or_paid_5_and_a_degrade_armor_1_or_paid_3() {
        for (upgrade, embiggen, armor) in [(true, false, 3), (false, false, 1), (true, true, 5), (false, true, 3)] {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOING_LONG, STOCKPILE], "field": [SCARAB], "library": [MENACE, TIMMY] },
                "p2": spare(json!({ "field": [BIGOT] })),
            }));
            let key = if embiggen { "paidArmor" } else { "armor" };
            let moved = if upgrade {
                crate::upgrade_number(&mut s, GOING_LONG, key)
            } else {
                crate::degrade_number(&mut s, GOING_LONG, key)
            };
            assert_eq!(moved, armor);
            s.play(GOING_LONG, json!({ "zone": 1, "embiggen": embiggen }));
            assert_eq!(shown_armor(&s, P1), armor);
        }
    }

    mod n84_going_long_the_opening_hand {
        use super::*;

        #[test]
        fn s6_2_quickdraw_the_engine_reads_the_flag_off_a_real_instance_of_either_face() {
            crate::register_all();
            // Setup moves every library card carrying `quickdraw` into the opening hand (the engine's own
            // setup test). This card owes the flag; `flags_of` resolves it through the instance's face, so
            // this asserts the radiant face carries it too.
            let s = scenario(json!({
                "p1": { "library": [GOING_LONG, { "def": GOING_LONG, "radiant": true }] },
            }));
            let library = s.pile(P1, "library");
            let plain = library.first().expect("the plain copy");
            let shiny = library.get(1).expect("the radiant copy");

            assert_eq!(flags_of(s.state(), plain).quickdraw, Some(true));
            assert_eq!(flags_of(s.state(), shiny).quickdraw, Some(true));
        }
    }

    mod n84_going_long_base {
        use super::*;

        #[test]
        fn s4_4_step_2_paid_2_gives_your_hero_armor_2_so_a_6_attack_hit_lands_for_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [GOING_LONG], "hand": [STOCKPILE] },
                "p2": { "field": [BIGOT], "hand": [STOCKPILE] },
            }));

            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), vec![4]);
            s.expect_health(P1, 26);
        }

        #[test]
        fn s10_8_the_hero_block_shows_the_armor_the_pipeline_reads() {
            crate::register_all();
            let s = scenario(json!({
                "p1": { "backrow": [GOING_LONG], "hand": [STOCKPILE] },
                "p2": spare(json!({})),
            }));

            assert_eq!(shown_armor(&s, P1), 2);
            assert_eq!(shown_armor(&s, P2), 0);
        }

        #[test]
        fn r63_a_hit_reduced_to_0_by_the_armor_deals_nothing_heals_nothing_and_emits_no_damage() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [GOING_LONG], "hand": [STOCKPILE] },
                "p2": { "field": [POSTDOC], "hand": [STOCKPILE] }, // attack 2, exactly the Armor
            }));

            s.attack(POSTDOC, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), Vec::<i32>::new());
            s.expect_health(P1, 30);
            assert_eq!(healed_count(&s), 0);
        }

        #[test]
        fn your_hero_the_armor_protects_its_controller_only() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": [GOING_LONG], "field": [BIGOT], "hand": [STOCKPILE] },
                "p2": { "hand": [STOCKPILE] },
            }));

            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p2"), vec![6]);
            s.expect_health(P2, 24);
        }

        #[test]
        fn embiggen_4_costs_4_mana_and_gives_armor_4_so_the_same_6_attack_hit_lands_for_2() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOING_LONG, STOCKPILE], "field": [SCARAB], "library": [MENACE, TIMMY] },
                "p2": spare(json!({ "field": [BIGOT] })),
            }));

            s.play(GOING_LONG, json!({ "zone": 1, "embiggen": true }));
            s.expect_mana(P1, 0); // MAX_MANA 4 − the embiggen price 4

            s.end_turn(); // p2's turn
            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), vec![2]);
            s.expect_health(P1, 28);
        }

        #[test]
        fn the_base_price_is_still_2_and_paying_it_gives_armor_2_rather_than_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOING_LONG, STOCKPILE], "field": [SCARAB], "library": [MENACE, TIMMY] },
                "p2": spare(json!({ "field": [BIGOT] })),
            }));

            s.play(GOING_LONG, json!({ "zone": 1 }));
            s.expect_mana(P1, 2);

            s.end_turn();
            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), vec![4]);
            s.expect_health(P1, 26);
        }

        #[test]
        fn s4_4_step_2_ignores_armor_skips_it_true_strike_lands_whole_where_an_attack_is_reduced() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [GOING_LONG], "hand": [STOCKPILE] },
                "p2": { "field": [POINTMASTER], "hand": [TRUE_STRIKE, STOCKPILE] },
            }));

            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            assert_eq!(damage_to(&s, "hero-p1"), vec![4]); // the printed 4, armor skipped

            s.attack(POINTMASTER, "hero");
            assert_eq!(damage_to(&s, "hero-p1"), vec![4, 5]); // 7 − 2 armor
            s.expect_health(P1, 21);
        }

        #[test]
        fn the_armor_stops_the_moment_the_field_spell_leaves_the_backrow() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [GOING_LONG], "hand": [STOCKPILE] },
                "p2": { "field": [BIGOT, BIGOT], "hand": [MAGIC_JAMMED, STOCKPILE] },
            }));
            let going_long = s.backrow(P1, 1).expect("Going Long in the backrow");
            let first = s.unit(P2, 1).expect("the first Bigot");
            let second = s.unit(P2, 2).expect("the second Bigot");

            s.attack(&first, "hero"); // 6 − 2 = 4
            s.play(MAGIC_JAMMED, json!({ "targets": targeting(&going_long.id) }));
            s.expect_in_zone(&going_long, "graveyard");
            s.attack(&second, "hero"); // no Armor left: the whole 6

            assert_eq!(damage_to(&s, "hero-p1"), vec![4, 6]);
            s.expect_health(P1, 20);
            assert_eq!(shown_armor(&s, P1), 0);
        }
    }

    mod n84_going_long_radiant {
        use super::*;

        #[test]
        fn armor_4_paid_2_on_the_radiant_face_so_a_6_attack_hit_lands_for_2() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [{ "def": GOING_LONG, "radiant": true }], "hand": [STOCKPILE] },
                "p2": { "field": [BIGOT], "hand": [STOCKPILE] },
            }));

            assert_eq!(shown_armor(&s, P1), 4);

            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), vec![2]);
            s.expect_health(P1, 28);
        }

        #[test]
        fn paid_4_armor_8_the_radiant_embiggen_price_gives_armor_8_so_a_6_attack_hit_is_nothing_r63() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": GOING_LONG, "radiant": true }, STOCKPILE],
                    "field": [SCARAB],
                    "library": [MENACE, TIMMY],
                },
                "p2": spare(json!({ "field": [BIGOT] })),
            }));

            s.play(GOING_LONG, json!({ "zone": 1, "embiggen": true }));
            s.expect_mana(P1, 0);
            assert_eq!(shown_armor(&s, P1), 8);

            s.end_turn();
            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), Vec::<i32>::new());
            s.expect_health(P1, 30);
            assert_eq!(healed_count(&s), 0);
        }

        #[test]
        fn the_radiant_face_still_reduces_rather_than_blocks_a_14_attack_hit_lands_for_6_at_armor_8() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": GOING_LONG, "radiant": true }, STOCKPILE],
                    "field": [SCARAB],
                    "library": [MENACE, TIMMY],
                },
                "p2": spare(json!({ "field": [{ "def": POINTMASTER, "radiant": true }] })), // radiant #20 is a 14/2
            }));

            s.play(GOING_LONG, json!({ "zone": 1, "embiggen": true }));
            s.end_turn();
            s.attack(POINTMASTER, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), vec![6]); // 14 − 8
            s.expect_health(P1, 24);
        }

        #[test]
        fn s6_2_the_radiant_face_is_still_quickdraw_and_still_a_field_spell_in_the_backrow() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": GOING_LONG, "radiant": true }, STOCKPILE],
                    "field": [SCARAB],
                    "library": [MENACE, TIMMY],
                },
                "p2": spare(json!({})),
            }));

            s.play(GOING_LONG, json!({ "zone": 2 }));

            let placed = s.backrow(P1, 2);
            assert!(placed.is_some());
            let placed = placed.unwrap();
            assert!(placed.radiant);
            assert_eq!(flags_of(s.state(), &placed).quickdraw, Some(true));
            s.expect_mana(P1, 2);
        }
    }
}
