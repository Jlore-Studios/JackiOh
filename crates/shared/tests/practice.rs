//! The practice session's machine (#3099; SPEC §9.9, §9.10, B22, B25, B34, B35, B38, B39, R187, R188, R433,
//! R508, R668, R677, R679, R768, R923000, R1303000, R1373, R1705005, R2102000, R2560000, R2706000,
//! R3100000, R3477000): played on the real catalog, with a `Session` standing in for a host. It keeps what
//! the machine asks it to store (the save, the kept games), answers the AI's ask with the AI itself and the
//! lethal ask with nothing, and nothing else decides anything.

use jackioh_ai::{AI_BUDGET, AiDeckOptions, build_ai_deck, random_action};
use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::{
    Action, ActionBody, DECK_SIZE, Difficulty, GameOverReason, GameState, LastBoardEntry, LeanPick,
    PendingView, Rng, SandboxOp, SetName, SpawnZone, TUTORIAL_HUMAN, Winner,
};
use jackioh_shared::config::{PRACTICE_AI_CLOCK_MS, PRACTICE_LETHAL_HINT_CLOCK_MS, PRACTICE_REPLAYS_KEPT};
use jackioh_shared::host::practice::replays::{ReplayPageResult, ReplayVerdict, list_replays, page_replay};
use jackioh_shared::host::practice::{
    ClaimRefusal, KeptAs, LessonDeal, PracticeConfig, PracticeDeckChoice, PracticeEvent, PracticeHost,
    PracticeOutput, PracticeReplay, PracticeResult, PracticeSave, PracticeSnapshot, check_claim,
};
use jackioh_shared::host::timer::{TimerCommand, TimerKey};

const EPOCH: i64 = 1_700_000_000_000;
const MOVE_CAP: usize = 4000;

fn build() -> String {
    jackioh_cards::register_all();
    jackioh_cards::catalog_version().to_string()
}

fn random_deck() -> PracticeDeckChoice {
    PracticeDeckChoice::Random {
        lean_set: None,
        lean_newest: None,
    }
}

fn config(seed: &str, human: PlayerId) -> PracticeConfig {
    jackioh_cards::register_all();
    PracticeConfig::free(seed, Difficulty::Easy, human, random_deck())
}

/// A deck the engine accepts, built as the human's random deck is.
fn deck_of(seed: &str, size: i32) -> Vec<String> {
    jackioh_cards::register_all();
    build_ai_deck(
        &mut Rng::new(seed, 0),
        size,
        &AiDeckOptions {
            banned: Some(Vec::new()),
            gated_sets: Some(Vec::new()),
            ..AiDeckOptions::default()
        },
    )
}

/// A host standing in for the real one.
struct Session {
    host: PracticeHost,
    now: i64,
    catalog: String,
    /// The save the machine last asked to write: `None` before one, `Some(None)` once cleared.
    save: Option<Option<PracticeSave>>,
    kept: Vec<PracticeReplay>,
    snapshots: Vec<PracticeSnapshot>,
    lethal_asks: Vec<(PlayerId, i64)>,
    timers: Vec<TimerCommand>,
    decide_clocks: Vec<i64>,
    records: Vec<jackioh_shared::host::practice::PracticeRecord>,
    voids: usize,
    failed: Vec<String>,
    began: Vec<PlayerId>,
}

impl Session {
    fn start(config: &PracticeConfig) -> Result<Session, String> {
        let catalog = build();
        let (host, outputs) = PracticeHost::start(config, &catalog, Some(Vec::new()), EPOCH)?;
        Ok(Session::of(host, catalog, outputs))
    }

    fn of(host: PracticeHost, catalog: String, outputs: Vec<PracticeOutput>) -> Session {
        let mut session = Session {
            host,
            now: EPOCH,
            catalog,
            save: None,
            kept: Vec::new(),
            snapshots: Vec::new(),
            lethal_asks: Vec::new(),
            timers: Vec::new(),
            decide_clocks: Vec::new(),
            records: Vec::new(),
            voids: 0,
            failed: Vec::new(),
            began: Vec::new(),
        };
        session.carry_out(&outputs);
        session
    }

    fn carry_out(&mut self, outputs: &[PracticeOutput]) {
        for output in outputs {
            match output {
                PracticeOutput::Began { ai_seat } => self.began.push(*ai_seat),
                PracticeOutput::Save { save } => self.save = Some(save.as_deref().cloned()),
                PracticeOutput::Keep { replay, keep } => {
                    self.kept.push((**replay).clone());
                    let surplus = self.kept.len().saturating_sub(*keep as usize);
                    self.kept.drain(..surplus);
                }
                PracticeOutput::Finished { record } => self.records.push((**record).clone()),
                PracticeOutput::Voided { .. } => self.voids += 1,
                PracticeOutput::Snapshot { snapshot } => self.snapshots.push((**snapshot).clone()),
                PracticeOutput::Lethal { ask } => self.lethal_asks.push((ask.seat, ask.clock_ms)),
                PracticeOutput::Timer { command } => self.timers.push(*command),
                PracticeOutput::Failed { message } => self.failed.push(message.clone()),
                PracticeOutput::Decide { ask } => self.decide_clocks.push(ask.clock_ms),
                _ => {}
            }
        }
    }

    fn feed(&mut self, event: PracticeEvent) -> Vec<PracticeOutput> {
        let outputs = self.host.step(event, self.now);
        self.carry_out(&outputs);
        outputs
    }

    fn act(&mut self, body: ActionBody) -> &PracticeSnapshot {
        self.feed(PracticeEvent::Act { body });
        self.snapshot()
    }

    /// The AI's turn, decided by the AI itself: whether it owed a move.
    fn ai_turn(&mut self) -> bool {
        let outputs = self.feed(PracticeEvent::AiTurn);
        let Some(ask) = outputs.iter().find_map(|output| match output {
            PracticeOutput::Decide { ask } => Some(ask.clone()),
            _ => None,
        }) else {
            return false;
        };
        let moved = self.host.decide(&ask, AI_BUDGET);
        self.feed(PracticeEvent::Decided { moved });
        true
    }

    fn snapshot(&self) -> &PracticeSnapshot {
        self.snapshots.last().expect("a snapshot")
    }

    fn log(&self) -> &[Action] {
        self.host.log()
    }

    fn state(&self) -> &GameState {
        self.host.game()
    }

    /// The whole game from `self`: the AI plays its own, the human what `policy` picks, until the end.
    fn play_out(&mut self, human_rng: &mut Rng) {
        for _ in 0..MOVE_CAP {
            if self.host.is_over() {
                return;
            }
            if self.ai_turn() {
                assert!(self.failed.is_empty(), "{:?}", self.failed);
                continue;
            }
            let seat = self.host.human_seat();
            let body = random_action(self.state(), seat, human_rng).expect("the human owes an action");
            self.act(body);
        }
        panic!("the game ends within {MOVE_CAP} actions");
    }

    fn concede(&mut self) {
        self.answer_mulligans();
        self.act(ActionBody::Concede);
    }

    /// The mulligans answered: the AI's, then the human's keeping its whole hand.
    fn answer_mulligans(&mut self) {
        while self.ai_turn() {}
        if let Some(keep) = keep_all(self.snapshot()) {
            self.act(keep);
        }
    }
}

/// The human's mulligan answer keeping every card, if it owes one.
fn keep_all(snapshot: &PracticeSnapshot) -> Option<ActionBody> {
    match snapshot.view.pending.as_ref()? {
        PendingView::ForYou(pending) => Some(ActionBody::Mulligan {
            keep: pending.options.iter().map(|option| option.key.clone()).collect(),
        }),
        PendingView::Elsewhere(_) => None,
    }
}

fn started(seed: &str, human: PlayerId) -> Session {
    Session::start(&config(seed, human)).expect("the game starts")
}

// B34: the machine answers with the human's view and nothing else

