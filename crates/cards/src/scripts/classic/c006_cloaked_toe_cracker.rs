//! C #6 Cloaked Toe Cracker (SPEC §8.6 row 6, §6.3 Cost, §6.3 Mana; R33, R65, R70, R455, R783005). Unit,
//! Human, cost 2, Common, 3/6 → 6/12. Base "Cloaked\nAura: Your Traps cost ({discount}) less."; Radiant
//! "Cloaked\nAura: Your Traps cost (0)."
//!
//! Issue #4440 (Stinkyscape+ Balance Changes VI) made it 3/6 (Radiant 6/12), from 3/4 (Radiant 6/8),
//! its base Aura (2) less, from (0), and took the Radiant face's "After you play a Trap, gain {mana}
//! mana." `script_v0_6_11c` keeps the script before it for the replays of v0.6.11c and before (R767000).
//!
//! CLOAKED is printed on both faces (since patch v0.3.7): it may be set face-down from hand at its own
//! (2) and revealed by its Reveal (`alt_play`, R783004–R783006). Set, it is a Field Trap whose text is
//! dormant, so its Aura does not act until it reveals.
//!
//! THE AURA is a price rule the card lays while it acts on the field (B5 E15, `Script.costAura`) on its
//! controller's Traps and Field Traps ("Trap" names "Field Trap" too): on the base face an add of
//! −`discount` (R455's order, a price never below (0)), on the Radiant face "costs (0)", which wins
//! over every add. A price is a play's, so it reaches the cards a play takes (the hand, R65) and lasts
//! only while the card stands. A card set by its Cloaked is priced as the card it is in hand (R783005),
//! so the Aura never lowers one.

use jackioh_engine::effects::gain_mana;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-006";

const TRAP_TYPES: &[CardType] = &[CardType::Trap, CardType::FieldTrap];

/// The Aura's reach: its controller's Traps and Field Traps.
fn your_traps(rule: CostRule) -> Vec<CostAura> {
    vec![CostAura {
        rule: CostRule {
            types: Some(TRAP_TYPES.to_vec()),
            ..rule
        },
        whose: CostAuraWhose::Yours,
        ban: None,
    }]
}

/// The base face's "cost ({discount}) less", the declared number (R386).
fn discount_aura() -> CostAuraHook {
    read_hook(|args| {
        your_traps(CostRule {
            amount: Some(-param(&args, "discount")),
            ..CostRule::default()
        })
    })
}

/// "cost (0)": the Radiant face's since issue #4440, both faces' before it.
fn free_aura() -> CostAuraHook {
    read_hook(|_args| {
        your_traps(CostRule {
            set_to: Some(0),
            ..CostRule::default()
        })
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cost_aura: Some(discount_aura()),
            ..Script::default()
        },
        radiant: Script {
            cost_aura: Some(free_aura()),
            ..Script::default()
        },
    }
}

/// Through v0.6.11c: after you play a Trap or a Field Trap (a cast is a play, R70), the Radiant face
/// gained the card's mana. A permanent's trigger reads its condition in `run` (a `when` predicate is a
/// trap's, R99): any other play answers with nothing. The play is read as it counts (`played_type_of`),
/// so a Cloaked card's face-down play, a Trap play (R783005), answered it; the `manaChanged` it made
/// names no card, so a face-down trap stayed unnamed to the opponent (R33, R97).
fn after_you_play_a_trap() -> TriggerDef {
    TriggerDef::new("toe-cracker-mana", &[GameEventType::CardPlayed], |ctx, event| {
        let GameEvent::CardPlayed { player, .. } = event else {
            return vec![];
        };
        if *player != ctx.controller {
            return vec![];
        }
        // R783005: the play as it counts, so a card set by its Cloaked is the Trap it was played as.
        if !played_type_of(&*ctx.state, event).is_some_and(|played| TRAP_TYPES.contains(&played)) {
            return vec![];
        }
        vec![gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") })))]
    })
}

/// R767000: through v0.6.11c both faces' Traps cost (0) and the Radiant face gained its mana after a
/// Trap (issue #4440); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_11c() -> CardScripts {
    CardScripts {
        base: Script {
            cost_aura: Some(free_aura()),
            ..Script::default()
        },
        radiant: Script {
            cost_aura: Some(free_aura()),
            triggers: vec![after_you_play_a_trap()],
            ..Script::default()
        },
    }
}

