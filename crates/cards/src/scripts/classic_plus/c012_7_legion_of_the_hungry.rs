//! C+ #12.7 Legion of the Hungry (SPEC §8.7 row 12.7, R408): (2) Field Spell, Pancake, Token (printed Legendary).
//!   Base:    "Cry: Exile {cards} random cards from your deck. Summon the Units among them."
//!   Radiant: "… Summon the Units among them and make them Radiant."
//! R408's Cry reading. Five different cards (R60), all of them if fewer; the Units among them are
//! summoned out of exile in the order they were exiled (no Cry, R1), per R64, until the board is full,
//! the rest staying exiled; a unit-token card exiled from a deck has ceased to exist (R11).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012-7";

/// `forEachCard`'s `cards`, typed (TS `(ctx) => readonly (CardInstance | string)[]`, ids here).
fn cards_of(f: impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> ForEachCardCards {
    Arc::new(f)
}

/// `forEachCard`'s `each`, typed (TS `(instanceId) => Effect`).
fn each_of(f: impl Fn(&str) -> Effect + Send + Sync + 'static) -> ForEachCardEach {
    Arc::new(f)
}

fn legion(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // ponytail: the picks are drawn as the Cry resolves; nothing in this list can pause between them.
            let library = zone_cards(&ctx.state, ctx.controller, OffFieldZone::Library);
            let shuffled = ctx.rng.shuffle(&library);
            let count = param(&*ctx, "cards").max(0) as usize;
            let picks: Vec<CardInstance> = shuffled.into_iter().take(count).collect();
            let units: Vec<String> = picks
                .iter()
                .filter(|card| card_type_of(&ctx.state, card) == CardType::Unit)
                .map(|card| card.id.clone())
                .collect();
            let pick_ids: Vec<String> = picks.iter().map(|card| card.id.clone()).collect();
            vec![
                for_each_card(ForEachCardArgs {
                    cards: cards_of(move |_| pick_ids.clone()),
                    each: each_of(|instance_id| {
                        exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                    }),
                }),
                // A summon that finds no zone leaves the card in exile, and on its old face.
                for_each_card(ForEachCardArgs {
                    cards: cards_of(move |_| units.clone()),
                    each: each_of(move |instance_id| {
                        summon(json_as(json!({
                            "instance": { "of": "instance", "instanceId": instance_id },
                            "radiant": radiant,
                        })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: legion(false),
        radiant: legion(true),
    }
}

// C+ #12.7 Legion of the Hungry — SPEC §8.7 row 12.7, R408, BUILD M9 Classic+ row C+ 12.7: "Field Spell
// whose unlabelled text is its Cry (R408): exiles 5 different random cards of your deck (all if fewer,
// R60), then summons the Units among them out of exile into your leftmost open zones, no Cry (R1),
// until your board is full, the rest staying exiled; an empty deck does nothing and draws nothing
// (R129); the exiles are public and the deck's order stays hidden (R97); the card count reads through
// `param()`; radiant the summoned Units are made Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const LEGION: &str = "classicplus-012-7";
    const TOKENS: &str = "core-015"; // (1) Unit 1/1, Cry: summon a Rush Token
    const MENACE: &str = "core-019"; // (3) Unit
    const POINTMASTER: &str = "core-020"; // (2) Unit
    const RUSH: &str = "core-t-rush";
    const LUNAR: &str = "core-035"; // a Spell
    const STOCKPILE: &str = "core-005"; // a Spell

    /// TS `{ ...base, ...extra }` on two object literals (a shallow merge; `extra`'s keys win).
    fn merged(base: Value, extra: Value) -> Value {
        let mut out = base;
        if let (Some(into), Value::Object(from)) = (out.as_object_mut(), extra) {
            for (key, value) in from {
                into.insert(key, value);
            }
        }
        out
    }

    fn legion(radiant: bool, library: Value, p1: Value, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut options = json!({
            "p1": merged(
                json!({ "hand": [{ "def": LEGION, "radiant": radiant }, STOCKPILE], "library": library, "mana": 8 }),
                p1,
            ),
            "p2": { "hand": [STOCKPILE] },
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        let mut s = scenario(options);
        s.play(LEGION, json!({}));
        s
    }

    fn units_of(s: &Scenario) -> Vec<String> {
        (1..=5).filter_map(|lane| s.unit(P1, lane).map(|unit| unit.def_id)).collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r408_r60_its_cry_exiles_5_different_random_cards_of_your_deck() {
            crate::register_all();
            let s = legion(
                false,
                json!([LUNAR, STOCKPILE, LUNAR, STOCKPILE, LUNAR, STOCKPILE, LUNAR, STOCKPILE]),
                json!({}),
                None,
            );
            let exile = s.pile(P1, "exile");
            assert_eq!(exile.len(), 5);
            assert_eq!(exile.iter().map(|card| card.id.clone()).collect::<IndexSet<_>>().len(), 5);
            assert_eq!(s.pile(P1, "library").len(), 3);
        }

        #[test]
        fn all_of_them_when_fewer_than_5() {
            crate::register_all();
            let s = legion(false, json!([LUNAR, STOCKPILE, MENACE]), json!({}), None);
            assert_eq!(s.pile(P1, "library").len(), 0);
            assert_eq!(units_of(&s), vec![MENACE]);
        }

        #[test]
        fn r1_the_units_among_them_are_summoned_out_of_exile_with_no_cry_the_rest_stay_exiled() {
            crate::register_all();
            let s = legion(false, json!([TOKENS, LUNAR, MENACE]), json!({}), None);
            let mut units = units_of(&s);
            units.sort();
            let mut want = vec![MENACE.to_string(), TOKENS.to_string()];
            want.sort();
            assert_eq!(units, want);
            assert!(!s
                .events()
                .iter()
                .any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH)));
            assert_eq!(def_ids(&s.pile(P1, "exile")), vec![LUNAR]);
        }

        #[test]
        fn r64_they_fill_the_leftmost_open_zones_in_the_order_they_were_exiled_until_the_board_is_full() {
            crate::register_all();
            for i in 0..6 {
                let s = legion(
                    false,
                    json!([MENACE, POINTMASTER, TOKENS]),
                    json!({ "field": [MENACE, MENACE, { "def": MENACE, "lane": 5 }] }),
                    Some(&format!("legion-{i}")),
                );
                // Two open zones (3 and 4) for three Units: the first two exiled are summoned, the third stays.
                let exiled_order: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Exiled { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(s.unit(P1, 3).map(|unit| unit.id), exiled_order.first().cloned());
                assert_eq!(s.unit(P1, 4).map(|unit| unit.id), exiled_order.get(1).cloned());
                assert_eq!(
                    s.pile(P1, "exile").iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
                    vec![exiled_order.get(2).cloned().unwrap_or_default()]
                );
            }
        }

        #[test]
        fn r11_a_unit_token_card_exiled_from_the_deck_ceases_to_exist_and_is_not_summoned() {
            crate::register_all();
            let s = legion(false, json!([RUSH, LUNAR]), json!({}), None);
            assert_eq!(units_of(&s), Vec::<String>::new());
            assert_eq!(def_ids(&s.pile(P1, "exile")), vec![LUNAR]);
        }

        #[test]
        fn r129_an_empty_deck_does_nothing_and_draws_nothing_from_the_rng() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [LEGION, STOCKPILE], "library": [], "mana": 8 },
                "p2": { "hand": [STOCKPILE] },
            }));
            let cursor = s.state().rng_cursor;
            s.play(LEGION, json!({}));
            assert_eq!(s.state().rng_cursor, cursor);
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(LEGION.to_string()));
        }

        #[test]
        fn r97_the_exiles_are_public_no_event_carries_a_deck_position() {
            crate::register_all();
            let s = legion(false, json!([LUNAR, STOCKPILE, MENACE, LUNAR, STOCKPILE, LUNAR]), json!({}), None);
            let events = s.view(P2).events;
            let exiled: Vec<&GameEvent> =
                events.iter().filter(|event| event.event_type() == GameEventType::Exiled).collect();
            assert_eq!(exiled.len(), 5);
            for event in &exiled {
                if let GameEvent::Exiled { def_id, .. } = event {
                    assert_ne!(def_id, "hidden");
                }
            }
            assert!(!serde_json::to_string(&events).unwrap().contains("\"position\""));
        }

        #[test]
        fn r386_the_card_count_reads_through_param_a_degrade_exiles_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [LEGION, STOCKPILE], "library": [LUNAR, LUNAR, LUNAR, LUNAR, LUNAR, LUNAR], "mana": 8 },
                "p2": { "hand": [STOCKPILE] },
            }));
            step_param(s.card_mut(LEGION), "cards", -1);
            s.play(LEGION, json!({}));
            assert_eq!(s.pile(P1, "exile").len(), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_summoned_units_are_made_radiant_a_unit_left_in_exile_is_not() {
            crate::register_all();
            let s = legion(
                true,
                json!([MENACE, POINTMASTER, LUNAR]),
                json!({ "field": [MENACE, MENACE, MENACE, { "def": MENACE, "lane": 5 }] }),
                None,
            );
            let summoned = s.unit(P1, 4);
            assert_eq!(summoned.map(|card| card.radiant), Some(true));
            let left: Vec<CardInstance> =
                s.pile(P1, "exile").into_iter().filter(|card| card.def_id != LUNAR).collect();
            assert_eq!(left.len(), 1);
            assert_eq!(left.first().map(|card| card.radiant), Some(false));
        }
    }
}