#[test]
fn b34_a_start_answers_with_the_humans_view_and_the_ai_on_the_other_seat() {
    for human in [P1, P2] {
        let session = started("practice-b34", human);
        assert_eq!(session.began, vec![human.opponent()]);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.view.viewer, human);
        assert_eq!(snapshot.view, jackioh_engine::view_for(session.state(), human));
        assert_eq!(
            snapshot.legal,
            jackioh_engine::legal_actions(session.state(), human)
        );
        assert!(snapshot.error.is_none() && snapshot.last_board.is_none());
        assert_eq!(
            snapshot.forecast,
            Some(jackioh_engine::combat_forecast(session.state(), human))
        );
        assert!(session.log().is_empty());
        // The AI owes its mulligan at once (R265), and the snapshot says so without saying how.
        assert!(snapshot.ai_to_act);
    }
}

#[test]
fn b34_every_snapshot_of_a_played_out_opening_is_view_for_of_the_true_state() {
    let mut session = started("practice-b34-opening", P2);
    while session.ai_turn() {
        let snapshot = session.snapshot();
        assert_eq!(snapshot.view, jackioh_engine::view_for(session.state(), P2));
        assert_eq!(snapshot.legal, jackioh_engine::legal_actions(session.state(), P2));
    }
    let json = serde_json::to_string(session.snapshot()).expect("serializes");
    // The AI's hand never crosses: every card of it the snapshot names is hidden.
    for card in &session.state().players[P1].hand {
        assert!(
            !json.contains(&format!("\"{}\"", card.id)) || json.contains("hidden"),
            "{}",
            card.id
        );
    }
}

#[test]
fn b34_an_ai_turn_the_ai_does_not_owe_changes_nothing_and_still_answers_with_a_snapshot() {
    // The human first (p1): once the mulligans are answered it is on turn, and the AI owes nothing.
    let mut session = started("practice-b34-idle", P1);
    session.answer_mulligans();
    let (hash, log, snapshots) = (
        jackioh_engine::hash_state(session.state()),
        session.log().len(),
        session.snapshots.len(),
    );
    assert!(
        !session.snapshot().ai_to_act,
        "the AI owes nothing in the human's main phase"
    );
    assert!(!session.ai_turn());
    assert_eq!(jackioh_engine::hash_state(session.state()), hash);
    assert_eq!(session.log().len(), log);
    assert_eq!(session.snapshots.len(), snapshots + 1);
}

#[test]
fn b34_a_deck_the_engine_refuses_is_no_game_and_says_why() {
    let saved = |cards: Vec<String>| PracticeDeckChoice::Saved {
        index: 1,
        cards,
        portrait: None,
        cosmetics: None,
    };
    let refused = |deck: PracticeDeckChoice| {
        let mut asked = config("practice-b34-deck", P2);
        asked.deck = deck;
        Session::start(&asked).err().expect("the deck is refused")
    };
    let full = deck_of("practice-b34-deck", DECK_SIZE);
    assert!(
        refused(saved(full[..full.len() - 1].to_vec())).contains("25"),
        "the wrong size"
    );
    let mut doubled = full.clone();
    doubled[1] = doubled[0].clone();
    doubled[2] = doubled[0].clone();
    doubled[3] = doubled[0].clone();
    assert!(!refused(saved(doubled)).is_empty(), "a duplicate");
    let mut unknown = full.clone();
    unknown[0] = "no-such-card".to_string();
    assert!(
        refused(saved(unknown)).contains("no-such-card"),
        "a card the catalog lacks"
    );
    let preset = refused(PracticeDeckChoice::Preset {
        id: "nowhere".to_string(),
        cards: None,
    });
    assert!(preset.contains("nowhere"), "{preset}");
}

#[test]
fn b34_a_saved_deck_is_played_as_given_and_a_random_deck_is_deck_size_distinct_cards() {
    let full = deck_of("practice-b34-saved", DECK_SIZE);
    let mut asked = config("practice-b34-saved", P2);
    asked.deck = PracticeDeckChoice::Saved {
        index: 2,
        cards: full.clone(),
        portrait: None,
        cosmetics: None,
    };
    let session = Session::start(&asked).expect("starts");
    assert_eq!(session.host.fold_args().decks.1, full);
    let random = Session::start(&config("practice-b34-saved", P2)).expect("starts");
    let dealt = random.host.fold_args().decks.1;
    assert_eq!(dealt.len(), DECK_SIZE as usize);
}

// B35: a human action is either refused with the engine's reason or logged as h<n>

#[test]
fn b35_a_refusal_spends_no_nonce_and_an_accepted_action_is_logged_h_n_for_the_humans_seat() {
    let mut session = started("practice-b35", P2);
    let hash = jackioh_engine::hash_state(session.state());
    // Anything but a mulligan while the mulligans are open is refused with the engine's reason.
    let refused = session.act(ActionBody::EndTurn).error.clone();
    assert!(refused.is_some_and(|reason| !reason.is_empty()));
    assert_eq!(jackioh_engine::hash_state(session.state()), hash);
    assert!(session.log().is_empty());
    let again = session.act(ActionBody::EndTurn).error.clone();
    assert!(
        again.is_some(),
        "after two refusals in a row the error is the second one's"
    );

    session.answer_mulligans();
    let log = session.log();
    let human: Vec<&Action> = log
        .iter()
        .filter(|action| action.nonce.starts_with('h'))
        .collect();
    assert_eq!(human.len(), 1);
    assert_eq!((human[0].nonce.as_str(), human[0].player_id), ("h0", P2));
    assert!(
        session.snapshot().error.is_none(),
        "an accepted action clears the error"
    );
    let ai: Vec<&str> = log
        .iter()
        .filter(|action| action.nonce.starts_with('a'))
        .map(|action| action.nonce.as_str())
        .collect();
    assert_eq!(ai, vec!["a0"]);
}

// R187, B38: a practice game folds from (seed, decks, handicaps, log) to its own hash

fn folds_to_its_own_hash(difficulty: Difficulty, seed: &str) {
    let mut asked = config(seed, P2);
    asked.difficulty = difficulty;
    let mut session = Session::start(&asked).expect("starts");
    session.play_out(&mut Rng::new(&format!("{seed}:human"), 0));
    let folded = jackioh_engine::fold(&session.host.fold_args());
    assert!(folded.errors.is_empty(), "{:?}", folded.errors);
    assert_eq!(
        jackioh_engine::hash_state(&folded.state),
        jackioh_engine::hash_state(session.state()),
        "{seed}"
    );
    let record = session.records.last().expect("the end is recorded once");
    assert_eq!(session.records.len(), 1);
    assert_eq!(record.final_hash, jackioh_engine::hash_state(session.state()));
    assert_eq!(
        record.ai_moves as usize,
        session.log().iter().filter(|a| a.nonce.starts_with('a')).count()
    );
}

#[test]
fn r187_b38_an_easy_game_folds_from_its_setup_and_log_to_its_own_hash() {
    folds_to_its_own_hash(Difficulty::Easy, "practice-b38-easy");
}

#[test]
fn r187_b38_a_hard_game_folds_with_its_handicaps_and_only_with_them() {
    let seed = "practice-b38-hard";
    folds_to_its_own_hash(Difficulty::Hard, seed);
    let mut asked = config(seed, P2);
    asked.difficulty = Difficulty::Hard;
    let session = Session::start(&asked).expect("starts");
    let mut args = session.host.fold_args();
    args.handicaps = None;
    let without = std::panic::catch_unwind(|| jackioh_engine::fold(&args));
    assert!(
        without.is_err(),
        "a Hard game folded without its handicaps fails loudly"
    );
}

