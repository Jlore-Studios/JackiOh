//! Audit (Stinkyscape+ cards, second half): each test asserts a card's printed text or its ruling and
//! fails on the bug it proves.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

const BARD: &str = "stinkyscapeplus-102"; // Cosmic Bard
const OMURA: &str = "stinkyscapeplus-125"; // Omura
const PSYCHIC_SCREAM: &str = "stinkyscape-012-3"; // Psychic Scream: its Radiant face raises what it shuffles away
const FILLER: &str = "core-005"; // Stockpile: a Spell to fill a library
const FREE_SPELL: &str = "stand-in (0) Spell";
const FREE_UNIT: &str = "stand-in (0) 1/1";
const ONE_UNIT: &str = "stand-in (1) 1/1";

fn game(setup: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(setup)
}

/// The price `def`'s card has in p1's hand.
fn price_in_hand(s: &Scenario, def: &str) -> i32 {
    let card = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id == def)
        .unwrap_or_else(|| panic!("{def} is not in p1's hand"));
    effective_cost(s.state(), &card, Default::default())
}

/// S+ #102 Cosmic Bard, Radiant: "Combo: Draw 2 Units and set both their costs to the cheapest drawn."
/// R3568001: "... takes the least, and sets every drawn Unit that is in the hand to it with a
/// `costOverride`; a cost change a card carries is taken off the override, so each card ends at that one
/// price." A drawn Unit that carries a cost raise larger than the cheapest price (R78: `costMod` persists
/// in every zone) keeps part of the raise: the script clamps `cheapest - cost_mod` at 0.
#[test]
fn r3568001_radiant_cosmic_bard_sets_every_drawn_unit_to_the_cheapest_price() {
    let mut s = game(json!({
        "p1": {
            "mana": 10,
            "hand": [FREE_SPELL, { "def": BARD, "radiant": true }],
            // The top Unit is a (0) that costs (2) more, as a Radiant Psychic Scream leaves it: price 2.
            // The next is a plain (1): the cheapest drawn is 1.
            "library": [{ "def": FREE_UNIT, "costMod": 2 }, ONE_UNIT, FILLER, FILLER],
        },
        "p2": { "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    s.play(FREE_SPELL, json!({}));
    s.play(BARD, json!({ "zone": 1 }));
    assert_eq!(price_in_hand(&s, ONE_UNIT), 1, "the cheapest drawn stays at its price");
    assert_eq!(
        price_in_hand(&s, FREE_UNIT),
        1,
        "R3568001: each drawn Unit ends at the cheapest drawn price, the raise it carried taken off"
    );
}

/// The same bug reached by play alone: p2's Radiant Psychic Scream ("Shuffle all permanents into your
/// opponent's deck. Each costs (2) more") puts a (0) Unit into p1's deck at price 2, and p1's Radiant
/// Cosmic Bard then draws it with a (1) Unit.
#[test]
fn r3568001_radiant_cosmic_bard_after_a_radiant_psychic_scream() {
    // The shuffle and p1's turn draw are random: take the first seed whose draw leaves both Units in the deck.
    for seed in 0..64 {
        let mut s = game(json!({
            "seed": format!("audit-bard-scream-{seed}"),
            "active": "p2",
            "p1": {
                "hand": [FREE_SPELL, { "def": BARD, "radiant": true }],
                "library": [FILLER, FILLER, ONE_UNIT, FILLER, FILLER, FILLER],
            },
            "p2": {
                "hand": [{ "def": PSYCHIC_SCREAM, "radiant": true }, FILLER],
                "field": [{ "def": FREE_UNIT, "lane": 1 }],
                "library": [FILLER, FILLER, FILLER, FILLER],
            },
        }));
        s.play(PSYCHIC_SCREAM, json!({}));
        // p2's turn ends by itself once nothing but ending it is left (§2.5); else end it.
        if s.state().active == P2 {
            s.end_turn();
        }
        assert_eq!(s.state().active, P1);
        let in_deck = |def: &str| s.pile(P1, "library").iter().any(|card| card.def_id == def);
        if !in_deck(FREE_UNIT) || !in_deck(ONE_UNIT) {
            continue;
        }
        s.play(FREE_SPELL, json!({}));
        s.play(BARD, json!({ "zone": 1 }));
        assert_eq!(price_in_hand(&s, ONE_UNIT), 1, "the cheapest drawn stays at its price");
        assert_eq!(
            price_in_hand(&s, FREE_UNIT),
            1,
            "R3568001: each drawn Unit ends at the cheapest drawn price (seed {seed})"
        );
        return;
    }
    panic!("no seed left both Units in p1's deck");
}

/// S+ #125 Omura: "Activate: Deal 12 Pierce Trample damage. Has a 3-turn cooldown." R3560008: "Sei's and
/// Omura's \"Deal N …\" aims at one declared enemy Unit". Sei marks that target as needed (R703: "the play
/// is refused and `legalActions` never offers it"), and the set's other Activates that aim at an enemy
/// Unit are "not offered while the opponent has no Unit" (§8.12 rows 143 to 148). Omura's is offered with
/// nothing to aim at, and using it starts the 3-turn cooldown for no damage.
#[test]
fn r3560008_omura_activate_is_not_offered_with_no_enemy_unit_to_aim_at() {
    let mut s = game(json!({
        "p1": { "field": [{ "def": OMURA, "lane": 1 }], "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
        "p2": { "mana": 10, "hand": ["stand-in (1) 1/10"], "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    let offered = s.activations_of(OMURA);
    if !offered.is_empty() {
        // What the offer costs: the use fizzles, and with an enemy Unit there on Omura's next turn the
        // Activate is still on cooldown.
        s.activate(OMURA, json!({ "ability": "omura" }));
        s.end_turn();
        s.play("stand-in (1) 1/10", json!({ "zone": 1 }));
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(s.unit_at(P2, 1).damage, 0, "the fizzled use dealt nothing");
        assert!(s.activations_of(OMURA).is_empty(), "and the cooldown it started is still running");
    }
    assert!(
        offered.is_empty(),
        "R3560008, R703: with no enemy Unit to aim at, Omura's Activate is not offered; got {offered:?}"
    );
}
