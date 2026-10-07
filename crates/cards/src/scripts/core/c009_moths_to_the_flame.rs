//! #9 Moths to the Flame (SPEC §8.1): a 2-cost 1/14 → 2/28 Unit, "Start of turn: every enemy unit
//! attacks this". The radiant cell is "Armor 1; same": Armor 1 is printed on the radiant face in
//! `catalog.json` and applied by §10.4's keyword layer, and "same" says the text is unchanged
//! (§8 Conventions), so the two faces run the identical hook.
//!
//! §8.1's Engine cell is "Forced attacks in enemy lane order (§4.2); stops when Moths dies", which
//! is R53 in full: the forced attacks skip §4.2 steps 1-3 (no validator, so position, summoning
//! sickness and Taunt are all irrelevant), spend no exertion, the target still strikes back, each
//! forced attack is its own combat followed by its own state check, and the next forced attacker
//! attacks only while the target is still on the field. `combat::force_attacks_on` implements exactly
//! that, and `start_turn` fires this hook for the controller alone, before their draw (§2.2, R68).
//!
//! Armor is not this card's business either: §4.4 step 2 subtracts the defender's armor from each
//! incoming hit, so the radiant face's printed Armor 1 reduces every forced attack by 1 on its own.
//!
//! A card file returns `Vec<Effect>` and never touches state (CLAUDE.md rule 5), so it cannot call
//! `combat::force_attacks_on`, which takes an `EngineSink`. It goes through `forced_attacks_on`
//! (engine/src/effects/combat.rs, beside #60 Bear Honeypot's `forced_attacks`), a thin wrapper:
//! `EffectContext` derefs to an `EngineSink`, so the wrapper names the enemy's units in lane order
//! and hands the rest to `force_attacks_on`, and R53 stays in one place.

use jackioh_engine::effects::forced_attacks_on;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-009";

