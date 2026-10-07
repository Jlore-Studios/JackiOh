//! The concurrent mulligan and the draw offer's lifetime (SPEC §2.1, §2.5, §10.1, §10.8, R265–R269).
//!
//!  - R265: both seats' mulligans open at once, either may answer first, and the second answer
//!    resolves both in seat order — so the game is the same whichever seat answered first, and a
//!    state waiting on one answer survives JSON and replays exactly.
//!  - R266: an answer is sealed: the other seat learns only that it is in.
//!  - R267: a sealed answer is read at resolution, against the hand as it stands then.
//!  - R268: a timed-out mulligan keeps the whole hand, and only the timing-out seat's.
//!  - R269: a draw offer stands until it is answered or its offerer's turn ends, both seats see it,
//!    and one that lapses blocks nothing.
//!
//! Port of `packages/engine/test/mulligan-concurrent.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::effects::{add_to_hand, discard_random};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{new_game, setup_catalog};
use crate::rules::fixtures::scripts::{cn_virus, hinder};

const SEATS: [PlayerId; 2] = [P1, P2];

/// TS's module `let counter`.
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// An `ActionInput` from its TS object literal.
fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn act(state: &GameState, body: ActionInput) -> GameState {
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let action_type = body.body.action_type();
    let player = body.player_id;
    let result = reduce(state, &body.with_nonce(format!("mc-{counter}")));
    if let Some(error) = &result.error {
        panic!("{action_type} for {player}: {error}");
    }
    result.state
}

/// A random subset of the seat's offered cards, drawn from the test's own stream.
fn some_keep(state: &GameState, player: PlayerId, pick: &mut Rng) -> Vec<String> {
    let prompt = mulligan_prompt_for(state, player).unwrap_or_else(|| panic!("{player} owes no mulligan"));
    prompt
        .options
        .iter()
        .map(|option| option.key.clone())
        .filter(|_| pick.chance(0.5))
        .collect()
}

struct Answered {
    state: GameState,
    log: Vec<Action>,
}

/// Both seats answer, in `order`, each with its keep list; the log is what a table would record.
fn answer_in(begun: &GameState, order: &[PlayerId], keep: &PerPlayer<Vec<String>>) -> Answered {
    let mut state = begun.clone();
    let mut log: Vec<Action> = Vec::new();
    for &player in order {
        let action = Action::new(
            ActionBody::Mulligan {
                keep: keep[player].clone(),
            },
            player,
            format!("m-{player}"),
        );
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{player}: {error}");
        }
        state = result.state;
        log.push(action);
    }
    Answered { state, log }
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// R265 the mulligans are open at once
mod r265_the_mulligans_are_open_at_once {
    use super::*;

