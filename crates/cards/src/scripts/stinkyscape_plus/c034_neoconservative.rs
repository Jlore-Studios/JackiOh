//! S+ #34 Neoconservative (SPEC §8.12 row 34). (2) Unit, Human, Common, 5/3 → 10/6.
//!   Base:    "Cry: Add a Jlockeed card from the past to your hand."
//!   Radiant: "Cry: Add a Radiant Jlockeed card from the past to your hand."
//!
//! Cry (R1): one draw from the past Jlockeed definitions of the sets that ship
//! (`{ tags: ["Jlockeed"], past: "only" }`, R3538004), Radiant on the Radiant face. The card reaches
//! a hand, so the opponent never reads it (R440).
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-034";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Jlockeed"], "past": "only" },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S+ #34 Neoconservative — SPEC §8.12 row 34 (R3538004).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, "core-005"],
                "field": [{ "def": "core-008", "lane": 1 }],
                "library": filler(8),
                "mana": 4,
            },
            "p2": {
                "hand": ["core-005"],
                "field": [{ "def": "core-008", "lane": 1 }],
                "library": filler(8),
            },
        }))
    }

    fn added(s: &Scenario, before: usize) -> Vec<CardInstance> {
        s.hand(P1).into_iter().skip(before).collect()
    }

    #[test]
    fn r3538004_base_adds_a_past_jlockeed_card_hidden_from_the_opponent() {
        crate::register_all();
        let mut s = board(false, "neoconservative");
        let before = s.hand(P1).len() - 1;
        s.play(ID, json!({ "zone": 2 }));
        let got = added(&s, before);
        assert_eq!(got.len(), 1, "one card added");
        let past = &got[0].def_id;
        assert!(split_past_id(past).is_some(), "{past}");
        let def = past_def(past).expect("a past definition");
        assert!(def.tags.contains(&Tag::Jlockeed), "{past}");
        assert!(!got[0].radiant);
        assert_eq!(
            crate::js(&s.view(P2).opponent.hand),
            json!({ "count": s.hand(P1).len() }),
            "the opponent counts the hand, never reads it (R440)"
        );
    }

    #[test]
    fn r3538004_radiant_adds_it_radiant() {
        crate::register_all();
        let mut s = board(true, "neoconservative-radiant");
        let before = s.hand(P1).len() - 1;
        s.play(ID, json!({ "zone": 2 }));
        let got = added(&s, before);
        assert_eq!(got.len(), 1, "one card added");
        assert!(split_past_id(&got[0].def_id).is_some(), "{}", got[0].def_id);
        assert!(got[0].radiant);
    }
}
