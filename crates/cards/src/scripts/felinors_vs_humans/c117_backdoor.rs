//! SPEC §8.14 FvH #117 Plagiarized: Backdoor, a frozen copy of IA #133 Backdoor (R2195003, FV4), a card of
//! the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Backdoor`, tagged Plagiarized only, and a token it makes stays the IA token. Its change from
//! IA #133: none. The IA file's notes, as they stood:
//!
//! SPEC §8.13 IA #133 Backdoor ((1) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent plays a Spell: Steal a random card from their hand."
//!   Radiant: "Reveals when your opponent plays a Spell: Steal two random cards from their hand."
//!
//! S T-AI-9 Honeypot's shape on an opponent's Spell play (R61, R99): from the hand the Spell just
//! left, `count` random cards change hands (§6.3 Steal, R60). A Unit play, and the controller's own
//! plays, leave it armed.

use jackioh_engine::effects::give_from_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-117";

/// "Steal a random card" (base) or "two random cards" (Radiant).
const STEALS: i32 = 1;
const RADIANT_STEALS: i32 = 2;

fn backdoor(steals: i32) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("backdoor", &[GameEventType::CardPlayed], move |_ctx, _event| {
                vec![give_from_hand(json_as(json!({
                    "from": "enemy",
                    "cards": "random",
                    "count": steals,
                })))]
            })
            .with_when(|ctx, event| {
                let GameEvent::CardPlayed { player, .. } = event else {
                    return false;
                };
                // R783005: the play as it counts, so a Unit set by its Cloaked is the Trap it was
                // set as, never a Spell.
                *player != ctx.controller
                    && played_type_of(&*ctx.state, event) == Some(CardType::Spell)
            }),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: backdoor(STEALS),
        radiant: backdoor(RADIANT_STEALS),
    }
}

// FvH #117 Plagiarized: Backdoor — SPEC §8.14 row 117; IA #133's tests (SPEC §8.13 row 133), on this card.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
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
                "library": crate::filler(3),
            },
            // p2's Unit keeps its turn from ending by itself once its hand is taken.
            "p2": { "hand": [SPELL, VANILLA, VANILLA], "field": [{ "def": "stand-in (1) 1/1", "lane": 5 }], "library": crate::filler(3), "mana": 10 },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn a_spell_play_reveals_it_and_steals_one_random_card() {
            let mut s = board(false);
            let before: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
            s.play(SPELL, json!({}));
            s.expect_in_zone(ID, "graveyard");
            let after: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(after.len(), before.len() + 1, "one card stolen: {after:?}");
            assert_eq!(s.hand(P2).len(), 1, "the Spell left and one card was taken");
        }

        #[test]
        fn a_unit_play_leaves_it_armed() {
            let mut s = board(false);
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(s.card(ID).face_up, Some(false));
        }

        #[test]
        fn your_own_spell_play_leaves_it_armed() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SPELL],
                    "backrow": [{ "def": ID, "faceUp": false, "lane": 2 }],
                    "library": crate::filler(2),
                },
                "p2": { "hand": [FILLER], "library": crate::filler(2) },
            }));
            s.play(SPELL, json!({}));
            assert_eq!(s.card(ID).face_up, Some(false));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_spell_play_reveals_it_and_steals_two_random_cards() {
            let mut s = board(true);
            let before: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
            s.play(SPELL, json!({}));
            s.expect_in_zone(ID, "graveyard");
            let after: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(after.len(), before.len() + 2, "two cards stolen: {after:?}");
            assert!(s.hand(P2).is_empty(), "the Spell left and two cards were taken");
        }
    }
}