/// Both faces: every enemy unit, in enemy lane order, is forced to attack this card (R53).
fn moths() -> Script {
    Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![forced_attacks_on(json_as(json!({ "target": { "of": "self" }, "attackers": "enemy" })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = moths();
    let radiant = moths();
    CardScripts { base, radiant }
}

// #9 Moths to the Flame — SPEC §8.1 row 9, BUILD M4-T4 must-pass: "Each enemy unit attacks it in
// lane order at controller's start of turn, no exertion spent, sick units included, each attack is
// its own combat, stops when Moths dies (R53); radiant Armor 1 reduces each hit (R53)".
//
// `start_turn()` is the controller's own start of turn, so it is the whole trigger here. The
// attackers are picked for their arithmetic rather than their text: #7 (1/1), #12 (3/4), #2 (6/1)
// and #13 (8/10) have Cry or end-of-turn text only, and nothing they do fires when the harness
// places them on the field or when they are forced to attack.
//
// "Sick units included" is the one clause no card test can reach: `is_sick` is
// `summonedTurn === state.turn` (combat.rs), and a unit the opponent summoned on their own turn
// is one turn old by the time this card's controller starts a turn, so a summoning-sick ENEMY unit
// cannot exist at this moment on any legal line of play. `force_attack` never asks the question at
// all — it skips §4.2 steps 1-3 entirely — which is what the tests below assert in the reachable
// form (a Defense Position unit, which also cannot legally attack, is forced all the same), and
// `crates/engine/tests/rules/combat_resolution.rs` covers the sick attacker directly.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const LIBRARY: [&str; 3] = ["core-011", "core-011", "core-011"];

    /// One `attackDeclared` event: who attacked whom, and whether the attack was forced.
    #[derive(Clone, Debug, PartialEq)]
    struct Declared {
        attacker_id: String,
        target_id: String,
        forced: bool,
    }

    /// Who attacked whom, in the order the step declared it, and whether the attack was forced.
    fn attacks(s: &Scenario) -> Vec<Declared> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced,
                } => Some(Declared {
                    attacker_id: attacker_id.clone(),
                    target_id: target_id.clone(),
                    forced: *forced,
                }),
                _ => None,
            })
            .collect()
    }

    mod n9_moths_to_the_flame_s8_1_row_9 {
        use super::*;

        #[test]
        fn r53_every_enemy_unit_attacks_it_in_lane_order_each_its_own_combat_and_it_strikes_back() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-009-base",
                "p1": { "field": ["core-009"], "library": LIBRARY },
                // Lane order: 1/1, then 3/4, then 6/1 (§4.2, R53).
                "p2": { "field": ["core-007", "core-012", "core-002"] }
            }));
            let scarab = s.card("core-007").clone();
            let felinors = s.card("core-012").clone();
            let bigot = s.card("core-002").clone();

            s.start_turn();

            // Three separate forced combats, in enemy lane order.
            let declared = attacks(&s);
            assert_eq!(
                declared.iter().map(|event| event.attacker_id.clone()).collect::<Vec<_>>(),
                vec![scarab.id.clone(), felinors.id.clone(), bigot.id.clone()]
            );
            assert_eq!(declared.iter().map(|event| event.forced).collect::<Vec<_>>(), vec![true, true, true]);
            let moths = s.card("core-009").id.clone();
            assert_eq!(
                declared.iter().map(|event| event.target_id.clone()).collect::<Vec<_>>(),
                vec![moths.clone(), moths.clone(), moths.clone()]
            );

            // 1 + 3 + 6 landed on a 1/14.
            s.expect_stats("core-009", json!({ "attack": 1, "health": 4, "maxHealth": 14 }));
            // The target still strikes back: its 1 attack killed the 1-health attackers and damaged the
            // rest.
            s.expect_in_zone(&scarab, "graveyard");
            s.expect_stats(&felinors, json!({ "health": 3, "maxHealth": 4 }));
            s.expect_in_zone(&bigot, "graveyard");
        }

        #[test]
        fn r53_the_forced_attacks_spend_no_exertion_and_ignore_position_and_the_validator() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-009-exertion",
                "p1": { "field": ["core-009"], "library": LIBRARY },
                // A Defense Position unit cannot legally attack at all (§4.2 step 1), and its position
                // gives it Armor 1, which absorbs Moths' whole strike-back.
                "p2": { "field": [{ "def": "core-012", "position": "DEF" }] }
            }));
            let felinors = s.card("core-012").clone();

            s.start_turn();

            assert_eq!(
                attacks(&s).iter().map(|event| event.attacker_id.clone()).collect::<Vec<_>>(),
                vec![felinors.id.clone()]
            );
            s.expect_stats("core-009", json!({ "health": 11, "maxHealth": 14 }));
            // Position Armor 1 reduced the 1 strike-back to nothing (§4.4 step 2, R63).
            s.expect_stats(&felinors, json!({ "health": 4, "maxHealth": 4 }));
            // R53: no exertion was spent, so the unit is untouched for its own controller's turn.
            assert_eq!(s.card(&felinors).exertion, Exertion::default());
            assert_eq!(s.card(&felinors).position, Some(Position::Def));
        }

        #[test]
        fn r53_the_run_stops_once_moths_has_left_the_field() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-009-stops",
                "p1": { "field": ["core-009"], "library": LIBRARY },
                // 6 then 8 is lethal to a 1/14 after two combats; lane 3 never gets to attack.
                "p2": { "field": ["core-002", "core-013", "core-007"] }
            }));
            let moths = s.card("core-009").clone();
            let bigot = s.card("core-002").clone();
            let shredder = s.card("core-013").clone();
            let scarab = s.card("core-007").clone();

            s.start_turn();

            assert_eq!(
                attacks(&s).iter().map(|event| event.attacker_id.clone()).collect::<Vec<_>>(),
                vec![bigot.id.clone(), shredder.id.clone()]
            );
            s.expect_in_zone(&moths, "graveyard");
            // Each forced attack was its own combat with its own state check: the 6/1 died to the
            // strike-back of the combat it started, before the 8/10 attacked.
            s.expect_in_zone(&bigot, "graveyard");
            s.expect_stats(&shredder, json!({ "health": 9, "maxHealth": 10 }));
            // Lane 3 was never asked: undamaged, unexerted, still on the field.
            s.expect_in_zone(&scarab, "field");
            s.expect_stats(&scarab, json!({ "health": 1, "maxHealth": 1 }));
            assert_eq!(s.card(&scarab).exertion, Exertion::default());
        }

        #[test]
        fn r53_radiant_armor_1_reduces_each_hit_and_the_radiant_body_is_a_2_28() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-009-radiant",
                "p1": { "field": [{ "def": "core-009", "radiant": true }], "library": LIBRARY },
                "p2": { "field": ["core-012", "core-002"] }
            }));
            let felinors = s.card("core-012").clone();
            let bigot = s.card("core-002").clone();

            s.start_turn();

            // 3 and 6 attack, each reduced by 1: 2 + 5 = 7, not 9 - 1 (§4.4 step 2).
            s.expect_stats("core-009", json!({ "attack": 2, "health": 21, "maxHealth": 28 }));
            // The radiant face strikes back with 2.
            s.expect_stats(&felinors, json!({ "health": 2, "maxHealth": 4 }));
            s.expect_in_zone(&bigot, "graveyard");
        }
    }
}