    #[test]
    fn r265_either_seat_answers_first_and_the_game_dealt_is_the_same_whichever_did() {
        for n in 0..60 {
            let seed = format!("r265-{n}");
            let decks: (Vec<String>, Vec<String>) = (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21));
            let begun = begin_game(&new_game(&seed, Some(decks.clone()))).state;
            assert!(begun.pending.is_none());
            assert_eq!(mulligan_owed(&begun), vec![P1, P2]);

            let mut pick = Rng::new(&format!("r265-pick-{n}"), 0);
            let p1 = some_keep(&begun, P1, &mut pick);
            let p2 = some_keep(&begun, P2, &mut pick);
            let keep = PerPlayer { p1, p2 };
            let first = answer_in(&begun, &[P1, P2], &keep);
            let second = answer_in(&begun, &[P2, P1], &keep);

            // The same game, whichever answered first: the state after setup hashes the same.
            assert_eq!(hash_state(&second.state), hash_state(&first.state), "{seed}");
            assert_eq!(first.state.turn, 1);
            assert_eq!(first.state.phase, Phase::Main);
            assert!(first.state.mulligan.is_none());
            assert_eq!(first.state.mulliganed, vec![P1, P2]);

            // Each order's log folds back to that game (§9.3).
            for answered in [&first, &second] {
                let replayed = fold(&FoldArgs {
                    seed: seed.clone(),
                    decks: decks.clone(),
                    log: answered.log.clone(),
                    ..FoldArgs::default()
                });
                assert!(replayed.errors.is_empty(), "{seed}");
                assert_eq!(hash_state(&replayed.state), hash_state(&first.state));
            }
        }
    }

    #[test]
    fn r265_the_order_does_not_matter_with_cast_on_draw_replacements_and_a_handicapped_seat_either() {
        setup_catalog();
        // Hinder and CN-Virus are cast when drawn, so replacement draws cast, draw the match rng and
        // touch both seats (§2.4, R70); a Hard seat mulligans four cards as p1 (R182). CN-Virus is a
        // token, so it joins each library after the decks are checked.
        let hard = AI_DIFFICULTY.hard;
        let handicaps = PerPlayerOpt {
            p1: Some(hard),
            p2: None,
        };
        let mut casts = 0;
        for n in 0..40 {
            let seed = format!("r265-cast-{n}");
            let mut first_deck = vec![hinder().id];
            first_deck.extend(vanilla_deck(hard.deck_size - 1, 1));
            let decks: (Vec<String>, Vec<String>) = (first_deck, vanilla_deck(DECK_SIZE, 21));
            let mut game = create_game(&CreateGameOptions {
                seed: seed.clone(),
                decks,
                handicaps: Some(handicaps.clone()),
                ..CreateGameOptions::default()
            });
            for seat in SEATS {
                let virus = new_instance(&mut game, &cn_virus().id, seat, Zone::Library { player: seat });
                game.players[seat].library.push(virus);
            }
            let begun = begin_game(&game).state;
            // Every opening card goes back, so each seat draws as many replacements as it can.
            let keep = PerPlayer {
                p1: Vec::new(),
                p2: Vec::new(),
            };
            let first = answer_in(&begun, &[P1, P2], &keep);
            let second = answer_in(&begun, &[P2, P1], &keep);
            assert_eq!(hash_state(&second.state), hash_state(&first.state), "{seed}");
            let played: Vec<&CardInstance> = SEATS
                .iter()
                .flat_map(|&seat| first.state.players[seat].graveyard.iter())
                .collect();
            if played.iter().any(|card| card.def_id == hinder().id || card.def_id == cn_virus().id) {
                casts += 1;
            }
        }
        // Not vacuous: some of these setups cast a replacement.
        assert!(casts > 0);
    }

    #[test]
    fn r265_a_state_waiting_on_one_answer_survives_a_json_round_trip_and_goes_on_exactly_as_the_live_one() {
        let begun = begin_game(&new_game("r265-json", None)).state;
        let waiting = act(&begun, input(json!({ "type": "mulligan", "keep": [], "playerId": "p2" })));
        let copy: GameState =
            serde_json::from_str(&serde_json::to_string(&waiting).expect("serialises")).expect("parses");
        assert_eq!(copy, waiting);
        assert_eq!(legal_actions(&copy, P1), legal_actions(&waiting, P1));
        assert_eq!(legal_actions(&copy, P2), vec![ActionBody::Concede]);
        assert_eq!(seat_to_act(&copy), Some(P1));

        let keep: Vec<String> = ids(&copy.players.p1.hand[1..]);
        let live = reduce(
            &waiting,
            &Action::new(ActionBody::Mulligan { keep: keep.clone() }, P1, "json-p1"),
        );
        let thawed = reduce(&copy, &Action::new(ActionBody::Mulligan { keep }, P1, "json-p1"));
        assert_eq!(hash_state(&thawed.state), hash_state(&live.state));
        assert_eq!(thawed.events, live.events);
    }
}

/// R265 a game that ends while the mulligans are open
mod r265_a_game_that_ends_while_the_mulligans_are_open {
    use super::*;

    #[test]
    fn r265_r216_closes_both_mulligans_with_the_game_so_no_view_offers_one_no_one_can_answer() {
        let begun = begin_game(&new_game("r265-concede", None)).state;
        let waiting = act(&begun, input(json!({ "type": "mulligan", "keep": [], "playerId": "p1" })));
        for (state, who) in [(&begun, P1), (&waiting, P2), (&waiting, P1)] {
            let over = act(state, input(json!({ "type": "concede", "playerId": who })));
            assert_eq!(over.result.map(|result| result.reason), Some(GameOverReason::Concede));
            assert!(over.mulligan.is_none());
            assert_eq!(mulligan_owed(&over), Vec::<PlayerId>::new());
            for seat in SEATS {
                assert!(view_for(&over, seat).pending.is_none());
                assert!(view_for(&over, seat).mulligan.is_none());
                assert_eq!(legal_actions(&over, seat), Vec::<ActionBody>::new());
            }
        }
        // A disconnect or the ceiling ends it the same way.
        let dropped = act(
            &begun,
            input(json!({ "type": "disconnectExpired", "player": "p2", "playerId": "p2" })),
        );
        assert!(dropped.mulligan.is_none());
    }
}

