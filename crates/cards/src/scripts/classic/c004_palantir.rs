//! C #4 Palantir (SPEC §8.6 row 4). Field Spell, cost 2, Legendary.
//!   Base:    "Aura: Your opponent draws at most {drawLimit|card|cards} a turn.
//!             When your opponent plays a (1) Cost Spell, Tribute this to steal and cast it."
//!   Radiant: "Aura: Your opponent draws at most {drawLimit|card|cards} a turn.
//!             When your opponent plays a (1) Cost Spell, steal and cast it. Once it has stolen
//!             {threshold}, Tribute this."
//!
//! The Aura is a draw limit (§2.4, B5 E3) on the opponent, on every turn, the start-of-turn draw
//! included: a draw past the limit does not happen at all (no card moves, no fatigue, nothing is cast
//! on draw), and with several limits the lowest holds (`Script.drawLimit`, read by `draw.ts` while this
//! card stands on the field; it goes when Palantir leaves).
//!
//! The steal answers the opponent's announce (`cardAnnounced`, §10.5 step 3a) as a Counter (§6.3, B5
//! E1, R448): the card is cancelled before it moves and goes to your hand as your own card (B5 E2, the
//! Steal off the field: its owner changes, §3.2, R12; a full hand burns it into your graveyard, R317),
//! and then you cast it (R831001): a cast by Palantir's controller (R70), every choice made at random
//! and each target aimed in your favour (R452, R656), after which it goes to your graveyard as your
//! card. A cast is a play and is answered too (R70). The Spells it answers are the (1) Cost ones, their
//! own cost as they stand out of play (R65: an X Spell counts 0, R396), Books or not.
//!   - Base: "Tribute this to steal and cast it" is a price paid without asking (no "you may", balance patch 1):
//!     Palantir sacrifices itself, then counters the Spell and casts it. Palantir is not a Trap, so it
//!     answers after the traps the announce woke (§10.3), and the paused play waits on `state.work`
//!     meanwhile (R113).
//!   - Radiant: every such Spell, with no question, counted in `memory.stolen`; once it has stolen
//!     ({threshold}) Palantir Tributes itself — never before.
//!
//! Patch v0.3.10 (#831): it cost (1) and answered only Books (the Radiant face every Spell, until the
//! costs it had stolen reached (2)), and it kept the stolen card in your hand.

use jackioh_engine::effects::{cast, counter_play, remember, sacrifice};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-004";

/// Where the Radiant face counts the Spells it has stolen so far.
const STOLEN: &str = "stolen";

/// Patch v0.3.10 (#831): the Spells it answers are the (1) Cost ones (R65: a card's own cost as it
/// stands out of play, an X Spell 0), Books or not.
const STOLEN_COST: i32 = 1;

