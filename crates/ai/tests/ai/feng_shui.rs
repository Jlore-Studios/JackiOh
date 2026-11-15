//! Meditative #40 Feng Shui meets the AI (issue #3951, R3951100). The search already plays every verdict
//! of the turn through `reduce` (R981–R985), so it saw a play's Radiant face and its hit; what it did not
//! see was the element it ends on, which judges its next play. While a Feng Shui acts, the evaluation now
//! counts the best verdict that element gives a card in the AI's own hand (`feng_shui_outlook`), the
//! move order tries a rewarded play first and a punished one last, and the beam keeps apart two orders
//! of the same plays that end on different elements (`search_signature`).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, close, register_cards, scenario};

const FENG_SHUI: &str = "meditative-040";
/// Core #8 Mr. Vanilla: a (1) 4/4 with no text, Radiant 12/12; index 8, 木.
const WOOD: &str = "core-008";
/// TG #1: a (1) 5/2 with no text, Radiant 15/6; index 1, 水.
const WATER: &str = "traininggrounds-001";
/// TG #6: a (1) 3/3 Divine Shield, Radiant 9/9; index 6, 水.
const WATER_SHIELD: &str = "traininggrounds-006";
/// M #100: a (2) 7/7 with no text; index 100, 土.
const EARTH: &str = "meditative-100";
/// Core #96: a Trap, which no Feng Shui judges (R982), so the card a reply draws moves no verdict.
const TRAP: &str = "core-096";

fn library() -> Value {
    json!([TRAP, TRAP, TRAP, TRAP, TRAP, TRAP])
}

/// The AI on its turn with `mana`, `hand` and `last` as its last element (R982), and a Feng Shui
/// (`radiant` for its Radiant face) in `judge`'s backrow, or none.
fn scene(
    seed: &str,
    judge: Option<(PlayerId, bool)>,
    mana: i32,
    hand: Value,
    last: Option<CardElement>,
) -> GameState {
    register_cards();
    let backrow = |player: PlayerId| match judge {
        Some((owner, radiant)) if owner == player => json!([{ "def": FENG_SHUI, "radiant": radiant }]),
        _ => json!([]),
    };
    let s = scenario(json!({
        "seed": seed,
        "p1": { "mana": mana, "backrow": backrow(AI), "hand": hand, "library": library() },
        "p2": { "hand": [TRAP], "backrow": backrow(HUMAN), "library": library() },
    }));
    let mut state = s.state().clone();
    state.players[AI]
        .game_log
        .get_or_insert_with(GameLog::default)
        .last_element = last;
    state
}

fn decide_at(state: &GameState, seed: &str) -> Decision {
    decide(state, AI, &mut AiOptions::new(create_rng(seed, 0))).expect("the AI acts")
}

/// The AI's hand card of `def_id`.
fn in_hand<'a>(state: &'a GameState, def_id: &str) -> &'a CardInstance {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .expect("the card is in the AI's hand")
}

fn plays(action: &ActionBody, card: &CardInstance) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id)
}

/// Every verdict a Feng Shui gave `player`'s plays, in order (the public `fengShui` events, R984).
fn verdicts_on(state: &GameState, player: PlayerId) -> Vec<FengShuiOutcome> {
    state
        .applied
        .iter()
        .flat_map(|applied| applied.events.iter())
        .filter_map(|event| match event {
            GameEvent::FengShui {
                player: judged,
                outcome,
                ..
            } if *judged == player => Some(*outcome),
            _ => None,
        })
        .collect()
}

/// The AI's whole turn from `state`.
fn ai_turn(state: &GameState, seed: &str) -> GameState {
    play_ai_turn(state, AI, &mut AiOptions::new(create_rng(seed, 0))).state
}

#[test]
fn r3951100_the_ai_plays_the_card_feng_shui_rewards_not_the_one_it_punishes() {
    // 金 generates 水 and overcomes 木 (R981). The human's Feng Shui, so no Rock of the AI's own changes it.
    let state = scene(
        "ai-feng-shui-rewarded",
        Some((HUMAN, false)),
        1,
        json!([WOOD, WATER_SHIELD]),
        Some(CardElement::Metal),
    );
    let water = in_hand(&state, WATER_SHIELD).clone();
    let decision = decide_at(&state, "ai-feng-shui-rewarded");
    assert!(plays(&decision.action, &water), "{:?}", decision.action);
    let after = ai_turn(&state, "ai-feng-shui-rewarded");
    assert_eq!(verdicts_on(&after, AI), vec![FengShuiOutcome::Positive]);
    assert_eq!(after.players[AI].hero.health, HERO_HEALTH, "no punishment");
}

