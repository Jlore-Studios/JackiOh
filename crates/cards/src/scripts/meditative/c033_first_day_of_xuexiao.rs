//! M #33 First Day of 学校 (SPEC §8.8 row 33): (1) Spell, CN, Rare.
//!
//! Base:    "Add {cards|random CN card|random CN cards} to your hand." — 3 (2 until issue #3996).
//! Radiant: "Add {cards|random Radiant CN card|random Radiant CN cards} to your hand." — 3.
//! Engine: the `chinese` rider (ME-CN) is set before the card moves, as the Radiant rider is, so a
//! card the hand cap burns is Chinese in its graveyard too. The pool never holds this card (R387),
//! and R673 applies. As the designer asked, the printed text never says Chinese. Through v0.6.5b each
//! card also cost ({discount}) less, which `script_v0_6_5b` keeps for those replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-033";

/// `discounted`: each card also costs `discount` less, as through v0.6.5b.
fn first_day(radiant: bool, discounted: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "query": { "tags": ["CN"] },
                "count": param(&*ctx, "cards"),
                "chinese": true,
            });
            if discounted {
                args["costMod"] = json!(-param(&*ctx, "discount"));
            }
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = first_day(false, false);
    let radiant = first_day(true, false);
    CardScripts { base, radiant }
}

/// R767000: through v0.6.5b each card it added cost ({discount}) less (issue #3996); `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    let base = first_day(false, true);
    let radiant = first_day(true, true);
    CardScripts { base, radiant }
}

// M #33 First Day of 学校 — SPEC §8.8 row 33, BUILD M10 row M 33: three random non-token CN cards of any
// set (R380) at their printed cost since issue #3996 (two, each costing (1) less, before).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FIRST_DAY: &str = "meditative-033";
    const FILLER: &str = "core-005";

    /// p1 holds First Day (base unless `radiant_face`); both sides keep cards in hand so no turn
    /// auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": FIRST_DAY, "radiant": radiant_face }, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The cards the add brought.
    fn added(s: &Scenario, before: usize) -> Vec<CardInstance> {
        crate::hand_and_summoned_tatches(s, P1).into_iter().skip(before).collect()
    }

    mod m33_first_day_of_xuexiao {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn adds_three_cn_cards_at_their_printed_cost_chinese_and_hidden() {
                let mut s = casting("first-day", false);
                let before = s.hand(P1).len();
                s.play(FIRST_DAY, json!({}));
                let new_cards = added(&s, before - 1);
                assert_eq!(new_cards.len(), 3);
                for card in &new_cards {
                    assert_ne!(card.def_id, FIRST_DAY, "never itself (R387)");
                    let def = crate::card_def(&card.def_id);
                    assert!(def.tags.contains(&Tag::Cn));
                    assert!(!def.token, "no token (R380)");
                    assert_eq!(card.cost_mod, 0, "no discount since issue #3996");
                    assert_eq!(card.chinese, Some(true));
                    assert!(!card.radiant);
                }
                // The opponent sees a count, never the cards (MD-B12).
                let view: Value = js(&view_for(s.state(), P2));
                assert_eq!(view["opponent"]["hand"], json!({ "count": 4 }));
            }

            #[test]
            fn a_burned_card_reaches_the_graveyard_chinese_at_its_printed_cost() {
                crate::register_all();
                // Nine in hand besides the Spell: the first add fits, the second and third burn.
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
                assert_eq!(kept.cost_mod, 0);
                let burned: Vec<&CardInstance> = s
                    .state()
                    .players
                    .p1
                    .graveyard
                    .iter()
                    .filter(|card| card.def_id != FIRST_DAY)
                    .collect();
                assert_eq!(burned.len(), 2);
                for card in burned {
                    assert_eq!(card.chinese, Some(true));
                    assert_eq!(card.cost_mod, 0);
                }
            }

            #[test]
            fn cards_reads_through_param() {
                let mut s = casting("first-day-param", false);
                set_param(s.card_mut(FIRST_DAY), "cards", 1);
                let before = s.hand(P1).len();
                s.play(FIRST_DAY, json!({}));
                let new_cards = added(&s, before - 1);
                assert_eq!(new_cards.len(), 1);
                assert_eq!(new_cards[0].cost_mod, 0);
            }

            /// R767000: through v0.6.5b it added two, each costing ({discount}) less (issue #3996): the
            /// kept script under v0.6.5b's numbers.
            #[test]
            fn r767000_the_script_kept_for_v0_6_5b_discounts_each_card_it_adds() {
                let old = |def: &mut CardDef| {
                    def.params = json_as(json!([
                        { "key": "cards", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                        { "key": "discount", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 },
                    ]));
                };
                crate::under_kept_as(FIRST_DAY, script_v0_6_5b(), old, || {
                    for radiant in [false, true] {
                        let mut s = casting("first-day-kept", radiant);
                        let before = s.hand(P1).len();
                        s.play(FIRST_DAY, json!({}));
                        let new_cards = added(&s, before - 1);
                        assert_eq!(new_cards.len(), 2);
                        for card in &new_cards {
                            assert_eq!(card.cost_mod, -1);
                            assert_eq!(card.radiant, radiant);
                        }
                    }
                });
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_three_cards_are_radiant_at_their_printed_cost() {
                let mut s = casting("first-day-radiant", true);
                let before = s.hand(P1).len();
                s.play(FIRST_DAY, json!({}));
                let new_cards = added(&s, before - 1);
                assert_eq!(new_cards.len(), 3);
                for card in &new_cards {
                    assert_ne!(card.def_id, FIRST_DAY, "never itself (R387)");
                    let def = crate::card_def(&card.def_id);
                    assert!(def.tags.contains(&Tag::Cn));
                    assert!(card.radiant);
                    assert_eq!(card.cost_mod, 0);
                    assert_eq!(card.chinese, Some(true));
                }
            }
        }
    }
}
