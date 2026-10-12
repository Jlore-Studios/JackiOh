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
                // What each pick added, as `addedToHand` names it: a card may change once in the
                // hand (M #37 CN in a Bottle transforms as it arrives), so the hand alone cannot say.
                let picks: Vec<(String, String)> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AddedToHand {
                            player,
                            instance_id,
                            def_id,
                        } if *player == P1 => Some((instance_id.clone(), def_id.clone())),
                        _ => None,
                    })
                    .collect();
                assert_eq!(picks.len(), 3);
                for ((_, pick), tag) in picks.iter().zip(["KY", "CN", "Book"]) {
                    assert!(
                        pool_ids(json!({ "tags": [tag] })).contains(pick),
                        "{pick} is no {tag} card"
                    );
                    assert_ne!(pick, INTELLECT);
                }
                // Each card still as it was added: at its printed cost, on its base face.
                for card in got
                    .iter()
                    .filter(|card| picks.iter().any(|(id, _)| *id == card.id))
                {
                    assert_eq!(card.cost_override, None);
                    assert!(!card.radiant);
                }
            }

            #[test]
            fn paid_they_cost_0() {
                let s = cast("intellect-paid", false, true);
                let got = added(&s);
                assert_eq!(got.len(), 3);
                for card in &got {
                    assert_eq!(card.cost_override, Some(0));
                }
            }

            #[test]
            fn set_cost_reads_through_param() {
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
                let got = added(&s);
                assert_eq!(got.len(), 3);
                for card in got {
                    assert_eq!(card.cost_override, Some(1));
                }
            }

            #[test]
            fn r387_each_a_non_token_card_of_any_set_never_itself() {
                // The pools span the sets: KY and CN cards come from more than one.
                for tag in ["KY", "CN"] {
                    let sets: Vec<SetName> = pool_ids(json!({ "tags": [tag] }))
                        .iter()
                        .map(|id| crate::card_def(id).set)
                        .fold(Vec::new(), |mut sets, set| {
                            if !sets.contains(&set) {
                                sets.push(set);
                            }
                            sets
                        });
                    assert!(sets.len() > 1, "{tag} cards of one set only: {sets:?}");
                }
                for n in 0..40 {
                    let s = cast(&format!("intellect-sweep-{n}"), false, false);
                    for event in s.events() {
                        if let GameEvent::AddedToHand { player, def_id, .. } = event
                            && *player == P1
                        {
                            assert_ne!(def_id, INTELLECT, "never itself (R387)");
                            assert!(!crate::card_def(def_id).token, "{def_id} is a token");
                        }
                    }
                }
            }

            #[test]
            fn the_hand_cap_burns_in_that_order() {
                // Eight cards stay after the play: the KY card and the CN card land, the Book burns.
                let mut hand = vec![json!(INTELLECT)];
                hand.extend(std::iter::repeat_n(json!(FILLER), HAND_CAP as usize - 2));
                let mut s = scenario(json!({
                    "seed": "intellect-cap",
                    "p1": { "hand": hand, "library": [FILLER, FILLER], "mana": 4 },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(INTELLECT, json!({}));
                assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
                let landed: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AddedToHand { player, def_id, .. } if *player == P1 => {
                            Some(def_id.clone())
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(landed.len(), 2, "two land: {landed:?}");
                assert!(pool_ids(json!({ "tags": ["KY"] })).contains(&landed[0]));
                assert!(pool_ids(json!({ "tags": ["CN"] })).contains(&landed[1]));
                let burned: Vec<CardInstance> = s
                    .pile(P1, "graveyard")
                    .into_iter()
                    .filter(|card| card.def_id != INTELLECT && card.def_id != FILLER)
                    .collect();
                assert_eq!(burned.len(), 1, "the third, the Book, burns");
                assert!(pool_ids(json!({ "tags": ["Book"] })).contains(&burned[0].def_id));
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
