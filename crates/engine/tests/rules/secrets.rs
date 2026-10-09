//! ME-SECRET's verbs (Meditative MB05, R860–R865; docs/meditative-set.md, group A's Systems,
//! ME-SECRET): `keep_secret` files the declared choice as a secret record, tells both players only
//! that a secret exists and arms its reward at the caster's next start of turn; `guess_secret`
//! judges a Fortify Mind's guess against the linked secret as rock, paper, scissors; and
//! `resolve_secret` resolves the secret away, revealing it once.

use jackioh_engine::effects::{guess_secret, keep_secret, resolve_secret};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::instance_data::instance_game;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

fn game(seed: &str) -> GameState {
    let mut state = instance_game(seed, None);
    state.turn = 5;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// The effect applied for p1 on a sink over `state`, the rng cursor written back; its events.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn keep(choice: &str) -> Effect {
    keep_secret(json_as(json!({ "choice": choice, "step": "reward" })))
}

fn js<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// The events of one action, as each player's view sends them.
fn sent(state: &mut GameState, events: Vec<GameEvent>, viewer: PlayerId) -> Value {
    state.applied = vec![AppliedAction {
        nonce: "t".to_string(),
        events,
    }];
    js(&view_for(state, viewer).events)
}

fn guess(secret_id: &str, guess: &str) -> Effect {
    guess_secret(json_as(json!({ "secretId": secret_id, "guess": guess })))
}

#[test]
fn r860_keep_secret_files_the_record_and_tells_only_that_it_exists() {
    let mut state = game("secret-r860");
    let events = run(&mut state, keep("attack"));
    let secrets = state.secrets.clone().expect("a secret is filed");
    assert_eq!(secrets.len(), 1);
    let secret = &secrets[0];
    assert_eq!(secret.owner, P1);
    assert_eq!(secret.choice, Some(SecretChoice::Attack));
    assert_eq!(secret.revealed, None);
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::SecretChosen { player, secret_id }
        if *player == P1 && *secret_id == secret.id
    )));
    // The opponent's view lists only the id: no choice on the entry, and none anywhere in the
    // events their view sends.
    let foe = view_for(&state, P2);
    let held = foe
        .opponent
        .secrets
        .clone()
        .expect("the opponent sees a secret is held");
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].id, secret.id);
    assert_eq!(held[0].choice, None);
    assert!(!sent(&mut state, events, P2).to_string().contains("choice"));
    // The owner's view carries the choice.
    let own = view_for(&state, P1);
    let mine = own.you.secrets.clone().expect("the owner reads their secret");
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].choice, Some(SecretChoice::Attack));
}

#[test]
fn r861_keep_secret_schedules_the_reward_at_the_owners_next_turn_start() {
    let mut state = game("secret-r861");
    run(&mut state, keep("greed"));
    let secret_id = kept_id(&state);
    assert_eq!(state.delayed.len(), 1);
    let entry = &state.delayed[0];
    assert_eq!(entry.at.phase, Phase::Start);
    assert_eq!(entry.at.player, P1);
    assert_eq!(entry.not_before, Some(state.turn + 1));
    assert_eq!(entry.resume.hook, RESUME_HOOK);
    assert_eq!(entry.resume.step, "reward");
    assert_eq!(entry.resume.data.get(SECRET_KEY), Some(&Value::String(secret_id)));
}

fn kept_id(state: &GameState) -> String {
    state
        .secrets
        .as_ref()
        .and_then(|secrets| secrets.first())
        .map(|secret| secret.id.clone())
        .expect("a secret is filed")
}

#[test]
fn r862_keep_secret_links_the_card_it_hands_the_enemy() {
    let mut state = game("secret-r862");
    let events = run(
        &mut state,
        keep_secret(json_as(json!({
            "choice": "defend",
            "step": "reward",
            "link": { "defId": plain.id.clone(), "player": "enemy", "costOverride": 0 },
        }))),
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, GameEvent::SecretChosen { player: P1, .. }))
    );
    let secret_id = kept_id(&state);
    let handed: Vec<CardInstance> = state.players[P2]
        .hand
        .iter()
        .filter(|card| card.def_id == plain.id.as_str())
        .cloned()
        .collect();
    assert_eq!(handed.len(), 1);
    assert_eq!(handed[0].memory.get(SECRET_KEY), Some(&Value::String(secret_id)));
    assert_eq!(handed[0].cost_override, Some(0));
}

