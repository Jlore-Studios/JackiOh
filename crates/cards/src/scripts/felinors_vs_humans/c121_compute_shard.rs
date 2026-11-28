//! SPEC §8.14 FvH #121 Plagiarized: Compute Shard, a frozen copy of IA #194 Compute Shard (R2195003, FV4), a
//! card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Compute Shard`, tagged Plagiarized only, and a token it makes stays the IA token. Its change
//! from IA #194: its Start of turn is an End of turn: the hook is `end_of_turn`, not `start_of_turn`. The IA
//! file's notes, as they stood:
//!
//! IA #194 Compute Shard (SPEC §8.13 row 194), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Start of turn: The most expensive card in your hand costs ({discount}) less."
//!   Radiant: "Start of turn: The most expensive card in your hand costs ({discount}) less."
//!
//! Engine: a `start_of_turn` hook (its controller's, R68): the dearest hand card by `cost_now`
//! takes a lasting `costMod` of the declared `discount` (R4593002). Tied dearest cards are drawn
//! uniformly through the match rng, a single candidate drawing none; at (0) or with an empty
//! hand, nothing happens. Built by C9 (#4593).

use jackioh_engine::effects::set_cost_mod;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-121";

fn shard() -> Script {
    Script {
        // FV4: IA #194's Start of turn is this card's End of turn (its controller's, R68).
        end_of_turn: Some(hook(|ctx| {
            let hand = zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand);
            let dearest = hand
                .iter()
                .map(|card| cost_now(ctx.state, card))
                .max()
                .unwrap_or(0);
            if dearest <= 0 {
                return vec![];
            }
            let tied: Vec<CardInstance> = hand
                .into_iter()
                .filter(|card| cost_now(ctx.state, card) == dearest)
                .collect();
            // R4593002: one candidate draws no rng.
            let picked = if tied.len() == 1 {
                tied[0].id.clone()
            } else {
                let Some(picked) = ctx.rng.pick(&tied).cloned() else {
                    return vec![];
                };
                picked.id
            };
            let discount = param(&*ctx, "discount");
            vec![set_cost_mod(json_as(json!({
                "target": { "of": "instance", "instanceId": picked },
                "amount": -discount,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's (2) is its declared `discount`.
    let radiant = shard();
    CardScripts {
        base: shard(),
        radiant,
    }
}

// FvH #121 Plagiarized: Compute Shard — SPEC §8.14 row 121; IA #194's tests (SPEC §8.13 row 194, R4593002),
// on this card.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.
    const CHEAP: &str = "stand-in (2) 2/2";
    const DEAR: &str = "stand-in (4) 4/4";
    const DEAR_TWIN: &str = "stand-in (4) 4/5";
    const FREE: &str = "stand-in (0) 1/1";

    /// p1's turn ends once: the Shard's End of turn has run, and no turn of p1's has started.
    fn ended(radiant: bool, hand: Vec<Value>, seed: &str) -> Scenario {
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

    fn cost_of(s: &Scenario, def_id: &str) -> i32 {
        let card = s
            .hand(P1)
            .into_iter()
            .find(|held| held.def_id == def_id)
            .unwrap_or_else(|| panic!("no {def_id} in hand"));
        effective_cost(s.state(), &card, Default::default())
    }

    mod base {
        use super::*;

        #[test]
        fn r4593002_the_dearest_hand_card_costs_1_less_at_the_end_of_its_controllers_turn() {
            let s = ended(false, vec![json!(CHEAP), json!(DEAR)], "shard-base");
            assert_eq!(cost_of(&s, DEAR), 3);
            assert_eq!(cost_of(&s, CHEAP), 2);
        }

        #[test]
        fn fv4_nothing_happens_at_the_start_of_its_controllers_turn() {
            let mut s = ended(false, vec![json!(CHEAP), json!(DEAR)], "shard-start");
            s.end_turn();
            assert_eq!(s.state().active, P1, "p1's turn has started");
            assert_eq!(cost_of(&s, DEAR), 3, "discounted once, at the end of p1's turn");
            assert_eq!(cost_of(&s, CHEAP), 2);
        }

        #[test]
        fn r4593002_a_tie_on_a_fixed_seed_discounts_exactly_one() {
            let s = ended(false, vec![json!(DEAR), json!(DEAR_TWIN)], "shard-tie");
            let discounted = [cost_of(&s, DEAR), cost_of(&s, DEAR_TWIN)]
                .into_iter()
                .filter(|cost| *cost == 3)
                .count();
            assert_eq!(discounted, 1);
        }

        #[test]
        fn r4593002_at_0_nothing_happens() {
            let before = scenario(json!({
                "seed": "shard-free",
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FREE],
                    "library": crate::filler(4),
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            let cursor = before.state().rng_cursor;
            let mut s = before;
            s.end_turn();
            s.end_turn();
            assert_eq!(cost_of(&s, FREE), 0);
            assert_eq!(s.state().rng_cursor, cursor, "no draw was made");
            let mut empty = scenario(json!({
                "seed": "shard-empty",
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [],
                    "library": crate::filler(4),
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            let cursor = empty.state().rng_cursor;
            empty.end_turn();
            empty.end_turn();
            assert_eq!(empty.state().rng_cursor, cursor, "no draw was made");
            // Only the turn's own draw arrived, undiscounted.
            assert_eq!(empty.hand(P1).len(), 1);
            assert!(
                empty.hand(P1).iter().all(|card| card.cost_mod == 0),
                "nothing is discounted"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593002_the_dearest_hand_card_costs_2_less_at_the_end_of_its_controllers_turn() {
            let s = ended(true, vec![json!(CHEAP), json!(DEAR)], "shard-radiant");
            assert_eq!(cost_of(&s, DEAR), 2);
            assert_eq!(cost_of(&s, CHEAP), 2);
        }
    }
}
