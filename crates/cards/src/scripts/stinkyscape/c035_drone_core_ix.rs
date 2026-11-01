//! S #35 Drone Core IX (SPEC §8.10 row 35): (6) Spell, Drone Core, Legendary, a card of the
//! Stinkyscape set.
//!   Base:    "Float reduces cost.\nSummon {corvettes|Drone Corvette|Drone Corvettes}." — corvettes 3
//!   Radiant: "Float reduces cost.\nSummon {corvettes|Radiant Drone Corvette|Radiant Drone Corvettes}."
//!            — corvettes 3
//! Engine: "Float reduces cost" as S #34 (R1563000): a hand trigger on its owner's Float (a deck trigger too through v0.5.9f, R2657002), a
//!   `costMod` kept until a graveyard or exile (R766). Then one summon per Corvette, each into the caster's
//!   leftmost open unit zone in turn (R64), then the backrow's once the unit row is full, where each stands
//!   (R2248000), no Cry (R1), as many as fit; the count is the declared
//!   `corvettes` (R386). Each Corvette draws its own combat keywords as it arrives (R781004). Radiant:
//!   summoned Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-035";

/// The Drone Corvette token (§8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

/// One face. `deck` adds the Float deck trigger it had through v0.5.9f (R2657002).
fn face(radiant: bool, deck: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            (0..param(&*ctx, "corvettes"))
                .map(|_| summon(json_as(json!({ "defId": CORVETTE, "radiant": radiant }))))
                .collect()
        })),
        hand_triggers: vec![float_reduces_cost("drone-core-ix-float-hand")],
        deck_triggers: if deck {
            vec![float_reduces_cost("drone-core-ix-float-deck")]
        } else {
            vec![]
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false, false),
        radiant: face(true, false),
    }
}

/// R767000: through v0.5.9f a Float lowered its cost in the deck too, before R2657002; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    CardScripts {
        base: face(false, true),
        radiant: face(true, true),
    }
}

// S #35 Drone Core IX — SPEC §8.10 row 35, BUILD M12 row S 35: "Float lowers its cost as S 34's; summons
// 3 Drone Corvettes, into the backrow once the unit row is full (R2248000); corvettes reads through `param()`; radiant 3 Radiant ones".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-035";
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn price_of(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), CostOptions::default())
    }

    /// p1 holds one Drone Core IX and has one in the deck, with `mana` left to Float.
    fn floating(mana: i32, radiant: bool) -> (Scenario, String, String) {
        let s = scenario(json!({
            "p1": {
                "hand": [{ "def": CORE, "radiant": radiant }, FILLER],
                "library": [{ "def": CORE, "radiant": radiant }, FILLER, FILLER],
                "mana": mana,
            },
            "p2": { "hand": [FILLER], "library": filler(2) },
        }));
        let hand = s.hand(P1).into_iter().find(|card| card.def_id == CORE).map(|card| card.id).unwrap_or_default();
        let deck = s.pile(P1, "library").into_iter().find(|card| card.def_id == CORE).map(|card| card.id).unwrap_or_default();
        (s, hand, deck)
    }

    fn setup(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// p1's unit row, lane 1 to 5: each zone's definition and face, or `None` when it is empty.
    fn row(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    /// p1's backrow, lane 1 to 5, read as `row` reads the unit row.
    fn backrow(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant)))
            .collect()
    }

    fn corvette(radiant: bool) -> Option<(String, bool)> {
        Some((CORVETTE.to_string(), radiant))
    }

    #[test]
    fn r1563000_r2657002_a_float_of_2_makes_it_cost_two_less_in_hand_never_in_the_deck() {
        for radiant in [false, true] {
            let (mut s, hand, deck) = floating(2, radiant);
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, -2, "radiant {radiant}");
            assert_eq!(s.card(deck.as_str()).cost_mod, 0, "radiant {radiant}");
            assert_eq!(price_of(&s, &hand), 4, "radiant {radiant}");
        }
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        let (mut s, hand, _) = floating(0, false);
        s.end_turn();
        s.state_mut().players.p2.mana.current = 4;
        s.end_turn();
        assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        assert_eq!(price_of(&s, &hand), 6);
    }

    #[test]
    fn r64_r2248000_summons_three_corvettes_the_unit_row_first_then_the_backrow() {
        let mut s = setup(false, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![corvette(false), corvette(false), corvette(false), None, None]);
        assert_eq!(backrow(&s), vec![None; 5]);

        let mut s = setup(false, json!(vec![VANILLA; 4]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s)[4], corvette(false), "one zone, one Corvette");
        assert_eq!(backrow(&s), vec![corvette(false), corvette(false), None, None, None]);
    }

    #[test]
    fn corvettes_reads_through_param() {
        let mut s = setup(false, json!([]));
        set_param(s.card_mut(CORE), "corvettes", 4);
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![corvette(false), corvette(false), corvette(false), corvette(false), None]);
    }

    #[test]
    fn radiant_summons_three_radiant_ones() {
        let mut s = setup(true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![corvette(true), corvette(true), corvette(true), None, None]);
    }
}
