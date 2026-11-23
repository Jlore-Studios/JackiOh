//! ME-SECRET's AI half (Meditative MB05, R865; docs/meditative-set.md, group A's Systems,
//! ME-SECRET): `redact` drops the other seat's unrevealed choices and blanks the modes of their
//! in-flight play of a `secret_modes` card, and `determinize` samples each dropped choice, so the
//! AI never reads a secret it may not know. The search signs each secret of the state it searches,
//! a dropped choice as none, so the three choices of one cast are three positions to it (R3772000).

use indexmap::{IndexMap, IndexSet};
use jackioh_ai::{
    AI_BUDGET, AiOptions, DeterminizeOptions, Prices, beam_search, create_node_counter, decide, determinize,
    redact, search_signature,
};
use jackioh_engine::testkit::{
    ActionBody, GameState, PlayerId, Resume, SecretChoice, SecretRecord, Value, WorkItem, create_rng,
    hash_state, json, run_of,
};

use super::support::{AI, HUMAN, register_cards, scenario};

const MIND_GAMES: &str = "meditative-022";

/// p1 keeps Greed, p2 keeps Attack: one secret each, both unrevealed. p1 also holds a Stockpile, so
/// its turn does not end by itself once Mind Games resolves (R82) and `end_turn` ends p1's turn.
fn played() -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": "secret-ai",
        "p1": { "hand": [{ "def": MIND_GAMES }, "core-005"], "library": ["core-005", "core-005", "core-005"], "mana": 10 },
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

/// R3772000: the first mode an action declares (no card these boards hold has modes but Mind Games, so
/// the choice a play of it keeps), `None` for an action declaring none.
fn kept_mode(action: &ActionBody) -> Option<&str> {
    match action {
        ActionBody::Play {
            modes: Some(modes), ..
        } => modes.first().map(String::as_str),
        _ => None,
    }
}

/// R3772000: p1 (the AI) holds Mind Games and a Stockpile (so its turn stays open after the cast), with
/// 7 mana; p2 holds a full hand (`HAND_CAP`, R4), so the Fortify Mind the cast hands it burns, which is no
/// discard (R862), and the reply plays no guess: R3696001 draws one on each sampled world, which would
/// cancel the reward in some and bury the choices' difference in noise.
fn mind_games_board(p1_health: i32, p2_health: i32, p2_field: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": "r3772000",
        "p1": { "hand": [{ "def": MIND_GAMES }, "core-005"], "library": ["core-005", "core-005", "core-005"], "mana": 7, "health": p1_health },
        "p2": { "hand": vec!["core-005"; 10], "library": ["core-005", "core-005"], "health": p2_health, "field": p2_field },
    }))
    .state()
    .clone()
}

/// The enemy hero stands at 8, as far as Attack's 8 damage reaches; the AI has no other way to win.
fn within_attacks_reach() -> GameState {
    mind_games_board(30, 8, json!([]))
}

/// p1 stands at 7 against two units that swing 3 each and that Attack cannot kill (20 health), at a hero
/// Attack cannot reach (30), so only Defend's heal and Armor pay.
fn must_survive() -> GameState {
    let wall = |def: &str| json!({ "def": def, "statsOverride": { "attack": 3, "health": 20 } });
    mind_games_board(7, 30, json!([wall("core-008"), wall("core-011")]))
}

/// What `decide` makes of `state` on each of four seeds: the mode of the Mind Games it casts.
fn decided_modes(state: &GameState) -> Vec<Option<String>> {
    ["r3772000-a", "r3772000-b", "r3772000-c", "r3772000-d"]
        .into_iter()
        .map(|seed| {
            let mut options = AiOptions::with_budget(create_rng(seed, 0), AI_BUDGET);
            let decision = decide(state, AI, &mut options).expect("the AI acts");
            kept_mode(&decision.action).map(str::to_string)
        })
        .collect()
}

#[test]
fn r3772000_states_differing_only_in_the_opponents_unrevealed_choice_sign_alike_once_redacted() {
    let state = played();
    let greed = with_foe_choice(&state, SecretChoice::Greed);
    let defend = with_foe_choice(&state, SecretChoice::Defend);
    assert_ne!(search_signature(&greed, AI), search_signature(&defend, AI));
    assert_eq!(
        search_signature(&redact(&greed, AI), AI),
        search_signature(&redact(&defend, AI), AI)
    );
}

#[test]
fn r3772000_the_beam_keeps_all_three_mind_games_choices() {
    let lines = beam_search(
        &within_attacks_reach(),
        AI,
        &create_node_counter(AI_BUDGET.nodes, None),
        AI_BUDGET,
        Prices::NONE,
    );
    let mut firsts: Vec<&str> = lines
        .iter()
        .filter_map(|line| line.actions.first().and_then(kept_mode))
        .collect();
    firsts.sort_unstable();
    firsts.dedup();
    assert_eq!(firsts, ["attack", "defend", "greed"]);
}

#[test]
fn r3772000_decide_keeps_attack_when_its_damage_wins() {
    let modes = decided_modes(&within_attacks_reach());
    assert_eq!(modes, vec![Some("attack".to_string()); modes.len()]);
}

#[test]
fn r3772000_decide_keeps_defend_when_it_must_survive() {
    let modes = decided_modes(&must_survive());
    assert_eq!(modes, vec![Some("defend".to_string()); modes.len()]);
}