/// R266 an answer is sealed until both are in
mod r266_an_answer_is_sealed_until_both_are_in {
    use super::*;

    #[test]
    fn r266_the_other_seat_sees_that_a_seat_is_ready_and_nothing_of_what_it_kept() {
        let begun = begin_game(&new_game("r266", None)).state;
        let hand: Vec<String> = ids(&begun.players.p1.hand);
        let kept_all = act(&begun, input(json!({ "type": "mulligan", "keep": hand, "playerId": "p1" })));
        let kept_none = act(&begun, input(json!({ "type": "mulligan", "keep": [], "playerId": "p1" })));

        // Nothing moves until p2 answers: p1's hand is as it was dealt.
        assert_eq!(ids(&kept_none.players.p1.hand), hand);

        // p2's view is the same whatever p1 chose (§9.1, rule 7): p1 is ready, and that is all.
        let seen = view_for(&kept_all, P2);
        assert_eq!(view_for(&kept_none, P2), seen);
        assert_eq!(
            seen.mulligan,
            Some(MulliganView {
                you_ready: false,
                opponent_ready: true,
                kept: None
            })
        );
        assert!(matches!(
            &seen.pending,
            Some(PendingView::ForYou(PendingPromptView {
                for_you: true,
                kind: PromptKind::Mulligan,
                ..
            }))
        ));
        assert!(!serde_json::to_string(&seen).expect("serialises").contains("\"kept\""));

        // p1 sees its own answer and waits on p2's, whose options it is never shown.
        let own = view_for(&kept_none, P1);
        assert_eq!(
            own.mulligan,
            Some(MulliganView {
                you_ready: true,
                opponent_ready: false,
                kept: Some(Vec::new())
            })
        );
        assert_eq!(
            own.pending,
            Some(PendingView::Elsewhere(PendingElsewhereView {
                for_you: false,
                pending_for: P2
            }))
        );
        assert_eq!(view_for(&kept_all, P1).mulligan.and_then(|view| view.kept), Some(hand.clone()));

        // An answer is a set (R221): two spellings of one answer are one state.
        let two_cards: Vec<String> = hand[..2].to_vec();
        let mut reversed = two_cards.clone();
        reversed.reverse();
        let forwards = act(&begun, input(json!({ "type": "mulligan", "keep": two_cards, "playerId": "p1" })));
        let backwards = act(&begun, input(json!({ "type": "mulligan", "keep": reversed, "playerId": "p1" })));
        assert_eq!(hash_state(&backwards), hash_state(&forwards));
        assert_eq!(view_for(&backwards, P1).mulligan.and_then(|view| view.kept), Some(two_cards));

        // Before anyone answers, each seat sees its own prompt; after, the window is gone from the view.
        assert_eq!(
            view_for(&begun, P1).mulligan,
            Some(MulliganView {
                you_ready: false,
                opponent_ready: false,
                kept: None
            })
        );
        let done = act(&kept_none, input(json!({ "type": "mulligan", "keep": [], "playerId": "p2" })));
        for seat in SEATS {
            assert!(view_for(&done, seat).mulligan.is_none());
        }
    }
}

/// R267 a sealed answer is read against the hand at resolution
mod r267_a_sealed_answer_is_read_against_the_hand_at_resolution {
    use super::*;