// C #6 Cloaked Toe Cracker — SPEC §8.6 row 6, BUILD M9 Classic row C 6 (R65, R177, R70, R33, R97, R386).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CRACKER: &str = "classic-006";
    const TWO_TRAP: &str = "classic-052"; // Final Gambit: Trap that answers nothing these tests do (Unlicensed Experimentation, core-085 since issue #3136)
    const THREE_TRAP: &str = "core-085"; // Unlicensed Experimentation: a Trap dearer than the discount, held in hand here
    const SHEEPISH: &str = "core-041"; // Sheepish: a Trap
    const TESLA: &str = "classic-005"; // Tesla: a Field Trap
    const FIELD_SPELL: &str = "stand-in (3) Field Spell";
    const STOCKPILE: &str = "core-005"; // Stockpile: a Spell
    const HIT_JOB: &str = "core-016"; // Hit Job: destroy target Unit

    use crate::js;

    fn cost_of(s: &Scenario, card: &str) -> i32 {
        effective_cost(s.state(), s.card(card), Default::default())
    }

    mod c_n6_cloaked_toe_cracker {
        use super::*;

        #[test]
        fn declares_its_one_number_the_base_face_s_discount_r386() {
            crate::register_all();
            assert_eq!(
                js(&crate::card_def(CRACKER).params),
                json!([{ "key": "discount", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1, "tunedOn": "base" }])
            );
            let scripts = script();
            assert!(scripts.base.triggers.is_empty());
            assert!(scripts.radiant.triggers.is_empty(), "issue #4440 took the Radiant mana");
        }

        mod base {
            use super::*;

            #[test]
            fn issue_4440_aura_your_traps_and_field_traps_in_hand_cost_2_less_while_it_is_on_the_field() {
                let mut s = scenario(json!({
                    "p1": { "hand": [TWO_TRAP, SHEEPISH, TESLA, THREE_TRAP], "field": [CRACKER] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                assert_eq!(cost_of(&s, THREE_TRAP), 1, "(3) less 2");
                assert_eq!(cost_of(&s, TWO_TRAP), 0);
                assert_eq!(cost_of(&s, SHEEPISH), 0, "(1) less 2 is no price below (0)");
                assert_eq!(cost_of(&s, TESLA), 0);
                s.play(TWO_TRAP, json!({ "zone": 1 }));
                s.play(TESLA, json!({ "zone": 2 }));
                s.expect_mana(P1, 4);
            }

            #[test]
            fn played_from_hand_the_trap_costs_2_less_at_once() {
                let mut s = scenario(json!({ "p1": { "hand": [CRACKER, THREE_TRAP, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                assert_eq!(cost_of(&s, THREE_TRAP), 3);
                s.play(CRACKER, json!({}));
                assert_eq!(cost_of(&s, THREE_TRAP), 1);
            }

            #[test]
            fn r386_an_upgrade_of_the_discount_takes_3() {
                let mut s = scenario(json!({
                    "p1": { "hand": [THREE_TRAP, STOCKPILE], "field": [CRACKER] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                step_param(s.card_mut(CRACKER), "discount", 1);
                assert_eq!(cost_of(&s, THREE_TRAP), 0);
            }

            #[test]
            fn once_it_leaves_the_field_your_traps_return_to_their_own_cost() {
                let mut s = scenario(json!({
                    "p1": { "hand": [TWO_TRAP, STOCKPILE], "field": [CRACKER] },
                    "p2": { "hand": [HIT_JOB, STOCKPILE] },
                    "active": "p2",
                }));
                assert_eq!(cost_of(&s, TWO_TRAP), 0);
                let cracker = s.card(CRACKER).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": cracker }] }));
                s.expect_in_zone(CRACKER, "graveyard");
                assert_eq!(cost_of(&s, TWO_TRAP), 2);
            }

            #[test]
            fn the_opponent_s_traps_and_your_field_spells_and_spells_are_untouched() {
                let s = scenario(json!({
                    "p1": { "hand": [FIELD_SPELL, STOCKPILE], "field": [CRACKER] },
                    "p2": { "hand": [TWO_TRAP, STOCKPILE] },
                }));
                assert_eq!(cost_of(&s, FIELD_SPELL), 3);
                assert_eq!(cost_of(&s, STOCKPILE), 1);
                assert_eq!(cost_of(&s, TWO_TRAP), 2);
            }

            #[test]
            fn the_base_face_gains_no_mana_after_a_trap() {
                let mut s = scenario(json!({ "p1": { "hand": [TWO_TRAP, STOCKPILE], "field": [CRACKER] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(TWO_TRAP, json!({ "zone": 1 }));
                s.expect_mana(P1, 4);
            }

            #[test]
            fn r177_the_opponent_reads_your_hand_as_a_count_and_no_cost_of_it() {
                let mut s = scenario(json!({ "p1": { "hand": [CRACKER, TWO_TRAP, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(CRACKER, json!({}));
                let theirs = js(&s.view(P2));
                assert_eq!(theirs["opponent"]["hand"], json!({ "count": 2 }));
                for event in theirs["events"].as_array().cloned().unwrap_or_default() {
                    if event["type"] == "costChanged" {
                        assert_eq!(event["cost"], json!(-1));
                    }
                }
                assert!(!theirs.to_string().contains(TWO_TRAP));
                let your_hand = js(&s.view(P1))["you"]["hand"].clone();
                let Some(cards) = your_hand.as_array() else {
                    panic!("your own hand travels in full (§10.8)");
                };
                let experiment = cards.iter().find(|card| card["defId"] == TWO_TRAP);
                assert_eq!(experiment.map(|card| card["cost"].clone()), Some(json!(0)));
            }

            #[test]
            fn r13_dormant_under_a_stack_pile_its_aura_is_off_your_traps_cost_their_own_price() {
                let s = scenario(json!({
                    "p1": { "hand": [TWO_TRAP, STOCKPILE], "field": [CRACKER, { "def": "core-092", "stack": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some("core-092".to_string()));
                assert_eq!(cost_of(&s, TWO_TRAP), 2);
            }
        }

        fn cracker_in_hand(s: &Scenario) -> String {
            s.hand(P1)
                .iter()
                .find(|card| card.def_id == CRACKER)
                .map(|card| card.id.clone())
                .expect("a Toe Cracker in hand")
        }

        /// R783004: set the Toe Cracker in hand face-down by its Cloaked into backrow lane 1; its fresh id.
        fn set_cracker(s: &mut Scenario) -> String {
            let id = cracker_in_hand(s);
            s.play(&id, json!({ "zone": 1, "row": "backrow", "faceDown": "cloaked" }));
            s.backrow(P1, 1).expect("the set Toe Cracker").id
        }

        fn reveal(s: &mut Scenario, id: &str) {
            s.activate(id, json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));
        }

        mod cloaked {
            use super::*;

            #[test]
            fn r783004_both_faces_print_cloaked_before_the_aura() {
                crate::register_all();
                let def = crate::card_def(CRACKER);
                for face in [&def.base, &def.radiant] {
                    assert_eq!(face.keywords, vec![Keyword::Cloaked]);
                    assert!(face.text.starts_with("Cloaked\nAura: Your Traps cost ("), "{}", face.text);
                }
                assert!(def.base.text.ends_with("cost ({discount}) less."), "{}", def.base.text);
                assert!(def.radiant.text.ends_with("cost (0)."), "{}", def.radiant.text);
            }

            #[test]
            fn r783005_set_face_down_it_pays_its_own_2_with_or_without_another_toe_cracker() {
                crate::register_all();
                for beside in [false, true] {
                    let field = if beside { json!([{ "def": CRACKER, "lane": 3 }]) } else { json!([]) };
                    let mut s = scenario(json!({
                        "p1": { "hand": [CRACKER, STOCKPILE], "field": field },
                        "p2": { "hand": [STOCKPILE] },
                    }));
                    let id = cracker_in_hand(&s);
                    assert!(
                        s.plays_of(&id).iter().any(|action| matches!(action,
                            ActionBody::Play { face_down: Some(RevealAt::Cloaked), .. })),
                        "offered Cloaked"
                    );
                    set_cracker(&mut s);
                    s.expect_mana(P1, 2);
                }
            }

            #[test]
            fn r783006_set_its_aura_is_dormant_revealed_it_is_a_3_6_and_the_aura_starts() {
                let mut s = scenario(json!({
                    "p1": { "hand": [CRACKER, TWO_TRAP, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let set = set_cracker(&mut s);
                assert_eq!(cost_of(&s, TWO_TRAP), 2, "face-down, its Aura does not act");
                assert!(!js(&s.view(P2)).to_string().contains(CRACKER));
                reveal(&mut s, &set);
                let unit = s.unit(P1, 1).expect("it animated into its own lane");
                assert_eq!(unit.def_id, CRACKER);
                s.expect_stats(unit.id.as_str(), json!({ "attack": 3, "health": 6, "maxHealth": 6 }));
                assert_eq!(cost_of(&s, TWO_TRAP), 0, "revealed, its Aura acts");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn issue_4440_is_a_6_12_whose_traps_cost_0() {
                let mut s = scenario(json!({
                    "p1": { "hand": [TWO_TRAP, THREE_TRAP], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.expect_stats(CRACKER, json!({ "attack": 6, "health": 12, "maxHealth": 12 }));
                assert_eq!(cost_of(&s, TWO_TRAP), 0);
                assert_eq!(cost_of(&s, THREE_TRAP), 0);
            }

            #[test]
            fn r783006_revealed_it_is_a_6_12_its_traps_cost_0_and_a_trap_gains_no_mana() {
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": CRACKER, "radiant": true }, TWO_TRAP, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let set = set_cracker(&mut s);
                reveal(&mut s, &set);
                let unit = s.unit(P1, 1).expect("revealed").id;
                s.expect_stats(unit.as_str(), json!({ "attack": 6, "health": 12, "maxHealth": 12 }));
                assert_eq!(cost_of(&s, TWO_TRAP), 0);
                s.play(TWO_TRAP, json!({ "zone": 2 }));
                s.expect_mana(P1, 2);
            }

            #[test]
            fn issue_4440_a_trap_or_a_field_trap_gains_no_mana() {
                let mut s = scenario(json!({
                    "p1": { "hand": [TESLA, SHEEPISH, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(TESLA, json!({ "zone": 1 }));
                s.play(SHEEPISH, json!({ "zone": 2 }));
                s.expect_mana(P1, 4);
            }
        }

        /// R767000: the script kept for v0.6.11c, with the `mana` its Radiant face declared then.
        mod kept_v0_6_11c {
            use super::*;

            fn kept<T>(run: impl FnOnce() -> T) -> T {
                crate::under_kept_as(
                    CRACKER,
                    script_v0_6_11c(),
                    |def| {
                        def.params = serde_json::from_value(json!([
                            { "key": "mana", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }
                        ]))
                        .expect("the v0.6.11c params");
                    },
                    run,
                )
            }

            #[test]
            fn r767000_through_v0_6_11c_the_base_face_s_traps_cost_0() {
                kept(|| {
                    let s = scenario(json!({
                        "p1": { "hand": [THREE_TRAP, STOCKPILE], "field": [CRACKER] },
                        "p2": { "hand": [STOCKPILE] },
                    }));
                    assert_eq!(cost_of(&s, THREE_TRAP), 0);
                });
            }

            #[test]
            fn r767000_through_v0_6_11c_a_radiant_one_gained_1_mana_after_a_trap_or_a_field_trap() {
                kept(|| {
                    let mut s = scenario(json!({
                        "p1": { "hand": [TESLA, SHEEPISH, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }] },
                        "p2": { "hand": [STOCKPILE] },
                    }));
                    s.play(TESLA, json!({ "zone": 1 }));
                    s.play(SHEEPISH, json!({ "zone": 2 }));
                    s.expect_mana(P1, 6);
                });
            }
        }
    }
}
