//! M #63 Skull of J'Nari (SPEC §8.8 row 63, R1083): (3) Field Spell, Legendary.
//!
//! Base:    "Start of turn: Summon {units|random Unit|random Units} from your hand."
//! Radiant: "Start of turn: Summon {units|random Unit|random Units} from your hand. Make it Radiant."
//! Engine: `start_of_turn` returns `param("units")` copies of `summon_random_from_hand` (R1083),
//! Radiant with `radiant: true`.
//! Voice: the `trigger` hook in `card-audio.json5` ("This one will prove very useful! ...Probably.",
//! "There IS a method to my madness!", "Good units are hard to find."), picked by the client's own
//! rng when the runner starts the `summoned` entry this card caused (`sourceId`, R1088).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-063";

fn skull(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            (0..param(&*ctx, "units"))
                .map(|_| {
                    summon_random_from_hand(json_as(json!({
                        "radiant": radiant,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: skull(false),
        radiant: skull(true),
    }
}

// M #63 Skull of J'Nari — SPEC §8.8 row 63, BUILD M10 row M 63: "At your start of turn only, one
// random Unit card from your hand is summoned into your leftmost open unit zone with no Cry,
// summoning sick (R1083); a Spell in hand is never picked; no Unit in hand or no open zone draws
// no random number (R129); the opponent sees only the arrival; one of its voice lines plays from
// the client's own rng, never the match's; units reads through `param()`; radiant the Unit becomes
// Radiant on the field".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SKULL: &str = "meditative-063";
    /// (1) 4/4 Unit with no hooks.
    const VANILLA: &str = "core-008";
    /// (1) Spell.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 holds the Skull (`radiant_face`) on the backrow with `hand` cards in hand. One `start_turn`
    /// runs p1's start of turn (the Skull fires), with its opening draw.
    fn skull_with(seed: &str, radiant_face: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": hand,
                "backrow": [{ "def": SKULL, "lane": 1, "radiant": radiant_face }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.start_turn();
        s
    }

    fn hand_def_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).iter().map(|card| card.def_id.clone()).collect()
    }

    mod m63_skull_of_jnari {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn start_of_turn_summons_a_hand_unit_leftmost_with_no_cry() {
                let s = skull_with("skull-summon", false, json!([VANILLA, FILLER]));

                let unit = s.unit(P1, 1).expect("the summoned unit");
                assert_eq!(unit.def_id, VANILLA);
                assert!(!unit.radiant);
                // Summoning sick: it arrived this turn.
                assert_eq!(unit.summoned_turn, Some(s.state().turn));
                // The Vanilla left the hand (the draw plus the Filler stay).
                assert!(!hand_def_ids(&s).contains(&VANILLA.to_string()));
                assert!(hand_def_ids(&s).contains(&FILLER.to_string()));
            }

            #[test]
            fn a_spell_in_hand_is_never_picked() {
                let s = skull_with("skull-spell", false, json!([FILLER, FILLER]));

                assert!(s.unit(P1, 1).is_none());
                // Nothing left the hand but the opening draw entered it.
                assert_eq!(s.hand(P1).len(), 3);
            }

            #[test]
            fn r1083_no_open_zone_draws_nothing() {
                crate::register_all();
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
                s.start_turn();

                // The Vanilla never left the hand, and no pick was drawn.
                assert!(hand_def_ids(&s).contains(&VANILLA.to_string()));
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

            #[test]
            fn units_reads_through_param() {
                crate::register_all();
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
                s.start_turn();

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
        }
    }
}