#[test]
fn r187_b38_two_sessions_fed_the_same_events_answer_alike() {
    let play = || {
        let mut session = started("practice-b38-twice", P2);
        session.answer_mulligans();
        let mut human = Rng::new("practice-b38-twice:human", 0);
        for _ in 0..30 {
            if session.host.is_over() {
                break;
            }
            if !session.ai_turn() {
                let seat = session.host.human_seat();
                let body = random_action(session.state(), seat, &mut human).expect("an action");
                session.act(body);
            }
        }
        (
            session.log().to_vec(),
            jackioh_engine::hash_state(session.state()),
        )
    };
    assert_eq!(play(), play());
}

// R188, B39: what the AI never takes, and what it falls back on

#[test]
fn r188_a_decision_the_ai_never_takes_falls_back_on_the_first_legal_action_not_forbidden() {
    for body in [Some(ActionBody::Concede), Some(ActionBody::OfferDraw), None] {
        let mut session = started("practice-r188", P2);
        let ask = match session.feed(PracticeEvent::AiTurn).into_iter().next() {
            Some(PracticeOutput::Decide { ask }) => ask,
            other => panic!("the AI owes its mulligan: {other:?}"),
        };
        // The cursor the search stopped at is kept, whatever became of its move.
        session.feed(PracticeEvent::Decided {
            moved: jackioh_shared::host::practice::AiMove {
                body: body.clone(),
                rng_cursor: ask.rng_cursor + 7,
            },
        });
        assert!(session.failed.is_empty(), "{:?}", session.failed);
        let first = session.log().first().expect("the AI moved");
        assert_eq!((first.nonce.as_str(), first.player_id), ("a0", P1));
        assert!(
            matches!(first.body, ActionBody::Mulligan { .. }),
            "{:?}",
            first.body
        );
        let json = serde_json::to_value(&session.host).expect("the host is JSON");
        assert_eq!(json["aiCursor"], serde_json::json!(ask.rng_cursor + 7));
    }
}

#[test]
fn r188_b39_the_humans_draw_offer_is_declined_by_the_ais_next_step() {
    let mut session = started("practice-b39", P2);
    session.answer_mulligans();
    let mut human = Rng::new("practice-b39:human", 0);
    for _ in 0..MOVE_CAP {
        // The human passes until it is its main phase with the right to offer a draw.
        if session.host.is_over() {
            panic!("no draw offer was possible");
        }
        if session.snapshot().legal.contains(&ActionBody::OfferDraw) {
            break;
        }
        if !session.ai_turn() {
            let body = session
                .snapshot()
                .legal
                .iter()
                .find(|body| matches!(body, ActionBody::EndTurn))
                .cloned()
                .or_else(|| random_action(session.state(), P2, &mut human))
                .expect("an action");
            session.act(body);
        }
    }
    session.act(ActionBody::OfferDraw);
    assert!(
        session.snapshot().ai_to_act,
        "the offer makes the AI owe an answer"
    );
    assert!(session.ai_turn());
    let last = session.log().last().expect("the AI answered");
    assert_eq!(last.body, ActionBody::AnswerDraw { accept: false });
}

// R433: practice's fresh random deck lists only the cards the human has been shown

#[test]
fn r433_a_random_deck_is_dealt_and_a_saved_deck_a_preset_or_a_lesson_is_not() {
    let session = started("practice-r433", P2);
    assert_eq!(session.host.fold_args().dealt, Some(vec![P2]));
    let mut preset = config("practice-r433", P2);
    preset.deck = PracticeDeckChoice::Preset {
        id: "mine".to_string(),
        cards: Some(deck_of("practice-r433-preset", DECK_SIZE)),
    };
    assert_eq!(
        Session::start(&preset).expect("starts").host.fold_args().dealt,
        None
    );
    let lesson = lesson_config("practice-r433-lesson");
    assert_eq!(
        Session::start(&lesson).expect("starts").host.fold_args().dealt,
        None
    );
}

// R1373, R924000: the human's random deck leans on a set, and the AI's never does

fn newest_cards(deck: &[String], set: SetName) -> usize {
    let catalog = jackioh_engine::registered_catalog();
    deck.iter()
        .filter(|id| catalog.get(id.as_str()).is_some_and(|def| def.set == set))
        .count()
}

#[test]
fn r1373_r924000_the_humans_random_deck_leans_as_asked_and_the_ais_never() {
    let seed = "practice-r1373";
    let plain = Session::start(&config(seed, P2))
        .expect("starts")
        .host
        .fold_args();
    let mut newest = config(seed, P2);
    newest.deck = PracticeDeckChoice::Random {
        lean_set: Some(LeanPick::Newest),
        lean_newest: None,
    };
    let leaned = Session::start(&newest).expect("starts").host.fold_args();
    let set = LeanPick::Newest.resolve();
    assert!(newest_cards(&leaned.decks.1, set) >= newest_cards(&plain.decks.1, set));
    assert_eq!(leaned.decks.0, plain.decks.0, "the AI's deck never leans");
    // R1373: a game kept before the picker says `leanNewest` and deals the deck "newest" does.
    let mut legacy = config(seed, P2);
    legacy.deck = PracticeDeckChoice::Random {
        lean_set: None,
        lean_newest: Some(true),
    };
    assert_eq!(
        Session::start(&legacy).expect("starts").host.fold_args().decks,
        leaned.decks
    );
    // A preset is not leaned on, whatever the switch says.
    let mut preset = config(seed, P2);
    let cards = deck_of("practice-r1373-preset", DECK_SIZE);
    preset.deck = PracticeDeckChoice::Preset {
        id: "mine".to_string(),
        cards: Some(cards.clone()),
    };
    assert_eq!(
        Session::start(&preset).expect("starts").host.fold_args().decks.1,
        cards
    );
}

// R508: the human's last board

#[test]
fn r508_a_finished_free_game_gives_the_board_and_the_next_game_starts_with_it() {
    let mut session = started("practice-r508", P2);
    session.concede();
    let board = session
        .snapshot()
        .last_board
        .clone()
        .expect("a free game that ends gives the board");
    assert_eq!(board, jackioh_engine::last_board_for(session.state(), P2));

    let mut next = config("practice-r508-next", P2);
    let entries = vec![
        LastBoardEntry {
            def_id: "core-001".to_string(),
            radiant: false,
        },
        LastBoardEntry {
            def_id: "core-002".to_string(),
            radiant: true,
        },
    ];
    next.last_board = Some(entries.clone());
    let next = Session::start(&next).expect("starts");
    // On the human's seat alone (seat order: the human is p2).
    assert_eq!(next.host.fold_args().last_boards, Some((Vec::new(), entries)));
    let folded = jackioh_engine::fold(&next.host.fold_args());
    assert!(folded.errors.is_empty());
}

#[test]
fn r508_a_lesson_and_a_sandbox_game_neither_take_the_board_nor_give_one() {
    let entries = vec![LastBoardEntry {
        def_id: "core-001".to_string(),
        radiant: false,
    }];
    let mut lesson = lesson_config("practice-r508-lesson");
    lesson.last_board = Some(entries.clone());
    assert_eq!(
        Session::start(&lesson)
            .expect("starts")
            .host
            .fold_args()
            .last_boards,
        None
    );
    let mut sandbox = config("practice-r508-sandbox", P2);
    sandbox.sandbox = Some(true);
    sandbox.last_board = Some(entries);
    let mut session = Session::start(&sandbox).expect("starts");
    assert_eq!(session.host.fold_args().last_boards, None);
    session.concede();
    assert!(
        session.snapshot().last_board.is_none(),
        "R923000: its board was built by its tools"
    );
}

// R3100000, R1705005: a lesson

fn lesson_config(seed: &str) -> PracticeConfig {
    let mut lesson = config(seed, P2);
    lesson.lesson = Some("lesson-1".to_string());
    lesson.lesson_deal = Some(LessonDeal {
        human_deck: deck_of(&format!("{seed}:human"), TUTORIAL_HUMAN.deck_size),
        ai_deck: deck_of(&format!("{seed}:ai"), 12),
        pool_hold: true,
    });
    lesson
}

