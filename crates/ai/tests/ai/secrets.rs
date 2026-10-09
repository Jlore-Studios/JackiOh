//! ME-SECRET's AI half (Meditative MB05, R865; docs/meditative-set.md, group A's Systems,
//! ME-SECRET): `redact` drops the other seat's unrevealed choices and blanks the modes of their
//! in-flight play of a `secret_modes` card, and `determinize` samples each dropped choice, so the
//! AI never reads a secret it may not know.

use indexmap::{IndexMap, IndexSet};
use jackioh_ai::{DeterminizeOptions, determinize, redact};
use jackioh_engine::testkit::{
    GameState, PlayerId, Resume, SecretChoice, SecretRecord, WorkItem, create_rng, hash_state, json, run_of,
};

use super::support::{AI, HUMAN, register_cards, scenario};

const MIND_GAMES: &str = "meditative-022";

/// p1 keeps Greed, p2 keeps Attack: one secret each, both unrevealed.
fn played() -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": "secret-ai",
        "p1": { "hand": [{ "def": MIND_GAMES }], "library": ["core-005", "core-005", "core-005"], "mana": 10 },
        "p2": { "hand": [{ "def": MIND_GAMES }], "library": ["core-005", "core-005", "core-005"], "mana": 10 },
    }));
    s.play(MIND_GAMES, json!({ "modes": ["greed"] }));
    s.end_turn();
    s.play(MIND_GAMES, json!({ "modes": ["attack"] }));
    s.state().clone()
}

fn choice_of(state: &GameState, owner: PlayerId) -> Option<SecretChoice> {
    state
        .secrets
        .as_ref()
        .and_then(|secrets| secrets.iter().find(|secret| secret.owner == owner))
        .and_then(|secret| secret.choice)
}

fn with_foe_choice(state: &GameState, choice: SecretChoice) -> GameState {
    let mut next = state.clone();
    for secret in next.secrets.as_mut().expect("secrets").iter_mut() {
        if secret.owner == HUMAN {
            secret.choice = Some(choice);
        }
    }
    next
}

#[test]
fn r865_redact_drops_the_other_seats_choice_and_keeps_its_own() {
    let state = played();
    assert_eq!(choice_of(&state, AI), Some(SecretChoice::Greed));
    assert_eq!(choice_of(&state, HUMAN), Some(SecretChoice::Attack));
    let redacted = redact(&state, AI);
    assert_eq!(choice_of(&redacted, AI), Some(SecretChoice::Greed));
    assert_eq!(choice_of(&redacted, HUMAN), None);
    // A revealed choice is public history both seats watched happen, so it stays.
    let mut revealed = state.clone();
    for secret in revealed.secrets.as_mut().expect("secrets").iter_mut() {
        if secret.owner == HUMAN {
            secret.revealed = Some(true);
        }
    }
    assert_eq!(
        choice_of(&redact(&revealed, AI), HUMAN),
        Some(SecretChoice::Attack)
    );
}

#[test]
fn r865_states_differing_only_in_the_choice_redact_to_the_same_hash() {
    let state = played();
    let greed = redact(&with_foe_choice(&state, SecretChoice::Greed), AI);
    let defend = redact(&with_foe_choice(&state, SecretChoice::Defend), AI);
    assert_eq!(hash_state(&greed), hash_state(&defend));
}

#[test]
fn r865_determinize_samples_every_choice_over_seeds() {
    let public = redact(&played(), AI);
    assert_eq!(choice_of(&public, HUMAN), None);
    let mut seen: IndexSet<SecretChoice> = IndexSet::new();
    for n in 0..30 {
        let world = determinize(
            &public,
            AI,
            &mut create_rng(&format!("secret-ai:{n}"), 0),
            DeterminizeOptions::default(),
        );
        let choice = choice_of(&world, HUMAN).expect("a sampled choice");
        seen.insert(choice);
    }
    assert_eq!(seen.len(), SecretChoice::ALL.len());
}

#[test]
fn r865_redact_blanks_an_in_flight_secret_play() {
    register_cards();
    let mut state = scenario(json!({
        "seed": "secret-ai-play",
        "p1": { "hand": ["core-005"], "library": ["core-005"] },
        "p2": { "hand": ["core-005"], "library": ["core-005"] },
    }))
    .state()
    .clone();
    state.work.push(WorkItem {
        id: "w1".to_string(),
        seq: 1,
        owner: HUMAN,
        resume: Resume {
            def_id: MIND_GAMES.to_string(),
            hook: "play".to_string(),
            step: "resolve".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::from([(
                "__play".to_string(),
                json!({
                    "instanceId": "c1",
                    "player": "p2",
                    "defId": MIND_GAMES,
                    "modes": ["attack"],
                    "at": 3,
                }),
            )]),
        },
    });
    let redacted = redact(&state, AI);
    assert_eq!(redacted.work.len(), 1);
    let run = run_of(&redacted.work[0].resume).expect("a play run");
    assert!(run.modes.is_empty());
    // The seat's own in-flight play keeps its modes.
    let mut own = state.clone();
    if let Some(item) = own.work.first_mut() {
        item.resume.data.insert(
            "__play".to_string(),
            json!({
                "instanceId": "c1",
                "player": "p1",
                "defId": MIND_GAMES,
                "modes": ["greed"],
                "at": 3,
            }),
        );
    }
    let redacted = redact(&own, AI);
    let run = run_of(&redacted.work[0].resume).expect("a play run");
    assert_eq!(run.modes, vec!["greed".to_string()]);
}

/// `redact` drops the record's choice, never the record: determinize still has a secret to sample.
#[test]
fn r865_redact_keeps_the_record_it_empties() {
    register_cards();
    let mut state = played();
    state.secrets = Some(vec![SecretRecord {
        id: "secret-1".to_string(),
        owner: HUMAN,
        choice: Some(SecretChoice::Defend),
        revealed: None,
    }]);
    let redacted = redact(&state, AI);
    let secrets = redacted.secrets.expect("the record stays");
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].choice, None);
}
