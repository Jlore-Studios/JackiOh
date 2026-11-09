//! #49 Snom Bunny Mind Control (SPEC §8.2): (5) Spell, Rare. "Float reduces cost. / Steal an enemy card.",
//! radiant "Steal an enemy card. Make it Radiant."
//!
//! The radiant face steals the same card and then sets its flag (§8 Conventions).
//!
//! "Card" is any enemy permanent, a Unit, Field Spell, Trap or Field Trap, since issue #3139 (R3139002): a
//! DECLARED play-time target (R81), validated by R90, only the top card of a Stack pile on offer (R13) and a
//! face-down Trap offered by its id alone (R177). No enemy permanent fizzles and the spell still counts as
//! played. It is the steal of v0.3.7b again, which `script_v0_3_7b` keeps for replays (R767000); through
//! v0.5.11r (`script_v0_5_11o`, kept) it stole an enemy Unit only, since patch v0.3.12 (#1149), and cost (4).
//!
//! "Float reduces cost" (§6.2, R1563000) is the engine's `float_reduces_cost`, a hand trigger (R464) as
//! Drone Core VIII's: each Float of its owner's lowers its cost by that much while it is in their hand,
//! never in the deck (R2657002). At (5) it is past `MAX_MANA` and is played only after a Float or with mana
//! gained (§2.3).
//!
//! Placement is §6.3 Steal and R15 (`effects/steal.rs`). Control is field-only (R12): only `controller`
//! moves; owner, damage, buffs, counters and position stay (R78 is about leaving). `set_radiant` is the
//! flag alone (R74): the base layer swaps, damage and buffs stay, no Cry re-fires (R22). It resolves by
//! instance id, so the steal's zone move cannot make it miss.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-049";

/// "An enemy Unit": the enemy's Units, never a card of the backrow's other types.
fn enemy_unit() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))]
}

/// "An enemy card" (R3139002): §6.3's permanent is a Unit, Field Spell, Trap or Field Trap, so both enemy
/// rows are on offer; v0.3.7b's declaration.
fn enemy_permanent() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "backrow"] }))]
}

pub fn script() -> CardScripts {
    scripts_with(enemy_permanent, true)
}

/// R767000: the scripts v0.3.7b and every version before it ran, a steal of any enemy permanent with no
/// Float; `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    scripts_with(enemy_permanent, false)
}

/// R767000: the scripts v0.5.11r and the versions back to patch v0.3.12 (#1149) ran, a steal of an enemy Unit
/// with no Float; `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_5_11o() -> CardScripts {
    scripts_with(enemy_unit, false)
}

/// `float`: whether "Float reduces cost" is printed (since issue #3139), a hand trigger on both faces.
fn scripts_with(targets: fn() -> Vec<TargetDecl>, float: bool) -> CardScripts {
    let hand_triggers = || if float { vec![float_reduces_cost("snom-bunny-float-hand")] } else { vec![] };
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| vec![steal(json_as(json!({ "target": { "of": "chosen" } })))])),
            hand_triggers: hand_triggers(),
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
            hand_triggers: hand_triggers(),
            ..Script::default()
        },
    }
}

// #49 Snom Bunny Mind Control — SPEC §8.2, BUILD M4-T4, R15, R3139002.
// Props: #25, #45, #8, #15, #41, and #14 Jlockeed's Weapons.
#[cfg(test)]
mod tests {
    use super::{script_v0_3_7b, script_v0_5_11o};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const MIND_CONTROL: &str = "core-049";
    const SEVEN_SEVEN: &str = "core-025";
    const DUELIST: &str = "core-045";
    const VANILLA: &str = "core-008";
    const TOKEN_MAKER: &str = "core-015";
    const SHEEPISH: &str = "core-041";
    const WEAPONS: &str = "core-014";
    const FILLER: &str = "core-005"; // (1) Spell: a card kept in hand, so a Float has mana to leave.

    const P1: PlayerId = PlayerId::P1;

    /// Mind Control costs (5), past the harness's 4 mana (issue #3139): a game gives p1 the mana to play it.
    const COST: i32 = 5;

    /// The target lists of every `play` `legal_actions` offers p1 for this card (R81).
    // Differs from Scenario::offered: that card's targets as ids, per play, not the seat's whole legal list.
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
                "p1": { "hand": [MIND_CONTROL], "mana": COST },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let prey = g.unit(PlayerId::P2, 3).expect("setup: p2 should hold the 7/7 in lane 3").clone();

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert_eq!(g.unit(PlayerId::P1, 3).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert!(g.unit(PlayerId::P2, 3).is_none());
            assert_eq!(g.card(&prey).controller, PlayerId::P1);
            assert_eq!(g.card(&prey).owner, PlayerId::P2);
            g.expect_events(json!(["cardPlayed", "controlChanged", "enteredGraveyard"]));
            // Cost 5 since issue #3139.
            g.expect_mana(PlayerId::P1, 0);
        }

