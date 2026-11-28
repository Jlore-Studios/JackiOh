//! #4595's sweep findings: each Infinite Assault card the sweep flagged as never played, put where its
//! text makes playing it clearly right. The AI plays it there at both budgets on two seeds, every
//! decision legal, none a fallback, no simulation panicking: piloted, and passed over in the sweep's
//! games for better plays (the report's "piloted; passed over").

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, is_legal, register_cards, scenario};

const DECK: [&str; 3] = ["core-008", "core-011", "core-020"];

/// #5246: the player's six cards.
const SIX: [&str; 6] = [
    "core-019", "core-019", "core-008", "core-011", "core-020", "core-008",
];

/// (id, why, the scenario's p1 and p2).
fn rows() -> Vec<(&'static str, &'static str, Value, Value)> {
    vec![
        (
            "infiniteassault-048",
            "Ego Death: one card in hand against six, so the swap gains five",
            json!({ "hand": [{ "def": "infiniteassault-048", "radiant": true }], "mana": 6, "library": DECK }),
            json!({ "hand": ["core-019", "core-019", "core-008", "core-011", "core-020", "core-008"], "library": DECK }),
        ),
        (
            "infiniteassault-072",
            "Overclock to Infinity: two (3) 9/9 Taunts in hand at 4 mana, both for (0), where one fits without it",
            json!({ "hand": [{ "def": "infiniteassault-072", "radiant": true }, "core-019", "core-019"], "mana": 4, "library": DECK }),
            json!({ "hand": ["core-011"], "library": DECK }),
        ),
        (
            "infiniteassault-123",
            "Cost Injection: 1 mana to spare and its only play, so the tax harms the enemy's next turn",
            json!({ "hand": [{ "def": "infiniteassault-123", "radiant": false }], "mana": 1, "library": DECK }),
            json!({ "hand": ["core-011"], "library": DECK }),
        ),
        (
            "infiniteassault-123",
            "Cost Injection (Radiant): 1 mana to spare and its only play, so the (2) tax harms the enemy's next turn",
            json!({ "hand": [{ "def": "infiniteassault-123", "radiant": true }], "mana": 1, "library": DECK }),
            json!({ "hand": ["core-011"], "library": DECK }),
        ),
        (
            "infiniteassault-211",
            "Grey Goo: an empty board, so no neighbour of the AI's turns into a copy",
            json!({ "hand": [{ "def": "infiniteassault-211", "radiant": true }], "mana": 3, "library": DECK }),
            json!({ "hand": ["core-011"], "field": ["core-008"], "library": DECK }),
        ),
        (
            "infiniteassault-137",
            "Patch Tuesday Exploit (#5246): 2 mana to spare and its only play, so its Nerfs on the player's six cards are its gain",
            json!({ "hand": [{ "def": "infiniteassault-137", "radiant": false }], "mana": 2, "library": DECK }),
            json!({ "hand": SIX, "library": DECK }),
        ),
        (
            "infiniteassault-137",
            "Patch Tuesday Exploit (Radiant) (#5246): 2 mana to spare and its only play, so its two Nerfs each on the player's six cards are its gain",
            json!({ "hand": [{ "def": "infiniteassault-137", "radiant": true }], "mana": 2, "library": DECK }),
            json!({ "hand": SIX, "library": DECK }),
        ),
        (
            "infiniteassault-130",
            "Prompt Injection (#5246): 2 mana to spare and its only play, so the misfire on the player's next Spell is its gain",
            json!({ "hand": [{ "def": "infiniteassault-130", "radiant": false }], "mana": 2, "library": DECK }),
            json!({ "hand": SIX, "library": DECK }),
        ),
        (
            "infiniteassault-130",
            "Prompt Injection (Radiant) (#5246): 2 mana to spare and its only play, so the misfires on the player's next two Spells are its gain",
            json!({ "hand": [{ "def": "infiniteassault-130", "radiant": true }], "mana": 2, "library": DECK }),
            json!({ "hand": SIX, "library": DECK }),
        ),
    ]
}

#[test]
fn assault_the_ai_plays_each_swept_card_in_its_clear_spot() {
    register_cards();
    for (id, why, p1, p2) in rows() {
        let s =
            scenario(json!({ "seed": "assault-clear-spot", "active": AI, "turn": 9, "p1": p1, "p2": p2 }));
        let card = s.card(id).id.clone();
        for budget in [AI_GATE_BUDGET, AI_BUDGET] {
            for seed in ["assault-a", "assault-b"] {
                let turn = play_ai_turn(s.state(), AI, &mut ai_options(seed, budget));
                let mut state = s.state().clone();
                for (decision, action) in turn.decisions.iter().zip(&turn.actions) {
                    assert!(
                        is_legal(&state, AI, &decision.action),
                        "{id}: an illegal decision"
                    );
                    assert_ne!(decision.reason, DecisionReason::Fallback, "{id}: a fallback");
                    assert_eq!(decision.stats.sim_panics, 0, "{id}: a simulation panicked");
                    state = super::support::act(&state, AI, &action.body);
                }
                assert!(
                    turn.decisions.iter().any(|decision| matches!(&decision.action,
                        ActionBody::Play { instance_id, .. } if *instance_id == card)),
                    "{id} ({why}) is not played at {budget:?} on {seed}"
                );
            }
        }
    }
}

#[test]
fn assault_cost_injection_yields_to_better_play() {
    register_cards();
    let s = scenario(json!({
        "seed": "assault-cost-injection-yields",
        "active": AI,
        "turn": 9,
        "p1": { "hand": ["infiniteassault-123", "core-008"], "mana": 1, "library": DECK },
        "p2": { "hand": ["core-011"], "library": DECK },
    }));
    let vanilla = s.card("core-008").id.clone();
    for budget in [AI_GATE_BUDGET, AI_BUDGET] {
        for seed in ["assault-a", "assault-b"] {
            let turn = play_ai_turn(s.state(), AI, &mut ai_options(seed, budget));
            assert!(
                turn.decisions.iter().any(|decision| matches!(&decision.action,
                    ActionBody::Play { instance_id, .. } if *instance_id == vanilla)),
                "core-008 is not played over Cost Injection at {budget:?} on {seed}"
            );
        }
    }
}

/// R5246000 (issue #5246): with a (2) 4/4 beside `id` at 2 mana and the player holding one card, the AI plays
/// the Unit: the Nerf or the misfire never outweighs a real play.
fn yields_to_a_2_mana_unit(id: &str) {
    register_cards();
    let s = scenario(json!({
        "seed": "r5246000-yields",
        "active": AI,
        "turn": 9,
        "p1": { "hand": [id, "stand-in (2) 4/4"], "mana": 2, "library": DECK },
        "p2": { "hand": ["core-011"], "library": DECK },
    }));
    let unit = s.card("stand-in (2) 4/4").id.clone();
    for budget in [AI_GATE_BUDGET, AI_BUDGET] {
        for seed in ["assault-a", "assault-b"] {
            let turn = play_ai_turn(s.state(), AI, &mut ai_options(seed, budget));
            assert!(
                turn.decisions.iter().any(|decision| matches!(&decision.action,
                    ActionBody::Play { instance_id, .. } if *instance_id == unit)),
                "the (2) 4/4 is not played over {id} at {budget:?} on {seed}"
            );
        }
    }
}

#[test]
fn r5246000_patch_tuesday_exploit_yields_to_better_play() {
    yields_to_a_2_mana_unit("infiniteassault-137");
}

#[test]
fn r5246000_prompt_injection_yields_to_better_play() {
    yields_to_a_2_mana_unit("infiniteassault-130");
}
