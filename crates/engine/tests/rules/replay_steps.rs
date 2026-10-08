// R768, issue #508: what a replay shows. A seat's view at any step of a finished game is
// `view_for` of the state after that many accepted actions; a page and a step cost at most
// REPLAY_CHECKPOINT_EVERY `reduce` calls; a rejected action is no step; another catalog version or
// another final hash is refused with no step.

use jackioh_engine::config::{REPLAY_CHECKPOINT_EVERY, REPLAY_PAGE_STEPS};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{play_random_game, setup_catalog};

/// The build's catalog version, and the one the games below record.
const BUILD: &str = "v-replay";

fn input(seed: &str, decks: &(Vec<String>, Vec<String>), log: Vec<Action>) -> ReplayInput {
    ReplayInput {
        seed: seed.to_string(),
        decks: decks.clone(),
        log,
        ..Default::default()
    }
}

fn record(hash: &str) -> ReplayRecord {
    ReplayRecord {
        catalog_version: BUILD.to_string(),
        final_hash: hash.to_string(),
    }
}

/// Open a replay that must be ready: its step count and checkpoints.
fn opened(input: &ReplayInput, hash: &str) -> (usize, ReplayCheckpoints) {
    match replay_open(input, &record(hash), BUILD) {
        ReplayOpen::Ready { steps, checkpoints } => (steps, checkpoints),
        ReplayOpen::Refused { reason } => panic!("{}: the replay was refused: {reason}", input.seed),
    }
}

/// Every step of `seat`, a page at a time from step 0, each page holding to the reduce budget.
fn all_steps(checkpoints: &ReplayCheckpoints, seat: PlayerId) -> Vec<ReplayStep> {
    let total = checkpoints.accepted.len() + 1;
    let mut steps: Vec<ReplayStep> = Vec::new();
    while steps.len() < total {
        let page = replay_page(checkpoints, seat, steps.len(), REPLAY_PAGE_STEPS);
        assert!(
            page.reduces <= REPLAY_CHECKPOINT_EVERY,
            "a page made {} reduces",
            page.reduces
        );
        assert!(!page.steps.is_empty(), "no step at {}", steps.len());
        steps.extend(page.steps);
    }
    steps
}

#[test]
fn r768_every_step_is_view_for_after_a_fresh_fold_of_its_prefix() {
    for seed in ["replay-steps-1", "replay-steps-2", "replay-steps-3"] {
        let live = play_random_game(seed, None);
        setup_catalog();
        let (steps, checkpoints) = opened(
            &input(seed, &live.decks, live.log.clone()),
            &hash_state(&live.state),
        );
        assert_eq!(steps, live.log.len() + 1, "{seed}");
        assert_eq!(
            checkpoints.states.len(),
            live.log.len() / REPLAY_CHECKPOINT_EVERY + 1,
            "{seed}"
        );

        for seat in PLAYER_IDS {
            let shown = all_steps(&checkpoints, seat);
            assert_eq!(shown.len(), steps, "{seed} {seat}");
            for (k, step) in shown.iter().enumerate() {
                let folded = fold(&input(seed, &live.decks, live.log[..k].to_vec()));
                assert!(folded.errors.is_empty(), "{seed} step {k}");
                assert_eq!(step.step, k, "{seed} {seat}");
                assert_eq!(step.turn, folded.state.turn, "{seed} {seat} step {k}");
                assert_eq!(step.view, view_for(&folded.state, seat), "{seed} {seat} step {k}");
            }
        }
    }
}

