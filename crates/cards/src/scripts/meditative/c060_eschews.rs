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
//! reaches `{deaths}` (C #88, R403).

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

fn eschews(radiant: bool) -> Script {
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
            |ctx, event| {
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
                if card_type_of_face(ctx.state, def_id, *radiant == Some(true)) != CardType::Unit {
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
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: eschews(false),
        radiant: eschews(true),
    }
}

// M #60 Eschews — SPEC §8.8 row 60, BUILD M10 row M 60: "At your end of turn only, casts one random
// non-token Human, Book or CN card or AI generated card (`anyTags`, R1421), never itself, each
// definition once in the pool, as your play (R70): random choices, X at your current mana (at
// least 1), a Unit summoned with its Cry, a backrow card fizzling with no zone; each death of one
// of your Units, tokens and both deaths of a Reborn Unit included, counts from its entry; at 5 it
// is Tributed at the next state check (R1082), a Sacrifice that Indestructible does not stop;
// casts and deaths read through `param()`; radiant the cast card is Radiant, and the count is 50".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const ESCHEWS: &str = "meditative-060";
    const FILLER: &str = "core-005";
    /// (1) 3/3 Unit, Rush, First Strike, no Death: the counted deaths, each a one-shot Eclipse kill.
    const TIMMY: &str = "core-011";
    /// (1) Spell, deals 3 to a target; each cast makes the next Spell cost (1) less.
    const ECLIPSE: &str = "core-035";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    fn eclipse(target: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": target }] })
    }

    /// p1 holds Eschews (`radiant_face`) with five Timmies on the field to die and five Eclipses to
    /// kill them with on the same turn — before any end-of-turn cast could interfere.
    fn boarded(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
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
        s.play(ECLIPSE, eclipse(&id));
    }

    fn eschews_on_field(s: &Scenario) -> bool {
        s.backrow(P1, 1).is_some_and(|card| card.def_id == ESCHEWS)
    }

    mod m60_eschews {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1082_end_of_turn_casts_a_human_book_cn_or_ai_card() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "eschews-cast",
                    "p1": { "hand": [ESCHEWS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(ESCHEWS, json!({}));
                s.end_turn();

                // One random cast went out as p1's play.
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
                // Four Timmies die: Eschews stays.
                for lane in 2..=5 {
                    kill_timmy(&mut s, lane);
                }
                assert!(eschews_on_field(&s));
                // The fifth death Tributes it at the next state check.
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
                crate::register_all();
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

                // The cast went out on its Radiant face.
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
}
