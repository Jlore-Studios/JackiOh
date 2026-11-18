//! S #18 Imitated Instrumentality (SPEC §8.10 row S 18; §6.3, R1, R64, R1843000, R1843001, R1866000):
//! (3) Spell, Tech, Legendary. Tribute 1 since issue #4154 (R101): `staticFlags.tribute`, the play's cost;
//! through v0.6.6g it cost (4) and asked none (`script_v0_6_6g`, R767000).
//!   Base:    "Summon MELCHIOR, BALTHASAR and CASPER. They offer great strategic value, but they can vote
//!            to remove you."
//!   Radiant: "Summon a Radiant MELCHIOR, a Radiant BALTHASAR and a Radiant CASPER. They offer great
//!            strategic value, but they can vote to remove you."
//!
//! Engine: three `summon`s, S #18.1 MELCHIOR, S #18.2 BALTHASAR and S #18.3 CASPER, in that order, each
//! a fresh Field Spell into its caster's leftmost open backrow zone (R64), one with no zone left not
//! summoned; summoned, so no Cry (R1). Radiant: each `radiant: true`. The ending is the Brains' own
//! (R1843000): each votes at its controller's start of turn on its own condition, and three votes in one
//! turn remove that player, a loss for them and a win for the opponent (R1843001).

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

/// "Tribute 1" (R101, R366: printed, declared nowhere), since issue #4154.
const TRIBUTE: i32 = 1;

pub fn script() -> CardScripts {
    let mut scripts = script_v0_6_6g();
    for face in [&mut scripts.base, &mut scripts.radiant] {
        face.static_flags = Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        });
    }
    scripts
}

/// R767000: no Tribute through v0.6.6g (issue #4154); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    CardScripts {
        base: instrumentality(false),
        radiant: instrumentality(true),
    }
}

// S #18 Imitated Instrumentality — SPEC §8.10 row S 18, BUILD M12 row S 18 (R1843000, R1843001,
// R1866000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const INSTRUMENTALITY: &str = "stinkyscape-018";
    const MELCHIOR: &str = "stinkyscape-018-1";
    const BALTHASAR: &str = "stinkyscape-018-2";
    const CASPER: &str = "stinkyscape-018-3";
    const MANA_WELL: &str = "core-006";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-016"; // Hit Job: inert in a hand.
    /// The unit its Tribute 1 takes (issue #4154).
    const BODY: &str = "stand-in (1) 1/1";

    fn backrow(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5).map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant))).collect()
    }

    fn cast(radiant: bool, backrow: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": INSTRUMENTALITY, "radiant": radiant }, FILLER],
                "field": [BODY],
                "backrow": backrow,
            },
            "p2": { "hand": [FILLER] },
        }));
        s.play(INSTRUMENTALITY, json!({ "tributes": [BODY] }));
        s
    }

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
            // At 10 health with no Unit against two (its Tribute took p1's one), after a turn that left 1 mana
            // unspent (4 mana for the (3) it costs since #4154): each Brain's condition holds at p1's next
            // start of turn.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [INSTRUMENTALITY, FILLER],
                    "field": [BODY],
                    "health": 10,
                    "library": filler(4),
                    "mana": 4,
                },
                "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA], "library": filler(4) },
            }));
            s.play(INSTRUMENTALITY, json!({ "tributes": [BODY] }));
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

    /// Issue #4154: Tribute 1 on both faces (R101): with no unit to pay it the play is refused; a game on
    /// v0.6.6g plays it with none, under the kept script (R767000).
    #[test]
    fn issue_4154_both_faces_need_tribute_1() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": INSTRUMENTALITY, "radiant": radiant }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            s.expect_refused_with(|s| s.play(INSTRUMENTALITY, json!({})), "Tribute");
            crate::under_kept(INSTRUMENTALITY, super::script_v0_6_6g(), || {
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": INSTRUMENTALITY, "radiant": radiant }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(INSTRUMENTALITY, json!({}));
                assert_eq!(summoned(&s), [MELCHIOR, BALTHASAR, CASPER], "radiant {radiant}");
            });
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
