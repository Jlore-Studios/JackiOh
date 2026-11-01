//! S #34 Drone Core VIII (SPEC §8.10 row 34): (6) Spell, Drone Core, Legendary, a card of the
//! Stinkyscape set.
//!   Base:    "Float reduces cost.\nFill your board with Drone Interceptors."
//!   Radiant: "Float reduces cost.\nFill your board with Radiant Drone Interceptors."
//! Engine: "Float reduces cost" (§6.2, R1563000) is the engine's `float_reduces_cost`, a hand trigger
//!   (R464), a deck trigger too through v0.5.9f (R2657002): each Float of its owner's lowers its cost by that much, a `costMod` kept until a
//!   graveyard or exile (R766), so at (6), past `MAX_MANA`, it is played only so or with mana gained. The
//!   fill is §7's `fill_board` with the Drone Interceptor: one into each of the caster's empty unit zones
//!   (R64), then, as it flies, one into each empty backrow zone, where it stands (R2248000); each
//!   Interceptor draws its own combat keyword as it arrives (R781004). Radiant: Radiant Interceptors.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-034";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

/// One face. `deck` adds the Float deck trigger it had through v0.5.9f (R2657002).
fn face(radiant: bool, deck: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![fill_board(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))]
        })),
        hand_triggers: vec![float_reduces_cost("drone-core-viii-float-hand")],
        deck_triggers: if deck {
            vec![float_reduces_cost("drone-core-viii-float-deck")]
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

// S #34 Drone Core VIII — SPEC §8.10 row 34, BUILD M12 row S 34: "a Float of 2 at its owner's end of turn
// makes it cost (4) in hand, never in the deck (R2657002), the opponent's Float nothing (R1563000); played, it summons an
// Interceptor into every empty unit zone of its caster, then every empty backrow zone (R2248000); radiant
// Radiant ones".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-034";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn price_of(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), CostOptions::default())
    }

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

    fn cast(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CORE, json!({}));
        s
    }

    fn row(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    fn backrow(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant)))
            .collect()
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
        assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        s.state_mut().players.p2.mana.current = 4;
        s.end_turn();
        assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        assert_eq!(price_of(&s, &hand), 6);
    }

    #[test]
    fn r64_r2248000_fills_every_empty_unit_zone_then_every_empty_backrow_zone() {
        let s = cast(false, json!([{ "def": VANILLA, "lane": 2 }]));
        let drone = Some((INTERCEPTOR.to_string(), false));
        let vanilla = Some((VANILLA.to_string(), false));
        assert_eq!(row(&s), vec![drone.clone(), vanilla, drone.clone(), drone.clone(), drone.clone()]);
        assert_eq!(backrow(&s), vec![drone; 5]);
    }

    #[test]
    fn r2248000_a_full_unit_row_fills_the_backrow() {
        let s = cast(false, json!(vec![VANILLA; 5]));
        assert_eq!(row(&s), vec![Some((VANILLA.to_string(), false)); 5]);
        assert_eq!(backrow(&s), vec![Some((INTERCEPTOR.to_string(), false)); 5]);
    }

    #[test]
    fn radiant_fills_it_with_radiant_ones() {
        let s = cast(true, json!([]));
        assert_eq!(row(&s), vec![Some((INTERCEPTOR.to_string(), true)); 5]);
        assert_eq!(backrow(&s), vec![Some((INTERCEPTOR.to_string(), true)); 5]);
    }
}
