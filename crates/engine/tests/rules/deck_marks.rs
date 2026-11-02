//! R2702000: a deck's marks, cards of a seat's list dealt on their Radiant face or Buffed at setup, as
//! a dungeon run's deck arrives with its upgraded cards (issue #2702, part A of #2701). Setup, not an
//! action: a fold, a summary and a Glitch reset deal the same marks, and a game made without any
//! stores nothing and hashes as before.
//!
//! Fixtures: the vanilla decks `fx-1`… and `fx-21`…, Units whose every Buff row is open. `create_game`
//! panics on a bad mark, caught by `refusal`; `validate_setup` answers `Result`.

use jackioh_engine::effects::{TuneDirection, tune_once};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::subsystems::glitch::reset_match;
use jackioh_engine::testkit::*;
use jackioh_engine::tuning::is_tuned;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{json_of, setup_catalog, sink_for};

type Marks = PerPlayerOpt<Vec<DeckMark>>;

fn decks() -> (Vec<String>, Vec<String>) {
    (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))
}

fn options(seed: &str, marks: Option<Marks>) -> CreateGameOptions {
    setup_catalog();
    CreateGameOptions {
        seed: seed.to_string(),
        decks: decks(),
        deck_marks: marks,
        ..Default::default()
    }
}

fn mark(index: usize, radiant: bool, upgrades: i32) -> DeckMark {
    DeckMark {
        index,
        radiant,
        upgrades,
    }
}

fn on_p1(list: Vec<DeckMark>) -> Marks {
    PerPlayerOpt {
        p1: Some(list),
        p2: None,
    }
}

fn on_p2(list: Vec<DeckMark>) -> Marks {
    PerPlayerOpt {
        p1: None,
        p2: Some(list),
    }
}

/// Every card `player` holds in hand and library.
fn hand_and_library(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let side = &state.players[player];
    side.hand.iter().chain(side.library.iter()).collect()
}

/// What a panic said, or `None` when `run` returned.
fn refusal(run: impl FnOnce()) -> Option<String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(()) => None,
        Err(payload) => Some(
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_string()))
                .unwrap_or_default(),
        ),
    }
}

/// Why setup refuses `opts`, from `validate_setup` and from `create_game`'s panic, which must agree.
fn refused(opts: &CreateGameOptions) -> String {
    let checked = validate_setup(opts, registered_catalog(), &IndexMap::new(), SetupKind::New)
        .expect_err("validate_setup refuses the marks")
        .message;
    let panicked = refusal(|| {
        create_game(opts);
    })
    .expect("create_game refuses the marks");
    assert_eq!(checked, panicked);
    checked
}