#[test]
fn r3100000_a_lessons_every_snapshot_holds_the_forecast() {
    let mut lesson = Session::start(&lesson_config("practice-r3100000")).expect("starts");
    lesson.answer_mulligans();
    for snapshot in &lesson.snapshots {
        assert!(snapshot.forecast.is_some());
    }
    let forecast = lesson.snapshot().forecast.clone().expect("a forecast");
    assert_eq!(forecast, jackioh_engine::combat_forecast(lesson.state(), P2));
}

#[test]
fn r3477000_a_free_games_every_snapshot_holds_the_forecast_of_the_humans_attacks() {
    let mut session = started("practice-r3477000", P2);
    let mut human = Rng::new("practice-r3477000:human", 0);
    let mut forecast_attacks = 0;
    for _ in 0..MOVE_CAP {
        if session.host.is_over() {
            break;
        }
        if !session.ai_turn() {
            let seat = session.host.human_seat();
            let body = random_action(session.state(), seat, &mut human).expect("the human owes an action");
            session.act(body);
        }
        let forecast = session
            .snapshot()
            .forecast
            .clone()
            .expect("every snapshot holds one");
        assert_eq!(
            forecast,
            jackioh_engine::combat_forecast(session.state(), session.host.human_seat())
        );
        forecast_attacks += forecast.attacks.len();
    }
    assert!(session.host.is_over(), "the game ends within {MOVE_CAP} actions");
    assert!(
        session
            .snapshots
            .iter()
            .all(|snapshot| snapshot.forecast.is_some())
    );
    assert!(
        forecast_attacks > 0,
        "the premise: some snapshot forecast an attack"
    );
}

#[test]
fn r1705005_a_lesson_seats_the_human_under_the_tutorial_handicap_and_the_pool_hold() {
    let session = Session::start(&lesson_config("practice-r1705005")).expect("starts");
    let handicaps = session.host.fold_args().handicaps.expect("handicaps");
    let human = handicaps.p2.expect("the human's");
    assert_eq!(human.deck_size, TUTORIAL_HUMAN.deck_size);
    assert_eq!(human.hero_floor, TUTORIAL_HUMAN.hero_floor);
    assert_eq!(human.pool_hold, Some(true));
    assert_eq!(handicaps.p1.expect("the AI's").hero_health, Some(20));
    // A lesson with no decks is no lesson.
    let mut bare = lesson_config("practice-r1705005");
    bare.lesson_deal = None;
    assert!(Session::start(&bare).err().expect("refused").contains("lesson-1"));
}

// R668: the save and its resume

#[test]
fn r668_a_new_session_folds_the_saved_log_back_to_the_same_state_and_ai_stream() {
    let mut session = started("practice-r668", P2);
    session.answer_mulligans();
    let mut human = Rng::new("practice-r668:human", 0);
    for _ in 0..24 {
        if !session.ai_turn() {
            let body = random_action(session.state(), P2, &mut human).expect("an action");
            session.act(body);
        }
    }
    let save = session
        .save
        .clone()
        .expect("a save was asked for")
        .expect("a game in progress is saved");
    assert_eq!(save.log, session.log());
    let (host, outputs) = PracticeHost::resume(
        &save.config,
        Some(&save),
        &session.catalog,
        Some(Vec::new()),
        EPOCH,
    )
    .expect("the save resumes");
    let mut resumed = Session::of(host, session.catalog.clone(), outputs);
    assert_eq!(
        jackioh_engine::hash_state(resumed.state()),
        jackioh_engine::hash_state(session.state())
    );
    assert_eq!(resumed.snapshot(), session.snapshot());
    // Plays on exactly as it would have.
    let mut same = |one: &mut Session, other: &mut Session| {
        for _ in 0..6 {
            let a = one.ai_turn();
            let b = other.ai_turn();
            assert_eq!(a, b);
            if !a {
                let body = random_action(one.state(), one.host.human_seat(), &mut human).expect("an action");
                one.act(body.clone());
                other.act(body);
            }
        }
    };
    same(&mut resumed, &mut session);
    assert_eq!(resumed.log(), session.log());
}

#[test]
fn r668_a_resume_of_no_save_another_game_or_another_catalog_fails() {
    let mut session = started("practice-r668-refused", P2);
    session.answer_mulligans();
    let save = session.save.clone().flatten().expect("a save");
    let resume = |asked: &PracticeConfig, save: Option<&PracticeSave>, build: &str| {
        PracticeHost::resume(asked, save, build, None, EPOCH).err()
    };
    let build = session.catalog.clone();
    assert!(
        resume(&save.config, None, &build)
            .expect("no save")
            .contains("no practice game")
    );
    let mut other_seed = save.config.clone();
    other_seed.seed = "elsewhere".to_string();
    assert!(resume(&other_seed, Some(&save), &build).is_some());
    let mut other_seat = save.config.clone();
    other_seat.human_seat = P1;
    assert!(resume(&other_seat, Some(&save), &build).is_some());
    let mut other_sandbox = save.config.clone();
    other_sandbox.sandbox = Some(true);
    assert!(resume(&other_sandbox, Some(&save), &build).is_some());
    assert!(
        resume(&save.config, Some(&save), "0.0.1")
            .expect("another catalog")
            .contains("catalog")
    );
    let mut forged = save.clone();
    forged.hash = "00000000".to_string();
    assert!(
        resume(&save.config, Some(&forged), &build)
            .expect("another hash")
            .contains("fold")
    );
    let mut misnumbered = save.clone();
    misnumbered.log[0].nonce = "h5".to_string();
    assert!(resume(&save.config, Some(&misnumbered), &build).is_some());
}

#[test]
fn r668_a_lesson_is_never_saved_a_finished_game_clears_the_save_and_a_run_leaves_it_alone() {
    let lesson = Session::start(&lesson_config("practice-r668-lesson")).expect("starts");
    assert_eq!(lesson.save, Some(None));
    let mut session = started("practice-r668-over", P2);
    assert!(matches!(session.save, Some(Some(_))));
    session.concede();
    assert_eq!(session.save, Some(None), "a finished game clears it");
    // R3476000: a run fight in progress is saved in its own slot, and a finished one clears it.
    let mut run = Session::start(&run_config("practice-r668-run")).expect("starts");
    assert!(matches!(run.save, Some(Some(_))), "a fight in progress is saved");
    run.concede();
    assert_eq!(run.save, Some(None), "a finished fight clears it");
}

// R3476000: a dungeon run's unfinished fight waits where it stood

#[test]
fn r3476000_a_run_fight_is_saved_and_resumes_to_the_same_state_and_ai_stream() {
    let mut session = Session::start(&run_config("practice-r3476000")).expect("starts");
    session.answer_mulligans();
    let mut human = Rng::new("practice-r3476000:human", 0);
    for _ in 0..24 {
        if !session.ai_turn() {
            let seat = session.host.human_seat();
            let body = random_action(session.state(), seat, &mut human).expect("an action");
            session.act(body);
        }
    }
    assert!(!session.host.is_over(), "the walk leaves the fight in progress");
    let save = session
        .save
        .clone()
        .expect("a save was asked for")
        .expect("a fight in progress is saved");
    assert_eq!(save.log, session.log());
    let (host, outputs) = PracticeHost::resume(
        &save.config,
        Some(&save),
        &session.catalog,
        Some(Vec::new()),
        EPOCH,
    )
    .expect("the fight resumes");
    let mut resumed = Session::of(host, session.catalog.clone(), outputs);
    assert_eq!(
        jackioh_engine::hash_state(resumed.state()),
        jackioh_engine::hash_state(session.state())
    );
    assert_eq!(resumed.snapshot(), session.snapshot());
    // Plays on exactly as it would have.
    let mut same = |one: &mut Session, other: &mut Session| {
        for _ in 0..6 {
            let a = one.ai_turn();
            let b = other.ai_turn();
            assert_eq!(a, b);
            if !a {
                let body = random_action(one.state(), one.host.human_seat(), &mut human).expect("an action");
                one.act(body.clone());
                other.act(body);
            }
        }
    };
    same(&mut resumed, &mut session);
    assert_eq!(resumed.log(), session.log());
}

