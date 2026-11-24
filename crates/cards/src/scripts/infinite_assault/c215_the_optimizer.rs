//! IA #215 The Optimizer (SPEC §8.13 row 215), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "End of turn: Transform a random card in your hand into a random Infinite Assault
//!            card that costs ({step}) more."
//!   Radiant: "End of turn: Transform two random cards in your hand into random Infinite Assault
//!            cards that cost ({step}) more."
//!
//! Engine: an `end_of_turn` hook (its controller's, R68): one distinct hand card — two on the
//! Radiant face — picked uniformly through the match rng, none drawn when the hand holds no
//! more cards than picks; each `transform_random`s (§6.3) into the set-named pool (R4577000) at
//! its printed cost (R4577006) plus the declared `step`, keeping its face, and stays itself with
//! no such card (R129, R4593010). Built by C9 (#4593).

use jackioh_engine::effects::transform_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-215";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// The base face rewrites one hand card; the Radiant face two.
const BASE_PICKS: i32 = 1;
const RADIANT_PICKS: i32 = 2;

fn optimizer(picks: i32) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let hand = zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand);
            if hand.is_empty() {
                return vec![];
            }
            // R4593010: no more cards than picks takes them all and draws no rng.
            let ids: Vec<String> = if hand.len() as i32 <= picks {
                hand.iter().map(|card| card.id.clone()).collect()
            } else {
                ctx.rng
                    .shuffle(&hand)
                    .into_iter()
                    .take(picks.max(0) as usize)
                    .map(|card| card.id.clone())
                    .collect()
            };
            let step = param(&*ctx, "step");
            ids.into_iter()
                .filter_map(|id| {
                    let card = find_instance(ctx.state, &id)?.clone();
                    let def = def_of(Some(&*ctx.state), &card.def_id);
                    let cost = query_cost(def) + step;
                    Some(transform_random(json_as(json!({
                        "target": { "of": "instance", "instanceId": id },
                        "query": { "set": SET, "cost": cost },
                        "radiant": "keep",
                    }))))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: optimizer(BASE_PICKS),
        radiant: optimizer(RADIANT_PICKS),
    }
}

// IA #215 The Optimizer — SPEC §8.13 row 215 (R4593010).
#[cfg(test)]
mod tests {
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const ONE: &str = "stand-in (1) 4/4";
    const ONE_OTHER: &str = "stand-in (1) 4/5";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn optimized(radiant: bool, hand: Vec<Value>, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": hand,
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.end_turn();
        s
    }

    fn hand_defs(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
    }

    mod base {
        use super::*;

        #[test]
        fn r4593010_a_1_cost_hand_card_becomes_an_infinite_assault_2_cost_card() {
            let s = optimized(false, vec![json!(ONE)], "optimizer-base");
            let hand = hand_defs(&s);
            assert_eq!(hand.len(), 1);
            assert_ne!(hand[0].def_id, ONE);
            let def = def_of(Some(s.state()), &hand[0].def_id);
            assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
            assert_eq!(query_cost(def), 2, "{}", def.id);
            assert!(!def.is_token(), "{}", def.id);
            assert!(!hand[0].radiant, "the face is kept");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593010_two_1_cost_hand_cards_become_infinite_assault_2_cost_cards() {
            let s = optimized(true, vec![json!(ONE), json!(ONE_OTHER)], "optimizer-radiant");
            let hand = hand_defs(&s);
            assert_eq!(hand.len(), 2);
            for card in &hand {
                assert_ne!(card.def_id, ONE);
                assert_ne!(card.def_id, ONE_OTHER);
                let def = def_of(Some(s.state()), &card.def_id);
                assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
                assert_eq!(query_cost(def), 2, "{}", def.id);
                assert!(!def.is_token(), "{}", def.id);
            }
        }
    }
}
