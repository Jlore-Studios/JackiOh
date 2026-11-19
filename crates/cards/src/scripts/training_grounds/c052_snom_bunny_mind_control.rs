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
//! (R2195003). Since issue #3996 it costs (3) and both faces read "Steal an enemy Unit. Next turn, lose
//! ({loss}) mana.", 2 on the base face and 1 on the Radiant: the target is an enemy Unit, and the loss is
//! `next_turn_mana` on its caster (R1848004), their next refresh giving their max mana less it, never below
//! 0, a badge both seats read. Once the Spell resolves with its target, the loss is paid whatever the
//! steal does (R3996101): both effects run in a list, so a steal that finds no free zone (R15), which
//! destroys the Unit instead (R4205000), still costs the mana, as Hearthstone's Overload is locked in as its card is played. With no enemy Unit to target the
//! Spell fizzles whole (§8's Conventions), and nothing is lost.
//!
//! Kept for replays (R767000): from issue #3139 through v0.6.5b the base face read "Steal an enemy Unit.
//! Set its stats to 2/2." (`set_stats` on the stolen Unit, a fixed number; 4/4 through v0.6.4c, issue
//! #3951) and the Radiant face, literally, "Steal an enemy card.": any enemy permanent as Core #49's is
//! (R3139002), not made Radiant (`script_v0_6_5b`, `script_v0_6_4c`); `script_v0_5_11o` keeps the earlier
//! Unit-only steal, whose Radiant face made it Radiant, for the replays of v0.5.11r and before.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-052";

/// From issue #3139 through v0.6.5b: "Set its stats to 2/2", the Attack and Health the stolen Unit was set
/// to (§6.3 Set stats, R2198000).
const SET_TO_V0_6_5B: i32 = 2;
/// Through v0.6.4c (issue #3951): "Set its stats to 4/4".
const SET_TO_V0_6_4C: i32 = 4;

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

