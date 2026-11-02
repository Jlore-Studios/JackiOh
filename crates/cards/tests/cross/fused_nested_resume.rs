//! R102, B3.4 rule 5: a fusion several levels deep names the ingredient whose text is running by a path
//! of indices from the outermost card in (`work.PART_KEY`), and `param` walks that path down the fused
//! ids. The path is built by the hooks of each level, so every level has to add its index.
//! Found by the fuzz gate (seed 598 of fuzz-handicap): a Final Gambit set under a card with no table
//! of steps (`Script.resume`) of its own read its numbers off the outer card's first ingredient. Fusion
//! ids stay short enough to spell out here, so the digest ids of the seed (R468) are not needed.
//! Test paths: SURFACE §4.1, §8.

use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::subsystems::fuse::FuseArgs;
use jackioh_engine::testkit::*;

const GAMBIT: &str = "classic-052"; // (2) Trap: a lethal hit is canceled; then heal {heal} 10 and draw {draw} 3 (resume step).
const INCOME_TAX: &str = "classic-009"; // (2) Trap with a table of steps of its own, and no "heal".
const COUNTERSPELL: &str = "classic-017"; // (2) Trap with no table of steps: the outer ingredient of the fusion.
const FILLER: &str = "core-005";
const ATTACKER: &str = "core-008"; // 4/4

use super::scenario;

fn craft(s: &mut Scenario, ingredients: [CardInstance; 2]) -> CardInstance {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
    let fused = subsystems::fuse::fuse(
        &mut sink,
        FuseArgs {
            ingredients: ingredients.to_vec(),
            to_hand: Some(P1),
            ..Default::default()
        },
    );
    match fused {
        Some(card) => card,
        None => panic!("the fusion did not happen"),
    }
}

fn lethal_against_nested() -> Scenario {
    let mut s = scenario(json!({
        // Arbitrary seed: no pool or random roll decides this test (audited under #848).
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
    fn r102_resumes_its_own_step_the_hit_is_canceled_the_hero_heals_10_and_draws_3_as_a_final_gambit_standing_alone_does()
     {
        let mut s = lethal_against_nested();

        s.expect_events(json!(["attackDeclared", "trapFired", "healed"]));
        assert!(
            !s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Redirected { .. } | GameEvent::Damage { .. }))
        );
        s.expect_health(P1, 14);
        s.expect_health(P2, 30);
        assert_eq!(s.hand(P1).len(), 4);
    }
}