#[test]
fn r3951100_the_ai_orders_two_plays_so_both_are_rewarded_and_neither_punished() {
    // 金 → 水 → 木 generates twice; 木 first would be punished (金 overcomes 木) and leave 水 plain.
    let state = scene(
        "ai-feng-shui-order",
        Some((HUMAN, false)),
        2,
        json!([WOOD, WATER]),
        Some(CardElement::Metal),
    );
    let water = in_hand(&state, WATER).clone();
    let decision = decide_at(&state, "ai-feng-shui-order");
    assert!(plays(&decision.action, &water), "{:?}", decision.action);
    let after = ai_turn(&state, "ai-feng-shui-order");
    assert_eq!(
        verdicts_on(&after, AI),
        vec![FengShuiOutcome::Positive, FengShuiOutcome::Positive]
    );
    assert_eq!(after.players[AI].hero.health, HERO_HEALTH, "no punishment");
}

#[test]
fn r3951100_the_ai_ends_its_turn_on_the_element_that_rewards_its_next_play() {
    // Both plays are plain after 木. The Vanilla is the better body now, but the 5/2 leaves 水, which
    // generates 木, so the Vanilla enters next turn as a Radiant 12/12; the Vanilla would leave 木, which
    // rewards nothing the AI holds.
    let state = scene(
        "ai-feng-shui-setup",
        Some((HUMAN, false)),
        1,
        json!([WOOD, WATER]),
        Some(CardElement::Wood),
    );
    let water = in_hand(&state, WATER).clone();
    let wood = in_hand(&state, WOOD).clone();
    let decision = decide_at(&state, "ai-feng-shui-setup");
    assert!(plays(&decision.action, &water), "{:?}", decision.action);

    // The outlook is what tips it: without it the evaluation prefers the Vanilla's board.
    let after = |card: &CardInstance| {
        act(
            &state,
            AI,
            json_as::<ActionBody>(json!({ "type": "play", "instanceId": card.id })),
        )
    };
    let (played_water, played_wood) = (after(&water), after(&wood));
    let blind = EvalWeights {
        feng_shui_outlook: 0.0,
        ..AI_EVAL
    };
    let score = |state: &GameState, w: &EvalWeights| evaluate(state, AI, NextSwing::Enemy, w);
    assert!(score(&played_wood, &blind) > score(&played_water, &blind));
    assert!(score(&played_water, &AI_EVAL) > score(&played_wood, &AI_EVAL));
}

/// `evaluate` of the AI holding only the 5/2 (水) after `last`, under `judge`'s Feng Shui.
fn holding_water(judge: Option<(PlayerId, bool)>, last: CardElement) -> f64 {
    let state = scene("ai-feng-shui-outlook", judge, 0, json!([WATER]), Some(last));
    evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL)
}

#[test]
fn r3951100_a_hand_card_the_last_element_punishes_costs_a_share_of_the_hit() {
    // 土 overcomes 水; 木 neither generates nor overcomes it (R981).
    let own = Some((AI, false));
    let hit = f64::from(FENG_SHUI_DAMAGE.on(false));
    let life = f64::from(HERO_HEALTH);
    let curve =
        AI_EVAL.hero_health * (life * life).sqrt() - AI_EVAL.hero_health * ((life - hit) * life).sqrt();
    close(
        holding_water(own, CardElement::Wood) - holding_water(own, CardElement::Earth),
        AI_EVAL.feng_shui_outlook * curve,
    );
    // The human's base Feng Shui judges the AI too (R984).
    close(
        holding_water(Some((HUMAN, false)), CardElement::Wood)
            - holding_water(Some((HUMAN, false)), CardElement::Earth),
        AI_EVAL.feng_shui_outlook * curve,
    );
    // An own Radiant Feng Shui punishes only the opponent (R984), and with none the element is nothing.
    close(
        holding_water(Some((AI, true)), CardElement::Wood),
        holding_water(Some((AI, true)), CardElement::Earth),
    );
    close(
        holding_water(None, CardElement::Wood),
        holding_water(None, CardElement::Earth),
    );
}

