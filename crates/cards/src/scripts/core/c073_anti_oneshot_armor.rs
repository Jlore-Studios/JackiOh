//! #73 Anti-oneshot Armor (SPEC §8.3): "Your hero can't take more than 5 damage in one instance.
//! Cry: draw 1", radiant "Your hero can't take more than 3 damage in one instance. Cry: draw 2" (§8's
//! cell "Cap 3; Cry: draw 2", R275: the cap is tightened AND the Cry draws twice as many).
//!
//! The cap is not an effect and not an aura: it is step 3 of the §4.4 damage pipeline
//! ("Hero cap: if the target is a hero with Anti-oneshot Armor, clamp to 5 (radiant 3)"), so the card
//! contributes a static flag and the pipeline reads it. `damage.rs hero_damage_cap` walks the cards
//! acting on the player's side for `static_flags.anti_oneshot`, takes `ANTI_ONESHOT_CAP.radiant` when the INSTANCE is
//! radiant and `.base` otherwise, and clamps with the smallest cap on that side. Three consequences
//! this card gets for free and must not re-implement:
//!   - the cap is per damage instance, so two 12-damage hits cost the hero 5 each;
//!   - it is hero-only — `hero_damage_cap` is consulted only for `target.kind === "hero"` — so units
//!     take their full hit;
//!   - the radiant number comes from `jackioh_engine::config`, not from this file, which is why
//!     "Cap 3" needs no radiant-specific code at all. The Cry's draw count is the one number the
//!     two faces differ in here.
//!
//! R18 is likewise the engine's: "lose health" is not damage — no Armor, no Anti-oneshot cap — and
//! `damage.rs lose_health` never calls `hero_damage_cap`. #27 Blood Ridden Glowy Jelly Bean's "you lose
//! 5 health" therefore goes through in full past this card, which the test proves.
//!
//! §5.1 lists Anti-oneshot Armor as the example of "a Field Spell that may have a Cry", and §3.2
//! makes a played Field Spell public, so nothing here touches `face_up` either (`summon_onto` and the
//! play pipeline set it). The Cry fires only when the card is played from hand or cast (R1).

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-073";

