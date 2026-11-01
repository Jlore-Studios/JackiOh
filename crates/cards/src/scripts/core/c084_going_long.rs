//! #84 Going Long (SPEC §8.4 row 84): Field Spell, cost 2 embiggen 4, Rare (no Quickdraw since #2521).
//!   Base:    "Your hero has Armor 2 (Embiggened (4): 5)"
//!   Radiant: "Armor 4 (Embiggened (4): 10)" — the cell changes only the two numbers (§8 Conventions).
//!   (Embiggened 4 and 8 until #2657.)
//!   Engine:  "Hero armor in pipeline step 2".
//!
//! Four numbers: normal 2, Embiggened 5; radiant normal 4, Embiggened 10 (`config.HERO_ARMOR`). The embiggen choice
//! lands on the instance (R81) and Radiant is its flag, so the engine reads both off the card in the
//! backrow and no card-side code is needed. #2521 took its Quickdraw tag away, and with it §6.2's
//! `quickdraw` static flag, so setup deals it as any other card. The Armor is the `heroArmor` static
//! flag, #73's shape: the pipeline sums every backrow grant (R124: sources add, unlike step 3's cap),
//! and §4.4 step 2, lethal, the scorer and §10.8's hero block all read that one function. Nothing is
//! stored, so the Armor stops when the Field Spell leaves, and "Ignores armor" skips step 2 whole.
//! Through v0.5.11 it was Quickdraw, which `script_v0_5_11` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-084";

/// Both faces are the same script: the `heroArmor` flag, and no Quickdraw (#2521). The four values
/// live in `config.HERO_ARMOR`, selected by the instance's `radiant` and `embiggened`. The card
/// declares them as `armor` and `paidArmor` (R386), which `damage::hero_armor_of` reads through
/// `params::declared_or`, so a Degrade or an Upgrade moves the Armor the price selects.
fn going_long() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            hero_armor: Some(FlagOrCount::Flag(true)),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = going_long();
    // "Armor 4 (Embiggened (4): 10)": two numbers, both of them the pipeline's to read (`HERO_ARMOR.radiant`).
    let radiant = going_long();
    CardScripts { base, radiant }
}

/// R767000: Going Long through v0.5.11, with §6.2's Quickdraw flag beside the `heroArmor` one.
fn going_long_v0_5_11() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            hero_armor: Some(FlagOrCount::Flag(true)),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

/// R767000: through v0.5.11 it was Quickdraw, setup dealing it into the opening hand (R640);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    CardScripts {
        base: going_long_v0_5_11(),
        radiant: going_long_v0_5_11(),
    }
}