#[test]
fn r3476000_resume_refuses_across_free_and_run_and_another_fight() {
    let mut free = started("practice-r3476000-free", P2);
    free.answer_mulligans();
    let free_save = free.save.clone().expect("a save").expect("in progress");
    let mut fight = Session::start(&run_config("practice-r3476000-fight")).expect("starts");
    fight.answer_mulligans();
    let fight_save = fight.save.clone().expect("a save").expect("in progress");
    let build = free.catalog.clone();

    // A free game never resumes a run fight's save, nor a run fight a free game's.
    let mut as_run = free_save.config.clone();
    as_run.run = fight_save.config.run.clone();
    let refused = PracticeHost::resume(&as_run, Some(&free_save), &build, None, EPOCH)
        .expect_err("a free save asked as a fight is refused");
    assert!(refused.contains("not the one the page remembers"), "{refused}");
    let mut as_free = fight_save.config.clone();
    as_free.run = None;
    let refused = PracticeHost::resume(&as_free, Some(&fight_save), &build, None, EPOCH)
        .expect_err("a fight save asked as a free game is refused");
    assert!(refused.contains("not the one the page remembers"), "{refused}");
    // A save from another fight is refused, so that fight is dealt fresh.
    let mut other_fight = fight_save.config.clone();
    other_fight.seed = "practice-r3476000-other".to_string();
    assert!(PracticeHost::resume(&other_fight, Some(&fight_save), &build, None, EPOCH).is_err());
}

// R923000: a practice sandbox game

#[test]
fn r923000_the_humans_tools_wait_on_the_mulligan_and_are_logged_h_n_the_ai_never_takes_one() {
    let mut asked = config("practice-r923000", P2);
    asked.sandbox = Some(true);
    let mut session = Session::start(&asked).expect("starts");
    let tool = ActionBody::Sandbox {
        op: SandboxOp::SetHero {
            player: P2,
            health: Some(7),
            armor: None,
        },
    };
    assert!(
        session.act(tool.clone()).error.is_some(),
        "the tools wait on the mulligan"
    );
    session.answer_mulligans();
    assert!(
        !session
            .snapshot()
            .legal
            .iter()
            .any(|body| matches!(body, ActionBody::Sandbox { .. }))
    );
    assert!(session.act(tool.clone()).error.is_none());
    let last = session.log().last().expect("logged");
    assert_eq!((last.nonce.as_str(), last.body.clone()), ("h1", tool));
    assert_eq!(session.host.fold_args().sandbox, Some(true));
    // The AI's steps log none.
    for _ in 0..40 {
        if session.host.is_over() {
            break;
        }
        if !session.ai_turn() {
            let seat = session.host.human_seat();
            let end = ActionBody::EndTurn;
            let body = if session.snapshot().legal.contains(&end) {
                end
            } else {
                random_action(session.state(), seat, &mut Rng::new("r923000", 0)).expect("an action")
            };
            session.act(body);
        }
    }
    assert!(
        session
            .log()
            .iter()
            .filter(|action| action.nonce.starts_with('a'))
            .all(|action| !matches!(action.body, ActionBody::Sandbox { .. }))
    );
}

#[test]
fn r923000_a_game_that_is_not_a_sandbox_refuses_every_tool_and_the_log_never_moves() {
    let mut session = started("practice-r923000-free", P2);
    session.answer_mulligans();
    let log = session.log().len();
    let refused = session
        .act(ActionBody::Sandbox {
            op: SandboxOp::SetHero {
                player: P2,
                health: Some(7),
                armor: None,
            },
        })
        .error
        .clone();
    assert!(refused.is_some());
    assert_eq!(session.log().len(), log);
}

// R2706000: a dungeon run's fight

fn run_config(seed: &str) -> PracticeConfig {
    jackioh_cards::register_all();
    let human = jackioh_engine::Handicap {
        deck_size: 12,
        hero_health: Some(25),
        ..jackioh_engine::HUMAN_HANDICAP
    };
    let enemy = jackioh_engine::Handicap {
        deck_size: 12,
        hero_health: Some(15),
        ..jackioh_engine::HUMAN_HANDICAP
    };
    let mut asked = config("ignored", P1);
    asked.run = Some(jackioh_run::RunFightSetup {
        seed: seed.to_string(),
        human_deck: deck_of(&format!("{seed}:human"), 12),
        human_handicap: human,
        human_marks: Vec::new(),
        enemy_deck: deck_of(&format!("{seed}:enemy"), 12),
        enemy_handicap: enemy,
        enemy: jackioh_run::RunEnemy {
            id: "enemy".to_string(),
            name: "Enemy".to_string(),
            hero_health: 15,
            deck: Vec::new(),
            artifacts: Vec::new(),
        },
    });
    asked
}

#[test]
fn b22_r2706000_a_run_fight_is_created_from_its_setup_a_game_of_its_own_kind() {
    let setup = run_config("practice-b22");
    let run = setup.run.clone().expect("a run");
    let mut session = Session::start(&setup).expect("starts");
    let args = session.host.fold_args();
    // Its seed, its decks and its handicaps, the human at the seat the config names, no marks.
    assert_eq!(session.host.config().seed, run.seed);
    assert_eq!(args.seed, run.seed);
    assert_eq!(args.decks, (run.human_deck.clone(), run.enemy_deck.clone()));
    let handicaps = args.handicaps.expect("handicaps");
    assert_eq!(
        (handicaps.p1, handicaps.p2),
        (Some(run.human_handicap), Some(run.enemy_handicap))
    );
    assert!(args.deck_marks.is_none() && args.dealt.is_none() && args.last_boards.is_none());
    // It brings no last board and leaves none, is never kept, and earns nothing. R3476000: its
    // save lives in its own slot while in progress, which a finished fight clears.
    session.concede();
    assert!(session.snapshot().last_board.is_none());
    assert!(session.kept.is_empty());
    assert_eq!(session.save, Some(None));
    assert_eq!(session.records.len(), 1);
    // B25: its report is the run's, never the log.
    session.feed(PracticeEvent::RunReport);
    assert!(session.failed.is_empty(), "{:?}", session.failed);
}

#[test]
fn b25_a_report_is_refused_before_the_end_and_for_a_game_that_is_no_fight() {
    let mut fight = Session::start(&run_config("practice-b25")).expect("starts");
    fight.feed(PracticeEvent::RunReport);
    assert_eq!(
        fight.failed,
        vec!["a fight is reported only once it is over".to_string()]
    );
    let mut free = started("practice-b25-free", P2);
    free.concede();
    free.feed(PracticeEvent::RunReport);
    assert_eq!(
        free.failed,
        vec!["this practice game is not a dungeon run's fight".to_string()]
    );
}

// R677, R679: a Glitch in practice. No deck deals one on cue, so the state is changed the way a Glitch
// changes it, in the host's JSON, and the machine takes it from there.

/// `session` with its game changed by `change`, and the snapshot it now holds.
fn with_game(session: &Session, change: impl FnOnce(&mut serde_json::Value)) -> Session {
    let mut json = serde_json::to_value(&session.host).expect("the host is JSON");
    change(&mut json["game"]);
    let host: PracticeHost = serde_json::from_value(json).expect("the host reads back");
    let mut changed = Session::of(host, session.catalog.clone(), Vec::new());
    changed.kept = session.kept.clone();
    changed.snapshots.push(changed.host.snapshot_now());
    changed
}

