//! M #60 Eschews (SPEC §8.8 row 60, R1082): (3) Field Spell, Epic.
//!
//! Base:    "End of turn: Cast {casts|random Human, Book, CN or AI generated card|random Human,
//!          Book, CN or AI generated cards}.\nOnce {deaths} of your Units have died, Tribute this."
//! Radiant: "End of turn: Cast {casts|random Radiant Human, Book, CN or AI generated card|random
//!          Radiant Human, Book, CN or AI generated cards}.\nOnce {deaths} of your Units have died,
//!          Tribute this."
//! Engine: `cast_random` over `{ anyTags: [Human, Book, CN, AI] }` (R1422, R1421), each definition
//! once, never Eschews (R387); a trigger on `destroyed` of your Units counts in `memory.deaths`
//! since entry (R78 resets it); `tribute_when` Tributes it at the next state check once the count
//! reaches `{deaths}` (C #88, R403). R4423000: a Unit's death is read by what the card was on the field as
//! it died (`is_unit_death`), so a Unit set face-down by its Cloaked and destroyed before it reveals is not
//! counted and an Animated Field Spell or Trap killed in a unit zone is; `script_v0_6_9i` keeps the face's
//! printed type for v0.6.9l and before (R767000).

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-060";

/// The memory key the `destroyed` trigger counts under.
const DEATHS: &str = "deaths";

/// The deaths counted so far: the bare key, or any fused part's key for it (R102: each ingredient
/// remembers apart, so a fused Eschews reads every `deaths@…` path too).
fn deaths_counted(memory: &IndexMap<String, Value>) -> i64 {
    memory
        .iter()
        .filter(|(key, _)| key.as_str() == DEATHS || key.starts_with("deaths@"))
        .filter_map(|(_, value)| value.as_i64())
        .sum()
}

/// R2476000: "Once 5 of your Units have died" — the deaths it has counted since it arrived, after "of
/// your Units have died", which both faces print; the count `tribute_when` holds to `deaths`. On the
/// field only (R78 forgets it as it leaves), to both seats: each death was public.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        vec![count_preview("of your Units have died", deaths_counted(&ctx.self_.memory) as i32)]
    })
}

/// `by_print` reads a death by the dead card's printed face, as v0.6.9l and before did (R767000).
fn eschews(radiant: bool, by_print: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            vec![cast_random(CastRandomArgs {
                query: CastRandomQuery::Fixed(json_as(
                    json!({ "anyTags": ["Human", "Book", "CN", "AI"] }),
                )),
                count: Some(CastRandomCount::Fixed(param(&*ctx, "casts"))),
                radiant: Some(radiant),
                target_enemies: None,
                afterward: None,
            })]
        })),
        triggers: vec![TriggerDef::new(
            "eschews-deaths",
            &[GameEventType::Destroyed],
            move |ctx, event| {
                let GameEvent::Destroyed {
                    controller,
                    def_id,
                    radiant,
                    ..
                } = event
                else {
                    return Vec::new();
                };
                if *controller != ctx.controller {
                    return Vec::new();
                }
                let unit = if by_print {
                    card_type_of_face(ctx.state, def_id, *radiant == Some(true)) == CardType::Unit
                } else {
                    is_unit_death(ctx.state, event)
                };
                if !unit {
                    return Vec::new();
                }
                let count = recalled(ctx, DEATHS).and_then(|value| value.as_i64()).unwrap_or(0) + 1;
                vec![remember(json_as(json!({ "key": DEATHS, "value": count })))]
            },
        )],
        // R403: Tributed at the next state check once the counted deaths reach the number.
        tribute_when: Some(read_hook(|read| {
            deaths_counted(&read.self_.memory) >= i64::from(param(&read, "deaths"))
        })),
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: eschews(false, false),
        radiant: eschews(true, false),
    }
}

/// R767000: a Unit's death read by the dead card's printed face (R4423000), through v0.6.9l;
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_9i() -> CardScripts {
    CardScripts {
        base: eschews(false, true),
        radiant: eschews(true, true),
    }
}

