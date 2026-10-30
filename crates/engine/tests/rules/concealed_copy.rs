//! R1714000 (issue #1714): the concealed copy a dry run plays on (R222) and the autopilot's tries
//! play on (R1183003) keeps no record its viewer may not read, so two states that differ only in what
//! the viewer may not read give the same copy and the same scores.

use jackioh_engine::effects::{answered_correctly, choose_answer, damage, guess_secret};
use jackioh_engine::subsystems::autopilot;
use jackioh_engine::subsystems::board_history::record_board_snapshot;
use jackioh_engine::subsystems::scorer::{
    HIDDEN_CARD_DEF_ID, HIDDEN_TRAP_DEF_ID, ScorerOptions, concealed_copy, dry_run_base, rank,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::spell_def;
use crate::rules::fixtures::harness::run_effect;
use crate::rules::fixtures::outside_data::register_outside_data;

const SNARE: &str = "cc-snare";
const OTHER_SNARE: &str = "cc-other-snare";
const QUIZ: &str = "cc-quiz";
/// A vanilla Unit: what both seats' hands and libraries are dealt.
const FILLER: &str = "fx-10";
const OWN_BOARD: &str = "fx-3";
const ANSWERED: &str = "answered";
const PRIZE: i32 = 3;

fn register() {
    register_outside_data();
    let mut catalog = registered_catalog().clone();
    for def in [
        spell_def(
            17140,
            json!({ "id": SNARE, "name": "Snare (fixture)", "type": "Trap" }),
        ),
        spell_def(
            17141,
            json!({ "id": OTHER_SNARE, "name": "Other Snare (fixture)", "type": "Trap" }),
        ),
        spell_def(17142, json!({ "id": QUIZ, "name": "Quiz (fixture)" })),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let script = Script {
        cry: Some(hook(|_ctx| {
            vec![choose_answer(json_as(json!({
                "step": ANSWERED,
                "statement": "2 + 2 = ?",
                "options": ["3", "5", "4", "22"],
                "correct": 2,
                "shuffle": false,
            })))]
        })),
        resume: IndexMap::from([(
            ANSWERED,
            hook(|ctx| {
                if answered_correctly(ctx) {
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": PRIZE }),
                    ))]
                } else {
                    vec![]
                }
            }),
        )]),
        ..Script::default()
    };
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        QUIZ.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// p2's face-down `trap` in lane 1 with two Plague Counters, the board recorded in the history (R419).
fn board(trap: &str) -> GameState {
    register();
    let s = scenario(json!({
        "seed": "r1714000-board",
        "p1": { "hand": [FILLER], "library": [FILLER, FILLER] },
        "p2": {
            "hand": [FILLER],
            "backrow": [{ "def": trap, "lane": 1, "counters": { "plague": 2 } }],
            "library": [FILLER],
        },
    }));
    let mut state = s.state().clone();
    record_board_snapshot(&mut state);
    state
}

fn entry(def_id: &str) -> LastBoardEntry {
    LastBoardEntry {
        def_id: def_id.to_string(),
        radiant: false,
    }
}

/// Every record a viewer may not read, written into `state` (p2's unrevealed secret holds `choice`).
fn hide_records(state: &mut GameState, deck: &str, their_board: &str, choice: SecretChoice) {
    state.opening = Some(OpeningRecord {
        decks: (vec![deck.to_string()], vec![deck.to_string()]),
        dealt: None,
    });
    state.glitch_boards = Some(PerPlayerOpt {
        p1: Some(vec![entry(their_board)]),
        p2: Some(vec![entry(their_board)]),
    });
    state.last_boards = Some(PerPlayerOpt {
        p1: Some(vec![entry(OWN_BOARD)]),
        p2: Some(vec![entry(their_board)]),
    });
    state.secrets = Some(vec![
        SecretRecord {
            id: "secret-theirs".to_string(),
            owner: P2,
            choice: Some(choice),
            revealed: None,
        },
        SecretRecord {
            id: "secret-revealed".to_string(),
            owner: P2,
            choice: Some(SecretChoice::Greed),
            revealed: Some(true),
        },
        SecretRecord {
            id: "secret-mine".to_string(),
            owner: P1,
            choice: Some(SecretChoice::Defend),
            revealed: None,
        },
    ]);
}

fn choice_of(state: &GameState, id: &str) -> Option<SecretChoice> {
    secret_of(state, id).expect("the secret is filed").choice
}

fn run(state: &mut GameState, player: PlayerId, effect: Effect) -> Vec<GameEvent> {
    run_effect(state, effect, player)
}

mod r1714000_a_dry_runs_concealed_copy {
    use super::*;

