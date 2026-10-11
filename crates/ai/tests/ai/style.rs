//! `decide_styled` and its `AiStyle` (R1446): the server's ladder bots play below the AI's best by a
//! softmax temperature over the finalists and a chance to pass over lethal. The exact style
//! (`AI_STYLE_EXACT`) is `decide` itself, draw for draw, so nothing the browser or the gates play
//! moves.

use jackioh_ai::{
    AI_BUDGET, AI_STYLE_EXACT, AiOptions, AiStyle, DecisionReason, ai_to_act, decide, decide_styled,
    pick_finalist,
};
use jackioh_engine::testkit::{GameState, PLAYER_IDS, create_rng, json};

use super::support::{AI, act, random_policy_states, register_cards, scenario};

/// p1 (the AI) on turn 9 with Pointmaster 7/1 ready on the field; p2 at 5 health, no Taunt.
fn lethal_on_board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "style-lethal",
        "active": AI,
        "turn": 9,
        "p1": { "field": ["core-020"], "mana": 3, "library": ["core-008", "core-011"] },
        "p2": { "health": 5, "library": ["core-008", "core-011"] },
    }))
    .state()
    .clone()
}

#[test]
fn r1446_exact_style_decides_as_decide() {
    let mut compared = 0;
    for seed in ["style-exact-1", "style-exact-2"] {
        for state in random_policy_states(seed, 7, 140) {
            for seat in PLAYER_IDS {
                if !ai_to_act(&state, seat) {
                    continue;
                }
                let mut plain = AiOptions::new(create_rng(&format!("{seed}:ai"), 0));
                let mut styled = AiOptions::new(create_rng(&format!("{seed}:ai"), 0));
                let a = decide(&state, seat, &mut plain);
                let b = decide_styled(&state, seat, &mut styled, &AI_STYLE_EXACT);
                assert_eq!(a, b, "{seed} {seat}");
                assert_eq!(plain.rng.cursor(), styled.rng.cursor(), "{seed} {seat}");
                compared += 1;
            }
        }
    }
    assert!(compared > 10, "only {compared} decisions compared");
}

#[test]
fn r1446_pick_finalist_zero_temperature_takes_the_first_best_without_a_draw() {
    let mut rng = create_rng("style-pick-exact", 0);
    assert_eq!(pick_finalist(&[1.0, 3.0, 3.0, 2.0], 0.0, &mut rng), 1);
    assert_eq!(pick_finalist(&[4.0, 3.0], -1.0, &mut rng), 0);
    // One finalist is played whatever the temperature, with no draw.
    assert_eq!(pick_finalist(&[-2.0], 5.0, &mut rng), 0);
    assert_eq!(pick_finalist(&[], 5.0, &mut rng), 0);
    assert_eq!(rng.cursor(), 0);
}

#[test]
fn r1446_pick_finalist_spreads_with_temperature() {
    let means = [10.0, 9.0, 0.0];
    let mut rng = create_rng("style-pick-spread", 0);
    let mut picked = [0usize; 3];
    let draws = 2000;
    for _ in 0..draws {
        let at = pick_finalist(&means, 2.0, &mut rng);
        if let Some(count) = picked.get_mut(at) {
            *count += 1;
        }
    }
    // One draw per pick.
    assert_eq!(rng.cursor(), draws);
    // Weights 1, e^-0.5 ≈ 0.61, e^-5 ≈ 0.007: the best most, the near-best often, the far one rarely.
    let [best, near, far] = picked;
    assert!(best > near, "{picked:?}");
    assert!(near > draws as usize / 4, "{picked:?}");
    assert!(far < draws as usize / 50, "{picked:?}");

    // A cold style keeps to the best; a hot one plays them all about alike.
    let mut cold = [0usize; 3];
    let mut hot = [0usize; 3];
    for _ in 0..draws {
        if let Some(count) = cold.get_mut(pick_finalist(&means, 0.05, &mut rng)) {
            *count += 1;
        }
        if let Some(count) = hot.get_mut(pick_finalist(&means, 1000.0, &mut rng)) {
            *count += 1;
        }
    }
    assert_eq!(cold[0], draws as usize, "{cold:?}");
    assert!(hot.iter().all(|count| *count > draws as usize / 4), "{hot:?}");
}

#[test]
fn r1446_missed_lethal_one_passes_over_the_solver() {
    let state = lethal_on_board();
    let exact = decide(&state, AI, &mut AiOptions::new(create_rng("style-lethal:ai", 0)))
        .expect("the AI owes an action");
    assert_eq!(exact.reason, DecisionReason::Lethal);

    let mut plain = AiOptions::new(create_rng("style-lethal:ai", 0));
    decide(&state, AI, &mut plain);
    for temperature in [0.0, 3.0] {
        let blind = AiStyle {
            temperature,
            missed_lethal_chance: 1.0,
        };
        let mut options = AiOptions::with_budget(create_rng("style-lethal:ai", 0), AI_BUDGET);
        let missed = decide_styled(&state, AI, &mut options, &blind).expect("the AI owes an action");
        assert_ne!(missed.reason, DecisionReason::Lethal, "{temperature}");
        // The search plays on as if it had not seen the lethal: not the solver's attack, and not a
        // move that wins the game some other way.
        assert_ne!(missed.action, exact.action, "{temperature}");
        let after = act(&state, AI, &missed.action);
        assert!(after.result.is_none(), "{temperature}: {:?} won", missed.action);
        // The pass took one draw beyond the determinizations `decide` takes.
        assert!(options.rng.cursor() > plain.rng.cursor(), "{temperature}");
    }
}