// M #60 Eschews — SPEC §8.8 row 60, BUILD M10 row M 60 (R70).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const ESCHEWS: &str = "meditative-060";
    const FILLER: &str = "core-005";
    /// (1) 3/3 Unit, Rush, First Strike, no Death: the counted deaths, each a one-shot Eclipse kill.
    const TIMMY: &str = "core-011";
    /// (1) Spell, deals 3 to a target; each cast makes the next Spell cost (1) less.
    const ECLIPSE: &str = "core-035";

    /// p1 holds Eschews (`radiant_face`) with five Timmies on the field to die and five Eclipses to
    /// kill them with on the same turn — before any end-of-turn cast could interfere.
    fn boarded(seed: &str, radiant_face: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [
                    { "def": ESCHEWS, "radiant": radiant_face },
                    ECLIPSE, ECLIPSE, ECLIPSE, ECLIPSE, ECLIPSE,
                    FILLER,
                ],
                "field": [TIMMY, TIMMY, TIMMY, TIMMY, TIMMY],
                "library": filler(3),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// Kill the Timmy in `lane` with an Eclipse.
    fn kill_timmy(s: &mut Scenario, lane: i32) {
        let id = s.unit(P1, lane).expect("a timmy").id.clone();
        s.play(ECLIPSE, pick::targets(pick::id(&id)));
    }

    fn eschews_on_field(s: &Scenario) -> bool {
        s.backrow(P1, 1).is_some_and(|card| card.def_id == ESCHEWS)
    }

    /// R2476000: on the field, the deaths it has counted after "of your Units have died", to both seats:
    /// 0 as it lands, 2 after two of your Timmies die.
    #[test]
    fn r2476000_it_previews_the_deaths_it_has_counted() {
        for radiant in [false, true] {
            let mut s = boarded("eschews-preview", radiant);
            s.play(ESCHEWS, json!({ "zone": 1 }));
            let eschews = s.card(ESCHEWS).id.clone();
            let label = "of your Units have died".to_string();
            assert_eq!(crate::preview_seen(&s, P1, &eschews), vec![(label.clone(), 0, None)], "radiant {radiant}");
            kill_timmy(&mut s, 1);
            kill_timmy(&mut s, 2);
            assert_eq!(crate::preview_seen(&s, P1, &eschews), vec![(label.clone(), 2, None)], "radiant {radiant}");
            assert_eq!(crate::preview_seen(&s, PlayerId::P2, &eschews), vec![(label, 2, None)], "radiant {radiant}");
        }
    }

    mod m60_eschews {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1082_end_of_turn_casts_a_human_book_cn_or_ai_card() {
                let mut s = scenario(json!({
                    "seed": "eschews-cast",
                    "p1": { "hand": [ESCHEWS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(ESCHEWS, json!({}));
                s.end_turn();

                let plays = s
                    .events()
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::CardPlayed { player, .. } if *player == P1)
                    })
                    .count();
                assert!(plays >= 2, "Eschews plus its cast both played");
            }

            #[test]
            fn r1082_five_deaths_tribute_it_at_the_next_state_check() {
                let mut s = boarded("eschews-deaths", false);
                s.play(ESCHEWS, json!({}));
                for lane in 2..=5 {
                    kill_timmy(&mut s, lane);
                }
                assert!(eschews_on_field(&s));
                kill_timmy(&mut s, 1);
                s.expect_in_zone(ESCHEWS, "graveyard");
            }

            #[test]
            fn deaths_read_through_param() {
                crate::register_all();
                let mut s = boarded("eschews-param", false);
                set_param(s.card_mut(ESCHEWS), "casts", 1);
                set_param(s.card_mut(ESCHEWS), "deaths", 2);
                s.play(ESCHEWS, json!({}));
                kill_timmy(&mut s, 1);
                assert!(eschews_on_field(&s));
                kill_timmy(&mut s, 2);
                s.expect_in_zone(ESCHEWS, "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn fifty_deaths_before_the_tribute() {
                let mut s = boarded("eschews-radiant-count", true);
                s.play(ESCHEWS, json!({}));
                // Five deaths do nothing on the Radiant face: it waits for fifty.
                for lane in 1..=5 {
                    kill_timmy(&mut s, lane);
                }
                assert!(eschews_on_field(&s));
            }

            #[test]
            fn the_cast_card_is_radiant() {
                let mut s = scenario(json!({
                    "seed": "eschews-radiant-cast",
                    "p1": {
                        "hand": [{ "def": ESCHEWS, "radiant": true }, FILLER],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(ESCHEWS, json!({}));
                s.end_turn();

                let radiant_cast = s.events().iter().any(|event| match event {
                    GameEvent::CardResolved {
                        player,
                        def_id,
                        radiant,
                        ..
                    } => *player == P1 && def_id != ESCHEWS && *radiant == Some(true),
                    _ => false,
                });
                assert!(radiant_cast, "the cast resolved Radiant");
            }
        }
    }

    /// R4423000: a death of yours is counted by what the card was on the field as it died.
    mod r4423000_what_it_died_as {
        use super::*;

        const MAGIC_JAMMED: &str = "core-036";
        const HIT_JOB: &str = "core-016";

        fn counted(s: &Scenario) -> i64 {
            deaths_counted(&s.card(ESCHEWS).memory)
        }

        /// p1 sets a Cloaked Toe Cracker face-down and destroys it with Magic Jammed before it reveals,
        /// its Eschews face-up in backrow lane 1; the set card's id.
        fn jammed() -> (Scenario, String) {
            let mut s = scenario(json!({
                "seed": "eschews-cloaked",
                "p1": {
                    "mana": 10,
                    "hand": [crate::CLOAKED_UNIT, MAGIC_JAMMED, FILLER],
                    "backrow": [{ "def": ESCHEWS, "faceUp": true, "lane": 1 }],
                    "library": filler(3),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let set = crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&set) }));
            (s, set)
        }

        #[test]
        fn r4423000_a_set_cloaked_unit_destroyed_before_it_reveals_is_no_unit_death_for_it() {
            let (s, set) = jammed();
            assert_eq!(crate::died_as_of(&s, &set), Some(CardType::FieldTrap));
            assert_eq!(counted(&s), 0);
        }

        /// p1's set Catnip animates after p2's Spell, and p2's Hit Job kills it in its unit zone.
        #[test]
        fn r4423000_an_animated_field_trap_killed_in_a_unit_zone_is_a_unit_death_for_it() {
            let mut s = scenario(json!({
                "seed": "eschews-animated",
                "active": "p2",
                "p1": {
                    "hand": [FILLER],
                    "backrow": [
                        { "def": ESCHEWS, "faceUp": true, "lane": 1 },
                        { "def": crate::ANIMATED_TRAP, "faceUp": false, "lane": 2 },
                    ],
                    "library": filler(3),
                },
                "p2": { "mana": 10, "hand": [FILLER, HIT_JOB, FILLER], "library": filler(4) },
            }));
            s.play(FILLER, json!({}));
            let catnip = s.card(crate::ANIMATED_TRAP).id.clone();
            assert!(s.unit(P1, 2).is_some_and(|unit| unit.id == catnip), "Catnip animated");
            s.play(HIT_JOB, json!({ "targets": pick::id(&catnip) }));

            assert_eq!(crate::died_as_of(&s, &catnip), Some(CardType::Unit));
            assert_eq!(counted(&s), 1);
        }

        /// R767000: v0.6.9l and before counted the set card's death, by the printed face's type.
        #[test]
        fn r767000_the_script_kept_for_v0_6_9i_counts_a_set_cloaked_unit_s_death() {
            crate::under_kept(ID, script_v0_6_9i(), || {
                let (s, _) = jammed();
                assert_eq!(counted(&s), 1);
            });
        }
    }
}
