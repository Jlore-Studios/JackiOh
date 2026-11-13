//! Issue #3575, fullsend 3: what a play leaves in the seat's own library. R3575100: the learned network
//! reads each library without the Created cards a shuffle put in (`fitted_view`), since in the games it
//! was fitted on a library grew only by the deal and it read the count as the game's stage: Forced
//! Propagation's (S+ #24) ten copies lifted a finalist by about nine points. R3575101: the hand eval
//! prices the seat's own next `library_draws` draws by its library's mean hand price over an unseen
//! card's (`library_draws_worth`), so copies of a dear card shuffled in count and a tutor that takes one
//! out costs.
//!
//! Every board is a `scenario()` with the AI as p1 (support's header says why).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, close};

/// A 1-cost 1/1.
const ONE: &str = "stand-in (1) 1/1";
/// A 6-cost Spell with no text.
const SIX: &str = "stand-in (6) Spell";
/// S+ #24 Forced Propagation (Spell, 1; Embiggen 3): shuffle 10 copies (40 Embiggened) of a card.
const PROPAGATION: &str = "stinkyscapeplus-024";
/// S+ #68 Shadiris Visions (Spell, 1): Discover a Spell in your deck, add a copy of it to your hand.
const VISIONS: &str = "stinkyscapeplus-068";
/// A real card the network has rows for (Core's vanilla 2-drop), for the library.
const VANILLA: &str = "core-008";

fn board(seed: &str, hand: &[&str], mana: i32, library: &[&str]) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": seed,
        "active": "p1",
        "turn": 9,
        "p1": { "hand": hand, "mana": mana, "library": library, "field": [VANILLA] },
        "p2": { "hand": [ONE], "library": [VANILLA, VANILLA, VANILLA, VANILLA], "field": [VANILLA] },
    }))
    .state()
    .clone()
}

/// `state` with `count` Created copies of the seat's first library card at the bottom of `player`'s library.
fn with_created(state: &GameState, player: PlayerId, count: usize, created: bool) -> GameState {
    let mut s = state.clone();
    let model = s.players[player].library[0].clone();
    for n in 0..count {
        let mut card = model.clone();
        card.id = format!("made-{n}");
        card.created = created.then_some(true);
        s.players[player].library.push(card);
    }
    s
}

fn scored(state: &GameState, kind: EvalKind) -> f64 {
    finalist_score_with(state, AI, NextSwing::Enemy, kind, &LEARNED, Prices::NONE)
}

/// What the network adds to the hand eval at `state`.
fn correction(state: &GameState) -> f64 {
    scored(state, EvalKind::Learned) - scored(state, EvalKind::Hand)
}

#[test]
fn r3575100_created_library_cards_move_no_correction() {
    let library = [VANILLA; 12];
    let s = board("r3575100", &[ONE], 3, &library);
    let base = correction(&s);
    for count in [1, 10, 40] {
        close(correction(&with_created(&s, AI, count, true)), base);
    }
    // The view it reads is the redacted one less those cards, and nothing else.
    let made = with_created(&s, AI, 10, true);
    let view = fitted_view(&made, AI);
    assert_eq!(view.players[AI].library.len(), 12);
    assert_eq!(fitted_view(&s, AI), redact(&s, AI));
}

#[test]
fn r3575100_dealt_library_cards_still_move_it() {
    // The network reads a dealt library as it always has: only the Created cards are left out, so the
    // game's stage it reads from a library the deal left is unchanged.
    let s = board("r3575100-dealt", &[ONE], 3, &[VANILLA; 12]);
    let grown = with_created(&s, AI, 10, false);
    assert_eq!(fitted_view(&grown, AI).players[AI].library.len(), 22);
    assert!(
        (correction(&grown) - correction(&s)).abs() > 1e-9,
        "the shipped network reads a dealt library's size"
    );
}

