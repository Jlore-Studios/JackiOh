//! SPEC §8.13 IA #127 Bad Prompt ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Transform a random card in your opponent's hand into a random Common Infinite Assault card."
//!   Radiant: "Transform two random cards in your opponent's hand into random Common Infinite Assault cards."
//!
//! The Cry lists the enemy hand and draws one pick per face without replacement (#43 Big Felinor's
//! shape, R60), each replaced by a random Common card of the set (R1846001, R4577000: a pool that
//! names its set keeps to it while held). A shorter hand transforms what it holds; the caster's view
//! never names the new cards (R33, R97).

use jackioh_engine::effects::transform_random;
use jackioh_engine::prelude::*;
use jackioh_engine::zones::OffFieldZone;

pub const ID: &str = "infiniteassault-127";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// How many random cards of the enemy hand transform: one, Radiant two.
const TRANSFORMS: i32 = 1;
const RADIANT_TRANSFORMS: i32 = 2;

fn bad_prompt(count: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let opponent = opponent_of(ctx.controller);
            let mut left: Vec<String> = zone_cards(ctx.state, opponent, OffFieldZone::Hand)
                .into_iter()
                .map(|card| card.id)
                .collect();
            let mut effects = Vec::new();
            for _ in 0..count {
                if left.is_empty() {
                    break;
                }
                let at = ctx.rng.int(left.len() as i32) as usize;
                let id = left.remove(at);
                effects.push(transform_random(json_as(json!({
                    "instanceId": id,
                    "query": { "set": SET, "rarity": "Common" },
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bad_prompt(TRANSFORMS),
        radiant: bad_prompt(RADIANT_TRANSFORMS),
    }
}

// IA #127 Bad Prompt — SPEC §8.13 row 127.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool, seed: &str, hand: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(3) },
            "p2": { "hand": hand, "library": crate::filler(3) },
        }))
    }

    /// The def ids of p2's hand after the cast, in hand order.
    fn hand_defs(s: &Scenario) -> Vec<String> {
        s.hand(P2).iter().map(|card| card.def_id.clone()).collect()
    }

    /// Every Common Infinite Assault def id the pool may draw.
    fn pool() -> Vec<String> {
        jackioh_engine::catalog::query(&json_as(
            json!({ "set": "Infinite Assault", "rarity": "Common" }),
        ))
        .into_iter()
        .map(|def| def.id.clone())
        .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn transforms_one_random_card_of_the_enemy_hand_into_a_common_infinite_assault_card() {
            let before = [FILLER, FILLER, FILLER];
            let mut s = board(false, "bad-prompt", json!(before));
            s.play(ID, json!({}));
            let after = hand_defs(&s);
            assert_eq!(after.len(), 3);
            assert_eq!(
                after.iter().filter(|def| *def == FILLER).count(),
                2,
                "exactly one card transformed: {after:?}"
            );
            let pool = pool();
            for def in after.iter().filter(|def| *def != FILLER) {
                assert!(pool.contains(def), "{def} is a Common Infinite Assault card");
            }
        }

        #[test]
        fn the_caster_s_view_never_shows_the_new_def_id() {
            let mut s = board(false, "bad-prompt-hidden", json!([FILLER]));
            s.play(ID, json!({}));
            let after = hand_defs(&s);
            assert_ne!(after, vec![FILLER.to_string()], "it transformed");
            let mine = serde_json::to_string(&s.view(P1)).unwrap();
            for def in after {
                assert!(!mine.contains(&def), "p1's view names no new card");
            }
        }

        #[test]
        fn an_empty_hand_transforms_nothing() {
            let mut s = board(false, "bad-prompt-empty", json!([]));
            s.play(ID, json!({}));
            assert!(s.hand(P2).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn transforms_two_random_cards_of_the_enemy_hand() {
            let mut s = board(true, "bad-prompt-radiant", json!([FILLER, FILLER, FILLER]));
            s.play(ID, json!({}));
            let after = hand_defs(&s);
            assert_eq!(after.len(), 3);
            assert_eq!(
                after.iter().filter(|def| *def == FILLER).count(),
                1,
                "exactly two cards transformed: {after:?}"
            );
            let pool = pool();
            for def in after.iter().filter(|def| *def != FILLER) {
                assert!(pool.contains(def), "{def} is a Common Infinite Assault card");
            }
        }
    }
}
