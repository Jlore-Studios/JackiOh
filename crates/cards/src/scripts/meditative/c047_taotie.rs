//! M #47 饕餮 (SPEC §8.8 row 47): (4) Unit, CN, Legendary, 8/8 → 16/16.
//!
//! Base:    "End of turn: Fuse a random enemy permanent into this."
//! Radiant: "End of turn: Fuse a random enemy permanent and a random card from your opponent's deck
//!           into this."
//! Engine (R62): `end_of_turn` with this as the kept target and one ingredient drawn inside `apply`
//! from the match rng (ME-FUSE-RANDOM, MD-C20): the base face draws from the enemy permanents on
//! the field (tops of piles, both rows, face-down cards included, never an Immutable one); the
//! Radiant face then draws a second ingredient from the opponent's library. The fusion is R77 and
//! R102's: this keeps its instance, zone, damage and Radiant flag; stats are summed; keywords, tags
//! and texts joined; the eaten card ceases to exist with no Death. With no enemy permanent, nothing
//! is fused; an Immutable 饕餮 eats nothing (R23).

use jackioh_engine::effects::fuse_cards;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-047";

fn taotie(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let id = me.id.clone();
            let mut fusions = vec![fuse_cards(json_as(json!({
                "targetInstanceId": id,
                "randomIngredient": { "side": "enemy", "zones": ["field"] },
            })))];
            if radiant {
                fusions.push(fuse_cards(json_as(json!({
                    "targetInstanceId": id,
                    "randomIngredient": { "side": "enemy", "zones": ["library"] },
                }))));
            }
            fusions
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: taotie(false),
        radiant: taotie(true),
    }
}

// M #47 饕餮 — SPEC §8.8 row 47, BUILD M10 row M 47: "At your end of turn only, one random enemy
// permanent (tops of piles, both rows, face-down ones included, never an Immutable one) is fused
// into it (MD-C20) … with no enemy permanent, nothing; an Immutable 饕餮 eats nothing (R23); the
// fused definition survives a JSON round trip (R468); radiant 16/16 and also a random non-Immutable
// card of the opponent's deck".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TAOTIE: &str = "meditative-047";
    const BODY: &str = "core-012"; // Duplicating Felinors 3/4, the meal.
    const FILLER: &str = "core-005";

    fn meal(n: i32) -> Value {
        json!([{ "def": BODY, "lane": n }])
    }

    mod m47_taotie {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1020_at_your_end_of_turn_a_random_enemy_permanent_is_fused_into_this() {
                let mut s = scenario(json!({
                    "seed": "taotie-base",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "field": [{ "def": BODY, "lane": 1 }], "library": [FILLER] },
                }));
                let before = s.unit(P1, 1).expect("taotie");

                s.end_turn();

                // The meal is gone with no Death, and 饕餮 summed its stats (R77): 8 + 3 attack.
                assert!(s.unit(P2, 1).is_none());
                let after = s.unit(P1, 1).expect("taotie");
                assert_eq!(after.id, before.id);
                s.expect_stats(&after, json!({ "attack": 11, "health": 12 }));
            }

            #[test]
            fn r1020_with_no_enemy_permanent_nothing_is_fused() {
                let mut s = scenario(json!({
                    "seed": "taotie-none",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "library": [FILLER] },
                }));

                s.end_turn();

                let after = s.unit(P1, 1).expect("taotie");
                s.expect_stats(&after, json!({ "attack": 8, "health": 8 }));
            }

            #[test]
            fn r468_the_fused_definition_survives_json() {
                let mut s = scenario(json!({
                    "seed": "taotie-json",
                    "p1": { "field": [{ "def": TAOTIE, "lane": 1 }], "library": [FILLER] },
                    "p2": { "field": meal(1), "library": [FILLER] },
                }));

                s.end_turn();

                let after = s.unit(P1, 1).expect("taotie");
                let round_tripped: GameState =
                    serde_json::from_value(serde_json::to_value(s.state()).expect("state is JSON"))
                        .expect("JSON is a state");
                let same = find_instance(&round_tripped, &after.id).expect("the fused card");
                // The fusion's transient definition travelled with it: still one fused card with
                // the summed stats, not two cards and not the printed one.
                assert_eq!(same.def_id, after.def_id);
                assert_ne!(same.def_id, TAOTIE);
                let view = unit_view(&round_tripped, same);
                assert_eq!(view.attack, 11);
                assert!(s.unit(P2, 1).is_none());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1020_radiant_also_fuses_a_random_card_of_the_opponents_deck() {
                let mut s = scenario(json!({
                    "seed": "taotie-radiant",
                    "p1": { "field": [{ "def": TAOTIE, "radiant": true, "lane": 1 }], "library": [FILLER] },
                    "p2": { "field": meal(1), "library": [BODY, BODY, BODY] },
                }));
                let before = s.unit(P1, 1).expect("taotie");

                s.end_turn();

                // Both meals went in: the field's and one of the deck's (the turn's draw took
                // the third).
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(s.pile(P2, "library").len(), 1);
                let after = s.unit(P1, 1).expect("taotie");
                assert_eq!(after.id, before.id);
                // 16 + 3 (field) + 3 (deck): both fusions summed their stats (R77).
                s.expect_stats(&after, json!({ "attack": 22, "health": 24 }));
            }
        }
    }
}