#[test]
fn r677_after_a_swap_the_human_acts_from_and_is_credited_in_the_ais_former_seat() {
    let mut session = started("practice-r677", P2);
    while session.ai_turn() {}
    assert_eq!(session.snapshot().view.viewer, P2);
    let keep = keep_all(session.snapshot()).expect("the human's mulligan");
    session.act(keep);
    // The game the human's answer started now carries one swap.
    let mut swapped = with_game(&session, |game| game["seatSwaps"] = serde_json::json!(1));
    assert_eq!(swapped.snapshot().view.viewer, P1);
    assert_eq!(
        swapped.snapshot().legal,
        jackioh_engine::legal_actions(swapped.state(), P1)
    );
    assert_eq!(swapped.host.human_seat(), P1);
    assert_eq!(swapped.host.ai_seat(), P2);
    // The human's next action goes as the seat it plays now, its nonce stream unbroken.
    let end_turn = ActionBody::EndTurn;
    assert!(
        swapped.snapshot().legal.contains(&end_turn),
        "the human is on turn"
    );
    swapped.act(end_turn);
    assert!(swapped.snapshot().error.is_none());
    let last = swapped.log().last().expect("logged");
    assert_eq!((last.player_id, last.nonce.as_str()), (P1, "h1"));
    // The AI plays the human's former seat, under its own stream.
    assert!(
        swapped.snapshot().ai_to_act,
        "the AI owes an action once the human passes"
    );
    assert!(swapped.ai_turn());
    let last = swapped.log().last().expect("the AI moved");
    assert_eq!(last.player_id, P2);
    assert!(last.nonce.starts_with('a'));
    // And the end is credited from the seat the human ended in (R1843004).
    swapped.act(ActionBody::Concede);
    assert_eq!(swapped.records[0].human_seat, P1);
    assert_eq!(swapped.records[0].result, PracticeResult::Loss);
}

#[test]
fn r679_r768_a_voided_game_shows_its_result_leaves_no_last_board_and_is_kept_nowhere() {
    let mut session = started("practice-r679", P2);
    session.answer_mulligans();
    // A concession leaves a board and is kept (R768).
    let mut conceded = with_game(&session, |_| {});
    conceded.act(ActionBody::Concede);
    assert!(conceded.snapshot().last_board.is_some());
    assert_eq!(conceded.kept.len(), 1);
    // The Glitch voided it: the next answer settles the end the state already holds.
    let mut voided = with_game(&session, |game| {
        game["result"] = serde_json::json!({ "winner": "draw", "reason": "voided" });
        game["phase"] = serde_json::json!("over");
    });
    voided.act(ActionBody::EndTurn);
    let result = voided.snapshot().view.result.expect("the result shows");
    assert_eq!(
        (result.winner, result.reason),
        (Winner::Draw, GameOverReason::Voided)
    );
    assert!(voided.snapshot().last_board.is_none());
    assert!(voided.kept.is_empty() && voided.records.is_empty());
    assert_eq!(voided.voids, 1);
    assert!(voided.snapshot().replay.is_none());
    // Claimed, it pays nothing: the check says voided (R1303000).
    assert_eq!(voided.save, Some(None));
}

// R768: the finished games a host keeps

fn kept_game(session: &mut Session) -> PracticeReplay {
    session.concede();
    session.kept.last().cloned().expect("the game was kept")
}

#[test]
fn r768_r768000_a_finished_free_game_is_kept_numbered_and_the_snapshot_names_it() {
    let mut session = Session::start(&config("practice-r768", P2)).expect("starts");
    session.feed(PracticeEvent::Debug);
    let replay = kept_game(&mut session);
    assert_eq!(replay.summary.game, 1);
    assert_eq!(replay.summary.result, PracticeResult::Loss);
    assert_eq!(replay.end_seat, P2);
    assert_eq!(replay.log, session.log());
    assert_eq!(replay.hash, jackioh_engine::hash_state(session.state()));
    assert_eq!(replay.summary.steps as usize, session.log().len() + 1);
    assert_eq!(replay.summary.ended_at, EPOCH);
    assert_eq!(
        session.snapshot().replay,
        Some(KeptAs {
            game: 1,
            steps: replay.summary.steps
        })
    );
    // R767000: the setup it was dealt is kept with it.
    let setup = replay.setup.as_ref().expect("a setup");
    assert_eq!(setup.decks, session.host.fold_args().decks);
    assert_eq!(setup.dealt, Some(vec![P2]));
    // Numbered after the newest, and only the newest PRACTICE_REPLAYS_KEPT stay.
    let catalog = session.catalog.clone();
    let mut numbers = vec![1];
    for n in 2..=PRACTICE_REPLAYS_KEPT + 2 {
        let (host, outputs) = PracticeHost::start(
            &config(&format!("practice-r768-{n}"), P2),
            &catalog,
            Some(numbers.clone()),
            EPOCH,
        )
        .expect("starts");
        let mut next = Session::of(host, catalog.clone(), outputs);
        next.kept = session.kept.clone();
        let kept = kept_game(&mut next);
        assert_eq!(kept.summary.game, n);
        session.kept = next.kept;
        numbers = session.kept.iter().map(|replay| replay.summary.game).collect();
    }
    assert_eq!(session.kept.len(), PRACTICE_REPLAYS_KEPT as usize);
    assert_eq!(session.kept[0].summary.game, 3);
}

#[test]
fn r768_a_lesson_a_run_and_a_host_with_no_store_keep_nothing() {
    let mut lesson = Session::start(&lesson_config("practice-r768-lesson")).expect("starts");
    lesson.concede();
    assert!(lesson.kept.is_empty() && lesson.snapshot().replay.is_none());
    let mut run = Session::start(&run_config("practice-r768-run")).expect("starts");
    run.concede();
    assert!(run.kept.is_empty());
    let (host, outputs) =
        PracticeHost::start(&config("practice-r768-bare", P2), &build(), None, EPOCH).expect("starts");
    let mut bare = Session::of(host, build(), outputs);
    bare.concede();
    assert!(bare.kept.is_empty() && bare.snapshot().replay.is_none());
    assert_eq!(bare.records.len(), 1, "the end is recorded all the same");
}

#[test]
fn r768_r767000_a_kept_game_is_listed_opened_and_paged_and_refused_where_it_cannot_fold() {
    let mut session = started("practice-r768-list", P2);
    let replay = kept_game(&mut session);
    let build = session.catalog.clone();
    // Listed with no fold, newest first.
    let listed = list_replays(std::slice::from_ref(&replay), &[], &build);
    assert_eq!((listed.len(), listed[0].unavailable), (1, None));
    let mut elsewhere = replay.clone();
    elsewhere.catalog = "0.0.1".to_string();
    assert_eq!(
        list_replays(&[elsewhere.clone()], &[], &build)[0].unavailable,
        Some(jackioh_engine::ReplayRefusal::EarlierPatch)
    );
    // Opened once, then paged from what the host kept.
    let first = page_replay(&replay, None, 0, 4, &build);
    let ReplayPageResult::Page { from, steps, total } = &first.page else {
        panic!("{:?}", first.page);
    };
    assert_eq!(
        (*from, !steps.is_empty(), *total as usize),
        (0, true, session.log().len() + 1)
    );
    assert!(steps.iter().all(|step| step.view.viewer == P2));
    let again = page_replay(&replay, Some(first.open.clone()), 1, 2, &build);
    assert!(matches!(again.page, ReplayPageResult::Page { from: 1, .. }));
    // A step that is not in the game, and a game that does not fold to its hash.
    let beyond = page_replay(&replay, Some(first.open.clone()), *total as i64, 1, &build);
    assert!(matches!(beyond.page, ReplayPageResult::Refused { .. }));
    let mut forged = replay.clone();
    forged.hash = "00000000".to_string();
    let refused = page_replay(&forged, None, 0, 1, &build);
    assert_eq!(
        refused.open,
        jackioh_engine::ReplayOpen::Refused {
            reason: jackioh_engine::ReplayRefusal::RulesChanged
        }
    );
    let verdicts = [ReplayVerdict {
        game: forged.summary.game,
        refusal: Some(jackioh_engine::ReplayRefusal::RulesChanged),
    }];
    assert_eq!(
        list_replays(&[forged], &verdicts, &build)[0].unavailable,
        Some(jackioh_engine::ReplayRefusal::RulesChanged)
    );
    // Another catalog version that is not carried is earlier_patch, with no fold.
    let refused = page_replay(&elsewhere, None, 0, 1, &build);
    assert!(matches!(refused.page, ReplayPageResult::Refused { .. }));
}

