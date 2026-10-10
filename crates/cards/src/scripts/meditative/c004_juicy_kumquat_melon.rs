//! M #4 Juicy Kumquat Melon (SPEC §8.8 row 4, BUILD M10 row M 4). (4) Spell, Epic.
//!   Base:    "Draw a (0), (1), (2), (3), (4) and (5) Cost Meditative card."
//!   Radiant: the same, plus "They cost ({discount}) less." (2)
//!
//! Six draws in cost order (R801). For each cost N the script takes the topmost card of its
//! controller's library (index 0 is the top) whose definition's set is Meditative and whose cost,
//! read as `matches_library_filter` reads it (R65: an X card counts as 0, an embiggen card at its
//! base price, `costMod` included), is N. The six picks are chosen as the effect begins and noted on
//! the resolving card (`remember`), so a cast-on-draw pause cannot lose them; each is then drawn with
//! `draw_from_library`, a real §2.4 draw (cast on draw, R58's chain, the hand cap and R457's limit all
//! apply). A cost with no match draws nothing and causes no fatigue.
//!
//! On the Radiant face only, each picked card that reached the hand costs ({discount}) less
//! (`set_cost_mod`, `inHandOnly`; R4): a picked card cast on draw or burned never lands there and
//! takes no discount. The number is declared and read through `param` (R386); the base face prints
//! none (R749).

use jackioh_engine::effects::{ForEachCardArgs, draw_from_library, for_each_card, remember, set_cost_mod};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-004";

/// The six costs the card draws, in order.
const COSTS: [i32; 6] = [0, 1, 2, 3, 4, 5];

/// Where the resolving card notes its six picks as the effect begins.
const PICKED_KEY: &str = "kumquatPicked";

/// The topmost library card of `cost`, Meditative and costing it (R65, R801), or None.
fn topmost_of_cost(state: &GameState, controller: PlayerId, cost: i32) -> Option<String> {
    zone_cards(state, controller, OffFieldZone::Library)
        .iter()
        .find(|card| {
            def_of(Some(state), &card.def_id).set == SetName::Meditative
                && effective_cost(state, card, Default::default()) == cost
        })
        .map(|card| card.id.clone())
}