/// Issue #3996, both faces: "Steal an enemy Unit. Next turn, lose ({loss}) mana." (R1848004, R3996101).
fn steal_and_lose() -> Script {
    Script {
        targets: enemy_unit(),
        cry: Some(hook(|ctx| {
            vec![
                steal_chosen(),
                next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "loss") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: steal_and_lose(),
        radiant: steal_and_lose(),
    }
}

/// From issue #3139: the base face sets the stolen Unit to `set_to`/`set_to`, and the Radiant face steals
/// any enemy permanent.
fn mind_control(set_to: i32) -> CardScripts {
    CardScripts {
        base: Script {
            targets: enemy_unit(),
            cry: Some(hook(move |_ctx| {
                vec![
                    steal_chosen(),
                    set_stats(json_as(json!({ "target": { "of": "chosen" }, "attack": set_to, "health": set_to }))),
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

/// R767000: the scripts after v0.6.4c through v0.6.5b, setting the stolen Unit to 2/2 (issue #3996).
/// `KEPT_SCRIPTS` folds those versions' replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    mind_control(SET_TO_V0_6_5B)
}

/// R767000: the scripts v0.5.12 to v0.6.4c ran, setting the stolen Unit to 4/4 (issue #3951).
/// `KEPT_SCRIPTS` folds those versions' replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    mind_control(SET_TO_V0_6_4C)
}

/// R767000: the scripts v0.5.11r and every version before it ran: a steal of an enemy Unit, whose Radiant
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
// R1848004, R3996101. Steal placement per R15; both faces steal an enemy Unit and cost their caster mana
// at the next refresh, 2 on the base face and 1 on the Radiant; props (#25, #20, #45, #11, #8, #15, #41, #14).
#[cfg(test)]
mod tests {
    use super::{script_v0_5_11o, script_v0_6_4c, script_v0_6_5b};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

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
        plays_in(state, P1, &card.id)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { targets, .. } => Some(pick::instance_ids(&targets.unwrap_or_default())),
                _ => None,
            })
            .collect()
    }

    /// The face of the card in hand, so that face's text is the one that resolves (§5.2).
    fn mind_control(radiant: bool) -> Value {
        json!({ "def": MIND_CONTROL, "radiant": radiant })
    }

    /// The radiant face in hand.
    fn radiant_mind_control() -> Value {
        mind_control(true)
    }

    /// What p1's next refresh is changed by (§6.3 Mana, R1848004).
    fn next_refresh(g: &Scenario) -> i32 {
        g.state().players[P1].mana.next_turn_mod
    }

    /// The mana each face costs its caster at their next refresh.
    fn loss(radiant: bool) -> i32 {
        if radiant { 1 } else { 2 }
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
            // It costs (3) (issue #3996), out of the harness's 4.
            g.expect_mana(PlayerId::P1, 1);
        }

        /// Issue #3996: the stolen Unit keeps its stats, damage included, on both faces; the 2/2 is gone.
        #[test]
        fn the_stolen_unit_keeps_its_stats_on_both_faces() {
            for radiant in [false, true] {
                let mut g = scenario(json!({
                    "p1": { "hand": [mind_control(radiant)] },
                    "p2": { "field": [{ "def": DUELIST, "lane": 2, "damage": 2 }] },
                }));
                let prey = g.unit(PlayerId::P2, 2).expect("setup: p2 should hold Deft Duelist").clone();
                g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 3, "health": 1 }));

                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

                assert_eq!(g.unit(PlayerId::P1, 2).map(|u| u.id.clone()), Some(prey.id.clone()));
                g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 3, "health": 1 }));
                assert!(!g.card(&prey).radiant, "radiant {radiant}");
                assert!(!g.events().iter().any(|event| event.event_type().as_str() == "radiantSet"));
            }
        }

        /// Issue #3996, R1848004: "Next turn, lose (2) mana" (Radiant: 1) is its caster's next refresh,
        /// a badge until then, and the refresh after it is whole again.
        #[test]
        fn r3996101_next_turn_its_caster_has_2_mana_less_and_1_on_the_radiant_face() {
            for radiant in [false, true] {
                let mut g = scenario(json!({
                    "p1": { "hand": [mind_control(radiant), "core-005"], "library": filler(4) },
                    "p2": { "hand": ["core-005"], "field": [{ "def": VANILLA, "lane": 1 }], "library": filler(4) },
                }));
                let prey = g.unit(PlayerId::P2, 1).expect("setup: p2's Vanilla").clone();
                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
                assert_eq!(next_refresh(&g), -loss(radiant), "radiant {radiant}");
                g.end_turn().end_turn();
                assert_eq!(g.state().active, P1);
                let mana = g.state().players[P1].mana;
                assert_eq!(mana.current, mana.max - loss(radiant), "radiant {radiant}");
                assert_eq!(next_refresh(&g), 0);
                g.end_turn().end_turn();
                let mana = g.state().players[P1].mana;
                assert_eq!(mana.current, mana.max, "one turn only");
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

        /// R3996101, R4205000: a full row destroys the card, and the mana is lost all the same.
        #[test]
        fn r15_r3996101_r4205000_a_full_row_destroys_the_card_and_the_mana_is_still_lost() {
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

            assert_eq!(g.unit(PlayerId::P2, 3), None);
            g.expect_in_zone(&prey, "graveyard");
            assert!(g.events().iter().any(
                |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == prey.id)
            ));
            g.expect_in_zone(MIND_CONTROL, "graveyard");
            assert!(!g.events().iter().any(|event| event.event_type().as_str() == "controlChanged"));
            assert_eq!(next_refresh(&g), -2);
        }

        /// Issue #3996: "an enemy Unit" on both faces, so neither offers an enemy Field Spell or Trap.
        #[test]
        fn units_only_neither_face_offers_an_enemy_field_spell_or_trap() {
            for radiant in [false, true] {
                let mut g = scenario(json!({
                    "p1": { "hand": [mind_control(radiant)] },
                    "p2": {
                        "field": [{ "def": VANILLA, "lane": 2 }],
                        "backrow": [{ "def": WEAPONS, "lane": 1 }, { "def": SHEEPISH, "lane": 3 }],
                    },
                }));
                let vanilla = g.unit(PlayerId::P2, 2).expect("setup: p2's Vanilla").clone();
                let weapons = g.backrow(PlayerId::P2, 1).expect("setup: p2's Weapons").clone();
                let card = g.card(MIND_CONTROL).clone();

                assert_eq!(offered(g.state(), &card), vec![vec![vanilla.id.clone()]], "radiant {radiant}");
                // Named anyway, the Field Spell is refused (R90): the reducer reads the same declaration.
                let named = json!({ "targets": [{ "pick": "instance", "instanceId": weapons.id }] });
                g.expect_refused_with(|g| g.play(MIND_CONTROL, named.clone()), "not a legal target");
                assert_eq!(g.card(&weapons).controller, PlayerId::P2);
            }
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

        /// §8's Conventions, R3996101: an enemy board with no Unit offers nothing, and a Spell whose target
        /// set is empty fizzles whole, though it still counts as played: no mana is lost.
        #[test]
        fn s8_r3996101_an_enemy_board_with_no_unit_offers_nothing_and_the_spell_fizzles_whole_losing_no_mana() {
            for radiant in [false, true] {
                let mut g = scenario(json!({
                    "p1": { "hand": [mind_control(radiant)] },
                    "p2": { "backrow": [{ "def": WEAPONS, "lane": 1 }, { "def": SHEEPISH, "lane": 2 }] },
                }));
                let weapons = g.backrow(PlayerId::P2, 1).expect("setup: p2's Weapons").clone();

                g.play(MIND_CONTROL, json!({}));

                g.expect_in_zone(MIND_CONTROL, "graveyard");
                assert_eq!(g.card(&weapons).controller, PlayerId::P2);
                assert!(!g.events().iter().any(|event| event.event_type().as_str() == "controlChanged"));
                assert_eq!(next_refresh(&g), 0, "radiant {radiant}");
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
        fn r81_both_faces_declare_one_enemy_unit() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, MIND_CONTROL);
            assert_eq!(serde_json::to_value(def.type_).unwrap(), json!("Spell"));
            assert_eq!(serde_json::to_value(def.cost).unwrap(), json!(3));
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

    /// R767000, R3139002: after v0.6.4c through v0.6.5b the base face set the stolen Unit to 2/2 and the
    /// Radiant face stole any enemy permanent, a face-down Trap by its id alone (R177), not making it
    /// Radiant nor setting its stats; neither cost mana (issue #3996).
    #[test]
    fn r767000_r3139002_the_script_kept_for_v0_6_5b_sets_a_unit_to_2_2_and_its_radiant_face_steals_any_enemy_permanent() {
        crate::under_kept(super::ID, script_v0_6_5b(), || {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: the 7/7").clone();
            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
            assert_eq!(g.card(&prey).controller, PlayerId::P1);
            g.expect_stats(&prey, json!({ "attack": 2, "maxHealth": 2, "health": 2 }));
            assert_eq!(next_refresh(&g), 0);

            let board = || {
                json!({
                    "p1": { "hand": [radiant_mind_control()] },
                    "p2": {
                        "field": [{ "def": DUELIST, "lane": 2, "damage": 2 }],
                        "backrow": [{ "def": WEAPONS, "lane": 1, "faceUp": true }, { "def": SHEEPISH, "lane": 3, "faceUp": false }],
                    },
                })
            };
            let g = scenario(board());
            let card = g.card(MIND_CONTROL).clone();
            let mut want: Vec<String> = [DUELIST, WEAPONS, SHEEPISH].iter().map(|def| g.card(*def).id.clone()).collect();
            let mut offers: Vec<String> = offered(g.state(), &card).into_iter().flatten().collect();
            want.sort();
            offers.sort();
            assert_eq!(offers, want);
            for def in [DUELIST, WEAPONS, SHEEPISH] {
                let mut g = scenario(board());
                let prey = g.card(def).clone();
                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
                assert_eq!(g.card(&prey).controller, PlayerId::P1, "{def}");
                assert_eq!(g.card(&prey).owner, PlayerId::P2, "{def}");
                assert!(!g.card(&prey).radiant, "{def}");
                assert_eq!(next_refresh(&g), 0, "{def}");
            }
            let duelist = {
                let mut g = scenario(board());
                let prey = g.card(DUELIST).clone();
                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
                g.stats(&prey)
            };
            assert_eq!((duelist.attack, duelist.max_health), (4, 3));
        });
    }

    /// R767000: from v0.5.12 through v0.6.4c the base face set the stolen Unit to 4/4 (issue #3951).
    #[test]
    fn r767000_the_script_kept_for_v0_6_4c_sets_the_stolen_unit_to_4_4() {
        crate::under_kept(super::ID, script_v0_6_4c(), || {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: the 7/7").clone();
            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
            assert_eq!(g.card(&prey).controller, PlayerId::P1);
            g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 4, "health": 4 }));
        });
    }

    /// R767000: through v0.5.11r the base face stole an enemy Unit and the Radiant face made it Radiant, the
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

    /// R2195003, issues #3139 and #3996: the snapshot is its own copy of Core #49, frozen against its later
    /// patches and moved by its own: its faces cost mana next turn and its Radiant face does not make the Unit
    /// Radiant, where Core #49's base face only steals and its Radiant face makes it Radiant.
    #[test]
    fn r2195003_the_legacy_card_is_its_own_copy_and_no_longer_plays_as_core_049() {
        crate::register_all();
        let game = |id: &str, radiant: bool| {
            let mut g = scenario(json!({
                "p1": { "hand": [{ "def": id, "radiant": radiant }], "mana": 6 },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: the 7/7").clone();
            g.play(id, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
            (g.card(&prey).radiant, next_refresh(&g))
        };
        assert_eq!(game(MIND_CONTROL, false), (false, -2));
        assert_eq!(game("core-049", false), (false, 0));
        assert_eq!(game(MIND_CONTROL, true), (false, -1));
        assert_eq!(game("core-049", true), (true, 0));
    }
}
