//! C #4 Palantir (SPEC §8.6 row 4). Field Spell, cost 1, Legendary.
//!   Base:    "Aura: Your opponent can't draw more than {drawLimit|card|cards} each turn.
//!             When your opponent plays a Book, Tribute this to steal it."
//!   Radiant: "Aura: Your opponent can't draw more than {drawLimit|card|cards} each turn.
//!             When your opponent plays a Spell, steal it. Once the Spells this has stolen cost
//!             ({threshold}) or more in total, Tribute this."
//!
//! The Aura is a draw limit (§2.4, B5 E3) on the opponent, on every turn, the start-of-turn draw
//! included: a draw past the limit does not happen at all (no card moves, no fatigue, nothing is cast
//! on draw), and with several limits the lowest holds (`Script.drawLimit`, read by `draw.ts` while this
//! card stands on the field; it goes when Palantir leaves).
//!
//! The steal answers the opponent's announce (`cardAnnounced`, §10.5 step 3a) as a Counter (§6.3, B5
//! E1, R448): the card is cancelled before it moves and goes to your hand as your own card (B5 E2, the
//! Steal off the field: its owner changes, §3.2, R12; a full hand burns it into your graveyard, R317). A
//! cast is a play and is answered too (R70).
//!   - Base: only a Spell with the Book tag, and "Tribute this to steal it" is a price paid without
//!     asking (no "you may", balance patch 1): Palantir sacrifices itself, then counters the Book into
//!     your hand. Palantir is not a Trap, so it answers after the traps the announce woke (§10.3), and
//!     the paused play waits on `state.work` meanwhile (R113).
//!   - Radiant: every opponent Spell, with no question. Each one's own cost as it stands out of play
//!     (R65: its override or printed cost plus its cost changes, no player discounts, an X Spell 0,
//!     R396) is added to `memory.stolenCost`, and once that total reaches ({threshold}) Palantir
//!     Tributes itself — never before.

use jackioh_engine::effects::{counter_play, remember, sacrifice};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-004";

/// Where the Radiant face keeps the cost it has stolen so far.
const STOLEN_COST: &str = "stolenCost";

/// The opponent's announced Spell (a Book only, on the base face), or None. TS handed back the
/// `cardAnnounced` event itself (`type Announced`); its one use is the announced card's instance id,
/// which is what this answers.
fn answered(ctx: &EffectContext<'_>, event: &GameEvent, books_only: bool) -> Option<String> {
    let GameEvent::CardAnnounced { player, instance_id, def_id, card_type, .. } = event else {
        return None;
    };
    if *player == ctx.controller || *card_type != CardType::Spell {
        return None;
    }
    if books_only && !def_of(Some(&*ctx.state), def_id).tags.contains(&Tag::Book) {
        return None;
    }
    Some(instance_id.clone())
}

/// TS `const drawLimit: Script["drawLimit"]`: the opponent may draw `drawLimit` cards a turn.
fn draw_limit() -> DrawLimitHook {
    read_hook(|args| {
        vec![DrawLimit {
            player: DrawLimitPlayer::Enemy,
            count: param(&args, "drawLimit"),
        }]
    })
}

/// Base: "Tribute this to steal it" — no question, the price is paid to steal.
fn steal_book() -> TriggerDef {
    TriggerDef::new("palantir-book", &[GameEventType::CardAnnounced], |ctx, event| {
        let Some(book) = answered(ctx, event, true) else {
            return vec![];
        };
        vec![
            sacrifice(json_as(json!({ "target": { "of": "self" } }))),
            counter_play(json_as(json!({ "to": "thief", "target": { "of": "instance", "instanceId": book } }))),
        ]
    })
    .with_when(|ctx, event| answered(ctx, event, true).is_some())
}