#[test]
fn r3951100_a_hand_card_the_last_element_rewards_counts_a_share_of_its_radiant_face() {
    // 金 generates 水 (R981): the 5/2 would enter as its Radiant 15/6 (R983).
    let gain = AI_EVAL.radiant_in_hand
        + AI_EVAL.radiant_face_share
            * (AI_EVAL.attack * f64::from(15 - 5) + AI_EVAL.health * f64::from(6 - 2));
    for judge in [(AI, false), (AI, true), (HUMAN, false)] {
        close(
            holding_water(Some(judge), CardElement::Metal) - holding_water(Some(judge), CardElement::Wood),
            AI_EVAL.feng_shui_outlook * gain,
        );
    }
    // The opponent's Radiant Feng Shui rewards only its controller (R984).
    close(
        holding_water(Some((HUMAN, true)), CardElement::Metal),
        holding_water(Some((HUMAN, true)), CardElement::Wood),
    );
}

/// The hand cards' first plays among `state`'s candidates, by definition, in move order.
fn play_order(state: &GameState) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    for action in candidate_actions(state, AI, Prices::NONE) {
        let ActionBody::Play { instance_id, .. } = &action else {
            continue;
        };
        let Some(card) = state.players[AI].hand.iter().find(|card| card.id == *instance_id) else {
            continue;
        };
        if !order.contains(&card.def_id) {
            order.push(card.def_id.clone());
        }
    }
    order
}

#[test]
fn r3951100_the_move_order_tries_the_rewarded_play_first_and_the_punished_one_last() {
    // After 金: the 5/2 (水) is rewarded, the 7/7 (土) plain, the Vanilla (木) punished.
    let hand = json!([EARTH, WOOD, WATER]);
    let judged = scene(
        "ai-feng-shui-move-order",
        Some((AI, false)),
        2,
        hand.clone(),
        Some(CardElement::Metal),
    );
    assert_eq!(play_order(&judged), vec![WATER, EARTH, WOOD]);
    // With no Feng Shui the order is tier 2's own: dearest first.
    let plain = scene("ai-feng-shui-move-order", None, 2, hand, Some(CardElement::Metal));
    assert_eq!(play_order(&plain), vec![EARTH, WOOD, WATER]);
}

#[test]
fn r3951100_the_ai_takes_its_rock_to_turn_a_punished_play_into_a_rewarded_one() {
    // After 金 the Vanilla is punished, but the Activate's Auspicious Rock (M #39.1, index 39.1: 水) is
    // free, rewarded after 金, and leaves 水, which generates 木: the Vanilla enters as a 12/12.
    let state = scene(
        "ai-feng-shui-rock",
        Some((AI, false)),
        1,
        json!([WOOD]),
        Some(CardElement::Metal),
    );
    let wood = in_hand(&state, WOOD).id.clone();
    let after = ai_turn(&state, "ai-feng-shui-rock");
    assert!(
        !verdicts_on(&after, AI).contains(&FengShuiOutcome::Negative),
        "{:?}",
        verdicts_on(&after, AI)
    );
    let vanilla = active_units_of(&after, AI)
        .into_iter()
        .find(|unit| unit.id == wood)
        .expect("the Vanilla is on the field");
    assert!(vanilla.radiant, "rewarded after the Rock");
}

/// `search_signature` after the AI plays `first` and then `second` from `state`, the Vanilla into lane 1
/// and the 5/2 into lane 2 whichever comes first, so the two orders reach one board.
fn signature_after(state: &GameState, first: &str, second: &str) -> String {
    let play = |state: &GameState, def_id: &str| {
        let id = in_hand(state, def_id).id.clone();
        let lane = if def_id == WOOD { 1 } else { 2 };
        act(
            state,
            AI,
            json_as::<ActionBody>(
                json!({ "type": "play", "instanceId": id, "zone": { "row": "units", "lane": lane } }),
            ),
        )
    };
    let end = play(&play(state, first), second);
    search_signature(&end, AI)
}

#[test]
fn r3951100_two_orders_of_the_same_plays_that_end_on_different_elements_are_two_positions() {
    // The human's Radiant Feng Shui rewards only the human (R984), so after 木 neither order of the
    // Vanilla (木) and the 5/2 (水) is judged: one board, ending on 水 or on 木.
    let judged = scene(
        "ai-feng-shui-signature",
        Some((HUMAN, true)),
        2,
        json!([WOOD, WATER]),
        Some(CardElement::Wood),
    );
    let (wood_last, water_last) = (
        signature_after(&judged, WATER, WOOD),
        signature_after(&judged, WOOD, WATER),
    );
    assert_ne!(wood_last, water_last);
    // With no Feng Shui the element is nothing, and the two orders are one position, as they were.
    let plain = scene(
        "ai-feng-shui-signature",
        None,
        2,
        json!([WOOD, WATER]),
        Some(CardElement::Wood),
    );
    assert_eq!(
        signature_after(&plain, WATER, WOOD),
        signature_after(&plain, WOOD, WATER)
    );
}
