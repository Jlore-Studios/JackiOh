// Port of `packages/engine/test/setup.test.ts`: setup (M1-T5) and The Coin (R244).

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{new_game, setup_catalog};
use crate::rules::fixtures::scripts::{HERO_POWERS, going_long, heroic_power, hinder};

fn started(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    begin_game(&new_game(seed, decks)).state
}

fn mulligan(keep: Vec<String>, player: PlayerId, nonce: &str) -> Action {
    json_as(json!({ "type": "mulligan", "keep": keep, "playerId": player, "nonce": nonce }))
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].hand.iter().map(|c| c.id.clone()).collect()
}

fn hand_defs(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|c| c.def_id.clone())
        .collect()
}

fn is_hero_power(power: Option<&Value>) -> bool {
    power
        .and_then(Value::as_str)
        .is_some_and(|name| HERO_POWERS.iter().any(|known| known.as_str() == name))
}

mod setup_m1_t5 {
    use super::*;

    #[test]
    fn r265_deals_3_and_4_from_shuffled_libraries_and_opens_both_mulligans_at_once() {
        let state = started("setup", None);
        assert_eq!(state.players.p1.hand.len() as i32, OPENING_DRAW[0]);
        assert_eq!(state.players.p2.hand.len() as i32, OPENING_DRAW[1]);
        assert_eq!(state.players.p1.library.len() as i32, DECK_SIZE - 3);
        assert_eq!(state.players.p2.library.len() as i32, DECK_SIZE - 4);
        assert_eq!(state.phase, Phase::Mulligan);
        // §10.1's one prompt stays free: the two mulligans are their own step.
        assert!(state.pending.is_none());
        assert_eq!(mulligan_owed(&state), vec![PlayerId::P1, PlayerId::P2]);
        for player in [PlayerId::P1, PlayerId::P2] {
            let prompt = mulligan_prompt_for(&state, player).expect("an open mulligan");
            assert_eq!(prompt.kind, PromptKind::Mulligan);
            assert_eq!(prompt.player_id, player);
            let keys: Vec<String> = prompt.options.iter().map(|option| option.key.clone()).collect();
            assert_eq!(keys, hand_ids(&state, player));
        }
    }