#[test]
fn r3575101_library_draws_price_the_library_over_unseen_cards() {
    let w = AI_EVAL;
    let unseen = w.hand_card + w.hand_per_cost * w.opponent_hand_cost;
    let score = |library: &[&str]| evaluate(&board("r3575101", &[ONE], 0, library), AI, NextSwing::Enemy, &w);
    let count_only = |n: usize| w.library_card * (n as f64).min(w.library_comfort);
    // Ten 6-cost Spells against ten 1/1s: every one of the next draws differs by the hand price.
    let dear = score(&[SIX; 10]);
    let cheap = score(&[ONE; 10]);
    let per_card = w.hand_per_cost * (6.0f64.min(w.hand_cost_cap) - 1.0);
    close(dear - cheap, w.library_draws.min(10.0) * per_card);
    // A library shorter than `library_draws` counts every card once.
    assert!(w.library_draws > 2.0);
    let short = score(&[SIX; 2]) - score(&[ONE; 2]);
    close(short, 2.0 * per_card);
    // Each draw is counted over an unseen card's price, so the count alone stays `library_card`'s.
    let six = w.hand_card + w.hand_per_cost * 6.0f64.min(w.hand_cost_cap);
    let empty = score(&[]);
    close(score(&[SIX; 2]) - empty, count_only(2) + 2.0 * (six - unseen));
    close(
        score(&[SIX; 6]) - empty,
        count_only(6) + w.library_draws.min(6.0) * (six - unseen),
    );
    // The greedy baseline reads none of it.
    assert_eq!(GREEDY_EVAL.library_draws, 0.0);
}

#[test]
fn r3575101_the_enemy_library_never_moves_the_score() {
    let s = board("r3575101-enemy", &[ONE], 0, &[ONE; 6]);
    let mut dear = s.clone();
    for card in dear.players[HUMAN].library.iter_mut() {
        card.def_id = SIX.to_string();
    }
    close(
        evaluate(&dear, AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&s, AI, NextSwing::Enemy, &AI_EVAL),
    );
}

/// The AI's turn from `state`: `decide` and the reducer until the turn is no longer its own.
fn ai_turn(state: &GameState, seed: &str) -> Vec<ActionBody> {
    let mut options = AiOptions::new(create_rng(seed, 0));
    let mut current = state.clone();
    let mut actions = Vec::new();
    while current.active == AI && current.result.is_none() && actions.len() < 12 {
        let decision = decide(&current, AI, &mut options).expect("the AI decides");
        current = act(&current, AI, &decision.action);
        actions.push(decision.action);
    }
    actions
}

fn plays(state: &GameState, actions: &[ActionBody], def: &str) -> Option<ActionBody> {
    let ids: Vec<String> = state.players[AI]
        .hand
        .iter()
        .filter(|card| card.def_id == def)
        .map(|card| card.id.clone())
        .collect();
    actions
        .iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if ids.contains(instance_id)))
        .cloned()
}

#[test]
fn r3575101_forced_propagation_copies_a_dear_card_into_a_cheap_library_with_its_spare_crystal() {
    // Ten 1/1s left to draw, a 6-cost Spell in hand it cannot cast yet, and one crystal.
    let s = board("r3575101-propagation", &[PROPAGATION, SIX], 1, &[ONE; 10]);
    let six = s.players[AI].hand[1].id.clone();
    let actions = ai_turn(&s, "r3575101-propagation");
    let play = plays(&s, &actions, PROPAGATION).expect("it casts Forced Propagation");
    let ActionBody::Play { targets, .. } = play else {
        unreachable!()
    };
    assert_eq!(
        targets,
        Some(vec![Selection::Instance { instance_id: six }]),
        "on the dear card"
    );
}

#[test]
fn r3575101_forced_propagation_is_kept_when_the_library_is_already_dear() {
    // Ten 6-cost Spells left to draw, and only a 1/1 in hand and a 2-drop on the field to copy: ten copies
    // of either would only thin the draws.
    let s = board("r3575101-keep", &[PROPAGATION, ONE], 1, &[SIX; 10]);
    let actions = ai_turn(&s, "r3575101-keep");
    assert_eq!(plays(&s, &actions, PROPAGATION), None, "{actions:?}");
}

/// A guard, not a proof: Shadiris Visions keeps the library as it was, so neither ruling moves it, and the
/// AI casts it with a crystal it has no other use for at either weight.
#[test]
fn shadiris_visions_is_cast_with_a_spare_crystal() {
    let s = board("r3575101-visions", &[VISIONS], 1, &[SIX, SIX, SIX, ONE, ONE, ONE]);
    let actions = ai_turn(&s, "r3575101-visions");
    assert!(plays(&s, &actions, VISIONS).is_some(), "{actions:?}");
}