/// The picks the resolving card noted as the effect began.
fn picked(ctx: &EffectContext<'_>) -> Vec<String> {
    recalled(ctx, PICKED_KEY)
        .as_ref()
        .and_then(|value| value.as_array())
        .map(|ids| ids.iter().filter_map(|id| id.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// "They": the picked cards that reached the controller's hand (a cast-on-draw card never gets there,
/// R58; a burned one isn't there either).
fn picked_in_hand(ctx: &EffectContext<'_>) -> Vec<CardInstance> {
    let picked: std::collections::BTreeSet<String> = picked(ctx).into_iter().collect();
    zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Hand)
        .iter()
        .filter(|card| picked.contains(&card.id))
        .map(CardInstance::clone)
        .collect()
}

fn draw_picked(instance_id: &str) -> Effect {
    draw_from_library(json_as(json!({ "instanceId": instance_id })))
}

/// The six picks, chosen as the effect begins, then one `draw_from_library` each.
fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let state: &GameState = &ctx.state;
    let choices: Vec<Value> =
        COSTS.iter().filter_map(|cost| topmost_of_cost(state, ctx.controller, *cost)).map(Value::from).collect();
    vec![
        remember(json_as(json!({ "key": PICKED_KEY, "value": choices }))),
        for_each_card(ForEachCardArgs {
            cards: Arc::new(|now: &mut EffectContext<'_>| picked(now)),
            each: Arc::new(draw_picked),
        }),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|ctx| {
            // "They cost (2) less": the declared number `discount` (R386), on each picked card that
            // reached the hand (R4).
            let discount = param(&*ctx, "discount");
            let mut effects = cry(ctx);
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(|now: &mut EffectContext<'_>| {
                    picked_in_hand(now).into_iter().map(|card| card.id).collect()
                }),
                each: Arc::new(move |instance_id: &str| {
                    set_cost_mod(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                        "amount": -discount,
                        "inHandOnly": true,
                    })))
                }),
            }));
            effects
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// M #4 Juicy Kumquat Melon — SPEC §8.8 row 4, BUILD M10 row M 4: "Draw the topmost Meditative card of
// each cost 0 to 5, in cost order (R801); a non-Meditative card of that cost is passed over; a cost
// with no match draws nothing and causes no fatigue; an X card counts 0 and an embiggen card its
// base (R65); no rng; a full hand burns the draw; radiant: each card that reaches the hand costs 2
// less, a card that never gets there none; the tuned discount reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const MELON: &str = "meditative-004";
    const GIFT: &str = "meditative-006"; // (0) Spell.
    const CUTS: &str = "meditative-005"; // (1) Spell.
    const DISRUPTOR: &str = "meditative-001"; // (2) Field Spell.
    const INTROSPECTION: &str = "meditative-007"; // (4) Field Spell.
    const FISHING: &str = "meditative-021"; // (0, embiggen 1) Spell.
    const FILLER: &str = "core-005"; // (1) Spell.
    const CHEAP: &str = "core-010"; // (0) Spell.
    const MENACE: &str = "core-019"; // (3) Unit.
    const BIG: &str = "classic-060"; // (5) Spell.
    const DIVIDEND: &str = "core-024"; // (X) Spell.

    /// The def ids of p1's `drawn` events, in order.
    fn drawn_defs(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod meditative_004 {
        use super::*;

        #[test]
        fn r801_draws_the_topmost_meditative_card_of_each_cost_0_to_5_in_cost_order() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [MELON, FILLER],
                    "mana": 9,
                    "library": [GIFT, CUTS, DISRUPTOR, MENACE, INTROSPECTION, BIG],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(MELON, json!({}));

            // Costs 0, 1, 2 and 4 draw, in that order; 3 and 5 have no Meditative match.
            assert_eq!(drawn_defs(&s), vec![GIFT, CUTS, DISRUPTOR, INTROSPECTION]);
            assert_eq!(s.hand(P1).len(), 5);
            let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(library, vec![MENACE, BIG]);
            s.expect_health(P1, HERO_HEALTH);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
        }

        #[test]
        fn r801_a_non_meditative_card_of_that_cost_is_passed_over() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [MELON, FILLER],
                    "mana": 9,
                    // A (0) and a (1) that are not Meditative sit above the Meditative matches.
                    "library": [CHEAP, FILLER, GIFT, CUTS],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(MELON, json!({}));

            assert_eq!(drawn_defs(&s), vec![GIFT, CUTS]);
            let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(library, vec![CHEAP, FILLER]);
        }

        #[test]
        fn r801_a_cost_with_no_match_draws_nothing_and_no_fatigue() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [MELON, FILLER], "mana": 9, "library": [CHEAP, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(MELON, json!({}));

            assert!(drawn_defs(&s).is_empty());
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.pile(P1, "library").len(), 2);
            s.expect_health(P1, HERO_HEALTH);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
        }

        #[test]
        fn r65_an_x_card_counts_0_and_an_embiggen_card_its_base() {
            crate::register_all();
            // No X-cost Meditative card exists yet, so the X half pins R65's reading on the X card in
            // the deck: it counts 0, but it is passed over all the same for not being Meditative.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [MELON, FILLER],
                    "mana": 9,
                    "library": [DIVIDEND, FISHING, CUTS],
                },
                "p2": { "hand": [FILLER] },
            }));
            let dividend = s.card(DIVIDEND).clone();
            assert_eq!(effective_cost(s.state(), &dividend, Default::default()), 0);

            s.play(MELON, json!({}));

            // The embiggen card is read at its base price (0), not its paid price (1): it is the (0),
            // and the (1) is still the topmost Meditative (1).
            assert_eq!(drawn_defs(&s), vec![FISHING, CUTS]);
            let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(library, vec![DIVIDEND]);
        }

        #[test]
        fn draws_no_random_number() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-004-no-rng",
                "p1": {
                    "hand": [MELON, FILLER],
                    "mana": 9,
                    "library": [GIFT, CUTS, DISRUPTOR, MENACE, INTROSPECTION, BIG],
                },
                "p2": { "hand": [FILLER] },
            }));
            let cursor = s.state().rng_cursor;

            s.play(MELON, json!({}));

            assert_eq!(drawn_defs(&s).len(), 4);
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn a_full_hand_burns_the_draw() {
            crate::register_all();
            let mut hand: Vec<Value> = vec![json!(MELON)];
            hand.extend(vec![json!(FILLER); 9]);
            let mut s = scenario(json!({
                "p1": {
                    "hand": hand,
                    "mana": 9,
                    "library": [GIFT, CUTS, DISRUPTOR, INTROSPECTION],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(MELON, json!({}));

            // Nine cards plus the (0): the (1), (2) and (4) burn into the graveyard (R4).
            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(s.card(GIFT).zone.z(), ZoneName::Hand);
            for def in [CUTS, DISRUPTOR, INTROSPECTION] {
                assert_eq!(s.card(def).zone.z(), ZoneName::Graveyard, "{def} burns");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_each_card_that_reaches_the_hand_costs_2_less() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": MELON, "radiant": true }, FILLER],
                        "mana": 9,
                        "library": [GIFT, CUTS, DISRUPTOR, INTROSPECTION],
                    },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(MELON, json!({}));

                assert_eq!(drawn_defs(&s), vec![GIFT, CUTS, DISRUPTOR, INTROSPECTION]);
                for def in [GIFT, CUTS, DISRUPTOR, INTROSPECTION] {
                    assert_eq!(s.card(def).cost_mod, -2, "{def} costs 2 less");
                }
                // A card already in the hand is untouched.
                assert_eq!(s.card(FILLER).cost_mod, 0);
            }

            #[test]
            fn radiant_a_picked_card_that_never_reaches_the_hand_takes_no_discount() {
                crate::register_all();
                // No cast-on-draw Meditative card exists yet (it would take this test's place): a
                // picked card burned by a full hand takes the same path — never in the hand, so the
                // discount's hand filter passes it over.
                let mut hand: Vec<Value> = vec![json!({ "def": MELON, "radiant": true })];
                hand.extend(vec![json!(FILLER); 9]);
                let mut s = scenario(json!({
                    "p1": {
                        "hand": hand,
                        "mana": 9,
                        "library": [GIFT, CUTS],
                    },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(MELON, json!({}));

                assert_eq!(s.card(GIFT).cost_mod, -2);
                assert_eq!(s.card(GIFT).zone.z(), ZoneName::Hand);
                assert_eq!(s.card(CUTS).zone.z(), ZoneName::Graveyard);
                assert_eq!(s.card(CUTS).cost_mod, 0);
            }
        }
    }
}
