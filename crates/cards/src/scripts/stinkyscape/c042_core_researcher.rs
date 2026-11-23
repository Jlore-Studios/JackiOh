//! S #42 Core Researcher (SPEC §8.10 row 42): (2) Unit, Human, Rare, 3/3 → 6/6, a card of the
//! Stinkyscape set.
//!   Base:    "Death: Add {cards|random Radiant Drone Core card|random Radiant Drone Core cards} to your
//!            hand." — cards 1
//!   Radiant: the same words — cards 2
//! Engine: Death (§4.5 step 3, R89), so it adds as it dies however it reached the field, a summon
//!   included, and not as it is played; a bounce or an exile is no death. It picks from S #41's Drone Core
//!   pool (the non-token cards tagged Drone Core, R380), one draw of the match rng per card (R60), each
//!   added Radiant on both faces (§5.2) to its controller's hand as it died; a full hand burns it (§2.4).
//!   The count is the declared `cards`, read off the card as it died (R89, R386). The pool names no set,
//!   so it reaches the set's Drone Cores because the set ships (R1420). Through v0.6.5b it was a Cry (R1)
//!   (issue #3996), which `script_v0_6_5b` keeps for those replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-042";

/// "Add {cards} random Radiant Drone Core cards to your hand."
fn research(ctx: &EffectContext<'_>) -> Vec<Effect> {
    vec![add_random_from_catalog(json_as(json!({
        "query": { "tags": ["Drone Core"] },
        "count": param(ctx, "cards"),
        "radiant": true,
    })))]
}

pub fn script() -> CardScripts {
    // One script: both faces add Radiant cards, and the Radiant face's 2 and 6/6 are catalog data.
    let researcher = Script {
        death: Some(hook(|ctx| research(ctx))),
        ..Script::default()
    };
    CardScripts {
        base: researcher.clone(),
        radiant: researcher,
    }
}

/// R767000: the scripts v0.6.5b and every version before it ran, the cards added by a Cry (issue #3996).
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    let researcher = Script {
        cry: Some(hook(|ctx| research(ctx))),
        ..Script::default()
    };
    CardScripts {
        base: researcher.clone(),
        radiant: researcher,
    }
}

// S #42 Core Researcher — SPEC §8.10 row 42, BUILD M12 row S 42 (§4.5, R89; R1 for the kept Cry).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const RESEARCHER: &str = "stinkyscape-042";
    const CALL_TO_ARMS: &str = "core-069"; // Call to Arms: Recruit cheap Units; Radiant raises the cap.
    const HIT_JOB: &str = "core-016"; // Hit Job: destroy a Unit
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

    /// p1 plays the Researcher (base unless `radiant`), a Hit Job in hand to destroy it with.
    fn played(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "core-researcher",
            "p1": {
                "hand": [{ "def": RESEARCHER, "radiant": radiant }, HIT_JOB, FILLER],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(RESEARCHER, json!({}));
        s
    }

    /// p1's Hit Job on the Researcher in lane 1.
    fn destroy_it(s: &mut Scenario) {
        let researcher = s.unit_at(P1, 1);
        assert_eq!(researcher.def_id, RESEARCHER);
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": researcher.id }] }));
        s.expect_in_zone(&researcher, "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn played_it_adds_nothing() {
            let s = played(false);
            assert!(cores(&s).is_empty(), "no Cry");
        }

        #[test]
        fn r89_as_it_dies_it_adds_a_radiant_drone_core() {
            let mut s = played(false);
            destroy_it(&mut s);
            let added = cores(&s);
            assert_eq!(added.len(), 1, "{added:?}");
            assert!(added[0].1, "Radiant");
        }

        /// A Death answers however the card reached the field: recruited, it still adds as it dies.
        #[test]
        fn recruited_it_adds_nothing_until_it_dies() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": CALL_TO_ARMS, "radiant": true }, HIT_JOB, FILLER],
                    "library": [RESEARCHER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.play(CALL_TO_ARMS, json!({}));
            assert!(cores(&s).is_empty());
            destroy_it(&mut s);
            assert_eq!(cores(&s).len(), 1);
        }

        #[test]
        fn cards_reads_through_param() {
            let mut s = scenario(json!({
                "p1": { "hand": [RESEARCHER, HIT_JOB, FILLER], "library": filler(4), "mana": 10 },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            set_param(s.card_mut(RESEARCHER), "cards", 3);
            s.play(RESEARCHER, json!({}));
            destroy_it(&mut s);
            assert_eq!(cores(&s).len(), 3);
        }

        #[test]
        fn base_3_3() {
            let s = played(false);
            let stats = s.stats(s.unit_at(P1, 1).id.as_str());
            assert_eq!((stats.attack, stats.health), (3, 3));
        }

        /// R767000, R1: through v0.6.5b it was a Cry (issue #3996): played it added the card, and a summon
        /// added nothing.
        #[test]
        fn r767000_r1_the_script_kept_for_v0_6_5b_adds_as_it_is_played_and_not_as_it_is_recruited() {
            crate::under_kept(RESEARCHER, super::super::script_v0_6_5b(), || {
                let mut s = played(false);
                assert_eq!(cores(&s).len(), 1);
                destroy_it(&mut s);
                assert_eq!(cores(&s).len(), 1, "no Death");
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": CALL_TO_ARMS, "radiant": true }, FILLER],
                        "library": [RESEARCHER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(CALL_TO_ARMS, json!({}));
                assert_eq!(s.unit_at(P1, 1).def_id, RESEARCHER);
                assert!(cores(&s).is_empty(), "no Cry");
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_6_6_adds_nothing_played_and_two_radiant_ones_as_it_dies() {
            let mut s = played(true);
            let stats = s.stats(s.unit_at(P1, 1).id.as_str());
            assert_eq!((stats.attack, stats.health), (6, 6));
            assert!(cores(&s).is_empty());
            destroy_it(&mut s);
            let added = cores(&s);
            assert_eq!(added.len(), 2, "{added:?}");
            assert!(added.iter().all(|(_, radiant)| *radiant));
        }
    }
}