    #[test]
    fn r267_returns_only_the_offered_cards_it_did_not_keep_that_are_still_in_the_hand_a_card_that_arrived_since_stays() {
        setup_catalog();
        // A cast-on-draw card that reaches into the other seat's hand: it discards one of its cards at
        // random and gives it a new one. p1's replacement draw casts it before p2's answer resolves.
        let meddler: CardDef = json_as(json!({
            "id": "fx-r267-meddler",
            "index": "R267",
            "name": "Meddler (fixture)",
            "set": "Core",
            "type": "Spell",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": { "keywords": [], "text": "" },
            "radiant": { "keywords": [], "text": "" },
        }));
        let gift: String = vanilla_deck(1, 39)[0].clone();
        let mut catalog = registered_catalog().clone();
        catalog.insert(meddler.id.clone(), meddler.clone());
        register_catalog(catalog);
        let given = gift.clone();
        let script = Script {
            static_flags: Some(StaticFlags {
                cast_on_draw: Some(true),
                ..StaticFlags::default()
            }),
            cry: Some(hook(move |_ctx| {
                vec![
                    discard_random(json_as(json!({ "player": "enemy" }))),
                    add_to_hand(json_as(json!({ "defId": given, "player": "enemy" }))),
                ]
            })),
            ..Script::default()
        };
        let mut scripts = registered_scripts().clone();
        scripts.insert(
            meddler.id.clone(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        register_scripts(scripts);

        let game = create_game(&CreateGameOptions {
            seed: "r267".to_string(),
            decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
            ..CreateGameOptions::default()
        });
        let mut state = begin_game(&game).state;
        let card = new_instance(&mut state, &meddler.id, P1, Zone::Library { player: P1 });
        state.players.p1.library.insert(0, card);

        let offered: Vec<String> = ids(&state.players.p2.hand);
        let kept = offered[0].clone();
        state = act(&state, input(json!({ "type": "mulligan", "keep": [kept], "playerId": "p2" })));
        // p1 returns one card; its replacement is the meddler, cast as it is drawn (§2.4, R70).
        let p1_keep: Vec<String> = ids(&state.players.p1.hand[1..]);
        state = act(&state, input(json!({ "type": "mulligan", "keep": p1_keep, "playerId": "p1" })));

        let side = &state.players.p2;
        let discarded: Vec<String> = side
            .graveyard
            .iter()
            .filter(|card| offered.contains(&card.id))
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(discarded.len(), 1);
        let arrived = side
            .hand
            .iter()
            .find(|card| card.def_id == gift && !offered.contains(&card.id));
        assert!(arrived.is_some(), "the card the meddler gave p2 is still in its hand");
        assert!(ids(&side.hand).contains(&kept));
        // Returned: the offered cards p2 did not keep, less the one the meddler discarded.
        let returned: Vec<String> = offered
            .iter()
            .filter(|id| **id != kept && !discarded.contains(id))
            .cloned()
            .collect();
        for id in &returned {
            assert!(!side.hand.iter().any(|card| &card.id == id));
            assert!(side.library.iter().any(|card| &card.id == id));
        }
        // The kept card, the new one, a replacement for each returned card, and The Coin if dealt.
        let coins = side.hand.iter().filter(|card| card.def_id == "t-coin").count();
        assert_eq!(side.hand.len(), 2 + returned.len() + coins);
    }
}

/// R268 a mulligan whose clock runs out
mod r268_a_mulligan_whose_clock_runs_out {
    use super::*;

    #[test]
    fn r268_keeps_the_timing_out_seat_s_whole_hand_draws_nothing_from_the_rng_and_never_answers_the_other_seat_s() {
        let begun = begin_game(&new_game("r268", None)).state;
        let hand: Vec<String> = ids(&begun.players.p2.hand);

        let timed = reduce(&begun, &Action::new(ActionBody::Timeout, P2, "r268-p2"));
        assert_eq!(timed.error, None);
        assert_eq!(mulligan_owed(&timed.state), vec![P1]);
        assert_eq!(
            timed.state.mulligan.as_ref().and_then(|seats| seats.p2.keep.clone()),
            Some(hand.clone())
        );
        assert_eq!(timed.state.rng_cursor, begun.rng_cursor);
        assert_eq!(timed.state.turn, 0);

        // A second expiry for a seat that has answered does nothing at all.
        let again = reduce(&timed.state, &Action::new(ActionBody::Timeout, P2, "r268-p2-again"));
        assert_eq!(again.error, None);
        assert_eq!(again.state.mulligan, timed.state.mulligan);
        assert_eq!(again.state.rng_cursor, timed.state.rng_cursor);

        // The other seat's expiry answers its own the same way, and the game begins with both hands.
        let p1_hand: Vec<String> = ids(&begun.players.p1.hand);
        let started = reduce(&again.state, &Action::new(ActionBody::Timeout, P1, "r268-p1"));
        assert_eq!(started.error, None);
        assert!(!started
            .events
            .iter()
            .any(|event| event.event_type() == GameEventType::ShuffledIn));
        assert_eq!(started.state.turn, 1);
        assert_eq!(started.state.active, P1);
        assert_eq!(ids(&started.state.players.p1.hand[..p1_hand.len()]), p1_hand);
        assert_eq!(ids(&started.state.players.p2.hand[..hand.len()]), hand);
    }
}

/// R269 a draw offer's lifetime
mod r269_a_draw_offer_s_lifetime {
    use super::*;

    fn playing(seed: &str) -> GameState {
        let mut state = begin_game(&new_game(seed, None)).state;
        for player in SEATS {
            let keep: Vec<String> = ids(&state.players[player].hand);
            state = act(&state, input(json!({ "type": "mulligan", "keep": keep, "playerId": player })));
        }
        state
    }

    #[test]
    fn r269_stands_on_both_views_until_answered_or_until_its_offerer_s_turn_ends_and_a_lapsed_offer_blocks_nothing() {
        let mut state = playing("r269");
        for seat in SEATS {
            assert!(view_for(&state, seat).draw_offer.is_none());
        }

        state = act(&state, input(json!({ "type": "offerDraw", "playerId": "p1" })));
        for seat in SEATS {
            assert_eq!(view_for(&state, seat).draw_offer, Some(DrawOfferView { by: P1 }));
        }
        assert!(legal_actions(&state, P2).contains(&ActionBody::AnswerDraw { accept: true }));
        assert!(!legal_actions(&state, P1)
            .iter()
            .any(|action| action.action_type() == ActionType::OfferDraw));

        // p1 plays on and ends the turn without an answer: the offer lapses.
        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" })));
        for seat in SEATS {
            assert!(view_for(&state, seat).draw_offer.is_none());
        }
        let late = reduce(
            &state,
            &Action::new(ActionBody::AnswerDraw { accept: true }, P2, "r269-late"),
        );
        assert!(
            late.error.as_deref().is_some_and(|error| error.contains("no draw offer")),
            "{:?}",
            late.error
        );
        assert_eq!(state.players.p1.draw_offer.blocked_until, None);

        // A lapsed offer is not a declined one: p1 may offer again on its next turn (R36 blocks a decline).
        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p2" })));
        assert_eq!(state.active, P1);
        assert!(legal_actions(&state, P1).contains(&ActionBody::OfferDraw));

        // An answered offer is gone at once, from both views.
        state = act(&state, input(json!({ "type": "offerDraw", "playerId": "p1" })));
        state = act(&state, input(json!({ "type": "answerDraw", "accept": false, "playerId": "p2" })));
        for seat in SEATS {
            assert!(view_for(&state, seat).draw_offer.is_none());
        }
        assert!(!legal_actions(&state, P1)
            .iter()
            .any(|action| action.action_type() == ActionType::OfferDraw));
    }

    #[test]
    fn r269_r216_is_gone_once_the_game_is_over_however_it_ended() {
        let offered = act(&playing("r269-over"), input(json!({ "type": "offerDraw", "playerId": "p1" })));
        for (who, body) in [
            (P2, json!({ "type": "concede" })),
            (P1, json!({ "type": "concede" })),
            (P1, json!({ "type": "ceilingReached" })),
        ] {
            let mut literal = body;
            literal["playerId"] = json!(who);
            let over = act(&offered, input(literal));
            assert!(over.result.is_some());
            for seat in SEATS {
                assert!(view_for(&over, seat).draw_offer.is_none());
            }
        }
    }

    #[test]
    fn r269_does_not_survive_into_a_copy_of_the_state_as_anything_but_the_same_offer() {
        let state = act(&playing("r269-json"), input(json!({ "type": "offerDraw", "playerId": "p1" })));
        let copy = clone_state(&state);
        assert_eq!(view_for(&copy, P2).draw_offer, Some(DrawOfferView { by: P1 }));
        let accepted = act(&copy, input(json!({ "type": "answerDraw", "accept": true, "playerId": "p2" })));
        assert_eq!(
            accepted.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::DrawAccepted
            })
        );
    }
}