/// The opponent's announced (1) Cost Spell, or None: the announced card's instance id, which is what
/// this answers.
fn answered(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::CardAnnounced { player, instance_id, card_type, .. } = event else {
        return None;
    };
    if *player == ctx.controller || *card_type != CardType::Spell {
        return None;
    }
    let cost = find_instance(&*ctx.state, instance_id).and_then(|card| own_cost(&*ctx.state, card));
    if cost != Some(STOLEN_COST) {
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

/// "Steal it and cast it yourself": countered into your hand, then cast by you, every choice at
/// random but aimed in your favour (R831001, R452, R656).
fn steal_and_cast(spell: &str) -> Vec<Effect> {
    let card = json!({ "of": "instance", "instanceId": spell });
    vec![
        counter_play(json_as(json!({ "to": "thief", "target": card.clone() }))),
        cast(json_as(json!({ "target": card, "random": true, "targetEnemies": true }))),
    ]
}

/// Base: "Tribute this to steal and cast it" — no question, the price is paid to steal.
fn steal_one() -> TriggerDef {
    TriggerDef::new("palantir-steal", &[GameEventType::CardAnnounced], |ctx, event| {
        let Some(spell) = answered(ctx, event) else {
            return vec![];
        };
        let mut effects = vec![sacrifice(json_as(json!({ "target": { "of": "self" } })))];
        effects.extend(steal_and_cast(&spell));
        effects
    })
    .with_when(|ctx, event| answered(ctx, event).is_some())
}

/// Radiant: steal each one, count them, and Tribute this once it has stolen {threshold}.
fn steal_some() -> TriggerDef {
    TriggerDef::new("palantir-spells", &[GameEventType::CardAnnounced], |ctx, event| {
        let Some(spell) = answered(ctx, event) else {
            return vec![];
        };
        let before = recalled(ctx, STOLEN);
        // The count so far, 0 before the first steal.
        let total = before.as_ref().and_then(Value::as_f64).map_or(0, |n| n as i32) + 1;
        let mut effects = steal_and_cast(&spell);
        effects.push(remember(json_as(json!({ "key": STOLEN, "value": total }))));
        if total >= param(&*ctx, "threshold") {
            effects.push(sacrifice(json_as(json!({ "target": { "of": "self" } }))));
        }
        effects
    })
    .with_when(|ctx, event| answered(ctx, event).is_some())
}

pub fn script() -> CardScripts {
    let limit = draw_limit();
    let base = Script {
        draw_limit: Some(limit.clone()),
        triggers: vec![steal_one()],
        ..Script::default()
    };

    let radiant = Script {
        draw_limit: Some(limit),
        triggers: vec![steal_some()],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #4 Palantir — SPEC §8.6 row 4, BUILD M9 Classic row C 4: "Aura: the opponent's draws beyond 1 in a
// turn, on either player's turn and the start-of-turn draw included, do not happen at all: no card
// moves, no fatigue, nothing is cast on draw (§2.4); a draw made earlier that turn counts; with another
// limit the lowest holds; lifted when Palantir leaves; the opponent playing a (1) Cost Spell (a cast
// included, R70) opens the announce window (§10.5), where the base face steals with no "you may":
// Palantir is Tributed, the Spell is countered (never played by them: not counted, its mana spent)
// and you cast it, every choice at random and aimed your way (R831001), the card ending in your
// graveyard as yours; no prompt opens; another Spell opens no prompt either; the resolved steal
// survives a JSON round trip and replays; radiant: every such Spell is stolen and cast with no
// prompt, and Palantir Tributes itself once it has stolen 3, not before; its tuned numbers (draw
// limit, never below 1; radiant threshold) read through `param()` (R386)".
//
// Palantir sits face-up in p1's backrow; p2, the opponent, is active and plays or draws. The (1) Cost
// Spells are C #24 Book of Knowledge ("Draw 3") and #5 Stockpile ("Draw 2. Heal your hero 2."), whose
// resolution shows in whoever's draws.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PALANTIR: &str = "classic-004";
    const BOOK: &str = "classic-024"; // (1) Spell, Book: Draw 3.
    const FLAME: &str = "classic-016"; // (1) Spell, Book: Trample. Deal 4 damage (a declared target).
    const VIRUS: &str = "core-090-1"; // (1) Spell token: Pierce. Cast on draw: take 1 damage; copies.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const REPLENISH: &str = "core-010"; // (0) Spell.
    const DIVIDEND: &str = "core-024"; // (X) Spell.
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw.
    const VANILLA: &str = "core-008";
    const TIMMY: &str = "core-011";
    const TWINSPELL: &str = "core-079"; // (2) Field Spell.
    const CALL: &str = "core-069"; // (2) Spell: Recruit 3.
    const COUNTERSPELL: &str = "classic-017"; // Trap: counters the opponent's Spell.
    const GAMBIT: &str = "classic-052"; // Trap: reveals at a lethal hit on your hero; redirects it, heals 10, draws 3.

    const DECK: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

    use crate::js;

    use crate::matches_object;

    use crate::merged;

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": merged(
                json!({
                    "hand": [TIMMY],
                    "backrow": [{ "def": PALANTIR, "radiant": radiant_face, "faceUp": true, "lane": 1 }],
                    "library": DECK,
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [BOOK, STOCKPILE, VANILLA], "library": DECK }), p2),
        }))
    }

    /// Stockpile with (1) more on its cost: a draw Spell Palantir does not steal, for the Aura's tests.
    fn dear_stockpile() -> Value {
        json!({ "def": STOCKPILE, "costMod": 1 })
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

    fn stolen(s: &Scenario, palantir: &CardInstance) -> Option<Value> {
        s.card(palantir).memory.get("stolen").cloned()
    }

    /// Whether the card lies in p1's graveyard, p1 its owner.
    fn in_your_graveyard(s: &Scenario, card: &CardInstance) -> bool {
        matches_object(&js(&s.card(&card.id).zone), &json!({ "z": "graveyard", "player": "p1" }))
            && s.card(&card.id).owner == P1
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
                let mut s = setup(json!({}), json!({ "hand": [dear_stockpile(), VANILLA] }), false);

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
                    "p2": { "hand": [dear_stockpile(), VANILLA], "library": DECK },
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
                    json!({ "hand": [dear_stockpile(), VANILLA], "library": [VANILLA, { "def": HINDER, "radiant": true }, VANILLA] }),
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
                let mut s = setup(json!({}), json!({ "hand": [dear_stockpile(), VANILLA], "library": [VANILLA] }), false);

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 1);
                assert_eq!(count(s.last_events(), "fatigue"), 0);
                s.expect_health(P2, 32);
            }

            #[test]
            fn s2_4_it_holds_on_its_controller_s_turn_too_the_opponent_s_off_turn_draw_counts_and_a_second_is_stopped() {
                crate::register_all();
                // p1's Book of Flame would take p2 from 3 to 0, so p2's C #52 Final Gambit reveals on p1's
                // turn and draws p2 3: the first draw happens, the other two are past the limit.
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FLAME, TIMMY],
                        "backrow": [{ "def": PALANTIR, "faceUp": true, "lane": 1 }],
                        "library": [VANILLA],
                    },
                    "p2": {
                        "hand": [VANILLA],
                        "backrow": [{ "def": GAMBIT, "faceUp": false, "lane": 2 }],
                        "library": [VANILLA, VANILLA, VANILLA],
                        "health": 3,
                    },
                }));

                s.play(FLAME, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

                assert_eq!(s.state().active, P1);
                assert_eq!(draws_by(s.events(), P2), 1);
                assert_eq!(count(s.last_events(), "drawLimited"), 2);
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
                    json!({ "hand": [dear_stockpile(), VANILLA] }),
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
                let mut s = setup(json!({}), json!({ "hand": [dear_stockpile(), VANILLA] }), false);
                step_param(s.card_mut(PALANTIR), "drawLimit", 1);

                s.play(STOCKPILE, json!({}));

                assert_eq!(draws_by(s.last_events(), P2), 2);
            }

            #[test]
            fn r386_an_upgrade_never_takes_the_limit_below_1() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [dear_stockpile(), VANILLA] }), false);
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

        mod base_the_steal {
            use super::*;

            #[test]
            fn r448_r831001_the_opponent_s_1_cost_spell_opens_no_prompt_it_is_countered_and_you_cast_it() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let stockpile = s.card(STOCKPILE).clone();
                let palantir = s.card(PALANTIR).clone();

                s.play(&stockpile, json!({}));

                assert!(s.state().pending.is_none());
                assert_eq!(count(s.events(), "promptOpened"), 0);
                s.expect_in_zone(&palantir, "graveyard");
                // Countered for them (never their play), then cast by you: you draw 2 and heal 2.
                assert_eq!(count(s.events(), "countered"), 1);
                assert_eq!(s.state().players.p2.turn_log.cards_played, 0);
                assert_eq!(draws_by(s.events(), P2), 0);
                assert_eq!(draws_by(s.events(), P1), 2);
                s.expect_health(P1, 32);
                s.expect_mana(P2, 3);
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::CardPlayed { player: P1, instance_id, .. } if *instance_id == stockpile.id
                )));
                assert!(in_your_graveyard(&s, &stockpile));
            }

            #[test]
            fn r831001_a_book_is_stolen_as_any_1_cost_spell_and_you_draw_its_3() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let book = s.card(BOOK).clone();

                s.play(&book, json!({}));

                assert_eq!(draws_by(s.events(), P1), 3);
                assert_eq!(draws_by(s.events(), P2), 0);
                assert!(in_your_graveyard(&s, &book));
            }

            #[test]
            fn r831001_r656_a_stolen_damage_spell_is_aimed_at_your_opponent() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [FLAME, VANILLA] }), false);

                // They aim it at your hero; stolen, it is yours, aimed at their side (R656).
                s.play(FLAME, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));

                s.expect_health(P1, 30);
                s.expect_health(P2, 26);
            }

            #[test]
            fn r65_r396_a_0_cost_spell_an_x_spell_and_a_2_cost_spell_are_not_stolen() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [REPLENISH, DIVIDEND, CALL, VANILLA], "mana": 9 }), false);
                let palantir = s.card(PALANTIR).clone();

                s.play(REPLENISH, json!({}));
                s.play(DIVIDEND, json!({ "x": 1, "modes": ["mana"] }));
                s.play(CALL, json!({}));

                s.expect_in_zone(&palantir, "field");
                assert_eq!(count(s.events(), "countered"), 0);
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn r317_with_a_full_hand_the_stolen_spell_burns_into_your_graveyard_and_you_still_cast_it() {
                crate::register_all();
                let ten = vec![VANILLA; 10];
                let mut s = setup(json!({ "hand": ten }), json!({}), false);
                let stockpile = s.card(STOCKPILE).clone();

                s.play(&stockpile, json!({}));

                assert_eq!(s.hand(P1).len(), 10);
                assert!(in_your_graveyard(&s, &stockpile));
                // Cast from your graveyard, where the steal left it: you heal 2.
                s.expect_health(P1, 32);
            }

            #[test]
            fn r97_the_stolen_spell_is_cast_in_the_open_and_both_seats_read_it_in_your_graveyard() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let book = s.card(BOOK).clone();

                s.play(&book, json!({}));

                for viewer in [P1, P2] {
                    let graveyard = if viewer == P1 {
                        js(&s.view(viewer))["you"]["graveyard"].clone()
                    } else {
                        js(&s.view(viewer))["opponent"]["graveyard"].clone()
                    };
                    assert!(graveyard.to_string().contains(BOOK), "{viewer:?}");
                }
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
            fn s9_3_the_resolved_steal_survives_a_json_round_trip_with_the_spell_in_your_graveyard() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                let book = s.card(BOOK).clone();
                s.play(&book, json!({}));
                let done = s.state().clone();
                let revived: GameState = serde_json::from_value(js(&done)).expect("the state round-trips");
                assert_eq!(revived, done);
                assert!(revived.pending.is_none());
                assert!(revived.players.p1.graveyard.iter().any(|card| card.id == book.id));
            }
        }

        mod radiant_up_to_3_spells {
            use super::*;

            #[test]
            fn r831001_steals_and_casts_each_opponent_1_cost_spell_with_no_prompt() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                let spell = s.card(STOCKPILE).clone();
                let palantir = s.card(PALANTIR).clone();

                s.play(&spell, json!({}));

                assert!(s.state().pending.is_none());
                assert!(in_your_graveyard(&s, &spell));
                assert_eq!(draws_by(s.events(), P2), 0);
                assert_eq!(draws_by(s.events(), P1), 2);
                s.expect_in_zone(&palantir, "field");
                assert_eq!(stolen(&s, &palantir), Some(json!(1)));
            }

            #[test]
            fn counts_what_it_stole_and_tributes_itself_once_it_has_stolen_3_not_before() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [STOCKPILE, BOOK, STOCKPILE, VANILLA] }), true);
                let palantir = s.card(PALANTIR).clone();

                let first = s.hand(P2).into_iter().find(|card| card.def_id == STOCKPILE).expect("a Stockpile");
                s.play(&first, json!({}));
                s.expect_in_zone(&palantir, "field");
                s.play(BOOK, json!({}));
                s.expect_in_zone(&palantir, "field");
                assert_eq!(stolen(&s, &palantir), Some(json!(2)));

                let second = s.hand(P2).into_iter().find(|card| card.def_id == STOCKPILE).expect("a Stockpile");
                s.play(&second, json!({}));
                s.expect_in_zone(&palantir, "graveyard");
                assert!(in_your_graveyard(&s, &second));
            }

            #[test]
            fn r65_r396_a_0_cost_spell_and_an_x_spell_are_not_stolen() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [REPLENISH, DIVIDEND, VANILLA] }), true);
                let palantir = s.card(PALANTIR).clone();

                s.play(REPLENISH, json!({}));
                s.play(DIVIDEND, json!({ "x": 1, "modes": ["mana"] }));

                s.expect_in_zone(&palantir, "field");
                assert!(stolen(&s, &palantir).is_none());
            }

            #[test]
            fn r386_an_upgrade_of_its_threshold_lets_it_steal_4_before_it_goes() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "hand": [STOCKPILE, BOOK, STOCKPILE, VANILLA] }), true);
                let palantir = s.card(PALANTIR).clone();
                step_param(s.card_mut(&palantir.id), "threshold", 1);

                for _ in 0..3 {
                    let next = s
                        .hand(P2)
                        .into_iter()
                        .find(|card| card.def_id == STOCKPILE || card.def_id == BOOK)
                        .expect("a (1) Cost Spell");
                    s.play(&next, json!({}));
                }

                s.expect_in_zone(&palantir, "field");
                assert_eq!(stolen(&s, &palantir), Some(json!(3)));
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
                assert!(stolen(&s, &palantir).is_none());
            }

            #[test]
            fn r70_a_1_cost_spell_cast_on_draw_is_stolen_too_and_cast_by_you() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [TIMMY],
                        "backrow": [{ "def": PALANTIR, "radiant": true, "faceUp": true }],
                        "library": [VANILLA, VANILLA],
                    },
                    "p2": { "hand": [VANILLA], "library": [VIRUS, VANILLA] },
                }));
                let virus = s.card(VIRUS).clone();

                s.end_turn();

                // p2 drew the CN-Virus and its cast on draw was stolen: you take its 1, not them.
                assert_eq!(s.state().active, P2);
                s.expect_health(P1, 29);
                s.expect_health(P2, 30);
                assert!(in_your_graveyard(&s, &virus));
            }

            #[test]
            fn r448_a_spell_a_trap_has_already_countered_is_not_stolen_and_adds_nothing() {
                crate::register_all();
                let mut s = setup(
                    json!({ "backrow": [
                        { "def": PALANTIR, "radiant": true, "faceUp": true, "lane": 1 },
                        { "def": COUNTERSPELL, "faceUp": false, "lane": 2 },
                    ] }),
                    json!({}),
                    false,
                );
                let palantir = s.card(PALANTIR).clone();
                let stockpile = s.card(STOCKPILE).clone();

                s.play(&stockpile, json!({}));

                assert!(matches_object(&js(&s.card(&stockpile).zone), &json!({ "z": "graveyard", "player": "p2" })));
                assert_eq!(s.card(&stockpile).owner, P2);
                s.expect_in_zone(&palantir, "field");
                assert!(stolen(&s, &palantir).is_none());
            }

            #[test]
            fn r386_a_degrade_of_its_threshold_makes_it_tribute_itself_after_2() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                let palantir = s.card(PALANTIR).clone();
                step_param(s.card_mut(&palantir.id), "threshold", -1);

                s.play(STOCKPILE, json!({}));
                s.expect_in_zone(&palantir, "field");
                s.play(BOOK, json!({}));

                s.expect_in_zone(&palantir, "graveyard");
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
