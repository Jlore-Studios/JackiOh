//! S #42 Core Researcher (SPEC §8.10 row 42): (2) Unit, Human, Rare, 4/5 → 8/10, a card of the
//! Stinkyscape set.
//!   Base:    "Cry: Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your
//!            hand." — cards 1
//!   Radiant: the same words — cards 2
//! Engine: Cry (R1), so a summon (a Recruit, a Reborn) adds nothing. It picks from S #41's Drone Core pool
//!   (the non-token cards tagged Drone Core, R380), one draw of the match rng per card (R60), each added
//!   Radiant on both faces (§5.2); a full hand burns it (§2.4). The count is the declared `cards`, a Cry
//!   number (R386, R1748000). The pool names no set, so it reaches the set's Drone Cores because the set
//!   ships (R1420).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-042";

fn researcher() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Drone Core"] },
                "count": param(&*ctx, "cards"),
                "radiant": true,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: both faces add Radiant cards, and the Radiant face's 2 and 8/10 are catalog data.
    CardScripts {
        base: researcher(),
        radiant: researcher(),
    }
}

// S #42 Core Researcher — SPEC §8.10 row 42, BUILD M12 row S 42 (R1).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const RESEARCHER: &str = "stinkyscape-042";
    const CALL_TO_ARMS: &str = "core-069"; // (2) Spell: Recruit Units of (1) Cost or less; Radiant (2).
    const FILLER: &str = "core-005";

    fn cores(s: &Scenario) -> Vec<(String, bool)> {
        s.hand(P1)
            .into_iter()
            .filter(|card| {
                (27..=36)
                    .map(|n| format!("stinkyscape-{n:03}"))
                    .any(|tier| tier == card.def_id)
            })
            .map(|card| (card.def_id, card.radiant))
            .collect()
    }

    fn played(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "core-researcher",
            "p1": { "hand": [{ "def": RESEARCHER, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(RESEARCHER, json!({}));
        s
    }

    #[test]
    fn r1_played_adds_a_radiant_drone_core() {
        let s = played(false);
        let added = cores(&s);
        assert_eq!(added.len(), 1, "{added:?}");
        assert!(added[0].1, "Radiant");
    }

    #[test]
    fn r1_recruited_it_adds_nothing() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": CALL_TO_ARMS, "radiant": true }, FILLER],
                "library": [RESEARCHER, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CALL_TO_ARMS, json!({}));
        assert_eq!(s.unit(P1, 1).expect("the recruited Researcher").def_id, RESEARCHER);
        assert!(cores(&s).is_empty(), "no Cry");
    }

    #[test]
    fn cards_reads_through_param() {
        let mut s = scenario(json!({
            "p1": { "hand": [RESEARCHER, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        set_param(s.card_mut(RESEARCHER), "cards", 3);
        s.play(RESEARCHER, json!({}));
        assert_eq!(cores(&s).len(), 3);
    }

    #[test]
    fn radiant_8_10_adds_two_radiant_ones() {
        let s = played(true);
        let researcher = s.unit(P1, 1).expect("the Researcher");
        let stats = s.stats(researcher.id.as_str());
        assert_eq!((stats.attack, stats.health), (8, 10));
        let added = cores(&s);
        assert_eq!(added.len(), 2, "{added:?}");
        assert!(added.iter().all(|(_, radiant)| *radiant));
    }
}
