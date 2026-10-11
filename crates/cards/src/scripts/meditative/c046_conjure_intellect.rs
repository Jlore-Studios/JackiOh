//! M #46 Conjure Intellect (SPEC §8.8 row 46): (2, embiggen 4) Spell, Epic.
//!
//! Base:    "Add a random KY card, a random CN card and a random Book to your hand.
//!           Paid (4): They cost ({setCost})."
//! Radiant: "Add a random Radiant KY card, a random Radiant CN card and a random Radiant Book to
//!           your hand. Paid (4): They cost ({setCost})."
//! Engine: three independent `add_random_from_catalog` draws, with `tags: [KY]`, then `[CN]`, then
//! `[Book]`: non-token cards of every set (R380, R382), never this card (R387), repeats allowed
//! (R60). `costOverride` is `setCost` when the embiggen price was paid (`ctx.embiggened`, R81, R65);
//! `radiant` is set on the Radiant face. The hand cap burns extras (§2.4, R4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-046";

/// §5.1's pool for one tag: the tag's non-token cards of every set, minus this card (R387).
fn pool(tag: &str) -> Value {
    json!({ "tags": [tag], "excludeDefId": ID })
}

fn conjure(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // "(paid 4: it costs 0)" — the embiggen price, read as the Cry resolves, so the paid
            // face is the declared `setCost` (R386).
            let paid = ctx.embiggened.then(|| param(&*ctx, "setCost"));
            ["KY", "CN", "Book"]
                .iter()
                .map(|tag| {
                    let mut args = json!({ "query": pool(tag), "count": 1 });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    if let Some(cost) = paid {
                        args["costOverride"] = json!(cost);
                    }
                    add_random_from_catalog(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: conjure(false),
        radiant: conjure(true),
    }
}

// M #46 Conjure Intellect — SPEC §8.8 row 46, BUILD M10 row M 46: "At (2) adds a random KY card, a
// random CN card and a random Book (each a non-token card of any set, never itself) at their
// printed costs; paid (4) they cost (0); the hand cap burns in that order; setCost reads through
// `param()`; radiant the three are Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const INTELLECT: &str = "meditative-046";
    const FILLER: &str = "core-005";

    /// TS `pool(ownId, args).map((def) => def.id)` (`crate::query`, SPEC §5.1).
    fn pool_ids(args: Value) -> Vec<String> {
        crate::query::pool(INTELLECT, &json_as(args))
            .iter()
            .map(|def| def.id.clone())
            .collect()
    }

    fn cast(seed: &str, radiant: bool, embiggen: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": INTELLECT, "radiant": radiant }, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 4,
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        s.play(
            INTELLECT,
            if embiggen {
                json!({ "embiggen": true })
            } else {
                json!({})
            },
        );
        s
    }

    /// The three cards the Cry added, in order.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    mod m46_conjure_intellect {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn adds_a_random_ky_a_random_cn_and_a_random_book_at_printed_costs() {
                let s = cast("intellect-base", false, false);
                let got = added(&s);
                assert_eq!(got.len(), 3);
                for (card, tag) in got.iter().zip(["KY", "CN", "Book"]) {
                    assert!(
                        pool_ids(json!({ "tags": [tag] })).contains(&card.def_id),
                        "{} is no {} card",
                        card.def_id,
                        tag
                    );
                    assert_ne!(card.def_id, INTELLECT);
                    assert_eq!(card.cost_override, None);
                    assert!(!card.radiant);
                }
            }

            #[test]
            fn paid_they_cost_0_read_through_param() {
                let s = cast("intellect-paid", false, true);
                let got = added(&s);
                assert_eq!(got.len(), 3);
                for card in &got {
                    assert_eq!(card.cost_override, Some(0));
                }
                // `setCost` is the declared number: tuned, the paid cards cost that.
                let mut s = scenario(json!({
                    "seed": "intellect-paid-tuned",
                    "p1": {
                        "hand": [INTELLECT, FILLER],
                        "library": [FILLER, FILLER, FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                set_param(s.card_mut(INTELLECT), "setCost", 1);
                s.play(INTELLECT, json!({ "embiggen": true }));
                for card in added(&s) {
                    assert_eq!(card.cost_override, Some(1));
                }
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_three_are_radiant() {
                let s = cast("intellect-radiant", true, false);
                let got = added(&s);
                assert_eq!(got.len(), 3);
                for (card, tag) in got.iter().zip(["KY", "CN", "Book"]) {
                    assert!(
                        pool_ids(json!({ "tags": [tag] })).contains(&card.def_id),
                        "{} is no {} card",
                        card.def_id,
                        tag
                    );
                    assert!(card.radiant);
                }
            }

            #[test]
            fn paid_the_radiant_three_cost_0() {
                let s = cast("intellect-radiant-paid", true, true);
                let got = added(&s);
                assert_eq!(got.len(), 3);
                for card in &got {
                    assert!(card.radiant);
                    assert_eq!(card.cost_override, Some(0));
                }
            }
        }
    }
}
