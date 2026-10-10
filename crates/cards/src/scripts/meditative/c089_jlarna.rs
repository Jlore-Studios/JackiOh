//! M #89 Jlarna (SPEC §8.8 row 89). (3) Field Spell, Rare.
//!
//! Base:    "Aura: Credit line {credit}: you can spend mana you don't have, owing up to {credit} at
//!          once. Pay it back in {instalments|instalment|instalments}, one at the start of each of
//!          your next turns.\nEnd of turn: If you didn't use your credit line this turn, Tribute this."
//! Radiant: the Aura line alone.
//! Engine:  "The Aura, a credit line paid in 4 (MD-E13, rewritten): while Jlarna acts on its
//!          controller's field, any mana they spend may go past what they have (R1223) — current mana
//!          first, the shortfall borrowed, never owing more than 4 at once; one turn's debt is split
//!          into 4 instalments as evenly as possible, larger first; one falls due at each of the next
//!          four refreshes, taken off what the refresh gives (R1224); a short refresh forgives the
//!          rest; the debt outlives Jlarna; several Jlarnas share one line. The base face is Tributed
//!          at the end of a turn its controller borrowed nothing on — the turn it is played included
//!          (R1225). Tunes: credit 4 ↑; instalments 4 ↑."
//!
//! The Aura is the engine's credit line (`creditLine`, `creditInstalments`), read through the
//! declared numbers (R386); the lapsing face carries `creditLapses`. The end-of-turn Tribute is the
//! card's own hook, the way #54 Money Machine and #60 Eschews read theirs: a Sacrifice of itself
//! when the turn's log records no borrowing.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-089";

/// The catalog's declared numbers, read off the face that is up (R386).
fn declared(key: &str) -> Param {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == key))
    {
        Some(param) => param.clone(),
        None => panic!("meditative-089 declares {key} (catalog params)"),
    }
}