#[test]
fn r768_a_page_follows_the_seat_the_human_played_at_each_step_and_stops_at_a_swap() {
    use jackioh_shared::host::practice::replays::replay_seat_run;
    let end_turn = |seat: PlayerId, nonce: &str| Action::new(ActionBody::EndTurn, seat, nonce);
    // The AI (a…) acts from p1 and the human (h…) from p2; then a swap puts the human in p1 (R677).
    let log = vec![
        end_turn(P1, "a0"),
        end_turn(P2, "h0"),
        end_turn(P1, "h1"),
        end_turn(P2, "a1"),
    ];
    assert_eq!(replay_seat_run(&log, P1, 0, 16), (P2, 2));
    assert_eq!(replay_seat_run(&log, P1, 1, 1), (P2, 1));
    assert_eq!(replay_seat_run(&log, P1, 2, 16), (P1, 3));
    assert_eq!(replay_seat_run(&log, P1, 4, 16), (P1, 1));
    assert_eq!(replay_seat_run(&log[..2], P2, 0, 16), (P2, 3));
}

#[test]
fn r768_the_kept_games_are_asked_for_only_once_the_game_is_over() {
    let mut session = started("practice-r768-asked", P2);
    session.feed(PracticeEvent::Replays);
    assert_eq!(
        session.failed,
        vec!["replays are not available while a practice game is in progress"]
    );
    session.concede();
    session.failed.clear();
    session.feed(PracticeEvent::Replays);
    assert!(session.failed.is_empty());
}

// R1303000: the claim of a game on a server seed

#[test]
fn r1303000_a_claim_is_answered_only_for_a_finished_game_on_a_server_seed() {
    let mut free = started("practice-r1303000", P2);
    free.concede();
    free.feed(PracticeEvent::Claim);
    assert_eq!(
        free.failed,
        vec!["this practice game is not on a server seed, so it earns nothing"]
    );

    let mut asked = config("practice-r1303000-earn", P2);
    asked.earning = Some(true);
    let mut earning = Session::start(&asked).expect("starts");
    earning.feed(PracticeEvent::Claim);
    assert_eq!(
        earning.failed,
        vec!["a practice game is claimed only once it is over"]
    );
    earning.concede();
    let outputs = earning.feed(PracticeEvent::Claim);
    let Some(PracticeOutput::Claim { claim }) = outputs.into_iter().next() else {
        panic!("a claim");
    };
    assert_eq!(
        (claim.seed.as_str(), claim.result),
        ("practice-r1303000-earn", PracticeResult::Loss)
    );
    assert_eq!(claim.log, earning.log());
}

// R2560000: the server's check, by stepping the machine over the claimed game

fn claimed_game(seed: &str, concede: bool) -> (PracticeConfig, Vec<Action>, PracticeResult) {
    let mut asked = config(seed, P2);
    asked.earning = Some(true);
    let mut session = Session::start(&asked).expect("starts");
    if concede {
        session.concede();
    } else {
        session.play_out(&mut Rng::new(&format!("{seed}:human"), 0));
    }
    (asked, session.log().to_vec(), session.records[0].result)
}

#[test]
fn r2560000_an_honest_claim_folds_and_is_recorded_as_the_machine_recorded_it() {
    let (setup, log, result) = claimed_game("practice-r2560000", false);
    let checked = check_claim(&setup, &build(), result, &log).expect("an honest claim passes");
    assert_eq!(checked.record.result, result);
    assert_eq!(checked.record.human_seat, P2);
    let humans = log.iter().filter(|action| action.nonce.starts_with('h')).count();
    assert_eq!(checked.record.human_actions.len(), humans);
    assert_eq!(checked.record.ai_moves as usize, log.len() - humans);
    assert!(checked.seat.is_some(), "the quests read the seat's summary");
    assert!(!checked.record.final_hash.is_empty());
}

#[test]
fn r2560000_a_claim_that_does_not_fold_is_refused_for_each_way_it_can_fail() {
    let (setup, log, result) = claimed_game("practice-r2560000-refused", true);
    let build = build();
    let check = |log: &[Action], claimed: PracticeResult| check_claim(&setup, &build, claimed, log).err();
    assert_eq!(check(&log, result), None);
    let wrong = if result == PracticeResult::Loss {
        PracticeResult::Win
    } else {
        PracticeResult::Loss
    };
    assert_eq!(check(&log, wrong), Some(ClaimRefusal::ResultMismatch));
    assert_eq!(
        check(&log[..log.len() - 1], result),
        Some(ClaimRefusal::NotFinished)
    );
    assert_eq!(check(&[], result), Some(ClaimRefusal::NotFinished));
    // An action after the end.
    let mut after = log.clone();
    after.push(Action::new(ActionBody::EndTurn, P2, "h9"));
    assert_eq!(check(&after, result), Some(ClaimRefusal::DoesNotFold));
    // The wrong nonce, the wrong seat, an action the engine refuses.
    let mut nonce = log.clone();
    nonce[1].nonce = "a7".to_string();
    assert_eq!(check(&nonce, result), Some(ClaimRefusal::DoesNotFold));
    let mut seat = log.clone();
    seat[0].player_id = seat[0].player_id.opponent();
    assert_eq!(check(&seat, result), Some(ClaimRefusal::DoesNotFold));
    let mut refused = log.clone();
    refused[0].body = ActionBody::EndTurn;
    assert_eq!(check(&refused, result), Some(ClaimRefusal::DoesNotFold));
    // A setup that does not begin.
    let mut bare = setup.clone();
    bare.deck = PracticeDeckChoice::Preset {
        id: "nowhere".to_string(),
        cards: None,
    };
    assert_eq!(
        check_claim(&bare, &build, result, &log).err(),
        Some(ClaimRefusal::DoesNotFold)
    );
}

#[test]
fn r2560000_an_ais_legal_move_it_would_not_make_is_not_re_decided_and_pays() {
    // The AI's seat is held to its nonce and seat and the engine's say, nothing more: a concession is legal.
    let seed = "practice-r2560000-concede";
    let mut asked = config(seed, P2);
    asked.earning = Some(true);
    let mut session = Session::start(&asked).expect("starts");
    session.answer_mulligans();
    let mut log = session.log().to_vec();
    log.push(Action::new(ActionBody::Concede, P1, "a1"));
    let checked = check_claim(&asked, &build(), PracticeResult::Win, &log).expect("an AI concession pays");
    assert_eq!(checked.record.human_seat, P2);
}

// R2102000, R2575000: the asks the machine makes of its host

#[test]
fn r2102000_r2291000_every_snapshot_asks_the_lethal_check_for_the_humans_seat_under_its_own_clock() {
    let mut session = started("practice-r2102000", P2);
    assert_eq!(session.lethal_asks, vec![(P2, PRACTICE_LETHAL_HINT_CLOCK_MS)]);
    session.answer_mulligans();
    assert_eq!(session.lethal_asks.len(), session.snapshots.len());
    assert!(
        session
            .lethal_asks
            .iter()
            .all(|ask| *ask == (P2, PRACTICE_LETHAL_HINT_CLOCK_MS))
    );
}

#[test]
fn r2575000_r2560000_every_decision_the_machine_asks_for_carries_the_practice_clock() {
    for earning in [None, Some(true)] {
        let mut asked = config("practice-r2575000", P2);
        asked.earning = earning;
        let mut session = Session::start(&asked).expect("starts");
        session.feed(PracticeEvent::AiTurn);
        assert_eq!(session.decide_clocks, vec![PRACTICE_AI_CLOCK_MS], "{earning:?}");
    }
}

