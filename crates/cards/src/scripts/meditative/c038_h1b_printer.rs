//! M #38 H1B Printer (SPEC §8.8 row 38): (3) Field Spell, CN, KY, Rare.
//!
//! Base:    "Start of turn: Add a random Radiant KY or CN card to your hand."
//! Radiant: "Start of turn: Add a random Radiant KY or CN card to your hand. It costs (0)."
//! Engine: at the start of its controller's turn, `add_random_from_catalog({ query: { anyTags:
//! [KY, CN] }, radiant: true })`, with `costOverride: 0` on the Radiant face. `anyTags` asks for
//! any one of the tags (R1422); the Printer itself is excluded (R387), and R673 applies. The cards
//! it makes are not Chinese (⚠ designer).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-038";

fn printer(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            let mut args = json!({ "query": { "anyTags": ["KY", "CN"] }, "radiant": true });
            if radiant {
                args["costOverride"] = json!(0);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces add a Radiant KY or CN card; the Radiant face's costs (0).
    let base = printer(false);
    let radiant = printer(true);
    CardScripts { base, radiant }
}

// M #38 H1B Printer — SPEC §8.8 row 38, BUILD M10 row M 38: "At your start of turn only, adds one
// random Radiant non-token card with the KY tag, the CN tag or both (`anyTags`, MD-B20), never
// itself (R387), at its printed cost and not Chinese; a card with neither tag never appears; the
// hand cap burns it; hidden from the opponent; radiant it costs (0)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PRINTER: &str = "meditative-038";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1's Printer stands in the backrow (base unless `radiant_face`); both sides hold and draw
    /// fillers, so turns pass normally.
    fn standing(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": PRINTER, "radiant": radiant_face, "lane": 1 }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// Whether the catalog prints `def_id` with the KY tag, the CN tag or both.
    fn is_ky_or_cn(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        def.tags.contains(&Tag::Ky) || def.tags.contains(&Tag::Cn)
    }

    /// The cards the Printer added to p1's hand at this start of turn, as `addedToHand` names them
    /// (M #37 CN in a Bottle among them: added rather than drawn, it stays in the hand as itself).
    fn printed_at_start(s: &mut Scenario) -> Vec<(String, String)> {
        let mark = s.events().len();
        s.start_turn();
        let events = &s.events()[mark..];
        // The turn draw arrives too: it is the one with a `drawn` event.
        let drawn: Vec<&String> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand {
                    player,
                    instance_id,
                    def_id,
                } if *player == P1 && !drawn.contains(&instance_id) => {
                    Some((instance_id.clone(), def_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    mod m38_h1b_printer {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r927_start_of_turn_adds_one_radiant_ky_or_cn_card_not_chinese() {
                let mut s = standing("printer", false);
                let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                // The turn draw plus the Printer's card.
                s.start_turn();
                let hand = s.hand(P1);
                assert_eq!(hand.len(), before.len() + 2);
                let made: Vec<&CardInstance> =
                    hand.iter().filter(|card| !before.contains(&card.id)).collect();
                assert_eq!(made.len(), 2);
                let printed = made
                    .into_iter()
                    .find(|card| card.radiant)
                    .expect("the printed card");
                assert_ne!(printed.def_id, PRINTER, "never itself (R387)");
                assert!(is_ky_or_cn(&printed.def_id));
                assert!(!crate::card_def(&printed.def_id).token);
                assert_eq!(printed.chinese, None);
                assert_eq!(printed.cost_override, None);
                // Hidden from the opponent: a count, never the card.
                let view: Value = serde_json::to_value(view_for(s.state(), P2)).expect("the view serialises");
                assert_eq!(view["opponent"]["hand"], json!({ "count": hand.len() }));
            }

            #[test]
            fn nothing_on_the_opponent_s_turn() {
                let mut s = standing("printer-foe", false);
                let before = s.hand(P1).len();
                // p2's whole turn: p1's hand does not move.
                s.end_turn();
                assert_eq!(s.hand(P1).len(), before);
                // p1's next start of turn: exactly one printed card arrives.
                let ids: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                s.end_turn();
                let held = s.hand(P1);
                let fresh: Vec<&CardInstance> = held.iter().filter(|card| !ids.contains(&card.id)).collect();
                // The turn draw plus the one printed card.
                assert_eq!(fresh.len(), 2);
                assert_eq!(fresh.iter().filter(|card| card.radiant).count(), 1);
            }
            #[test]
            fn r387_never_itself() {
                // The Printer is a KY and CN card, so the pool would hold it but for R387.
                let unexcluded = jackioh_engine::catalog::query(&json_as(json!({ "anyTags": ["KY", "CN"] })));
                assert!(unexcluded.iter().any(|def| def.id == PRINTER));
                for n in 0..60 {
                    let mut s = standing(&format!("printer-self-{n}"), false);
                    let printed = printed_at_start(&mut s);
                    assert_eq!(printed.len(), 1, "one card a start of turn");
                    assert_ne!(printed[0].1, PRINTER, "never itself (R387)");
                }
            }

            #[test]
            fn a_card_with_neither_tag_never_appears() {
                for n in 0..60 {
                    let mut s = standing(&format!("printer-tags-{n}"), false);
                    for (_, def_id) in printed_at_start(&mut s) {
                        assert!(is_ky_or_cn(&def_id), "{def_id} has neither KY nor CN");
                        assert!(!crate::card_def(&def_id).token, "{def_id} is a token");
                    }
                }
            }

            #[test]
            fn at_its_printed_cost_and_not_chinese() {
                let mut s = standing("printer-cost", false);
                let printed = printed_at_start(&mut s);
                assert_eq!(printed.len(), 1);
                let card = s.card(&printed[0].0).clone();
                assert!(card.radiant);
                assert_eq!(card.cost_override, None, "at its printed cost");
                assert_eq!(card.cost_mod, 0);
                assert_eq!(card.chinese, None, "not Chinese");
            }

            #[test]
            fn the_hand_cap_burns_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "printer-burn",
                    "p1": {
                        "hand": filler(HAND_CAP as usize),
                        "backrow": [{ "def": PRINTER, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.start_turn();
                assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the cap holds");
                assert!(s.hand(P1).iter().all(|card| card.def_id == FILLER));
                let burned: Vec<CardInstance> = s
                    .pile(P1, "graveyard")
                    .into_iter()
                    .filter(|card| card.def_id != FILLER)
                    .collect();
                assert_eq!(burned.len(), 1, "the printed card burned");
                assert!(is_ky_or_cn(&burned[0].def_id));
            }

            #[test]
            fn hidden_from_the_opponent() {
                let mut s = standing("printer-hidden", false);
                let printed = printed_at_start(&mut s);
                let (id, def_id) = printed[0].clone();
                // p2 reads a count of p1's hand, and no event of its stream names the card.
                let view = s.view(P2);
                let text = serde_json::to_string(&view).expect("the view serialises");
                assert!(!text.contains(&id), "p2 never reads the printed card's id");
                assert!(!text.contains(&def_id), "p2 never reads {def_id}");
                let mine = serde_json::to_string(&s.view(P1)).expect("the view serialises");
                assert!(mine.contains(&id), "p1 reads its own card");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_printed_card_costs_0() {
                let mut s = standing("printer-radiant", true);
                let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                s.start_turn();
                let held = s.hand(P1);
                let made: Vec<&CardInstance> =
                    held.iter().filter(|card| !before.contains(&card.id)).collect();
                let printed = made
                    .into_iter()
                    .find(|card| card.radiant)
                    .expect("the printed card");
                assert!(is_ky_or_cn(&printed.def_id));
                assert_eq!(printed.cost_override, Some(0));
                assert_eq!(printed.chinese, None);
            }
        }
    }
}
