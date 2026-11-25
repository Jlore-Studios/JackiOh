//! Issue #4783 (R4783040): the AI keeps Classic #37 Last Hurrah and Stinkyscape+ #79 A Glimpse into the
//! Endless Sea for a last resort. A play of either is a candidate only while the AI's times are dire (its life
//! at most `AI_LAST_RESORT_DIRE_LIFE`, or the enemy's board threatening its life next turn) or while it has no
//! other play, having played no card this turn; Glimpse also while a card of its own would shuffle cards back
//! into its deck. The Trade of a Tradable Last Hurrah stays a candidate.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards, scenario};

const HURRAH: &str = "classic-037";
const GLIMPSE: &str = "stinkyscapeplus-079";
/// Core #8 Mr. Vanilla: a (1) 4/4 with no text.
const VANILLA: &str = "core-008";
/// Core #25: a (4) 7/7 with Armor 7, which 1 mana does not pay for.
const SEVEN_SEVEN: &str = "core-025";
/// Core #19 Midrange Menace: a (3) 9/9 Taunt.
const MENACE: &str = "core-019";
/// Classic #30 Recycle: "Shuffle your graveyard into your deck. Draw 1."
const RECYCLE: &str = "classic-030";
/// Stinkyscape+ #24 Forced Propagation: "Shuffle copies of it into your deck."
const PROPAGATION: &str = "stinkyscapeplus-024";

fn library() -> Value {
    json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA])
}

/// The AI on its turn with `mana`, `health` and `hand`, over five Mr. Vanillas, against the human's `field`.
fn scene(seed: &str, mana: i32, health: i32, hand: Value, field: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": { "mana": mana, "health": health, "hand": hand, "library": library() },
        "p2": { "hand": [VANILLA], "field": field, "library": library() },
    }))
    .state()
    .clone()
}

/// The id of the AI's hand card of `def_id`.
fn in_hand(state: &GameState, def_id: &str) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
        .expect("the card is in the AI's hand")
}

fn plays(action: &ActionBody, id: &str) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id)
}

/// Whether a play of the AI's hand card of `def_id` is among the search's candidates.
fn offered(state: &GameState, def_id: &str) -> bool {
    let id = in_hand(state, def_id);
    candidate_actions(state, AI, Prices::NONE)
        .iter()
        .any(|action| plays(action, &id))
}

/// Whether the AI's whole turn from `state` plays its hand card of `def_id`.
fn turn_plays(state: &GameState, seed: &str, def_id: &str) -> bool {
    let id = in_hand(state, def_id);
    play_ai_turn(state, AI, &mut AiOptions::new(create_rng(seed, 0)))
        .decisions
        .iter()
        .any(|decision| plays(&decision.action, &id))
}

#[test]
fn r4783040_the_ai_holds_last_hurrah_while_it_has_another_play() {
    let state = scene(
        "ai-hurrah-held",
        1,
        HERO_HEALTH,
        json!([HURRAH, VANILLA]),
        json!([]),
    );
    let hurrah = in_hand(&state, HURRAH);
    assert!(
        legal_actions(&state, AI)
            .iter()
            .any(|action| plays(action, &hurrah))
    );
    assert!(!offered(&state, HURRAH));
    assert!(offered(&state, VANILLA));
    // Its Trade stays a candidate.
    let trade = ActionBody::Trade { instance_id: hurrah };
    assert!(candidate_actions(&state, AI, Prices::NONE).contains(&trade));
    // Once the Vanilla is played the hand holds nothing else, but the turn has had its play: the AI does not
    // draw its deck as the turn's last action.
    for k in ["ai-hurrah-held-k1", "ai-hurrah-held-k2"] {
        assert!(!turn_plays(&state, k, HURRAH), "{k}");
    }
}

#[test]
fn r4783040_the_ai_plays_last_hurrah_when_its_times_are_dire() {
    let state = scene(
        "ai-hurrah-dire",
        1,
        AI_LAST_RESORT_DIRE_LIFE,
        json!([HURRAH, VANILLA]),
        json!([]),
    );
    assert!(offered(&state, HURRAH));
    assert!(turn_plays(&state, "ai-hurrah-dire-k1", HURRAH));
    // One more life and the times are not dire.
    let state = scene(
        "ai-hurrah-dire",
        1,
        AI_LAST_RESORT_DIRE_LIFE + 1,
        json!([HURRAH, VANILLA]),
        json!([]),
    );
    assert!(!offered(&state, HURRAH));
}

#[test]
fn r4783040_the_ai_may_play_last_hurrah_facing_lethal_on_the_board() {
    // Three 9/9s swing for 27 into the AI's 20 life next turn.
    let state = scene(
        "ai-hurrah-lethal",
        1,
        20,
        json!([HURRAH, VANILLA]),
        json!([MENACE, MENACE, MENACE]),
    );
    assert!(face_threat(&state, HUMAN) >= 20);
    assert!(offered(&state, HURRAH));
    // One 9/9 does not reach it.
    let state = scene(
        "ai-hurrah-lethal",
        1,
        20,
        json!([HURRAH, VANILLA]),
        json!([MENACE]),
    );
    assert!(!offered(&state, HURRAH));
}

#[test]
fn r4783040_the_ai_plays_last_hurrah_when_it_has_no_other_play() {
    // The 7/7 costs 4 and the AI has 1: Last Hurrah is its only play.
    let state = scene(
        "ai-hurrah-no-play",
        1,
        HERO_HEALTH,
        json!([HURRAH, SEVEN_SEVEN]),
        json!([]),
    );
    assert!(offered(&state, HURRAH));
    assert!(turn_plays(&state, "ai-hurrah-no-play-k1", HURRAH));
}

#[test]
fn r4783040_the_ai_holds_glimpse_while_it_has_another_play() {
    // 5 mana pays for the Vanilla and the Glimpse both.
    let state = scene(
        "ai-glimpse-held",
        5,
        HERO_HEALTH,
        json!([GLIMPSE, VANILLA]),
        json!([]),
    );
    assert!(!offered(&state, GLIMPSE));
    for k in ["ai-glimpse-held-k1", "ai-glimpse-held-k2"] {
        assert!(!turn_plays(&state, k, GLIMPSE), "{k}");
    }
    // Dire, it may.
    let dire = scene(
        "ai-glimpse-held",
        5,
        AI_LAST_RESORT_DIRE_LIFE,
        json!([GLIMPSE, VANILLA]),
        json!([]),
    );
    assert!(offered(&dire, GLIMPSE));
}

#[test]
fn r4783040_the_ai_may_play_glimpse_holding_a_card_that_shuffles_cards_back_into_its_deck() {
    for refill in [RECYCLE, PROPAGATION] {
        let state = scene(
            "ai-glimpse-refill",
            5,
            HERO_HEALTH,
            json!([GLIMPSE, VANILLA, refill]),
            json!([]),
        );
        assert!(offered(&state, GLIMPSE), "{refill}");
    }
    // Last Hurrah is no Glimpse: a refill in hand does not free it.
    let state = scene(
        "ai-glimpse-refill",
        5,
        HERO_HEALTH,
        json!([HURRAH, VANILLA, RECYCLE]),
        json!([]),
    );
    assert!(!offered(&state, HURRAH));
}