fn jlarna(radiant: bool) -> Script {
    let credit = declared("credit");
    let instalments = declared("instalments");
    let (line, parts) = if radiant {
        (credit.radiant, instalments.radiant)
    } else {
        (credit.base, instalments.base)
    };
    Script {
        static_flags: Some(StaticFlags {
            credit_line: Some(line),
            credit_instalments: Some(parts),
            credit_lapses: if radiant { None } else { Some(true) },
            ..StaticFlags::default()
        }),
        // R1225: use it or lose it — at the end of its controller's turn, with no borrowing this
        // turn, Jlarna is Tributed by its own text. Its own price can't be borrowed (the Aura acts
        // only once it is on the field), so the turn it is played is not exempt.
        end_of_turn: if radiant {
            None
        } else {
            Some(hook(|ctx| {
                if credit_used_this_turn(ctx.state, ctx.controller) {
                    vec![]
                } else {
                    vec![sacrifice(json_as(json!({ "target": { "of": "self" } })))]
                }
            }))
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jlarna(false),
        radiant: jlarna(true),
    }
}

// M #89 Jlarna — SPEC §8.8 row 89, BUILD M10 row M 89: "A (3) Field Spell Aura credit line: spend
// mana you don't have, owing up to 4 at once, paid back in 4 instalments over your next four
// refreshes (R1223, R1224); missed instalments forgiven; the base face is Tributed at the end of a
// turn you borrowed nothing on, the turn it is played included (R1225); credit and instalments read
// through `param()`; radiant never tributes itself".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::js;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-016";
    const VANILLA: &str = "core-008"; // (1) 4/4

    fn fielded(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [VANILLA, VANILLA, VANILLA, VANILLA, FILLER],
                "library": [FILLER, FILLER],
                "mana": 0,
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    mod m89_jlarna {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1225_played_and_nothing_borrowed_it_is_tributed() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = crate::scenario(json!({
                    "seed": "jlarna-lapses",
                    "p1": { "hand": [{ "def": ID }, VANILLA], "library": [FILLER, FILLER], "mana": 10 },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(ID, json!({ "zone": 1 }));
                s.expect_in_zone(ID, "field");
                // Nothing borrowed for the rest of the turn: the end of turn tributes it.
                s.end_turn();
                s.expect_in_zone(ID, "graveyard");
            }

            #[test]
            fn r1225_borrowed_after_playing_it_stays() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = crate::scenario(json!({
                    "seed": "jlarna-stays",
                    "p1": { "hand": [{ "def": ID }, VANILLA], "library": [FILLER, FILLER], "mana": 3 },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(ID, json!({ "zone": 1 }));
                // Jlarna cost 3 of 3; a (1) vanilla now borrows 1 of the line.
                s.play(VANILLA, json!({ "zone": 1 }));
                assert_eq!(s.state().players.p1.turn_log.mana_borrowed, Some(1));
                s.end_turn();
                s.expect_in_zone(ID, "field");
            }

            #[test]
            fn r1223_its_own_price_is_never_borrowed() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = crate::scenario(json!({
                    "seed": "jlarna-price",
                    "p1": { "hand": [{ "def": ID }, FILLER], "library": [FILLER, FILLER], "mana": 2 },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                // (3) with 2 mana and no line yet: refused, and the listing hides it.
                s.expect_refused(|s| s.play(ID, json!({ "zone": 1 })));
                let id = s.card(ID).id.clone();
                assert!(
                    legal_actions(s.state(), P1).iter().map(js).all(|action| action["instanceId"] != json!(id)),
                    "unaffordable even with the line it would open",
                );
            }

            #[test]
            fn r1224_a_shortfall_of_3_is_paid_1_1_1_0() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = fielded("jlarna-split", false);
                // Borrow 3 of nothing: the schedule is 1, 1, 1.
                s.play(VANILLA, json!({ "zone": 1 }));
                s.play(VANILLA, json!({ "zone": 2 }));
                s.play(VANILLA, json!({ "zone": 3 }));
                assert_eq!(s.state().players.p1.owed_instalments, Some(vec![1, 1, 1]));
            }

            #[test]
            fn r1225_no_check_on_the_opponents_turn() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = fielded("jlarna-foe", false);
                // Borrow on p1's turn, so its end of turn keeps Jlarna; p2's end of turn is no
                // check either, and Jlarna is still standing on p1's next turn.
                s.play(VANILLA, json!({ "zone": 1 }));
                s.end_turn();
                s.expect_in_zone(ID, "field");
                s.end_turn();
                s.expect_in_zone(ID, "field");
            }

            #[test]
            fn r1225_a_line_at_its_limit_lapses() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = fielded("jlarna-full", false);
                // Owe the full 4: the turn's borrowing keeps Jlarna this time...
                s.play(VANILLA, json!({ "zone": 1 }));
                s.play(VANILLA, json!({ "zone": 2 }));
                s.play(VANILLA, json!({ "zone": 3 }));
                s.play(VANILLA, json!({ "zone": 4 }));
                assert_eq!(owed_mana_of(s.state(), P1), 4);
                assert_eq!(credit_available(s.state(), P1), 0);
                s.end_turn();
                s.expect_in_zone(ID, "field");
                // ...nothing more can be borrowed while the full 4 is owed...
                assert_eq!(spendable_mana(s.state(), P1), s.state().players.p1.mana.current);
                s.end_turn();
                // ...and the instalment p1's refresh takes frees 1 of the limit again. Unused, the
                // line lapses at that turn's end.
                assert_eq!(owed_mana_of(s.state(), P1), 3);
                assert_eq!(credit_available(s.state(), P1), 1);
                s.end_turn();
                s.expect_in_zone(ID, "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_never_tributes_itself() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = fielded("jlarna-radiant", true);
                // A turn with nothing borrowed: the Radiant face has no self-Tribute.
                assert!(!credit_used_this_turn(s.state(), P1));
                s.end_turn();
                s.expect_in_zone(ID, "field");
                s.end_turn();
                // Its credit line still lends: p1's next turn, with no mana left, borrows 1.
                s.state_mut().players.p1.mana.current = 0;
                s.play(VANILLA, json!({ "zone": 1 }));
                assert_eq!(s.state().players.p1.owed_instalments, Some(vec![1]));
                s.end_turn();
                s.expect_in_zone(ID, "field");
            }

            #[test]
            fn its_numbers_read_through_param() {
                let def = crate::card_def(ID);
                let number = |key: &str| {
                    def.params
                        .as_ref()
                        .and_then(|params| params.iter().find(|entry| entry.key == key))
                        .expect(key)
                        .clone()
                };
                assert_eq!((number("credit").base, number("credit").radiant), (4, 4));
                assert_eq!((number("instalments").base, number("instalments").radiant), (4, 4));
                let entry = registered_entry(ID).expect("registered");
                let flags = |face: &Script| face.static_flags.clone().unwrap_or_default();
                assert_eq!(flags(&entry.base).credit_line, Some(4));
                assert_eq!(flags(&entry.base).credit_instalments, Some(4));
                assert_eq!(flags(&entry.base).credit_lapses, Some(true));
                assert_eq!(flags(&entry.radiant).credit_lapses, None);
            }
        }
    }
}
