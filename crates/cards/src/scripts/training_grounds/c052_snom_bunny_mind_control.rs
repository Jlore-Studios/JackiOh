//! SPEC §8.11 Training Grounds #52 Legacy: Snom Bunny Mind Control,
//! the Legacy card of Core #49 Snom Bunny Mind Control (`core-049`), a card of the held Training Grounds set (R1420,
//! R2194000), built by #2195.
//!
//! The code is a copy of the Core card's at the foundation's commit, not a re-export, so a later Core
//! patch does not move it (R2195003); it is named `Legacy: <the Core card's name>`, tagged Legacy
//! (R2511000), and is its own card, which a deck holds beside the Core card (R2511001). The Core
//! file's notes, as they stood:
//!
//! #49 Snom Bunny Mind Control (SPEC §8.2): "Steal an enemy Unit.", radiant "Steal an enemy Unit.
//! Make it Radiant."
//!
//! Reading the radiant cell (§8 Conventions): the second sentence adds an effect and keeps the base
//! clause, so the radiant face steals the same Unit and then sets its flag.
//!
//! Unit only since patch v0.3.12 (#1149): the declaration offers the enemy's Units and no Field Spell,
//! Trap or Field Trap. It is a DECLARED play-time target in the play action's `targets` (R81),
//! validated by R90, and only the top card of a Stack pile is on offer (R13). An enemy board with no
//! Unit fizzles and the spell still counts as played (§8 Conventions). Through v0.3.7b it stole any
//! enemy permanent, a face-down Trap included; `script_v0_3_7b` keeps that for those versions'
//! replays (R767000).
//!
//! Where the card lands is §6.3 Steal and R15 (`effects/steal.rs`). Control is field-only (R12), so
//! the steal moves `controller` and nothing else: the card keeps its owner, damage, buffs, counters
//! and position (R78 is about leaving).
//!
//! The radiant clause is `set_radiant`, the flag and nothing else (R74): the base stat layer swaps at
//! once, damage and buffs stay and no Cry re-fires (R22). It resolves the selection by instance id,
//! so the steal having moved the card between zones cannot make it miss.
//!
//! The snapshot has no shipped history, so it copies neither `script_v0_3_7b` nor its declaration
//! (R2195003): it steals an enemy Unit only.
//!
//! Since issue #3139 it is its own card in earnest, moving its own way where Core #49 moved another
//! (R2195003): the base face reads "Steal an enemy Unit. Set its stats to 4/4." (`set_stats` on the stolen
//! Unit, the number `SET_TO`, a fixed one: whether 4/4 helps hangs on the Unit picked, params.rs) and the
//! Radiant face, literally, "Steal an enemy card.": the target is any enemy permanent as Core #49's is
//! (R3139002), and it no longer makes the card Radiant. Both effects run in a list, so a steal that finds no
//! free zone (R15) still sets the Unit's stats, as Core #49's Radiant face still makes it Radiant. `script_v0_5_11o`
//! keeps the earlier Unit-only steal, whose Radiant face made it Radiant, for the replays of v0.5.11o and
//! before (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-052";

/// "Set its stats to 4/4": the Attack and Health the stolen Unit is set to (§6.3 Set stats, R2198000).
const SET_TO: i32 = 4;

/// "An enemy Unit": the enemy's Units, never a card of the backrow's other types.
fn enemy_unit() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))]
}

/// "An enemy card" (R3139002): §6.3's permanent, a Unit, Field Spell, Trap or Field Trap of the enemy's; a
/// face-down Trap is offered by its id alone.
fn enemy_permanent() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "backrow"] }))]
}

