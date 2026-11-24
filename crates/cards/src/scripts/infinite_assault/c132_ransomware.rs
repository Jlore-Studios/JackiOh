//! SPEC §8.13 IA #132 Ransomware ((2) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent plays a Unit: Freeze it. They lose ({mana}) mana next turn."
//!   Radiant: "Reveals when your opponent plays a Unit: Freeze it. They lose ({mana}) mana next turn."
//!
//! S T-AI-9 Honeypot's shape on an opponent's Unit play (R61, R99): the played Unit is Frozen from
//! where it stands and the opponent's next refresh loses the card's declared `mana` (§6.3 Mana).
//! A Spell play, and the controller's own plays, leave it armed.

use jackioh_engine::effects::{freeze, next_turn_mana};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-132";

/// "They lose ({mana}) mana next turn" — 1 on the base face, 2 Radiant (its declared `mana`).
fn ransomware() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("ransomware", &[GameEventType::CardPlayed], |ctx, event| {
                let GameEvent::CardPlayed { instance_id, .. } = event else {
                    return vec![];
                };
                vec![
                    freeze(json_as(
                        json!({ "target": { "of": "instance", "instanceId": instance_id } }),
                    )),
                    next_turn_mana(json_as(json!({
                        "amount": -param(&*ctx, "mana"),
                        "player": "enemy",
                    }))),
                ]
            })
            .with_when(|ctx, event| {
                let GameEvent::CardPlayed { player, .. } = event else {
                    return false;
                };
                *player != ctx.controller
                    && played_type_of(&*ctx.state, event) == Some(CardType::Unit)
            }),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's 2 is its declared `mana`.
    let base = ransomware();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #132 Ransomware — SPEC §8.13 row 132.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 2 }],
                "library": crate::filler(2),
            },
            "p2": { "hand": [VANILLA, SPELL], "library": crate::filler(2), "mana": 10 },
        }))
    }

    fn frozen(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen.is_some()
    }

    /// p1's turn, then p2's turn with the lowered refresh.
    fn next_refresh(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
    }

    mod base {
        use super::*;

        #[test]
        fn a_unit_play_reveals_it_freezes_the_unit_and_costs_1_next_mana() {
            let mut s = board(false);
            s.play(VANILLA, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "graveyard");
            let unit = s.unit_at(P2, 1);
            assert!(frozen(&s, &unit), "the played Unit is Frozen");
            assert_eq!(s.state().players.p2.mana.next_turn_mod, -1);
            next_refresh(&mut s);
            let max = s.state().players.p2.mana.max;
            s.expect_mana(P2, max - 1);
        }

        #[test]
        fn a_spell_play_leaves_it_armed() {
            let mut s = board(false);
            s.play(SPELL, json!({}));
            assert_eq!(s.card(ID).face_up, Some(false));
        }

        #[test]
        fn your_own_unit_play_leaves_it_armed() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [VANILLA],
                    "backrow": [{ "def": ID, "faceUp": false, "lane": 2 }],
                    "library": crate::filler(2),
                },
                "p2": { "hand": [FILLER], "library": crate::filler(2) },
            }));
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(s.card(ID).face_up, Some(false));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_unit_play_reveals_it_freezes_the_unit_and_costs_2_next_mana() {
            let mut s = board(true);
            s.play(VANILLA, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "graveyard");
            let unit = s.unit_at(P2, 1);
            assert!(frozen(&s, &unit), "the played Unit is Frozen");
            assert_eq!(s.state().players.p2.mana.next_turn_mod, -2);
            next_refresh(&mut s);
            let max = s.state().players.p2.mana.max;
            s.expect_mana(P2, max - 2);
        }
    }
}
