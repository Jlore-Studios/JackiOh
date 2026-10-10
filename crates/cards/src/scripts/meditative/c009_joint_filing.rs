//! Meditative #9 Joint Filing (SPEC §8.8 row 9; docs/meditative-set.md M6 #9; ME-TRIG (a); R820, R821).
//! (2) Field Spell, Epic.
//!   Base:    "Aura: Your Start of turn and End of turn effects trigger {extra|additional time|additional
//!            times}." (extra 1)
//!   Radiant: the same, extra 2.
//!
//! The engine's turn-hook multiplier (`Script.turn_hook_extra`): while this acts on its controller's
//! field, each of their start-of-turn and end-of-turn hooks is queued 1 + extra times as the turn's
//! trigger step queues them (`triggers::queue_hooks_in_trigger_order`), each copy its own entry re-read
//! as it pops (R153, R821). Several multipliers do not add: the highest holds (R820). Both faces run one
//! script; `param` reads the face's `extra`.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-009";

pub fn script() -> CardScripts {
    let base = Script {
        turn_hook_extra: Some(read_hook(|args| param(&args, "extra"))),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// Meditative #9 Joint Filing — SPEC §8.8 row 9, BUILD M10 row M 9: "While it acts, each of your
// start-of-turn and end-of-turn hooks runs twice, each copy its own queue entry right behind the
// original, with a state check between (R59, R821) and fresh random numbers; a card gone before its
// copy pops fizzles (R153); M 8 in your graveyard returns once …; delayed effects, the opponent's turn
// hooks and the end-of-turn trap window are not multiplied (R62); two Joint Filings give one extra, not
// two (R820); M 12's triggered End of turn effects are multiplied too; leaving the field ends it;
// extra reads through `param()`; radiant three runs in all".
//
// Core #13 Jlockeed Shredder-10's end of turn (2 to each enemy Unit and the enemy hero) and Core #58
// Rush Token Farm's start of turn (a Rush Token) are the hooks it multiplies; Core #23 Reoccurring
// Dream stands in for M 8, a Spell that returns from the graveyard at the end of the turn it was played
// (M 8 and M 21 are built by their own parts). M 12's case is in Fear Mongerer's file.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHREDDER: &str = "core-013"; // End of turn: 2 damage to each enemy Unit and the enemy hero.
    const FARM: &str = "core-058"; // Field Spell. Start of turn: summon a Rush Token.
    const RUSH_TOKEN: &str = "core-t-rush";
    const DREAM: &str = "core-023"; // Spell. End of turn: return this to hand (from the graveyard).
    const RECYCLING: &str = "core-039"; // Spell. Exile this. End of turn (delayed): copy each other card played.
    const BREAD_AND_BUTTER: &str = "core-018"; // Field Trap: fires in the end-of-turn trap window.
    const BREAD_TOKEN: &str = "core-t-bread";
    const MAGIC_JAMMED: &str = "core-036"; // Spell: destroy a backrow card, Lock its zone.
    const VANILLA: &str = "core-008"; // a (1) 4/4 with no text
    const FILLER: &str = "core-005"; // Stockpile, a (1) Spell

    /// A side with something to do and a deck to draw, so no turn auto-ends (§2.5) or fatigues; `extra`
    /// adds its own entries.
    fn side(extra: Value) -> Value {
        crate::merged(json!({ "hand": [FILLER], "library": [FILLER, FILLER, FILLER] }), extra)
    }

    fn units(s: &Scenario, player: PlayerId, def_id: &str) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|unit| unit.def_id == def_id)
            .count()
    }

    #[test]
    fn is_a_2_cost_epic_field_spell_one_script_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(def.rarity, Rarity::Epic);
        assert_eq!(
            crate::js(&def.params),
            json!([{ "key": "extra", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }])
        );
        let scripts = super::script();
        assert!(scripts.base.turn_hook_extra.is_some());
        assert!(scripts.radiant.turn_hook_extra.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r821_your_end_of_turn_hooks_run_twice() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            s.end_turn();
            s.expect_health(P2, 26);
        }

        #[test]
        fn r821_your_start_of_turn_hooks_run_twice() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID, FARM] })), "p2": side(json!({})) }));
            s.start_turn();
            assert_eq!(units(&s, P1, RUSH_TOKEN), 2);
        }

        #[test]
        fn r821_r153_a_return_spell_comes_back_once_its_copy_finding_it_gone() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID], "hand": [DREAM] })), "p2": side(json!({})) }));
            let dream = s.card(DREAM).clone();
            s.play(DREAM, json!({}));
            s.end_turn();
            s.expect_in_zone(&dream, "hand");
            let returns = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { instance_id, .. } if *instance_id == dream.id))
                .count();
            assert_eq!(returns, 1);
        }

        #[test]
        fn r821_a_delayed_end_of_turn_effect_runs_once() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [RECYCLING, VANILLA] })),
                "p2": side(json!({})),
            }));
            s.play(RECYCLING, json!({}));
            s.play(VANILLA, json!({}));
            s.end_turn();
            let copies = s.hand(P1).iter().filter(|card| card.def_id == VANILLA).count();
            assert_eq!(copies, 1, "Recycling Initiative's delayed copy is not multiplied (R62)");
        }

        #[test]
        fn r821_the_end_of_turn_trap_window_is_not_multiplied() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID, BREAD_AND_BUTTER] })),
                "p2": side(json!({})),
            }));
            s.end_turn();
            assert_eq!(units(&s, P1, BREAD_TOKEN), 1);
        }

        #[test]
        fn r821_the_opponents_end_of_turn_hooks_run_once() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID] })),
                "p2": side(json!({ "field": [SHREDDER] })),
                "active": "p2",
            }));
            s.end_turn();
            s.expect_health(P1, 28);
        }

        #[test]
        fn r820_two_joint_filings_give_one_extra_not_two() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID, ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            s.end_turn();
            s.expect_health(P2, 26);
        }

        #[test]
        fn r820_one_in_your_hand_multiplies_nothing() {
            let mut s = scenario(json!({ "p1": side(json!({ "hand": [ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            s.end_turn();
            s.expect_health(P2, 28);
        }

        #[test]
        fn r820_leaving_the_field_ends_it() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "field": [SHREDDER], "hand": [MAGIC_JAMMED] })),
                "p2": side(json!({})),
            }));
            let filing = s.card(ID).clone();
            s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": filing.id }] }));
            s.expect_in_zone(&filing, "graveyard");
            s.end_turn();
            s.expect_health(P2, 28);
        }

        #[test]
        fn r386_extra_reads_through_param_a_buff_gives_three_runs() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            assert_eq!(crate::upgrade_number(&mut s, ID, "extra"), 2);
            s.end_turn();
            s.expect_health(P2, 24);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r821_three_runs_in_all() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [{ "def": ID, "radiant": true }], "field": [SHREDDER] })),
                "p2": side(json!({})),
            }));
            s.end_turn();
            s.expect_health(P2, 24);
        }

        #[test]
        fn r821_three_rush_tokens_at_the_start_of_your_turn() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [{ "def": ID, "radiant": true }, FARM] })),
                "p2": side(json!({})),
            }));
            s.start_turn();
            assert_eq!(units(&s, P1, RUSH_TOKEN), 3);
        }
    }
}