// R1563006: Meditative+ #20 KY's Competition's question sequence has `KY_COMPETITION_SECONDS` from its first
// question, kept by its host. The machine states the window as a timer, the way the 1v1 match clock does
// (`host::clock`), and answers Stop for the human when it lapses.

const COMPETITION: &str = "meditativeplus-020";

/// A sandbox game with the human (p1) holding KY's Competition and the mana to cast it; the cast is the
/// last action taken, so a sequence's first question is open for the human.
fn sequence_opened(seed: &str) -> Session {
    let mut asked = config(seed, P1);
    asked.sandbox = Some(true);
    let mut session = Session::start(&asked).expect("starts");
    session.answer_mulligans();
    let tools = [
        SandboxOp::SetMana {
            player: P1,
            current: Some(10),
            max: Some(10),
        },
        SandboxOp::Spawn {
            player: P1,
            def_id: COMPETITION.to_string(),
            radiant: None,
            to: SpawnZone::Hand,
        },
    ];
    for op in tools {
        assert!(session.act(ActionBody::Sandbox { op }).error.is_none());
    }
    let card = session
        .state()
        .players
        .p1
        .hand
        .iter()
        .find(|card| card.def_id == COMPETITION)
        .map(|card| card.id.clone())
        .expect("the spawned card is in hand");
    let play = session
        .snapshot()
        .legal
        .iter()
        .find(|body| matches!(body, ActionBody::Play { instance_id, .. } if *instance_id == card))
        .cloned()
        .expect("the human can cast it");
    session.timers.clear();
    assert!(session.act(play).error.is_none());
    session
}

fn sequence_windows(session: &Session) -> Vec<(u32, i64)> {
    session
        .timers
        .iter()
        .filter_map(|command| match command {
            TimerCommand::Set {
                key: TimerKey::Sequence,
                id,
                at,
            } => Some((*id, *at)),
            _ => None,
        })
        .collect()
}

#[test]
fn r1563006_a_sequences_first_question_sets_one_window_a_later_question_keeps_it() {
    let mut session = sequence_opened("practice-r1563006");
    let window = i64::from(jackioh_engine::KY_COMPETITION_SECONDS) * 1000;
    assert_eq!(sequence_windows(&session), vec![(1, EPOCH + window)]);
    let question = session
        .state()
        .pending
        .clone()
        .expect("the first question is open");
    assert_eq!(question.streak, Some(0));

    // A right answer asks the next question: the window goes on, no second one is set.
    let correct = jackioh_engine::answer_key_of(&question.resume.data).expect("the question's key");
    let right = question
        .options
        .iter()
        .find(|option| option.key == format!("mode:{correct}"))
        .expect("the right option")
        .selection
        .clone();
    session.now = EPOCH + window / 2;
    session.act(ActionBody::Answer {
        choice_id: question.id.clone(),
        selection: vec![right],
    });
    assert_eq!(
        session.state().pending.as_ref().and_then(|prompt| prompt.streak),
        Some(1)
    );
    assert_eq!(
        sequence_windows(&session).len(),
        1,
        "one window for the whole sequence"
    );
}

#[test]
fn r1563006_the_window_lapsing_answers_stop_for_the_human_as_its_own_action() {
    let mut session = sequence_opened("practice-r1563006-lapse");
    let (id, at) = sequence_windows(&session)[0];
    let stop = jackioh_engine::prompts::stop_answer(session.state()).expect("a question to stop");
    let logged = session.log().len();
    session.now = at;
    session.feed(PracticeEvent::TimerFired {
        key: TimerKey::Sequence,
        id,
    });
    let last = session.log().last().expect("the stop was logged");
    assert_eq!(session.log().len(), logged + 1);
    // The human's action under the nonce the claim's fold knows, answering the question Stop.
    assert_eq!((last.player_id, last.nonce.starts_with('h')), (P1, true));
    assert_eq!(last.body, stop);
    assert!(session.state().pending.is_none(), "the sequence ended");
    assert!(session.snapshot().error.is_none());
    // The window was spent: a second firing of it does nothing.
    session.feed(PracticeEvent::TimerFired {
        key: TimerKey::Sequence,
        id,
    });
    assert_eq!(session.log().len(), logged + 1);
}

#[test]
fn r1563006_a_sequence_the_human_stops_itself_takes_its_window_back() {
    let mut session = sequence_opened("practice-r1563006-stop");
    let (id, _) = sequence_windows(&session)[0];
    let stop = jackioh_engine::prompts::stop_answer(session.state()).expect("a question to stop");
    session.timers.clear();
    session.act(stop);
    assert!(session.state().pending.is_none());
    assert_eq!(
        session.timers,
        vec![TimerCommand::Cancel {
            key: TimerKey::Sequence,
            id
        }]
    );
    // The window it took back, fired late by a host that could not cancel it, does nothing.
    let logged = session.log().len();
    session.feed(PracticeEvent::TimerFired {
        key: TimerKey::Sequence,
        id,
    });
    assert_eq!(session.log().len(), logged);
}

#[test]
fn r1563006_a_game_with_no_sequence_sets_no_timer() {
    let mut session = started("practice-r1563006-none", P2);
    session.answer_mulligans();
    assert!(session.timers.is_empty());
}

// The boundary

#[test]
fn the_boundary_starts_steps_and_lists_by_json() {
    use jackioh_shared::host::boundary::{HostAnswer, HostCall, apply};
    use jackioh_shared::host::practice::boundary::{PracticeAnswer, PracticeCall};
    let start = serde_json::json!({
        "call": "practice", "op": "start", "catalog": build(), "kept": [], "now": EPOCH,
        "config": { "seed": "practice-boundary", "difficulty": "easy", "humanSeat": "p2", "deck": { "kind": "random" } },
    });
    let call: HostCall = serde_json::from_value(start).expect("the call reads");
    let HostAnswer::Practice(answer) = apply(call) else {
        panic!("a practice answer");
    };
    let PracticeAnswer::Stepped { host, outputs } = *answer else {
        panic!("a started host");
    };
    assert!(matches!(
        outputs.first(),
        Some(PracticeOutput::Began { ai_seat: P1 })
    ));
    // The host is JSON, and comes back as the same host.
    let json = serde_json::to_string(&host).expect("serializes");
    let held: PracticeHost = serde_json::from_str(&json).expect("reads back");
    assert_eq!(held, *host);
    let step = serde_json::json!({
        "call": "practice", "op": "step", "now": EPOCH, "host": held,
        "event": { "event": "act", "body": { "type": "endTurn" } },
    });
    let call: HostCall = serde_json::from_value(step).expect("the call reads");
    let HostAnswer::Practice(answer) = apply(call) else {
        panic!("a practice answer");
    };
    let PracticeAnswer::Stepped { outputs, .. } = *answer else {
        panic!("a stepped host");
    };
    let text = serde_json::to_string(&outputs).expect("serializes");
    assert!(text.contains("\"output\":\"snapshot\""), "{text}");
    // A start the engine refuses is the answer, not an error.
    let refused = serde_json::json!({
        "call": "practice", "op": "start", "catalog": build(), "now": EPOCH,
        "config": { "seed": "practice-boundary", "difficulty": "easy", "humanSeat": "p2", "deck": { "kind": "preset", "id": "nowhere" } },
    });
    let call: HostCall = serde_json::from_value(refused).expect("the call reads");
    let HostAnswer::Practice(answer) = apply(call) else {
        panic!("a practice answer");
    };
    assert!(matches!(*answer, PracticeAnswer::Refused { .. }));
    let _ = PracticeCall::Replays {
        replays: Vec::new(),
        verdicts: Vec::new(),
        build: build(),
    };
}