    #[test]
    fn r1714000_the_copy_keeps_no_record_its_viewer_may_not_read() {
        let mut state = board(SNARE);
        hide_records(&mut state, "fx-1", "fx-4", SecretChoice::Attack);
        {
            let card = &mut state.players[P2].hand[0];
            card.created = Some(true);
            card.chinese = Some(true);
            card.return_to_hand_at_end_of_turn = Some(true);
            card.counters.grade = Some(2);
            card.x = Some(3);
        }

        let copy = concealed_copy(&state, P1);
        assert_eq!(copy.opening, None);
        assert_eq!(copy.glitch_boards, None);
        assert_eq!(
            copy.last_boards,
            Some(PerPlayerOpt {
                p1: Some(vec![entry(OWN_BOARD)]),
                p2: None,
            })
        );
        assert_eq!(choice_of(&copy, "secret-theirs"), None);
        assert_eq!(choice_of(&copy, "secret-revealed"), Some(SecretChoice::Greed));
        assert_eq!(choice_of(&copy, "secret-mine"), Some(SecretChoice::Defend));

        let hidden = &copy.players[P2].hand[0];
        assert_eq!(hidden.def_id, HIDDEN_CARD_DEF_ID);
        assert_eq!(hidden.created, None);
        assert_eq!(hidden.chinese, None);
        assert_eq!(hidden.x, None);
        assert_eq!(hidden.return_to_hand_at_end_of_turn, None);
        assert_eq!(hidden.counters, Counters::default());

        // R471: a face-down card's Plague Counters are public wherever it sits, so its stand-in keeps them.
        let trap = copy.players[P2].backrow[0]
            .as_ref()
            .expect("the trap is in lane 1");
        assert_eq!(trap.def_id, HIDDEN_TRAP_DEF_ID);
        assert_eq!(trap.counters.plague, Some(2));
        // R419: Rollback's snapshot of the field shows the same stand-in.
        let snapshot = copy.board_history.as_ref().and_then(|history| history.last());
        let held = snapshot.expect("the history holds a snapshot").sides[P2].backrow[0]
            .as_ref()
            .expect("the snapshot holds the trap");
        assert_eq!(held.def_id, HIDDEN_TRAP_DEF_ID);
        assert_eq!(held.counters.plague, Some(2));
        let real = state.board_history.as_ref().and_then(|history| history.last());
        let real = real.expect("the history holds a snapshot").sides[P2].backrow[0]
            .as_ref()
            .expect("the snapshot holds the trap");
        assert_eq!(real.def_id, SNARE);

        // The other seat reads its own secret and its own last board, and not p1's.
        let theirs = concealed_copy(&state, P2);
        assert_eq!(choice_of(&theirs, "secret-theirs"), Some(SecretChoice::Attack));
        assert_eq!(choice_of(&theirs, "secret-mine"), None);
        assert_eq!(
            theirs.last_boards,
            Some(PerPlayerOpt {
                p1: None,
                p2: Some(vec![entry("fx-4")]),
            })
        );
    }

    #[test]
    fn r1714000_states_differing_only_in_what_p1_may_not_read_conceal_and_score_alike() {
        let mut a = board(SNARE);
        hide_records(&mut a, "fx-1", "fx-4", SecretChoice::Attack);
        let mut b = board(OTHER_SNARE);
        hide_records(&mut b, "fx-2", "fx-5", SecretChoice::Defend);
        {
            let card = &mut b.players[P2].hand[0];
            card.def_id = "fx-11".to_string();
            card.created = Some(true);
            card.chinese = Some(true);
            card.counters.grade = Some(2);
        }
        assert_ne!(a, b);

        assert_eq!(concealed_copy(&a, P1), concealed_copy(&b, P1));
        let base = dry_run_base(&a, P1).expect("p1 may play now");
        assert_eq!(Some(base), dry_run_base(&b, P1));
        let options = ScorerOptions::default();
        assert_eq!(rank(&a, P1, &options), rank(&b, P1, &options));
    }

    #[test]
    fn r1714000_r865_a_guess_tried_on_the_copy_judges_nothing() {
        let mut state = board(SNARE);
        hide_records(&mut state, "fx-1", "fx-4", SecretChoice::Attack);
        let guess = || guess_secret(json_as(json!({ "secretId": "secret-theirs", "guess": "defend" })));

        let mut copy = concealed_copy(&state, P1);
        assert_eq!(run(&mut copy, P1, guess()), vec![]);
        assert_eq!(
            secret_of(&copy, "secret-theirs").map(|secret| secret.revealed),
            Some(None)
        );

        let events = run(&mut state, P1, guess());
        assert!(events.iter().any(|event| matches!(
            event,
            GameEvent::Predicted {
                outcome: PredictOutcome::Won,
                ..
            }
        )));
    }

    #[test]
    fn r1714000_r1183003_an_answer_trial_is_judged_by_no_key() {
        register();
        let mut s = scenario(json!({
            "seed": "r1714000-quiz",
            "p1": { "hand": [QUIZ], "library": [FILLER, FILLER], "mana": 4 },
            "p2": { "hand": [FILLER], "library": [FILLER] },
        }));
        s.play(QUIZ, json!({}));
        let state = s.state().clone();

        let real = state.pending.as_ref().expect("the problem is open");
        assert_eq!(real.resume.data.get(ANSWER_KEY), Some(&json!("C")));
        let copy = concealed_copy(&state, P1);
        let held = copy.pending.as_ref().expect("the copy holds the problem too");
        assert_eq!(held.resume.data.get(ANSWER_KEY), None);
        assert_eq!(held.options, real.options);
        assert_eq!(held.resume.step, real.resume.step);
        assert_eq!(held.resume.hook, real.resume.hook);

        // No answer is judged right on the copy, so none outranks the first listed.
        let first = legal_actions(&state, P1)
            .into_iter()
            .find(|action| action.action_type() == ActionType::Answer)
            .expect("an answer is listed");
        assert_eq!(autopilot::choose(&state, P1), Some(first.clone()));
        // Nor does the key the real state holds move the choice.
        let mut other = state.clone();
        other
            .pending
            .as_mut()
            .expect("the problem is open")
            .resume
            .data
            .insert(ANSWER_KEY.to_string(), json!("D"));
        assert_eq!(autopilot::choose(&other, P1), Some(first));

        s.answer(json!("C"));
        s.expect_health("p2", HERO_HEALTH - PRIZE);
    }
}
