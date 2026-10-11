//! M #33 First Day of 学校 (SPEC §8.8 row 33): (0) Spell, CN, Rare.
//!
//! Base:    "Add {cards|random CN card|random CN cards} to your hand. Each costs ({discount}) less."
//! Radiant: "Add {cards|random Radiant CN card|random Radiant CN cards} to your hand. Each costs
//!           ({discount}) less."
//! Engine: `add_random_from_catalog({ query: { tags: [CN] }, count: cards, costMod: -discount,
//! radiant })` with the NEW `chinese` rider (ME-CN), set before the card moves as the Radiant rider
//! is, so a card the hand cap burns is Chinese in its graveyard too. The pool never holds this card
//! (R387), and R673 applies. As the designer asked, the printed text never says Chinese.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-033";

fn first_day(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let cards = param(&*ctx, "cards");
            let discount = param(&*ctx, "discount");
            let mut args = json!({
                "query": { "tags": ["CN"] },
                "count": cards,
                "costMod": -discount,
                "chinese": true,
            });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces add the same count at the same discount; the Radiant face's cards are Radiant.
    let base = first_day(false);
    let radiant = first_day(true);
    CardScripts { base, radiant }
}

// M #33 First Day of 学校 — SPEC §8.8 row 33, BUILD M10 row M 33: "Adds two random non-token CN
// cards of any set (R380), never itself (R387), each costing (1) less (an X card untouched, R65)
// and each carrying the `chinese` flag, which the owner's client shows in Chinese and the opponent
// never sees on a hidden card (MD-B12); a burned card reaches the graveyard Chinese; cards and
// discount read through `param()`; radiant the two cards are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FIRST_DAY: &str = "meditative-033";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 holds First Day (base unless `radiant_face`); both sides keep cards in hand so no turn
    /// auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": FIRST_DAY, "radiant": radiant_face }, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The two cards an add of `count` brings, by def id.
    fn added(s: &Scenario, before: usize) -> Vec<CardInstance> {
        s.hand(P1).into_iter().skip(before).collect()
    }

    mod m33_first_day_of_xuexiao {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn adds_two_cn_cards_costing_1_less_chinese_and_hidden() {
                let mut s = casting("first-day", false);
                let before = s.hand(P1).len();
                s.play(FIRST_DAY, json!({}));
                let new_cards = added(&s, before - 1);
                assert_eq!(new_cards.len(), 2);
                for card in &new_cards {
                    assert_ne!(card.def_id, FIRST_DAY, "never itself (R387)");
                    let def = crate::card_def(&card.def_id);
                    assert!(def.tags.contains(&Tag::Cn));
                    assert!(!def.token, "no token (R380)");
                    assert_eq!(card.cost_mod, -1);
                    assert_eq!(card.chinese, Some(true));
                    assert!(!card.radiant);
                }
                // The opponent sees a count, never the cards (MD-B12).
                let view: Value =
                    serde_json::to_value(view_for(s.state(), P2)).expect("the view serialises");
                assert_eq!(view["opponent"]["hand"], json!({ "count": 3 }));
            }

            #[test]
            fn a_burned_card_reaches_the_graveyard_chinese_at_its_printed_cost() {
                crate::register_all();
                // Nine in hand besides the Spell: the first add fits, the second burns.
                let mut s = scenario(json!({
                    "seed": "first-day-burn",
                    "p1": {
                        "hand": [{ "def": FIRST_DAY }, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(FIRST_DAY, json!({}));
                assert_eq!(s.hand(P1).len(), 10);
                let kept = s.hand(P1).last().cloned().expect("the kept card");
                assert_eq!(kept.chinese, Some(true));
                assert_eq!(kept.cost_mod, -1);
                let burned: Vec<&CardInstance> = s
                    .state()
                    .players
                    .p1
                    .graveyard
                    .iter()
                    .filter(|card| card.def_id != FIRST_DAY)
                    .collect();
                assert_eq!(burned.len(), 1);
                assert_eq!(burned[0].chinese, Some(true));
                assert_eq!(burned[0].cost_mod, 0);
            }

            #[test]
            fn cards_and_discount_read_through_param() {
                let mut s = casting("first-day-param", false);
                set_param(s.card_mut(FIRST_DAY), "cards", 1);
                set_param(s.card_mut(FIRST_DAY), "discount", 2);
                let before = s.hand(P1).len();
                s.play(FIRST_DAY, json!({}));
                let new_cards = added(&s, before - 1);
                assert_eq!(new_cards.len(), 1);
                assert_eq!(new_cards[0].cost_mod, -2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_two_cards_are_radiant() {
                let mut s = casting("first-day-radiant", true);
                let before = s.hand(P1).len();
                s.play(FIRST_DAY, json!({}));
                let new_cards = added(&s, before - 1);
                assert_eq!(new_cards.len(), 2);
                for card in &new_cards {
                    assert_ne!(card.def_id, FIRST_DAY, "never itself (R387)");
                    let def = crate::card_def(&card.def_id);
                    assert!(def.tags.contains(&Tag::Cn));
                    assert!(card.radiant);
                    assert_eq!(card.cost_mod, -1);
                    assert_eq!(card.chinese, Some(true));
                }
            }
        }
    }
}
