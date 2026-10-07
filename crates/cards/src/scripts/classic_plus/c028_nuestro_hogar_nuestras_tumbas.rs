//! C+ #28 Nuestro hogar, nuestras tumbas (SPEC §8.7 row 28; §4.5, §6.1, R8, R19, R386). (2) Unit,
//! Common, 3/4 → 6/8.
//!   Base:    "Taunt, Reborn / Death: Heal your hero {heal}." — heal 3
//!   Radiant: "Taunt, Reborn, Divine Shield / Death: Heal your hero {heal}." — heal 8
//!   Engine:  "Death fires on both deaths of a Reborn unit (§4.5), so it heals twice. Tunes: heal 3 ↑."
//!
//! The keywords are the catalog's (Taunt, Reborn; Divine Shield on the Radiant face), which the layers
//! read off the face. The Death is §4.5 step 3's hook, which runs on every death, the first one that
//! Reborn answers included (R8), and never on an exile or a bounce, which are not deaths. A hero's heal
//! has no cap (R19). The amount is the declared number `heal` (R386), read off the dying card's own
//! face and tuning through `param`, so both faces run one script.

use jackioh_engine::effects::heal;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-028";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 8 is its declared `heal`, and its Divine Shield is catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #28 Nuestro hogar, nuestras tumbas — SPEC §8.7 row 28, BUILD M9 Classic+ row C+ 28: "Taunt,
// Reborn; Death heals your hero 3 on both deaths (R8), past 30 allowed; exile or a bounce heals
// nothing; the heal reads through `param()`; radiant Divine Shield too, heal 8".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HOGAR: &str = "classicplus-028";
    /// #19 Midrange Menace, 9/9 Taunt: an attacker that kills the 3/4 (and the 6/8) in one hit.
    const MENACE: &str = "core-019";
    /// #17 Flood: "Bounce all Units".
    const FLOOD: &str = "core-017";
    /// #100 Ceaseless Void: "Cry: Exile all other permanents" (played here at a `costOverride` of 0).
    const VOID: &str = "core-100";
    /// A card that keeps a hand from auto-ending the turn (§2.5).
    const FILLER: &str = "core-005";

    fn keywords(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card).keywords.iter().map(|keyword| keyword.kind().as_str().to_string()).collect()
    }

    fn healed(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// TS `s.unit(seat, lane) ?? fallback`: the unit's id, or the fallback reference.
    fn unit_or(s: &Scenario, seat: PlayerId, lane: i32, fallback: &str) -> String {
        s.unit(seat, lane).map(|card| card.id).unwrap_or_else(|| fallback.to_string())
    }

    /// p1's two Menaces attack p2's #28 twice: its first death and its Reborn body's.
    fn kill_twice(radiant_face: bool, health: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [MENACE, MENACE] },
            "p2": { "hand": [FILLER], "field": [{ "def": HOGAR, "radiant": radiant_face }], "health": health },
        }));
        let (first, second) = match (s.unit(P1, 1), s.unit(P1, 2)) {
            (Some(first), Some(second)) => (first, second),
            _ => panic!("two Menaces"),
        };
        s.attack(&first, HOGAR);
        if radiant_face {
            s.attack(&second, HOGAR); // the Divine Shield takes the first hit
        }
        s
    }

    mod c_n28_nuestro_hogar_nuestras_tumbas {
        use super::*;

        #[test]
        fn is_a_3_4_taunt_reborn_whose_two_faces_run_one_script() {
            crate::register_all();
            assert_eq!(ID, HOGAR);
            assert_eq!(crate::card_def(ID).id, HOGAR);
            let scripts = script();
            // TS `expect(radiant).toBe(base)`: the Radiant face is the base script itself.
            assert!(std::sync::Arc::ptr_eq(
                scripts.base.death.as_ref().unwrap(),
                scripts.radiant.death.as_ref().unwrap()
            ));
            let mut s = scenario(json!({ "p1": { "field": [HOGAR] } }));
            s.expect_stats(HOGAR, json!({ "attack": 3, "health": 4, "maxHealth": 4 }));
            let mut kinds = keywords(&s, HOGAR);
            kinds.sort();
            assert_eq!(kinds, ["Reborn", "Taunt"]);
        }

        mod base {
            use super::*;

            #[test]
            fn r8_its_death_heals_your_hero_3_on_the_first_death_which_reborn_answers() {
                crate::register_all();
                let mut s = kill_twice(false, 20);
                s.expect_health(P2, 23);
                assert_eq!(healed(&s), [3]);
                // Reborn brought it back at 1 health (§6.1).
                let body = unit_or(&s, P2, 1, HOGAR);
                s.expect_stats(&body, json!({ "health": 1 }));
            }

            #[test]
            fn r8_and_heals_3_again_when_the_reborn_body_dies() {
                crate::register_all();
                let mut s = kill_twice(false, 20);
                let second = s.unit(P1, 2).expect("a second Menace");
                let body = unit_or(&s, P2, 1, HOGAR);
                s.attack(&second, &body);
                s.expect_health(P2, 26);
                assert_eq!(healed(&s), [3, 3]);
                s.expect_in_zone(HOGAR, "graveyard");
            }

            #[test]
            fn r19_heals_your_hero_past_30() {
                crate::register_all();
                let mut s = kill_twice(false, 30);
                s.expect_health(P2, 33);
            }

            #[test]
            fn s4_5_a_bounce_is_no_death_flood_returns_it_and_nothing_is_healed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FLOOD, FILLER], "mana": 5 },
                    "p2": { "hand": [FILLER], "field": [HOGAR], "health": 20 },
                }));
                s.play(FLOOD, json!({}));
                s.expect_in_zone(HOGAR, "hand");
                s.expect_health(P2, 20);
                assert_eq!(healed(&s), Vec::<i32>::new());
            }

            #[test]
            fn s4_5_an_exile_is_no_death_ceaseless_void_exiles_it_and_nothing_is_healed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": VOID, "costOverride": 0 }, FILLER] },
                    "p2": { "hand": [FILLER], "field": [HOGAR], "health": 20 },
                }));
                s.play(VOID, json!({}));
                s.expect_in_zone(HOGAR, "exile");
                s.expect_health(P2, 20);
                assert_eq!(healed(&s), Vec::<i32>::new());
            }

            #[test]
            fn r386_an_upgrade_moves_heal_by_1_it_heals_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [MENACE] },
                    "p2": { "hand": [FILLER], "field": [HOGAR], "health": 20 },
                }));
                step_param(s.card_mut(HOGAR), "heal", 1);
                let attacker = unit_or(&s, P1, 1, MENACE);
                s.attack(&attacker, HOGAR);
                s.expect_health(P2, 24);
            }

            #[test]
            fn r386_a_degrade_moves_it_the_other_way_it_heals_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [MENACE] },
                    "p2": { "hand": [FILLER], "field": [HOGAR], "health": 20 },
                }));
                step_param(s.card_mut(HOGAR), "heal", -1);
                let attacker = unit_or(&s, P1, 1, MENACE);
                s.attack(&attacker, HOGAR);
                s.expect_health(P2, 22);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_6_8_with_taunt_reborn_and_divine_shield() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [{ "def": HOGAR, "radiant": true }] } }));
                s.expect_stats(HOGAR, json!({ "attack": 6, "health": 8, "maxHealth": 8 }));
                let mut kinds = keywords(&s, HOGAR);
                kinds.sort();
                assert_eq!(kinds, ["Divine Shield", "Reborn", "Taunt"]);
            }

            #[test]
            fn r8_the_divine_shield_takes_the_first_hit_its_death_heals_your_hero_8() {
                crate::register_all();
                let mut s = kill_twice(true, 20);
                s.expect_health(P2, 28);
                assert_eq!(healed(&s), [8]);
            }

            #[test]
            fn r386_an_upgrade_steps_from_8_it_heals_9() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [MENACE, MENACE] },
                    "p2": { "hand": [FILLER], "field": [{ "def": HOGAR, "radiant": true }], "health": 20 },
                }));
                step_param(s.card_mut(HOGAR), "heal", 1);
                let first = unit_or(&s, P1, 1, MENACE);
                s.attack(&first, HOGAR);
                let second = unit_or(&s, P1, 2, MENACE);
                s.attack(&second, HOGAR);
                s.expect_health(P2, 29);
            }
        }
    }
}
