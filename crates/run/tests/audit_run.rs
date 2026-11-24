//! Audit of the dungeon run (SPEC §15): the tests that prove the reported findings. Each asserts what
//! the spec says and fails on the code as it stands.

use jackioh_engine::config::{GLITCH_DEF_ID, HUMAN_HANDICAP, Handicap};
use jackioh_engine::reduce::{begin_game, legal_actions, reduce, seat_to_act};
use jackioh_engine::replay::ReplayInput;
use jackioh_engine::state::{CreateGameOptions, GameState, create_recorded_game};
use jackioh_engine::wire::{
    Action, ActionBody, GameOverReason, PerPlayerOpt, PlayerId, SandboxOp, SpawnZone, Winner,
};
use jackioh_run::config::RUN_STARTER_SIZE;
use jackioh_run::{
    RunAction, RunSetup, RunUnlocks, run_create, run_fight_report, run_placeholder_content, run_reduce,
    run_seed,
};

fn tg(n: &str) -> String {
    format!("traininggrounds-{n}")
}

/// One accepted action, logged under a fresh nonce.
fn send(state: &mut GameState, log: &mut Vec<Action>, seat: PlayerId, body: ActionBody) {
    let action = Action::new(body, seat, format!("n{}", log.len()));
    let result = reduce(state, &action);
    assert_eq!(result.error, None, "{action:?}");
    log.push(action);
    *state = result.state;
}

/// A fight the enemy's Glitch voided (R676's fourth outcome, R679), as the fold input a fight's report
/// reads: two small decks under a fight's two handicaps (the human P1 at 20, the enemy P2 at 10), both
/// opening hands kept, P1's first turn ended, then P2 plays a Glitch. The game is a sandbox only so the
/// test can put the Glitch in P2's hand: a real fight meets one at R673's 1 in 10,000 per generated card
/// after a "... in the System" play. The seed is the first whose Glitch draws the void.
fn voided_fight() -> ReplayInput {
    jackioh_cards::register_all();
    let decks = (
        ["058", "042", "043", "001", "008"].map(tg).to_vec(),
        ["006", "007", "047", "054", "048"].map(tg).to_vec(),
    );
    let handicap = |hero_health| Handicap {
        deck_size: 5,
        hero_health: Some(hero_health),
        ..HUMAN_HANDICAP
    };
    let handicaps = PerPlayerOpt {
        p1: Some(handicap(20)),
        p2: Some(handicap(10)),
    };
    for n in 0..200 {
        let seed = format!("ABCDEF-0-{n}");
        let options = CreateGameOptions {
            seed: seed.clone(),
            decks: decks.clone(),
            handicaps: Some(handicaps.clone()),
            sandbox: Some(true),
            ..Default::default()
        };
        let mut state = begin_game(&create_recorded_game(&options)).state;
        let mut log: Vec<Action> = Vec::new();
        loop {
            let seat = seat_to_act(&state).expect("a seat acts");
            if !legal_actions(&state, seat)
                .iter()
                .any(|body| matches!(body, ActionBody::Mulligan { .. }))
            {
                break;
            }
            let keep = state.players[seat].hand.iter().map(|c| c.id.clone()).collect();
            send(&mut state, &mut log, seat, ActionBody::Mulligan { keep });
        }
        send(&mut state, &mut log, PlayerId::P1, ActionBody::EndTurn);
        assert_eq!(seat_to_act(&state), Some(PlayerId::P2), "the enemy's turn");
        let spawn = SandboxOp::Spawn {
            player: PlayerId::P2,
            def_id: GLITCH_DEF_ID.to_string(),
            radiant: None,
            to: SpawnZone::Hand,
        };
        send(&mut state, &mut log, PlayerId::P2, ActionBody::Sandbox { op: spawn });
        let glitch = state.players[PlayerId::P2]
            .hand
            .iter()
            .find(|card| card.def_id == GLITCH_DEF_ID)
            .expect("the Glitch is in the enemy's hand")
            .id
            .clone();
        let play = legal_actions(&state, PlayerId::P2)
            .into_iter()
            .find(|body| matches!(body, ActionBody::Play { instance_id, .. } if *instance_id == glitch))
            .expect("R675: Glitch is always playable on its owner's turn");
        send(&mut state, &mut log, PlayerId::P2, play);
        let Some(result) = state.result else {
            continue;
        };
        assert_eq!(
            (result.winner, result.reason),
            (Winner::Draw, GameOverReason::Voided)
        );
        return ReplayInput {
            seed,
            decks,
            log,
            handicaps: Some(handicaps),
            sandbox: Some(true),
            ..Default::default()
        };
    }
    panic!("no seed's Glitch voids the fight");
}

/// SPEC §2.5's table of endings: "A Glitch voids the match | No result: the match never happened
/// (R679)". `crates/run/README.md`: `run_fight_report` refuses "a game with no result". §15.6 ends a
/// run on a fight's end that is no win; a fight that never happened is neither a win nor a loss, so
/// it must not be reported as a lost fight, which ends the whole run.
#[test]
fn a_fight_a_glitch_voided_is_not_reported_as_a_lost_fight() {
    let game = voided_fight();
    let report = run_fight_report(&game, PlayerId::P1);

    // What the run does with the report it gets today, for the failure message.
    let content = run_placeholder_content();
    let setup = RunSetup {
        seed: run_seed("audit-voided"),
        starter: content.starter_access[..RUN_STARTER_SIZE].to_vec(),
        artifacts: Vec::new(),
        curses: Vec::new(),
    };
    let fighting = run_reduce(
        &run_create(&setup, &RunUnlocks::default(), &content).unwrap(),
        &content,
        &RunAction::Go { col: 0 },
    )
    .unwrap();
    let after = report
        .as_ref()
        .ok()
        .and_then(|action| run_reduce(&fighting, &content, action).ok())
        .map(|run| format!("{:?} at {} HP", run.phase, run.hp));

    assert!(
        report.is_err(),
        "a voided fight has no result (§2.5, R679), yet it reports {report:?}, and the run is then {after:?}"
    );
}
