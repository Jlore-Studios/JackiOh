//! R2660000: the ladder bots' concession check (`unstoppable_lethal`, SPEC §9.12): whether a lethal
//! shows on the opponent's board that nothing the bot does this turn stops. Both turns are played
//! through the engine, and every doubt plays on.
//!
//! The bot sits in p1 on its own turn; p2's board is Tempo Timmy 3/3 and Mr. Vanilla 4/4, 7 to the
//! face. The scenarios' libraries are empty unless a test fills the bot's, so p2's next draw is a
//! fatigue point of its own.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

/// A scenario on `seed` with `active` to act, p1 as `p1` and p2 as `p2`.
fn board(seed: &str, active: PlayerId, p1: Value, p2: Value) -> GameState {
    register_cards();
    scenario(json!({ "seed": seed, "active": active, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// p2 with the 3/3 and the 4/4 on its field and whatever `extra` adds.
fn seven(extra: Value) -> Value {
    let mut p2 = json!({ "field": ["core-011", "core-008"] });
    if let (Some(side), Some(extra)) = (p2.as_object_mut(), extra.as_object()) {
        side.extend(extra.clone());
    }
    p2
}

/// The bot, p1 on its own turn, as `p1`, facing `seven(p2)`.
fn facing(p1: Value, p2: Value) -> GameState {
    board("unstoppable", AI, p1, seven(p2))
}

/// R2660000: 7 to the face of a bot on 6 or 7 with nothing to play, or with a card that changes no
/// board (Core #10 Rapid Replenish, its Combo unmet), kills it whatever it does
#[test]
fn r2660000_a_board_that_kills_the_bot_whatever_it_does_is_unstoppable() {
    for p1 in [
        json!({ "health": 6 }),
        json!({ "health": 7 }),
        json!({ "health": 6, "hand": ["core-010"] }),
    ] {
        let state = facing(p1.clone(), json!({}));
        assert!(unstoppable_lethal(&state, AI, None), "{p1}");
    }
}

/// R2660000: 7 to the face of a bot on 8 leaves it standing
#[test]
fn r2660000_a_board_short_of_lethal_is_not_unstoppable() {
    let state = facing(json!({ "health": 8 }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: p2's two burn spells would finish a bot on 8, but a hand is no board
#[test]
fn r2660000_lethal_only_with_the_opponents_hand_is_not_unstoppable() {
    let state = facing(
        json!({ "health": 8 }),
        json!({ "hand": ["core-044", "core-035"] }),
    );
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000, §4.1: a unit played in Defense Position has Taunt, which walls both attackers' way to
/// the face
#[test]
fn r2660000_a_unit_played_into_defense_walls_the_lethal() {
    let state = facing(json!({ "health": 6, "hand": ["core-008"] }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: Core #16 Hit Job destroys the 4/4, and 3 to the face leaves a bot on 6 standing
#[test]
fn r2660000_removal_in_hand_stops_the_lethal() {
    let state = facing(json!({ "health": 6, "hand": ["core-016"] }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: Core #19 Midrange Menace's Taunt, already on the bot's board, walls the lethal
#[test]
fn r2660000_a_taunt_on_the_bots_board_walls_the_lethal() {
    let state = facing(json!({ "health": 6, "field": ["core-019"] }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: a bot whose 4/4 swings p2 from 4 to 0 wins before p2's board ever attacks
#[test]
fn r2660000_a_bot_that_wins_this_turn_is_not_doomed() {
    let state = facing(
        json!({ "health": 6, "field": ["core-008"] }),
        json!({ "health": 4 }),
    );
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: a clock that has run out (the bot's decision deadline) stops the walk before it proves
/// anything, and a walk the clock stopped concedes nothing
#[test]
fn r2660000_a_search_the_clock_stops_proves_nothing() {
    let state = facing(json!({ "health": 6 }), json!({}));
    let stopped = || true;
    assert!(!unstoppable_lethal(&state, AI, Some(&stopped)));
    let running = || false;
    assert!(unstoppable_lethal(&state, AI, Some(&running)));
}

/// R2660000: on p2's turn the bot has no turn to play, whatever p2's board would deal
#[test]
fn r2660000_off_the_bots_own_turn_nothing_is_unstoppable() {
    let state = board("unstoppable", HUMAN, json!({ "health": 6 }), seven(json!({})));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000, R185: states that differ only in p2's hand, its face-down trap and the seed redact alike
/// and give one answer, the one p2's board alone gives: two Traps set face-down against the bot
/// change nothing, neither stopping its answer nor making the lethal
#[test]
fn r2660000_the_opponents_hidden_cards_change_nothing() {
    for health in [6, 8] {
        let a = board(
            "unstoppable-a",
            AI,
            json!({ "health": health }),
            seven(json!({
                "hand": ["core-044", "core-035"],
                "backrow": [{ "def": "core-041", "faceUp": false }],
            })),
        );
        let b = board(
            "unstoppable-b",
            AI,
            json!({ "health": health }),
            seven(json!({
                "hand": ["core-016", "core-002"],
                "backrow": [{ "def": "core-060", "faceUp": false }],
            })),
        );
        assert_ne!(hash_state(&a), hash_state(&b), "health {health}");
        assert_eq!(
            hash_state(&redact(&a, AI)),
            hash_state(&redact(&b, AI)),
            "health {health}"
        );
        let doomed = unstoppable_lethal(&a, AI, None);
        assert_eq!(unstoppable_lethal(&b, AI, None), doomed, "health {health}");
        assert_eq!(doomed, health == 6, "health {health}");
    }
}

/// R2660000: Core #23 Reoccurring Dream rolls the game's random numbers for the Rapid Replenish
/// beside it, and the walk sees only one outcome of a roll: a line that rolls proves nothing, so the
/// bot holding it plays on, though no outcome of this one saves it. With nothing in hand to roll for,
/// the Dream rolls nothing, and the same bot is doomed.
#[test]
fn r2660000_a_line_that_rolls_proves_nothing() {
    let rolls = facing(
        json!({ "health": 6, "hand": ["core-023", "core-010"] }),
        json!({}),
    );
    assert!(!unstoppable_lethal(&rolls, AI, None));
    let alone = facing(json!({ "health": 6, "hand": ["core-023"] }), json!({}));
    assert!(unstoppable_lethal(&alone, AI, None));
}

/// R2660000: Core #5 Stockpile draws 2 from a library whose order the bot cannot see: whether the
/// draw finds the Hit Job that would save it is that order, so a line that draws proves nothing. It
/// proves nothing either from a library with nothing in it to save the bot, since the walk guesses
/// the order and cannot rule a card in or out by it.
#[test]
fn r2660000_a_line_that_draws_proves_nothing() {
    for library in [
        json!(["core-010", "core-016", "core-010", "core-010"]),
        json!(["core-010", "core-010", "core-010", "core-010"]),
    ] {
        let state = facing(
            json!({ "health": 4, "hand": ["core-005"], "library": library }),
            json!({}),
        );
        assert!(!unstoppable_lethal(&state, AI, None), "{library}");
    }
    // PREMISE: without the Stockpile, the same bot is doomed.
    let bare = facing(
        json!({ "health": 4, "library": ["core-010", "core-010"] }),
        json!({}),
    );
    assert!(unstoppable_lethal(&bare, AI, None));
}

/// R2660000: the opponent's lethal rolls too: Stinkyscape #53 Flak v3's Activate deals each enemy a
/// random 1 to 3, so with it p2's 7 kills a bot on 9 only by the roll, which a lethal that counts
/// cannot leave to chance. The solver's line is played again and rolls, so it proves nothing, on 8
/// as well, where every outcome would kill: the walk sees one outcome and cannot rule the others in.
/// Without the Activate's 1, p2's 7 does not reach 8.
#[test]
fn r2660000_an_opponents_lethal_that_rolls_proves_nothing() {
    for health in [8, 9] {
        let state = facing(
            json!({ "health": health }),
            json!({ "field": ["core-011", "core-008", "stinkyscape-053"] }),
        );
        assert!(!unstoppable_lethal(&state, AI, None), "health {health}");
    }
}

/// `facing(p1, p2)` with the Marketplace open (R1853000), offering the bot `offers` (its owner's) at
/// 40 Credits each, of which it holds 100.
fn with_market(p1: Value, p2: Value, offers: &[&str]) -> GameState {
    let mut state = facing(p1, p2);
    state.players[AI].market_credits = Some(100);
    let offer = |def_id: &&str| MarketOffer {
        def_id: def_id.to_string(),
        radiant: None,
        base: 40,
        price: 40,
        stock: 1,
        bought: PerPlayer::new(0, 0),
        owner: Some(AI),
        fixed: None,
    };
    state.marketplace = Some(Marketplace {
        offers: offers.iter().map(offer).collect(),
        quotes: Vec::new(),
    });
    state
}

/// R2660000, R1853000: the opponent's turn start rolls it new Marketplace offers on the game's
/// random numbers, which play no part in its board: the bot doomed without a Marketplace is doomed
/// with one, and a Mr. Vanilla it can buy and play in Defense Position this turn is still its way out
#[test]
fn r2660000_the_marketplace_neither_hides_nor_makes_the_lethal() {
    let open = with_market(json!({ "health": 6 }), json!({}), &[]);
    assert!(unstoppable_lethal(&open, AI, None));
    let buys_a_wall = with_market(json!({ "health": 6 }), json!({}), &["core-008"]);
    assert!(
        legal_actions(&buys_a_wall, AI).contains(&ActionBody::MarketBuy { offer: 0 }),
        "PREMISE: the bot can buy the offer"
    );
    assert!(!unstoppable_lethal(&buys_a_wall, AI, None));
}
