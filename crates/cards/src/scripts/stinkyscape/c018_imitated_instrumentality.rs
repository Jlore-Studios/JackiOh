//! S #18 Imitated Instrumentality (SPEC §8.10 row S 18; §6.3, R1, R64, R1843000, R1843001, R1866000):
//! (3) Spell, Tech, Legendary.
//!   Base:    "Summon MELCHIOR, BALTHASAR and CASPER. They offer great strategic value, but they can vote
//!            to remove you."
//!   Radiant: "Summon a Radiant MELCHIOR, a Radiant BALTHASAR and a Radiant CASPER. They offer great
//!            strategic value, but they can vote to remove you."
//!
//! Engine: three `summon`s, S #18.1 MELCHIOR, S #18.2 BALTHASAR and S #18.3 CASPER, in that order, each
//! a fresh Field Spell into its caster's leftmost open backrow zone (R64), one with no zone left not
//! summoned; summoned, so no Cry (R1). Radiant: each `radiant: true`. The ending is the Brains' own
//! (R1843000): each votes at its controller's start of turn on its own condition, and three votes in one
//! turn remove that player, a loss for them and a win for the opponent (R1843001). Declares no numbers.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-018";

/// S #18.1 MELCHIOR, S #18.2 BALTHASAR, S #18.3 CASPER — the order they are summoned in (R1866000).
const BRAINS: [&str; 3] = ["stinkyscape-018-1", "stinkyscape-018-2", "stinkyscape-018-3"];

fn instrumentality(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            BRAINS
                .iter()
                .map(|def_id| {
                    let mut args = json!({ "defId": def_id });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    summon(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: instrumentality(false),
        radiant: instrumentality(true),
    }
}

// S #18 Imitated Instrumentality — SPEC §8.10 row S 18, BUILD M12 row S 18: "cast: MELCHIOR, BALTHASAR
// and CASPER summoned into your backrow in that order, one with no zone not summoned; three votes in one
// of your turns end the game, voted-out, a loss for you (R1843000, R1843001); radiant: the three Radiant"
// (R1866000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const INSTRUMENTALITY: &str = "stinkyscape-018";
    const MELCHIOR: &str = "stinkyscape-018-1";
    const BALTHASAR: &str = "stinkyscape-018-2";
    const CASPER: &str = "stinkyscape-018-3";
    /// A Field Spell to fill a backrow zone with.
    const MANA_WELL: &str = "core-006";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-016"; // Hit Job: inert in a hand.

    /// The def id and Radiant flag of each of p1's backrow zones, lane 1 to 5.
    fn backrow(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5).map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant))).collect()
    }

    /// p1 casts the card (Radiant on `radiant`) over `backrow`.
    fn cast(radiant: bool, backrow: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": INSTRUMENTALITY, "radiant": radiant }, FILLER], "backrow": backrow },
            "p2": { "hand": [FILLER] },
        }));
        s.play(INSTRUMENTALITY, json!({}));
        s
    }

    /// The def ids the scenario's `summoned` events name, in order: summoned, so none played (R1).
    fn summoned(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn brain(def_id: &str, radiant: bool) -> Option<(String, bool)> {
        Some((def_id.to_string(), radiant))
    }

    mod base {
        use super::*;

        #[test]
        fn r1866000_summons_melchior_balthasar_and_casper_in_that_order_into_the_leftmost_open_backrow_zones() {
            let s = cast(false, json!([{ "def": MANA_WELL, "lane": 2 }]));
            assert_eq!(
                backrow(&s),
                [
                    brain(MELCHIOR, false),
                    brain(MANA_WELL, false),
                    brain(BALTHASAR, false),
                    brain(CASPER, false),
                    None,
                ]
            );
            assert_eq!(summoned(&s), [MELCHIOR, BALTHASAR, CASPER]);
        }

        #[test]
        fn r1866000_one_with_no_zone_is_not_summoned() {
            let s = cast(false, json!([MANA_WELL, MANA_WELL, MANA_WELL]));
            assert_eq!(
                backrow(&s),
                [
                    brain(MANA_WELL, false),
                    brain(MANA_WELL, false),
                    brain(MANA_WELL, false),
                    brain(MELCHIOR, false),
                    brain(BALTHASAR, false),
                ]
            );
            assert_eq!(summoned(&s), [MELCHIOR, BALTHASAR]);
        }

        #[test]
        fn r1866000_r1843000_three_votes_at_your_next_start_of_turn_remove_you() {
            // At 10 health with no Unit against two, after a turn that left 1 mana unspent (5 mana for the
            // (4) it costs since #2657): each Brain's condition holds at p1's next start of turn.
            let mut s = scenario(json!({
                "p1": { "hand": [INSTRUMENTALITY, FILLER], "health": 10, "library": filler(4), "mana": 5 },
                "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA], "library": filler(4) },
            }));
            s.play(INSTRUMENTALITY, json!({}));
            s.end_turn();
            s.end_turn();

            let result = s.state().result.expect("the game is over");
            assert_eq!((result.winner, result.reason), (Winner::P2, GameOverReason::VotedOut));
            let events = s.last_events();
            let voted: Vec<String> = events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::VoteCast { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(voted, [MELCHIOR, BALTHASAR, CASPER]);
            assert_eq!(events[events.len() - 2].event_type(), GameEventType::VotedOut);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1866000_the_radiant_face_summons_three_radiant_brains() {
            let s = cast(true, json!([]));
            assert_eq!(
                backrow(&s),
                [brain(MELCHIOR, true), brain(BALTHASAR, true), brain(CASPER, true), None, None]
            );
        }
    }
}