/// A full random-policy game (§10.7) from `created`, as a fold must reproduce it.
fn play_from(seed: &str, created: &GameState) -> (GameState, Vec<Action>) {
    let mut state = begin_game(created).state;
    let mut policy = Rng::new(&format!("deck-marks-policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();
    while state.result.is_none() {
        assert!(log.len() <= 5000, "{seed} did not finish");
        let player = seat_to_act(&state).expect("a seat to act");
        let Some(chosen) = subsystems::ai_policy::choose_action(&state, player, &mut policy) else {
            panic!("{seed}: no legal action for {player}");
        };
        let action = Action::new(chosen, player, format!("dm{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{seed}: {} refused: {error}", action.action_type());
        }
        log.push(action);
        state = result.state;
    }
    (state, log)
}

#[test]
fn r2702000_b1_a_radiant_mark_deals_that_card_radiant_in_the_library() {
    // B1: the marked card is in the library on its Radiant face from setup on, and its owner's list
    // records that face (R311). A Radiant mark draws nothing from the rng.
    let marks = on_p1(vec![mark(3, true, 0)]);
    let state = create_game(&options("dm-b1", Some(marks.clone())));
    let card = &state.players.p1.library[3];
    assert!(card.radiant);
    assert_eq!(card.def_id, decks().0[3]);
    assert_eq!(
        card.known_as,
        Some(KnownAs {
            def_id: decks().0[3].clone(),
            radiant: true
        })
    );
    let radiant: Vec<&CardInstance> = PLAYER_IDS
        .into_iter()
        .flat_map(|player| state.players[player].library.iter())
        .filter(|card| card.radiant)
        .collect();
    assert_eq!(radiant.len(), 1, "only the marked card is Radiant");
    assert_eq!(state.rng_cursor, 0);
    assert_eq!(
        state.opening.as_ref().and_then(|o| o.deck_marks.clone()),
        Some(marks)
    );
}

#[test]
fn r2702000_b1_marked_cards_stay_radiant_once_drawn_into_the_hand() {
    // B1: every card of p1's list marked Radiant, so whatever the shuffle deals is a marked card: the
    // opening hand holds them on their Radiant face, and none of p2's unmarked cards is Radiant.
    let marks = on_p1(
        (0..DECK_SIZE as usize)
            .map(|index| mark(index, true, 0))
            .collect(),
    );
    let state = begin_game(&create_game(&options("dm-b1-hand", Some(marks)))).state;
    assert!(!state.players.p1.hand.is_empty(), "the opening hand is dealt");
    assert!(
        hand_and_library(&state, PlayerId::P1)
            .iter()
            .all(|card| card.radiant)
    );
    assert!(state.players.p1.hand.iter().all(|card| card.radiant));
    assert!(
        !hand_and_library(&state, PlayerId::P2)
            .iter()
            .any(|card| card.radiant)
    );
}

#[test]
fn r2702000_b2_upgrades_are_that_many_buffs_drawn_from_the_games_rng() {
    // B2: `upgrades: 3` is three Buffs of the marked card at setup, each drawn from the game's rng as a
    // Buff in play draws it: the same card three `tune_once` Upgrades of the unmarked deal make.
    let opts = options("dm-b2", Some(on_p1(vec![mark(0, false, 3)])));
    let marked = create_game(&opts);

    let mut expected = create_game(&options("dm-b2", None));
    let card = expected.players.p1.library[0].clone();
    let mut events = Vec::new();
    let mut rng = Rng::new(&expected.seed, expected.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut expected, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, HookOptions::default());
        for _ in 0..3 {
            tune_once(&mut ctx, &card, TuneDirection::Upgrade, true);
        }
    }
    let cursor = rng.cursor();

    let buffed = &marked.players.p1.library[0];
    assert_eq!(buffed, &expected.players.p1.library[0]);
    assert!(cursor > 0, "the Buffs drew from the rng");
    assert_eq!(marked.rng_cursor, cursor);
    assert!(
        buffed.cost_mod < 0 || is_tuned(buffed),
        "the Buffs changed the card"
    );
    assert!(!buffed.radiant, "a mark without `radiant` leaves the base face");
    for player in PLAYER_IDS {
        for (at, card) in marked.players[player].library.iter().enumerate() {
            if player == PlayerId::P1 && at == 0 {
                continue;
            }
            assert_eq!(card.cost_mod, 0, "{player} {at} is unmarked");
            assert!(!is_tuned(card), "{player} {at} is unmarked");
        }
    }
    assert_eq!(create_game(&opts), marked, "the same setup deals the same cards");
}

#[test]
fn r2702000_b2_a_marked_game_folds_and_summarizes_to_the_game_played() {
    // B2: a marked game played to its end folds from the same setup and log to the same hash; folded
    // without its marks it is another game, and its summary is the game played.
    let seed = "dm-b2-fold";
    let marks = PerPlayerOpt {
        p1: Some(vec![mark(2, true, 2)]),
        p2: Some(vec![mark(5, false, 4)]),
    };
    let created = create_game(&options(seed, Some(marks.clone())));
    let (live, log) = play_from(seed, &created);

    let args = FoldArgs {
        seed: seed.to_string(),
        decks: decks(),
        log,
        deck_marks: Some(marks),
        ..Default::default()
    };
    let folded = fold(&args);
    assert!(folded.errors.is_empty(), "{:?}", folded.errors);
    assert_eq!(hash_state(&folded.state), hash_state(&live));

    let unmarked = fold(&FoldArgs {
        deck_marks: None,
        ..args.clone()
    });
    assert_ne!(hash_state(&unmarked.state), hash_state(&live));

    let summary = summarize_game(&args).expect("a finished game summarizes");
    assert_eq!(summary.turns, live.turn);
    assert_eq!(
        Some(summary.winner),
        live.result.as_ref().map(|result| result.winner)
    );
}

#[test]
fn r2702000_a_glitch_reset_deals_the_marked_cards_again() {
    // R676: the opening record keeps the marks, so a Glitch reset deals the marked card again, on its
    // Radiant face and with the same Buffs, drawn again from the fresh deal's start.
    let marks = on_p1(vec![mark(4, true, 3)]);
    let created = create_game(&options("dm-glitch", Some(marks.clone())));
    let mut state = begin_game(&created).state;
    {
        let mut sink = sink_for(&mut state);
        reset_match(&mut sink);
    }
    assert_eq!(state.resets, Some(1));
    let radiant: Vec<&CardInstance> = hand_and_library(&state, PlayerId::P1)
        .into_iter()
        .filter(|card| card.radiant)
        .collect();
    assert_eq!(radiant.len(), 1, "the marked card, and only it, is Radiant again");
    let dealt = &created.players.p1.library[4];
    assert_eq!(radiant[0].def_id, decks().0[4]);
    assert_eq!(radiant[0].tuning, dealt.tuning);
    assert_eq!(radiant[0].cost_mod, dealt.cost_mod);
    assert!(
        dealt.cost_mod < 0 || is_tuned(dealt),
        "the Buffs changed the card"
    );
    assert_eq!(
        state.opening.as_ref().and_then(|o| o.deck_marks.clone()),
        Some(marks)
    );
}

#[test]
fn r2702000_b3_no_marks_store_nothing_and_hash_as_before() {
    // B3: no marks, an empty record of them and a seat with an empty list all deal the game made
    // without the field: nothing stored, nothing drawn, the same hash before and after the deal.
    let games: Vec<GameState> = [None, Some(PerPlayerOpt::default()), Some(on_p1(vec![]))]
        .into_iter()
        .map(|marks| create_game(&options("dm-b3", marks)))
        .collect();
    for state in &games {
        assert_eq!(state.rng_cursor, 0);
        assert!(state.opening.as_ref().is_some_and(|o| o.deck_marks.is_none()));
        assert!(!json_of(state).to_string().contains("deckMarks"));
        assert_eq!(hash_state(state), hash_state(&games[0]));
        assert_eq!(
            hash_state(&begin_game(state).state),
            hash_state(&begin_game(&games[0]).state)
        );
    }
}

#[test]
fn r2702000_b4_a_mark_past_its_seats_deck_refuses_create_game_with_a_reason() {
    // B4: index DECK_SIZE is one past the last card of p2's list, so setup refuses it and says why.
    let opts = options("dm-b4", Some(on_p2(vec![mark(DECK_SIZE as usize, false, 1)])));
    let reason = refused(&opts);
    assert!(
        reason.contains(&format!(
            "p2: deck mark index {DECK_SIZE} is past its deck of {DECK_SIZE} cards"
        )),
        "{reason}"
    );
}

#[test]
fn r2702000_two_marks_on_one_index_and_negative_upgrades_refuse_create_game() {
    let twice = options("dm-twice", Some(on_p1(vec![mark(1, true, 0), mark(1, false, 2)])));
    let reason = refused(&twice);
    assert!(reason.contains("p1: two deck marks name index 1"), "{reason}");

    let negative = options("dm-negative", Some(on_p1(vec![mark(0, false, -1)])));
    let reason = refused(&negative);
    assert!(
        reason.contains("p1: deck mark upgrades must be a non-negative integer (R2702000), got -1"),
        "{reason}"
    );
}
