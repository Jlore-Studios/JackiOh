//! C #6 Cloaked Toe Cracker (SPEC §8.6 row 6, §6.3 Cost, §6.3 Mana; R33, R65, R70, R783005). Unit,
//! Human, cost 2, Common, 3/4 → 6/8. Base "Cloaked\nAura: Your Traps cost (0)."; Radiant adds "After you
//! play a Trap, gain {mana} mana."
//!
//! CLOAKED is printed on both faces (since patch v0.3.7): it may be set face-down from hand at its own
//! (2) and revealed by its Reveal (`alt_play`, R783004–R783006). Set, it is a Field Trap whose text is
//! dormant, so neither its Aura nor its trigger acts until it reveals.
//!
//! THE AURA is a price rule the card lays while it acts on the field (B5 E15, `Script.costAura`): "costs
//! (0)" on its controller's Traps and Field Traps ("Trap" names "Field Trap" too). A price is a play's,
//! so it reaches the cards a play takes (the hand, R65) and lasts only while the card stands. A card set
//! by its Cloaked is priced as the card it is in hand (R783005), so the Aura never makes one free.
//!
//! THE RADIANT TRIGGER answers every `cardPlayed` of a Trap or Field Trap by its controller, a cast
//! included (R70), and gains the card's declared mana (`param(ctx, "mana")`) for this turn (§6.3 Mana).
//! The played card is read as it stands, so a Cloaked card's face-down play, a Trap play (R783005),
//! answers it. The `manaChanged` it makes names no card, so a face-down trap stays unnamed to the
//! opponent (R33, R97).

use jackioh_engine::effects::gain_mana;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-006";

const TRAP_TYPES: &[CardType] = &[CardType::Trap, CardType::FieldTrap];

/// Your Traps and Field Traps cost (0).
fn aura() -> CostAuraHook {
    read_hook(|_args| {
        vec![CostAura {
            rule: CostRule {
                types: Some(vec![CardType::Trap, CardType::FieldTrap]),
                set_to: Some(0),
                ..CostRule::default()
            },
            whose: CostAuraWhose::Yours,
            ban: None,
        }]
    })
}