// #84 Going Long (SPEC §8.4 row 84). BUILD M4-T4 (#2521). (Armor 4 and 8 until #2657) (patch v0.1.1: the
// paid-4 numbers were 5 and 10). The four numbers: base 2 / 5, radiant 4 / 10 (paid 2 / paid 4). The Armor is
// asserted through a real damage instance on the protected hero (§4.4 step 2); attackers are placed by
// `field`, so no Cry fires (R1). A card placed by setup is "paid 2"; "paid 4" is PLAYED with `embiggen:
// true`. R63: a hit that is 0 after Armor emits no `damage`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GOING_LONG: &str = "core-084"; // Field Spell, cost 2 embiggen 4, no Quickdraw (#2521)
    const BIGOT: &str = "core-002"; // set to 6/1 by `six_one()` (it prints 4/1 since patch v0.3.10)
    const POINTMASTER: &str = "core-020"; // 7/2
    const POSTDOC: &str = "core-061"; // 2/4
    const TRUE_STRIKE: &str = "core-044"; // Spell, 1 — "Deal 4 damage to a target, ignoring Armor"
    const MAGIC_JAMMED: &str = "core-036"; // Spell, 1 — "Destroy target backrow card; Lock its zone"
    const STOCKPILE: &str = "core-005"; // the spare card that keeps a turn meaningful (§2.5)
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";
    const SCARAB: &str = "core-007"; // 1/1, no Taunt — a body of p1's so §2.5 never auto-ends the turn

    /// Bigot at the 6/1 these hits are measured with.
    fn six_one() -> Value {
        json!({ "def": BIGOT, "statsOverride": { "attack": 6, "health": 1 } })
    }

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

    #[test]
    fn r386_an_upgrade_gives_armor_3_or_paid_6_and_a_degrade_armor_1_or_paid_4() {
        for (upgrade, embiggen, armor) in [(true, false, 3), (false, false, 1), (true, true, 6), (false, true, 4)] {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOING_LONG, STOCKPILE], "field": [SCARAB], "library": [MENACE, TIMMY] },
                "p2": spare(json!({ "field": [six_one()] })),
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

        /// Plain Core cards, none of them Quickdraw, to fill a 25-card deck around Going Long (§2.6 L3).
        const OTHERS: [&str; 24] = [
            "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
            "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
            "core-044", "core-053", "core-055", "core-001", "core-003", "core-004", "core-009", "core-010",
        ];

        /// Whether setup on `seed` deals Going Long into p1's opening hand.
        fn in_opening_hand(seed: &str) -> bool {
            let mut deck: Vec<String> = OTHERS.iter().map(|id| id.to_string()).collect();
            deck.push(GOING_LONG.to_string());
            let created = create_game(&CreateGameOptions {
                seed: seed.to_string(),
                decks: (deck.clone(), deck),
                ..CreateGameOptions::default()
            });
            let state = begin_game(&created).state;
            state.players.p1.hand.iter().any(|card| card.def_id == GOING_LONG)
        }

        #[test]
        fn no_quickdraw_on_either_face_so_setup_deals_it_as_any_other_card() {
            crate::register_all();
            // #2521: no Quickdraw tag, and no `quickdraw` flag on either face, the one setup reads (R640).
            assert!(!crate::card_def(GOING_LONG).tags.contains(&Tag::Quickdraw));
            let scripts = script();
            assert_ne!(scripts.base.flags().quickdraw, Some(true));
            assert_ne!(scripts.radiant.flags().quickdraw, Some(true));
            let s = scenario(json!({
                "p1": { "library": [GOING_LONG, { "def": GOING_LONG, "radiant": true }] },
            }));
            let library = s.pile(P1, "library");
            let plain = library.first().expect("the plain copy");
            let shiny = library.get(1).expect("the radiant copy");
            assert_ne!(flags_of(s.state(), plain).quickdraw, Some(true));
            assert_ne!(flags_of(s.state(), shiny).quickdraw, Some(true));

            // A Quickdraw card is dealt on every seed; this one stays in the library on some.
            let seed = seed_where("going-long-no-quickdraw", |seed| !in_opening_hand(seed));
            assert!(!in_opening_hand(&seed));
        }

        /// R767000: the script kept for the replays of v0.5.11 and before is Quickdraw on both faces.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11_is_quickdraw() {
            let kept = script_v0_5_11();
            for face in [&kept.base, &kept.radiant] {
                assert_eq!(face.flags().quickdraw, Some(true));
                assert!(face.flags().hero_armor.is_some());
            }
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
                "p2": { "field": [six_one()], "hand": [STOCKPILE] },
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
                "p1": { "backrow": [GOING_LONG], "field": [six_one()], "hand": [STOCKPILE] },
                "p2": { "hand": [STOCKPILE] },
            }));

            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p2"), vec![6]);
            s.expect_health(P2, 24);
        }

        #[test]
        fn embiggen_4_costs_4_mana_and_gives_armor_5_so_the_same_6_attack_hit_lands_for_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOING_LONG, STOCKPILE], "field": [SCARAB], "library": [MENACE, TIMMY] },
                "p2": spare(json!({ "field": [six_one()] })),
            }));

            s.play(GOING_LONG, json!({ "zone": 1, "embiggen": true }));
            s.expect_mana(P1, 0); // MAX_MANA 4 − the embiggen price 4

            s.end_turn(); // p2's turn
            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), vec![1]);
            s.expect_health(P1, 29);
        }

        #[test]
        fn the_base_price_is_still_2_and_paying_it_gives_armor_2_rather_than_5() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOING_LONG, STOCKPILE], "field": [SCARAB], "library": [MENACE, TIMMY] },
                "p2": spare(json!({ "field": [six_one()] })),
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
                "p2": { "field": [six_one(), six_one()], "hand": [MAGIC_JAMMED, STOCKPILE] },
            }));
            let going_long = s.backrow(P1, 1).expect("Going Long in the backrow");
            let first = s.unit(P2, 1).expect("the first Bigot");
            let second = s.unit(P2, 2).expect("the second Bigot");

            s.attack(&first, "hero"); // 6 − 2 = 4
            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&going_long.id) }));
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
                "p2": { "field": [six_one()], "hand": [STOCKPILE] },
            }));

            assert_eq!(shown_armor(&s, P1), 4);

            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), vec![2]);
            s.expect_health(P1, 28);
        }

        #[test]
        fn paid_4_armor_10_the_radiant_embiggen_price_gives_armor_10_so_a_6_attack_hit_is_nothing_r63() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": GOING_LONG, "radiant": true }, STOCKPILE],
                    "field": [SCARAB],
                    "library": [MENACE, TIMMY],
                },
                "p2": spare(json!({ "field": [six_one()] })),
            }));

            s.play(GOING_LONG, json!({ "zone": 1, "embiggen": true }));
            s.expect_mana(P1, 0);
            assert_eq!(shown_armor(&s, P1), 10);

            s.end_turn();
            s.attack(BIGOT, "hero");

            assert_eq!(damage_to(&s, "hero-p1"), Vec::<i32>::new());
            s.expect_health(P1, 30);
            assert_eq!(healed_count(&s), 0);
        }

        #[test]
        fn the_radiant_face_still_reduces_rather_than_blocks_a_14_attack_hit_lands_for_4_at_armor_10() {
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

            assert_eq!(damage_to(&s, "hero-p1"), vec![4]); // 14 − 10
            s.expect_health(P1, 26);
        }

        #[test]
        fn the_radiant_face_is_still_a_field_spell_in_the_backrow_and_has_no_quickdraw() {
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
            assert_ne!(flags_of(s.state(), &placed).quickdraw, Some(true));
            s.expect_mana(P1, 2);
        }
    }
}
