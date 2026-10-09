//! M #38 H1B Printer (SPEC §8.8 row 38): (2) Field Spell, CN, KY, Rare.
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
                let printed = made.into_iter().find(|card| card.radiant).expect("the printed card");
                assert_ne!(printed.def_id, PRINTER, "never itself (R387)");
                assert!(is_ky_or_cn(&printed.def_id));
                assert!(!crate::card_def(&printed.def_id).token);
                assert_eq!(printed.chinese, None);
                assert_eq!(printed.cost_override, None);
                // Hidden from the opponent: a count, never the card.
                let view: Value =
                    serde_json::to_value(view_for(s.state(), P2)).expect("the view serialises");
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
                let hand = s.hand(P1);
                let fresh: Vec<&CardInstance> = hand
                    .iter()
                    .filter(|card| !ids.contains(&card.id))
                    .collect();
                // The turn draw plus the one printed card.
                assert_eq!(fresh.len(), 2);
                assert_eq!(fresh.iter().filter(|card| card.radiant).count(), 1);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_printed_card_costs_0() {
                let mut s = standing("printer-radiant", true);
                let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                s.start_turn();
                let hand = s.hand(P1);
                let made: Vec<&CardInstance> =
                    hand.iter().filter(|card| !before.contains(&card.id)).collect();
                let printed = made.into_iter().find(|card| card.radiant).expect("the printed card");
                assert!(is_ky_or_cn(&printed.def_id));
                assert_eq!(printed.cost_override, Some(0));
                assert_eq!(printed.chinese, None);
            }
        }
    }
}