#[test]
fn r864_judge_is_rock_paper_scissors() {
    use PredictOutcome::{Lost, Same, Won};
    use SecretChoice::{Attack, Defend, Greed};
    let cases = [
        ((Attack, Attack), Same),
        ((Greed, Greed), Same),
        ((Defend, Defend), Same),
        ((Attack, Greed), Won),
        ((Greed, Defend), Won),
        ((Defend, Attack), Won),
        ((Greed, Attack), Lost),
        ((Defend, Greed), Lost),
        ((Attack, Defend), Lost),
    ];
    for ((guess, choice), outcome) in cases {
        assert_eq!(judge(guess, choice), outcome, "{guess:?} against {choice:?}");
    }
}

#[test]
fn r864_a_winning_guess_cancels_the_reward_and_reveals() {
    let mut state = game("secret-r864-won");
    run(&mut state, keep("greed"));
    let secret_id = kept_id(&state);
    assert_eq!(state.delayed.len(), 1);
    let events = run(&mut state, guess(&secret_id, "attack"));
    // The reveal comes first, naming the owner and the choice; the prediction is public.
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        GameEvent::SecretRevealed { player, secret_id: id, choice }
        if *player == P1 && *id == secret_id && *choice == SecretChoice::Greed
    ));
    assert!(matches!(
        &events[1],
        GameEvent::Predicted { player, secret_id: id, guess, outcome }
        if *player == P1 && *id == secret_id && *guess == SecretChoice::Attack && *outcome == PredictOutcome::Won
    ));
    // The secret and its reward's delayed effect are gone.
    assert!(state.secrets.is_none());
    assert!(state.delayed.is_empty());
}

#[test]
fn r864_a_tie_or_a_loss_reveals_and_keeps_the_reward() {
    // A tie: the same choice reveals, and the reward still lands.
    let mut tied = game("secret-r864-same");
    run(&mut tied, keep("attack"));
    let secret_id = kept_id(&tied);
    let events = run(&mut tied, guess(&secret_id, "attack"));
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Predicted {
            outcome: PredictOutcome::Same,
            ..
        }
    )));
    let record = secret_of(&tied, &secret_id).expect("the secret stays");
    assert_eq!(record.revealed, Some(true));
    assert_eq!(tied.delayed.len(), 1);
    // Both players now read the choice.
    assert_eq!(
        view_for(&tied, P2).opponent.secrets.expect("revealed to both")[0].choice,
        Some(SecretChoice::Attack)
    );
    // A loss: Greed against Attack takes the penalty's side and still reveals.
    let mut lost = game("secret-r864-lost");
    run(&mut lost, keep("attack"));
    let secret_id = kept_id(&lost);
    let events = run(&mut lost, guess(&secret_id, "greed"));
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Predicted {
            outcome: PredictOutcome::Lost,
            ..
        }
    )));
    assert_eq!(
        secret_of(&lost, &secret_id).expect("the secret stays").revealed,
        Some(true)
    );
    assert_eq!(lost.delayed.len(), 1);
    assert_eq!(
        view_for(&lost, P1).you.secrets.expect("kept for the owner")[0].choice,
        Some(SecretChoice::Attack)
    );
}

#[test]
fn r862_a_guess_with_no_secret_does_nothing() {
    let mut state = game("secret-r862-missing");
    let before = state.clone();
    let events = run(&mut state, guess("secret-999", "attack"));
    assert!(events.is_empty());
    assert_eq!(state.secrets, None);
    assert_eq!(state.delayed, before.delayed);
}

#[test]
fn r861_resolve_secret_reveals_once_and_removes_the_record() {
    let mut state = game("secret-r861-resolve");
    run(&mut state, keep("defend"));
    let secret_id = kept_id(&state);
    let events = run(
        &mut state,
        resolve_secret(json_as(json!({ "secretId": secret_id }))),
    );
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0],
        GameEvent::SecretRevealed { player, secret_id: id, choice }
        if *player == P1 && *id == secret_id && *choice == SecretChoice::Defend
    ));
    assert!(state.secrets.is_none());
    // Resolving again reveals nothing and removes nothing.
    let again = run(
        &mut state,
        resolve_secret(json_as(json!({ "secretId": secret_id }))),
    );
    assert!(again.is_empty());
}
