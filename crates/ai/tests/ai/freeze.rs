//! Frozen in the AI's reads (R1174000, R4170000): the candidates come from `legal_actions`, so a Frozen Unit's
//! attack and switch are never among them; the evaluation leaves a Unit still Frozen next turn out of the face
//! damage it counts (`face_threat_with`, through `can_swing`); and the search signature tells two positions
//! apart that differ only in a freeze. For a side whose attacks it reads as this turn's (the AI at its turn
//! start after the opponent's reply, the enemy on turn in a passed line), the evaluation also leaves out every
//! Frozen unit, though it thaws at this turn's cleanup, and a unit's hit on the hero while it is summoning sick
//! without Charge (`frozen_sick_before_swing`, #4170).
//!
//! A dozen shipped cards freeze, among them S+ #27 Gargantuar (itself, after it attacks), S+ #22 Cryofluid,
//! S+ #91 Tsunami, S #175 Frost Module's Activate, S #146 Flash Freeze, S #1 Freeze Ray, S #11.1 Frostbolt,
//! M+ #24 Bob and S #2 Absolute Zero (forever).
//! Most boards here freeze a unit on the field by writing its thaw, as `freeze::freeze_unit` would; one plays
//! Gargantuar's attack through the engine. The cards: Mr. Vanilla 4/4, Tempo Timmy 3/3 (Rush, First Strike), Gargantuar
//! 12/12, and Core #58 Rush Token Farm, whose Rush Token is a 3/3.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, close, register_cards, unit};

const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// AI_EVAL with the this-turn refusals off: every unit `can_swing` lets through counts, as before #4170.
const OFF: EvalWeights = EvalWeights {
    frozen_sick_before_swing: false,
    ..AI_EVAL
};

/// The AI's Mr. Vanilla ready on the field with a card in hand, and the human's Tempo Timmy; the AI's
/// turn 9.
fn board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "ai-frozen",
        "p1": { "hand": ["core-011"], "field": ["core-008"], "library": LIBRARY },
        "p2": { "hand": ["core-011"], "field": ["core-011"], "library": LIBRARY },
    }))
    .state()
    .clone()
}

/// `player`'s first unit on the field, frozen to thaw at the cleanup of its controller's turn `thaw`
/// or later (`CardInstance.frozen`).
fn freeze(state: &mut GameState, player: PlayerId, thaw: i32) -> String {
    let id = active_units_of(state, player)[0].id.clone();
    find_instance_mut(state, &id).expect("the unit").frozen = Some(thaw);
    id
}

#[test]
fn r1174000_a_frozen_units_attack_and_switch_are_never_candidates() {
    let mut state = board();
    let unit = active_units_of(&state, AI)[0].id.clone();
    let acts = |state: &GameState| {
        candidate_actions(state, AI, Prices::NONE)
            .iter()
            .any(|action| match action {
                ActionBody::Attack { attacker_id, .. } => *attacker_id == unit,
                ActionBody::SwitchPosition { instance_id } => *instance_id == unit,
                _ => false,
            })
    };
    assert!(acts(&state), "a ready unit attacks or switches");
    let turn = state.turn;
    freeze(&mut state, AI, turn);
    assert!(!acts(&state), "a Frozen one does neither");
}

#[test]
fn r1174000_face_threat_leaves_out_a_unit_still_frozen_next_turn() {
    let state = board();
    let ready = face_threat(&state, AI);
    assert!(ready > 0, "the AI's Mr. Vanilla threatens the human's face");

    // Frozen through its controller's next turn: no swing to count.
    let mut frozen = state.clone();
    let turn = frozen.turn;
    freeze(&mut frozen, AI, turn + 1);
    assert_eq!(face_threat(&frozen, AI), 0);

    // Thawing at this turn's cleanup, its controller's: it swings next turn.
    let mut thawing = state.clone();
    freeze(&mut thawing, AI, turn);
    assert_eq!(face_threat(&thawing, AI), ready);

    // A Frozen unit of the player not on turn is Frozen through that player's next turn, whatever
    // its stamp says.
    let human = face_threat(&state, HUMAN);
    assert!(human > 0);
    let mut theirs = state.clone();
    freeze(&mut theirs, HUMAN, turn);
    assert_eq!(face_threat(&theirs, HUMAN), 0);
}

#[test]
fn r1174000_two_positions_that_differ_only_in_a_freeze_sign_apart() {
    let state = board();
    let mut frozen = state.clone();
    let turn = frozen.turn;
    freeze(&mut frozen, HUMAN, turn + 1);
    assert_ne!(search_signature(&state, AI), search_signature(&frozen, AI));
}

