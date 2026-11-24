//! Audit of the opponent model (`crates/ai/src/reply.rs`): each test asserts the documented behaviour
//! and fails on the code as it stands.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;
const TURN: i32 = 9;
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];
/// Mr. Vanilla, a 4/4 with no text.
const VANILLA: &str = "core-008";
/// Zoomerbin Oomen, a 1/2.
const OOMEN: &str = "core-067";
/// Core #96 My Pawn, the Trap `AI_SEARCH.ai_turn_trap_ids` names.
const MY_PAWN: &str = "core-096";
/// Meditative #86 Mayor Medinamogger, a 5/4: "Aura: All targets are chosen at random."
const MAYOR: &str = "meditative-086";
/// Stinkyscape #66 Miner v5, a 0/8: "Activate 5: Gain a random Stinkyscape Resource" (five uses a turn).
const MINER: &str = "stinkyscape-066";

/// p1's turn 9 ended through the reducer: p2's turn 10, where `simulate_reply` plays p2's reply.
fn handed_over(p1: Value, p2: Value) -> GameState {
    handed_over_on("audit-reply", p1, p2)
}

fn handed_over_on(seed: &str, p1: Value, p2: Value) -> GameState {
    jackioh_cards::register_all();
    let mut p1 = p1;
    let mut p2 = p2;
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    let s = scenario(json!({ "seed": seed, "active": "p1", "turn": TURN, "p1": p1, "p2": p2 }));
    let ended = reduce(s.state(), &Action::new(ActionBody::EndTurn, AI, "audit-end".to_string()));
    assert!(ended.error.is_none(), "{:?}", ended.error);
    assert_eq!(ended.state.active, HUMAN);
    ended.state
}

fn on_field(state: &GameState, player: PlayerId, def_id: &str) -> bool {
    active_units_of(state, player)
        .into_iter()
        .any(|unit| unit.def_id == def_id)
}

/// reply.rs `reduce_for` (lines 368-373) answers `None` for an attack whose surcharge
/// (`surcharge_nodes`: 40 nodes for any attack of a unit whose attack reaches the seat's life while a My
/// Pawn lies in the seat's backrow) does not fit the counter, and `simulate_reply_with` (lines 986-997)
/// reads that `None` as a refused attack and ends the opponent's turn. So with fewer than 40 nodes left
/// the reply is `Some` state in which the opponent made no attack at all, where `simulate_reply`'s
/// contract is "Returns `None` when the node counter exhausts" (reply.rs line 888) and the reply's rule
/// is to make every attack worth making (README, "then its attacks").
#[test]
fn a_reply_short_of_an_attacks_surcharge_is_cut_not_played_as_a_passed_turn() {
    // The AI at 4 behind a 1/2 in Defense Position (Taunt, so no hero attack is legal and My Pawn can
    // answer nothing); the opponent's 4/4 kills the 1/2 and survives.
    let board = |backrow: Value| {
        handed_over(
            json!({ "health": 4, "field": [{ "def": OOMEN, "position": "DEF" }], "backrow": backrow }),
            json!({ "field": [VANILLA] }),
        )
    };
    let reply = |state: &GameState, nodes: usize| {
        let counter = create_node_counter(nodes, None);
        simulate_reply(state, AI, &counter, &hidden_card_ids(state, AI), Prices::NONE)
    };
    // Without the Trap, 30 nodes are plenty: the 4/4 kills the Taunt.
    let plain = board(json!([]));
    let ample = reply(&plain, 30).expect("a reply");
    assert!(!on_field(&ample, AI, OOMEN), "the 4/4 kills the 1/2");
    // With the Trap and a whole budget, the same attack is made.
    let trapped = board(json!([MY_PAWN]));
    let whole = reply(&trapped, 400).expect("a reply");
    assert!(!on_field(&whole, AI, OOMEN), "the 4/4 kills the 1/2 beside a My Pawn too");
    // With the Trap and 30 nodes, the attack's surcharge does not fit: the reply must say the counter ran
    // out, never hand back a turn in which the opponent attacked nothing.
    if let Some(short) = reply(&trapped, 30) {
        assert!(
            !on_field(&short, AI, OOMEN),
            "a reply cut by the counter came back as a whole reply in which the opponent passed: \
             active {:?}, turn {}",
            short.active,
            short.turn
        );
    }
}

