//! Audit, area `fuzz`. One finding: `redact` hands the search the true state's id counter
//! (`GameState.next_id`), which counts ids minted where the seat cannot see, and the search reads it
//! (a simulated prompt's id is `q{next_id}`, the seed of `auto_answers`' draw of four answers, and
//! every id a simulation mints starts from it). So two states the seat's view cannot tell apart give
//! two redactions and two decisions, against R185 (crates/ai/README.md: "`decide` ... reads the true
//! `GameState` only through `ai_to_act` and `redact` (R185): `redact` blanks what the seat may not know").
//!
//! The unseen ids here are S+ #17 Smoothie's: "When added to your hand, select two random fruits for
//! it" mints two ids as its owner draws it, and the other seat's view shows a hand one card larger and
//! nothing else.

use jackioh_ai::{
    AI_BUDGET, AiDeckOptions, AiOptions, MatchConfig, MatchHooks, MatchSetup, SearchBudget, SeatController,
    build_ai_deck, decide, play_match, redact,
};
use jackioh_engine::config::AI_DIFFICULTY;
use jackioh_engine::view_for::view_for;
use jackioh_engine::{
    Action, ActionBody, CardInstance, CreateGameOptions, GameState, Handicap, PerPlayer, PerPlayerOpt, Phase,
    PlayerId, Rng, SetName, begin_game, create_game, reduce, seat_to_act,
};
use serde_json::Value;

/// S+ #17 Smoothie (crates/cards/src/scripts/stinkyscape_plus/c017_smoothie.rs).
const SMOOTHIE: &str = "stinkyscapeplus-017";
/// The seed of the two-world game.
const SEED: &str = "audit-fuzz:smoothie:9";
/// The seat that decides: the Smoothie is the other seat's.
const SEAT: PlayerId = PlayerId::P2;

/// A deck of `handicap`'s size dealt from every card (no shadow ban, no gated set), as a player's
/// random deck is.
fn deck(seed: &str, seat: PlayerId, handicap: &Handicap, lean: Option<SetName>) -> Vec<String> {
    build_ai_deck(
        &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
        handicap.deck_size,
        &AiDeckOptions {
            mana_cap: Some(handicap.mana_cap),
            banned: Some(Vec::new()),
            gated_sets: Some(Vec::new()),
            lean_set: lean,
            ..AiDeckOptions::default()
        },
    )
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// Both seats keep their whole hand, then p1 ends its first turn: p2's first main phase.
fn to_p2s_first_turn(start: &GameState) -> GameState {
    let mut state = start.clone();
    for step in 0..6 {
        let seat = seat_to_act(&state).unwrap_or(state.active);
        let own_main = state.pending.is_none() && state.active == seat && state.phase == Phase::Main;
        if own_main && seat == PlayerId::P2 {
            return state;
        }
        let body = if own_main {
            ActionBody::EndTurn
        } else {
            ActionBody::Mulligan {
                keep: ids_of(&state.players[seat].hand),
            }
        };
        let result = reduce(&state, &Action::new(body, seat, format!("s{step}")));
        assert_eq!(result.error, None, "PREMISE: the opening steps are legal");
        state = result.state;
    }
    panic!("PREMISE: p2's first main phase comes within six actions");
}

/// Two games of `SEED` at p2's first main phase. They differ in one card of p1's deck, the one p1
/// drew on its first turn and still holds: the deck's own card in the first, a Smoothie in the second.
fn two_worlds() -> (GameState, GameState) {
    jackioh_cards::register_all();
    let easy = AI_DIFFICULTY.easy;
    let decks = (
        deck(SEED, PlayerId::P1, &easy, None),
        deck(SEED, PlayerId::P2, &easy, None),
    );
    assert!(
        !decks.0.iter().chain(decks.1.iter()).any(|id| id == SMOOTHIE),
        "PREMISE: neither dealt deck holds a Smoothie"
    );
    let options = |decks: (Vec<String>, Vec<String>)| CreateGameOptions {
        seed: SEED.to_string(),
        decks,
        ..CreateGameOptions::default()
    };
    let created = create_game(&options(decks.clone()));
    let dealt = begin_game(&created).state;
    let plain = to_p2s_first_turn(&dealt);
    // The card p1 drew since the deal. Ids and the shuffle are the seed's (R223), so the same deck
    // slot is drawn at the same moment whatever card it holds.
    let opening = ids_of(&dealt.players.p1.hand);
    let drawn = plain
        .players
        .p1
        .hand
        .iter()
        .find(|card| !opening.contains(&card.id))
        .expect("PREMISE: p1 drew a card on its first turn")
        .id
        .clone();
    let slot = created
        .players
        .p1
        .library
        .iter()
        .position(|card| card.id == drawn)
        .expect("the drawn card's deck slot");
    let mut other = decks;
    other.0[slot] = SMOOTHIE.to_string();
    let smoothie = to_p2s_first_turn(&begin_game(&create_game(&options(other))).state);
    // The premise of both tests: p2's window onto the two games (SPEC §10.8) is one and the same.
    assert_eq!(
        view_for(&plain, SEAT),
        view_for(&smoothie, SEAT),
        "PREMISE: the seat's view of the two worlds is the same"
    );
    (plain, smoothie)
}

/// The paths at which two JSON values differ.
fn diff(a: &Value, b: &Value, path: String, out: &mut Vec<String>) {
    match (a, b) {
        _ if a == b => {}
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let (p, q) = (x.get(key).unwrap_or(&Value::Null), y.get(key).unwrap_or(&Value::Null));
                diff(p, q, format!("{path}.{key}"), out);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (at, (p, q)) in x.iter().zip(y).enumerate() {
                diff(p, q, format!("{path}[{at}]"), out);
            }
        }
        _ => out.push(format!("{path}: {a} vs {b}")),
    }
}