        /// Issue #3139, R3139002: "an enemy card" is any enemy permanent, on both faces: a Unit, a Field Spell, a
        /// Trap (face-down by its id alone, R177), and none of the caster's own.
        #[test]
        fn r3139002_every_enemy_permanent_is_on_offer_a_unit_a_field_spell_and_a_face_down_trap() {
            for radiant in [false, true] {
                let mut g = scenario(json!({
                    "p1": {
                        "hand": [{ "def": MIND_CONTROL, "radiant": radiant }],
                        "field": [{ "def": SEVEN_SEVEN, "lane": 1 }],
                        "mana": COST,
                    },
                    "p2": {
                        "field": [{ "def": VANILLA, "lane": 2 }],
                        "backrow": [{ "def": WEAPONS, "lane": 1, "faceUp": true }, { "def": SHEEPISH, "lane": 3, "faceUp": false }],
                    },
                }));
                let card = g.card(MIND_CONTROL).clone();
                let mut want: Vec<String> = [VANILLA, WEAPONS, SHEEPISH].iter().map(|def| g.card(*def).id.clone()).collect();
                let mut offers: Vec<String> = offered(g.state(), &card).into_iter().flatten().collect();
                want.sort();
                offers.sort();
                assert_eq!(offers, want, "radiant {radiant}");

                // The face-down Trap is offered by its id alone: the view of the play names no card for it.
                let trap = g.card(SHEEPISH).clone();
                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": trap.id }] }));
                assert_eq!(g.card(&trap).controller, PlayerId::P1, "radiant {radiant}");
                assert_eq!(g.card(&trap).owner, PlayerId::P2, "radiant {radiant}");
            }
        }

