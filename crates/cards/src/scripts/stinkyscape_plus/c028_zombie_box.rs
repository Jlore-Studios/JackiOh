//! SPEC §8.12 S+ #28 Zombie-Box (Spell, cost 0, Rare), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Nerf your hand. Summon a Dormant 1 Gargantuar."
//!   Radiant: "Nerf your hand. Summon a Radiant Gargantuar."
//!
//! Cry, in the text's order: `degrade` over the hand (R386, R1320), one Nerf for each card in it (this
//! Spell is already resolving, so it is not in the hand and is not Nerfed; a card none can change is left
//! alone without a draw), then `summon` of S+ #27 Gargantuar into the leftmost open unit zone (R64, no
//! Cry, R1); it fizzles with no zone. The base face's Gargantuar enters Dormant 1: asleep, filling its
//! zone, until the start of its controller's next turn (R1839001), which `dormantTurns` asks of the
//! summon (R3541002) because the Gargantuar's own text prints no Dormant. The Radiant face prints no
//! Dormant, so its Radiant Gargantuar (24/24) is awake as it lands, summoning sick as any summon is.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-028";

/// S+ #27 Gargantuar.
const GARGANTUAR: &str = "stinkyscapeplus-027"; // Gargantuar

/// "Dormant 1": it awakens at the start of its controller's first turn from now (R1839001).
const DORMANT_TURNS: i32 = 1;

/// "Nerf your hand": one Nerf for each card in the hand.
fn nerf_hand() -> Effect {
    degrade(json_as(json!({ "scope": { "side": "self", "zones": ["hand"] } })))
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    nerf_hand(),
                    summon(json_as(
                        json!({ "defId": GARGANTUAR, "dormantTurns": DORMANT_TURNS }),
                    )),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    nerf_hand(),
                    summon(json_as(json!({ "defId": GARGANTUAR, "radiant": true }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// S+ #28 Zombie-Box — SPEC §8.12 row 28.
#[cfg(test)]
mod tests {
    use super::{GARGANTUAR, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn board(radiant: bool, hand: usize, field: Vec<Value>) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend((0..hand).map(|_| json!(VANILLA)));
        scenario(json!({
            "p1": { "hand": cards, "field": field, "library": crate::filler(4) },
            "p2": { "hand": [VANILLA], "field": [VANILLA], "library": crate::filler(4) },
        }))
    }

    fn nerfs_of(s: &Scenario, id: &str) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Degraded { instance_id, .. } if instance_id == id))
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r3541002_base_nerfs_hand_once_each_and_summons_dormant_1_gargantuar() {
            let mut s = board(false, 3, vec![]);
            let hand: Vec<String> = s
                .hand(P1)
                .iter()
                .filter(|card| card.def_id == VANILLA)
                .map(|card| card.id.clone())
                .collect();
            assert_eq!(hand.len(), 3);
            s.play(ID, json!({}));
            for id in &hand {
                assert_eq!(nerfs_of(&s, id), 1, "each card of the hand is Nerfed once");
            }
            let gargantuar = s.unit_at(P1, 1);
            assert_eq!(gargantuar.def_id, GARGANTUAR);
            assert!(!gargantuar.radiant);
            assert_eq!(slumber_turns_of(s.card(&gargantuar)), Some(1), "Dormant 1");
            assert!(is_slumbering(s.card(&gargantuar)));
            s.expect_stats(&gargantuar, json!({ "attack": 12, "health": 12 }));
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn the_spell_itself_is_not_nerfed_and_an_empty_hand_is_fine() {
            let mut s = board(false, 0, vec![]);
            s.play(ID, json!({}));
            assert_eq!(
                s.events()
                    .iter()
                    .filter(|e| matches!(e, GameEvent::Degraded { .. }))
                    .count(),
                0
            );
            assert_eq!(s.unit_at(P1, 1).def_id, GARGANTUAR);
        }

        #[test]
        fn the_opponents_hand_is_left_alone() {
            let mut s = board(false, 1, vec![]);
            let theirs = s.hand(P2)[0].id.clone();
            s.play(ID, json!({}));
            assert_eq!(nerfs_of(&s, &theirs), 0);
        }

        #[test]
        fn r3541002_dormant_gargantuar_awakens_on_its_owners_next_turn() {
            let mut s = board(false, 1, vec![]);
            s.play(ID, json!({}));
            let gargantuar = s.unit_at(P1, 1);
            s.expect_refused(|s| s.attack(&gargantuar, "hero"));
            s.end_turn();
            assert!(
                is_slumbering(s.card(&gargantuar)),
                "the opponent's turn does not count"
            );
            s.end_turn();
            assert!(
                !is_slumbering(s.card(&gargantuar)),
                "it awakens at the start of its owner's turn"
            );
            assert_eq!(s.card(&gargantuar).awakened, Some(true));
            s.end_turn();
            s.end_turn();
            // Awake, it strikes like any Unit that has stood a turn.
            s.attack(&gargantuar, "hero");
            s.expect_health(P2, HERO_HEALTH - 12);
        }

        #[test]
        fn a_full_unit_row_summons_nothing() {
            let mut s = board(
                false,
                1,
                vec![
                    json!(VANILLA),
                    json!(VANILLA),
                    json!(VANILLA),
                    json!(VANILLA),
                    json!(VANILLA),
                ],
            );
            s.play(ID, json!({}));
            assert!(
                s.events().iter().all(
                    |event| !matches!(event, GameEvent::Summoned { def_id, .. } if def_id == GARGANTUAR)
                )
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn nerfs_the_hand_and_summons_an_awake_radiant_gargantuar() {
            let mut s = board(true, 2, vec![]);
            let hand: Vec<String> = s
                .hand(P1)
                .iter()
                .filter(|card| card.def_id == VANILLA)
                .map(|card| card.id.clone())
                .collect();
            s.play(ID, json!({}));
            for id in &hand {
                assert_eq!(nerfs_of(&s, id), 1);
            }
            let gargantuar = s.unit_at(P1, 1);
            assert_eq!(gargantuar.def_id, GARGANTUAR);
            assert!(gargantuar.radiant);
            assert!(
                s.card(&gargantuar).slumber.is_none(),
                "the Radiant face prints no Dormant"
            );
            s.expect_stats(&gargantuar, json!({ "attack": 24, "health": 24 }));
            assert!(s.state().pending.is_none());
        }
    }
}
