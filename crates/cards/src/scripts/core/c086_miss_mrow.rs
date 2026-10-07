//! #86 "Miss" Mrow (SPEC §8.4, R12, R13, R15, R42, R78, R171, R361).
//!
//! Base: "Can't attack. Death: Take control of the Unit that destroyed this." Radiant: "Rush. Death:
//! Take control of the Unit that destroyed this." (patch v0.1.1: the Death used to steal every enemy
//! unit, and the Radiant face used to print Taunt).
//!
//! The Death clause is the same on both faces, so both faces run one Death hook and differ only in
//! the printed face, which is the catalog's: the base face prints the keyword `Can't attack`, which
//! `combat.ts`'s `whyAttackRefused` reads off `unitView(...).keywords`, and the radiant face prints
//! `Rush` (§8 Conventions: a keyword list gives the face's complete list), so it may attack units the
//! turn it lands. Neither needs a line of script.
//!
//! "The Unit that destroyed this" is R42's killer, which R361 makes a card-facing fact: the unit
//! whose hit took Mrow to 0 health (or whose Poisonous hit marked it), read off Mrow's last-known
//! state (R78) — the Death hook runs at §4.5 step 3 on that snapshot. `killerOf` answers it only while
//! the killer is a Unit acting on the field, so a destroy effect, a Tribute, a Spell's damage, a
//! killer that died in the same combat or one dormant under a Stack (R13) gives nothing to take.
//!
//! The take is one `steal`, which is §6.3's Steal: R15 places it (the same lane on Mrow's
//! controller's side when free, else the first free zone; with none, it stays), it keeps its damage
//! and buffs (R78), it has entered its new controller's side this turn (R171), and a killer its
//! controller already controls — a unit of Mrow's own side — is left where it is. `ctx.controller`
//! is the side Mrow was on as it died, so a stolen Mrow takes the killer for whoever controlled it.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-086";

/// "Death: Take control of the Unit that destroyed this" — identical on both faces.
fn death() -> Hook {
    hook(|ctx| {
        let killer = killer_of(&ctx.state, ctx.self_.as_ref()).map(|killer| killer.id.clone());
        match killer {
            None => vec![],
            Some(instance_id) => vec![steal(json_as(json!({ "instanceId": instance_id })))],
        }
    })
}

