//! M #1 Disruptive Disruptor (SPEC §8.8 row 1, BUILD M10 row M 1). (2) Field Spell, Common.
//!   Base:    "Aura: You can't be forced to discard cards during your opponent's turn."
//!   Radiant: the Aura, plus "Cry: Draw {draw} and gain {mana} mana." (1, 2)
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
    const FORTIFY_MIND: &str = "meditative-022-1"; // (0) Temporary Spell.
    const PICKLE: &str = "classic-008"; // (1) Spell: three questions, "they discard" among them.
    const JAMMED: &str = "core-036"; // (1) Spell: destroy a backrow card.
    const P2: PlayerId = PlayerId::P2;

    use crate::js;
    use jackioh_engine::effects::{discard, discard_from_hand, discard_hand, discard_random, discard_random_cards};
    use jackioh_engine::resolve::{HookOptions, cast_card, make_context};
    use jackioh_engine::triggers::{SettleOptions, settle};

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

        /// Apply one effect as `controller`'s, the way `resolve.rs` runs a hook's effects.
        fn run_as(s: &mut Scenario, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions { controller: Some(controller), ..Default::default() },
                );
                (effect.apply)(&mut ctx);
            }
            s.state_mut().rng_cursor = rng.cursor();
            events
        }

        /// p2 to act, p1 guarded by a face-up Disruptor and holding three cards.
        fn guarded() -> Scenario {
            crate::register_all();
            scenario(json!({
                "active": "p2",
                "p1": { "hand": [VANILLA, VANILLA, VANILLA], "backrow": [backrow(DISRUPTOR, false)] },
                "p2": { "hand": [BOOK_OF_PAIN, FILLER], "mana": 9 },
            }))
        }

        fn ids(cards: &[CardInstance]) -> Vec<String> {
            cards.iter().map(|card| card.id.clone()).collect()
        }

        #[test]
        fn r800_random_chosen_and_whole_hand_discards_move_nothing_and_draw_no_rng() {
            let mut s = guarded();
            let hand = ids(&s.hand(P1));
            let cursor = s.state().rng_cursor;

            let random = run_as(&mut s, discard_random(json_as(json!({ "count": 2, "player": "enemy" }))), P2);
            assert_eq!(random, vec![GameEvent::DiscardPrevented { player: P1, count: 2 }]);
            assert_eq!(s.state().rng_cursor, cursor, "a random discard draws no random number");

            let chosen = run_as(
                &mut s,
                discard(json_as(json!({ "target": { "of": "instance", "instanceId": hand[0] } }))),
                P2,
            );
            assert_eq!(chosen, vec![GameEvent::DiscardPrevented { player: P1, count: 1 }]);

            let whole = run_as(&mut s, discard_hand(json_as(json!({ "player": "enemy" }))), P2);
            assert_eq!(whole, vec![GameEvent::DiscardPrevented { player: P1, count: 3 }]);

            assert_eq!(ids(&s.hand(P1)), hand, "no card moved");
            assert!(s.pile(P1, "graveyard").is_empty());
            // `discardPrevented` is public: both seats see it.
            assert!(js(&GameEvent::DiscardPrevented { player: P1, count: 3 })["player"] == "p1");
        }

        #[test]
        fn r800_r384_r450_a_discard_paid_as_a_price_still_happens() {
            let mut s = guarded();
            assert!(jackioh_engine::draw::discard_guarded(s.state(), P1));
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
            let kept = {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                // An Activate's random discard (R384), then a targeting cost's (R450).
                discard_random_cards(&mut sink, P1, 1);
                let kept = sink.state.players.p1.hand[0].clone();
                discard_from_hand(&mut sink, &kept);
                kept
            };
            assert_eq!(count(&events, "discarded"), 2);
            assert_eq!(count(&events, "discardPrevented"), 0);
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.pile(P1, "graveyard").len(), 2);
            assert!(ids(&s.pile(P1, "graveyard")).contains(&kept.id));
        }

        #[test]
        fn r800_r637_temporarys_discard_at_your_own_end_of_turn_is_untouched() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FORTIFY_MIND, VANILLA],
                    "backrow": [backrow(DISRUPTOR, false)],
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            let temporary = s.card(FORTIFY_MIND).id.clone();

            s.end_turn();

            s.expect_in_zone(&temporary, "graveyard");
            assert_eq!(count(s.events(), "discardPrevented"), 0);
        }

        #[test]
        fn r800_pickle_offers_no_discard_option_against_you() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [VANILLA, VANILLA],
                    "library": [VANILLA, VANILLA],
                    "backrow": [backrow(DISRUPTOR, false)],
                },
                "p2": { "hand": [PICKLE, FILLER], "library": [VANILLA, VANILLA], "mana": 9 },
            }));

            s.play(PICKLE, json!({}));

            let pending = s.state().pending.clone().expect("Pickle's question is open");
            let offered: Vec<String> = pending
                .options
                .iter()
                .map(|option| match &option.selection {
                    Selection::Mode { option } => option.clone(),
                    _ => "?".to_string(),
                })
                .collect();
            assert_eq!(offered, vec!["exile", "draw"]);
        }

        #[test]
        fn r800_leaving_the_field_lifts_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [VANILLA, VANILLA, VANILLA], "backrow": [backrow(DISRUPTOR, false)] },
                "p2": { "hand": [JAMMED, BOOK_OF_PAIN, FILLER], "mana": 9 },
            }));
            let disruptor = s.card(DISRUPTOR).id.clone();

            s.play(JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": disruptor }] }));
            s.expect_in_zone(&disruptor, "graveyard");
            s.play(BOOK_OF_PAIN, json!({}));

            assert_eq!(s.hand(P1).len(), 1, "the discard happens once it has left");
            assert_eq!(count(s.last_events(), "discardPrevented"), 0);
        }

        #[test]
        fn r1_radiant_cry_fires_when_cast() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": DISRUPTOR, "radiant": true }, FILLER],
                    "mana": 3,
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "hand": [FILLER] },
            }));
            let disruptor = s.card(DISRUPTOR).clone();
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                cast_card(&mut sink, &disruptor, Default::default());
                settle(&mut sink, SettleOptions::default());
            }

            s.expect_in_zone(&disruptor, "field");
            // A cast is free: 3 kept, plus the Cry's temporary 2; the draw took one card.
            s.expect_mana(P1, 5);
            assert_eq!(s.pile(P1, "library").len(), 1);
            assert_eq!(s.hand(P1).len(), 2);
        }
    }
}
