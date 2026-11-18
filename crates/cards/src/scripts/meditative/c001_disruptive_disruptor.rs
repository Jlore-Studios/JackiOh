//! M #1 Disruptive Disruptor (SPEC §8.8 row 1, BUILD M10 row M 1). (2) Field Spell, Common.
//!   Base:    "Aura: You can't be forced to discard cards during your opponent's turn.\nActivate: Your
//!            opponent discards a random card."
//!   Radiant: the Aura, plus "Activate: Your opponent discards a random Radiant card, or a random card if
//!            they hold none."
//!
//! The Aura is a discard guard (R800): a pure-read hook beside `draw_limit` (R457), read off the cards
//! acting on the field while it is not the guarded player's turn. An effect's discard from a guarded
//! hand moves nothing, draws no random number (R129) and reports one public `discardPrevented`. A
//! discard paid as a price (an Activate's cost, R384; a targeting cost, R450) never asks the guard.
//! Issue #4154 took its Cry (and `draw` and `mana`) away and gave it an Activate, once a turn and free
//! (R384): an effect's discard of the opponent's (R16's "random", R682), so the opponent's own guard stops
//! it, drawing nothing (R800). The Radiant face's draws among the Radiant cards of that hand, and among
//! all of them when it holds none (R4154207); one card to draw from draws no random number (R129).
//! Through v0.6.6g it had "Cry: Draw {draw}" (Radiant: "and gain {mana} mana", R1), which
//! `script_v0_6_6g` keeps (R767000).

use jackioh_engine::effects::{discard, discard_random, draw, gain_mana};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-001";

/// The Activate's id.
const DISRUPT: &str = "disrupt";

/// "You can't be forced to discard cards during your opponent's turn": the controller's own side,
/// guarded on every turn but their own (`draw::discard_guarded`).
fn disruptor() -> Script {
    Script {
        discard_guard: Some(read_hook(|_args| vec![DrawLimitPlayer::SelfSide])),
        ..Script::default()
    }
}

/// R4154207: the card of the opponent's hand the Radiant face discards: a random Radiant one, else a
/// random one; none from an empty hand. A guarded hand (R800) is drawn from by no random number: its
/// first card is named, and the discard's guard stops it.
fn radiant_pick(c: &mut EffectContext<'_>) -> Vec<String> {
    let opponent = opponent_of(c.controller);
    let hand = zone_cards(&*c.state, opponent, OffFieldZone::Hand);
    if jackioh_engine::draw::discard_guarded(&*c.state, opponent) {
        return hand.into_iter().take(1).map(|card| card.id).collect();
    }
    let radiant: Vec<CardInstance> = hand.iter().filter(|card| card.radiant).cloned().collect();
    let pool = if radiant.is_empty() { hand } else { radiant };
    if pool.len() <= 1 {
        return pool.into_iter().map(|card| card.id).collect();
    }
    c.sink.rng.pick(&pool).map(|card| card.id.clone()).into_iter().collect()
}

/// "Activate: Your opponent discards a random (Radiant) card." (R384, R4154207): once a turn, free.
fn disrupt(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: DISRUPT.to_string(),
        label: if radiant {
            "Your opponent discards a random Radiant card, or a random card if they hold none".to_string()
        } else {
            "Your opponent discards a random card".to_string()
        },
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            if !radiant {
                return vec![discard_random(json_as(json!({ "player": "enemy" })))];
            }
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|c: &mut EffectContext<'_>| radiant_pick(c)),
                each: Arc::new(|instance_id: &str| {
                    discard(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                }),
            })]
        }),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![disrupt(false)],
            ..disruptor()
        },
        radiant: Script {
            activations: vec![disrupt(true)],
            ..disruptor()
        },
    }
}