pub fn script() -> CardScripts {
    let death = death();
    let base = Script {
        death: Some(death.clone()),
        ..Script::default()
    };
    let radiant = Script {
        death: Some(death),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #86 "Miss" Mrow (SPEC §8.4, BUILD M4-T4 row 86): "Cannot attack; Death takes control of the Unit
// that destroyed it (R42's killer, R361), placed per R15, and nothing when no Unit on the field
// destroyed it; radiant has Rush instead". Patch v0.1.1 replaced "Death: steal all enemy units" and
// the radiant face's Taunt.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MROW: &str = "core-086";

    // Field fixtures, chosen because every one of them has a Cry and nothing else: a unit placed by the
    // harness's `field` setup never fires its Cry, so these are inert boards.
    const GARY: &str = "core-004"; // 1/1
    const FELINORS: &str = "core-012"; // 3/4, kills a 1/1 Mrow and survives the 1 back
    const POSTDOC: &str = "core-061"; // 2/4
    const RENO: &str = "core-053"; // 4/6
    const SORCERER: &str = "core-068"; // 5/5; its Cry deals 4 damage to a target
    const STRAAZA: &str = "core-054"; // 8/8
    const HIT_JOB: &str = "core-016"; // Spell: destroy target Unit
    const LUNAR_ECLIPSE: &str = "core-035"; // Spell: deal 3 damage to a target

    // §2.5: a turn with nothing but `endTurn` left auto-ends and cascades into the next turn's draw.
    // One always-playable card in each hand keeps every scenario on the turn it started on.
    const FILLER: &str = "core-005";

    const SEED: &str = "mrow-86";

    /// An engine value as the JSON the TS test compares it with.
    fn js<T: serde::Serialize>(v: &T) -> Value {
        serde_json::to_value(v).expect("serialises")
    }

    fn control_changed(s: &Scenario) -> bool {
        s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::ControlChanged { .. }))
    }

    mod n86_miss_mrow_base {
        use super::*;

        #[test]
        fn cannot_attack_the_printed_keyword_is_the_catalog_s_and_combat_refuses_both_targets() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [FILLER], "field": [MROW] },
                "p2": { "hand": [FILLER], "field": [GARY] },
            }));

            assert_eq!(
                js(&keywords_of(s.state(), s.card(MROW))),
                json!([{ "kind": "Can't attack" }])
            );
            s.expect_refused_with(
                |s| {
                    s.attack(MROW, "hero")
                },
                "cannot attack",
            );
            s.expect_refused_with(
                |s| {
                    s.attack(MROW, GARY)
                },
                "cannot attack",
            );
        }

        #[test]
        fn r361_death_takes_control_of_the_unit_that_destroyed_it_into_the_same_lane_when_free_r15_and_only_that_one() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [{ "def": MROW, "lane": 3 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }, { "def": GARY, "lane": 2 }] },
            }));

            s.attack(FELINORS, MROW);

            s.expect_in_zone(MROW, "graveyard")
                .expect_events(json!(["destroyed", "controlChanged"]));
            // R15: p1's lane 1 was free, so the killer kept its lane.
            assert_eq!(s.unit(P1, 1).as_ref().map(|card| card.def_id.as_str()), Some(FELINORS));
            assert!(s.unit(P2, 1).is_none());
            // R12: control, never ownership; R78: it never left the field, so its damage came along.
            assert_eq!(s.unit(P1, 1).map(|card| card.owner), Some(P2));
            assert_eq!(s.unit(P1, 1).map(|card| card.controller), Some(P1));
            assert_eq!(s.unit(P1, 1).map(|card| card.damage), Some(1));
            // The other enemy unit had no part in it and stays with p2.
            assert_eq!(s.unit(P2, 2).as_ref().map(|card| card.def_id.as_str()), Some(GARY));
            assert_eq!(s.unit(P2, 2).map(|card| card.controller), Some(P2));
        }

        #[test]
        fn r361_falls_back_to_the_first_free_zone_which_mrow_s_own_death_has_freed_r15() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": {
                    "hand": [FILLER],
                    "field": [
                        { "def": MROW, "lane": 1 },
                        { "def": RENO, "lane": 2 },
                        { "def": STRAAZA, "lane": 3 },
                        { "def": SORCERER, "lane": 4 },
                        { "def": GARY, "lane": 5 },
                    ],
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": GARY, "lane": 1 }, { "def": FELINORS, "lane": 2 }, { "def": POSTDOC, "lane": 3 }],
                },
            }));

            s.attack(FELINORS, MROW);

            assert_eq!(s.unit(P1, 1).as_ref().map(|card| card.def_id.as_str()), Some(FELINORS));
            assert_eq!(s.unit(P1, 1).map(|card| card.owner), Some(P2));
            assert!(s.unit(P2, 2).is_none());
            assert_eq!(s.unit(P2, 1).map(|card| card.controller), Some(P2));
            assert_eq!(s.unit(P2, 3).map(|card| card.controller), Some(P2));
        }

        #[test]
        fn r361_a_unit_whose_cry_dealt_the_lethal_damage_destroyed_it_so_mrow_takes_that_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [{ "def": MROW, "lane": 1 }] },
                "p2": { "hand": [SORCERER, FILLER] },
            }));

            let mrow = s.card(MROW).id.clone();
            s.play(
                SORCERER,
                json!({ "zone": 2, "targets": [{ "pick": "instance", "instanceId": mrow }] }),
            );

            s.expect_in_zone(MROW, "graveyard");
            assert_eq!(s.unit(P1, 2).as_ref().map(|card| card.def_id.as_str()), Some(SORCERER));
            assert_eq!(s.unit(P1, 2).map(|card| card.owner), Some(P2));
        }

        #[test]
        fn r361_takes_nothing_when_the_killer_died_in_the_same_combat_r78() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [{ "def": MROW, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }, { "def": FELINORS, "lane": 2 }] },
            }));

            s.attack(GARY, MROW);

            s.expect_in_zone(MROW, "graveyard");
            s.expect_in_zone(GARY, "graveyard");
            assert!(!control_changed(&s));
            assert_eq!(s.unit(P2, 2).map(|card| card.controller), Some(P2));
        }

        #[test]
        fn r361_takes_nothing_when_a_spell_destroyed_it_or_a_spell_s_damage_killed_it_r42() {
            crate::register_all();
            for spell in [HIT_JOB, LUNAR_ECLIPSE] {
                let mut s = scenario(json!({
                    "seed": SEED,
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [{ "def": MROW, "lane": 1 }] },
                    "p2": { "hand": [spell, FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
                }));

                let mrow = s.card(MROW).id.clone();
                s.play(spell, json!({ "targets": [{ "pick": "instance", "instanceId": mrow }] }));

                s.expect_in_zone(MROW, "graveyard");
                assert!(!control_changed(&s));
                assert_eq!(s.unit(P2, 1).map(|card| card.controller), Some(P2));
            }
        }

        #[test]
        fn r361_with_no_free_zone_the_killer_stays_with_its_controller_r15() {
            crate::register_all();
            // Mrow sits on top of a pile in lane 5, so her death leaves that lane with the Gary beneath her
            // and p1's row stays full (§3.2).
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": {
                    "hand": [FILLER],
                    "field": [
                        { "def": RENO, "lane": 1 },
                        { "def": RENO, "lane": 2 },
                        { "def": STRAAZA, "lane": 3 },
                        { "def": SORCERER, "lane": 4 },
                        { "def": GARY, "lane": 5 },
                        { "def": MROW, "lane": 5, "stack": true },
                    ],
                },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));

            s.attack(FELINORS, MROW);

            s.expect_in_zone(MROW, "graveyard");
            assert_eq!(s.unit(P1, 5).as_ref().map(|card| card.def_id.as_str()), Some(GARY));
            assert_eq!(s.unit(P2, 1).as_ref().map(|card| card.def_id.as_str()), Some(FELINORS));
            assert_eq!(s.unit(P2, 1).map(|card| card.controller), Some(P2));
        }
    }

    mod n86_miss_mrow_radiant {
        use super::*;

        #[test]
        fn patch_v0_1_1_the_radiant_face_prints_rush_alone_so_it_may_attack_a_unit_the_turn_it_lands() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": MROW, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [GARY] },
            }));

            s.play(MROW, json!({ "zone": 1 }));
            assert_eq!(js(&keywords_of(s.state(), s.card(MROW))), json!([{ "kind": "Rush" }]));
            s.expect_stats(MROW, json!({ "attack": 2, "maxHealth": 2 }));
            // Rush: units only on the turn it lands (§6.1).
            s.expect_refused(|s| {
                s.attack(MROW, "hero")
            });
            s.attack(MROW, GARY);
            s.expect_in_zone(GARY, "graveyard");
        }

        #[test]
        fn the_radiant_face_has_no_taunt_an_enemy_attack_may_go_past_it_to_the_hero() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [{ "def": MROW, "radiant": true, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));

            s.attack(FELINORS, "hero");
            s.expect_health(P1, 27);
        }

        #[test]
        fn r361_radiant_the_same_death_takes_the_unit_whose_strike_back_killed_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [FILLER], "field": [{ "def": MROW, "radiant": true, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": SORCERER, "lane": 1 }, { "def": GARY, "lane": 2 }] },
            }));

            // The radiant 2/2 attacks a 5/5: it deals 2, takes 5 and dies, and the 5/5 destroyed it.
            s.attack(MROW, SORCERER);

            s.expect_in_zone(MROW, "graveyard");
            assert_eq!(s.unit(P1, 1).as_ref().map(|card| card.def_id.as_str()), Some(SORCERER));
            assert_eq!(s.unit(P1, 1).map(|card| card.damage), Some(2));
            assert_eq!(s.unit(P1, 1).map(|card| card.owner), Some(P2));
            assert_eq!(s.unit(P2, 2).as_ref().map(|card| card.def_id.as_str()), Some(GARY));
        }
    }
}