        #[test]
        fn r3139002_a_stolen_field_spell_is_the_casters_and_its_aura_follows_it() {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL], "field": [{ "def": VANILLA, "lane": 1 }], "mana": COST },
                "p2": { "backrow": [{ "def": WEAPONS, "lane": 1 }], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }] },
            }));
            let weapons = g.backrow(PlayerId::P2, 1).expect("setup: p2's Weapons").clone();
            // Core #14's aura: the enemy's 7/7 has the +attack while it stands; p1's Vanilla has none.
            let vanilla_before = g.stats(VANILLA).attack;
            let prey_before = g.stats(SEVEN_SEVEN).attack;
            assert!(prey_before > 7, "the enemy's Weapons arm the enemy's 7/7: {prey_before}");

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": weapons.id }] }));

            assert_eq!(g.card(&weapons).controller, PlayerId::P1);
            assert!(g.stats(VANILLA).attack > vanilla_before, "the stolen aura arms p1's Units now");
            assert_eq!(g.stats(SEVEN_SEVEN).attack, 7);
        }

        #[test]
        fn r767000_a_game_on_v0_3_7_folds_under_the_kept_script_a_face_down_enemy_trap_is_on_offer() {
            crate::register_all();
            let shipped = crate::shipped_catalog("v0.3.0c").expect("v0.3.0c is carried");
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
        fn s8_an_enemy_board_with_no_permanent_offers_nothing_and_the_spell_fizzles() {
            let mut g = scenario(json!({
                "p1": { "hand": [MIND_CONTROL], "mana": COST },
                "p2": { "hand": [FILLER] },
            }));
            let card = g.card(MIND_CONTROL).clone();
            assert_eq!(offered(g.state(), &card), vec![Vec::<String>::new()]);

            g.play(MIND_CONTROL, json!({}));

            g.expect_in_zone(MIND_CONTROL, "graveyard");
            assert!(!g.events().iter().any(|event| event.event_type().as_str() == "controlChanged"));
        }

        #[test]
        fn r22_r74_radiant_the_stolen_card_becomes_radiant_keeping_its_damage() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_mind_control()], "mana": COST },
                "p2": { "field": [{ "def": DUELIST, "lane": 2, "damage": 2 }] },
            }));
            let prey = g.unit(PlayerId::P2, 2).expect("setup: p2 should hold Deft Duelist").clone();
            g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 3, "health": 1 }));

            g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

            assert_eq!(g.unit(PlayerId::P1, 2).map(|u| u.id.clone()), Some(prey.id.clone()));
            assert!(g.card(&prey).radiant);
            g.expect_stats(&prey, json!({ "attack": 8, "maxHealth": 6, "health": 4 }));
            assert_eq!(g.card(&prey).damage, 2);
            g.expect_events(json!(["controlChanged", "radiantSet"]));
        }

        #[test]
        fn r22_radiant_the_flag_fires_no_cry_nothing_is_summoned_alongside_the_steal() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_mind_control()], "mana": COST },
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
        fn r81_both_faces_declare_one_enemy_permanent_and_the_radiant_face_adds_the_flag() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, MIND_CONTROL);
            assert_eq!(serde_json::to_value(def.type_).unwrap(), json!("Spell"));
            assert_eq!(serde_json::to_value(def.cost).unwrap(), json!(COST));
            let scripts = super::super::script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(
                    serde_json::to_value(&face.targets).unwrap(),
                    json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit", "backrow"] } }])
                );
                assert!(face.modes.is_empty());
                assert_eq!(face.hand_triggers.len(), 1);
            }
        }
    }

    mod float {
        use super::*;

        fn price_of(s: &Scenario, id: &str) -> i32 {
            jackioh_engine::mana::effective_cost(s.state(), s.card(id), jackioh_engine::mana::CostOptions::default())
        }

        /// p1 holds one Mind Control and has one in the deck, with `mana` left to Float.
        fn floating(mana: i32, radiant: bool) -> (Scenario, String, String) {
            let s = scenario(json!({
                "p1": {
                    "hand": [{ "def": MIND_CONTROL, "radiant": radiant }, FILLER],
                    "library": [{ "def": MIND_CONTROL, "radiant": radiant }, FILLER, FILLER],
                    "mana": mana,
                },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            let hand = s.hand(P1).into_iter().find(|card| card.def_id == MIND_CONTROL).map(|card| card.id).unwrap_or_default();
            let deck = s.pile(P1, "library").into_iter().find(|card| card.def_id == MIND_CONTROL).map(|card| card.id).unwrap_or_default();
            (s, hand, deck)
        }

        /// R1563000, R2657002, R3139002: "Float reduces cost" works in the hand and not in the deck.
        #[test]
        fn r3139002_r1563000_a_float_lowers_its_cost_in_hand_never_in_the_deck() {
            for radiant in [false, true] {
                let (mut s, hand, deck) = floating(3, radiant);
                s.end_turn();
                assert_eq!(s.card(hand.as_str()).cost_mod, -3, "radiant {radiant}");
                assert_eq!(s.card(deck.as_str()).cost_mod, 0, "radiant {radiant}");
                assert_eq!(price_of(&s, &hand), COST - 3, "radiant {radiant}");
                assert_eq!(price_of(&s, &deck), COST, "radiant {radiant}");
            }
        }

        #[test]
        fn r1563000_no_float_and_the_opponents_float_lower_nothing() {
            let (mut s, hand, _) = floating(0, false);
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, 0);
            // p2 now ends a turn with mana left: their Float, not p1's.
            s.state_mut().players.p2.mana.current = 4;
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        }

        /// At (5) it is past the 4 mana a turn gives: unplayable at 4, playable once a Float lowered it.
        #[test]
        fn r3139002_at_5_it_cannot_be_played_on_4_mana_until_a_float_has_lowered_it() {
            let mut s = scenario(json!({
                "p1": { "hand": [MIND_CONTROL, FILLER], "mana": 4 },
                "p2": { "field": [SEVEN_SEVEN] },
            }));
            let prey = s.card(SEVEN_SEVEN).clone();
            let named = json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] });
            s.expect_refused(|s| s.play(MIND_CONTROL, named.clone()));
            assert_eq!(s.card(&prey).controller, PlayerId::P2);

            s.card_mut(MIND_CONTROL).cost_mod = -1;
            s.play(MIND_CONTROL, named);
            assert_eq!(s.card(&prey).controller, PlayerId::P1);
        }
    }

    /// R767000: through v0.5.11r it cost (4), had no Float and stole an enemy Unit only, the Radiant face
    /// making it Radiant.
    #[test]
    fn r767000_the_script_kept_for_v0_5_11o_steals_a_unit_only_with_no_float() {
        assert!(script_v0_5_11o().base.hand_triggers.is_empty());
        assert!(script_v0_5_11o().radiant.hand_triggers.is_empty());
        crate::under_kept(MIND_CONTROL, script_v0_5_11o(), || {
            for radiant in [false, true] {
                let mut g = scenario(json!({
                    "p1": { "hand": [{ "def": MIND_CONTROL, "radiant": radiant }], "mana": COST },
                    "p2": { "field": [{ "def": DUELIST, "lane": 2 }], "backrow": [{ "def": WEAPONS, "lane": 1 }] },
                }));
                let prey = g.unit(PlayerId::P2, 2).expect("setup: Deft Duelist").clone();
                let card = g.card(MIND_CONTROL).clone();
                assert_eq!(offered(g.state(), &card), vec![vec![prey.id.clone()]], "radiant {radiant}");
                g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
                assert_eq!(g.card(&prey).controller, PlayerId::P1, "radiant {radiant}");
                assert_eq!(g.card(&prey).radiant, radiant, "radiant {radiant}");
            }
        });
    }

    /// R767000: v0.3.7b's steal of any permanent, with no Float, is the same declaration the card prints again.
    #[test]
    fn r767000_the_script_kept_for_v0_3_7b_has_the_same_target_and_no_float() {
        let kept = script_v0_3_7b();
        let now = super::script();
        assert_eq!(
            serde_json::to_value(&kept.base.targets).unwrap(),
            serde_json::to_value(&now.base.targets).unwrap()
        );
        assert!(kept.base.hand_triggers.is_empty());
        assert!(kept.radiant.hand_triggers.is_empty());
    }
}