/// The cap is read by the pipeline (`damage::hero_damage_cap`) as the card's declared number `cap`
/// (R386), 5 and 3 on the Radiant face, and the Cry's "draw 1" (radiant "draw 2") is the declared
/// number `draw`: both faces run this one script.
fn anti_oneshot_armor() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            anti_oneshot: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = anti_oneshot_armor();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #73 Anti-oneshot Armor — SPEC §8.3, BUILD M4-T4: "A 12 hit becomes 5 (radiant 3), per instance,
// hero only; Cry draws 1 (radiant 2, R275)", plus R18's "lose health bypasses the cap".
//
// The 12-damage hit is a radiant core-002 Bigot (12/2). Its script is a Cry only, and a unit placed
// by a `field` setup never fires one (R1), so the attack is a bare 12-damage instance through the
// §4.4 pipeline with nothing else in it. R18's bypass is #27 Blood Ridden Glowy Jelly Bean's "you
// lose 5 health", read against the RADIANT cap of 3: 5 is below the base cap of 5, so only the
// radiant face can tell a clamp apart from a bypass.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// `scenario(opts)` with the shipped cards registered first (the TS globalSetup's `registerAll()`).
    fn setup(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    use crate::js;

    fn damage_to(s: &Scenario, target_id: &str) -> Vec<i64> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "damage")
            .filter(|event| event["targetId"] == target_id)
            .filter_map(|event| event["amount"].as_i64())
            .collect()
    }

    fn drawn_count(s: &Scenario) -> usize {
        s.events().iter().map(js).filter(|event| event["type"] == "drawn").count()
    }

    mod anti_oneshot_armor_base {
        use super::*;

        #[test]
        fn s4_4_step_3_a_12_damage_hit_on_the_protected_hero_becomes_5() {
            let mut s = setup(json!({
                "seed": "core-073-cap-12",
                "active": "p2",
                "p1": { "backrow": ["core-073"], "hand": ["core-005"] },
                "p2": { "field": [{ "def": "core-002", "radiant": true }], "hand": ["core-005"] },
            }));
            let bigot = s.unit(P2, 1).expect("setup: a radiant Bigot in p2's lane 1");
            s.expect_stats(&bigot, json!({ "attack": 12 }));

            s.attack(&bigot, "hero");

            s.expect_health(P1, 25);
            assert_eq!(damage_to(&s, "hero-p1"), [5]);
        }

        #[test]
        fn the_cap_is_per_damage_instance_so_two_12_hits_cost_5_each() {
            let mut s = setup(json!({
                "seed": "core-073-per-instance",
                "active": "p2",
                "p1": { "backrow": ["core-073"], "hand": ["core-005"] },
                "p2": {
                    "field": [
                        { "def": "core-002", "radiant": true },
                        { "def": "core-002", "radiant": true },
                    ],
                    "hand": ["core-005"],
                },
            }));
            let first = s.unit(P2, 1).expect("setup: a radiant Bigot in p2's lane 1");
            let second = s.unit(P2, 2).expect("setup: a radiant Bigot in p2's lane 2");

            s.attack(&first, "hero").attack(&second, "hero");

            s.expect_health(P1, 20);
            assert_eq!(damage_to(&s, "hero-p1"), [5, 5]);
        }

        #[test]
        fn hero_only_a_unit_on_the_protected_side_takes_the_whole_12() {
            let mut s = setup(json!({
                "seed": "core-073-hero-only",
                "active": "p2",
                // core-019 Midrange Menace is 9/9 with Taunt, so it is the only legal target anyway.
                "p1": { "field": ["core-019"], "backrow": ["core-073"], "hand": ["core-005"] },
                "p2": { "field": [{ "def": "core-002", "radiant": true }], "hand": ["core-005"] },
            }));
            let menace = s.unit(P1, 1).expect("setup: Midrange Menace in p1's lane 1");
            let bigot = s.unit(P2, 1).expect("setup: a radiant Bigot in p2's lane 1");

            s.attack(&bigot, &menace);

            // A capped 5 would have left a 9-health unit alive; the full 12 kills it.
            assert_eq!(damage_to(&s, &menace.id), [12]);
            s.expect_in_zone(&menace, "graveyard").expect_health(P1, 30);
        }

        #[test]
        fn s4_4_puts_armor_step_2_before_the_cap_step_3_so_a_hit_already_under_the_cap_keeps_its_reduction() {
            let mut s = setup(json!({
                "seed": "core-073-armor-order",
                "active": "p2",
                "p1": { "backrow": ["core-073"], "hand": ["core-005"], "armor": 10 },
                "p2": { "field": [{ "def": "core-002", "radiant": true }], "hand": ["core-005"] },
            }));
            let bigot = s.unit(P2, 1).expect("setup: a radiant Bigot in p2's lane 1");

            s.attack(&bigot, "hero");

            // 12 − 10 armor = 2, and min(2, 5) is 2 — not the cap's 5.
            s.expect_health(P1, 28);
            assert_eq!(damage_to(&s, "hero-p1"), [2]);
        }

        #[test]
        fn cry_draw_1() {
            let mut s = setup(json!({
                "seed": "core-073-cry",
                "p1": { "hand": ["core-073", "core-005"], "library": ["core-035", "core-036"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play("core-073", json!({}));

            // Two in hand, one played, one drawn.
            assert_eq!(s.hand(P1).len(), 2);
            assert!(s.hand(P1).iter().any(|card| card.def_id == "core-035"));
            assert_eq!(s.pile(P1, "library").len(), 1);
            s.expect_events(json!(["cardPlayed", "summoned", "drawn", "addedToHand"]));
            // §5.1: a Field Spell with a Cry — it went to the backrow and stayed there.
            // (§3.2 also makes a played Field Spell public, but `reduce.playCard` does not set `faceUp`
            // the way `effects/summon.ts` does; that engine gap is reported, not asserted here.)
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some("core-073".to_string()));
        }

        #[test]
        fn r18_lose_health_is_not_damage_so_the_base_cap_of_5_never_sees_it() {
            let mut s = setup(json!({
                "seed": "core-073-r18-base",
                "p1": { "backrow": ["core-073"], "hand": ["core-027", "core-005"], "library": ["core-035"] },
                "p2": { "hand": ["core-005"] },
            }));

            // #27 Blood Ridden Glowy Jelly Bean: "you lose 5 health".
            s.play("core-027", json!({}));

            s.expect_health(P1, 25).expect_events(json!(["healthLost"]));
            // Not a damage instance at all: nothing for the pipeline to clamp.
            assert!(damage_to(&s, "hero-p1").is_empty());
        }
    }

    mod anti_oneshot_armor_radiant {
        use super::*;

        #[test]
        fn radiant_caps_at_3_the_same_12_damage_hit_becomes_3() {
            let mut s = setup(json!({
                "seed": "core-073-radiant-cap",
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-073", "radiant": true }], "hand": ["core-005"] },
                "p2": { "field": [{ "def": "core-002", "radiant": true }], "hand": ["core-005"] },
            }));
            let bigot = s.unit(P2, 1).expect("setup: a radiant Bigot in p2's lane 1");
            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(true));

            s.attack(&bigot, "hero");

            s.expect_health(P1, 27);
            assert_eq!(damage_to(&s, "hero-p1"), [3]);
        }

        #[test]
        fn radiant_is_still_hero_only_and_still_per_instance() {
            let mut s = setup(json!({
                "seed": "core-073-radiant-scope",
                "active": "p2",
                "p1": { "field": ["core-019"], "backrow": [{ "def": "core-073", "radiant": true }], "hand": ["core-005"] },
                "p2": {
                    "field": [
                        { "def": "core-002", "radiant": true },
                        { "def": "core-002", "radiant": true },
                    ],
                    "hand": ["core-005"],
                },
            }));
            let menace = s.unit(P1, 1).expect("setup: Midrange Menace in p1's lane 1");
            let first = s.unit(P2, 1).expect("setup: a radiant Bigot in p2's lane 1");
            let second = s.unit(P2, 2).expect("setup: a radiant Bigot in p2's lane 2");

            s.attack(&first, &menace);
            assert_eq!(damage_to(&s, &menace.id), [12]);

            // The Taunt unit is gone, so the hero is reachable; two instances would be 3 + 3.
            s.attack(&second, "hero");
            s.expect_health(P1, 27);
            assert_eq!(damage_to(&s, "hero-p1"), [3]);
        }

        #[test]
        fn radiant_cry_draw_2_r275_the_cap_tightens_and_the_cry_doubles() {
            let mut s = setup(json!({
                "seed": "core-073-radiant-cry",
                "p1": {
                    "hand": [{ "def": "core-073", "radiant": true }, "core-005"],
                    "library": ["core-035", "core-036", "core-037"],
                },
                "p2": { "hand": ["core-005"] },
            }));

            s.play("core-073", json!({}));

            // Two in hand, one played, two drawn — the top two, the third left in the library.
            assert_eq!(s.hand(P1).len(), 3);
            let hand: Vec<String> = s.hand(P1).into_iter().map(|card| card.def_id).collect();
            assert!(["core-035", "core-036"].iter().all(|id| hand.iter().any(|held| held == id)));
            let library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.def_id).collect();
            assert_eq!(library, ["core-037"]);
            assert_eq!(drawn_count(&s), 2);
            s.expect_events(json!(["cardPlayed", "summoned", "drawn", "drawn"]));
            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(true));
        }

        #[test]
        fn the_base_cry_still_draws_exactly_1_beside_the_radiants_2() {
            let mut s = setup(json!({
                "seed": "core-073-base-cry-count",
                "p1": { "hand": ["core-073", "core-005"], "library": ["core-035", "core-036", "core-037"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play("core-073", json!({}));

            assert_eq!(drawn_count(&s), 1);
            assert_eq!(s.pile(P1, "library").len(), 2);
        }

        #[test]
        fn r386_an_upgrade_draws_2_and_a_radiant_degrade_1() {
            for (radiant, upgrade, draws) in [(false, true, 2), (true, false, 1)] {
                let mut s = setup(json!({
                    "seed": "core-073-tuned-draw",
                    "p1": { "hand": [{ "def": "core-073", "radiant": radiant }, "core-005"], "library": ["core-035", "core-036", "core-037"] },
                    "p2": { "hand": ["core-005"] },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-073", "draw")
                } else {
                    crate::degrade_number(&mut s, "core-073", "draw")
                };
                assert_eq!(moved, draws);
                s.play("core-073", json!({}));
                assert_eq!(drawn_count(&s), draws as usize);
            }
        }

        #[test]
        fn r386_an_upgrade_lowers_the_cap_to_4_and_a_degrade_raises_it_to_6() {
            for (upgrade, cap) in [(true, 4), (false, 6)] {
                let mut s = setup(json!({
                    "seed": "core-073-tuned-cap",
                    "active": "p2",
                    "p1": { "backrow": ["core-073"], "hand": ["core-005"], "library": ["core-035"] },
                    "p2": { "hand": ["core-016", "core-005"], "field": ["core-019"] },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-073", "cap")
                } else {
                    crate::degrade_number(&mut s, "core-073", "cap")
                };
                assert_eq!(moved, cap);
                // Midrange Menace's 9 into p1's hero, held to the cap.
                s.attack("core-019", "hero");
                s.expect_health(P1, 30 - cap);
            }
        }

        #[test]
        fn r18_a_5_point_lose_health_goes_through_in_full_past_the_radiant_cap_of_3() {
            let mut s = setup(json!({
                "seed": "core-073-r18-radiant",
                "p1": {
                    "backrow": [{ "def": "core-073", "radiant": true }],
                    "hand": ["core-027", "core-005"],
                    "library": ["core-035"],
                },
                "p2": { "hand": ["core-005"] },
            }));

            s.play("core-027", json!({}));

            // Were the cap to apply, this would be 27. R18 says it does not.
            s.expect_health(P1, 25).expect_events(json!(["healthLost"]));
            assert!(damage_to(&s, "hero-p1").is_empty());
        }
    }
}
