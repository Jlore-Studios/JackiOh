//! M #63 Skull of J'Nari (SPEC §8.8 row 63, R1083): (3) Field Spell, Legendary.
//!
//! Base:    "End of turn: Summon {units|random Unit|random Units} from your hand."
//! Radiant: "End of turn: Summon {units|random Unit|random Units} from your hand. Make it Radiant."
//! Engine: `end_of_turn` returns `param("units")` copies of `summon_random_from_hand` (R1083),
//! Radiant with `radiant: true`. It runs at its controller's end of turn alone (R62), in the order the
//! turn's other end-of-turn hooks come; through v0.5.11o it was a `start_of_turn` hook (`script_v0_5_11o`,
//! R767000).
//! Voice: the `trigger` hook in `card-audio.json5`, its lines in `card-text.yaml` ("This one will
//! prove very useful! ...Probably.", "There IS a method to my madness!", "Good units are hard to
//! find."), picked by the client's own rng when the runner starts the `summoned` entry this card
//! caused (`sourceId`, R1088).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-063";

/// The summon of {units} random Units from the hand (R1083), Radiant on the Radiant face, which the Skull
/// runs at the end of its controller's turn since #3140 and ran at the start before.
fn summons(radiant: bool) -> Hook {
    hook(move |ctx| {
        (0..param(&*ctx, "units"))
            .map(|_| {
                summon_random_from_hand(json_as(json!({
                    "radiant": radiant,
                })))
            })
            .collect()
    })
}

fn skull(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(summons(radiant)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: skull(false),
        radiant: skull(true),
    }
}

/// R767000: through v0.5.11o the Skull summoned at the start of its controller's turn; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    let at_start = |radiant: bool| Script {
        start_of_turn: Some(summons(radiant)),
        ..Script::default()
    };
    CardScripts {
        base: at_start(false),
        radiant: at_start(true),
    }
}