#[test]
fn r768_a_step_or_a_page_costs_at_most_the_checkpoint_spacing() {
    let seed = "replay-steps-4";
    let live = play_random_game(seed, None);
    setup_catalog();
    let (steps, checkpoints) = opened(
        &input(seed, &live.decks, live.log.clone()),
        &hash_state(&live.state),
    );
    assert!(
        steps > 2 * REPLAY_CHECKPOINT_EVERY,
        "the game is too short to test checkpoints"
    );

    let singles = all_steps(&checkpoints, PlayerId::P1);
    for from in 0..steps {
        let one = replay_page(&checkpoints, PlayerId::P1, from, 1);
        assert_eq!(one.steps.len(), 1, "step {from}");
        assert_eq!(one.steps[0].step, from);
        assert_eq!(one.reduces, from % REPLAY_CHECKPOINT_EVERY, "step {from}");

        let page = replay_page(&checkpoints, PlayerId::P1, from, REPLAY_PAGE_STEPS);
        assert!(!page.steps.is_empty(), "page {from}");
        assert!(
            page.reduces <= REPLAY_CHECKPOINT_EVERY,
            "page {from}: {} reduces",
            page.reduces
        );
        assert!(page.steps.len() <= REPLAY_PAGE_STEPS, "page {from}");
        for (offset, step) in page.steps.iter().enumerate() {
            assert_eq!(step.step, from + offset, "page {from}");
            assert_eq!(
                *step,
                singles[from + offset],
                "page {from} step {}",
                from + offset
            );
        }
        if from % REPLAY_PAGE_STEPS == 0 {
            assert_eq!(
                page.steps.len(),
                REPLAY_PAGE_STEPS.min(steps - from),
                "page {from}"
            );
        }
    }

    for (from, count) in [(steps, 1), (0, 0)] {
        let page = replay_page(&checkpoints, PlayerId::P1, from, count);
        assert!(page.steps.is_empty(), "page {from}+{count}");
        assert_eq!(page.reduces, 0, "page {from}+{count}");
    }
    let capped = replay_page(&checkpoints, PlayerId::P1, 0, 3 * REPLAY_PAGE_STEPS);
    assert!(capped.steps.len() <= REPLAY_PAGE_STEPS);
}

#[test]
fn r768_a_rejected_action_is_not_a_step() {
    let seed = "replay-steps-5";
    let live = play_random_game(seed, None);
    setup_catalog();
    let hash = hash_state(&live.state);

    // The game's log with an action the reducer refuses before every fifth entry and after the last:
    // an end of turn from the seat that is not the active one.
    let mut state = fold(&input(seed, &live.decks, Vec::new())).state;
    let mut junked: Vec<Action> = Vec::new();
    let mut junk = 0;
    for (i, action) in live.log.iter().enumerate() {
        if i % 5 == 2 {
            junked.push(refused_action(&state, i));
            junk += 1;
        }
        state = reduce(&state, action).state;
        junked.push(action.clone());
    }
    junked.push(refused_action(&state, live.log.len()));
    junk += 1;
    assert_eq!(hash_state(&state), hash, "the replayed game is the live one");

    let folded = fold(&input(seed, &live.decks, junked.clone()));
    assert_eq!(folded.errors.len(), junk);

    let (steps, checkpoints) = opened(&input(seed, &live.decks, junked), &hash);
    assert_eq!(steps, live.log.len() + 1);
    assert_eq!(checkpoints.accepted, live.log);

    let (clean_steps, clean) = opened(&input(seed, &live.decks, live.log.clone()), &hash);
    assert_eq!(clean_steps, steps);
    for seat in PLAYER_IDS {
        assert_eq!(all_steps(&checkpoints, seat), all_steps(&clean, seat), "{seat}");
    }
}

/// An end of turn from the seat that is not the active one, which `reduce` refuses.
fn refused_action(state: &GameState, i: usize) -> Action {
    let action = Action::new(ActionBody::EndTurn, state.active.opponent(), format!("junk-{i}"));
    assert!(reduce(state, &action).error.is_some(), "junk-{i} was accepted");
    action
}

#[test]
fn r768_another_catalog_version_or_a_wrong_hash_is_refused_with_no_steps() {
    let seed = "replay-steps-6";
    let live = play_random_game(seed, None);
    setup_catalog();
    let hash = hash_state(&live.state);
    let game = input(seed, &live.decks, live.log.clone());

    let earlier = ReplayRecord {
        catalog_version: "v-earlier".to_string(),
        final_hash: hash.clone(),
    };
    let refused = |reason| ReplayOpen::Refused { reason };
    assert_eq!(
        replay_open(&game, &earlier, BUILD),
        refused(ReplayRefusal::EarlierPatch)
    );
    let earlier_and_wrong = ReplayRecord {
        final_hash: "00000000".to_string(),
        ..earlier
    };
    assert_eq!(
        replay_open(&game, &earlier_and_wrong, BUILD),
        refused(ReplayRefusal::EarlierPatch)
    );

    assert_eq!(
        replay_open(&game, &record("00000000"), BUILD),
        refused(ReplayRefusal::RulesChanged)
    );
    let short = input(seed, &live.decks, live.log[..live.log.len() - 1].to_vec());
    assert_eq!(
        replay_open(&short, &record(&hash), BUILD),
        refused(ReplayRefusal::RulesChanged)
    );

    // The refusal's wire shape, which the client's `ReplayOpen` types.
    assert_eq!(
        serde_json::to_value(refused(ReplayRefusal::RulesChanged)).expect("serialises"),
        json!({ "kind": "refused", "reason": "rules_changed" })
    );
}
