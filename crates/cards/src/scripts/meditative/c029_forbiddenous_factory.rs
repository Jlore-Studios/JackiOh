//! M #29 Forbiddenous Factory (SPEC §8.8 row 29): (3) Field Spell, Plague, Rare.
//!
//! Base and Radiant: "Cry: Place {counters|Plague Counter|Plague Counters} on this.
//! End of turn: Spend a Plague Counter from this to shuffle {curses|Ancient Curse|Ancient Curses}
//! into your opponent's deck."
//! Engine:
//! - **Cry** (only when played, R1): `place_plague({ amount: counters })` on itself. That is one
//!   placement of 3 (§6.3 Plague Counter, R689); R471's multiplier applies to it.
//! - **End of turn** (the controller's): when the running card's `counters.plague` is at least 1,
//!   run `consume_plague({ amount: 1 })` and then the same `shuffle_into` as M #28. With no counter
//!   it does nothing ("Spend … to …" is a price).
//! - Other Plague cards can refill it: C #53 Plague Crawler, C #61, C #76 and C #42's Activate.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-029";

/// M #28.1 Ancient Curse, the token the end-of-turn clause shuffles (§7).
const CURSE: &str = "meditative-028-1";

fn factory() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![place_plague(json_as(json!({ "amount": param(&*ctx, "counters") })))]
        })),
        end_of_turn: Some(hook(|ctx| {
            let stocked = ctx.live_self().is_some_and(|me| plague_on(me) >= 1);
            if !stocked {
                return vec![];
            }
            vec![
                consume_plague(json_as(json!({ "amount": 1 }))),
                shuffle_into(json_as(json!({
                    "defId": CURSE,
                    "count": param(&*ctx, "curses"),
                    "player": "enemy",
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces share the one script: the declared `counters` (3 and 3) and `curses` (1,
    // radiant 2) are what differ.
    let base = factory();
    let radiant = factory();
    CardScripts { base, radiant }
}

// M #29 Forbiddenous Factory — SPEC §8.8 row 29, BUILD M10 row M 29: "Cry (played or cast): one
// placement of 3 Plague Counters on itself (`counterChanged`), R471's multiplier applying; at your
// end of turn only, with a counter, one is removed and one Ancient Curse is shuffled into the
// opponent's deck; at 0 counters nothing is shuffled; three turns empty it, and a refill (C 53)
// extends it; counters and curses read through `param()`; radiant two Curses per counter". The name
// keeps the designer's spelling, "Forbiddenous".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P2: PlayerId = PlayerId::P2;

    const FACTORY: &str = "meditative-029";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// The `counterChanged` events, as JSON.
    fn changes(s: &Scenario) -> Vec<Value> {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CounterChanged { .. }))
            .map(crate::js)
            .collect()
    }

    /// The Ancient Curses in `player`'s library.
    fn curses_in(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.state().players[player]
            .library
            .iter()
            .filter(|card| card.def_id == CURSE)
            .cloned()
            .collect()
    }

    /// p1 plays the Factory (base unless `radiant_face`).
    fn played(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": FACTORY, "radiant": radiant_face }, FILLER],
                "library": filler(6),
                "mana": 4,
            },
            "p2": { "hand": [FILLER, FILLER], "library": filler(6) },
        }));
        s.play(FACTORY, json!({}));
        s
    }

    /// The Factory standing with no counters (as placed, never through its Cry).
    fn standing() -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": "factory-standing",
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": FACTORY, "lane": 1 }],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER, FILLER], "library": filler(6) },
        }))
    }

    mod m29_forbiddenous_factory {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn cry_places_one_placement_of_3_counters_on_itself() {
                let s = played("factory-cry", false);
                let me = s.card(FACTORY).clone();

                assert_eq!(me.counters.plague, Some(3));
                assert_eq!(
                    changes(&s),
                    vec![json!({
                        "type": "counterChanged",
                        "instanceId": me.id,
                        "counter": "plague",
                        "value": 3,
                        "placed": 3,
                    })]
                );
            }

            #[test]
            fn at_your_end_of_turn_a_counter_is_spent_for_one_curse() {
                let mut s = played("factory-spend", false);

                s.end_turn();

                assert_eq!(s.card(FACTORY).counters.plague, Some(2));
                assert_eq!(curses_in(&s, P2).len(), 1);
                let curse = curses_in(&s, P2).into_iter().next().expect("a curse");
                assert_eq!(curse.owner, P2);
                s.expect_events(json!(["counterChanged", "shuffledIn"]));
            }

            #[test]
            fn with_no_counter_nothing_is_shuffled() {
                let mut s = standing();
                let before = s.state().players[P2].library.len();

                s.end_turn();

                assert_eq!(s.card(FACTORY).counters.plague, None);
                // p2's own turn-start draw is the only movement: no curse went in.
                assert_eq!(s.state().players[P2].library.len(), before - 1);
                assert!(curses_in(&s, P2).is_empty());
                assert!(
                    !s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::ShuffledIn { .. }))
                );
            }

            #[test]
            fn three_ends_of_turn_empty_it() {
                let mut s = played("factory-empty", false);

                s.end_turn();
                s.end_turn();
                s.end_turn();
                s.end_turn();
                s.end_turn();
                s.end_turn();

                assert_eq!(s.card(FACTORY).counters.plague, None);
                assert_eq!(curses_in(&s, P2).len(), 3);
            }

            #[test]
            fn does_nothing_at_the_opponent_s_end_of_turn() {
                let mut s = played("factory-foe-turn", false);

                s.end_turn();
                assert_eq!(s.card(FACTORY).counters.plague, Some(2));
                assert_eq!(curses_in(&s, P2).len(), 1);

                s.end_turn();
                assert_eq!(s.card(FACTORY).counters.plague, Some(2));
                assert_eq!(curses_in(&s, P2).len(), 1);
            }

            #[test]
            fn numbers_read_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "factory-param",
                    "p1": { "hand": [FACTORY, FILLER], "library": filler(6), "mana": 4 },
                    "p2": { "hand": [FILLER, FILLER], "library": filler(6) },
                }));
                assert_eq!(crate::upgrade_number(&mut s, FACTORY, "counters"), 4);
                assert_eq!(crate::upgrade_number(&mut s, FACTORY, "curses"), 2);

                s.play(FACTORY, json!({}));
                assert_eq!(s.card(FACTORY).counters.plague, Some(4));

                s.end_turn();
                assert_eq!(s.card(FACTORY).counters.plague, Some(3));
                // Two Curses went in, one `shuffledIn` each. p2's own draw may land on one and
                // cast it on the same turn, so the total spans the library and the graveyards.
                let shuffled = s
                    .events()
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::ShuffledIn { def_id, .. } if def_id == CURSE)
                    })
                    .count();
                assert_eq!(shuffled, 2);
                let cast =
                    s.pile(P2, "graveyard").iter().filter(|card| card.def_id == CURSE).count();
                assert_eq!(curses_in(&s, P2).len() + cast, 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn two_curses_per_counter() {
                let mut s = played("factory-radiant", true);

                assert_eq!(s.card(FACTORY).counters.plague, Some(3));

                s.end_turn();

                assert_eq!(s.card(FACTORY).counters.plague, Some(2));
                assert_eq!(curses_in(&s, P2).len(), 2);
            }
        }
    }
}
