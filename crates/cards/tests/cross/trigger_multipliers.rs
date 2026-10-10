//! The Meditative batch 03 combo (issue #519, R821, R823): Fear Mongerer played under Joint Filing
//! and Double Counting, with Gatling Pea on the field. Double Counting runs the Cry twice; each
//! round Joint Filing doubles the Pea's hook, so one play fires the Pea four times, and the real
//! end of turn fires it twice more.

use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::scenario;

const FILING: &str = "meditative-009"; // Joint Filing: each end-of-turn hook runs twice.
const COUNTING: &str = "meditative-010"; // Double Counting: each Cry runs twice.
const FEAR: &str = "meditative-012"; // Fear Mongerer: Cry, trigger your End of turn effects.
const PEA: &str = "meditative-013"; // Gatling Pea: end of turn, 1 to the enemy hero, then grows.
const SPARE: &str = "core-010"; // (0) Spell, never played: it only keeps the turn open (R82).
const VANILLA: &str = "core-008"; // a (1) 4/4 with no text.

/// The Pea's declared damage as its tuning leaves it (R386).
fn damage(s: &Scenario) -> i32 {
    let instance = s.card(PEA).clone();
    jackioh_engine::prelude::param_value(
        s.state(),
        Some(&instance),
        "damage",
        jackioh_engine::prelude::ParamValueOptions::default(),
    )
}

#[test]
fn r821_r823_fear_mongerer_under_joint_filing_and_double_counting_fires_gatling_pea_four_times() {
    let mut s = scenario(json!({
        "p1": {
            "mana": 10,
            "backrow": [FILING, COUNTING],
            "field": [PEA],
            "hand": [FEAR, SPARE],
            "library": [VANILLA, VANILLA, VANILLA],
        },
        "p2": {
            "mana": 10,
            "hand": [SPARE],
            "library": [VANILLA, VANILLA, VANILLA],
        },
    }));
    s.play(FEAR, json!({}));
    // The Cry runs twice (R823); each round the Pea's hook runs twice (R821): 1 + 2 + 3 + 4.
    // The turn goes on (R824).
    assert_eq!(s.state().active, P1);
    s.expect_health(P2, 20);
    assert_eq!(damage(&s), 5);
    // The real end of turn runs the Pea's hook twice more: 5 + 6.
    s.end_turn();
    s.expect_health(P2, 9);
    assert_eq!(damage(&s), 7);
}
