//! A steal with no room is removal (R4205000): stealing a card when the thief's side has no room for it
//! destroys it, past Indestructible, so with its unit row full the AI reads Core #49 Snom Bunny Mind
//! Control on the opponent's Core #66 The Rock (Indestructible, 10/10) as the Rock's removal and casts it,
//! where before issue #4205 the steal did nothing and the card stayed in hand. The search scores the play
//! through the engine, so no AI code reads the rule. The AI's five Mr. Vanillas are summoning sick, so no
//! attack of theirs can open a zone first.

use jackioh_ai::{AiOptions, decide};
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const MIND_CONTROL: &str = "core-049";
const ROCK: &str = "core-066";
const VANILLA: &str = "core-008";
const STOCKPILE: &str = "core-005";
/// Mind Control's cost since issue #3139.
const COST: i32 = 5;

#[test]
fn r4205000_with_its_unit_row_full_the_ai_plays_mind_control_on_the_indestructible_rock_as_removal() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "ai-steal-no-room",
        "p1": {
            "hand": [MIND_CONTROL],
            "mana": COST,
            "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
            "library": [STOCKPILE, STOCKPILE],
        },
        "p2": { "field": [{ "def": ROCK, "lane": 2 }], "library": [STOCKPILE, STOCKPILE] },
    }));
    let turn = s.state().turn;
    for lane in 1..=5 {
        let id = s.unit_at(AI, lane).id;
        s.card_mut(id.as_str()).summoned_turn = Some(turn);
    }
    let rock = s.unit_at(HUMAN, 2).id;
    let card = s
        .hand(AI)
        .into_iter()
        .find(|card| card.def_id == MIND_CONTROL)
        .expect("setup: the AI holds Mind Control")
        .id;

    let decision = decide(
        s.state(),
        AI,
        &mut AiOptions::new(create_rng("ai-steal-no-room", 0)),
    )
    .expect("the AI has a move");

    assert!(
        decision.line.iter().any(|action| matches!(
            action,
            ActionBody::Play { instance_id, targets: Some(targets), .. }
                if *instance_id == card && pick::instance_ids(targets).contains(&rock)
        )),
        "the AI's line casts Mind Control on the Rock: {:?}",
        decision.line
    );
}
