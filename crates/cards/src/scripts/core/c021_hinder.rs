//! #21 Hinder (SPEC §8.2): "Cast on draw: Your opponent has 1 less mana next turn. Discard 1.",
//! radiant "Cast on draw: Your opponent has 2 less mana next turn." Only the base face discards (R431).
//!
//! Nothing here casts the card or draws again: `staticFlags.castOnDraw` is the whole of that. The
//! engine casts it, repeats the draw up to CAST_ON_DRAW_CHAIN_CAP (R58) and counts the free cast as a
//! card played (R40, R70). The floor is not this card's either: `nextTurnMana` moves
//! `mana.nextTurnMod`, which the refresh adds to §2.3's max, floors at 0 and clears; max mana is untouched.
//!
//! THE DISCARD (R431, R682, R70). "Discard 1" names no "of your choice", so it is random from the
//! caster's hand (R682): no declaration travels in the play action (R81) and a cast on a draw asks
//! nothing as it begins (R70), so no prompt pauses the draw. It comes second, after the mana clause;
//! with an empty hand there is nothing to discard and the mana clause still lands.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-021";

/// One Hinder face: whether the caster discards 1. How much lower the opponent's refresh is, is the
/// declared number `mana` (R386): 1, 2 on the Radiant face.
fn hinder(discards: bool) -> Script {
    let static_flags = Some(StaticFlags {
        cast_on_draw: Some(true),
        ..StaticFlags::default()
    });
    if !discards {
        return Script {
            static_flags,
            cry: Some(hook(|ctx| {
                vec![next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "mana"), "player": "enemy" })))]
            })),
            ..Script::default()
        };
    }
    Script {
        static_flags,
        // R682: "Discard 1" — one random card of the caster's own hand.
        cry: Some(hook(|ctx| {
            vec![
                next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "mana"), "player": "enemy" }))),
                discard_random(json_as(json!({ "count": 1 }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: hinder(true),
        radiant: hinder(false),
    }
}

// #21 Hinder — SPEC §8.2, BUILD M4-T4 row 21: auto-casts on draw and draws again; the opponent's next
// refresh −1 floored at 0; counts as played (R40, R70); radiant −2. The base face's caster discards 1
// at random with no prompt, nothing with an empty hand (R431, R682); the Radiant face discards nothing.
// R635 keeps it out of the opening deal and the mulligan, so the "real game" below proves the deal.
//
// The default board is turn 9 with p1 active, both sides at MAX_MANA (4/4): 4 − 1 = 3 base, 4 − 2 = 2
// radiant. The floor needs a refresh below 2, so that fixture starts at turn 1 and p2's first refresh
// is 1: 1 − 2 floors at 0 (§2.3). Hinder lowers the refresh, not max mana.
// Both sides keep a unit on the board and a card in hand, or the engine would auto-end the turns.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HINDER: &str = "core-021";
    /// Tempo Timmy: drawn into a hand it does nothing, and on the board it keeps the turn alive.
    const CONTROL: &str = "core-011";
    const P1_FILLER: &str = "core-016"; // Hit Job, a spell with no hand trigger.
    const P1_SECOND: &str = "core-044"; // True Strike, the other card a choice would weigh.
    const P2_FILLER: &str = "core-005"; // Stockpile, likewise.

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    fn hinder_on_top(seed: &str, is_radiant: bool, turn: Option<i32>, hand: &[&str]) -> Scenario {
        let mut options = json!({
            "seed": seed,
            "p1": {
                "hand": hand,
                "field": [CONTROL],
                "library": [{ "def": HINDER, "radiant": is_radiant }, CONTROL, CONTROL, CONTROL],
            },
            "p2": { "hand": [P2_FILLER], "field": [CONTROL], "library": [P2_FILLER, P2_FILLER, P2_FILLER] },
        });
        if let Some(turn) = turn {
            options["turn"] = json!(turn);
        }
        scenario(options)
    }

    /// Take p1's turn draw (two `endTurn`s from p1's main phase), which casts the Hinder on top.
    fn draw_hinder(mut s: Scenario) -> Scenario {
        s.end_turn(); // p2's turn.
        s.end_turn(); // p1's turn: the draw that casts Hinder.
        s
    }

    mod n21_hinder {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r431_r682_the_cast_asks_nothing_no_prompt_opens_and_the_draw_goes_on() {
                crate::register_all();
                let mut s = draw_hinder(hinder_on_top("hinder-asks", false, None, &[P1_FILLER]));

                // No question stops the draw: the random discard landed and the draw repeated.
                assert!(s.state().pending.is_none());
                assert!(s.view(P2).pending.is_none());
                // The filler went to the graveyard as a discard, and the next card is in the hand.
                s.expect_in_zone(P1_FILLER, "graveyard");
                assert_eq!(def_ids(&s.hand(P1)), vec![CONTROL.to_string()]);
                s.expect_in_zone(HINDER, "graveyard");
            }

            #[test]
            fn r58_r70_it_discards_a_random_card_draws_again_and_never_reaches_the_hand() {
                crate::register_all();
                let mut s = draw_hinder(hinder_on_top("hinder-cast", false, None, &[P1_FILLER]));

                // R682: the random card went to the graveyard as a discard.
                s.expect_in_zone(P1_FILLER, "graveyard");
                s.expect_events(json!(["drawn", "cardPlayed", "discarded", "drawn"]));
                // R58: the cast-on-draw card is cast, the draw repeats and the next card goes to the hand.
                assert_eq!(def_ids(&s.hand(P1)), vec![CONTROL.to_string()]);
                // §5.1: a Spell that has resolved is in the graveyard.
                s.expect_in_zone(HINDER, "graveyard");
            }

            #[test]
            fn r682_r431_the_discard_is_random_of_two_cards_one_goes_and_one_stays() {
                crate::register_all();
                let s = draw_hinder(hinder_on_top("hinder-choice", false, None, &[P1_FILLER, P1_SECOND]));

                assert!(s.state().pending.is_none());
                // The draw repeated past the cast, so the hand holds the survivor and the next card.
                assert_eq!(s.hand(P1).len(), 2);
                assert!(def_ids(&s.hand(P1)).contains(&CONTROL.to_string()));
                // The graveyard holds the resolved Hinder and the one random discard.
                let grave = def_ids(&s.pile(P1, "graveyard"));
                assert!(grave.contains(&HINDER.to_string()));
                let discarded: Vec<String> = grave.iter().filter(|def_id| *def_id != HINDER).cloned().collect();
                assert_eq!(discarded.len(), 1);
                assert!([P1_FILLER, P1_SECOND].contains(&discarded[0].as_str()));
                let survivor = s
                    .hand(P1)
                    .iter()
                    .find(|card| card.def_id != CONTROL)
                    .map(|card| card.def_id.clone());
                assert!(survivor.as_deref().is_some_and(|id| [P1_FILLER, P1_SECOND].contains(&id)));
                assert_ne!(survivor.as_deref(), Some(discarded[0].as_str()));
            }

            #[test]
            fn r682_the_random_discard_comes_from_the_match_rng_the_same_game_discards_the_same_card() {
                crate::register_all();
                let first = draw_hinder(hinder_on_top("hinder-roundtrip", false, None, &[P1_FILLER, P1_SECOND]));
                let second = draw_hinder(hinder_on_top("hinder-roundtrip", false, None, &[P1_FILLER, P1_SECOND]));
                let ids = |s: &Scenario| -> Vec<String> {
                    s.events()
                        .iter()
                        .filter_map(|event| match event {
                            GameEvent::Discarded { instance_id, .. } => Some(instance_id.clone()),
                            _ => None,
                        })
                        .collect()
                };
                assert_eq!(ids(&first), ids(&second));
            }

            #[test]
            fn r431_r90_with_an_empty_hand_there_is_nothing_to_discard_no_prompt_and_the_rest_still_lands() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "hinder-empty",
                    "p1": { "field": [CONTROL], "library": [HINDER, CONTROL, CONTROL, CONTROL] },
                    "p2": { "hand": [P2_FILLER], "field": [CONTROL], "library": [P2_FILLER, P2_FILLER, P2_FILLER] },
                }));
                s.start_turn(); // p1's draw, with nothing in hand.

                assert!(s.state().pending.is_none());
                assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
                assert_eq!(s.state().players.p2.mana.next_turn_mod, -1);
                // The draw repeated: the next card is in the empty hand.
                assert_eq!(def_ids(&s.hand(P1)), vec![CONTROL.to_string()]);
                s.expect_in_zone(HINDER, "graveyard");
            }

            #[test]
            fn r40_r70_counts_as_a_card_played_this_turn_at_cost_0() {
                crate::register_all();
                let s = draw_hinder(hinder_on_top("hinder-played", false, None, &[P1_FILLER]));

                let played: Vec<&GameEvent> = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == HINDER))
                    .collect();
                assert_eq!(played.len(), 1);
                assert!(matches_object(
                    &serde_json::to_value(played[0]).unwrap(),
                    &json!({ "player": "p1", "costPaid": 0 })
                ));
                // R40: the cast is in the turn log every Combo card counts.
                let hinder_id = s.card(HINDER).id.clone();
                assert!(s.state().players.p1.turn_log.played_ids.contains(&hinder_id));
            }

            #[test]
            fn the_opponent_has_1_less_mana_next_turn() {
                crate::register_all();
                let mut s = draw_hinder(hinder_on_top("hinder-refresh", false, None, &[P1_FILLER]));
                s.end_turn(); // p2's turn: the lowered refresh.

                assert_eq!(s.state().active, P2);
                s.expect_mana(P2, 3);
                assert_eq!(s.view(P2).you.mana, ManaView { current: 3, max: 4 });
            }

            #[test]
            fn the_modifier_is_one_shot_the_refresh_after_that_is_back_to_4_s2_3() {
                crate::register_all();
                let mut s = draw_hinder(hinder_on_top("hinder-oneshot", false, None, &[P1_FILLER]));
                s.end_turn(); // p2's lowered refresh.
                s.expect_mana(P2, 3);
                s.end_turn(); // p1.
                s.end_turn(); // p2 again, with nothing owed.

                s.expect_mana(P2, 4);
                assert_eq!(s.state().players.p2.mana.next_turn_mod, 0);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_opponent_has_2_less_mana_next_turn() {
                crate::register_all();
                let mut s = draw_hinder(hinder_on_top("hinder-radiant", true, None, &[P1_FILLER]));
                s.end_turn();

                s.expect_mana(P2, 2);
                assert_eq!(s.view(P2).you.mana, ManaView { current: 2, max: 4 });
            }

            #[test]
            fn r431_discards_nothing_and_asks_nothing_casts_itself_on_draw_and_draws_again() {
                crate::register_all();
                let mut s = draw_hinder(hinder_on_top("hinder-radiant-cast", true, None, &[P1_FILLER]));

                assert!(s.state().pending.is_none());
                assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
                s.expect_in_zone(P1_FILLER, "hand");
                assert!(!def_ids(&s.hand(P1)).contains(&HINDER.to_string()));
                s.expect_in_zone(HINDER, "graveyard");
                s.expect_events(json!(["drawn", "cardPlayed", "drawn"]));
            }

            #[test]
            fn s2_3_the_refresh_floors_at_0_rather_than_going_negative() {
                crate::register_all();
                // Turn 1: p2 has started no turn yet, so their first refresh is 1 and −2 would be −1.
                let mut s = hinder_on_top("hinder-floor", true, Some(1), &[P1_FILLER]);
                s.start_turn(); // p1's draw, which casts Hinder; the turn does not change hands.
                assert_eq!(s.state().players.p2.mana.next_turn_mod, -2);

                s.end_turn(); // p2's first turn: the refresh.
                assert_eq!(s.state().active, P2);
                s.expect_mana(P2, 0);
                assert_eq!(s.view(P2).you.mana, ManaView { current: 0, max: 1 });
            }
        }

        #[test]
        fn r386_an_upgrade_in_the_deck_takes_2_mana_and_a_radiant_degrade_1() {
            for (radiant, upgrade, mana) in [(false, true, 2), (true, false, 1)] {
                crate::register_all();
                let mut s = hinder_on_top("hinder-tuned", radiant, None, &[P1_FILLER]);
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, HINDER, "mana")
                } else {
                    crate::degrade_number(&mut s, HINDER, "mana")
                };
                assert_eq!(moved, mana);
                let mut s = draw_hinder(s);
                s.end_turn();
                s.expect_mana(P2, 4 - mana);
            }
        }

        #[test]
        fn r431_both_faces_are_cast_on_draw_neither_declares_a_discard_pick_r682_random() {
            let scripts = script();
            assert_eq!(scripts.base.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
            assert_eq!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
            assert!(scripts.base.targets.is_empty());
            // The repeat draw and the 0 floor belong to `drawOne` and `refreshMana`, not to this card.
            assert!(scripts.base.modes.is_empty());
            assert!(scripts.radiant.targets.is_empty());
        }
    }

    // A real game: setup deals no Hinder (R635), so both mulligans open at once over hands that
    // hold none; turn 1's draw is what meets it, discarding at random with no prompt (R682), and
    // the log folds back to the same game (§9.3).

    /// Twenty legal Core cards with no other cast-on-draw card among them, Hinder first.
    const DECK: [&str; 20] = [
        HINDER, "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
        "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
        "core-044", "core-053", "core-055",
    ];

    /// Apply one action with the next nonce, refusing loudly, and log it.
    fn act(state: &GameState, body: ActionBody, player: PlayerId, nonce: &mut u32, log: &mut Vec<Action>) -> GameState {
        *nonce += 1;
        let action = Action::new(body, player, format!("hinder-game-{nonce}"));
        let result = reduce(state, &action);
        if let Some(error) = result.error {
            panic!("{error}");
        }
        log.push(action);
        result.state
    }

    use crate::matches_object;

    mod n21_hinder_in_a_real_game_r635_r682_s9_3 {
        use super::*;

        #[test]
        fn r431_r635_r682_setup_neither_deals_nor_casts_a_hinder_turn_1_s_draw_casts_it_with_a_random_discard_and_no_prompt_and_the_log_replays_to_the_same_state() {
            crate::register_all();
            let deck: Vec<String> = DECK.iter().map(|id| id.to_string()).collect();
            let mut nonce = 0u32;
            // A seed whose shuffle-in puts the Hinder on top of p1's library, so turn 1's draw casts it.
            let mut found: Option<(String, GameState, Vec<Action>)> = None;
            let mut at = 0;
            while at < 300 && found.is_none() {
                let seed = format!("hinder-deal-{at}");
                let mut log: Vec<Action> = Vec::new();
                let created = create_game(&CreateGameOptions {
                    seed: seed.clone(),
                    decks: (deck.clone(), deck.clone()),
                    ..CreateGameOptions::default()
                });
                let mut state = begin_game(&created).state;

                // §2.1, R635: setup casts nothing and asks nothing, so the mulligans open at once, over a hand
                // that holds no Hinder, with the Hinder waiting in the library.
                assert!(state.pending.is_none());
                assert!(state.mulligan.is_some());
                assert!(!def_ids(&state.players.p1.hand).contains(&HINDER.to_string()));
                assert!(state.players.p1.graveyard.is_empty());
                assert!(def_ids(&state.players.p1.library).contains(&HINDER.to_string()));

                for player in [P1, P2] {
                    let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
                    state = act(&state, ActionBody::Mulligan { keep }, player, &mut nonce, &mut log);
                }
                assert_eq!(state.turn, 1);
                // R682: the cast discards at random and asks nothing, so a turn-1 cast leaves the Hinder and
                // one discard in the graveyard with no prompt open.
                if def_ids(&state.players.p1.graveyard).contains(&HINDER.to_string()) {
                    found = Some((seed, state, log));
                }
                at += 1;
            }
            let Some((seed, state, log)) = found else {
                panic!("no seed in hinder-deal-0..299 puts the Hinder on top of p1's library");
            };

            assert!(state.pending.is_none());
            let grave = def_ids(&state.players.p1.graveyard);
            assert_eq!(grave.iter().filter(|def_id| *def_id == HINDER).count(), 1);
            assert_eq!(grave.len(), 2);
            // The draw repeated past the cast: the hand is full again.
            assert_eq!(state.players.p1.hand.len(), 3);
            assert_eq!(state.turn, 1);

            let replayed = fold(&json_as::<FoldArgs>(json!({
                "seed": seed,
                "decks": [deck, deck],
                "log": log,
            })));
            assert!(replayed.errors.is_empty());
            assert_eq!(hash_state(&replayed.state), hash_state(&state));
        }
    }
}
