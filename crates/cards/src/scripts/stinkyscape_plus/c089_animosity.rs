//! SPEC §8.12 S+ #89 Animosity (2) Spell, Terminus, Mythic, a card of the held Stinkyscape+ set
//! (R1420, R3535000; R3569007).
//!   Base:    "Always starts on the bottom of your deck. If several cards do, their order is random.
//!            When your exile has {card} cards, cast this, wherever in your deck it is.
//!            Add a random Terminus card to your hand."
//!   Radiant: "… Add a random Radiant Terminus card to your hand."
//!
//! Its Cry is built here: one card drawn from the Terminus pool, every Terminus card of the shipped
//! sets, tokens included (R3552001, R380), through the generator, so it is never this card (R387) and
//! R673's Glitch odds apply; the Radiant face adds it Radiant (R3569007).
//!
//! NOT YET BUILT: the bottom-of-deck placement (E8's `StaticFlags.starts_at_bottom`) and the cast from
//! the deck when the owner's exile reaches `card` cards (E8's `Script.cast_from_deck_when`) belong to
//! E8 (#3558), which the branch this part was built on does not hold yet. When E8 lands, this card sets
//! the one flag and the one hook, the threshold being "at least `card`" exiled cards (R3569007), and
//! gains the tests of both.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-089";

fn animosity(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Terminus"], "withTokens": true },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: animosity(false),
        radiant: animosity(true),
    }
}

// S+ #89 Animosity — SPEC §8.12 row 89 (R3569007; R3552001, R387, R673).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn game(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": filler(10), "mana": 4 },
            "p2": { "library": filler(10) },
        }))
    }

    /// The Terminus pool of the shipped sets, tokens included.
    fn pool() -> Vec<String> {
        crate::register_all();
        catalog::query(&json_as(json!({ "tags": ["Terminus"], "withTokens": true })))
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3569007_animosity_adds_a_random_terminus_card() {
            let pool = pool();
            let mut seen: Vec<String> = Vec::new();
            for n in 0..12 {
                let mut s = game(&format!("animosity-base-{n}"), false);
                s.play(ID, json!({}));
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(
                    pool.contains(&made[0].def_id),
                    "{} is no Terminus card",
                    made[0].def_id
                );
                assert_ne!(made[0].def_id, ID, "never itself (R387)");
                assert!(!made[0].radiant);
                if !seen.contains(&made[0].def_id) {
                    seen.push(made[0].def_id.clone());
                }
            }
            assert!(seen.len() > 1, "a random pick: only {seen:?}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569007_radiant_animosity_adds_a_radiant_terminus_card() {
            let pool = pool();
            for n in 0..6 {
                let mut s = game(&format!("animosity-radiant-{n}"), true);
                s.play(ID, json!({}));
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(pool.contains(&made[0].def_id));
                assert_ne!(made[0].def_id, ID);
                assert!(made[0].radiant, "seed {n}");
            }
        }
    }
}
