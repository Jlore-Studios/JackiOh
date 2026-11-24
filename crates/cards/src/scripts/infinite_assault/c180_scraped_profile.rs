//! SPEC §8.13 IA #180 Scraped Profile ((1) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent plays a card: Add a copy of it to your hand."
//!   Radiant: "Reveals when your opponent plays a card: Add two copies of it to your hand."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-180";

fn profile(copies: usize) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("scraped-profile", &[GameEventType::CardPlayed], move |_ctx, event| {
                let GameEvent::CardPlayed { instance_id, .. } = event else {
                    return vec![];
                };
                let mut effects = Vec::new();
                for _ in 0..copies {
                    effects.push(add_to_hand(json_as(json!({ "copyOf": instance_id }))));
                }
                effects
            })
            .with_when(|ctx, event| match event {
                GameEvent::CardPlayed { player, def_id, .. } => {
                    *player != ctx.controller && {
                        let def = def_of(Some(&*ctx.state), def_id);
                        def.type_ != CardType::Trap && def.type_ != CardType::FieldTrap
                    }
                }
                _ => false,
            }),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: profile(1),
        radiant: profile(2),
    }
}

// IA #180 Scraped Profile — SPEC §8.13 row 180.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "core-016"; // Hit Job

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 1 }],
                "mana": 10,
            },
            "p2": { "hand": [SPELL], "mana": 10 },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn reveals_and_adds_one_copy() {
            let mut s = board(false);
            s.play(SPELL, json!({}));
            s.expect_in_zone(ID, "graveyard");
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 1);
            assert_eq!(hand[0].def_id, SPELL);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn reveals_and_adds_two_copies() {
            let mut s = board(true);
            s.play(SPELL, json!({}));
            s.expect_in_zone(ID, "graveyard");
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 2);
            assert_eq!(hand[0].def_id, SPELL);
            assert_eq!(hand[1].def_id, SPELL);
        }
    }
}