fn steal_chosen() -> Effect {
    steal(json_as(json!({ "target": { "of": "chosen" } })))
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: enemy_unit(),
            cry: Some(hook(|_ctx| {
                vec![
                    steal_chosen(),
                    set_stats(json_as(json!({ "target": { "of": "chosen" }, "attack": SET_TO, "health": SET_TO }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            targets: enemy_permanent(),
            cry: Some(hook(|_ctx| vec![steal_chosen()])),
            ..Script::default()
        },
    }
}

/// R767000: the scripts v0.5.11o and every version before it ran: a steal of an enemy Unit, whose Radiant
/// face made it Radiant. `KEPT_SCRIPTS` folds those versions' replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: Script {
            targets: enemy_unit(),
            cry: Some(hook(|_ctx| vec![steal_chosen()])),
            ..Script::default()
        },
        radiant: Script {
            targets: enemy_unit(),
            cry: Some(hook(|_ctx| {
                vec![steal_chosen(), set_radiant(json_as(json!({ "target": { "of": "chosen" } })))]
            })),
            ..Script::default()
        },
    }
}

// TG #52 Legacy: Snom Bunny Mind Control — SPEC §8.11, BUILD M13 row TG 52, R15, R12, R22, R74, §6.3,
// R3139002. Steal placement per R15; the base face sets the stolen Unit to 4/4 and the Radiant face steals
// any enemy permanent; props (#25, #20, #45, #11, #8, #15, #41, #14).
#[cfg(test)]
mod tests {
    use super::script_v0_5_11o;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const MIND_CONTROL: &str = "traininggrounds-052";
    const SEVEN_SEVEN: &str = "core-025";
    const POINTMASTER: &str = "core-020";
    const DUELIST: &str = "core-045";
    const TIMMY: &str = "core-011";
    const VANILLA: &str = "core-008";
    const TOKEN_MAKER: &str = "core-015";
    const SHEEPISH: &str = "core-041";
    const WEAPONS: &str = "core-014";

    const P1: PlayerId = PlayerId::P1;

    /// The target lists of every `play` `legal_actions` offers p1 for this card (R81).
    fn offered(state: &GameState, card: &CardInstance) -> Vec<Vec<String>> {
        legal_actions(state, P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id, targets, ..
                } if instance_id == card.id => Some(
                    targets
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|target| match target {
                            Selection::Instance { instance_id } => Some(instance_id),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }

    /// The radiant face in hand, so the radiant text is the one that resolves (§5.2).
    fn radiant_mind_control() -> Value {
        json!({ "def": MIND_CONTROL, "radiant": true })
    }

    mod n49_snom_bunny_mind_control {
        use super::*;

        #[test]
        fn r15_base_the_stolen_unit_lands_in_the_same_lane_when_that_zone_is_free() {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: p2 should hold the 7/7 in lane 3").clone();

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert_eq!(g.unit(PlayerId::P1, 3).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert!(g.unit(PlayerId::P2, 3).is_none());
            // R12: control is a field-only notion; ownership never moves.
            assert_eq!(g.card(&prey).controller, PlayerId::P1);
            assert_eq!(g.card(&prey).owner, PlayerId::P2);
            g.expect_events(json!(["cardPlayed", "controlChanged", "enteredGraveyard"]));
            // It costs (4) (#40), out of the harness's 4.
            g.expect_mana(PlayerId::P1, 0);
        }

        /// Issue #3139: the base face sets the stolen Unit to 4/4, whatever it was, damaged or not.
        #[test]
        fn base_the_stolen_unit_is_set_to_4_4_a_big_one_down_and_a_small_one_up() {
            for (def, damage) in [(SEVEN_SEVEN, 0), (DUELIST, 2), (TOKEN_MAKER, 0)] {
                let mut g = scenario(json!({
                    "p1": { "hand": [MIND_CONTROL] },
                    "p2": { "field": [{ "def": def, "lane": 2, "damage": damage }] },
                }));
                let prey = g.unit(PlayerId::P2, 2).expect("setup: the prey").clone();

                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

                assert_eq!(g.unit(PlayerId::P1, 2).map(|u| u.id.clone()), Some(prey.id.clone()), "{def}");
                g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 4, "health": 4 }));
                assert!(!g.card(&prey).radiant, "{def}");
            }
        }

        #[test]
        fn r15_base_the_first_free_zone_of_the_row_when_the_same_lane_is_taken() {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL], "field": [{ "def": VANILLA, "lane": 3 }] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: p2 should hold the 7/7 in lane 3").clone();

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            // Lane 3 is occupied, so `first_free_zone` scans lanes 1..5 and lane 1 wins.
            assert_eq!(g.unit(PlayerId::P1, 1).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert_eq!(g.unit(PlayerId::P1, 3).map(|u| u.def_id.clone()), Some(VANILLA.to_string()));
            assert!(g.unit(PlayerId::P2, 3).is_none());
        }

        #[test]
        fn r15_base_a_full_row_leaves_the_card_with_the_opponent_and_the_spell_still_resolves() {
            let mut g = scenario(json!({
                "p1": {
                    "hand": [MIND_CONTROL],
                    "field": [
                        { "def": SEVEN_SEVEN, "lane": 1 },
                        { "def": POINTMASTER, "lane": 2 },
                        { "def": DUELIST, "lane": 3 },
                        { "def": TIMMY, "lane": 4 },
                        { "def": VANILLA, "lane": 5 },
                    ],
                },
                "p2": { "field": [{ "def": TOKEN_MAKER, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: p2 should hold the target").clone();

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert_eq!(g.unit(PlayerId::P2, 3).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, PlayerId::P2);
            g.expect_in_zone(MIND_CONTROL, "graveyard");
            assert!(!g.events().iter().any(|event| event.event_type().as_str() == "controlChanged"));
        }

        #[test]
        fn units_only_the_base_face_offers_no_enemy_field_spell_or_trap() {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL] },
                "p2": {
                    "field": [{ "def": VANILLA, "lane": 2 }],
                    "backrow": [{ "def": WEAPONS, "lane": 1 }, { "def": SHEEPISH, "lane": 3 }],
                },
            }));
            let vanilla = g.unit(PlayerId::P2, 2).expect("setup: p2's Vanilla").clone();
            let weapons = g.backrow(PlayerId::P2, 1).expect("setup: p2's Weapons").clone();
            let card = g.card(MIND_CONTROL).clone();

            assert_eq!(offered(g.state(), &card), vec![vec![vanilla.id.clone()]]);
            // Named anyway, the Field Spell is refused (R90): the reducer reads the same declaration.
            let named = json!({ "targets": [{ "pick": "instance", "instanceId": weapons.id }] });
            g.expect_refused_with(|g| g.play(MIND_CONTROL, named.clone()), "not a legal target");
            assert_eq!(g.card(&weapons).controller, PlayerId::P2);
        }

        #[test]
        fn r2195003_the_snapshot_keeps_no_v0_3_7b_script_a_face_down_enemy_trap_is_never_on_offer() {
            let g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL] },
                "p2": { "field": [VANILLA], "backrow": [{ "def": SHEEPISH, "lane": 2 }] },
            }));
            let vanilla = g.card(VANILLA).clone();
            let card = g.card(MIND_CONTROL).clone();
            assert_eq!(offered(g.state(), &card), vec![vec![vanilla.id.clone()]]);
        }

        #[test]
        fn s8_an_enemy_board_with_no_unit_offers_nothing_and_the_spell_fizzles() {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL] },
                "p2": { "backrow": [{ "def": WEAPONS, "lane": 1 }, { "def": SHEEPISH, "lane": 2 }] },
            }));
            let weapons = g.backrow(PlayerId::P2, 1).expect("setup: p2's Weapons").clone();

            g.play(MIND_CONTROL, json!({}));

            g.expect_in_zone(MIND_CONTROL, "graveyard");
            assert_eq!(g.card(&weapons).controller, PlayerId::P2);
            assert!(!g.events().iter().any(|event| event.event_type().as_str() == "controlChanged"));
        }

        /// Issue #3139: "Steal an enemy card." and nothing more: the card is not made Radiant and keeps its
        /// stats, damage included, where the base face sets the Unit to 4/4.
        #[test]
        fn r3139002_radiant_steals_the_card_and_neither_makes_it_radiant_nor_sets_its_stats() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_mind_control()] },
                "p2": { "field": [{ "def": DUELIST, "lane": 2, "damage": 2 }] },
            }));
            let prey = g.unit(PlayerId::P2, 2).expect("setup: p2 should hold Deft Duelist").clone();
            g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 3, "health": 1 }));

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert_eq!(g.unit(PlayerId::P1, 2).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert!(!g.card(&prey).radiant);
            g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 3, "health": 1 }));
            assert_eq!(g.card(&prey).damage, 2);
            assert!(!g.events().iter().any(|event| event.event_type().as_str() == "radiantSet"));
        }

        /// Issue #3139, R3139002: "card" is any enemy permanent on the Radiant face: a Unit, a Field Spell, a
        /// Trap, the face-down one by its id alone (R177). The base face was not offered them.
        #[test]
        fn r3139002_radiant_steals_an_enemy_unit_field_spell_or_face_down_trap() {
            let board = || {
                json!({
                    "p1": { "hand": [radiant_mind_control()] },
                    "p2": {
                        "field": [{ "def": VANILLA, "lane": 2 }],
                        "backrow": [{ "def": WEAPONS, "lane": 1, "faceUp": true }, { "def": SHEEPISH, "lane": 3, "faceUp": false }],
                    },
                })
            };
            let g = scenario(board());
            let card = g.card(MIND_CONTROL).clone();
            let mut want: Vec<String> = [VANILLA, WEAPONS, SHEEPISH].iter().map(|def| g.card(*def).id.clone()).collect();
            let mut offers: Vec<String> = offered(g.state(), &card).into_iter().flatten().collect();
            want.sort();
            offers.sort();
            assert_eq!(offers, want);

            for def in [VANILLA, WEAPONS, SHEEPISH] {
                let mut g = scenario(board());
                let prey = g.card(def).clone();

                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

                assert_eq!(g.card(&prey).controller, PlayerId::P1, "{def}");
                assert_eq!(g.card(&prey).owner, PlayerId::P2, "{def}");
                assert!(!g.card(&prey).radiant, "{def}");
            }
        }

        #[test]
        fn s6_3_radiant_a_card_that_is_already_radiant_is_stolen_and_left_alone() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_mind_control()] },
                "p2": { "field": [{ "def": DUELIST, "lane": 4, "radiant": true }] },
            }));
            let prey = g.unit(PlayerId::P2, 4).expect("setup: p2 should hold a radiant Deft Duelist").clone();

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert_eq!(g.unit(PlayerId::P1, 4).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert!(g.card(&prey).radiant);
            // §6.3 Make Radiant: no effect on a Radiant card, so the flag change emits nothing.
            assert_eq!(
                g.events().iter().filter(|event| event.event_type().as_str() == "radiantSet").count(),
                0
            );
        }

        #[test]
        fn r81_the_base_face_declares_one_enemy_unit_and_the_radiant_face_one_enemy_permanent() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, MIND_CONTROL);
            assert_eq!(serde_json::to_value(def.type_).unwrap(), json!("Spell"));
            assert_eq!(serde_json::to_value(def.cost).unwrap(), json!(4));
            let scripts = super::super::script();
            assert_eq!(
                serde_json::to_value(&scripts.base.targets).unwrap(),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } }])
            );
            assert_eq!(
                serde_json::to_value(&scripts.radiant.targets).unwrap(),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit", "backrow"] } }])
            );
            for face in [&scripts.base, &scripts.radiant] {
                assert!(face.modes.is_empty());
            }
        }
    }

    /// R767000: through v0.5.11o the base face stole an enemy Unit and the Radiant face made it Radiant, the
    /// target a Unit on both.
    #[test]
    fn r767000_the_script_kept_for_v0_5_11o_steals_a_unit_and_the_radiant_face_makes_it_radiant() {
        crate::under_kept(super::ID, script_v0_5_11o(), || {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }], "backrow": [{ "def": WEAPONS, "lane": 1 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: the 7/7").clone();
            let card = g.card(MIND_CONTROL).clone();
            assert_eq!(offered(g.state(), &card), vec![vec![prey.id.clone()]]);
            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
            assert_eq!(g.card(&prey).controller, PlayerId::P1);
            assert!(!g.card(&prey).radiant);
            g.expect_stats(&prey, json!({ "attack": 7, "maxHealth": 7 }));

            let mut g = scenario(json!({
                "p1": { "hand": [radiant_mind_control()] },
                "p2": { "field": [{ "def": DUELIST, "lane": 2, "damage": 2 }], "backrow": [{ "def": WEAPONS, "lane": 1 }] },
            }));
            let prey = g.unit(PlayerId::P2, 2).expect("setup: Deft Duelist").clone();
            let card = g.card(MIND_CONTROL).clone();
            assert_eq!(offered(g.state(), &card), vec![vec![prey.id.clone()]]);
            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
            assert!(g.card(&prey).radiant);
            g.expect_stats(&prey, json!({ "attack": 8, "maxHealth": 6, "health": 4 }));
        });
    }

    /// R2195003, issue #3139: the snapshot is its own copy of Core #49, frozen against its later patches and now
    /// moved by its own: the base face sets the stolen Unit to 4/4 and the Radiant face does not make it Radiant,
    /// where Core #49's base face only steals and its Radiant face makes it Radiant.
    #[test]
    fn r2195003_the_legacy_card_is_its_own_copy_and_no_longer_plays_as_core_049() {
        crate::register_all();
        let game = |id: &str, radiant: bool| {
            let mut g = scenario(json!({
                "p1": { "hand": [{ "def": id, "radiant": radiant }], "mana": 5 },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: the 7/7").clone();
            g.play(id, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
            (g.card(&prey).radiant, g.stats(&prey))
        };
        let (_, legacy_base) = game(MIND_CONTROL, false);
        let (_, core_base) = game("core-049", false);
        assert_eq!((legacy_base.attack, legacy_base.max_health), (4, 4));
        assert_eq!((core_base.attack, core_base.max_health), (7, 7));
        let (legacy_radiant, _) = game(MIND_CONTROL, true);
        let (core_radiant, _) = game("core-049", true);
        assert!(!legacy_radiant);
        assert!(core_radiant);
    }
}
