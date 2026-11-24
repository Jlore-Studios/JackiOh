//! SPEC §8.13 IA #123 Cost Injection ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "The cards in your opponent's hand cost (1) more during their next turn."
//!   Radiant: "The cards in your opponent's hand cost (2) more during their next turn."
//!
//! T-AI-7 Alignment Tax's price rule turned outward (R4590004): it holds only during the opponent's
//! next turn, reaching every card they play then, including cards drawn during it (R48).

use jackioh_engine::effects::add_cost_rule;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-123";

/// "(1) more", Radiant "(2) more". IA #123 declares no params, so the hike lives here;
/// `tests/cross/params.rs` lists it as a fixed price (R1748003).
const HIKE: i32 = 1;
const RADIANT_HIKE: i32 = 2;

fn injection(hike: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_cost_rule(json_as(json!({
                "player": "enemy",
                "rule": { "amount": hike },
                "lasts": "theirNextTurn",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: injection(HIKE),
        radiant: injection(RADIANT_HIKE),
    }
}

// IA #123 Cost Injection — SPEC §8.13 row 123 (R48, R4590004).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "cost-injection",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "hand": [VANILLA, FILLER], "library": crate::filler(3) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4590004_the_opponent_s_hand_costs_1_more_during_their_next_turn() {
            let mut s = board(false);
            s.play(ID, json!({}));
            s.end_turn();
            let vanilla = s.card(VANILLA).clone();
            assert_eq!(cost_of(&s, &vanilla), 2, "the hike holds on their turn");
            s.end_turn();
            let vanilla = s.card(VANILLA).clone();
            assert_eq!(cost_of(&s, &vanilla), 1, "gone at its cleanup");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590004_the_opponent_s_hand_costs_2_more_during_their_next_turn() {
            let mut s = board(true);
            s.play(ID, json!({}));
            s.end_turn();
            let vanilla = s.card(VANILLA).clone();
            assert_eq!(cost_of(&s, &vanilla), 3, "the Radiant hike holds on their turn");
            s.end_turn();
            let vanilla = s.card(VANILLA).clone();
            assert_eq!(cost_of(&s, &vanilla), 1, "gone at its cleanup");
        }
    }
}
