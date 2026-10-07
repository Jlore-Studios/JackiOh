//! T-coin The Coin (SPEC §7, §2.1, §2.3; R244, R245). Spell, Token, cost 0.
//!   Base:    "Gain 1 mana this turn"
//!   Radiant: "Gain 2 mana this turn" — §8 Conventions: a cell that changes only a number changes
//!            only that number, as #51.1's "Draw 2" does.
//!
//! No card generates it. §2.1 deals one to the seat going second once both mulligans are answered
//! (R244), which is the engine's setup (`engine/src/setup.ts`, `dealCoins`), not this file: a card
//! file owns what the card does when it is played and nothing about how it got into a hand.
//!
//! The mana is §2.3's temporary mana. `gainMana` (effects/mana.ts) adds to `mana.current` and never
//! touches `max`, so it may go above the cap (4, or a handicapped seat's `manaCap`, R181) and the next
//! refresh sets current back to max, which is all "this turn" means for mana. It lands on the caster,
//! because `gainMana` defaults `player` to "self" (§6.3).
//!
//! Being a token is data, not script, exactly as for the other spell tokens (§7): `token: true` and the
//! `Token` tag keep it out of every deck (§2.6, §9.4 L3, R184), every random pool and Discover (§5.1),
//! and `isUnitToken` is false for a Spell, so it goes to the graveyard when it resolves (R11).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-t-coin";

/// §7: base gains 1, radiant 2.
const BASE_MANA: i32 = 1;
const RADIANT_MANA: i32 = 2;

