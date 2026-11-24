//! Audit of the opponent model (`crates/ai/src/reply.rs`): each test asserts the documented behaviour.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;

fn games(count: usize, tag: &str) -> Vec<(GameState, GameState, PlayerId, ActionBody)> {
    jackioh_cards::register_all();
    let mut out = Vec::new();
    for game in 0..count {
        let seed = format!("audit-reply:{tag}:{game}");
        let deck = |seat: PlayerId| {
            build_ai_deck(
                &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
                DECK_SIZE,
                &AiDeckOptions {
                    banned: Some(vec![]),
                    gated_sets: Some(vec![]),
                    ..AiDeckOptions::default()
                },
            )
        };
        let config = MatchConfig {
            seed: seed.clone(),
            decks: (deck(PlayerId::P1), deck(PlayerId::P2)),
            handicaps: None,
            controllers: PerPlayer {
                p1: SeatController::Random,
                p2: SeatController::Random,
            },
            max_actions: Some(600),
            setup: MatchSetup::default(),
        };
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(
                |before: &GameState, after: &GameState, seat: PlayerId, action: &ActionBody| {
                    out.push((before.clone(), after.clone(), seat, action.clone()));
                },
            )),
            ..MatchHooks::default()
        };
        play_match(&config, &mut hooks);
    }
    out
}

#[test]
fn fuzz_reply_score() {
    let steps = games(12, "fuzz");
    let (mut scored, mut midturn, mut none, mut panics, mut second) = (0, 0, 0, 0, 0);
    let mut max_used = 0;
    for (index, (before, after, seat, action)) in steps.iter().enumerate() {
        if !matches!(action, ActionBody::EndTurn) || after.result.is_some() {
            continue;
        }
        let seat = *seat;
        let hidden = hidden_card_ids(after, seat);
        let run = || {
            let counter = create_node_counter(400, None);
            let score = reply_score(after, seat, before.turn, &counter, &hidden, Prices::NONE);
            (score, counter.used(), counter.sim_panics(), counter.first_sim_panic())
        };
        let (score, used, sim_panics, first) = run();
        let again = run();
        assert_eq!((score, used), (again.0, again.1), "step {index}: not deterministic");
        assert!(used <= 400, "step {index}: {used} nodes");
        max_used = max_used.max(used);
        if sim_panics > 0 {
            panics += 1;
            println!("step {index}: sim panic {first:?}");
        }
        match score {
            Some(score) => {
                assert!(score.is_finite(), "step {index}: {score}");
                scored += 1;
            }
            None => none += 1,
        }
        if after.delayed.iter().any(has_countdown) {
            second += 1;
        }
        let counter = create_node_counter(400, None);
        if let Some(reply) = simulate_reply(after, seat, &counter, &hidden, Prices::NONE) {
            if reply.result.is_none() && reply.active != seat {
                midturn += 1;
                println!(
                    "step {index}: reply stopped mid-turn, used {}, pending {:?}",
                    counter.used(),
                    reply.pending.as_ref().map(|p| (p.player_id, p.kind))
                );
                let opp = seat.opponent();
                for (label, st) in [("before", after), ("after", &reply)] {
                    for who in [opp, seat] {
                        let units: Vec<String> = active_units_of(st, who)
                            .into_iter()
                            .map(|u| {
                                let v = unit_view(st, u);
                                format!("{}:{} {}/{} {:?} ex{:?} kw{:?}", u.id, u.def_id, v.attack, v.health, v.position, u.exertion, v.keywords)
                            })
                            .collect();
                        println!("  {label} {who}: hp {} mana {:?} units {units:#?}", st.players[who].hero.health, st.players[who].mana);
                    }
                }
                println!("  legal now: {}", legal_actions(&reply, opp).len());
                let known: Vec<&str> = after.applied.iter().map(|a| a.nonce.as_str()).collect();
                for entry in reply.applied.iter().filter(|a| !known.contains(&a.nonce.as_str())) {
                    let kinds: Vec<String> = entry.events.iter().map(|e| {
                        let text = format!("{e:?}");
                        text.chars().take(150).collect()
                    }).collect();
                    println!("  {} -> {kinds:#?}", entry.nonce);
                }
            }
        }
    }
    println!("scored {scored}, none {none}, midturn {midturn}, panics {panics}, countdowns {second}, max used {max_used}");
    let _ = (AI, HUMAN);
}
