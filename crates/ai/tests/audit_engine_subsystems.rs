//! Audit: the engine's special-purpose state machines (Fuse, Activate, Starships, Dormant, Evolve,
//! Quests, the hero power, the crafter, ...). Each test asserts the ruled or printed behaviour and fails
//! on the bug it proves.

#![allow(unused_imports, dead_code)]

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const TRUE_CRAFT: &str = "meditative-017"; // True Craft a Card: asks a cost, then a `craft` prompt

/// A Unit recipe with every point in stats and no block the prompt's offer could refuse (R1563007).
fn unit_recipe(cost: i32, attack: i64, health: i64) -> Value {
    json!({
        "cost": cost,
        "type": "Unit",
        "adjective": "Pure",
        "noun": "Closure",
        "attack": attack,
        "health": health,
        "keywords": [],
        "echo": 0,
        "hats": [],
    })
}

/// R880 (`subsystems/craft.rs`, `validate_recipe`): "it costs {points} points, over the budget", and
/// §9.3: `reduce` refuses an illegal action itself, the state unchanged. A `craft` prompt's answer is a
/// recipe the client writes (`Selection::Craft`), and its Unit stats have no upper bound but the points
/// budget, which `recipe_points` computes as `attack + health - 1` in `i32`. Two huge stats overflow
/// that sum: this build panics inside `reduce`; the shipped builds wrap it to a negative number, which
/// is under every budget, so the recipe is accepted and a 2147483647/2147483647 Unit is minted for (0).
#[test]
fn r880_a_crafted_units_stats_past_the_budget_are_refused_however_large() {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "seed": "audit-craft-overflow",
        "p1": { "hand": [TRUE_CRAFT], "mana": 10 },
        "p2": { "hand": ["core-053"], "library": ["core-001", "core-002"] },
    }));
    s.play(TRUE_CRAFT, json!({}));
    s.answer(json!(["0"]));
    assert_eq!(
        s.state().pending.as_ref().map(|pending| pending.kind),
        Some(PromptKind::Craft)
    );
    // The control: an ordinary over-budget Unit is refused in the validator's words.
    s.expect_refused_with(
        |s| s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(0, 10, 10) }])),
        "over the budget",
    );
    // The same refusal is owed to any stats past the budget.
    let max = i64::from(i32::MAX);
    s.expect_refused_with(
        |s| s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(0, max, max) }])),
        "over the budget",
    );
}
