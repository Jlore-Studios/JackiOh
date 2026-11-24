//! Audit (hosts): exploratory. The ladder bot's decision as `crates/shared/src/bots/decide.rs` makes
//! it (the skill roll, then `decide_styled` or the skill miss), replayed here over the AI crate's own
//! exports, since this crate cannot depend on `jackioh-shared`.

use jackioh_ai::{
    AiDeckOptions, AiOptions, AiStyle, SearchBudget, ai_to_act, build_ai_deck, decide_styled, plays_best,
    skill_miss_action, unanswered_draw_offer,
};
use jackioh_engine::testkit::{
    Action, ActionBody, CreateGameOptions, DECK_SIZE, GameState, PlayerId, Rng, begin_game, create_game,
    create_rng, legal_actions, reduce, seat_to_act,
};

fn register_cards() {
    jackioh_cards::register_all();
}

/// A ladder bot's deck as `bots::pool::deal_bot_deck` deals it: every set, no shadow ban, no gate.
fn bot_deck(seed: &str) -> Vec<String> {
    build_ai_deck(
        &mut create_rng(seed, 0),
        DECK_SIZE,
        &AiDeckOptions {
            banned: Some(Vec::new()),
            gated_sets: Some(Vec::new()),
            strength: Some(0.0),
            ..AiDeckOptions::default()
        },
    )
}

fn dealt(seed: &str) -> GameState {
    register_cards();
    begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: (bot_deck(&format!("{seed}:p1-deck")), bot_deck(&format!("{seed}:p2-deck"))),
        ..CreateGameOptions::default()
    }))
    .state
}

#[derive(Clone, Copy)]
struct Strength {
    budget: SearchBudget,
    temperature: f64,
    missed_lethal_chance: f64,
    skill: f64,
}

/// `bots::decide`'s move for a bot that does not pass for a person (no concession, no draw habits).
fn bot_move(state: &GameState, seat: PlayerId, strength: &Strength, rng: &mut Rng) -> Option<ActionBody> {
    if plays_best(strength.skill, rng) {
        let mut options = AiOptions {
            rng: rng.clone(),
            budget: strength.budget,
            should_stop: None,
            wrap_up: None,
        };
        let style = AiStyle {
            temperature: strength.temperature,
            missed_lethal_chance: strength.missed_lethal_chance,
        };
        let decision = decide_styled(state, seat, &mut options, &style);
        *rng = options.rng;
        decision.map(|decision| decision.action)
    } else if unanswered_draw_offer(state, seat) {
        Some(ActionBody::AnswerDraw { accept: false })
    } else {
        skill_miss_action(state, seat, rng)
    }
}

/// `host::bot::fallback_action`.
fn fallback(state: &GameState, seat: PlayerId) -> Option<ActionBody> {
    let legal: Vec<ActionBody> = legal_actions(state, seat)
        .into_iter()
        .filter(|body| {
            !matches!(
                body,
                ActionBody::Concede
                    | ActionBody::OfferDraw
                    | ActionBody::AnswerDraw { accept: true }
                    | ActionBody::Emote { .. }
            )
        })
        .collect();
    legal
        .iter()
        .find(|body| matches!(body, ActionBody::EndTurn))
        .or_else(|| legal.first())
        .cloned()
}

const WEAKEST: Strength = Strength {
    budget: SearchBudget {
        nodes: 150,
        lethal_nodes: 40,
        determinizations: 1,
        beam_width: 2,
        root_branching: 10,
        branching: 4,
        max_depth: 6,
        finalists: 3,
    },
    temperature: 3.0,
    missed_lethal_chance: 0.4,
    skill: 0.0,
};

#[test]
fn explore_miss_refusals() {
    let mut refusals = 0usize;
    let mut nothings = 0usize;
    let mut games = 0usize;
    let mut moves = 0usize;
    for n in 0..30 {
        let seed = format!("audit-hosts-{n}");
        let mut state = dealt(&seed);
        let mut rngs = [
            create_rng(&format!("{seed}:bot-ai:p1"), 0),
            create_rng(&format!("{seed}:bot-ai:p2"), 0),
        ];
        let mut step = 0usize;
        while state.result.is_none() && step < 1500 {
            let seat = seat_to_act(&state).unwrap_or(state.active);
            if !ai_to_act(&state, seat) {
                println!("seed {seed} step {step}: seat_to_act {seat} but !ai_to_act (turn {}, phase {:?})", state.turn, state.phase);
                break;
            }
            let rng = &mut rngs[if seat == PlayerId::P1 { 0 } else { 1 }];
            let chosen = bot_move(&state, seat, &WEAKEST, rng);
            let body = match chosen.clone() {
                Some(body) => body,
                None => {
                    nothings += 1;
                    println!("seed {seed} step {step} turn {}: the miss answered nothing for {seat}", state.turn);
                    match fallback(&state, seat) {
                        Some(body) => body,
                        None => break,
                    }
                }
            };
            let result = reduce(&state, &Action::new(body.clone(), seat, format!("a{step}")));
            if let Some(error) = result.error {
                refusals += 1;
                println!(
                    "seed {seed} step {step} turn {} seat {seat}: REFUSED {} -> {error}",
                    state.turn,
                    serde_json::to_string(&body).unwrap()
                );
                let Some(back) = fallback(&state, seat) else { break };
                let retry = reduce(&state, &Action::new(back, seat, format!("a{step}")));
                if retry.error.is_some() {
                    println!("  and the fallback was refused too");
                    break;
                }
                state = retry.state;
            } else {
                state = result.state;
            }
            step += 1;
            moves += 1;
        }
        games += 1;
        println!("seed {seed}: {step} steps, turn {}, result {:?}", state.turn, state.result.map(|r| r.reason));
    }
    println!("games {games}, moves {moves}, refusals {refusals}, nothings {nothings}");
}