/// README ("`decide` runs a bounded lethal solver…"): the reply plays the Activates it can see "then its
/// attacks", and `simulate_reply` runs "until the seat's next main phase or prompt, or game end"
/// (reply.rs line 887). Three unblocked 4/4s against the AI at 12 are lethal, and the reply takes it. A
/// Miner v5 beside them (five Activates a turn, each followed by the play of the Resource it gained, a
/// card not in `hidden`) spends ten of `AI_REPLY.max_steps` (12) before the first attack, so the loop
/// (reply.rs line 911) runs out after two attacks and hands back a state with the opponent still on turn
/// and its third 4/4 unswung, which `reply_score` then scores as the whole reply.
#[test]
fn a_five_use_activate_does_not_crowd_the_opponents_lethal_attacks_out_of_the_reply() {
    // On this seed the five Resources are Gellium and Korrelite (a heal and a 1-damage ping), so nothing
    // but the step cap stands between the opponent and its third attack.
    let reply = |field: Value| {
        let state = handed_over_on("audit-reply-miner-7", json!({ "health": 12 }), json!({ "field": field }));
        let counter = create_node_counter(600, None);
        simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE).expect("a reply")
    };
    let plain = reply(json!([VANILLA, VANILLA, VANILLA]));
    assert_eq!(
        plain.result.map(|result| result.winner),
        Some(Winner::from(HUMAN)),
        "three 4/4s at the open face of a hero at 12 are lethal"
    );
    let mined = reply(json!([VANILLA, VANILLA, VANILLA, MINER]));
    let unswung = active_units_of(&mined, HUMAN)
        .into_iter()
        .filter(|unit| unit.def_id == VANILLA && !unit.exertion.attacked)
        .count();
    assert_eq!(
        mined.result.map(|result| result.winner),
        Some(Winner::from(HUMAN)),
        "beside a Miner v5 the reply stopped with the opponent still on turn: active {:?}, turn {}, \
         the AI at {}, {unswung} of the 4/4s unswung",
        mined.active,
        mined.turn,
        mined.players[AI].hero.health
    );
}

/// R1200 (`crates/engine/src/reduce.rs` `attack`, `config::RANDOM_ATTACK_TARGET`): while a Mayor
/// Medinamogger acts on either field, `legal_actions` lists each attacker's one attack with the target
/// `"random"`, and the engine draws the target. reply.rs `ranked_attacks` (lines 348-355) compares that
/// id with the seat's hero id and then asks `trade_value` for a Unit of that id, which answers
/// `f64::NEG_INFINITY` (lines 162-168), so no attack is ever "worth making" and the opponent ends its
/// turn. README: the reply plays "its attacks"; `b41` (docs/polish/3-ai.md): it takes lethal in reach.
#[test]
fn r1200_the_reply_attacks_while_a_mayor_draws_every_target_at_random() {
    let reply = |p1: Value, p2: Value| {
        let state = handed_over(p1, p2);
        let counter = create_node_counter(200, None);
        simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE).expect("a reply")
    };
    // The opponent's Mayor (5/4) and 4/4 against the AI at 4 with no Unit: the hero is the only target
    // the draw can name, and either hit is lethal.
    let lethal = reply(json!({ "health": 4 }), json!({ "field": [VANILLA, MAYOR] }));
    assert_eq!(
        lethal.result.map(|result| result.winner),
        Some(Winner::from(HUMAN)),
        "under its own Mayor the opponent attacked nothing: active {:?}, turn {}, the AI at {}",
        lethal.active,
        lethal.turn,
        lethal.players[AI].hero.health
    );
}

/// The same with the Mayor on the AI's own side: the opponent's 4/4 has one legal attack, drawn between
/// the Mayor and the AI's hero, and the reply never makes it.
#[test]
fn r1200_the_reply_attacks_while_the_ais_own_mayor_acts() {
    let state = handed_over(json!({ "health": 4, "field": [MAYOR] }), json!({ "field": [VANILLA] }));
    assert!(
        legal_actions(&state, HUMAN)
            .iter()
            .any(|action| matches!(action, ActionBody::Attack { target_id, .. } if target_id == "random")),
        "the engine offers the 4/4 its random attack"
    );
    let counter = create_node_counter(200, None);
    let reply =
        simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE).expect("a reply");
    let swung = reply.result.is_some()
        || active_units_of(&reply, HUMAN)
            .into_iter()
            .any(|unit| unit.def_id == VANILLA && unit.exertion.attacked)
        || !on_field(&reply, HUMAN, VANILLA);
    assert!(
        swung,
        "the opponent's 4/4 never attacked: the AI at {}, its Mayor on the field: {}",
        reply.players[AI].hero.health,
        on_field(&reply, AI, MAYOR)
    );
}