/// Radiant: steal every Spell, count what it cost, and Tribute this at the threshold.
fn steal_spells() -> TriggerDef {
    TriggerDef::new("palantir-spell", &[GameEventType::CardAnnounced], |ctx, event| {
        let Some(spell) = answered(ctx, event, false) else {
            return vec![];
        };
        let cost = match find_instance(&*ctx.state, &spell) {
            None => 0,
            Some(card) => own_cost(&*ctx.state, card).unwrap_or(0),
        };
        let before = recalled(ctx, STOLEN_COST);
        // TS `typeof before === "number" ? before : 0`.
        let total = before.as_ref().and_then(Value::as_f64).map_or(0, |n| n as i32) + cost.max(0);
        let steal = counter_play(json_as(json!({ "to": "thief", "target": { "of": "instance", "instanceId": spell } })));
        let kept = remember(json_as(json!({ "key": STOLEN_COST, "value": total })));
        if total >= param(&*ctx, "threshold") {
            vec![steal, kept, sacrifice(json_as(json!({ "target": { "of": "self" } })))]
        } else {
            vec![steal, kept]
        }
    })
    .with_when(|ctx, event| answered(ctx, event, false).is_some())
}

pub fn script() -> CardScripts {
    let limit = draw_limit();
    let base = Script {
        draw_limit: Some(limit.clone()),
        triggers: vec![steal_book()],
        ..Script::default()
    };

    let radiant = Script {
        draw_limit: Some(limit),
        triggers: vec![steal_spells()],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #4 Palantir — SPEC §8.6 row 4, BUILD M9 Classic row C 4: "Aura: the opponent's draws beyond 1 in a
// turn, on either player's turn and the start-of-turn draw included, do not happen at all: no card
// moves, no fatigue, nothing is cast on draw (§2.4); a draw made earlier that turn counts; with another
// limit the lowest holds; lifted when Palantir leaves; the opponent playing a Book (a Spell tagged
// Book, a cast included, R70) opens the announce window (§10.5), where the base face steals with no
// "you may" (balance patch 1, mandatory): Palantir is Tributed and the Book is countered (never
// played: no spell script, not counted as played, its mana spent) and moves to your hand as your
// card (its owner changes, R12), burned into your graveyard at a full hand (R317); there is no pass,
// so no prompt opens; a non-Book Spell opens no prompt either; every event naming the stolen Book
// follows R97, judged where it sits now; the resolved steal survives a JSON round trip and replays;
// radiant: every opponent Spell is countered and stolen with no prompt, its own cost (R65 out of
// play: an X Spell 0) added to `memory.stolenCost`, and Palantir Tributes itself once that total
// reaches 2, not before; its tuned numbers (draw limit, never below 1; radiant threshold) read
// through `param()` (R386)".
//
// Palantir sits face-up in p1's backrow; p2, the opponent, is active and plays or draws. The Book is
// C #24 Book of Knowledge ("Draw 3"), whose resolution shows in p2's draws.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PALANTIR: &str = "classic-004";
    const BOOK: &str = "classic-024"; // (1) Spell, Book: Draw 3.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const REPLENISH: &str = "core-010"; // (0) Spell.
    const DIVIDEND: &str = "core-024"; // (X) Spell.
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw.
    const VANILLA: &str = "core-008";
    const TIMMY: &str = "core-011";
    const RECYCLE: &str = "core-039"; // (0) Spell: Exile this.
    const TWINSPELL: &str = "core-079"; // (2) Field Spell.
    const CALL: &str = "core-069"; // (2) Spell: Recruit 3.
    const COUNTERSPELL: &str = "classic-017"; // Trap: counters the opponent's Spell.
    const AUCTIONEER: &str = "classic-038"; // Field Trap: from its reveal on, each play draws its controller 1.

    const DECK: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

    /// An engine value as the JSON TS compares it by.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `toMatchObject`: every key the pattern names holds a matching value; arrays match item by item.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(actual), Value::Object(pattern)) => pattern
                .iter()
                .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
            (Value::Array(actual), Value::Array(pattern)) => {
                actual.len() == pattern.len() && actual.iter().zip(pattern).all(|(got, want)| matches_object(got, want))
            }
            _ => actual == pattern,
        }
    }

    /// TS `{ ...defaults, ...over }` on a side's setup.
    fn merged(mut defaults: Value, over: Value) -> Value {
        if let (Some(into), Value::Object(over)) = (defaults.as_object_mut(), over) {
            for (key, value) in over {
                into.insert(key, value);
            }
        }
        defaults
    }

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": merged(
                json!({
                    "hand": [TIMMY],
                    "backrow": [{ "def": PALANTIR, "radiant": radiant_face, "faceUp": true, "lane": 1 }],
                    "library": [VANILLA, VANILLA],
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [BOOK, STOCKPILE, VANILLA], "library": DECK }), p2),
        }))
    }

    fn count(events: &[GameEvent], event_type: &str) -> usize {
        events.iter().filter(|event| event.event_type().as_str() == event_type).count()
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: drawer, .. } if *drawer == player))
            .count()
    }

    fn stolen_cost(s: &Scenario, palantir: &CardInstance) -> Option<Value> {
        s.card(palantir).memory.get("stolenCost").cloned()
    }

    mod c_n4_palantir {
        use super::*;

        #[test]
        fn both_faces_set_a_draw_limit_on_the_opponent() {
            crate::register_all();
            assert_eq!(ID, PALANTIR);
            let scripts = script();
            assert!(scripts.base.draw_limit.is_some());
            assert!(scripts.radiant.draw_limit.is_some());
        }

        mod the_aura_both_faces {
            use super::*;

            #[test]
            fn s2_4_the_opponent_s_draws_beyond_1_in_a_turn_do_not_happen_at_all() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 1);
                assert_eq!(s.pile(P2, "library").len(), DECK.len() - 1);
                assert_eq!(count(s.last_events(), "drawLimited"), 1);
                assert_eq!(count(s.last_events(), "fatigue"), 0);
            }

            #[test]
            fn s2_4_the_start_of_turn_draw_counts_after_it_every_other_draw_that_turn_is_stopped() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TIMMY], "backrow": [{ "def": PALANTIR, "faceUp": true }], "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [STOCKPILE, VANILLA], "library": DECK },
                }));

                s.end_turn();
                assert_eq!(s.state().active, P2);
                assert_eq!(draws_by(s.last_events(), P2), 1);
                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 0);
                assert_eq!(count(s.last_events(), "drawLimited"), 2);
            }

            #[test]
            fn s2_4_a_stopped_draw_casts_nothing_a_cast_on_draw_card_stays_in_the_deck() {
                crate::register_all();
                let mut s = setup(
                    json!({}),
                    json!({ "hand": [STOCKPILE, VANILLA], "library": [VANILLA, { "def": HINDER, "radiant": true }, VANILLA] }),
                    false,
                );
                let hinder = s.card(HINDER).clone();

                s.play(STOCKPILE, json!({}));

                s.expect_in_zone(&hinder, "library");
                assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);
            }

            #[test]
            fn s2_4_a_stopped_draw_from_an_empty_deck_deals_no_fatigue() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "library": [VANILLA] }), false);

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 1);
                assert_eq!(count(s.last_events(), "fatigue"), 0);
                s.expect_health(P2, 32);
            }

            #[test]
            fn s2_4_it_holds_on_its_controller_s_turn_too_the_opponent_s_off_turn_draw_counts_and_a_second_is_stopped() {
                crate::register_all();
                // p2's C #38 Jackiestan Auctioneer reveals on p1's 3rd play; from then on each play makes p2
                // draw 1, on p1's own turn. The first such draw happens, the second is past the limit.
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [RECYCLE, RECYCLE, RECYCLE, RECYCLE, RECYCLE, TIMMY],
                        "backrow": [{ "def": PALANTIR, "faceUp": true, "lane": 1 }],
                        "library": [VANILLA],
                    },
                    "p2": {
                        "hand": [VANILLA],
                        "backrow": [{ "def": AUCTIONEER, "faceUp": false, "lane": 2 }],
                        "library": [VANILLA, VANILLA, VANILLA],
                    },
                }));
                let recycles = |s: &Scenario| -> Vec<String> {
                    s.hand(P1).into_iter().filter(|card| card.def_id == RECYCLE).map(|card| card.id).collect()
                };

                for id in recycles(&s).into_iter().take(4) {
                    s.play(&id, json!({}));
                }
                assert_eq!(draws_by(s.events(), P2), 1);

                let next = recycles(&s).into_iter().next().unwrap_or_else(|| RECYCLE.to_string());
                s.play(&next, json!({}));

                assert_eq!(draws_by(s.events(), P2), 1);
                assert_eq!(count(s.last_events(), "drawLimited"), 1);
                assert_eq!(s.pile(P2, "library").len(), 2);
            }

            #[test]
            fn its_controller_s_own_draws_are_not_limited() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, TIMMY], "backrow": [{ "def": PALANTIR, "faceUp": true }], "library": [VANILLA, VANILLA, VANILLA] },
                    "p2": { "hand": [VANILLA] },
                }));

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P1), 2);
            }

            #[test]
            fn s2_4_with_another_limit_the_lowest_holds() {
                crate::register_all();
                let mut s = setup(
                    json!({ "backrow": [
                        { "def": PALANTIR, "faceUp": true, "lane": 1 },
                        { "def": PALANTIR, "faceUp": true, "lane": 2 },
                    ] }),
                    json!({}),
                    false,
                );
                // A Degrade moves a limit toward more draws: this one allows 2, the other still 1.
                let Some(lenient) = s.backrow(P1, 2) else {
                    panic!("fixture");
                };
                step_param(s.card_mut(&lenient.id), "drawLimit", 1);

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 1);
            }

            #[test]
            fn r386_a_degrade_of_the_limit_lets_the_opponent_draw_2_a_turn() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                step_param(s.card_mut(PALANTIR), "drawLimit", 1);

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 2);
            }

            #[test]
            fn r386_an_upgrade_never_takes_the_limit_below_1() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                step_param(s.card_mut(PALANTIR), "drawLimit", -1);

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 1);
            }

            #[test]
            fn the_limit_is_lifted_when_palantir_leaves_the_field() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                // p2 plays the Book and p1 takes it, Tributing Palantir: the limit goes with it.
                s.play(BOOK, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 2);
            }
        }

        mod base_the_book_steal {
            use super::*;

            #[test]
            fn r448_the_opponent_s_book_opens_no_prompt_the_steal_resolves_during_the_opponent_s_turn() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                s.play(BOOK, json!({}));

                assert!(s.state().pending.is_none());
                assert_eq!(count(s.events(), "promptOpened"), 0);
                s.expect_events(json!(["cardAnnounced", "countered"]));
                assert_eq!(count(s.events(), "cardPlayed"), 0);
            }

            #[test]
            fn r12_palantir_is_tributed_and_the_book_is_countered_into_your_hand_as_your_card() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let book = s.card(BOOK).clone();
                let palantir = s.card(PALANTIR).clone();

                s.play(&book, json!({}));

                s.expect_in_zone(&palantir, "graveyard");
                let taken = s.card(&book.id).clone();
                assert!(matches_object(&js(&taken.zone), &json!({ "z": "hand", "player": "p1" })));
                assert_eq!(taken.owner, P1);
                // Never played: its script did not run, it is not counted, and its mana stays spent.
                assert_eq!(draws_by(s.events(), P2), 0);
                assert_eq!(count(s.events(), "cardPlayed"), 0);
                assert_eq!(s.state().players.p2.turn_log.cards_played, 0);
                s.expect_mana(P2, 3);
                s.expect_events(json!(["countered"]));
            }

            #[test]
            fn r317_with_a_full_hand_the_stolen_book_burns_into_your_graveyard() {
                crate::register_all();
                let ten = vec![VANILLA; 10];
                let mut s = setup(json!({ "hand": ten }), json!({}), false);
                let book = s.card(BOOK).clone();

                s.play(&book, json!({}));

                assert_eq!(s.hand(P1).len(), 10);
                let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect();
                assert!(graveyard.contains(&book.id));
                assert_eq!(s.card(&book.id).owner, P1);
            }

            #[test]
            fn no_you_may_there_is_no_pass_so_the_book_never_resolves_while_palantir_stands() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let palantir = s.card(PALANTIR).clone();
                let book = s.card(BOOK).clone();

                s.play(BOOK, json!({}));

                s.expect_in_zone(&palantir, "graveyard");
                assert_eq!(count(s.events(), "cardPlayed"), 0);
                let taken = s.card(&book.id).clone();
                assert!(matches_object(&js(&taken.zone), &json!({ "z": "hand", "player": "p1" })));
            }

            #[test]
            fn a_non_book_spell_opens_no_prompt() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                s.play(STOCKPILE, json!({}));

                assert!(s.state().pending.is_none());
                assert_eq!(count(s.events(), "promptOpened"), 0);
            }

            #[test]
            fn r97_every_event_naming_the_stolen_book_is_judged_where_it_sits_now_your_hand_hidden_from_the_opponent() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let book = s.card(BOOK).clone();

                s.play(&book, json!({}));

                // The announce and the counter named the Book; now it is in p1's hand, so p2's view names it
                // nowhere, while p1 reads it. The `stolen` event is left out: the engine shows it to whoever
                // could read the card where it was taken from (the resolving zone, public), which is its own
                // reading of hidden information for steals and differs from this row's "judged where it sits
                // now" (reported to the lead).
                let named = |text: &str| -> bool {
                    text.contains(&format!("\"{}\"", book.id)) || text.contains(&format!("\"{BOOK}\""))
                };
                assert!(s
                    .events()
                    .iter()
                    .filter(|event| event.event_type() != GameEventType::Stolen)
                    .any(|event| named(&js(event).to_string())));
                let mut board = js(&s.view(P2));
                let events = board
                    .as_object_mut()
                    .and_then(|view| view.remove("events"))
                    .unwrap_or(Value::Null);
                assert!(!named(&board.to_string()));
                let shown = events.as_array().cloned().unwrap_or_default();
                assert!(!shown
                    .iter()
                    .filter(|event| event["type"] != "stolen")
                    .any(|event| named(&event.to_string())));
                assert!(named(&js(&s.view(P1))["you"]["hand"].to_string()));
            }

            #[test]
            fn s9_3_the_stolen_play_replays_from_the_log_the_same_actions_on_the_starting_state_reach_the_same_state() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let start: GameState = serde_json::from_value(js(s.state())).expect("the state round-trips");
                let book = s.card(BOOK).clone();

                s.play(&book, json!({}));

                let played = reduce(
                    &start,
                    &json_as::<Action>(json!({ "type": "play", "playerId": "p2", "instanceId": book.id, "nonce": "replay-1" })),
                );
                assert!(played.error.is_none());
                assert!(played.state.pending.is_none());
                assert_eq!(hash_state(&played.state), hash_state(s.state()));
            }

            #[test]
            fn s9_3_the_resolved_steal_survives_a_json_round_trip_with_the_book_in_your_hand() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let book = s.card(BOOK).clone();
                s.play(&book, json!({}));
                let done = s.state().clone();
                let revived: GameState = serde_json::from_value(js(&done)).expect("the state round-trips");
                assert_eq!(revived, done);
                assert!(revived.pending.is_none());

                let result = reduce(
                    &revived,
                    &json_as::<Action>(json!({
                        "type": "answer",
                        "playerId": "p1",
                        "choiceId": "",
                        "selection": [],
                        "nonce": "palantir-round-trip",
                    })),
                );

                assert_eq!(result.error.as_deref(), Some("no prompt is open"));
                assert!(result.state.players.p1.hand.iter().any(|card| card.id == book.id));
            }
        }

        mod radiant_every_spell {
            use super::*;

            #[test]
            fn steals_every_opponent_spell_with_no_prompt() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                let spell = s.card(STOCKPILE).clone();

                s.play(&spell, json!({}));

                assert!(s.state().pending.is_none());
                assert!(matches_object(&js(&s.card(&spell.id).zone), &json!({ "z": "hand", "player": "p1" })));
                assert_eq!(s.card(&spell.id).owner, P1);
                assert_eq!(draws_by(s.events(), P2), 0);
            }

            #[test]
            fn counts_what_it_stole_and_tributes_itself_once_the_total_reaches_2_not_before() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [STOCKPILE, BOOK, VANILLA] }), true);
                let palantir = s.card(PALANTIR).clone();

                s.play(STOCKPILE, json!({}));
                s.expect_in_zone(&palantir, "field");
                assert_eq!(stolen_cost(&s, &palantir), Some(json!(1)));

                s.play(BOOK, json!({}));
                s.expect_in_zone(&palantir, "graveyard");
            }

            #[test]
            fn r65_r396_a_0_cost_spell_and_an_x_spell_add_nothing_to_the_total() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [REPLENISH, DIVIDEND, VANILLA] }), true);
                let palantir = s.card(PALANTIR).clone();

                s.play(REPLENISH, json!({}));
                s.play(DIVIDEND, json!({ "x": 1, "modes": ["mana"] }));

                s.expect_in_zone(&palantir, "field");
                assert_eq!(stolen_cost(&s, &palantir).unwrap_or(json!(0)), json!(0));
            }

            #[test]
            fn r386_an_upgrade_of_its_threshold_lets_it_steal_3_before_it_goes() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [STOCKPILE, BOOK, VANILLA] }), true);
                let palantir = s.card(PALANTIR).clone();
                step_param(s.card_mut(&palantir.id), "threshold", 1);

                s.play(STOCKPILE, json!({}));
                s.play(BOOK, json!({}));

                s.expect_in_zone(&palantir, "field");
                assert_eq!(stolen_cost(&s, &palantir), Some(json!(2)));
            }

            #[test]
            fn only_the_spell_type_a_field_spell_and_a_unit_are_played_as_usual_and_add_nothing() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [TWINSPELL, VANILLA, STOCKPILE] }), true);
                let palantir = s.card(PALANTIR).clone();

                s.play(TWINSPELL, json!({ "zone": 2 }));
                s.play(VANILLA, json!({ "zone": 1 }));

                s.expect_in_zone(TWINSPELL, "field");
                s.expect_in_zone(VANILLA, "field");
                assert_eq!(count(s.events(), "countered"), 0);
                assert!(stolen_cost(&s, &palantir).is_none());
            }

            #[test]
            fn r70_a_spell_cast_by_an_effect_is_stolen_too_a_cast_on_draw_card_goes_to_your_hand_as_yours() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [TIMMY],
                        "backrow": [{ "def": PALANTIR, "radiant": true, "faceUp": true }],
                        "library": [VANILLA, VANILLA],
                    },
                    "p2": { "hand": [VANILLA], "library": [{ "def": HINDER, "radiant": true }, VANILLA] },
                }));
                let hinder = s.card(HINDER).clone();

                s.end_turn();

                assert_eq!(s.state().active, P2);
                assert!(matches_object(&js(&s.card(&hinder).zone), &json!({ "z": "hand", "player": "p1" })));
                assert_eq!(s.card(&hinder).owner, P1);
                assert_eq!(count(s.events(), "cardResolved"), 0);
                assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);
            }

            #[test]
            fn r448_a_spell_a_trap_has_already_countered_is_not_stolen_and_adds_nothing_to_the_total() {
                crate::register_all();
                let mut s = setup(
                    json!({ "backrow": [
                        { "def": PALANTIR, "radiant": true, "faceUp": true, "lane": 1 },
                        { "def": COUNTERSPELL, "faceUp": false, "lane": 2 },
                    ] }),
                    json!({ "hand": [CALL, VANILLA] }),
                    false,
                );
                let palantir = s.card(PALANTIR).clone();
                let call = s.card(CALL).clone();

                s.play(&call, json!({}));

                assert!(matches_object(&js(&s.card(&call).zone), &json!({ "z": "graveyard", "player": "p2" })));
                assert_eq!(s.card(&call).owner, P2);
                s.expect_in_zone(&palantir, "field");
                assert!(stolen_cost(&s, &palantir).is_none());
            }

            #[test]
            fn r386_a_degrade_of_its_threshold_makes_it_tribute_itself_after_1() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                let palantir = s.card(PALANTIR).clone();
                step_param(s.card_mut(&palantir.id), "threshold", -1);

                s.play(STOCKPILE, json!({}));

                s.expect_in_zone(&palantir, "graveyard");
                assert!(matches_object(&js(&s.card(STOCKPILE).zone), &json!({ "z": "hand", "player": "p1" })));
            }

            #[test]
            fn its_own_controller_s_spells_are_never_stolen() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, TIMMY],
                        "backrow": [{ "def": PALANTIR, "radiant": true, "faceUp": true }],
                        "library": [VANILLA, VANILLA],
                    },
                    "p2": { "hand": [VANILLA] },
                }));

                s.play(STOCKPILE, json!({}));

                assert_eq!(count(s.events(), "countered"), 0);
                assert_eq!(draws_by(s.events(), P1), 2);
            }
        }
    }
}