// M #63 Skull of J'Nari — SPEC §8.8 row 63, BUILD M10 row M 63 (R129, R1083, R767000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use crate::filler;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const SKULL: &str = "meditative-063";
    /// (1) 4/4 Unit with no hooks.
    const VANILLA: &str = "core-008";
    /// (1) Spell.
    const FILLER: &str = "core-005";

    /// p1 holds the Skull (`radiant_face`) on the backrow with `hand` cards in hand, a library to draw from
    /// and nothing run yet.
    fn skull_before(seed: &str, radiant_face: bool, hand: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": hand,
                "backrow": [{ "def": SKULL, "lane": 1, "radiant": radiant_face }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// `skull_before`, then p1 ends its turn: the Skull fires, and p2's turn begins.
    fn skull_with(seed: &str, radiant_face: bool, hand: Value) -> Scenario {
        let mut s = skull_before(seed, radiant_face, hand);
        s.end_turn();
        s
    }

    mod m63_skull_of_jnari {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn end_of_turn_summons_a_hand_unit_leftmost_with_no_cry() {
                let mut s = skull_before("skull-summon", false, json!([VANILLA, FILLER]));
                let turn = s.state().turn;
                s.end_turn();

                let unit = s.unit(P1, 1).expect("the summoned unit");
                assert_eq!(unit.def_id, VANILLA);
                assert!(!unit.radiant);
                // Summoning sick: it arrived on p1's turn that just ended.
                assert_eq!(unit.summoned_turn, Some(turn));
                // The Vanilla left the hand; p1 draws nothing at the end of its turn.
                assert!(!def_ids(&s.hand(P1)).contains(&VANILLA.to_string()));
                assert_eq!(def_ids(&s.hand(P1)), vec![FILLER.to_string()]);
            }

            #[test]
            fn r62_it_is_the_end_of_turn_not_the_start_so_nothing_is_summoned_as_the_turn_begins() {
                let mut s = skull_before("skull-not-start", false, json!([VANILLA, FILLER]));
                s.start_turn();
                assert!(s.unit(P1, 1).is_none());
                assert!(def_ids(&s.hand(P1)).contains(&VANILLA.to_string()));
                // The summon comes when the turn ends, after the plays the Unit could not have waited for.
                s.end_turn();
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
            }

            #[test]
            fn a_spell_in_hand_is_never_picked() {
                let s = skull_with("skull-spell", false, json!([FILLER, FILLER]));

                assert!(s.unit(P1, 1).is_none());
                // Nothing left the hand.
                assert_eq!(s.hand(P1).len(), 2);
            }

            #[test]
            fn r1083_no_open_zone_draws_nothing() {
                let mut s = scenario(json!({
                    "seed": "skull-full",
                    "p1": {
                        "hand": [VANILLA, FILLER],
                        "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                        "backrow": [{ "def": SKULL, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let cursor = s.state().rng_cursor;
                s.end_turn();

                assert!(def_ids(&s.hand(P1)).contains(&VANILLA.to_string()));
                assert_eq!(cursor, s.state().rng_cursor);
            }

            #[test]
            fn the_summoned_event_names_its_source() {
                let s = skull_with("skull-source", false, json!([VANILLA, FILLER]));

                let sources: Vec<Option<String>> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Summoned {
                            def_id, source_id, ..
                        } if def_id == VANILLA => Some(source_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(sources.len(), 1);
                let skull = s.card(SKULL);
                assert_eq!(sources[0], Some(skull.id.clone()));
            }

            /// Me and Mr Token (Core #15): (1) Unit, "Cry: Summon a Rush Token".
            const MR_TOKEN: &str = "core-015";
            /// Anti-oneshot Armor (Core #73): a Field Spell, so never picked.
            const FIELD_SPELL: &str = "core-073";

            fn p1_units(s: &Scenario) -> usize {
                (1..=UNIT_ZONES)
                    .filter(|lane| s.unit(P1, *lane).is_some())
                    .count()
            }

            #[test]
            fn the_summoned_unit_runs_no_cry() {
                let s = skull_with("skull-no-cry", false, json!([MR_TOKEN, FILLER]));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(MR_TOKEN.to_string()));
                // Its Cry would have summoned a Rush Token beside it.
                assert_eq!(p1_units(&s), 1);
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn it_takes_the_leftmost_open_unit_zone() {
                let mut s = scenario(json!({
                    "seed": "skull-leftmost",
                    "p1": {
                        "hand": [MR_TOKEN, FILLER],
                        "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }],
                        "backrow": [{ "def": SKULL, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.end_turn();
                assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(MR_TOKEN.to_string()));
            }

            #[test]
            fn the_opponents_end_of_turn_summons_nothing() {
                // p2 holds the turn, with the Skull p1's: p2 ending it is not p1's end of turn.
                let mut s = scenario(json!({
                    "seed": "skull-theirs",
                    "active": "p2",
                    "p1": {
                        "hand": [VANILLA, FILLER],
                        "backrow": [{ "def": SKULL, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER, FILLER], "library": filler(4) },
                }));
                s.end_turn();
                assert_eq!(s.state().active, PlayerId::P1);
                assert_eq!(p1_units(&s), 0);
                assert!(def_ids(&s.hand(P1)).contains(&VANILLA.to_string()));
            }

            #[test]
            fn the_opponents_skull_summons_for_them_at_their_end_of_turn_alone() {
                // The Skull is p2's: p1 ending its own turn summons nothing for p2.
                let mut s = scenario(json!({
                    "seed": "skull-other-side",
                    "p1": { "hand": [VANILLA, FILLER], "library": filler(4) },
                    "p2": {
                        "hand": [VANILLA, FILLER],
                        "backrow": [{ "def": SKULL, "lane": 1 }],
                        "library": filler(4),
                    },
                }));
                s.end_turn();
                assert!(s.unit(PlayerId::P2, 1).is_none());
                assert!(s.unit(P1, 1).is_none());
                s.end_turn();
                assert_eq!(s.unit(PlayerId::P2, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
                assert!(s.unit(P1, 1).is_none());
            }

            #[test]
            fn r129_no_unit_in_hand_draws_no_random_number() {
                let mut s = scenario(json!({
                    "seed": "skull-no-unit",
                    "p1": {
                        "hand": [FILLER, FIELD_SPELL],
                        "backrow": [{ "def": SKULL, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let cursor = s.state().rng_cursor;
                s.end_turn();
                assert_eq!(p1_units(&s), 0);
                assert_eq!(cursor, s.state().rng_cursor, "no pick was drawn");
            }

            #[test]
            fn the_opponent_sees_only_the_arrival() {
                let s = skull_with("skull-hidden", false, json!([VANILLA, FIELD_SPELL]));
                let view = s.view(PlayerId::P2);
                let seen = serde_json::to_string(&view).expect("the view serializes");
                assert!(
                    view.opponent
                        .units
                        .iter()
                        .flatten()
                        .any(|unit| unit.def_id == VANILLA),
                    "p2 sees the summoned Vanilla"
                );
                assert!(!seen.contains(FIELD_SPELL), "p2 reads the rest of p1's hand");
            }

            #[test]
            fn units_reads_through_param() {
                let mut s = scenario(json!({
                    "seed": "skull-param",
                    "p1": {
                        "hand": [VANILLA, VANILLA, FILLER],
                        "backrow": [{ "def": SKULL, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                set_param(s.card_mut(SKULL), "units", 2);
                s.end_turn();

                assert!(s.unit(P1, 1).is_some());
                assert!(s.unit(P1, 2).is_some());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_summoned_unit_becomes_radiant_on_the_field() {
                let s = skull_with("skull-radiant", true, json!([VANILLA, FILLER]));

                let unit = s.unit(P1, 1).expect("the summoned unit");
                assert_eq!(unit.def_id, VANILLA);
                assert!(unit.radiant);
            }

            #[test]
            fn it_too_waits_for_the_end_of_turn() {
                let mut s = skull_before("skull-radiant-not-start", true, json!([VANILLA, FILLER]));
                s.start_turn();
                assert!(s.unit(P1, 1).is_none());
                s.end_turn();
                assert!(s.unit(P1, 1).is_some_and(|unit| unit.radiant));
            }
        }

        /// R767000: the script kept for the replays of v0.5.11o and before, which summoned at the start.
        mod v0_5_11o {
            use super::*;

            #[test]
            fn r767000_a_game_on_v0_5_11o_summons_at_the_start_of_turn_and_not_at_the_end() {
                for radiant in [false, true] {
                    crate::under_kept(SKULL, script_v0_5_11o(), || {
                        let mut s = skull_before("skull-kept", radiant, json!([VANILLA, FILLER]));
                        s.start_turn();
                        let unit = s.unit(P1, 1).expect("summoned at the start");
                        assert_eq!(unit.def_id, VANILLA);
                        assert_eq!(unit.radiant, radiant);

                        let mut s = skull_before("skull-kept-end", radiant, json!([VANILLA, FILLER]));
                        s.end_turn();
                        assert!(s.unit(P1, 1).is_none(), "nothing at its end of turn");
                    });
                }
            }
        }
    }
}