    #[test]
    fn shuffles_the_opening_hand_is_not_the_top_of_the_deck_list_on_every_seed() {
        let orders: IndexSet<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|seed| hand_defs(&started(seed, None), PlayerId::P1).join(","))
            .collect();
        assert!(orders.len() > 1);
    }

    #[test]
    fn puts_every_quickdraw_card_in_the_opening_hand_and_draws_that_many_fewer_6_2() {
        // §6.2
        let mut deck = vec![going_long().id, heroic_power().id];
        deck.extend(vanilla_deck(DECK_SIZE - 2, 1));
        let state = started("quickdraw", Some((deck, vanilla_deck(DECK_SIZE, 21))));
        let hand = hand_defs(&state, PlayerId::P1);

        assert!(hand.contains(&going_long().id));
        assert!(hand.contains(&heroic_power().id));
        assert_eq!(hand.len() as i32, OPENING_DRAW[0]);
        assert_eq!(state.players.p1.library.len() as i32, DECK_SIZE - OPENING_DRAW[0]);
        // Two Quickdraw cards and an opening draw of 3 leaves exactly one random draw.
        let others = hand
            .iter()
            .filter(|def_id| **def_id != going_long().id && **def_id != heroic_power().id)
            .count();
        assert_eq!(others, 1);
    }

    #[test]
    fn draws_no_random_cards_when_quickdraw_already_fills_the_opening_hand() {
        let mut quickdraw_deck = vec![going_long().id, heroic_power().id];
        quickdraw_deck.extend(vanilla_deck(DECK_SIZE - 2, 1));
        let state = started("qd-full", Some((vanilla_deck(DECK_SIZE, 21), quickdraw_deck)));
        assert_eq!(state.players.p2.hand.len() as i32, OPENING_DRAW[1]);
    }

    #[test]
    fn r635_a_cast_on_draw_card_sits_out_the_opening_deal_while_other_cards_remain() {
        let mut deck = vec![hinder().id];
        deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
        for seed in ["r635-a", "r635-b", "r635-c"] {
            let state = started(seed, Some((deck.clone(), vanilla_deck(DECK_SIZE, 21))));
            // No cast asks during the deal: both mulligans open at once, on every seed.
            assert!(state.pending.is_none(), "{seed}");
            assert_eq!(mulligan_owed(&state), vec![PlayerId::P1, PlayerId::P2], "{seed}");
            assert!(
                !state.players.p1.hand.iter().any(|c| c.def_id == hinder().id),
                "{seed}"
            );
            assert_eq!(state.players.p1.hand.len() as i32, OPENING_DRAW[0], "{seed}");
            // It waits at the bottom of the library, behind every drawable card, to be shuffled in once
            // the mulligans are done (the all-cast-on-draw fallback lives in setup-aside.test.ts).
            let library: Vec<String> = state
                .players
                .p1
                .library
                .iter()
                .map(|c| c.def_id.clone())
                .collect();
            assert_eq!(library.last(), Some(&hinder().id), "{seed}");
        }
    }

    #[test]
    fn r9_replacements_are_drawn_before_the_returned_cards_are_shuffled_back() {
        for i in 0..100 {
            let state = started(&format!("mull-{i}"), None);
            // p2's mulligan, so turn 1's draw (p1's) does not touch the hand under test.
            let hand = &state.players.p2.hand;
            let keep = vec![hand[0].id.clone()];
            let returned: Vec<String> = hand[1..].iter().map(|c| c.id.clone()).collect();

            let sealed = reduce(&state, &mulligan(keep.clone(), PlayerId::P2, &format!("n{i}")));
            assert!(sealed.error.is_none());
            let after = reduce(
                &sealed.state,
                &mulligan(hand_ids(&state, PlayerId::P1), PlayerId::P1, &format!("k{i}")),
            );
            assert!(after.error.is_none());

            let new_hand = hand_ids(&after.state, PlayerId::P2);
            assert_eq!(new_hand.len(), hand.len());
            assert!(new_hand.contains(&keep[0]));
            for id in &returned {
                assert!(!new_hand.contains(id));
            }
            for id in &returned {
                assert!(after.state.players.p2.library.iter().any(|c| c.id == *id));
            }
            // R9's order, read off the events: every replacement is drawn before the first shuffle-back.
            let own: Vec<GameEventType> = after
                .events
                .iter()
                .filter(|event| match event {
                    GameEvent::Drawn { player, .. } | GameEvent::ShuffledIn { player, .. } => {
                        *player == PlayerId::P2
                    }
                    _ => false,
                })
                .map(|event| event.event_type())
                .collect();
            let first_shuffle = own
                .iter()
                .position(|kind| *kind == GameEventType::ShuffledIn)
                .expect("a shuffle-back");
            assert!(
                own[..first_shuffle]
                    .iter()
                    .all(|kind| *kind == GameEventType::Drawn)
            );
            assert!(
                own[first_shuffle..]
                    .iter()
                    .all(|kind| *kind == GameEventType::ShuffledIn)
            );
            assert_eq!(first_shuffle, returned.len());
        }
    }

    #[test]
    fn r10_r265_waits_for_both_mulligans_then_starts_turn_1_with_a_draw() {
        let state = started("flow", None);
        let first = reduce(
            &state,
            &mulligan(hand_ids(&state, PlayerId::P1), PlayerId::P1, "m1"),
        )
        .state;
        // p1's answer is sealed: nothing moves until p2 has answered too (R266).
        assert!(first.pending.is_none());
        assert_eq!(first.phase, Phase::Mulligan);
        assert_eq!(mulligan_owed(&first), vec![PlayerId::P2]);
        assert_eq!(first.players.p1.hand, state.players.p1.hand);

        let second = reduce(
            &first,
            &mulligan(hand_ids(&first, PlayerId::P2), PlayerId::P2, "m2"),
        )
        .state;

        assert!(second.pending.is_none());
        assert_eq!(second.phase, Phase::Main);
        assert_eq!(second.turn, 1);
        assert_eq!(second.active, PlayerId::P1);
        assert_eq!(second.players.p1.hand.len() as i32, OPENING_DRAW[0] + 1);
        assert_eq!(second.players.p1.mana.current, 1);
        assert_eq!(second.players.p1.mana.max, 1);
    }

    #[test]
    fn r43_heroic_power_rolls_its_power_during_setup_deterministically_from_the_seed() {
        let mut deck = vec![heroic_power().id];
        deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
        let decks = (deck, vanilla_deck(DECK_SIZE, 21));

        let power = |seed: &str| -> Option<Value> {
            let mut state = started(seed, Some(decks.clone()));
            state = reduce(
                &state,
                &mulligan(hand_ids(&state, PlayerId::P1), PlayerId::P1, "a"),
            )
            .state;
            state = reduce(
                &state,
                &mulligan(hand_ids(&state, PlayerId::P2), PlayerId::P2, "b"),
            )
            .state;
            let card = state
                .players
                .p1
                .hand
                .iter()
                .find(|c| c.def_id == heroic_power().id);
            card.and_then(|c| c.memory.get("power").cloned())
        };

        let first = power("power-seed");
        assert!(is_hero_power(first.as_ref()), "{first:?}");
        assert_eq!(power("power-seed"), first);
    }

    #[test]
    fn r43_rolls_a_power_for_a_heroic_power_the_mulligan_returned_to_the_library() {
        let mut deck = vec![heroic_power().id];
        deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
        let mut state = started("mulliganed-power", Some((deck, vanilla_deck(DECK_SIZE, 21))));

        // Quickdraw put it in the opening hand; return it, so it is in the library at start of game.
        let keep: Vec<String> = state
            .players
            .p1
            .hand
            .iter()
            .filter(|c| c.def_id != heroic_power().id)
            .map(|c| c.id.clone())
            .collect();
        assert_eq!(keep.len(), state.players.p1.hand.len() - 1);
        state = reduce(&state, &mulligan(keep, PlayerId::P1, "mp1")).state;
        state = reduce(
            &state,
            &mulligan(hand_ids(&state, PlayerId::P2), PlayerId::P2, "mp2"),
        )
        .state;

        let side = &state.players.p1;
        let card = side
            .library
            .iter()
            .chain(side.hand.iter())
            .find(|c| c.def_id == heroic_power().id);
        assert!(
            card.is_some(),
            "the returned Heroic Power is in the library or back in hand"
        );
        assert!(is_hero_power(card.and_then(|c| c.memory.get("power"))));
    }
}

