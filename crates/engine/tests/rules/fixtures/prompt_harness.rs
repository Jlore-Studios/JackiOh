//! Helpers the prompts-and-movement tests share (docs/classic-sets.md B5 E13, E16–E18, E26). A sink
//! borrows the state, so `cast_now` and `answer_keys` return its events and write the rng's cursor
//! back to the state the caller passed in.

use jackioh_engine::testkit::*;

use super::catalog::vanilla_deck;
use super::harness::{act_logged, new_game, setup_catalog};
use super::prompts::register_prompt_fixtures;

/// p1's main phase on turn 3 with 4 mana, the fixtures registered (after `new_game`'s own).
pub fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_prompt_fixtures();
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state.players.p2.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

/// A card of `def_id` resolving for `player` (§10.5 step 4), where a Spell's Cry runs.
pub fn resolving_card(state: &mut GameState, def_id: &str, player: PlayerId, radiant: bool) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Resolving { player });
    card.radiant = radiant;
    state.players[player].resolving.push(card.clone());
    card
}

pub struct SinkResult {
    pub events: Vec<GameEvent>,
}

impl std::ops::Deref for SinkResult {
    type Target = Vec<GameEvent>;

    fn deref(&self) -> &Vec<GameEvent> {
        &self.events
    }
}

/// Run a resolving card's Cry the resumable way and settle, as the pipeline does.
pub fn cast_now(state: &mut GameState, def_id: &str, player: PlayerId, radiant: bool) -> SinkResult {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    let card = resolving_card(state, def_id, player, radiant);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        prompts::run_hook_resumable(
            &mut sink,
            &card,
            "cry",
            prompts::HookResumableOptions {
                controller: Some(player),
                ..Default::default()
            },
        );
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
    SinkResult { events }
}

/// The sink's events, and the refusal (`None` when accepted).
pub struct AnswerResult {
    pub events: Vec<GameEvent>,
    pub error: Option<String>,
}

/// Answer the open prompt by option key, as a client sends back what it was offered.
pub fn answer_keys(state: &mut GameState, keys: &[&str]) -> AnswerResult {
    let pending = must(state.pending.clone(), "an open prompt");
    let selection: Vec<Selection> = keys
        .iter()
        .map(|key| {
            must(
                pending.options.iter().find(|option| option.key == *key),
                &format!("option {key}"),
            )
            .selection
            .clone()
        })
        .collect();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    let error = {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let answered = prompts::answer_prompt(
            &mut sink,
            &prompts::AnswerInput {
                player_id: pending.player_id,
                choice_id: pending.id.clone(),
                selection,
            },
        );
        match answered {
            Ok(_) => {
                settle(&mut sink, SettleOptions::default());
                None
            }
            Err(error) => Some(error.message),
        }
    };
    state.rng_cursor = rng.cursor();
    AnswerResult { events, error }
}

pub fn open_as(state: &GameState, kind: PromptKind, player: PlayerId) -> PendingChoice {
    let pending = must(state.pending.clone(), &format!("a {kind} prompt"));
    assert_eq!(pending.kind, kind);
    assert_eq!(pending.player_id, player);
    pending
}

/// §9.3: a paused state survives JSON, and the copy answers exactly as the original does.
pub fn round_trip(state: &GameState) -> GameState {
    let text = serde_json::to_string(state).expect("a state serializes");
    serde_json::from_str(&text).expect("a state parses back")
}

pub fn event_types(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| event.event_type().as_str().to_string())
        .collect()
}

pub struct Replayable {
    pub state: GameState,
    pub log: Vec<Action>,
    pub decks: (Vec<String>, Vec<String>),
}

/// A replayable game (§9.2, §9.3), p1's first main phase: each deck holds the named Quickdraw cards
/// (R225), so the opening hands hold them whatever the shuffle, and both mulligans keep everything.
pub fn replayable(seed: &str, p1_cards: &[String], p2_cards: &[String]) -> Replayable {
    setup_catalog();
    register_prompt_fixtures();
    let mut first = vanilla_deck(DECK_SIZE - p1_cards.len() as i32, 1);
    first.extend(p1_cards.iter().cloned());
    let mut second = vanilla_deck(DECK_SIZE - p2_cards.len() as i32, 21);
    second.extend(p2_cards.iter().cloned());
    let decks = (first, second);
    let mut log: Vec<Action> = Vec::new();
    let mut state = begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.clone(),
        ..CreateGameOptions::default()
    }))
    .state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act_logged(
        &state,
        json!({ "type": "mulligan", "playerId": "p1", "keep": keep }),
        &mut log,
    )
    .state;
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act_logged(
        &state,
        json!({ "type": "mulligan", "playerId": "p2", "keep": keep }),
        &mut log,
    )
    .state;
    Replayable { state, log, decks }
}

/// §9.2: fold the log from scratch and compare hashes with the live state.
pub fn expect_replays(seed: &str, decks: &(Vec<String>, Vec<String>), log: &[Action], live: &GameState) {
    let folded = fold(&json_as::<FoldArgs>(
        json!({ "seed": seed, "decks": decks, "log": log }),
    ));
    assert!(folded.errors.is_empty(), "the fold refused nothing");
    assert_eq!(hash_state(&folded.state), hash_state(live));
}

pub fn hand_card(state: &GameState, player: PlayerId, def_id: &str) -> CardInstance {
    must(
        state.players[player]
            .hand
            .iter()
            .find(|card| card.def_id == def_id),
        &format!("{def_id} in {player}'s hand"),
    )
    .clone()
}
