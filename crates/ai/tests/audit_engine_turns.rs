//! Audit (engine: turns, mana, drawing, zones, setup and end): one failing test per reported finding.
//! Each asserts the ruled behaviour and names the rule that makes it so.

use jackioh_engine::testkit::{GameEvent, PlayerId, Scenario, json, scenario};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const FILLER: &str = "core-005"; // Stockpile: a Spell.
const SHEEP: &str = "core-t-sheep"; // Sheep Token: a Unit token.
const RUSH: &str = "core-t-rush"; // Rush Token: a Unit token (#75 Infinite Reserves puts its card in a hand).
const SCREAM: &str = "stinkyscape-012-3"; // Psychic Scream: "Shuffle all permanents into your opponent's deck."
const SYNDICATE: &str = "stinkyscape-164"; // The Syndicate: "Aura: Cards in your hand have Tradable."

/// Every `shuffledIn` the scenario has reported, as (library's player, instance id, position).
fn shuffled_in(s: &Scenario) -> Vec<(PlayerId, String, i32)> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::ShuffledIn {
                player,
                instance_id,
                position,
                ..
            } => Some((*player, instance_id.clone(), *position)),
            _ => None,
        })
        .collect()
}

/// Finding 1a. SPEC §3.2 Tokens and R11: "A unit token ceases to exist when it leaves the field for any
/// reason", so Psychic Scream's own header says "a unit token ceasing (R11)". The token is gone and the
/// deck did not grow, yet `draw::into_library` (draw.rs:288-312) never reads `move_to_zone`'s
/// `MoveResult::Vanished`: it reports `shuffledIn` for the token at a deck position and answers
/// `ShuffleInOutcome::Library`. A FULL deck reports the same token honestly (`libraryOverflow` with
/// outcome `ceased`, draw.rs:250-262); only the deck with room lies.
#[test]
fn finding_1a_a_unit_token_shuffled_into_a_deck_with_room_is_reported_shuffled_in_though_it_ceased() {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "p1": { "hand": [SCREAM, FILLER], "field": [SHEEP], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    let token = s.unit_at(P1, 1).id.clone();

    s.play(SCREAM, json!({}));

    // R11: the token ceased to exist, and p2's deck holds what it held.
    s.expect_in_zone(&token, "gone");
    assert_eq!(s.pile(P2, "library").len(), 3);
    // So nothing was shuffled in, and no event may say otherwise (§10.3: an event is what happened).
    let reported: Vec<_> = shuffled_in(&s)
        .into_iter()
        .filter(|(_, id, _)| *id == token)
        .collect();
    assert!(
        reported.is_empty(),
        "the Sheep Token ceased to exist (R11) and p2's deck still holds 3 cards, but the engine reported \
         it shuffled in: {reported:?}"
    );
}

/// Finding 1b. The same hole through a Trade (R1838000, R1864002): The Syndicate lends Tradable to every
/// card in its controller's hand, a unit-token card among them (#75's Rush Token). R11: a unit-token card
/// "ceases to exist if it leaves that zone other than by being drawn or played", so the Trade ends it, and
/// `traded` is followed by a `shuffledIn` for a card that is in no deck.
#[test]
fn finding_1b_a_traded_unit_token_card_is_reported_shuffled_in_though_it_ceased() {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "p1": {
            "hand": [RUSH, FILLER],
            "field": [{ "def": SYNDICATE, "lane": 1 }],
            "library": [FILLER, FILLER, FILLER, FILLER],
        },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
    }));
    let token = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id == RUSH)
        .map(|card| card.id)
        .expect("a Rush Token card in p1's hand");

    s.trade(&token);

    // R11: the token card left the hand by no draw and no play, so it ceased to exist; the Trade's draw
    // took one of the deck's four cards.
    s.expect_in_zone(&token, "gone");
    assert_eq!(s.pile(P1, "library").len(), 3);
    let reported: Vec<_> = shuffled_in(&s)
        .into_iter()
        .filter(|(_, id, _)| *id == token)
        .collect();
    assert!(
        reported.is_empty(),
        "the Rush Token card ceased to exist (R11) and is in no deck, but the engine reported it \
         shuffled in: {reported:?}"
    );
}