mod the_coin_r244 {
    use super::*;

    /// A stand-in for the catalog's The Coin: the engine deals the id and reads nothing else of it.
    fn coin() -> CardDef {
        json_as(json!({
            "id": COIN_DEF_ID,
            "index": "T-coin",
            "name": "Fixture Coin",
            "set": "Core",
            "type": "Spell",
            "tags": ["Token"],
            "rarity": "Token",
            "token": true,
            "cost": 0,
            "base": { "keywords": [], "text": "gain 1 mana" },
            "radiant": { "keywords": [], "text": "gain 2 mana" },
        }))
    }

    fn keep_all(state: &GameState) -> GameState {
        let mut next = state.clone();
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep = hand_ids(&next, player);
            next = reduce(&next, &mulligan(keep, player, &format!("coin-{player}"))).state;
        }
        next
    }

    #[test]
    fn r244_deals_each_seat_its_opening_coins_copies_once_both_mulligans_are_answered_as_its_last_card() {
        setup_catalog();
        let mut catalog = registered_catalog().clone();
        catalog.insert(coin().id, coin());
        register_catalog(catalog);
        let created = create_game(&CreateGameOptions {
            seed: "coin".into(),
            decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
            ..Default::default()
        });
        let dealt = begin_game(&created).state;
        assert!(
            !dealt.players.p2.hand.iter().any(|c| c.def_id == COIN_DEF_ID),
            "not during the deal"
        );

        let state = keep_all(&dealt);

        assert_eq!(state.turn, 1);
        for (seat, player) in [PlayerId::P1, PlayerId::P2].into_iter().enumerate() {
            let coins = state.players[player]
                .hand
                .iter()
                .filter(|c| c.def_id == COIN_DEF_ID)
                .count() as i32;
            assert_eq!(coins, OPENING_COINS.get(seat).copied().unwrap_or(0));
        }
        let p2 = &state.players.p2.hand;
        assert_eq!(p2.len() as i32, OPENING_DRAW[1] + 1);
        assert_eq!(p2.last().map(|c| c.def_id.as_str()), Some(COIN_DEF_ID));
        setup_catalog();
    }

    #[test]
    fn r244_a_registered_catalog_without_the_coin_deals_none_which_is_how_the_engines_fixture_catalogs_run() {
        let state = keep_all(&started("no-coin", None));
        assert!(registered_catalog().get(COIN_DEF_ID).is_none());
        assert_eq!(state.players.p2.hand.len() as i32, OPENING_DRAW[1]);
    }
}
