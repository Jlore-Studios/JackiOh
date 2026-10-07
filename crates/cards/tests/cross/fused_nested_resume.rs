//! R102, B3.4 rule 5: a fusion several levels deep names the ingredient whose text is running by a path
//! of indices from the outermost card in (`work.PART_KEY`), and `param` walks that path down the fused
//! ids. The path is built by the hooks of each level, so every level has to add its index.
//!
//! Found by the fuzz gate (seed 598 of fuzz-handicap, once the pool changed by a card): a Final Gambit
//! four fusions down resumed its step ("heal your hero, draw") against the wrong ingredient and threw
//! `declares no number "heal"`. A table of steps (`Script.resume`) held by only one ingredient of a
//! fusion was passed up as it stood, so no hook at that level wrapped its steps; the one that did sat
//! at the first level with two such tables, and its path began there. A Final Gambit set under a card
//! with no table of its own read its numbers off the outer card's first ingredient. Fusion ids stay
//! short enough to spell out here, so the digest ids of the seed (R468) are not needed to show it.
//!
//! Port of `packages/cards/test/fused-nested-resume.test.ts` (SURFACE §4.1, §8).

use jackioh_cards::register_all;
use jackioh_engine::subsystems::fuse::FuseArgs;
use jackioh_engine::testkit::*;
use jackioh_engine::PlayerId::{P1, P2};

const GAMBIT: &str = "classic-052"; // (2) Trap: a lethal hit is re-aimed; then heal {heal} 10 and draw {draw} 3 (resume step).
const INCOME_TAX: &str = "classic-009"; // (2) Trap with a table of steps of its own, and no "heal".
const COUNTERSPELL: &str = "classic-017"; // (2) Trap with no table of steps: the outer ingredient of the fusion.
const FILLER: &str = "core-005";
const ATTACKER: &str = "core-008"; // 4/4

/// `scenario(...)` over the real cards: what importing the TS harness registered (`registerAll()`).
fn scenario(setup: Value) -> Scenario {
    register_all();
    jackioh_engine::testkit::scenario(setup)
}

/// TS `craft(s, ingredients)`: `subsystems.fuse` over a sink on the scenario's state, with an event
/// list of its own and an rng at the state's cursor that nothing writes back.
fn craft(s: &mut Scenario, ingredients: [CardInstance; 2]) -> CardInstance {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
    let fused = subsystems::fuse::fuse(
        &mut sink,
        FuseArgs { ingredients: ingredients.to_vec(), to_hand: Some(P1), ..Default::default() },
    );
    match fused {
        Some(card) => card,
        None => panic!("the fusion did not happen"),
    }
}

/// p1 sets `Counterspell + (Final Gambit + Income Tax)` face-down at 4 health; p2 attacks its hero for 4.
fn lethal_against_nested() -> Scenario {
    let mut s = scenario(json!({
        "seed": "fused-nested-resume",
        "p1": { "hand": [GAMBIT, INCOME_TAX, COUNTERSPELL, FILLER], "library": [FILLER, FILLER, FILLER, FILLER, FILLER], "health": 4, "mana": 10 },
        "p2": { "hand": [FILLER], "field": [ATTACKER], "library": [FILLER, FILLER, FILLER, FILLER, FILLER] },
    }));
    let gambit = s.card(GAMBIT).clone();
    let income_tax = s.card(INCOME_TAX).clone();
    let inner = craft(&mut s, [gambit, income_tax]);
    let counterspell = s.card(COUNTERSPELL).clone();
    let outer = craft(&mut s, [counterspell, inner]);
    s.play(&outer.id, json!({}));
    s.end_turn();
    s.attack(ATTACKER, "hero");
    s
}

mod r102_a_final_gambit_under_a_fusion_whose_other_ingredients_hold_no_table_of_steps {
    use super::*;

    #[test]
    fn r102_resumes_its_own_step_the_hit_is_re_aimed_the_hero_heals_10_and_draws_3_as_a_final_gambit_standing_alone_does() {
        let s = lethal_against_nested();

        s.expect_events(json!(["attackDeclared", "trapFired", "redirected", "damage", "healed"]));
        s.expect_health(P1, 14);
        s.expect_health(P2, 26);
        assert_eq!(s.hand(P1).len(), 4);
    }
}