/// R185: what the seat cannot see is not in its redaction. The two worlds differ only in what p1 drew,
/// and p2's view of them is the same, so `redact` must give one state. It gives `nextId` 54 and 56.
#[test]
fn r185_redact_is_the_same_for_two_worlds_the_seat_cannot_tell_apart() {
    let (plain, smoothie) = two_worlds();
    let mut differing = Vec::new();
    diff(
        &serde_json::to_value(redact(&plain, SEAT)).expect("a state serializes"),
        &serde_json::to_value(redact(&smoothie, SEAT)).expect("a state serializes"),
        String::new(),
        &mut differing,
    );
    assert_eq!(differing, Vec::<String>::new());
}

/// R185, README "Budgets" ("The same state, seed and budget always give the same `Decision`"): the
/// same stream and the shipped budget on the two worlds must give the same decision. The search walks
/// 424 nodes on one and 394 on the other.
#[test]
fn r185_decide_is_the_same_for_two_worlds_the_seat_cannot_tell_apart() {
    let (plain, smoothie) = two_worlds();
    let decided = |state: &GameState| {
        let mut options = AiOptions::with_budget(Rng::new(&format!("{SEED}:ctl:p2"), 0), AI_BUDGET);
        let decision = decide(state, SEAT, &mut options).expect("a decision");
        (decision.action, decision.stats, options.rng.cursor())
    };
    assert_eq!(decided(&plain), decided(&smoothie));
}

/// The same counter two ids on, as the Smoothie leaves it, changes the action itself. The state is
/// the 61st decision of a seeded game between two AIs (Hard at p2, both decks leaning on Meditative,
/// every card dealt) at an eighth of the shipped nodes; the only change made to it is `next_id`, the one
/// field the first test shows `redact` passing on.
#[test]
fn r185_two_ids_minted_unseen_do_not_change_the_action() {
    const SMALL: SearchBudget = SearchBudget {
        nodes: 150,
        lethal_nodes: 40,
        determinizations: 2,
        ..AI_BUDGET
    };
    jackioh_cards::register_all();
    let seed = "audit-fuzz:hard:4";
    let (easy, hard) = (AI_DIFFICULTY.easy, AI_DIFFICULTY.hard);
    let lean = Some(SetName::Meditative);
    let ai = SeatController::Ai {
        budget: Some(SMALL),
        skill: None,
    };
    let config = MatchConfig {
        seed: seed.to_string(),
        decks: (
            deck(seed, PlayerId::P1, &easy, lean),
            deck(seed, PlayerId::P2, &hard, lean),
        ),
        handicaps: Some(PerPlayerOpt {
            p1: Some(easy),
            p2: Some(hard),
        }),
        controllers: PerPlayer::new(ai, ai),
        max_actions: Some(61),
        setup: MatchSetup::default(),
    };
    // The state and the controller's stream before the game's last decision.
    let mut last: Option<(GameState, PlayerId, Rng)> = None;
    let record = {
        let mut hooks = MatchHooks {
            time_decision: Some(Box::new(|state, seat, rng, run| {
                last = Some((state.clone(), seat, rng.clone()));
                run();
            })),
            ..MatchHooks::default()
        };
        play_match(&config, &mut hooks)
    };
    assert_eq!(record.log.len(), 61, "PREMISE: the game reaches its 61st action");
    let (state, seat, rng) = last.expect("a decision");
    let decided = |state: &GameState| {
        let mut options = AiOptions::with_budget(rng.clone(), SMALL);
        decide(state, seat, &mut options).expect("a decision").action
    };
    let mut minted = state.clone();
    minted.next_id += 2;
    assert_eq!(decided(&state), decided(&minted));
}