/// The AI's `field` and `backrow` with a card in hand, and the human with a card in hand and an empty field;
/// the AI's turn 9 at seed `seed`.
fn duel(seed: &str, field: Value, backrow: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": { "hand": ["core-011"], "field": field, "backrow": backrow, "library": LIBRARY },
        "p2": { "hand": ["core-011"], "library": LIBRARY },
    }))
    .state()
    .clone()
}

/// The position the AI scores after the human's reply to `ended` (the human on turn): the AI's own turn
/// started, its start-of-turn effects resolved, with this turn's attacks to come.
fn replied(ended: &GameState) -> GameState {
    assert_eq!(ended.active, HUMAN);
    let counter = create_node_counter(100, None);
    let after =
        simulate_reply(ended, AI, &counter, &hidden_card_ids(ended, AI), Prices::NONE).expect("a reply");
    assert_eq!(after.active, AI);
    after
}

/// The AI's first unit on the field, mutably.
fn own_unit(state: &mut GameState) -> &mut CardInstance {
    let id = active_units_of(state, AI)[0].id.clone();
    find_instance_mut(state, &id).expect("the unit")
}

/// At the AI's turn start (`NextSwing::Seat`, the AI on turn), its Mr. Vanilla Frozen to thaw at this turn's
/// cleanup attacks nothing this turn, so its 4 damage leave the pressure term. The end-of-turn read
/// (`face_threat`) still counts it, as R1174000 says, and the read with the enemy swinging first does too.
#[test]
fn r4170000_a_unit_frozen_until_this_turns_cleanup_adds_no_pressure_at_the_seats_turn_start() {
    let ready = board();
    let mut frozen = ready.clone();
    let turn = frozen.turn;
    freeze(&mut frozen, AI, turn);
    assert_eq!(face_threat(&ready, AI), 4);
    assert_eq!(face_threat(&frozen, AI), 4);
    let lost = AI_EVAL.pressure_per_damage * 4.0;
    close(
        evaluate(&ready, AI, NextSwing::Seat, &AI_EVAL) - evaluate(&frozen, AI, NextSwing::Seat, &AI_EVAL),
        lost,
    );
    close(
        evaluate(&frozen, AI, NextSwing::Seat, &OFF) - evaluate(&frozen, AI, NextSwing::Seat, &AI_EVAL),
        lost,
    );
    close(
        evaluate(&frozen, AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&frozen, AI, NextSwing::Enemy, &OFF),
    );
}

/// The issue's case, through the engine: Gargantuar hits the human for 12 and freezes itself, stamped with the
/// AI's next turn. After the human's reply (`simulate_reply`) the AI is on that turn and Gargantuar is still
/// Frozen, so its 12 add nothing to the pressure the AI scores, as they did before #4170 (OFF).
#[test]
fn r4170000_a_gargantuar_that_froze_itself_adds_no_pressure_after_the_reply() {
    let state = duel(
        "ai-frozen-gargantuar",
        json!([{ "def": "stinkyscapeplus-027", "lane": 3 }]),
        json!([]),
    );
    let gargantuar = unit(&state, AI, "stinkyscapeplus-027").id.clone();
    let attacked = act(
        &state,
        AI,
        &ActionBody::Attack {
            attacker_id: gargantuar.clone(),
            target_id: format!("hero-{HUMAN}"),
        },
    );
    assert_eq!(attacked.players[HUMAN].hero.health, 18);
    assert!(
        find_instance(&attacked, &gargantuar)
            .expect("Gargantuar")
            .frozen
            .is_some()
    );

    let after = replied(&act(&attacked, AI, &ActionBody::EndTurn));
    assert!(
        find_instance(&after, &gargantuar)
            .expect("Gargantuar")
            .frozen
            .is_some()
    );
    assert_eq!(
        face_threat(&after, AI),
        12,
        "the end-of-turn read: it thaws at this turn's cleanup"
    );

    let mut thawed = after.clone();
    find_instance_mut(&mut thawed, &gargantuar)
        .expect("Gargantuar")
        .frozen = None;
    close(
        evaluate(&thawed, AI, NextSwing::Seat, &AI_EVAL) - evaluate(&after, AI, NextSwing::Seat, &AI_EVAL),
        AI_EVAL.pressure_per_damage * 12.0,
    );
    close(
        evaluate(&thawed, AI, NextSwing::Seat, &OFF),
        evaluate(&after, AI, NextSwing::Seat, &OFF),
    );
}