/// After you play a Trap or a Field Trap (a cast is a play, R70), gain the card's mana. A permanent's
/// trigger reads its condition in `run` (a `when` predicate is a trap's, R99): any other play answers
/// with nothing.
fn after_you_play_a_trap() -> TriggerDef {
    TriggerDef::new("toe-cracker-mana", &[GameEventType::CardPlayed], |ctx, event| {
        let GameEvent::CardPlayed {
            player,
            instance_id,
            def_id,
            ..
        } = event
        else {
            return vec![];
        };
        if *player != ctx.controller {
            return vec![];
        }
        // R783005: the card as it stands, so a card set by its Cloaked is the Trap it was played as.
        let played = find_instance(ctx.state, instance_id)
            .map_or_else(|| def_of(Some(&*ctx.state), def_id).type_, |card| card_type_of(ctx.state, card));
        if !TRAP_TYPES.contains(&played) {
            return vec![];
        }
        vec![gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") })))]
    })
}

pub fn script() -> CardScripts {
    let cost_aura = aura();
    let base = Script {
        cost_aura: Some(cost_aura.clone()),
        ..Script::default()
    };

    let radiant = Script {
        cost_aura: Some(cost_aura),
        triggers: vec![after_you_play_a_trap()],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #6 Cloaked Toe Cracker — SPEC §8.6 row 6, BUILD M9 Classic row C 6: your Traps and Field Traps in hand
// cost (0) while it is on the field (R65) and return to their cost when it leaves; the opponent's view of
// your changed hand costs shows −1 (R177); radiant 6/8: after you play a Trap or Field Trap (a cast
// included, R70) gain 1 mana this turn, never naming the face-down trap (R33, R97); its tuned number
// (radiant mana) reads via `param()` (R386).
//
// The cast case uses Classic+ #37 Wardrum, whose end-of-turn copy is a cast Trap.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CRACKER: &str = "classic-006";
    const EXPERIMENT: &str = "core-085"; // (2) Trap
    const SHEEPISH: &str = "core-041"; // (1) Trap
    const TESLA: &str = "classic-005"; // (2) Field Trap
    const MANA_WELL: &str = "core-006"; // (3) Field Spell
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit
    const WARDRUM: &str = "classicplus-037"; // (5) Unit: "End of turn: Cast a copy of a random Spell, Field Spell or Trap you played this turn."

    use crate::js;

    fn cost_of(s: &Scenario, card: &str) -> i32 {
        effective_cost(s.state(), s.card(card), Default::default())
    }

    mod c_n6_cloaked_toe_cracker {
        use super::*;

        #[test]
        fn declares_its_one_number_the_radiant_face_s_mana_r386() {
            crate::register_all();
            assert_eq!(
                js(&crate::card_def(CRACKER).params),
                json!([{ "key": "mana", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }])
            );
            let scripts = script();
            assert!(scripts.base.triggers.is_empty());
            let ons: Vec<Vec<GameEventType>> = scripts.radiant.triggers.iter().map(|trigger| trigger.on.clone()).collect();
            assert_eq!(ons, vec![vec![GameEventType::CardPlayed]]);
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_3_4() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE], "field": [CRACKER] }, "p2": { "hand": [STOCKPILE] } }));
                s.expect_stats(CRACKER, json!({ "attack": 3, "health": 4, "maxHealth": 4 }));
            }

            #[test]
            fn r65_aura_your_traps_and_field_traps_in_hand_cost_0_while_it_is_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, SHEEPISH, TESLA], "field": [CRACKER] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
                assert_eq!(cost_of(&s, SHEEPISH), 0);
                assert_eq!(cost_of(&s, TESLA), 0);
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                s.play(TESLA, json!({ "zone": 2 }));
                s.expect_mana(P1, 4);
            }

            #[test]
            fn played_from_hand_the_trap_costs_0_at_once() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRACKER, EXPERIMENT, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                assert_eq!(cost_of(&s, EXPERIMENT), 2);
                s.play(CRACKER, json!({}));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
            }

            #[test]
            fn once_it_leaves_the_field_your_traps_return_to_their_own_cost() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [CRACKER] },
                    "p2": { "hand": [HIT_JOB, STOCKPILE] },
                    "active": "p2",
                }));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
                let cracker = s.card(CRACKER).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": cracker }] }));
                s.expect_in_zone(CRACKER, "graveyard");
                assert_eq!(cost_of(&s, EXPERIMENT), 2);
            }

            #[test]
            fn the_opponent_s_traps_and_your_field_spells_and_spells_are_untouched() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [MANA_WELL, STOCKPILE], "field": [CRACKER] },
                    "p2": { "hand": [EXPERIMENT, STOCKPILE] },
                }));
                assert_eq!(cost_of(&s, MANA_WELL), 3);
                assert_eq!(cost_of(&s, STOCKPILE), 1);
                assert_eq!(cost_of(&s, EXPERIMENT), 2);
            }

            #[test]
            fn the_base_face_gains_no_mana_after_a_trap() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [CRACKER] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                s.expect_mana(P1, 4);
            }

            #[test]
            fn r177_the_opponent_reads_your_hand_as_a_count_and_no_cost_of_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [CRACKER, EXPERIMENT, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(CRACKER, json!({}));
                let theirs = js(&s.view(P2));
                assert_eq!(theirs["opponent"]["hand"], json!({ "count": 2 }));
                for event in theirs["events"].as_array().cloned().unwrap_or_default() {
                    if event["type"] == "costChanged" {
                        assert_eq!(event["cost"], json!(-1));
                    }
                }
                assert!(!theirs.to_string().contains(EXPERIMENT));
                // The change is real in your own view: the Trap reads (0) there.
                let your_hand = js(&s.view(P1))["you"]["hand"].clone();
                let Some(cards) = your_hand.as_array() else {
                    panic!("your own hand travels in full (§10.8)");
                };
                let experiment = cards.iter().find(|card| card["defId"] == EXPERIMENT);
                assert_eq!(experiment.map(|card| card["cost"].clone()), Some(json!(0)));
            }

            #[test]
            fn r13_dormant_under_a_stack_pile_its_aura_is_off_your_traps_cost_their_own_price() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [CRACKER, { "def": "core-092", "stack": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some("core-092".to_string()));
                assert_eq!(cost_of(&s, EXPERIMENT), 2);
            }
        }

        /// The Toe Cracker in p1's hand.
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
                    assert!(face.text.starts_with("Cloaked\nAura: Your Traps cost (0)."), "{}", face.text);
                }
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
                        legal_actions(s.state(), P1).iter().any(|action| matches!(action,
                            ActionBody::Play { instance_id, face_down: Some(RevealAt::Cloaked), .. } if *instance_id == id)),
                        "offered Cloaked"
                    );
                    set_cracker(&mut s);
                    // A Unit set by its Cloaked is priced as the Unit, so the Aura's (0) misses it.
                    s.expect_mana(P1, 2);
                }
            }

            #[test]
            fn r783006_set_its_aura_is_dormant_revealed_it_is_a_3_4_and_the_aura_starts() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [CRACKER, EXPERIMENT, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let set = set_cracker(&mut s);
                assert_eq!(cost_of(&s, EXPERIMENT), 2, "face-down, its Aura does not act");
                // The opponent reads a face-down card, never the Toe Cracker.
                assert!(!js(&s.view(P2)).to_string().contains(CRACKER));
                reveal(&mut s, &set);
                let unit = s.unit(P1, 1).expect("it animated into its own lane");
                assert_eq!(unit.def_id, CRACKER);
                s.expect_stats(unit.id.as_str(), json!({ "attack": 3, "health": 4, "maxHealth": 4 }));
                assert_eq!(cost_of(&s, EXPERIMENT), 0, "revealed, its Aura acts");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r783005_a_set_toe_cracker_is_a_trap_play_another_radiant_one_gains_mana_its_own_trigger_sleeps() {
                crate::register_all();
                // Beside a face-up Radiant Toe Cracker, setting this one is a Trap play: +1 mana.
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": CRACKER, "radiant": true }, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true, "lane": 3 }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                set_cracker(&mut s);
                s.expect_mana(P1, 3);
                // Alone, its own trigger is dormant face-down: no mana for its own set, nor for a Trap
                // played after it.
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": CRACKER, "radiant": true }, EXPERIMENT, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                set_cracker(&mut s);
                s.expect_mana(P1, 2);
                s.play(EXPERIMENT, json!({ "zone": 2 }));
                s.expect_mana(P1, 0);
            }

            #[test]
            fn r783006_revealed_it_is_a_6_8_its_traps_cost_0_and_a_trap_gains_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": CRACKER, "radiant": true }, EXPERIMENT, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let set = set_cracker(&mut s);
                reveal(&mut s, &set);
                let unit = s.unit(P1, 1).expect("revealed").id;
                s.expect_stats(unit.as_str(), json!({ "attack": 6, "health": 8, "maxHealth": 8 }));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
                s.play(EXPERIMENT, json!({ "zone": 2 }));
                s.expect_mana(P1, 3);
            }

            #[test]
            fn is_a_6_8_with_the_same_aura() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.expect_stats(CRACKER, json!({ "attack": 6, "health": 8, "maxHealth": 8 }));
                assert_eq!(cost_of(&s, EXPERIMENT), 0);
            }

            #[test]
            fn after_you_play_a_trap_gain_1_mana_this_turn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                s.expect_mana(P1, 5);
            }

            #[test]
            fn a_field_trap_counts_as_a_trap() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TESLA, SHEEPISH, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(TESLA, json!({ "zone": 1 }));
                s.play(SHEEPISH, json!({ "zone": 2 }));
                s.expect_mana(P1, 6);
            }

            #[test]
            fn a_spell_a_field_spell_or_the_opponent_s_trap_gains_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, MANA_WELL], "field": [{ "def": CRACKER, "radiant": true }], "library": [STOCKPILE, STOCKPILE] },
                    "p2": { "hand": [SHEEPISH, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                }));
                s.play(STOCKPILE, json!({}));
                s.expect_mana(P1, 3);
                s.play(MANA_WELL, json!({ "zone": 1 }));
                s.expect_mana(P1, 0);
                s.end_turn();
                s.play(SHEEPISH, json!({ "zone": 1 }));
                s.expect_mana(P2, 3);
                assert_eq!(s.view(P1).you.mana.current, 0);
            }

            #[test]
            fn the_mana_is_this_turn_s_it_is_gone_at_your_next_refresh() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }], "library": [STOCKPILE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                }));
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                s.expect_mana(P1, 5);
                s.end_turn();
                s.end_turn();
                s.expect_mana(P1, 4);
            }

            #[test]
            fn r33_r97_the_gain_never_names_the_face_down_trap_in_the_opponent_s_view() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                let Some(trap) = s.backrow(P1, 1) else {
                    panic!("the Trap should be set");
                };
                let theirs = js(&s.view(P2));
                assert_eq!(theirs["opponent"]["backrow"][0], json!({ "faceDown": true, "cost": 2 }));
                let text = theirs.to_string();
                assert!(!text.contains(&format!("\"{}\"", trap.id)));
                assert!(!text.contains(EXPERIMENT));
                let mana_changes: Vec<Value> = theirs["events"]
                    .as_array()
                    .map(|events| events.iter().filter(|event| event["type"] == "manaChanged").cloned().collect())
                    .unwrap_or_default();
                assert!(mana_changes.contains(&json!({ "type": "manaChanged", "player": "p1", "current": 5, "max": 4 })));
            }

            #[test]
            fn r70_a_cast_trap_counts_wardrum_s_copy_of_the_trap_you_played_gains_mana_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [SHEEPISH, STOCKPILE],
                        "field": [{ "def": CRACKER, "radiant": true }, WARDRUM],
                        "library": [STOCKPILE, STOCKPILE],
                    },
                    "p2": { "hand": [STOCKPILE, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                }));
                s.play(SHEEPISH, json!({ "zone": 1 }));
                s.expect_mana(P1, 5);
                s.end_turn(); // Wardrum casts a copy of the Sheepish at the end of p1's turn
                let gains = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::ManaChanged { player, current, .. } if *player == P1 && *current == 6))
                    .count();
                assert_eq!(gains, 1);
            }

            #[test]
            fn r386_a_degrade_of_mana_never_goes_below_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                step_param(s.card_mut(CRACKER), "mana", -1);
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                s.expect_mana(P1, 5);
            }

            #[test]
            fn r13_dormant_under_a_stack_pile_it_gains_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [SHEEPISH, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }, { "def": "core-092", "stack": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(SHEEPISH, json!({ "zone": 1 }));
                s.expect_mana(P1, 3);
            }

            #[test]
            fn r386_an_upgrade_of_mana_gains_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXPERIMENT, STOCKPILE], "field": [{ "def": CRACKER, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                step_param(s.card_mut(CRACKER), "mana", 1);
                s.play(EXPERIMENT, json!({ "zone": 1 }));
                s.expect_mana(P1, 6);
            }
        }
    }
}
