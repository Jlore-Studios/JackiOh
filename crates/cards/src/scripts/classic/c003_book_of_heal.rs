//! C #3 Book of Heal (SPEC §8.6 row 3). (1) Spell, Book, Epic.
//!   Base:    "Heal a target {heal}." — heal 9
//!   Radiant: "Heal a target {heal}." — heal 18
//!   Engine:  "§6.3 Heal on any unit or hero, as #47 Fig of Life reads "Heal" (R19). Tunes: heal 9 ↑
//!            (step 2)."
//!
//! R19 and §8's Conventions: "a target" is any unit or hero on either side, so the declaration names
//! both kinds with `side: "any"` (a bare `target` is units only, R90). The pick is declared, so it
//! travels in the play action (R81) and resolution never pauses.
//!
//! What "heal" does is §6.3's row, not this card's: a unit loses up to that much damage and never
//! rises past its max health, a hero gains it with no cap (§3). `effects/heal.ts` is that split.
//!
//! The amount is the declared number `heal` (R386), read through `param`: 9 on the base face, 18 on
//! the Radiant one, moved by a Degrade or an Upgrade two at a time (its declared step). Both faces run
//! this one script, since the faces differ only in that number.

use jackioh_engine::effects::heal;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-003";

/// R19: any unit or hero, either side. (TS `const targets: TargetDecl[]`.)
fn targets() -> Vec<TargetDecl> {
    vec![
        // R656: a heal helps, so a random cast that targets enemies aims this at friends.
        TargetDecl {
            aim: Some(TargetAim::Help),
            ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))
        },
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![heal(json_as(json!({ "target": { "of": "chosen" }, "amount": param(&*ctx, "heal") })))]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 18 is its declared `heal`, which `param` reads off the running face.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #3 Book of Heal — SPEC §8.6 row 3, BUILD M9 Classic row C 3: "Heal a target 9, either side: a
// Unit up to its max health, a hero with no cap (R19); a full-health Unit gains nothing; radiant 18;
// its tuned number (heal, step 2) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classic-003";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt; Radiant 18/18.
    const FILLER: &str = "core-005"; // a card that keeps a hand from auto-ending the turn (§2.5).

    /// An engine value as the JSON TS compares it by.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// The unit in that lane, as the play's `targets` (`Selection[]`).
    fn at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit in {} lane {lane}", player.as_str());
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    fn healed_amounts(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    mod c_n3_book_of_heal {
        use super::*;

        #[test]
        fn declares_one_target_any_unit_or_hero_on_either_side_r19_and_runs_one_script_on_both_faces() {
            crate::register_all();
            assert_eq!(ID, BOOK);
            let scripts = script();
            assert_eq!(
                js(&scripts.base.targets),
                json!([{ "kind": "target", "min": 1, "max": 1, "aim": "help", "filter": { "side": "any", "of": ["unit", "hero"] } }])
            );
            // TS `expect(radiant).toBe(base)`: the radiant face is the very same script.
            let (Some(base_cry), Some(radiant_cry)) = (&scripts.base.cry, &scripts.radiant.cry) else {
                panic!("both faces have a Cry");
            };
            assert!(Arc::ptr_eq(base_cry, radiant_cry));
            assert_eq!(js(&scripts.radiant.targets), js(&scripts.base.targets));
        }

        mod base {
            use super::*;

            #[test]
            fn r19_heals_your_own_damaged_unit_never_past_its_max_health() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BOOK, FILLER], "field": [{ "def": MENACE, "damage": 5 }] },
                    "p2": { "hand": [FILLER] },
                }));

                let targets = at(&s, P1, 1);
                s.play(BOOK, json!({ "targets": targets }));

                s.expect_stats(MENACE, json!({ "health": 9, "maxHealth": 9 }));
                assert_eq!(healed_amounts(&s), vec![5]);
            }

            #[test]
            fn r19_heals_an_enemy_unit_9_either_side_is_legal() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BOOK, FILLER] },
                    "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "radiant": true, "damage": 12 }] },
                }));

                let targets = at(&s, P2, 1);
                s.play(BOOK, json!({ "targets": targets }));

                s.expect_stats(MENACE, json!({ "health": 15, "maxHealth": 18 }));
                assert_eq!(healed_amounts(&s), vec![9]);
            }

            #[test]
            fn r19_heals_your_hero_9_with_no_cap_a_hero_at_30_reaches_39() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER] } }));

                s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(P1, 39);
            }

            #[test]
            fn r19_heals_the_enemy_hero_too() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER], "health": 12 } }));

                s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

                s.expect_health(P2, 21);
            }

            #[test]
            fn a_full_health_unit_gains_nothing_no_heal_happens() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "field": [MENACE] }, "p2": { "hand": [FILLER] } }));

                let targets = at(&s, P1, 1);
                s.play(BOOK, json!({ "targets": targets }));

                s.expect_stats(MENACE, json!({ "health": 9, "maxHealth": 9 }));
                assert_eq!(healed_amounts(&s), Vec::<i32>::new());
                s.expect_in_zone(BOOK, "graveyard");
            }

            #[test]
            fn r386_an_upgrade_moves_heal_by_its_declared_step_of_2_it_heals_11() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 10 }, "p2": { "hand": [FILLER] } }));
                step_param(s.card_mut(BOOK), "heal", 1);

                s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(P1, 21);
            }

            #[test]
            fn r386_a_degrade_moves_it_the_other_way_by_2_it_heals_7() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER], "health": 10 }, "p2": { "hand": [FILLER] } }));
                step_param(s.card_mut(BOOK), "heal", -1);

                s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(P1, 17);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn heals_a_target_18_a_hero_at_30_reaches_48() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(P1, 48);
            }

            #[test]
            fn r19_heals_a_unit_18_still_never_past_its_max_health() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": BOOK, "radiant": true }, FILLER],
                        "field": [{ "def": MENACE, "radiant": true, "damage": 17 }, { "def": MENACE, "damage": 8 }],
                    },
                    "p2": { "hand": [FILLER] },
                }));

                let targets = at(&s, P1, 1);
                s.play(BOOK, json!({ "targets": targets }));
                match s.unit(P1, 1) {
                    Some(unit) => s.expect_stats(&unit, json!({ "health": 18, "maxHealth": 18 })),
                    None => s.expect_stats(MENACE, json!({ "health": 18, "maxHealth": 18 })),
                };
                assert_eq!(healed_amounts(&s), vec![17]);
            }

            #[test]
            fn r386_an_upgrade_on_the_radiant_face_steps_from_18_it_heals_20() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }, FILLER], "health": 10 },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(BOOK), "heal", 1);

                s.play(BOOK, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(P1, 30);
            }
        }
    }
}