/// A spell's script is its `cry` hook (§10.9): the on-resolve hook, fired by `runHook`.
fn coin(amount: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![gain_mana(json_as(json!({ "amount": amount })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: coin(BASE_MANA),
        radiant: coin(RADIANT_MANA),
    }
}

// T-coin The Coin (SPEC §7, §2.1, §2.3; R244, R245). BUILD M4-T4's row: "0-cost Spell token dealt to
// the seat going second after the mulligan; gain 1 mana this turn (radiant 2), may exceed the cap;
// never in a deck or a random pool".
//
// Two halves, one file, because both are about this card: what the card does once played (R245,
// through `scenario()` like every card test), and who is dealt it and when (R244). The second half
// is §2.1's setup, which a `scenario()` skips, so it builds real games with `createGame` and
// `beginGame` over the real catalog, the way setup-and-mulligan.test.ts does.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::catalog::query;
    use jackioh_engine::reduce::{begin_game, legal_actions, reduce};
    use jackioh_engine::replay::{FoldArgs, fold, hash_state};
    use jackioh_engine::setup::{deal_coins, mulligan_prompt_for};
    use jackioh_engine::state::create_game;
    use jackioh_engine::testkit::*;
    use jackioh_engine::view_for::view_for;
    use jackioh_engine::zones::{OffFieldZone, move_to_zone};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SEED: &str = "t-coin";

    /// `core-NNN` for NNN in [from, from + count): a run of deckable cards.
    fn run(from: i32, count: i32) -> Vec<String> {
        (0..count).map(|at| format!("core-{:03}", from + at)).collect()
    }

    /// Two legal 20-card decks with no cast-on-draw card (#21, #27, #90.1) and no Quickdraw card (#65,
    /// #84, #98), so the opening hands are exactly §2.1's table and nothing asks during setup (R224).
    fn deck_a() -> Vec<String> {
        run(1, 20)
    }

    fn deck_b() -> Vec<String> {
        run(41, 20)
    }

    /// 30 cards for a Hard seat (R184), from the same card-free-of-casts stretch.
    fn deck_hard() -> Vec<String> {
        run(31, 30)
    }

    /// `OPENING_DRAW[seat] ?? 0`.
    fn opening_draw(seat: usize) -> usize {
        OPENING_DRAW.get(seat).copied().unwrap_or(0) as usize
    }

    struct Game {
        state: GameState,
        log: Vec<Action>,
        events: Vec<GameEvent>,
    }

    /// Every seat keeps its whole opening hand, then the game is at p1's turn 1 (§2.1 step 5).
    fn through_mulligans(decks: (Vec<String>, Vec<String>), handicaps: Option<PerPlayerOpt<Handicap>>) -> Game {
        let created = create_game(&CreateGameOptions {
            seed: SEED.into(),
            decks,
            handicaps,
            ..Default::default()
        });
        let mut state = begin_game(&created).state;
        let mut log: Vec<Action> = Vec::new();
        let mut events: Vec<GameEvent> = Vec::new();
        for player in PLAYER_IDS {
            let pending = mulligan_prompt_for(&state, player);
            assert_eq!(
                pending.as_ref().map(|pending| pending.kind),
                Some(PromptKind::Mulligan),
                "{player}'s mulligan is open"
            );
            assert_eq!(pending.as_ref().map(|pending| pending.player_id), Some(player));
            let action = Action::new(
                ActionBody::Mulligan {
                    keep: state.players[player].hand.iter().map(|card| card.id.clone()).collect(),
                },
                player,
                format!("m-{player}"),
            );
            let result = reduce(&state, &action);
            assert_eq!(result.error, None);
            state = result.state;
            log.push(action);
            events.extend(result.events);
        }
        Game { state, log, events }
    }

    fn coins_in(cards: &[CardInstance]) -> usize {
        cards.iter().filter(|card| card.def_id == COIN_DEF_ID).count()
    }

    /// The ids of a query's answer.
    fn ids<'a>(defs: impl IntoIterator<Item = &'a CardDef>) -> Vec<String> {
        defs.into_iter().map(|card| card.id.clone()).collect()
    }

    use crate::js;

    /// TS's `step`: stamp the body with its player and the next nonce, reduce, and log it.
    fn step(state: &mut GameState, log: &mut Vec<Action>, player_id: PlayerId, action: ActionBody) {
        let stamped = Action::new(action, player_id, format!("s{}", log.len()));
        let result = reduce(state, &stamped);
        assert_eq!(result.error, None);
        *state = result.state;
        log.push(stamped);
    }

    mod t_coin_the_coin_card_data_s7 {
        use super::*;

        #[test]
        fn r245_prints_a_0_cost_token_spell_whose_faces_gain_1_and_2_mana() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.id, COIN_DEF_ID);
            assert_eq!(def.index, "T-coin");
            assert_eq!(def.name, "The Coin");
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(js(&def.cost), json!(0));
            assert!(def.token);
            assert_eq!(def.tags, vec![Tag::Token]);
            assert_eq!(js(&def.rarity), json!("Token"));
            assert_eq!(def.base.text, "Gain 1 mana this turn.");
            assert_eq!(def.radiant.text, "Gain 2 mana this turn.");
            // TS `expect(base).not.toBe(radiant)`: two scripts, two hooks.
            let scripts = script();
            assert!(!Arc::ptr_eq(
                scripts.base.cry.as_ref().unwrap(),
                scripts.radiant.cry.as_ref().unwrap()
            ));
        }

        #[test]
        fn r245_is_never_in_a_deck_creategame_refuses_one_that_holds_it_s2_6_s9_4_l3() {
            crate::register_all();
            let mut deck = run(1, 19);
            deck.push(COIN_DEF_ID.to_string());
            expect_throw_with(
                || {
                    create_game(&CreateGameOptions {
                        seed: SEED.into(),
                        decks: (deck, deck_b()),
                        ..Default::default()
                    });
                },
                "Token card",
            );
        }

        #[test]
        fn r245_is_never_in_a_random_pool_only_a_query_that_names_the_token_pool_reaches_it_s5_1() {
            crate::register_all();
            let coin = COIN_DEF_ID.to_string();
            assert!(!ids(query(&json_as(json!({})))).contains(&coin));
            assert!(!ids(query(&json_as(json!({ "type": "Spell", "cost": 0 })))).contains(&coin));
            assert!(ids(query(&json_as(json!({ "tags": ["Token"] })))).contains(&coin));
            assert_eq!(ids(query(&json_as(json!({ "defId": "core-t-coin" })))), vec![coin]);
        }
    }

    mod t_coin_the_coin_base_gain_1_mana_this_turn {
        use super::*;

        #[test]
        fn r245_gains_1_temporary_mana_above_the_cap_and_max_mana_does_not_move_s2_3() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": SEED, "p1": { "hand": ["core-t-coin", "core-010"] } }));
            let mana = s.state().players.p1.mana;
            assert_eq!((mana.current, mana.max), (4, 4));

            s.play("core-t-coin", json!({}));

            s.expect_mana(P1, 5);
            assert_eq!(s.state().players.p1.mana.max, 4);
            s.expect_events(json!(["cardPlayed", "manaChanged"]));
        }

        #[test]
        fn r245_the_mana_lasts_this_turn_only_the_next_refresh_sets_current_back_to_max_s2_3() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": SEED, "p1": { "hand": ["core-t-coin", "core-010"] } }));
            s.play("core-t-coin", json!({})).expect_mana(P1, 5);

            s.start_turn();

            s.expect_mana(P1, 4);
        }

        #[test]
        fn r245_on_the_second_seat_s_first_turn_it_pays_for_a_2_cost_card_that_1_mana_cannot() {
            crate::register_all();
            // p2's first turn is turn 2: one started turn, so 1 mana (§2.3).
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "turn": 2,
                "p2": { "hand": ["core-t-coin", "core-020"] },
            }));
            let pointmaster = s.card("core-020").clone();
            let playable = |s: &Scenario| -> Vec<String> {
                legal_actions(s.state(), P2)
                    .into_iter()
                    .filter_map(|action| match action {
                        ActionBody::Play { instance_id, .. } => Some(instance_id),
                        _ => None,
                    })
                    .collect()
            };
            assert_eq!(s.state().players.p2.mana.current, 1);
            assert!(!playable(&s).contains(&pointmaster.id));

            s.play("core-t-coin", json!({}));
            assert!(playable(&s).contains(&pointmaster.id));
            s.play(&pointmaster, json!({}));

            s.expect_in_zone(&pointmaster, "field").expect_mana(P2, 0);
        }

        #[test]
        fn r245_is_a_play_and_a_spell_token_goes_to_the_graveyard_when_it_resolves_r11_r70() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": SEED, "p1": { "hand": ["core-t-coin", "core-010"] } }));
            let played = s.state().counters.played;

            s.play("core-t-coin", json!({}));

            s.expect_in_zone("core-t-coin", "graveyard");
            assert_eq!(s.state().players.p1.turn_log.cards_played, 1);
            assert_eq!(s.state().counters.played, played + 1);
        }

        #[test]
        fn r245_n64_gifted_program_makes_it_radiant_as_it_is_played_so_it_gains_2_r213() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": ["core-t-coin", "core-010"], "backrow": ["core-064"] },
            }));

            s.play("core-t-coin", json!({}));

            s.expect_mana(P1, 6);
            assert!(s.card("core-t-coin").radiant);
        }
    }

    mod t_coin_the_coin_radiant_gain_2_mana_this_turn {
        use super::*;

        #[test]
        fn r245_a_radiant_coin_gains_2_s8_conventions_only_the_number_changes() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": "core-t-coin", "radiant": true }, "core-010"] },
            }));

            s.play("core-t-coin", json!({}));

            s.expect_mana(P1, 6);
            assert_eq!(s.state().players.p1.mana.max, 4);
            s.expect_in_zone("core-t-coin", "graveyard");
        }

        #[test]
        fn r245_a_radiant_coin_s_mana_is_temporary_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": "core-t-coin", "radiant": true }, "core-010"] },
            }));
            s.play("core-t-coin", json!({})).expect_mana(P1, 6);

            s.start_turn();

            s.expect_mana(P1, 4);
        }
    }

    mod t_coin_the_coin_r244_the_seat_going_second_is_dealt_it_after_the_mulligan {
        use super::*;

        #[test]
        fn r244_p2_holds_its_opening_hand_plus_the_coin_as_its_last_card_at_turn_1_p1_holds_none() {
            crate::register_all();
            let Game { state, .. } = through_mulligans((deck_a(), deck_b()), None);

            assert_eq!(state.turn, 1);
            assert_eq!(state.active, P1);
            let p2 = &state.players.p2.hand;
            assert_eq!(p2.len(), opening_draw(1) + 1);
            assert_eq!(p2.last().map(|card| card.def_id.as_str()), Some(COIN_DEF_ID));
            assert_eq!(coins_in(p2), 1);
            // p1's opening hand and its turn-1 draw (R10), and no Coin.
            assert_eq!(state.players.p1.hand.len(), opening_draw(0) + 1);
            assert_eq!(coins_in(&state.players.p1.hand), 0);
        }

        #[test]
        fn r244_the_coin_is_not_part_of_the_mulligan_neither_prompt_offers_it_and_it_arrives_after_both() {
            crate::register_all();
            let created = create_game(&CreateGameOptions {
                seed: SEED.into(),
                decks: (deck_a(), deck_b()),
                ..Default::default()
            });
            let mut state = begin_game(&created).state;
            for player in PLAYER_IDS {
                let options = mulligan_prompt_for(&state, player)
                    .map(|pending| pending.options.len())
                    .unwrap_or(0);
                assert_eq!(options, state.players[player].hand.len());
                assert_eq!(
                    coins_in(&state.players.p2.hand),
                    0,
                    "no Coin while {player}'s mulligan is open"
                );
                let result = reduce(
                    &state,
                    &Action::new(ActionBody::Mulligan { keep: vec![] }, player, format!("all-back-{player}")),
                );
                assert_eq!(result.error, None);
                state = result.state;
            }
            // Returning the whole hand shuffled four cards back and drew four; The Coin came after that.
            assert_eq!(coins_in(&state.players.p2.hand), 1);
            assert_eq!(coins_in(&state.players.p2.library), 0);
        }

        #[test]
        fn r244_it_is_an_add_to_hand_not_a_draw_addedtohand_alone_and_n100_s_draw_counter_does_not_move_r55() {
            crate::register_all();
            let Game { state, events, .. } = through_mulligans((deck_a(), deck_b()), None);
            let coin = state.players.p2.hand.iter().find(|card| card.def_id == COIN_DEF_ID);
            assert!(coin.is_some());
            let coin_id = coin.map(|card| card.id.clone()).unwrap_or_default();
            let named: Vec<Value> = events
                .iter()
                .map(js)
                .filter(|event| event["instanceId"] == coin_id.as_str())
                .collect();
            let types: Vec<Value> = named.iter().map(|event| event["type"].clone()).collect();
            assert_eq!(types, vec![json!("addedToHand")]);
            // Four opening draws for p2, three for p1, and p1's turn-1 draw: the Coin is not among them.
            assert_eq!(state.counters.drawn as usize, opening_draw(0) + opening_draw(1) + 1);
        }

        #[test]
        fn r244_p1_sees_a_fifth_card_in_p2_s_hand_and_not_which_one_it_is_r97() {
            crate::register_all();
            let Game { state, .. } = through_mulligans((deck_a(), deck_b()), None);
            let mine = js(&view_for(&state, P2));
            let theirs = js(&view_for(&state, P1));
            assert_eq!(theirs["opponent"]["hand"], json!({ "count": opening_draw(1) + 1 }));
            assert!(!theirs.to_string().contains(COIN_DEF_ID));
            let no_events: Vec<Value> = Vec::new();
            let dealt: Vec<&Value> = theirs["events"]
                .as_array()
                .unwrap_or(&no_events)
                .iter()
                .filter(|event| event["type"] == "addedToHand" && event["player"] == "p2")
                .collect();
            assert!(!dealt.is_empty());
            for event in dealt {
                assert_eq!(event["defId"], "hidden");
            }
            // Its holder reads it like any card in its own hand.
            let hand = mine["you"]["hand"].as_array().cloned().unwrap_or_default();
            let def_ids: Vec<Value> = hand.iter().map(|card| card["defId"].clone()).collect();
            assert!(def_ids.contains(&json!(COIN_DEF_ID)));
        }

        #[test]
        fn r244_a_handicapped_seat_going_second_is_dealt_it_too_after_its_extra_opening_card_r180_r182() {
            crate::register_all();
            let Game { state, .. } = through_mulligans(
                (deck_a(), deck_hard()),
                Some(PerPlayerOpt {
                    p1: None,
                    p2: Some(AI_DIFFICULTY.hard),
                }),
            );
            let hand = &state.players.p2.hand;
            assert_eq!(
                hand.len(),
                opening_draw(1) + AI_DIFFICULTY.hard.extra_opening_cards as usize + 1
            );
            assert_eq!(hand.last().map(|card| card.def_id.as_str()), Some(COIN_DEF_ID));
        }

        #[test]
        fn r244_a_human_seated_second_against_a_handicapped_first_seat_gets_it_and_the_first_seat_none() {
            crate::register_all();
            let Game { state, .. } = through_mulligans(
                (deck_hard(), deck_b()),
                Some(PerPlayerOpt {
                    p1: Some(AI_DIFFICULTY.hard),
                    p2: None,
                }),
            );
            assert_eq!(coins_in(&state.players.p1.hand), 0);
            assert_eq!(coins_in(&state.players.p2.hand), 1);
        }

        #[test]
        fn r244_a_full_hand_burns_it_into_the_graveyard_as_any_card_added_to_one_s2_4_r4() {
            crate::register_all();
            // No Core opening hand reaches HAND_CAP, so the hand is filled by hand before the deal.
            let mut state = create_game(&CreateGameOptions {
                seed: SEED.into(),
                decks: (deck_a(), deck_b()),
                ..Default::default()
            });
            let filling: Vec<CardInstance> =
                state.players.p2.library.iter().take(HAND_CAP as usize).cloned().collect();
            for mut card in filling {
                let _ = move_to_zone(&mut state, &mut card, OffFieldZone::Hand, Default::default());
            }
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
                deal_coins(&mut sink);
            }

            let side = &state.players.p2;
            assert_eq!(side.hand.len(), HAND_CAP as usize);
            assert_eq!(coins_in(&side.hand), 0);
            assert_eq!(coins_in(&side.graveyard), 1);
            let types: Vec<GameEventType> = events.iter().map(GameEvent::event_type).collect();
            assert_eq!(types, vec![GameEventType::Burned, GameEventType::EnteredGraveyard]);
        }

        #[test]
        fn r244_the_game_still_folds_exactly_from_seed_decks_handicaps_log_the_coin_included_s9_3_r187() {
            crate::register_all();
            let decks = (deck_a(), deck_hard());
            let handicaps = PerPlayerOpt {
                p1: None,
                p2: Some(AI_DIFFICULTY.hard),
            };
            let game = through_mulligans(decks.clone(), Some(handicaps.clone()));
            let mut state = game.state;
            let mut log: Vec<Action> = game.log.clone();
            // p1 ends turn 1; p2 plays The Coin on turn 2.
            step(&mut state, &mut log, P1, ActionBody::EndTurn);
            let coin = state.players.p2.hand.iter().find(|card| card.def_id == COIN_DEF_ID);
            assert!(coin.is_some());
            let coin_id = coin.map(|card| card.id.clone()).unwrap_or_default();
            step(
                &mut state,
                &mut log,
                P2,
                json_as(json!({ "type": "play", "instanceId": coin_id, "tributes": [], "targets": [], "modes": [] })),
            );
            // Hard's first turn refreshes to 2 (R181), and The Coin adds 1.
            assert_eq!(state.players.p2.mana.current, 3);

            let replayed = fold(&json_as::<FoldArgs>(json!({
                "seed": SEED,
                "decks": [decks.0, decks.1],
                "handicaps": handicaps,
                "log": log,
            })));
            assert!(replayed.errors.is_empty());
            assert_eq!(hash_state(&replayed.state), hash_state(&state));
            assert_eq!(replayed.state.players.p2.mana.current, 3);
            assert_eq!(coins_in(&replayed.state.players.p2.graveyard), 1);
        }

        #[test]
        fn r244_it_moves_nothing_about_the_turn_order_or_the_cap_p1_takes_turn_1_p2_turn_2_s2_5_r2() {
            crate::register_all();
            let Game { state, .. } = through_mulligans((deck_a(), deck_b()), None);
            assert_eq!(state.turn, 1);
            assert_eq!(state.active, P1);
            let next = reduce(&state, &Action::new(ActionBody::EndTurn, P1, "e1")).state;
            assert_eq!(next.turn, 2);
            assert_eq!(next.active, P2);
            assert_eq!(next.players.p2.turns_started, 1);
        }

        #[test]
        fn r244_the_catalog_the_engine_deals_from_holds_it() {
            crate::register_all();
            assert!(crate::CATALOG.get(COIN_DEF_ID).is_some());
        }
    }
}