/// R767000: the scripts through v0.6.6g, the Aura and a Cry: "Draw {draw}" (Radiant: "and gain {mana}
/// mana"), which issue #4154 replaced with the Activate. `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    CardScripts {
        base: Script {
            // "Cry: Draw {draw}" (patch v0.3.10, #831): the declared number `draw` (R386).
            cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
            ..disruptor()
        },
        radiant: Script {
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

// M #1 Disruptive Disruptor — SPEC §8.8 row 1, BUILD M10 row M 1.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const DISRUPTOR: &str = "meditative-001";
    const BOOK_OF_PAIN: &str = "classicplus-056"; // Book of Pain: your opponent discards 2 at random
    const DEVILS_PACT: &str = "classic-023"; // Devil's Pact: Cry discards your own cards at random.
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const FORTIFY_MIND: &str = "meditative-022-1"; // Fortify Mind: Temporary Spell
    const PICKLE: &str = "classic-008"; // Pickle: three deferred questions, "they discard" among them (R1279000)
    const JAMMED: &str = "core-036"; // Magic Jammed: destroy a backrow card
    const P2: PlayerId = PlayerId::P2;

    use crate::js;
    use jackioh_engine::effects::{discard, discard_from_hand, discard_hand, discard_random, discard_random_cards};
    use jackioh_engine::resolve::{HookOptions, cast_card, make_context};
    use jackioh_engine::triggers::{SettleOptions, settle};

    fn backrow(def: &str, radiant: bool) -> Value {
        json!({ "def": def, "radiant": radiant, "faceUp": true, "lane": 1 })
    }

    // Differs from Scenario::count: it counts any event list, and one call passes a list no scenario holds.
    fn count(events: &[GameEvent], event_type: &str) -> usize {
        events.iter().filter(|event| event.event_type().as_str() == event_type).count()
    }

    mod meditative_001 {
        use super::*;
        use crate::ids;

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
            assert!(scripts.base.cry.is_none() && scripts.radiant.cry.is_none(), "no Cry since issue #4154");
            assert_eq!(scripts.base.activations.len(), 1);
            assert_eq!(scripts.radiant.activations.len(), 1);
        }

        #[test]
        fn r800_book_of_pain_on_the_opponents_turn_discards_nothing() {
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
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [VANILLA, VANILLA, VANILLA], "graveyard": [DISRUPTOR] },
                "p2": { "hand": [BOOK_OF_PAIN, FILLER], "mana": 9 },
            }));

            s.play(BOOK_OF_PAIN, json!({}));

            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(count(s.last_events(), "discardPrevented"), 0);
        }

        /// R767000: `run` under the scripts v0.6.6g ran and its declared `draw` (1, Radiant 2) and `mana`
        /// (2, the Radiant face's), which issue #4154 took off.
        fn v0_6_6g<T>(run: impl FnOnce() -> T) -> T {
            crate::under_kept_as(
                ID,
                script_v0_6_6g(),
                |def| {
                    def.params = Some(vec![
                        json_as(json!({ "key": "draw", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1, "read": "cry" })),
                        json_as(json!({
                            "key": "mana", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1,
                            "tunedOn": "radiant", "read": "cry",
                        })),
                    ]);
                },
                run,
            )
        }

        /// p1's Disruptor face-up on its backrow, with mana and a card in hand, against p2's `hand`.
        fn disrupting(radiant: bool, hand: Value) -> Scenario {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "mana": 9, "backrow": [backrow(DISRUPTOR, radiant)], "library": [VANILLA, VANILLA] },
                "p2": { "hand": hand, "library": [VANILLA, VANILLA] },
            }));
            s.state_mut().players.p1.auto_end_turn = Some(false);
            s
        }

        #[test]
        fn r384_r4154207_its_activate_makes_the_opponent_discard_a_random_card_once_a_turn() {
            for radiant in [false, true] {
                let mut s = disrupting(radiant, json!([VANILLA, FILLER, FILLER]));
                let held = ids(&s.hand(P2));
                s.activate(DISRUPTOR, json!({}));
                assert_eq!(s.hand(P2).len(), 2, "radiant {radiant}");
                let gone: Vec<Value> =
                    s.last_events().iter().map(js).filter(|event| event["type"] == "discarded").collect();
                assert_eq!(gone.len(), 1);
                assert_eq!(gone[0]["owner"], "p2");
                assert!(held.contains(&gone[0]["instanceId"].as_str().unwrap_or_default().to_string()));
                assert_eq!(s.hand(P1).len(), 1, "its own hand is untouched");
                s.expect_in_zone(DISRUPTOR, "field");
                s.expect_refused(|s| s.activate(DISRUPTOR, json!({})));
            }
        }

        #[test]
        fn r800_r4154207_the_opponents_own_disruptor_stops_it_and_no_random_number_is_drawn() {
            for radiant in [false, true] {
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "mana": 9, "backrow": [backrow(DISRUPTOR, radiant)], "library": [VANILLA] },
                    "p2": {
                        "hand": [VANILLA, { "def": FILLER, "radiant": true }, VANILLA],
                        "backrow": [backrow(DISRUPTOR, false)],
                        "library": [VANILLA],
                    },
                }));
                s.state_mut().players.p1.auto_end_turn = Some(false);
                let mine = s.backrow(P1, 1).expect("p1's Disruptor").id;
                let cursor = s.state().rng_cursor;
                s.activate(&mine, json!({}));
                assert_eq!(s.hand(P2).len(), 3, "radiant {radiant}");
                assert_eq!(count(s.last_events(), "discardPrevented"), 1);
                assert_eq!(s.state().rng_cursor, cursor, "radiant {radiant}");
            }
        }

        #[test]
        fn r4154207_an_empty_hand_discards_nothing() {
            let mut s = disrupting(true, json!([]));
            s.activate(DISRUPTOR, json!({}));
            assert_eq!(count(s.last_events(), "discarded"), 0);
        }

        #[test]
        fn r4154207_radiant_discards_the_radiant_card_the_hand_holds() {
            let mut s = disrupting(true, json!([VANILLA, { "def": FILLER, "radiant": true }, VANILLA]));
            s.activate(DISRUPTOR, json!({}));
            let left = s.hand(P2);
            assert_eq!(left.len(), 2);
            assert!(left.iter().all(|card| !card.radiant), "the Radiant card goes");
        }

        #[test]
        fn r4154207_radiant_discards_a_random_card_from_a_hand_with_no_radiant_one() {
            let mut s = disrupting(true, json!([VANILLA, FILLER]));
            s.activate(DISRUPTOR, json!({}));
            assert_eq!(s.hand(P2).len(), 1);
            assert_eq!(count(s.last_events(), "discarded"), 1);
        }

        #[test]
        fn r767000_its_cry_draws_1_and_gains_no_mana() {
            v0_6_6g(|| {
                let mut s = scenario(json!({
                    "p1": { "hand": [DISRUPTOR, FILLER], "mana": 9, "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(DISRUPTOR, json!({}));

                // One played, one drawn: the hand holds the filler and the draw.
                assert_eq!(s.hand(P1).len(), 2);
                assert_eq!(s.pile(P1, "library").len(), 1);
                s.expect_mana(P1, 7);
                s.expect_in_zone(DISRUPTOR, "field");
            });
        }

        #[test]
        fn r767000_r386_its_draw_reads_through_param() {
            v0_6_6g(|| {
                let mut s = scenario(json!({
                    "p1": { "hand": [DISRUPTOR, FILLER], "mana": 9, "library": [VANILLA, VANILLA, VANILLA] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(DISRUPTOR), "draw", 1);

                s.play(DISRUPTOR, json!({}));

                assert_eq!(s.hand(P1).len(), 3);
            });
        }

        #[test]
        fn r767000_radiant_cry_draws_2_and_gains_2_mana() {
            v0_6_6g(|| {
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": DISRUPTOR, "radiant": true }, FILLER],
                        "mana": 9,
                        "library": [VANILLA, VANILLA, VANILLA],
                    },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(DISRUPTOR, json!({}));

                // One played, two drawn: the hand holds the filler and the draws.
                assert_eq!(s.hand(P1).len(), 3);
                assert_eq!(s.pile(P1, "library").len(), 1);
                s.expect_mana(P1, 9);
                s.expect_in_zone(DISRUPTOR, "field");
            });
        }

        #[test]
        fn r767000_r386_radiant_draw_and_mana_read_through_param() {
            v0_6_6g(|| {
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": DISRUPTOR, "radiant": true }, FILLER],
                        "mana": 9,
                        "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                    },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(DISRUPTOR), "draw", 1);
                step_param(s.card_mut(DISRUPTOR), "mana", 1);

                s.play(DISRUPTOR, json!({}));

                assert_eq!(s.hand(P1).len(), 4);
                s.expect_mana(P1, 10);
            });
        }

        #[test]
        fn r767000_r1_radiant_cry_fires_when_cast() {
            v0_6_6g(|| {
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
                // A cast is free: 3 kept, plus the Cry's temporary 2; the draw took two cards.
                s.expect_mana(P1, 5);
                assert_eq!(s.pile(P1, "library").len(), 0);
                assert_eq!(s.hand(P1).len(), 3);
            });
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
            scenario(json!({
                "active": "p2",
                "p1": { "hand": [VANILLA, VANILLA, VANILLA], "backrow": [backrow(DISRUPTOR, false)] },
                "p2": { "hand": [BOOK_OF_PAIN, FILLER], "mana": 9 },
            }))
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
        fn r800_pickle_s_deferred_question_opens_on_your_turn_where_the_guard_sleeps() {
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
            // R1279000: no prompt opens for a player who is not active.
            assert!(s.state().pending.is_none());
            s.end_turn();

            let pending = s.state().pending.clone().expect("Pickle's question is open");
            assert_eq!(pending.player_id, P1);
            // The guard stops only discards off the holder's turn: on p1's own turn it
            // sleeps, so "discard" is offered.
            assert_eq!(s.prompt_modes(), vec!["discard", "exile", "draw"]);
            assert_eq!(pending.options.len(), 3, "an option other than a Mode is offered");
        }

        #[test]
        fn r800_leaving_the_field_lifts_it() {
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

    }
}