/// The same refusal for the enemy: a freeze the AI lands on the human's Tempo Timmy on its own turn is stamped
/// with the human's next turn. Once that turn starts, in a line the AI has passed (`NextSwing::Enemy`, the
/// human on turn), Timmy attacks nothing this turn, so its 3 damage leave the threat term.
#[test]
fn r4170000_an_enemy_unit_frozen_on_the_ais_turn_adds_no_threat_once_its_own_turn_starts() {
    let mut state = board();
    let turn = state.turn;
    let timmy = freeze(&mut state, HUMAN, turn + 1);
    let ended = act(&state, AI, &ActionBody::EndTurn);
    assert_eq!((ended.active, ended.turn), (HUMAN, turn + 1));
    assert_eq!(
        find_instance(&ended, &timmy).expect("Timmy").frozen,
        Some(turn + 1)
    );
    close(
        evaluate(&ended, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(&ended, AI, NextSwing::Enemy, &OFF),
        AI_EVAL.threat_per_damage * 3.0,
    );
}

/// At the AI's turn start, its Mr. Vanilla summoned this turn attacks nothing without Rush or Charge, and with
/// Rush alone lands nothing on the hero (§6.1); with Charge it counts as a ready unit. The read with the enemy
/// swinging first is untouched.
#[test]
fn r4170000_a_unit_summoned_this_turn_hits_the_hero_only_with_charge() {
    let arrived = board();
    let turn = arrived.turn;
    let mut sick = arrived.clone();
    own_unit(&mut sick).summoned_turn = Some(turn);
    let lost = AI_EVAL.pressure_per_damage * 4.0;
    close(
        evaluate(&arrived, AI, NextSwing::Seat, &AI_EVAL) - evaluate(&sick, AI, NextSwing::Seat, &AI_EVAL),
        lost,
    );
    close(
        evaluate(&sick, AI, NextSwing::Seat, &OFF) - evaluate(&sick, AI, NextSwing::Seat, &AI_EVAL),
        lost,
    );
    close(
        evaluate(&sick, AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&sick, AI, NextSwing::Enemy, &OFF),
    );

    let mut rush = sick.clone();
    own_unit(&mut rush).granted_keywords.push(Keyword::Rush);
    close(
        evaluate(&rush, AI, NextSwing::Seat, &OFF) - evaluate(&rush, AI, NextSwing::Seat, &AI_EVAL),
        lost,
    );

    let mut charge = sick.clone();
    own_unit(&mut charge).granted_keywords.push(Keyword::Charge);
    close(
        evaluate(&charge, AI, NextSwing::Seat, &AI_EVAL),
        evaluate(&charge, AI, NextSwing::Seat, &OFF),
    );
}

/// Through the engine: Rush Token Farm summons a 3/3 Rush Token at the AI's turn start, inside the reply. At
/// that turn start the token may attack only Units, so its 3 add nothing to the pressure the AI scores.
#[test]
fn r4170000_a_rush_token_summoned_at_the_ais_turn_start_adds_no_pressure_after_the_reply() {
    let state = duel("ai-frozen-farm", json!([]), json!(["core-058"]));
    let after = replied(&act(&state, AI, &ActionBody::EndTurn));
    assert_eq!(unit(&after, AI, "core-t-rush").summoned_turn, Some(after.turn));
    close(
        evaluate(&after, AI, NextSwing::Seat, &OFF) - evaluate(&after, AI, NextSwing::Seat, &AI_EVAL),
        AI_EVAL.pressure_per_damage * 3.0,
    );
}

/// The greedy baseline the gates measure against reads as before #4170: the flag is off in GREEDY_EVAL, so
/// a unit Frozen to thaw at this turn's cleanup and one summoned this turn each count as a ready unit there.
#[test]
fn r4170000_greedy_eval_reads_frozen_and_new_units_as_before() {
    const { assert!(AI_EVAL.frozen_sick_before_swing) };
    const { assert!(!GREEDY_EVAL.frozen_sick_before_swing) };
    let ready = board();
    let turn = ready.turn;
    let mut frozen = ready.clone();
    freeze(&mut frozen, AI, turn);
    let mut sick = ready.clone();
    own_unit(&mut sick).summoned_turn = Some(turn);
    let greedy = |state: &GameState| evaluate(state, AI, NextSwing::Seat, &GREEDY_EVAL);
    close(greedy(&frozen), greedy(&ready));
    close(greedy(&sick), greedy(&ready));
}
