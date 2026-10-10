//! M #1 Disruptive Disruptor (SPEC §8.8 row 1, BUILD M10 row M 1). (2) Field Spell, Common.
//!   Base:    "Aura: You can't be forced to discard cards during your opponent's turn."
//!   Radiant: the Aura, plus "Cry: Draw {draw|card|cards} and gain {mana} mana." (1, 2)
//!
//! The Aura is a discard guard (R800): a pure-read hook beside `draw_limit` (R457), read off the cards
//! acting on the field while it is not the guarded player's turn. An effect's discard (`discard`,
//! `discard_random`, `discard_hand`) from a guarded hand moves nothing, draws no random number (R129)
//! and reports one public `discardPrevented`. A discard paid as a price (an Activate's cost, R384; a
//! targeting cost, R450) never asks the guard, and several Disruptors do no more than one.
//!
//! The Radiant Cry is an ordinary `draw` then `gain_mana` (R1: fires when played or cast). Both numbers
//! are declared and read through `param` (R386); the base face prints neither (R749).

use jackioh_engine::effects::{draw, gain_mana};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-001";

/// "You can't be forced to discard cards during your opponent's turn": the controller's own side,
/// guarded on every turn but their own (`draw::discard_guarded`).
fn disruptor() -> Script {
    Script {
        discard_guard: Some(read_hook(|_args| vec![DrawLimitPlayer::SelfSide])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: disruptor(),
        radiant: Script {
            // "Cry: Draw 1 and gain 2 mana": the declared numbers `draw` and `mana` (R386).
            cry: Some(hook(|ctx| {
                vec![
                    draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                    gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                ]
            })),
            ..disruptor()
        },
    }
}

// M #1 Disruptive Disruptor — SPEC §8.8 row 1, BUILD M10 row M 1: "Aura: an effect's discard from
// your hand while it is not your turn moves nothing, draws no rng and reports `discardPrevented`
// (R800); a discard you pay still happens; two Disruptors report once; in the graveyard it guards
// nothing; radiant Cry draws 1 and gains 2 mana; its tuned numbers (draw, mana) read through
// `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const DISRUPTOR: &str = "meditative-001";
    const BOOK_OF_PAIN: &str = "classicplus-056"; // (2) Spell: your opponent discards 2 at random.
    const DEVILS_PACT: &str = "classic-023"; // (5) Spell: Cry discards your own cards at random.
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005"; // (1) Spell.

    use crate::js;

    fn backrow(def: &str, radiant: bool) -> Value {
        json!({ "def": def, "radiant": radiant, "faceUp": true, "lane": 1 })
    }

    fn count(events: &[GameEvent], event_type: &str) -> usize {
        events.iter().filter(|event| event.event_type().as_str() == event_type).count()
    }

    mod meditative_001 {
        use super::*;

        #[test]
        fn is_a_2_cost_common_field_spell_guarding_on_both_faces() {
            crate::register_all();
            assert_eq!(ID, DISRUPTOR);
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(2));
            assert_eq!(def.rarity, Rarity::Common);
            assert_eq!(def.type_, CardType::FieldSpell);
            let scripts = script();
            assert!(scripts.base.discard_guard.is_some());
            assert!(scripts.radiant.discard_guard.is_some());
            assert!(scripts.base.cry.is_none());
            assert!(scripts.radiant.cry.is_some());
        }

        #[test]
        fn r800_book_of_pain_on_the_opponents_turn_discards_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [VANILLA, VANILLA, VANILLA], "backrow": [backrow(DISRUPTOR, false)] },
                "p2": { "hand": [BOOK_OF_PAIN, FILLER], "mana": 9 },
            }));
            let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();

            s.play(BOOK_OF_PAIN, json!({}));

            let after: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
            assert_eq!(before, after);
            assert_eq!(count(s.last_events(), "discarded"), 0);
            assert_eq!(count(s.last_events(), "discardPrevented"), 1);
            let prevented: Vec<Value> = s
                .last_events()
                .iter()
                .map(js)
                .filter(|event| event["type"] == "discardPrevented")
                .collect();
            assert_eq!(prevented, vec![json!({ "type": "discardPrevented", "player": "p1", "count": 2 })]);
        }

        #[test]
        fn r800_your_own_devils_pact_still_discards() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [DEVILS_PACT, VANILLA, VANILLA, FILLER],
                    "mana": 9,
                    "backrow": [backrow(DISRUPTOR, false)],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(DEVILS_PACT, json!({}));

            // On its controller's own turn the guard is quiet: the hand is gone.
            assert!(s.hand(P1).is_empty());
            assert_eq!(count(s.last_events(), "discardPrevented"), 0);
        }

        #[test]
        fn r800_two_disruptors_report_one_prevention() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [VANILLA, VANILLA, VANILLA],
                    "backrow": [backrow(DISRUPTOR, false), json!({ "def": DISRUPTOR, "faceUp": true, "lane": 2 })],
                },
                "p2": { "hand": [BOOK_OF_PAIN, FILLER], "mana": 9 },
            }));

            s.play(BOOK_OF_PAIN, json!({}));

            assert_eq!(s.hand(P1).len(), 3);
            assert_eq!(count(s.last_events(), "discardPrevented"), 1);
        }

        #[test]
        fn r800_in_the_graveyard_it_guards_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [VANILLA, VANILLA, VANILLA], "graveyard": [DISRUPTOR] },
                "p2": { "hand": [BOOK_OF_PAIN, FILLER], "mana": 9 },
            }));

            s.play(BOOK_OF_PAIN, json!({}));

            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(count(s.last_events(), "discardPrevented"), 0);
        }

        #[test]
        fn radiant_cry_draws_1_and_gains_2_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": DISRUPTOR, "radiant": true }, FILLER],
                    "mana": 9,
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(DISRUPTOR, json!({}));

            // One played, one drawn: the hand holds the filler and the draw.
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(s.pile(P1, "library").len(), 1);
            s.expect_mana(P1, 9);
            s.expect_in_zone(DISRUPTOR, "field");
        }

        #[test]
        fn r386_radiant_draw_and_mana_read_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": DISRUPTOR, "radiant": true }, FILLER],
                    "mana": 9,
                    "library": [VANILLA, VANILLA, VANILLA],
                },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(DISRUPTOR), "draw", 1);
            step_param(s.card_mut(DISRUPTOR), "mana", 1);

            s.play(DISRUPTOR, json!({}));

            assert_eq!(s.hand(P1).len(), 3);
            s.expect_mana(P1, 10);
        }
    }
}
