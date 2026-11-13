//! SPEC §8.12 S+ #T-AI-6 Abacus Optimizer, a card of the Stinkyscape+ set (R1420, R3535000). Built by S25 (#3571).
//!   Base:    "The leftmost and rightmost cards in your hand cost (1) less."
//!   Radiant: "The leftmost and rightmost cards in your hand cost (2) less."
//! Engine: a (1) Spell, CN, AI, Token (no printed rarity, R482). R3571002: the first and last cards of its
//!   caster's hand as the Spell resolves (it is out of the hand by then, R1563001) each take a `costMod` of
//!   −1, Radiant −2 (R78, R65: a cost never goes below (0), and an X-cost card is unaffected); a one-card hand's
//!   card takes it once, and an empty hand takes nothing. Declares no numbers (R482: its numbers are the script's).

use jackioh_engine::effects::set_cost_mod;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-ai-06";

/// "cost (1) less".
const DISCOUNT: i32 = 1;

/// "cost (2) less".
const RADIANT_DISCOUNT: i32 = 2;

/// R3571002: the first and last cards of its caster's hand as it resolves (the Spell is out of it,
/// R1563001); a one-card hand's card once.
fn ends_of_hand(ctx: &EffectContext<'_>) -> Vec<String> {
    let hand = zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand);
    let mut ends: Vec<String> = hand
        .first()
        .into_iter()
        .chain(hand.last())
        .map(|card| card.id.clone())
        .collect();
    ends.dedup();
    ends
}

fn face(discount: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            ends_of_hand(ctx)
                .into_iter()
                .map(|instance_id| {
                    set_cost_mod(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                        "amount": -discount,
                        "inHandOnly": true,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(DISCOUNT),
        radiant: face(RADIANT_DISCOUNT),
    }
}

// S+ #T-AI-6 Abacus Optimizer — SPEC §8.12 row T-AI-6 (R3571002; R65, R78, R1563001).
#[cfg(test)]
mod tests {
    use super::{DISCOUNT, ID, RADIANT_DISCOUNT};
    use crate::weapon_board;
    use jackioh_engine::mana::effective_cost;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SMALL: &str = "stand-in (3) 1/1";
    const MIDDLE: &str = "stand-in (3) 2/2";
    const LARGE: &str = "stand-in (3) 3/3";

    fn discount(radiant: bool) -> i32 {
        if radiant { RADIANT_DISCOUNT } else { DISCOUNT }
    }

    /// The Optimizer first in p1's hand with `rest` behind it, played.
    fn cast(radiant: bool, rest: &[&str]) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(rest.iter().map(|id| json!(id)));
        let mut s = weapon_board("s-t-ai-06", json!({ "hand": hand, "mana": 10 }), json!({}));
        s.play(ID, json!({}));
        s
    }

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        effective_cost(s.state(), card, Default::default())
    }

    #[test]
    fn r3571002_the_leftmost_and_rightmost_cards_cost_less() {
        for radiant in [false, true] {
            let d = discount(radiant);
            let s = cast(radiant, &[SMALL, MIDDLE, LARGE]);
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 3, "radiant {radiant}");
            for (at, mod_) in [(0, -d), (1, 0), (2, -d)] {
                assert_eq!(hand[at].cost_mod, mod_, "radiant {radiant}: card {at}");
                assert_eq!(cost_of(&s, &hand[at]), 3 + mod_, "radiant {radiant}: card {at}");
            }
        }
    }

    #[test]
    fn r3571002_a_one_card_hand_takes_it_once() {
        for radiant in [false, true] {
            let s = cast(radiant, &[LARGE]);
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 1, "radiant {radiant}");
            assert_eq!(
                hand[0].cost_mod,
                -discount(radiant),
                "radiant {radiant}: once, not twice"
            );
        }
    }

    #[test]
    fn r3571002_an_empty_hand_takes_nothing() {
        for radiant in [false, true] {
            let s = cast(radiant, &[]);
            assert!(s.hand(P1).is_empty(), "radiant {radiant}");
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::CostChanged { .. })),
                "radiant {radiant}"
            );
        }
    }

    #[test]
    fn r3571002_never_below_zero() {
        for radiant in [false, true] {
            let s = cast(radiant, &["stand-in (0) Spell", MIDDLE]);
            let hand = s.hand(P1);
            assert_eq!(hand[0].cost_mod, -discount(radiant), "radiant {radiant}");
            assert_eq!(
                cost_of(&s, &hand[0]),
                0,
                "radiant {radiant}: a (0) card stays (0)"
            );
        }
    }
}
