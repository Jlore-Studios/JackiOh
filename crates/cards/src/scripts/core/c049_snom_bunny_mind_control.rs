//! #49 Snom Bunny Mind Control (SPEC §8.2): "Steal an enemy Unit.", radiant "Steal an enemy Unit.
//! Make it Radiant."
//!
//! Reading the radiant cell (§8 Conventions): the second sentence adds an effect and keeps the base
//! clause, so the radiant face steals the same Unit and then sets its flag.
//!
//! Unit only since patch v0.3.11 (#1149): the declaration offers the enemy's Units and no Field Spell,
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

use jackioh_engine::prelude::*;

pub const ID: &str = "core-049";

/// "An enemy Unit": the enemy's Units, never a card of the backrow's other types.
fn enemy_unit() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))]
}

/// v0.3.7b's declaration: §6.3's permanent is a Unit, Field Spell, Trap or Field Trap, so both enemy
/// rows were on offer.
fn enemy_permanent() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "backrow"] }))]
}

pub fn script() -> CardScripts {
    scripts_with(enemy_unit)
}

/// R767000: the scripts v0.3.7b and every version before it ran, a steal of any enemy permanent;
/// `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    scripts_with(enemy_permanent)
}

fn scripts_with(targets: fn() -> Vec<TargetDecl>) -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| vec![steal(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        },
        radiant: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| {
                vec![
                    steal(json_as(json!({ "target": { "of": "chosen" } }))),
                    set_radiant(json_as(json!({ "target": { "of": "chosen" } }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// #49 Snom Bunny Mind Control — SPEC §8.2, BUILD M4-T4: "Steal placement per R15; radiant sets the
// flag on the stolen card".
//
// R15 has three clauses, one case each: the same lane when free, the first free zone of the row,
// and "excess remain with the opponent" when the row is full. R12 is asserted alongside every steal:
// `controller` moves and `owner` never does.
//
// The radiant clause is R22/R74; an already Radiant card is untouched (§6.3 Make Radiant). The
// props are #25, #20, #45 (4/3 → 8/6, the damage-and-radiant case), #11, #8, #15 (its Cry would
// summon a Rush Token, which checks "no Cry re-fires"), #41 Sheepish, a face-down Trap, and #14
// Jlockeed's Weapons, a Field Spell: neither is a Unit, so neither is on offer.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const MIND_CONTROL: &str = "core-049";
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
            crate::register_all();
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
            // (4) since patch v0.2.0 (issue #40), out of the harness's 4.
            g.expect_mana(PlayerId::P1, 0);
        }

        #[test]
        fn r15_base_the_first_free_zone_of_the_row_when_the_same_lane_is_taken() {
            crate::register_all();
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
            crate::register_all();
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
        fn units_only_legal_actions_offers_no_enemy_field_spell_or_trap() {
            for radiant in [false, true] {
                crate::register_all();
                let mut g = scenario(json!({
                    "p1": { "hand": [{ "def": MIND_CONTROL, "radiant": radiant }] },
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
        }

        #[test]
        fn r767000_a_game_on_v0_3_7_folds_under_the_kept_script_a_face_down_enemy_trap_is_on_offer() {
            crate::register_all();
            let shipped = crate::shipped_catalog("v0.3.7").expect("v0.3.7 is carried");
            jackioh_engine::with_shipped(Some(shipped), || {
                let mut g = scenario(json!({
                    "p1": { "hand": [MIND_CONTROL] },
                    "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }] },
                }));
                let trap = g.backrow(PlayerId::P2, 2).expect("setup: p2's Sheepish").clone();
                let card = g.card(MIND_CONTROL).clone();
                assert_eq!(offered(g.state(), &card), vec![vec![trap.id.clone()]]);

                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": trap.id }] }));

                assert_eq!(g.card(&trap).controller, PlayerId::P1);
            });
        }

        #[test]
        fn s8_an_enemy_board_with_no_unit_offers_nothing_and_the_spell_fizzles() {
            crate::register_all();
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

        #[test]
        fn r22_r74_radiant_the_stolen_card_becomes_radiant_keeping_its_damage() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_mind_control()] },
                "p2": { "field": [{ "def": DUELIST, "lane": 2, "damage": 2 }] },
            }));
            let prey = g.unit(PlayerId::P2, 2).expect("setup: p2 should hold Deft Duelist").clone();
            g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 3, "health": 1 }));

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert_eq!(g.unit(PlayerId::P1, 2).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert!(g.card(&prey).radiant);
            // R22: the base layer swaps at once (4/3 → 8/6) and the 2 damage stays.
            g.expect_stats(&prey, json!({ "attack": 8, "maxHealth": 6, "health": 4 }));
            assert_eq!(g.card(&prey).damage, 2);
            // The steal moved the card first; `set_radiant` still found it, because it resolves by id.
            g.expect_events(json!(["controlChanged", "radiantSet"]));
        }

        #[test]
        fn r22_radiant_the_flag_fires_no_cry_nothing_is_summoned_alongside_the_steal() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_mind_control()] },
                "p2": { "field": [{ "def": TOKEN_MAKER, "lane": 1 }] },
            }));
            let prey = g.unit(PlayerId::P2, 1).expect("setup: p2 should hold Me and Mr Token").clone();

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert!(g.card(&prey).radiant);
            // Me and Mr Token's Cry summons a Rush Token; setting a flag is not an entry to the field.
            let mine: Vec<String> = [1, 2, 3, 4, 5]
                .into_iter()
                .filter_map(|lane| g.unit(PlayerId::P1, lane).map(|unit| unit.def_id.clone()))
                .collect();
            assert_eq!(mine, vec![prey.def_id.clone()]);
        }

        #[test]
        fn s6_3_radiant_a_card_that_is_already_radiant_is_stolen_and_left_alone() {
            crate::register_all();
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
        fn r81_both_faces_declare_one_enemy_unit_and_the_radiant_face_adds_the_flag() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, MIND_CONTROL);
            assert_eq!(serde_json::to_value(def.type_).unwrap(), json!("Spell"));
            assert_eq!(serde_json::to_value(def.cost).unwrap(), json!(4));
            let scripts = super::super::script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(
                    serde_json::to_value(&face.targets).unwrap(),
                    json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } }])
                );
                assert!(face.modes.is_empty());
            }
        }
    }
}
